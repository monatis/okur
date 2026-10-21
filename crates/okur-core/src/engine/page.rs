//! Pages: loading them, route changes, changes to their controls, selections, and live
//! regions.

use super::{AppMode, ScreenReaderEngine};
use crate::aom::{AomNode, Role, UNIQUE_ID_ATTR};
use crate::buffer::VirtualBuffer;
use crate::event::{AccessibilityEvent, FocusEvent, LiveRegionEvent, Politeness};
use crate::messages::msg;
use crate::settings::LiveRegionAppend;
use crate::speech::SpeechFormatter;

/// Builds of the page after one of its nodes changed, made by the host off the engine
/// (performance review P3): building a big page's lines takes 100-160 ms at 25,000 nodes, and
/// the keyboard hook waits on the engine.
#[derive(Debug, Default)]
pub(super) struct PageBuilds {
    /// Pages of at least this many nodes are built by the host; `None`: every page here.
    from_nodes: Option<usize>,
    /// Counts the changes: a build made before the last one is stale.
    generation: u64,
    /// A change waits for the host to take the page and build it. The page is not held here:
    /// shared, the next change would copy all of it to change one node.
    wanted: bool,
    /// The page the host last took to build: its build is installed only while the buffer
    /// still holds it.
    base: std::sync::Weak<AomNode>,
}

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
    /// Whether a page captured with `title` and address `url` is the one already read: by its
    /// address, by its title when it has none.
    pub fn is_page_in_front(&self, title: &str, url: Option<&str>) -> bool {
        self.app_mode == AppMode::VirtualBuffer
            && match (self.cursor.buffer.document_url(), url) {
                (Some(old), Some(new)) => old == new,
                _ => self.cursor.buffer.document_title() == Some(title),
            }
    }

    /// A capture of the page already read, built without the engine (the live screen reader
    /// builds such refreshes on a thread of its own, so that the events after it are not
    /// held up): kept to be applied as [`Self::handle_document_loaded`] keeps it. Dropped
    /// when another page has been loaded meanwhile.
    pub fn handle_page_refreshed(&mut self, title: &str, buffer: VirtualBuffer) {
        // After Ctrl+R or F5 the page comes back as a refresh: it is a new load, announced
        // and read [obs, 507/s1; live9, where the reload came as "page refreshed"].
        if self.reload_asked && self.is_page_in_front(title, buffer.document_url()) {
            let _ = self.handle_document_loaded(title, buffer);
            return;
        }
        if self.is_page_in_front(title, buffer.document_url()) {
            if let Some(old) = self.pending_page.replace(buffer) {
                crate::retire::retire(old);
            }
        } else {
            crate::retire::retire(buffer);
        }
    }

    pub fn handle_document_loaded(&mut self, title: &str, buffer: VirtualBuffer) -> Option<String> {
        // The page already in the buffer, captured again (the first focus in it after
        // start-up): refresh it without announcing it or leaving forms mode.
        // A page is known by its address, by its title when it has none: a title can change
        // on the page (WhatsApp's counts unread chats, "(8) WhatsApp"), and JAWS keeps its
        // place then (dev box, 2026-10-17).
        // After Ctrl+R or F5 the same page is loaded anew, and announced [obs, 507/s1].
        let reloaded = std::mem::take(&mut self.reload_asked);
        if self.is_page_in_front(title, buffer.document_url()) && !reloaded {
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
        self.cursor_moved_since_load = false;
        self.field_texts.clear();
        self.focus_table_unsaid = false;
        self.tab_start = None;
        self.option_list = None;
        self.web_application = None;
        self.last_live_text.clear();
        self.live_streams.clear();
        self.load_buffer(buffer);
        self.app_mode = AppMode::VirtualBuffer;
        self.last_window_title = Some(title.to_owned());
        // JAWS on a page load: the page's title, its summary, then Say All from the top, its
        // "Automatically read pages when loaded" being on by default [obs, 507/s1, 538/s1,
        // 539/s1]. JAWS's Smart Glance sentence after the summary is not Okur's.
        let said = [
            format!("{title}  \u{1d} {} \u{1e}", msg::role_document()),
            self.page_summary(),
        ];
        for (i, part) in said.iter().enumerate() {
            self.speech.speak(part, i == 0);
        }
        let reading = self.start_page_say_all().unwrap_or_default();
        Some(
            said.into_iter()
                .chain(std::iter::once(reading))
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }

    /// The page's summary as JAWS says it when the page loads: "Page has 5Regions, 5headings and
    /// 12links", "Page has 1heading and no links" with no regions [obs, 538/s1, 539/s1].
    pub(super) fn page_summary(&self) -> String {
        fn count(node: &AomNode, counts: &mut (usize, usize, usize)) {
            match node.role {
                Role::Landmark { .. } => counts.0 += 1,
                Role::Heading { .. } => counts.1 += 1,
                Role::Link { .. } => counts.2 += 1,
                _ => {}
            }
            for child in &node.children {
                count(child, counts);
            }
        }
        let mut counts = (0, 0, 0);
        if let Some(page) = self.cursor.buffer.page() {
            count(page, &mut counts);
        }
        let (regions, headings, links) = counts;
        let headings = msg::page_load_headings(headings).to_string();
        let links = msg::page_load_links(links).to_string();
        if regions == 0 {
            msg::page_load_summary_two(headings, links).to_string()
        } else {
            msg::page_load_summary_three(
                msg::page_load_regions(regions).to_string(),
                headings,
                links,
            )
            .to_string()
        }
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
        let before = self.cursor.buffer.node(id)?;
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
            // A select box opened from the focus it already had (Space after Enter closed its
            // list): "expanded", said with its list as the list takes the focus [obs, 276/s7].
            // Opened from the virtual cursor, the focus comes to it with the same key and only
            // its list is said [obs, 276/s2, 229/s21].
            Role::ComboBox
                if control.states.expanded == Some(true) && self.focus_at_key == Some(id) =>
            {
                self.select_opened = true;
                return None;
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
        // Said first: building the page again takes long on a big page (116 ms with 25,000
        // nodes; responsiveness review, 2026-10-19), and the change is known already.
        self.speech.speak(&speech, true);
        let changed = self.change_page_node(id, |node| {
            node.states.expanded = control.states.expanded.or(node.states.expanded);
            node.states.checked = control.states.checked;
            node.states.selected = control.states.selected;
            if control.value.is_some() {
                node.value = control.value.clone();
            }
        });
        if !changed {
            return None;
        }
        self.cursor.move_to_node_id(id);
        Some(speech)
    }

    /// Where the virtual cursor is, as Say Window Title adds it after the title (Insert+T):
    /// the innermost region or article around the cursor by its first line ("article",
    /// "Related stories complementary information"), its level (the page is 1, a region in it
    /// 2, an article in the main region 3), and the last heading in it up to the cursor,
    /// "heading is What changes". Tables, lists and sections without a name are not counted;
    /// outside every region there is nothing [obs, 624, 625].
    pub(super) fn title_context(&self) -> Vec<crate::announcement::Line> {
        let buffer = &self.cursor.buffer;
        let Some(line) = buffer.line_at_offset(self.cursor.offset) else {
            return Vec::new();
        };
        let Some(root) = buffer.root_id.and_then(|r| buffer.node(r)) else {
            return Vec::new();
        };
        let mut ancestors = Vec::new();
        if !ancestors_of(root, line.node_id, &mut ancestors) {
            return Vec::new();
        }
        let is_region = |node: &AomNode| {
            matches!(node.role, Role::Landmark { .. })
                || (node.role == Role::Group
                    && node.attributes.get("aria_role").map(String::as_str) == Some("article"))
        };
        let mut regions: Vec<usize> = ancestors
            .iter()
            .filter(|n| is_region(n))
            .map(|n| n.id)
            .collect();
        if buffer.node(line.node_id).is_some_and(is_region) {
            regions.push(line.node_id);
        }
        let Some(&innermost) = regions.last() else {
            return Vec::new();
        };
        let Some(start) = buffer.start_of(innermost) else {
            return Vec::new();
        };
        let Some(name) = buffer.line_at_offset(start).map(|l| l.text.clone()) else {
            return Vec::new();
        };
        use crate::announcement::{Line, PartKind};
        // Short messages (Intermediate, Advanced) say the level and the heading alone:
        // "article | 3 | What changes" [obs, 636/s7, 637/s7].
        let level = regions.len() + 1;
        let mut said = vec![
            Line::of(PartKind::Message, name),
            Line::of(
                PartKind::Message,
                msg::title_region_level(level).to_string(),
            )
            .short(level.to_string()),
        ];
        let heading = buffer
            .headings
            .iter()
            .rfind(|&&(at, _)| at >= start && at <= self.cursor.offset)
            .and_then(|&(at, _)| buffer.line_at_offset(at))
            .map(|l| l.text.clone());
        if let Some(heading) = heading {
            said.push(
                Line::of(
                    PartKind::Message,
                    msg::title_heading_is(heading.as_str()).to_string(),
                )
                .short(heading),
            );
        }
        said
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
        same_roles(&focus.role)
            .iter()
            .filter_map(|role| buffer.node_with_role_and_name(role, name))
            .min()
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
        let speech = self
            .said(crate::native_focus::change_speech(&before, &control)?)
            .render_all();
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
        let field = self.last_native_focus.as_ref()?;
        if field.role != Role::Edit {
            return None;
        }
        // An edit combo's list: Up and Down put the item they select in the field, whose line
        // is then read with the item's position [obs, 541/s3, s4]. The dialog's other lists
        // follow it, selecting their own items, which are not the field's text.
        if field.container_role == Some(Role::ComboBox) {
            if let (Some(name), Some(index)) = (&item.name, item.item_index) {
                self.combo_items.push((name.trim().to_owned(), index));
            }
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

    /// The page that parts read again are spliced into (the one waiting to be applied, if
    /// any), for the live screen reader to splice and build without holding the engine. `None`
    /// outside a page.
    pub fn page_for_parts(&self) -> Option<std::sync::Arc<AomNode>> {
        if self.app_mode != AppMode::VirtualBuffer {
            return None;
        }
        self.pending_page
            .as_ref()
            .unwrap_or(&self.cursor.buffer)
            .page()
            .cloned()
    }

    /// Takes `buffer`, built from `base` with page parts spliced in, as the page waiting to be
    /// applied: as [`Self::page_parts_loaded`] would have made it. False, and `buffer` is
    /// dropped, when the page is another by now: the parts are then handled here.
    pub fn install_page_parts(
        &mut self,
        base: &std::sync::Arc<AomNode>,
        buffer: VirtualBuffer,
    ) -> bool {
        let same = self
            .page_for_parts()
            .is_some_and(|page| std::sync::Arc::ptr_eq(&page, base));
        if !same {
            crate::retire::retire(buffer);
            return false;
        }
        if let Some(old) = self.pending_page.replace(buffer) {
            crate::retire::retire(old);
        }
        true
    }

    /// Parts spliced without the engine into `base` were not all in it: the whole page is read
    /// again, as [`Self::page_parts_loaded`] does, without splicing them here once more. False
    /// when the page is another by now: the parts may be in that one.
    pub fn page_parts_not_found(&mut self, base: &std::sync::Arc<AomNode>) -> bool {
        let same = self
            .page_for_parts()
            .is_some_and(|page| std::sync::Arc::ptr_eq(&page, base));
        if same {
            if let Some(actions) = &self.actions {
                actions.page_changed(true);
            }
        }
        same
    }

    /// The host builds pages of at least `from_nodes` nodes after one of their nodes changed,
    /// off the engine ([`Self::take_page_build`]); smaller pages are built here, at once. The
    /// live screen reader does; replay and tests build every page here.
    pub fn set_page_builds_elsewhere(&mut self, from_nodes: usize) {
        self.page_builds.from_nodes = Some(from_nodes);
    }

    /// Page node `id` changed (a check box toggled, a combo box's value or name): the buffer's
    /// node takes the change at once, and the page's lines are built again, by the host on a
    /// big page, where they keep the old state until its build is installed; else here.
    /// False when the page has no such node.
    pub(super) fn change_page_node(
        &mut self,
        id: usize,
        change: impl FnOnce(&mut AomNode),
    ) -> bool {
        let by_host = self.pending_page.is_none()
            && self
                .page_builds
                .from_nodes
                .is_some_and(|from| self.cursor.buffer.node_count() >= from);
        if by_host {
            let Some(node) = self.cursor.buffer.node_mut(id) else {
                return false;
            };
            change(node);
            self.page_builds.generation += 1;
            self.page_builds.wanted = true;
            return true;
        }
        let Some(rebuilt) = self.cursor.buffer.with_node_changed(id, change) else {
            return false;
        };
        crate::retire::retire(std::mem::replace(&mut self.cursor.buffer, rebuilt));
        // Line numbers changed: a Say All stops following.
        self.say_all = None;
        true
    }

    /// The page a change asked the host to build, with the change's generation, for
    /// [`Self::install_page_build`]. The host copies the page before it builds, and drops
    /// it: while it holds it, a change copies the whole page to change one node.
    pub fn take_page_build(&mut self) -> Option<(u64, std::sync::Arc<AomNode>)> {
        if !std::mem::take(&mut self.page_builds.wanted) {
            return None;
        }
        let page = self.cursor.buffer.page()?.clone();
        self.page_builds.base = std::sync::Arc::downgrade(&page);
        Some((self.page_builds.generation, page))
    }

    /// Takes `buffer`, built for generation `generation`, as the page waiting to be applied
    /// (at the next key or quiet moment). False, and `buffer` is dropped, when a later change,
    /// another page or a page read again came meanwhile.
    pub fn install_page_build(&mut self, generation: u64, buffer: VirtualBuffer) -> bool {
        let current = generation == self.page_builds.generation
            && self.pending_page.is_none()
            && self
                .page_builds
                .base
                .upgrade()
                .zip(self.cursor.buffer.page())
                .is_some_and(|(base, page)| std::sync::Arc::ptr_eq(&base, page));
        if !current {
            crate::retire::retire(buffer);
            return false;
        }
        self.pending_page = Some(buffer);
        true
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
            let buffer = self.labeled(buffer);
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
            // Line numbers changed: a Say All stops following.
            self.say_all = None;
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
        // The region by its DOM id, from the buffer's index (the smallest id with it): a
        // sorted scan of every node did this once per change (performance review, 2026-10-20,
        // P9).
        let buffer = &self.cursor.buffer;
        let region = live
            .region_id
            .as_deref()
            .and_then(|dom_id| buffer.node_with_dom_id_indexed(dom_id))
            .and_then(|id| buffer.node(id));
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

/// The roles a page node of a focus event may have, to find it by role and name: its own; a
/// button opening a menu is a menu button to MSAA, which focus events follow, and a plain
/// button to UIA, which a page's quick first capture reads; and the focus on an item's own
/// focusable content, named after the item (WhatsApp's chat, a grid cell whose inner group
/// takes the focus; dev box, 2026-10-17), not its row, which has no line of its own.
fn same_roles(focus: &Role) -> Vec<Role> {
    let mut roles = vec![focus.clone()];
    match focus {
        Role::Button => roles.push(Role::MenuButton),
        Role::MenuButton => roles.push(Role::Button),
        Role::Group => roles.extend([Role::TableCell, Role::ListItem, Role::TreeItem]),
        _ => {}
    }
    roles
}

/// Whether a focus event's role is the page node's, as [`same_roles`] lists them: the test's
/// oracle for it.
#[cfg(test)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aom::AomNode;

    /// The roles [`same_roles`] lists are those [`same_role`] accepts.
    #[test]
    fn the_roles_listed_are_the_roles_accepted() {
        let roles = [
            Role::Button,
            Role::MenuButton,
            Role::Group,
            Role::TableCell,
            Role::ListItem,
            Role::TreeItem,
            Role::TableRow,
            Role::CheckBox,
            Role::Link { href: None },
        ];
        for focus in &roles {
            for node in &roles {
                assert_eq!(
                    same_roles(focus).contains(node),
                    same_role(node, focus),
                    "{node:?} for {focus:?}"
                );
            }
        }
    }

    /// A page node found by role and name is the one scanning the nodes found: the smallest
    /// id, ASCII letters in either case, by name or else by text, of a node the buffer reads.
    #[test]
    fn a_node_is_found_by_role_and_name_as_the_scan_found_it() {
        let page = AomNode::new(0, Role::Document)
            .with_name("Page")
            .with_children(vec![
                AomNode::new(9, Role::Button).with_name("Send"),
                AomNode::new(3, Role::MenuButton).with_name("SEND"),
                AomNode::new(5, Role::ListItem).with_text("Chat with Ada"),
                AomNode::new(7, Role::Group).with_children(vec![
                    AomNode::new(8, Role::TableCell).with_name("chat with ada"),
                    AomNode::new(4, Role::Link { href: None }).with_name("Ölçü"),
                ]),
            ]);
        let buffer = VirtualBuffer::from_aom(&page);
        let scan = |role: &Role, name: &str| {
            let mut ids: Vec<usize> = buffer.node_ids().collect();
            ids.sort_unstable();
            ids.into_iter().find(|id| {
                buffer.node(*id).is_some_and(|n| {
                    same_role(&n.role, role)
                        && n.name
                            .as_deref()
                            .or(n.text.as_deref())
                            .is_some_and(|nn| nn.eq_ignore_ascii_case(name))
                })
            })
        };
        for (role, name) in [
            (Role::Button, "send"),
            (Role::MenuButton, "Send"),
            (Role::Group, "CHAT WITH ADA"),
            (Role::Link { href: None }, "ölçü"),
            (Role::Link { href: None }, "Ölçü"),
            (Role::CheckBox, "Send"),
        ] {
            let found = same_roles(&role)
                .iter()
                .filter_map(|r| buffer.node_with_role_and_name(r, name))
                .min();
            assert_eq!(found, scan(&role, name), "{role:?} {name}");
        }
    }
}
