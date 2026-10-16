//! Pages: loading them, route changes, changes to their controls, selections, and live
//! regions.

use super::{AppMode, ScreenReaderEngine};
use crate::aom::{AomNode, Role};
use crate::buffer::VirtualBuffer;
use crate::event::{AccessibilityEvent, FocusEvent, LiveRegionEvent, Politeness};
use crate::settings::LiveRegionAppend;
use crate::speech::SpeechFormatter;

impl ScreenReaderEngine {
    /// Handles client-side SPA routing transitions: announces new page title and retargets cursor.
    pub fn handle_route_transition(&mut self, new_title: &str, target_id: Option<&str>) -> String {
        let title_announcement = SpeechFormatter::format_page_title_change(new_title);
        let mut announcements = vec![title_announcement];

        if let Some(target) = target_id {
            let resolved_offset = if let Ok(num_id) = target.parse::<usize>() {
                self.cursor.move_to_node_id(num_id);
                Some(self.cursor.offset)
            } else if let Some(&offset) = self.cursor.buffer.target_anchors.get(target) {
                self.cursor.offset = offset;
                self.cursor.current_node_id =
                    self.cursor.buffer.node_at_offset(offset).map(|n| n.id);
                Some(offset)
            } else {
                None
            };

            if resolved_offset.is_some() {
                let line = self.cursor.current_line();
                if !line.is_empty() {
                    announcements.push(line);
                }
            }
        }

        let full_text = announcements.join("\n");
        self.speech.speak(&full_text, true);
        full_text
    }

    /// Handles a page load ([`AccessibilityEvent::DocumentLoaded`]) whose buffer is already
    /// built. Building a big page's buffer takes long, so the live screen reader builds it
    /// before taking the engine, which the keyboard hook waits on.
    pub fn handle_document_loaded(&mut self, title: &str, buffer: VirtualBuffer) -> Option<String> {
        // The page already in the buffer, captured again (the first focus in it after
        // start-up): refresh it without announcing it or leaving forms mode.
        let same_page = self.app_mode == AppMode::VirtualBuffer
            && self.cursor.buffer.document_title() == Some(title);
        if same_page {
            // Applied once the events of this change are handled: several live regions can
            // change at once, and each is compared with the page before the change
            // [obs, 206/s2].
            self.pending_page = Some(buffer);
            return None;
        }
        self.pending_page = None;
        self.last_page_focus = None;
        self.tab_start = None;
        self.option_list = None;
        self.web_application = None;
        self.last_live_text.clear();
        self.live_streams.clear();
        self.load_buffer(buffer);
        self.app_mode = AppMode::VirtualBuffer;
        self.last_window_title = Some(title.to_owned());
        let welcome = format!("{}  \u{001d} Virtual PC Cursor \u{001e}", title);
        self.speech.speak(&welcome, true);
        Some(welcome)
    }

    /// The node of the line under the virtual cursor.
    pub(super) fn node_under_cursor(&self) -> Option<AomNode> {
        let line = self.cursor.buffer.line_at_offset(self.cursor.offset)?;
        self.cursor.buffer.node(line.node_id).cloned()
    }

    /// A page control changed (Space toggled a check box, a combo box took a new value): the
    /// buffer is rebuilt with the new state, the virtual cursor stays on the control, and the
    /// change is spoken.
    fn page_change(&mut self, control: &FocusEvent) -> Option<String> {
        let id = self.page_node_for(control)?;
        let before = self.cursor.buffer.node(id)?.clone();
        let speech = match control.role {
            Role::CheckBox | Role::RadioButton
                if before.states.checked != control.states.checked =>
            {
                let state = if control.states.checked == Some(true) {
                    "checked"
                } else {
                    "not checked"
                };
                crate::linearize::state_marker(state)
            }
            // An autocomplete's list opened or closed as it is typed in: " expanded " (JAWS on
            // the dev box). Its value is the typing, already echoed.
            Role::ComboBox
                if before
                    .attributes
                    .get("editable")
                    .is_some_and(|e| e == "true") =>
            {
                if control.states.expanded.is_none()
                    || before.states.expanded == control.states.expanded
                {
                    return None;
                }
                crate::linearize::state_marker(if control.states.expanded == Some(true) {
                    "expanded"
                } else {
                    "collapsed"
                })
            }
            Role::ComboBox if before.value != control.value => control.value.clone()?,
            // Space on a tab selects it [obs, 29/s4].
            Role::Tab
                if before.states.selected != control.states.selected
                    && control.states.selected == Some(true) =>
            {
                crate::linearize::state_marker("selected")
            }
            // A disclosure button opened or closed (Space or Enter on it): JAWS 2025 says the
            // new state (dev box, GitHub's "Solutions" menu button: " expanded ").
            Role::Button | Role::MenuButton
                if control.states.expanded.is_some()
                    && before.states.expanded != control.states.expanded =>
            {
                crate::linearize::state_marker(if control.states.expanded == Some(true) {
                    "expanded"
                } else {
                    "collapsed"
                })
            }
            _ => return None,
        };
        let rebuilt = self.cursor.buffer.with_node_changed(id, |node| {
            node.states.expanded = control.states.expanded.or(node.states.expanded);
            node.states.checked = control.states.checked;
            node.states.selected = control.states.selected;
            if control.value.is_some() {
                node.value = control.value.clone();
            }
        })?;
        self.cursor.buffer = rebuilt;
        self.cursor.move_to_node_id(id);
        self.speech.speak(&speech, true);
        Some(speech)
    }

    /// The region markers for the landmarks around a page node, outermost first: `main`
    /// and forms (named or not) [obs, 19/s2; 204/s1].
    /// The lists and landmarks around `to` that are not around `from`, outermost first, each
    /// on its line: "Repository files ‹ NavigationRegion ›", "‹ list › | with 1 items",
    /// " ‹ MainRegion › ", " ‹ FormRegion › ", "Related ‹ ComplementaryRegion ›", "Updates
    /// ‹ Region ›" (JAWS's names, dev box 2026-10-16; a table is said otherwise, and left out).
    pub(super) fn entered_containers(&self, from: usize, to: usize) -> String {
        let ancestors = |id: usize| -> Vec<&AomNode> {
            let mut path = Vec::new();
            if let Some(root) = self
                .cursor
                .buffer
                .root_id
                .and_then(|r| self.cursor.buffer.node(r))
            {
                ancestors_of(root, id, &mut path);
            }
            path
        };
        let around_from: std::collections::HashSet<usize> =
            ancestors(from).iter().map(|n| n.id).collect();
        ancestors(to)
            .into_iter()
            .filter(|n| !around_from.contains(&n.id) && n.id != from)
            .filter_map(|n| {
                let name = n.name.as_deref().map(str::trim).unwrap_or_default();
                let named = |marker: &str| {
                    if name.is_empty() {
                        format!(" \u{1d} {marker} \u{1e} \n")
                    } else {
                        format!("{name} \u{1d} {marker} \u{1e} \n")
                    }
                };
                match &n.role {
                    Role::Landmark { landmark_type } => match landmark_type.as_str() {
                        "main" => Some(named("MainRegion")),
                        "form" => Some(named("FormRegion")),
                        "navigation" => Some(named("NavigationRegion")),
                        "banner" => Some(named("BannerRegion")),
                        "complementary" => Some(named("ComplementaryRegion")),
                        "search" => Some(named("SearchRegion")),
                        "contentinfo" => Some(named("ContentInfoRegion")),
                        "region" => Some(named("Region")),
                        _ => None,
                    },
                    Role::List => {
                        let items = n
                            .children
                            .iter()
                            .filter(|c| c.role == Role::ListItem)
                            .count();
                        Some(format!("{}with {items} items\n", named("list")))
                    }
                    _ => None,
                }
            })
            .collect()
    }

    pub(super) fn region_markers(&self, node_id: usize) -> String {
        let Some(root) = self
            .cursor
            .buffer
            .root_id
            .and_then(|r| self.cursor.buffer.node(r))
        else {
            return String::new();
        };
        let mut ancestors = Vec::new();
        ancestors_of(root, node_id, &mut ancestors);
        ancestors
            .iter()
            .filter_map(|n| {
                let aria = n.attributes.get("aria_role").map(String::as_str);
                match &n.role {
                    Role::Landmark { landmark_type } if landmark_type == "main" => {
                        Some("MainRegion")
                    }
                    Role::Landmark { landmark_type } if landmark_type == "form" => {
                        Some("FormRegion")
                    }
                    _ if aria == Some("form") => Some("FormRegion"),
                    _ => None,
                }
            })
            .map(|r| format!(" \u{1d} {r} \u{1e} \n"))
            .collect()
    }

    /// The buffer node a page event concerns: by node id, then by DOM id, then by role and
    /// name. Candidates are taken in document order, so repeated names resolve to the first.
    pub(super) fn page_node_for(&self, focus: &FocusEvent) -> Option<usize> {
        let buffer = &self.cursor.buffer;
        if let Some(id) = focus.node_id.filter(|id| buffer.contains_node(*id)) {
            return Some(id);
        }
        let mut ids: Vec<usize> = buffer.node_ids().collect();
        ids.sort_unstable();
        let node = |id: &usize| buffer.node(*id);
        if let Some(dom_id) = focus.dom_id.as_deref() {
            if let Some(id) = ids.iter().find(|id| {
                node(id).is_some_and(|n| n.attributes.get("id").map(String::as_str) == Some(dom_id))
            }) {
                return Some(*id);
            }
        }
        let name = focus.name.as_deref()?;
        ids.into_iter().find(|id| {
            node(id).is_some_and(|n| {
                same_role(&n.role, &focus.role)
                    && n.name
                        .as_deref()
                        .or(n.text.as_deref())
                        .is_some_and(|nn| nn.eq_ignore_ascii_case(name))
            })
        })
    }

    /// The focused control changed without focus moving (Space on a check box, a tree item
    /// expanded, a combo box value). Speaks what changed, as JAWS does.
    pub fn handle_change(
        &mut self,
        kind: crate::event::ChangeKind,
        control: &FocusEvent,
    ) -> Option<String> {
        tracing::debug!(?kind, role = ?control.role, name = ?control.name, "control changed");
        if control.in_document {
            return self.page_change(control);
        }
        let before = self.last_native_focus.replace(control.clone())?;
        let speech = crate::native_focus::change_speech(&before, control)?;
        self.speech.speak(&speech, true);
        Some(speech)
    }

    /// An item selected in a list while the focus stays in a field (Windows search's results,
    /// JAWS 2025 on Windows 11, dev box: typing "note" gives "Notepad, App, Press right to view
    /// options | 1 of 5", Down "notepad, See more search results, Press right to switch preview
    /// | 2 of 5"): its name and position. The field's edit key that led to it is not read: its
    /// effect was the selection.
    pub(super) fn handle_selected(&mut self, item: &FocusEvent) -> Option<String> {
        let name = item
            .name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())?;
        self.pending_edit = None;
        let speech = match (item.item_index, item.item_count) {
            (Some(i), Some(n)) => format!("{name}\n{i} of {n}"),
            _ => name.to_owned(),
        };
        self.speech.speak(&speech, true);
        Some(speech)
    }

    /// Replaces the buffer with the page refreshed after a change, keeping the virtual
    /// cursor's place.
    pub(super) fn apply_pending_page(&mut self) {
        if let Some(buffer) = self.pending_page.take() {
            let offset = self.cursor.offset;
            let line = self.cursor.buffer.line_at_offset(offset);
            // A cursor on the focus stays on it: the focus is found again by its state, where
            // its line's text can change (the full capture knows more of a control than the
            // quick one: "edit combo" for "Combo box").
            let on_focus = line.is_some_and(|l| Some(l.node_id) == self.last_page_focus);
            let place = line.map(|line| {
                (
                    line.text.clone(),
                    line.start_offset,
                    offset.saturating_sub(line.start_offset),
                )
            });
            self.cursor.buffer = buffer;
            // Node ids are the capture's own: the focus is found again by its state.
            self.last_page_focus = self.cursor.buffer.focused_node();
            self.tab_start = self.last_page_focus;
            let focus_line = self
                .last_page_focus
                .filter(|_| on_focus)
                .and_then(|id| self.cursor.buffer.lines.iter().find(|l| l.node_id == id))
                .map(|l| l.start_offset);
            self.cursor.offset = match (focus_line, place) {
                (Some(start), _) => start,
                (None, Some((text, start, column))) => same_line(&self.cursor.buffer, &text, start)
                    .map(|line_start| line_start + column)
                    .unwrap_or(offset),
                (None, None) => offset,
            }
            .min(self.cursor.buffer.len());
            // The cursor's node, as the new capture numbers it.
            self.cursor.current_node_id = self
                .cursor
                .buffer
                .line_at_offset(self.cursor.offset)
                .map(|l| l.node_id);
        }
    }

    /// Completes an edit key the application did not act on (Down on the last line, Ctrl+C):
    /// the field's text is unchanged, so no change event follows, but JAWS still speaks
    /// (313/s5–s8). Live, the platform reads the field after every edit key and reports it
    /// changed or not; replay, which has only the recorded change events, calls this after a
    /// step's events.
    pub fn settle_pending_edit(&mut self) -> Option<String> {
        self.pending_edit?;
        let unchanged = self.last_text.clone()?;
        self.handle_accessibility_event(&AccessibilityEvent::TextContext(unchanged))
    }

    /// Called when the OS event stream goes quiet: speech JAWS defers until no further
    /// event supersedes it (a closed popup menu, unless focus moved on).
    pub fn on_idle(&mut self) -> Option<String> {
        self.apply_pending_page();
        let menu = self.menus.idle();
        if let Some(speech) = &menu {
            self.speech.speak(speech, true);
        }
        let streamed = self.finish_live_streams();
        menu.or(streamed)
    }

    /// Handles an ARIA live region announcement event with politeness consideration.
    pub fn handle_live_region_event(&mut self, live: &LiveRegionEvent) -> Option<String> {
        if live.politeness == Politeness::Off {
            // Kept current, never spoken.
            self.live_region_additions(live);
            return None;
        }
        let (added, continues) = match self.settings.live_region_append {
            LiveRegionAppend::Appended => self.live_stream_piece(live),
            LiveRegionAppend::WholeText => (self.live_region_additions(live), false),
        };
        if added.trim().is_empty() {
            return None;
        }
        // JAWS speaks what was added to the region, after the region's name: "Messages |
        // You: Message 1" [obs, 206/s2-s3]. A growing region's later pieces follow on from
        // its first without the name, queued: a piece never cuts off the one before it.
        let text = match live.container_name.as_deref() {
            Some(name) if !continues => format!("{name}\n{added}"),
            _ => added,
        };
        if live.politeness == Politeness::Assertive && !continues {
            self.speech.stop();
            self.speech.speak(&text, true);
        } else {
            self.speech.speak(&text, false);
        }
        Some(text)
    }

    /// What a live region's change adds to what was spoken of it, and whether it continues a
    /// growing region ([`LiveRegionAppend::Appended`]). Text appended to the spoken text is
    /// read from the start of the word the spoken text ended in; while the region grows
    /// within a line, its last word waits until it is whole ([`Self::finish_live_streams`]
    /// reads it when the region goes quiet). Any other change reads the lines the region
    /// lacked, as [`LiveRegionAppend::WholeText`] does.
    fn live_stream_piece(&mut self, live: &LiveRegionEvent) -> (String, bool) {
        let key = live.region_id.clone().unwrap_or_default();
        let new_text = live.text.clone();
        let (spoken_text, existed) = match self.live_streams.get(&key) {
            Some(stream) => (
                stream
                    .text
                    .get(..stream.spoken)
                    .unwrap_or_default()
                    .to_owned(),
                true,
            ),
            None => (self.region_old_lines(live).join("\n"), false),
        };
        if !new_text.starts_with(&spoken_text) {
            let added = self.live_region_additions(live);
            let spoken = new_text.len();
            self.live_streams.insert(
                key,
                LiveStream {
                    text: new_text,
                    spoken,
                    quiet: 0,
                },
            );
            return (added, false);
        }
        self.note_live_text(live);
        let continues = existed && !spoken_text.is_empty();
        // While the last piece is still being spoken, what arrives gathers, and is read as one
        // phrase once speech is free: a fast stream is read in phrases, not word by word.
        if continues && self.speech.is_speaking() {
            if let Some(stream) = self.live_streams.get_mut(&key) {
                stream.text = new_text;
                stream.quiet = 0;
            }
            return (String::new(), true);
        }
        let appended_on_line = !new_text
            .get(spoken_text.len()..)
            .unwrap_or_default()
            .starts_with('\n');
        let start = crate::live_text::word_start(&new_text, spoken_text.len());
        let end = if continues && appended_on_line {
            crate::live_text::complete_end(&new_text, start)
        } else {
            new_text.len()
        };
        let piece = new_text
            .get(start..end)
            .unwrap_or_default()
            .trim()
            .to_owned();
        self.live_streams.insert(
            key,
            LiveStream {
                text: new_text,
                spoken: end,
                quiet: 0,
            },
        );
        (piece, continues)
    }

    /// Reads what growing live regions have gathered once speech is free: their whole words
    /// so far, and once a region has been quiet for a moment, the rest too (the end of a
    /// streamed reply, whose last word no later piece completes).
    fn finish_live_streams(&mut self) -> Option<String> {
        if self.speech.is_speaking() {
            return None;
        }
        let mut pieces = Vec::new();
        for stream in self.live_streams.values_mut() {
            if stream.spoken >= stream.text.len() {
                continue;
            }
            stream.quiet = stream.quiet.saturating_add(1);
            let start = crate::live_text::word_start(&stream.text, stream.spoken);
            let end = if stream.quiet < LIVE_STREAM_QUIET_TICKS {
                crate::live_text::complete_end(&stream.text, start)
            } else {
                stream.text.len()
            };
            let piece = stream.text.get(start..end).unwrap_or_default().trim();
            if !piece.is_empty() {
                pieces.push(piece.to_owned());
            }
            stream.spoken = stream.spoken.max(end);
        }
        if pieces.is_empty() {
            return None;
        }
        let speech = pieces.join("\n");
        self.speech.speak(&speech, false);
        Some(speech)
    }

    /// A live region's text before this change: as last reported, or in the page.
    fn region_old_lines(&self, live: &LiveRegionEvent) -> Vec<String> {
        fn texts(node: &AomNode, out: &mut Vec<String>) {
            if let Some(t) = node
                .text
                .as_deref()
                .map(str::trim)
                .filter(|t| !t.is_empty())
            {
                out.push(t.to_owned());
            }
            for child in &node.children {
                texts(child, out);
            }
        }
        // The region's text when it last changed, if newer than the page: the page is
        // refreshed after the change, so two quick changes (two chat messages) would otherwise
        // compare the second with a page that lacks the first.
        if let Some(last) = live
            .region_id
            .as_ref()
            .and_then(|id| self.last_live_text.get(id))
        {
            return last.clone();
        }
        let buffer = &self.cursor.buffer;
        let mut ids: Vec<usize> = buffer.node_ids().collect();
        ids.sort_unstable();
        let region = live.region_id.as_deref().and_then(|dom_id| {
            ids.iter().find_map(|id| {
                buffer
                    .node(*id)
                    .filter(|n| n.attributes.get("id").map(String::as_str) == Some(dom_id))
            })
        });
        let mut old = Vec::new();
        if let Some(region) = region {
            texts(region, &mut old);
        }
        old
    }

    /// Remembers a live region's text as this change left it.
    fn note_live_text(&mut self, live: &LiveRegionEvent) {
        if let Some(id) = &live.region_id {
            self.last_live_text
                .insert(id.clone(), live.text.lines().map(str::to_owned).collect());
        }
    }

    /// The lines of a live region's text that its previous text in the buffer lacked.
    fn live_region_additions(&mut self, live: &LiveRegionEvent) -> String {
        let old = self.region_old_lines(live);
        self.note_live_text(live);
        let new: Vec<&str> = live.text.lines().collect();
        let added = if old.len() <= new.len() && old.iter().zip(&new).all(|(o, n)| o.as_str() == *n)
        {
            new.get(old.len()..).unwrap_or_default().to_vec()
        } else {
            new
        };
        added.join("\n")
    }
}

/// Quiet ticks (the live screen reader's are 150 ms apart) before a growing live region's
/// held-back word is read.
const LIVE_STREAM_QUIET_TICKS: u8 = 3;

/// A live region being read as it grows ([`LiveRegionAppend::Appended`]).
#[derive(Debug, Clone)]
pub(super) struct LiveStream {
    /// The region's text now.
    text: String,
    /// How much of it has been spoken, in bytes.
    spoken: usize,
    /// Quiet ticks since it last changed.
    quiet: u8,
}

/// Whether a focus event's role is the page node's: a button opening a menu is a menu button
/// to MSAA, which focus events follow, and a plain button to UIA, which a page's quick first
/// capture reads.
fn same_role(node: &Role, focus: &Role) -> bool {
    node == focus
        || matches!(
            (node, focus),
            (Role::Button, Role::MenuButton) | (Role::MenuButton, Role::Button)
        )
}

/// Where a line with `text` starts in `buffer`, the one nearest to `near`: the cursor's line in
/// a page captured again, whose lines can shift (a live region grew above it, or the page was
/// first read quickly and then in full). `None` when no line has that text.
fn same_line(buffer: &VirtualBuffer, text: &str, near: usize) -> Option<usize> {
    buffer
        .lines
        .iter()
        .filter(|line| line.text == text)
        .map(|line| line.start_offset)
        .min_by_key(|start| start.abs_diff(near))
}

/// The nodes from `node` down to node `id`, without it, into `out`: its ancestors, outermost
/// first. False when `id` is not below `node`.
fn ancestors_of<'a>(node: &'a AomNode, id: usize, out: &mut Vec<&'a AomNode>) -> bool {
    if node.id == id {
        return true;
    }
    out.push(node);
    if node.children.iter().any(|c| ancestors_of(c, id, out)) {
        return true;
    }
    out.pop();
    false
}
