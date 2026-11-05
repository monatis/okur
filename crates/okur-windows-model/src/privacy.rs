//! Keeps people's own data out of what the harness records. The corpus is public, and the
//! reference machine's browser may show its user's account: Chrome's new tab page named the
//! signed-in Google account, with its e-mail address, in a trace of scenario 220 (2026-10-19).

/// Whether `event` is from a web view the focus is not in: Windows' taskbar news feed renders
/// in one (msedgewebview2.exe), and its headlines and links changed in a trace of scenario 1033
/// while File Explorer had the focus (round 298). That is the internet's content, never a
/// fixture's; a web view app in front has its focus chain, and is kept.
pub fn internet_feed(event: &crate::event_trace::TraceEvent) -> bool {
    event.process.as_deref() == Some("msedgewebview2.exe") && event.focus_chain.is_empty()
}

/// Whether a change event's object is a list item neither focused nor selected: a list's row
/// changing as the list draws it. Windows Settings' Apps & features reports one for each app it
/// shows as it opens, and a trace of scenario 3155 held the reference machine's installed
/// programs (2026-11-02). Nothing reads them: the engine takes a change only for the focus, and
/// live they are dropped before they are read.
pub fn unvisited_list_item(object: &crate::event_trace::TraceObject) -> bool {
    const ROLE_LISTITEM: i32 = 0x22;
    const STATE_SELECTED: u32 = 0x2;
    const STATE_FOCUSED: u32 = 0x4;
    object.role == Some(ROLE_LISTITEM) && object.states & (STATE_SELECTED | STATE_FOCUSED) == 0
}

/// Whether a mouse read (where the mouse rests, read as a trace starts) is over a list's row
/// that is not selected: the reference machine's installed programs, in a trace of scenario
/// 3156 (round-ui2a, 2026-11-03), where Programs and Features lay under the mouse. A row
/// selected is one a scenario visits, and stays.
pub fn unvisited_row_under_mouse(event: &crate::event_trace::TraceEvent) -> bool {
    const UIA_LISTITEM: i32 = 50007;
    const UIA_DATAITEM: i32 = 50029;
    event.point.is_some()
        && event.focus_chain.iter().any(|node| {
            (matches!(node.control_type, UIA_LISTITEM | UIA_DATAITEM)
                || node
                    .automation_id
                    .as_deref()
                    .is_some_and(|id| id.starts_with("ListViewSubItem")))
                && node.is_selected != Some(true)
        })
}

/// `event` without the names of the items beside the focus in JAWS's Dictionary Manager
/// (JDiction.exe): its tree shows the dictionary JAWS ships, which is JAWS's data, not a
/// fixture's ("paciello=Pah see ello", in the traces of 1154's first run, 2026-11-05). The items
/// stay, so their number and the focus's place among them are kept, and the focused one keeps
/// its name: the entry a scenario added.
pub fn without_shipped_dictionary(event: &mut crate::event_trace::TraceEvent) {
    const UIA_TREEITEM: i32 = 50024;
    if event.process.as_deref() != Some("JDiction.exe") {
        return;
    }
    for sibling in &mut event.focus_siblings {
        if sibling.control_type == UIA_TREEITEM && !sibling.has_keyboard_focus {
            sibling.name = None;
        }
    }
}

/// `event` without JAWS's own help text and the lab machine's paths that its viewers carry:
/// - a name that is a page address JAWS's viewers show their text through: the Results
///   Viewer's `data:` page holds a whole help screen (the Insert+Space layer's, 628's first
///   runs), Command Search's `serve.html?` page the machine's folders;
/// - in Command Search (CommandsSearchViewer.exe, its window "Search for JAWS Commands"), the
///   names beside the focus: its list of JAWS's commands and their descriptions, JAWS's data.
///   The items stay, so their number and the focus's place among them are kept.
pub fn without_jaws_help_text(event: &mut crate::event_trace::TraceEvent) {
    fn address(name: &Option<String>) -> bool {
        name.as_deref()
            .is_some_and(|n| n.starts_with("data:") || n.contains("serve.html?"))
    }
    fn clean(node: &mut crate::uia_snapshot::UiaNode, siblings: bool) {
        if address(&node.name) || (siblings && !node.has_keyboard_focus) {
            node.name = None;
            node.value = None;
            node.full_description = None;
            node.help_text = None;
        }
        for child in &mut node.children {
            clean(child, siblings);
        }
    }
    let search = event.process.as_deref() == Some("CommandsSearchViewer.exe")
        || event
            .window_title
            .as_deref()
            .is_some_and(|t| t.starts_with("Search for JAWS Commands"));
    if address(&event.window_title) {
        event.window_title = None;
    }
    if let Some(object) = event.object.as_mut() {
        if address(&object.name) {
            object.name = None;
        }
    }
    for node in &mut event.focus_chain {
        clean(node, false);
    }
    for node in &mut event.focus_siblings {
        clean(node, search);
    }
    if let Some(subtree) = event.focus_subtree.as_mut() {
        clean(subtree, search);
    }
}

/// What an e-mail address is replaced with.
pub const ADDRESS_MARK: &str = "<address>";

/// `text` with every e-mail address replaced by `<address>`, except addresses at the domains
/// reserved for examples (RFC 2606: `example.com`, `example.net`, `example.org`, and names
/// under `.example`, `.test`, `.invalid`, `.localhost`), which the fixtures use and replay
/// must see as they are.
pub fn redact_addresses(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        if c == '@' {
            let local = out.chars().rev().take_while(|c| is_local(*c)).count();
            let domain: String = chars
                .iter()
                .skip(i + 1)
                .take_while(|c| is_domain(**c))
                .collect();
            // A sentence's period after the address is not part of it.
            let domain = domain.trim_end_matches('.');
            if local > 0 && domain.contains('.') && !domain.starts_with('.') && !reserved(domain) {
                for _ in 0..local {
                    out.pop();
                }
                out.push_str(ADDRESS_MARK);
                i += 1 + domain.chars().count();
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

fn is_local(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '%' | '+' | '-')
}

fn is_domain(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '-')
}

fn reserved(domain: &str) -> bool {
    let domain = domain.to_ascii_lowercase();
    ["example.com", "example.net", "example.org"]
        .iter()
        .any(|d| domain == *d || domain.ends_with(&format!(".{d}")))
        || [".example", ".test", ".invalid", ".localhost"]
            .iter()
            .any(|tld| domain.ends_with(tld))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The taskbar's news feed in its web view is left out; a web view the focus is in stays.
    #[test]
    fn a_web_view_out_of_focus_is_left_out() {
        let feed = r#"{"t_ms":1,"event":"description_change","win_event":32781,"hwnd":1,"id_object":-4,"id_child":-7,"process":"msedgewebview2.exe","object":{"role":30,"name":"Markets","value":"https://www.example.com/markets","states":0}}"#;
        let event: Option<crate::event_trace::TraceEvent> = serde_json::from_str(feed).ok();
        assert_eq!(event.as_ref().map(internet_feed), Some(true));
        let focused = feed.replace(
            r#""process""#,
            r#""focus_chain":[{"control_type":50030,"localized_control_type":"document","name":"App","runtime_id":[42,1],"is_enabled":true,"is_offscreen":false,"is_password":false,"is_required_for_form":false,"is_keyboard_focusable":true,"has_keyboard_focus":true,"is_control_element":true,"is_content_element":true}],"process""#,
        );
        let event: Option<crate::event_trace::TraceEvent> = serde_json::from_str(&focused).ok();
        assert_eq!(event.as_ref().map(internet_feed), Some(false));
    }

    /// In JAWS's Dictionary Manager the tree's items beside the focus lose their names, JAWS's
    /// own entries; the focused entry, the dialog's other controls and other programs keep
    /// theirs.
    #[test]
    fn jaws_shipped_dictionary_is_left_out() {
        let node = |control_type: i32, name: &str, focused: bool| {
            format!(
                r#"{{"control_type":{control_type},"name":"{name}","is_enabled":true,"is_offscreen":false,"is_password":false,"is_required_for_form":false,"is_keyboard_focusable":true,"has_keyboard_focus":{focused},"is_control_element":true,"is_content_element":true}}"#
            )
        };
        let line = |process: &str| {
            format!(
                r#"{{"t_ms":1,"event":"focus","win_event":32773,"hwnd":1,"id_object":-4,"id_child":0,"process":"{process}","focus_siblings":[{},{},{}]}}"#,
                node(50024, "paciello=Pah see ello", false),
                node(50024, "total=sum", true),
                node(50000, "Delete", false)
            )
        };
        let names = |process: &str| {
            let event: Option<crate::event_trace::TraceEvent> =
                serde_json::from_str(&line(process)).ok();
            event
                .map(|mut event| {
                    without_shipped_dictionary(&mut event);
                    event.focus_siblings
                })
                .unwrap_or_default()
                .into_iter()
                .map(|s| s.name)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            names("JDiction.exe"),
            [
                None,
                Some("total=sum".to_owned()),
                Some("Delete".to_owned())
            ]
        );
        assert_eq!(
            names("notepad.exe").first(),
            Some(&Some("paciello=Pah see ello".to_owned()))
        );
    }

    /// JAWS's help text and Command Search's list leave the trace: a viewer's page address (a
    /// help screen in a `data:` page, the machine's folders in `serve.html?`), and in Command
    /// Search the names beside the focus; the focused result and other programs keep theirs.
    #[test]
    fn jaws_help_text_is_left_out() {
        let node = |name: &str, focused: bool| {
            format!(
                r#"{{"control_type":50007,"name":"{name}","is_enabled":true,"is_offscreen":false,"is_password":false,"is_required_for_form":false,"is_keyboard_focusable":true,"has_keyboard_focus":{focused},"is_control_element":true,"is_content_element":true}}"#
            )
        };
        let line = |process: &str| {
            format!(
                r#"{{"t_ms":1,"event":"focus","win_event":32773,"hwnd":1,"id_object":-4,"id_child":0,"process":"{process}","focus_chain":[{}],"focus_siblings":[{},{}]}}"#,
                node("data:text/html;base64,PGgyPkdlbmVyYWw=", false),
                node("Says the time.", false),
                node("Say Font", true)
            )
        };
        let read = |process: &str| {
            let event: Option<crate::event_trace::TraceEvent> =
                serde_json::from_str(&line(process)).ok();
            event.map(|mut event| {
                without_jaws_help_text(&mut event);
                (
                    event
                        .focus_chain
                        .into_iter()
                        .map(|n| n.name)
                        .collect::<Vec<_>>(),
                    event
                        .focus_siblings
                        .into_iter()
                        .map(|n| n.name)
                        .collect::<Vec<_>>(),
                )
            })
        };
        assert_eq!(
            read("CommandsSearchViewer.exe"),
            Some((vec![None], vec![None, Some("Say Font".to_owned())]))
        );
        assert_eq!(
            read("notepad.exe"),
            Some((
                vec![None],
                vec![
                    Some("Says the time.".to_owned()),
                    Some("Say Font".to_owned())
                ]
            ))
        );
    }

    /// A list's rows drawn are left out; its focused and selected items stay, as does
    /// anything not a list item.
    #[test]
    fn a_lists_rows_drawn_are_left_out() {
        let item = |role: i32, states: u32| crate::event_trace::TraceObject {
            role: Some(role),
            name: Some("Some Program 36.1 MB".to_owned()),
            value: None,
            states,
            shortcut: None,
            location: None,
        };
        // Selectable, focusable, collapsed (Settings' apps as the page opens).
        assert!(unvisited_list_item(&item(0x22, 0x0030_0400)));
        assert!(!unvisited_list_item(&item(0x22, 0x0030_0402)));
        assert!(!unvisited_list_item(&item(0x22, 0x0030_0404)));
        assert!(!unvisited_list_item(&item(0x2B, 0x0010_0000)));
    }

    /// A mouse read over a list's row nobody selected is left out; over a selected row, or
    /// over anything else, it stays.
    #[test]
    fn a_mouse_read_over_an_unvisited_row_is_left_out() {
        let read = r#"{"t_ms":1,"event":"mouse_point","win_event":0,"hwnd":0,"id_object":0,"id_child":0,"focus_chain":[{"control_type":50020,"localized_control_type":"text","name":"Some Publisher","automation_id":"ListViewSubItem-1","is_enabled":true,"is_offscreen":false,"is_password":false,"is_required_for_form":false,"is_keyboard_focusable":true,"has_keyboard_focus":false,"is_control_element":false,"is_content_element":false}],"point":{"at":[960,540],"texts":{}}}"#;
        let event: Option<crate::event_trace::TraceEvent> = serde_json::from_str(read).ok();
        assert_eq!(event.as_ref().map(unvisited_row_under_mouse), Some(true));
        let selected = read.replace(
            r#""is_enabled":true"#,
            r#""is_selected":true,"is_enabled":true"#,
        );
        let event: Option<crate::event_trace::TraceEvent> = serde_json::from_str(&selected).ok();
        assert_eq!(event.as_ref().map(unvisited_row_under_mouse), Some(false));
        let overlay = read.replace(r#","automation_id":"ListViewSubItem-1""#, "");
        let event: Option<crate::event_trace::TraceEvent> = serde_json::from_str(&overlay).ok();
        assert_eq!(event.as_ref().map(unvisited_row_under_mouse), Some(false));
    }

    #[test]
    fn real_addresses_go_and_example_ones_stay() {
        assert_eq!(
            redact_addresses(
                r#"{"name":"Google Account: A. Person (a.person+x@mail.co.uk), Member"}"#
            ),
            r#"{"name":"Google Account: A. Person (<address>), Member"}"#
        );
        assert_eq!(
            redact_addresses("Mail ada@example.org or sam@example.com."),
            "Mail ada@example.org or sam@example.com."
        );
        assert_eq!(
            redact_addresses("Write to me@host.net."),
            "Write to <address>."
        );
        assert_eq!(
            redact_addresses("a@b, @handle, x@localhost"),
            "a@b, @handle, x@localhost"
        );
        assert_eq!(redact_addresses("dev@app.test"), "dev@app.test");
        assert_eq!(redact_addresses("ışık çağ"), "ışık çağ");
    }
}
