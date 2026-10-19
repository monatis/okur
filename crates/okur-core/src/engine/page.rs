//! Pages: loading them, route changes, changes to their controls, selections, and live
//! regions.

use super::{AppMode, ScreenReaderEngine};
use crate::aom::{AomNode, Role, UNIQUE_ID_ATTR};
use crate::buffer::VirtualBuffer;
use crate::event::{AccessibilityEvent, FocusEvent, LiveRegionEvent, Politeness};
use crate::messages::msg;
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
        // A page is known by its address, by its title when it has none: a title can change
        // on the page (WhatsApp's counts unread chats, "(8) WhatsApp"), and JAWS keeps its
        // place then (dev box, 2026-10-17).
        let same_page = self.app_mode == AppMode::VirtualBuffer
            && match (self.cursor.buffer.document_url(), buffer.document_url()) {
                (Some(old), Some(new)) => old == new,
                _ => self.cursor.buffer.document_title() == Some(title),
            };
        if same_page {
            // Applied once the events of this change are handled: several live regions can
            // change at once, and each is compared with the page before the change
            // [obs, 206/s2].
            if let Some(old) = self.pending_page.replace(buffer) {
                crate::retire::retire(old);
            }
            return None;
        }
        if let Some(old) = self.pending_page.take() {
            crate::retire::retire(old);
        }
        self.page_stale = false;
        self.last_page_focus = None;
        self.focus_table_unsaid = false;
        self.tab_start = None;
        self.option_list = None;
        self.web_application = None;
        self.last_live_text.clear();
        self.live_streams.clear();
        self.load_buffer(buffer);
        self.app_mode = AppMode::VirtualBuffer;
        self.last_window_title = Some(title.to_owned());
        let welcome = format!(
            "{}  \u{001d} {} \u{001e}",
            title,
            msg::role_virtual_pc_cursor()
        );
        self.speech.speak(&welcome, true);
        Some(welcome)
    }

    /// The node of the line under the virtual cursor. Borrowed: a list's line is the whole
    /// list, and Enter on one cloned 5,000 items in the keyboard hook (51 ms; responsiveness
    /// review, 2026-10-19).
    pub(super) fn node_under_cursor(&self) -> Option<&AomNode> {
        let line = self.cursor.buffer.line_at_offset(self.cursor.offset)?;
        self.cursor.buffer.node(line.node_id)
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
                    msg::state_checked()
                } else {
                    msg::state_not_checked()
                };
                crate::linearize::state_marker(state.as_str())
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
                crate::linearize::state_marker(
                    if control.states.expanded == Some(true) {
                        msg::state_expanded()
                    } else {
                        msg::state_collapsed()
                    }
                    .as_str(),
                )
            }
            Role::ComboBox if before.value != control.value => control.value.clone()?,
            // Space on a tab selects it [obs, 29/s4].
            Role::Tab
                if before.states.selected != control.states.selected
                    && control.states.selected == Some(true) =>
            {
                crate::linearize::state_marker(msg::state_selected().as_str())
            }
            // A disclosure button opened or closed (Space or Enter on it): JAWS 2025 says the
            // new state (dev box, GitHub's "Solutions" menu button: " expanded ").
            Role::Button | Role::MenuButton
                if control.states.expanded.is_some()
                    && before.states.expanded != control.states.expanded =>
            {
                crate::linearize::state_marker(
                    if control.states.expanded == Some(true) {
                        msg::state_expanded()
                    } else {
                        msg::state_collapsed()
                    }
                    .as_str(),
                )
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
        crate::retire::retire(std::mem::replace(&mut self.cursor.buffer, rebuilt));
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
                        "main" => Some(named(msg::role_region_main().as_str())),
                        "form" => Some(named(msg::role_region_form().as_str())),
                        "navigation" => Some(named(msg::role_region_navigation().as_str())),
                        "banner" => Some(named(msg::role_region_banner().as_str())),
                        "complementary" => Some(named(msg::role_region_complementary().as_str())),
                        "search" => Some(named(msg::role_region_search().as_str())),
                        "contentinfo" => Some(named(msg::role_region_content_info().as_str())),
                        "region" => Some(named(msg::role_region().as_str())),
                        _ => None,
                    },
                    Role::List => {
                        let items = n
                            .children
                            .iter()
                            .filter(|c| c.role == Role::ListItem)
                            .count();
                        Some(format!(
                            "{}{}\n",
                            named(msg::role_list().as_str()),
                            msg::list_items_count(items)
                        ))
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
                        Some(msg::role_region_main())
                    }
                    Role::Landmark { landmark_type } if landmark_type == "form" => {
                        Some(msg::role_region_form())
                    }
                    _ if aria == Some("form") => Some(msg::role_region_form()),
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
        // The live object, then the DOM id, from the buffer's indexes (each the smallest id
        // with it); then a node of the focus's role and name.
        if let Some(id) = focus
            .unique_id
            .as_deref()
            .and_then(|object| buffer.node_with_object(object))
        {
            return Some(id);
        }
        if let Some(id) = focus
            .dom_id
            .as_deref()
            .and_then(|dom_id| buffer.node_with_dom_id_indexed(dom_id))
        {
            return Some(id);
        }
        let name = focus.name.as_deref()?;
        let mut ids: Vec<usize> = buffer.node_ids().collect();
        ids.sort_unstable();
        let node = |id: &usize| buffer.node(*id);
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
        let control = self.keys_as_output(control);
        let before = self.last_native_focus.replace(control.clone())?;
        let speech = crate::native_focus::change_speech(&before, &control)?;
        self.speech.speak(&speech, true);
        Some(speech)
    }

    /// An item selected in a list while the focus stays in a field (Windows search's results,
    /// JAWS 2025 on Windows 11, dev box: typing "note" gives "Notepad, App, Press right to view
    /// options | 1 of 5", Down "notepad, See more search results, Press right to switch preview
    /// | 2 of 5"): its name and position. The field's edit key that led to it is not read: its
    /// effect was the selection.
    pub(super) fn handle_selected(&mut self, item: &FocusEvent) -> Option<String> {
        // A selection is read while the focus stays in a text field (a search box's results
        // or suggestions); where the focus moves with the selection (a list), the focus says
        // the item.
        if !self
            .last_native_focus
            .as_ref()
            .is_some_and(|f| f.role == Role::Edit)
        {
            return None;
        }
        let name = item
            .name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())?;
        self.pending_edit = None;
        // The same result selected again as the results change (Windows search, typing):
        // JAWS says its name alone ("Settings, System, Press right to view options" after
        // "… | 1 of 4"; dev box, 2026-10-17).
        let again = self.last_selected.as_deref() == Some(name);
        self.last_selected = Some(name.to_owned());
        let speech = match (item.item_index, item.item_count) {
            (Some(i), Some(n)) if !again => format!("{name}\n{}", msg::position(i, n)),
            _ => name.to_owned(),
        };
        self.speech.speak(&speech, true);
        Some(speech)
    }

    /// The page in front changed outside its live regions
    /// ([`AccessibilityEvent::PageChanged`]): in browse mode it is captured again once the
    /// changes settle. In forms mode it waits: typing changes a page with each key
    /// (WhatsApp's composer, a search box's results), and forms mode reads the focus, not
    /// the buffer.
    pub(super) fn page_changed(&mut self) {
        if self.app_mode != AppMode::VirtualBuffer {
            return;
        }
        if self.forms_mode {
            self.page_stale = true;
        } else if let Some(actions) = &self.actions {
            actions.page_changed(false);
        }
    }

    /// The parts of the page read again after they changed
    /// ([`AccessibilityEvent::PagePartsLoaded`]): spliced into the page (the one waiting to be
    /// applied, if any), which is rebuilt once and applied as a page captured again, keeping the
    /// cursor's place. A part the page does not hold is new: the whole page is read again.
    pub(super) fn page_parts_loaded(&mut self, parts: &[AomNode]) {
        if self.app_mode != AppMode::VirtualBuffer {
            return;
        }
        let base = self.pending_page.as_ref().unwrap_or(&self.cursor.buffer);
        match base.with_subtrees_replaced(parts) {
            Some(buffer) => {
                if let Some(old) = self.pending_page.replace(buffer) {
                    crate::retire::retire(old);
                }
            }
            None => {
                if let Some(actions) = &self.actions {
                    actions.page_changed(true);
                }
            }
        }
    }

    /// Captures a page that changed during forms mode, now that forms mode is over: at once,
    /// since its changes are.
    pub(super) fn refresh_stale_page(&mut self) {
        if !self.page_stale || self.forms_mode {
            return;
        }
        self.page_stale = false;
        if self.app_mode == AppMode::VirtualBuffer {
            if let Some(actions) = &self.actions {
                actions.page_changed(true);
            }
        }
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
            let object = |buffer: &VirtualBuffer, id: usize| {
                buffer
                    .node(id)
                    .and_then(|n| n.attributes.get(UNIQUE_ID_ATTR))
                    .cloned()
            };
            let place = line.map(|line| {
                (
                    line.text.clone(),
                    line.start_offset,
                    offset.saturating_sub(line.start_offset),
                    object(&self.cursor.buffer, line.node_id),
                )
            });
            let old_focus = self
                .last_page_focus
                .and_then(|id| self.cursor.buffer.node(id))
                .map(|n| {
                    (
                        n.attributes.get(UNIQUE_ID_ATTR).cloned(),
                        n.attributes.get("id").cloned(),
                        n.role.clone(),
                        n.name.clone(),
                    )
                });
            crate::retire::retire(std::mem::replace(&mut self.cursor.buffer, buffer));
            // Node ids are the capture's own: the focus is found again as the same object (IA2
            // captures), else by its DOM id, else as the focused node with its role and name,
            // else by its state alone. A page can
            // leave the state on more than its focus: Google Docs' menu bar is still "focused"
            // through IA2 after a menu closes, while the document has the keyboard (dev box,
            // 2026-10-16), and its document's field has no DOM id.
            let buffer = &self.cursor.buffer;
            self.last_page_focus = old_focus
                .as_ref()
                .and_then(|(object, ..)| object.as_deref())
                .and_then(|object| buffer.node_with_object(object))
                .or_else(|| {
                    let dom_id = old_focus.as_ref()?.1.as_deref()?;
                    buffer.node_with_dom_id(dom_id)
                })
                .or_else(|| {
                    let (_, _, role, name) = old_focus.as_ref()?;
                    buffer
                        .nodes()
                        .find(|n| n.states.focused && &n.role == role && &n.name == name)
                        .map(|n| n.id)
                })
                .or_else(|| buffer.focused_node());
            self.tab_start = self.last_page_focus;
            // A capture asked for a focus the old one did not hold puts the cursor on it.
            let to_focus = std::mem::take(&mut self.cursor_to_focus);
            let focus_line = self
                .last_page_focus
                .filter(|_| on_focus || to_focus)
                .and_then(|id| self.cursor.buffer.start_of(id));
            self.cursor.offset = match (focus_line, place) {
                (Some(start), _) => start,
                // The same object's line (IA2 captures): WhatsApp repeats lines ("Siz:") and
                // loads older messages above as it is read up, so the line with the same
                // text nearest the old offset can be another copy.
                (None, Some((text, start, column, object))) => object
                    .and_then(|object| same_object_line(&self.cursor.buffer, &object, &text))
                    .map(|(line_start, length)| line_start + column.min(length))
                    .or_else(|| {
                        same_line(&self.cursor.buffer, &text, start)
                            .map(|line_start| line_start + column)
                    })
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
        // A notification of just the character the last key typed is the application's echo
        // of it (Windows Terminal with the Turkish "ı"; dev box, 2026-10-17): typing echo says
        // typed characters, as the setting has it.
        let mut text = live.text.trim().chars();
        let notification = live.region_id.as_deref() == Some("uia-notification");
        if notification
            && self.last_typed.is_some()
            && text.next() == self.last_typed
            && text.next().is_none()
        {
            return None;
        }
        // While an edit key's result is being read in a field the application also announces,
        // and after it until the next key (a console redrawing the line after Backspace:
        // Windows Terminal said "c" after deleting "b"), the reading says it.
        if notification && self.edit_read_in_console {
            return None;
        }
        if notification
            && self.pending_edit.is_some()
            && self
                .last_native_focus
                .as_ref()
                .is_some_and(|field| self.app_for(field).is_some_and(|app| app.types_into(field)))
        {
            return None;
        }
        if live.politeness == Politeness::Off {
            // Kept current, never spoken.
            self.live_region_additions(live);
            return None;
        }
        // An atomic region filled again with the text it had says it again: Google Docs says
        // each character the caret passes over in one, and Left back over the one Right
        // passed is "o" again. What a change adds is read alone, as in any region: Docs adds a
        // deleted character to what it said last ("e", then "e r"), which JAWS reads whole,
        // the character at the caret with the one deleted (dev box, 2026-10-16).
        let (added, continues) = if live.atomic {
            let added = self.live_region_additions(live);
            if added.trim().is_empty() {
                (live.text.clone(), false)
            } else {
                (added, false)
            }
        } else {
            match self.settings.live_region_append {
                LiveRegionAppend::Appended => self.live_stream_piece(live),
                LiveRegionAppend::WholeText => (self.live_region_additions(live), false),
            }
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
        // The focus on an item's own focusable content, named after the item: WhatsApp's
        // chat, a grid cell whose inner group takes the focus (dev box, 2026-10-17).
        // Not its row, which has no line of its own.
        || (focus == &Role::Group
            && matches!(node, Role::TableCell | Role::ListItem | Role::TreeItem))
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

/// The line of the object with IA2 uniqueID `object`, preferring one with `text` (an object
/// can have several lines), as its start offset and length.
fn same_object_line(buffer: &VirtualBuffer, object: &str, text: &str) -> Option<(usize, usize)> {
    let id = buffer.node_with_object(object)?;
    let mut lines = buffer.lines.iter().filter(|line| line.node_id == id);
    let first = lines.next()?;
    let line = std::iter::once(first)
        .chain(lines)
        .find(|line| line.text == text)
        .unwrap_or(first);
    Some((
        line.start_offset,
        line.end_offset.saturating_sub(line.start_offset),
    ))
}

/// The nodes from `node` down to node `id`, without it, into `out`: its ancestors, outermost
/// first. False when `id` is not below `node`.
impl ScreenReaderEngine {
    /// Whether page node `id` is an item of a list, grid, table row, tab list or tree, or a
    /// plain element inside one: WhatsApp's chats are grid cells, its messages groups inside
    /// rows. A link, button or field inside an item is not: it is a control of its own.
    pub(super) fn in_widget_item(&self, id: usize) -> bool {
        let item = |role: &Role| {
            matches!(
                role,
                Role::TableCell | Role::TableRow | Role::ListItem | Role::TreeItem | Role::Tab
            )
        };
        let Some(node) = self.cursor.buffer.node(id) else {
            return false;
        };
        if item(&node.role) {
            return true;
        }
        if !matches!(
            node.role,
            Role::Group | Role::Generic | Role::Text | Role::Paragraph
        ) {
            return false;
        }
        let Some(root) = self
            .cursor
            .buffer
            .root_id
            .and_then(|r| self.cursor.buffer.node(r))
        else {
            return false;
        };
        let mut path = Vec::new();
        ancestors_of(root, id, &mut path) && path.iter().rev().take(8).any(|n| item(&n.role))
    }
}

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
