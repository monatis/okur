//! Pages: loading them, route changes, changes to their controls, selections, and live
//! regions.

use super::{AppMode, ScreenReaderEngine};
use crate::aom::{AomNode, Role};
use crate::buffer::VirtualBuffer;
use crate::event::{AccessibilityEvent, FocusEvent, LiveRegionEvent, Politeness};
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
        self.last_live_text.clear();
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
        self.cursor.buffer.node_map.get(&line.node_id).cloned()
    }

    /// A page control changed (Space toggled a check box, a combo box took a new value): the
    /// buffer is rebuilt with the new state, the virtual cursor stays on the control, and the
    /// change is spoken.
    fn page_change(&mut self, control: &FocusEvent) -> Option<String> {
        let id = self.page_node_for(control)?;
        let before = self.cursor.buffer.node_map.get(&id)?.clone();
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
            Role::ComboBox if before.value != control.value => control.value.clone()?,
            // Space on a tab selects it [obs, 29/s4].
            Role::Tab
                if before.states.selected != control.states.selected
                    && control.states.selected == Some(true) =>
            {
                crate::linearize::state_marker("selected")
            }
            _ => return None,
        };
        let rebuilt = self.cursor.buffer.with_node_changed(id, |node| {
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
    pub(super) fn region_markers(&self, node_id: usize) -> String {
        fn path<'a>(node: &'a AomNode, id: usize, out: &mut Vec<&'a AomNode>) -> bool {
            if node.id == id {
                return true;
            }
            out.push(node);
            if node.children.iter().any(|c| path(c, id, out)) {
                return true;
            }
            out.pop();
            false
        }
        let Some(root) = self
            .cursor
            .buffer
            .root_id
            .and_then(|r| self.cursor.buffer.node_map.get(&r))
        else {
            return String::new();
        };
        let mut ancestors = Vec::new();
        path(root, node_id, &mut ancestors);
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
        let map = &self.cursor.buffer.node_map;
        if let Some(id) = focus.node_id.filter(|id| map.contains_key(id)) {
            return Some(id);
        }
        let mut ids: Vec<usize> = map.keys().copied().collect();
        ids.sort_unstable();
        let node = |id: &usize| map.get(id);
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
                n.role == focus.role
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
            self.cursor.buffer = buffer;
            self.cursor.offset = offset.min(self.cursor.buffer.len());
            self.last_page_focus = None;
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
        let speech = self.menus.idle()?;
        self.speech.speak(&speech, true);
        Some(speech)
    }

    /// Handles an ARIA live region announcement event with politeness consideration.
    pub fn handle_live_region_event(&mut self, live: &LiveRegionEvent) -> Option<String> {
        // JAWS speaks what was added to the region, after the region's name: "Messages |
        // You: Message 1" [obs, 206/s2-s3]. The region's previous text is in the buffer.
        let added = self.live_region_additions(live);
        if added.trim().is_empty() {
            return None;
        }
        let text = match live.container_name.as_deref() {
            Some(name) => format!("{name}\n{added}"),
            None => added,
        };
        match live.politeness {
            Politeness::Assertive => {
                self.speech.stop();
                self.speech.speak(&text, true);
                Some(text)
            }
            Politeness::Polite => {
                self.speech.speak(&text, false);
                Some(text)
            }
            Politeness::Off => None,
        }
    }

    /// The lines of a live region's text that its previous text in the buffer lacked.
    fn live_region_additions(&mut self, live: &LiveRegionEvent) -> String {
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
        let map = &self.cursor.buffer.node_map;
        let mut ids: Vec<usize> = map.keys().copied().collect();
        ids.sort_unstable();
        let region = live.region_id.as_deref().and_then(|dom_id| {
            ids.iter().find_map(|id| {
                map.get(id)
                    .filter(|n| n.attributes.get("id").map(String::as_str) == Some(dom_id))
            })
        });
        let mut old = Vec::new();
        if let Some(region) = region {
            texts(region, &mut old);
        }
        // The region's text when it last changed, if newer than the page: the page is
        // refreshed after the change, so two quick changes (two chat messages) would otherwise
        // compare the second with a page that lacks the first.
        if let Some(last) = live
            .region_id
            .as_ref()
            .and_then(|id| self.last_live_text.get(id))
        {
            old = last.clone();
        }
        if let Some(id) = &live.region_id {
            self.last_live_text
                .insert(id.clone(), live.text.lines().map(str::to_owned).collect());
        }
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
