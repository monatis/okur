use super::*;

const RADIO: i32 = 50013;
const GROUP: i32 = 50026;
const TEXT: i32 = 50020;

fn node(control_type: i32, name: &str, focused: bool) -> UiaNode {
    UiaNode {
        control_type,
        name: Some(name.to_owned()),
        has_keyboard_focus: focused,
        is_enabled: true,
        ..UiaNode::default()
    }
}

/// Say Window in Notepad's Font dialog, from its tree as the tracer reads it (a list's
/// shown items, a combo box's place, the sample by its control id): JAWS's words
/// [obs, 665/s3], but for the Size list's name, which JAWS says and the other lists'
/// it does not.
#[test]
fn say_window_reads_a_font_dialog_as_jaws_does() {
    const COMBOBOX: i32 = 50003;
    const EDIT: i32 = 50004;
    const LINK: i32 = 50005;
    const LISTITEM: i32 = 50007;
    const LIST: i32 = 50008;
    const SCROLLBAR: i32 = 50014;
    const TEXT: i32 = 50020;
    const GROUP: i32 = 50026;
    const PANE: i32 = 50033;
    let with = |mut n: UiaNode, children: Vec<UiaNode>| {
        n.children = children;
        n
    };
    let valued = |control_type: i32, name: &str, value: &str| UiaNode {
        value: Some(value.to_owned()),
        ..node(control_type, name, false)
    };
    let combo = |label: &str, value: &str, items: &[&str], place: (i32, i32)| {
        let item = |name: &&str| UiaNode {
            is_selected: Some(*name == value),
            ..node(LISTITEM, name, false)
        };
        let list = with(
            node(LIST, label, false),
            std::iter::once(with(
                node(SCROLLBAR, "Vertical", false),
                vec![node(50000, "Line down", false)],
            ))
            .chain(items.iter().map(item))
            .collect(),
        );
        UiaNode {
            position_in_set: Some(place.0),
            size_of_set: Some(place.1),
            ..with(
                valued(COMBOBOX, label, value),
                vec![list, valued(EDIT, label, value)],
            )
        }
    };
    let fonts = [
        "Consolas",
        "Constantia",
        "Cooper",
        "Copperplate Gothic",
        "Corbel",
        "Courier",
        "Courier New",
    ];
    let script = UiaNode {
        position_in_set: Some(1),
        size_of_set: Some(7),
        ..with(
            valued(COMBOBOX, "Script:", "Western"),
            vec![
                valued(TEXT, "Script:", "Western"),
                node(50000, "Open", false),
            ],
        )
    };
    let tree = with(
        node(50032, "Font", false),
        vec![
            node(TEXT, "Font:", false),
            UiaNode {
                automation_id: Some("1136".to_owned()),
                ..combo("Font:", "Consolas", &fonts, (38, 187))
            },
            node(TEXT, "Font style:", false),
            combo(
                "Font style:",
                "Regular",
                &["Regular", "Italic", "Bold", "Bold Italic"],
                (1, 4),
            ),
            node(GROUP, "Sample", false),
            UiaNode {
                automation_id: Some("1093".to_owned()),
                ..node(TEXT, "AaBbYyZz", false)
            },
            node(TEXT, "Script:", false),
            script,
            with(
                node(PANE, "Show more fonts", false),
                vec![node(LINK, "Show more fonts", false)],
            ),
            node(50000, "OK", false),
        ],
    );
    let said = okur_core::native_focus::say_window("Font", &window_items(&tree)).render();
    let words = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    assert_eq!(
        words(&said),
        words(
            "Font dialog\nFont: \nCombo box\nConsolas Consolas Constantia Cooper \
                 Copperplate Gothic Corbel Courier Courier New\n38 of 187\n \u{1d} List box \
                 \u{1e} \nConsolas\nFont:  \u{1d} edit combo \u{1e} \nConsolas\n38 of 187\n\
                 Font style: \nCombo box\nRegular Regular Italic Bold Bold Italic\n1 of 4\n \
                 \u{1d} List box \u{1e} \nRegular\nFont style:  \u{1d} edit combo \u{1e} \n\
                 Regular\n1 of 4\nSample  \u{1d} group \u{1e} \nScript: \nCombo box\n\
                 Western\n1 of 7\nShow more fonts  \u{1d} Link \u{1e} \nOK  \u{1d} Button \u{1e}"
        )
    );
}

/// Say Window in the Run dialog: its drop-down combo box is the box, collapsed, with its value,
/// its field and its button, with no place, its label said before it [obs, 3159/s9]. JAWS's
/// type words there are lowercase ("edit", "button"); that is still to do.
#[test]
fn say_window_reads_a_drop_down_combo_box_as_its_parts() {
    const COMBOBOX: i32 = 50003;
    const EDIT: i32 = 50004;
    const LIST: i32 = 50008;
    const TEXT: i32 = 50020;
    let valued = |control_type: i32, name: &str, value: &str| UiaNode {
        value: Some(value.to_owned()),
        ..node(control_type, name, false)
    };
    let combo = UiaNode {
        position_in_set: Some(1),
        size_of_set: Some(26),
        children: vec![
            valued(EDIT, "Open:", "C:"),
            node(UIA_BUTTON, "Open", false),
            node(LIST, "Open:", false),
        ],
        ..valued(COMBOBOX, "Open:", "C:")
    };
    let tree = UiaNode {
        children: vec![
            node(TEXT, "Open:", false),
            combo,
            node(UIA_BUTTON, "OK", false),
        ],
        ..node(UIA_WINDOW, "Run", false)
    };
    let said = okur_core::native_focus::say_window("Run", &window_items(&tree)).render();
    let words = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    assert_eq!(
        words(&said),
        words(
            "Run dialog\nOpen: \nOpen:  \u{1d} combo box \u{1e}  \u{1d}collapsed\u{1e} C: \n\
             Open:  \u{1d} Edit \u{1e} C: \nOpen  \u{1d} Button \u{1e} \nOK  \u{1d} Button \u{1e}"
        )
    );
}

/// A dialog's controls read for one focus are used again for the next while only the
/// focus moves among them; any other change in its process reads them anew.
#[test]
fn a_dialog_is_read_again_only_after_a_change() {
    for moved in ["focus", "uia_focus", "caret", "text_selection_change"] {
        assert!(keeps_dialog_read(moved, false), "{moved}");
    }
    // The default button follows the focus onto Find Next [obs, 540].
    assert!(keeps_dialog_read("state_change", true));
    // A check box toggled.
    assert!(!keeps_dialog_read("state_change", false));
    for changed in [
        "value_change",
        "selection",
        "selection_add",
        "name_change",
        "description_change",
        "dialog_start",
        "foreground",
    ] {
        assert!(!keeps_dialog_read(changed, false), "{changed}");
        assert!(!keeps_dialog_read(changed, true), "{changed}");
    }
}

/// A dialog's tree keeps its tree views' own items and drops what is inside them: a file
/// dialog's Quick access holds the account's recent folders (525, round 35).
#[test]
fn a_dialog_tree_keeps_no_nested_tree_items() {
    let mut quick_access = node(UIA_TREEITEM, "Quick access", false);
    quick_access.children = vec![node(UIA_TREEITEM, "recent folder", false)];
    let mut tree = node(50023, "Tree View", false);
    tree.children = vec![quick_access, node(UIA_TREEITEM, "This PC", false)];
    let mut dialog = node(50032, "Open", false);
    dialog.children = vec![tree, node(50000, "Open", false)];
    let pruned = without_nested_tree_items(dialog);
    let names = |n: &UiaNode| -> Vec<String> {
        fn walk(n: &UiaNode, out: &mut Vec<String>) {
            out.extend(n.name.clone());
            for c in &n.children {
                walk(c, out);
            }
        }
        let mut out = Vec::new();
        walk(n, &mut out);
        out
    };
    assert_eq!(
        names(&pruned),
        ["Open", "Tree View", "Quick access", "This PC", "Open"]
    );
}

/// A window's scroll bars are not traced: a scrolling list's state changes held the Run
/// dialog's focus back 5 s (319). The window's own changes are.
#[test]
fn scroll_bar_events_are_ignored() {
    assert_eq!(event_name(0x800A, -5), None);
    assert_eq!(event_name(0x800A, -6), None);
    assert_eq!(event_name(0x800A, -4), Some("state_change"));
    assert_eq!(event_name(0x800C, 0), Some("name_change"));
}

#[test]
fn siblings_are_needed_in_a_dialog_box_or_without_a_reported_position() {
    let window = |class: &str| UiaNode {
        class_name: Some(class.to_owned()),
        ..node(UIA_WINDOW, "Window", false)
    };
    let item = node(RADIO, "Option", true);
    let dialog = [window("#32770"), item.clone()];
    let main = [window("WindowsForms10.Window.8.app"), item.clone()];
    assert!(in_dialog_box(&dialog));
    assert!(!in_dialog_box(&main));

    assert!(!reports_position(&main));
    let placed = UiaNode {
        position_in_set: Some(2),
        size_of_set: Some(5),
        ..item
    };
    assert!(reports_position(&[window("Main"), placed]));
}

/// A live region's politeness and atomicity come from its ARIA properties, as Chromium
/// gives them: Google Docs' announcements are assertive and atomic (dev box, 2026-10-16).
#[test]
fn a_live_region_keeps_its_aria_properties() {
    let region = |attributes: &[(&str, &str)]| crate::ia2_snapshot::Ia2Node {
        attributes: attributes
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
        children: vec![crate::ia2_snapshot::Ia2Node {
            msaa_role: Some(0x29),
            name: Some("o".to_owned()),
            ..Default::default()
        }],
        ..Default::default()
    };
    let live = |region| {
        let mut event = focus_event(Vec::new());
        event.event = "live_region_change".to_owned();
        event.region = Some(region);
        match live_region_events(&event).first() {
            Some(okur_core::AccessibilityEvent::LiveRegion(live)) => Some(live.clone()),
            _ => None,
        }
    };
    let docs = live(region(&[
        ("id", "docs-aria-speakable"),
        ("live", "assertive"),
        ("atomic", "true"),
    ]));
    assert_eq!(
        docs.as_ref()
            .map(|l| (l.politeness, l.atomic, l.text.as_str())),
        Some((okur_core::Politeness::Assertive, true, "o"))
    );
    // A region inside a live region takes the container's properties.
    let inner = live(region(&[
        ("container-live", "polite"),
        ("container-atomic", "false"),
    ]));
    assert_eq!(
        inner.map(|l| (l.politeness, l.atomic)),
        Some((okur_core::Politeness::Polite, false))
    );
}

/// Windows Settings' Display page (UIA chains read on the dev box, 2026-10-16): the
/// groups a control is directly in, outermost first, a text counting as one; none when
/// they are inside a control (the slider in the "Show more settings" button).
#[test]
fn a_controls_groups_are_the_ones_it_nests_in() {
    use okur_core::GroupEntry;
    const UIA_GROUP: i32 = 50026;
    const UIA_PANE: i32 = 50033;
    const UIA_SLIDER: i32 = 50015;
    const UIA_LIST: i32 = 50008;
    const UIA_CUSTOM: i32 = 50025;
    let groups = |names: &[&str]| -> Vec<GroupEntry> {
        names.iter().map(|n| GroupEntry::group(*n)).collect()
    };
    // An unnamed WPF group box, named by its header [obs, 775/s6].
    let mut group_box = node(UIA_GROUP, "", false);
    group_box.name = None;
    group_box.label = Some("Received Notification".to_owned());
    let wpf = [group_box, node(50003, "Rule Type", true)];
    assert_eq!(
        nested_groups(&wpf),
        (groups(&["Received Notification"]), false)
    );
    let toggle = [
        node(UIA_PANE, "", false),
        node(UIA_GROUP, "Brightness & color", false),
        node(UIA_GROUP, "Night light", false),
        node(UIA_BUTTON, "Night light", true),
    ];
    assert_eq!(
        nested_groups(&toggle),
        (groups(&["Brightness & color", "Night light"]), false)
    );
    let link = [
        node(UIA_PANE, "", false),
        node(UIA_GROUP, "Brightness & color", false),
        node(UIA_GROUP, "HDR", false),
        node(UIA_TEXT, "More about HDR ", false),
        node(50005, "More about HDR", true),
    ];
    assert_eq!(
        nested_groups(&link),
        (
            groups(&["Brightness & color", "HDR", "More about HDR"]),
            false
        )
    );
    // Sound's output device: through its list and the scroll panes, up to the window's
    // navigation view.
    let device = [
        node(UIA_CUSTOM, "", false),
        node(UIA_PANE, "", false),
        node(UIA_GROUP, "Output", false),
        node(UIA_GROUP, "Choose where to play sound.", false),
        node(UIA_LIST, "", false),
        node(UIA_PANE, "", false),
        node(UIA_GROUP, "Hoparlör", false),
        node(UIA_BUTTON, "Hoparlör", true),
    ];
    let mut expected = groups(&["Output", "Choose where to play sound."]);
    expected.push(GroupEntry::list(""));
    expected.extend(groups(&["Hoparlör"]));
    assert_eq!(nested_groups(&device), (expected, false));
    let slider = [
        node(UIA_GROUP, "Brightness, Brightness", false),
        node(UIA_BUTTON, "Show more settings", false),
        node(UIA_GROUP, "Brightness", false),
        node(UIA_SLIDER, "Brightness", true),
    ];
    assert_eq!(nested_groups(&slider), (Vec::new(), true));
    // Sound's "Output devices" link, in its list item: the item's groups.
    let link_in_item = [
        node(UIA_GROUP, "Advanced", false),
        node(UIA_LIST, "", false),
        node(UIA_LISTITEM, "Output devices", false),
        node(50005, "Output devices", true),
    ];
    let mut expected = groups(&["Advanced"]);
    expected.push(GroupEntry::list(""));
    assert_eq!(nested_groups(&link_in_item), (expected, true));
    // Network's Wi-Fi switch, in a "Wi-Fi" group in its list item.
    let switch_in_item = [
        node(UIA_LIST, "", false),
        node(UIA_LISTITEM, "Wi-Fi", false),
        node(UIA_GROUP, "Wi-Fi", false),
        node(UIA_BUTTON, "Wi-Fi", true),
    ];
    assert_eq!(
        nested_groups(&switch_in_item),
        (vec![GroupEntry::list(""), GroupEntry::group("Wi-Fi")], true)
    );
    // Notifications' app switch, in the app's button, its group and the list: the list's
    // entry, as an item.
    let switch_in_entry = [
        node(
            UIA_GROUP,
            "Notifications from apps and other senders",
            false,
        ),
        node(UIA_LIST, "", false),
        node(UIA_PANE, "", false),
        node(UIA_GROUP, "Claude", false),
        node(UIA_BUTTON, "Claude", false),
        node(UIA_BUTTON, "Claude", true),
    ];
    let mut expected = groups(&["Notifications from apps and other senders"]);
    expected.push(GroupEntry::list(""));
    expected.extend(groups(&["Claude"]));
    assert_eq!(nested_groups(&switch_in_entry), (expected, true));
}

/// A native live region says the texts it holds, then its name; a text region its name.
#[test]
fn a_native_live_region_says_its_texts_then_its_name() {
    // Windows Settings' brightness expander, traced on the dev box (2026-10-17).
    const UIA_GROUP: i32 = 50026;
    const UIA_SLIDER: i32 = 50015;
    let mut group = node(UIA_GROUP, "Brightness", false);
    group.children = vec![
        node(UIA_TEXT, "Brightness", false),
        node(
            UIA_TEXT,
            "Change brightness for the built-in display",
            false,
        ),
        node(UIA_SLIDER, "Brightness", false),
    ];
    let mut expander = node(UIA_BUTTON, "Show more settings", false);
    expander.live_setting = Some(1);
    expander.children = vec![group];
    let live = uia_live_region_event(&expander).map(|l| (l.text, l.politeness));
    assert_eq!(
        live,
        Some((
            "Brightness Change brightness for the built-in display\nShow more settings".to_owned(),
            okur_core::Politeness::Polite
        ))
    );
    let mut status = node(UIA_TEXT, "Discoverable as \u{201c}PC\u{201d}", false);
    status.live_setting = Some(2);
    assert_eq!(
        uia_live_region_event(&status).map(|l| (l.text, l.politeness)),
        Some((
            "Discoverable as \u{201c}PC\u{201d}".to_owned(),
            okur_core::Politeness::Assertive
        ))
    );
    status.live_setting = Some(0);
    assert!(uia_live_region_event(&status).is_none());
}

/// A WPF item named after its data object's type is read by its texts [obs, 774/s3].
#[test]
fn a_wpf_items_type_name_is_left_out() {
    assert!(is_type_name("NotificationViewer.Services.NotificationData"));
    assert!(!is_type_name("example.com"));
    assert!(!is_type_name("Report.final"));
    assert!(!is_type_name("Notes"));
    assert!(!is_type_name("Mr. Smith"));
}

/// A XAML item named after its texts has no further columns; a list view's row does
/// [obs, 3155/s7].
#[test]
fn an_item_named_after_its_texts() {
    let texts = |t: &[&str]| t.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    assert!(names_its_texts(
        "7-Zip 18.05 (x64) 4.93 MB \u{200E}11/\u{200E}7/\u{200E}2020",
        &texts(&[
            "7-Zip 18.05 (x64)",
            "4.93 MB",
            "\u{200E}11/\u{200E}7/\u{200E}2020"
        ])
    ));
    assert!(!names_its_texts("notes.txt", &texts(&["2 KB"])));
    assert!(!names_its_texts("notes.txt 2 KB", &texts(&["2 KB"])));
    assert!(!names_its_texts("notes.txt", &[]));
}

/// An editable combo box's focus, read once the combo box gave the focus to its edit, is not
/// said while UI Automation still gives the combo box; its edit's focus follows. Read at the
/// edit, it is the edit's focus.
#[test]
fn an_editable_combo_boxs_focus_given_to_its_edit_is_the_edits() {
    let combo = node(UIA_COMBOBOX, "Open:", false);
    let mut event = focus_event(Vec::new());
    event.window_class = Some("ComboBox".to_owned());
    event.focus_window_class = Some("Edit".to_owned());
    event.object = Some(TraceObject {
        role: Some(0x2E),
        name: Some("Open:".to_owned()),
        value: Some("C:".to_owned()),
        states: 0x0010_0400,
        shortcut: None,
    });
    // UI Automation still gives the combo box: its edit's focus follows [obs, 3159/s6].
    event.focus_chain = vec![node(UIA_WINDOW, "Run", false), combo.clone()];
    assert!(EventConverter::default().convert(&event).is_empty());
    // The chain reaches the edit: this is the edit's focus [obs, 541/s5].
    event.focus_chain = vec![
        node(UIA_WINDOW, "Font", false),
        combo,
        node(UIA_EDIT, "Font style:", true),
    ];
    assert!(!EventConverter::default().convert(&event).is_empty());
}

/// A focus read after the keyboard focus had left its window, its object no longer
/// focused, is not said; the focus that follows is [obs, 3156/s6]. The same object still
/// focused is said.
#[test]
fn a_focus_that_moved_on_before_its_read_is_not_said() {
    let mut passed = focus_event(Vec::new());
    passed.object = Some(TraceObject {
        role: Some(0x21),
        name: Some("Folder View".to_owned()),
        value: None,
        states: 0x0010_0000,
        shortcut: None,
    });
    passed.focus_elsewhere = true;
    assert!(EventConverter::default().convert(&passed).is_empty());
    if let Some(object) = passed.object.as_mut() {
        object.states |= 0x4;
    }
    assert!(!EventConverter::default().convert(&passed).is_empty());
}

/// An item with a name of its own and a further text, outside a list view, has the text
/// as its description: Windows Settings' home [obs, 3157/s4]. In a list view it is a
/// column.
#[test]
fn an_items_further_text_outside_a_list_view_is_its_description() {
    let mut event = focus_event(Vec::new());
    let devices = node(UIA_LISTITEM, "Devices", true);
    event.focus_chain = vec![
        node(UIA_WINDOW, "Settings", false),
        node(UIA_LIST, "", false),
        devices.clone(),
    ];
    event.focus_subtree = Some(UiaNode {
        children: vec![
            node(UIA_TEXT, "Devices", false),
            node(UIA_TEXT, "Bluetooth, printers, mouse", false),
        ],
        ..devices
    });
    let focus = focus_event_from_trace(&event);
    assert_eq!(
        focus
            .as_ref()
            .map(|f| (f.description.as_deref(), f.columns.len())),
        Some((Some("Bluetooth, printers, mouse"), 0))
    );
    if let Some(list) = event.focus_chain.get_mut(1) {
        list.class_name = Some("SysListView32".to_owned());
    }
    let focus = focus_event_from_trace(&event);
    assert_eq!(
        focus.map(|f| (f.description, f.columns)),
        Some((None, vec!["Bluetooth, printers, mouse".to_owned()]))
    );
}

/// A toast, as the shell reports it opening: its view, then the window around it, once
/// said [obs, 770/s1].
#[test]
fn a_toast_is_said_once_by_its_views_name() {
    const TOAST: &str =
            "New notification from Windows PowerShell, SR Fixture, Build finished in 12 seconds. 1 of 1";
    let view = UiaNode {
        automation_id: Some("NormalToastView".to_owned()),
        children: vec![
            UiaNode {
                automation_id: Some("SenderName".to_owned()),
                ..node(UIA_TEXT, "Windows PowerShell", false)
            },
            UiaNode {
                automation_id: Some("TitleText".to_owned()),
                ..node(UIA_TEXT, "SR Fixture", false)
            },
        ],
        ..node(UIA_WINDOW, TOAST, false)
    };
    let window = UiaNode {
        children: vec![view.clone()],
        ..node(UIA_WINDOW, "New notification", false)
    };
    let mut converter = EventConverter::default();
    let mut opened = focus_event(Vec::new());
    opened.event = "uia_window_opened".to_owned();
    opened.focus_subtree = Some(view);
    assert_eq!(
        converter.convert(&opened),
        vec![okur_core::AccessibilityEvent::Toast {
            text: TOAST.to_owned(),
            sender: Some("Windows PowerShell".to_owned())
        }]
    );
    opened.t_ms += 13;
    opened.focus_subtree = Some(window);
    assert!(converter.convert(&opened).is_empty());
}

/// A closed WinUI combo box holds only its selected item: that is its value, not a
/// position (Windows Settings' "Scale Combo box | 125% (Recommended)").
#[test]
fn a_closed_combo_boxs_only_item_is_its_value() {
    let mut event = focus_event(Vec::new());
    let scale = node(UIA_COMBOBOX, "Scale", true);
    event.focus_chain = vec![node(UIA_WINDOW, "Settings", false), scale.clone()];
    event.focus_subtree = Some(UiaNode {
        children: vec![UiaNode {
            is_selected: Some(true),
            ..node(UIA_LISTITEM, "125% (Recommended)", false)
        }],
        ..scale
    });
    let focus = focus_event_from_trace(&event);
    assert_eq!(
        focus.map(|f| (f.value, f.item_index)),
        Some((Some("125% (Recommended)".to_owned()), None))
    );
}

/// A page's menu item focused through its document (Chrome, Google Docs' menus): the menu
/// item, as its MSAA focus names it.
#[test]
fn a_page_menu_item_focused_through_its_document_is_a_menu_item() {
    let mut event = focus_event(Vec::new());
    event.focus_chain = vec![
        node(UIA_DOCUMENT, "Untitled document - Google Docs", false),
        node(UIA_MENUITEM, "Graphics g", true),
    ];
    let focus = focus_event_from_trace(&event);
    assert_eq!(
        focus.map(|f| (f.role, f.name)),
        Some((okur_core::Role::MenuItem, Some("Graphics g".to_owned())))
    );
}

/// Windows Settings' Power page: its first link is in an InfoBar, an unnamed dialog named
/// by its first text (JAWS: "Power mode is set automatically ... dialog | More about power
/// mode Link").
#[test]
fn a_link_in_an_info_bar_has_the_bar_among_its_groups() {
    let mut event = focus_event(Vec::new());
    let bar = UiaNode {
        is_dialog: Some(true),
        label: Some("Power mode is set automatically.".to_owned()),
        ..node(50017, "", false)
    };
    event.focus_chain = vec![
        node(UIA_WINDOW, "Settings", false),
        node(50033, "", false),
        bar,
        node(50005, "More about power mode", true),
    ];
    let focus = focus_event_from_trace(&event);
    assert_eq!(
        focus.map(|f| f.groups),
        Some(vec![okur_core::GroupEntry::dialog(
            "Power mode is set automatically."
        )])
    );
}

/// A slider says the value it reports, else its range value, as a percentage from 0 to
/// 100: Windows Settings' brightness "100", volume "34%" (JAWS on the dev box).
#[test]
fn a_sliders_value_is_its_reported_value_or_its_percentage() {
    const UIA_SLIDER: i32 = 50015;
    let slider = |value: Option<&str>, now: f64| {
        let mut event = focus_event(Vec::new());
        event.focus_chain = vec![
            node(UIA_WINDOW, "Settings", false),
            UiaNode {
                value: value.map(str::to_owned),
                range_value: Some(now),
                range_minimum: Some(0.0),
                range_maximum: Some(100.0),
                ..node(UIA_SLIDER, "Volume", true)
            },
        ];
        focus_event_from_trace(&event).and_then(|f| f.value)
    };
    assert_eq!(slider(Some("100"), 100.0), Some("100".to_owned()));
    assert_eq!(slider(None, 34.0), Some("34%".to_owned()));
}

/// A UIA notification is said as an announcement, and the same one again within two
/// seconds once (Settings' "Suggestions available" for each letter typed).
#[test]
fn a_notification_is_said_once_while_it_repeats() {
    let notify = |t_ms: u128| {
        let mut event = focus_event(Vec::new());
        event.event = "uia_notification".to_owned();
        event.t_ms = t_ms;
        event.notification = Some(TraceNotification {
            text: "Suggestions available".to_owned(),
            processing: 3,
            process: Some("SystemSettings.exe".to_owned()),
        });
        event
    };
    let mut converter = EventConverter::default();
    let said = |events: Vec<okur_core::AccessibilityEvent>| match events.first() {
        Some(okur_core::AccessibilityEvent::AppNotification { text, app, .. }) => {
            assert_eq!(app.as_deref(), Some("SystemSettings"));
            Some(text.clone())
        }
        _ => None,
    };
    assert_eq!(
        said(converter.convert(&notify(1_000))),
        Some("Suggestions available".to_owned())
    );
    assert_eq!(said(converter.convert(&notify(1_500))), None);
    assert_eq!(
        said(converter.convert(&notify(4_000))),
        Some("Suggestions available".to_owned())
    );
}

/// Windows Settings' breadcrumb is a Button with a heading level: a button to focus, with
/// the position it reports. A button reporting none gets no position counted among its
/// siblings (a dialog's buttons, where JAWS says none).
#[test]
fn a_native_heading_button_is_a_button() {
    let mut event = focus_event(Vec::new());
    let home = UiaNode {
        heading_level: Some(80051),
        position_in_set: Some(1),
        size_of_set: Some(1),
        ..node(UIA_BUTTON, "Home", true)
    };
    event.focus_chain = vec![node(UIA_WINDOW, "Settings", false), home];
    let focus = focus_event_from_trace(&event);
    assert_eq!(
        focus.map(|f| (f.role, f.item_index, f.item_count)),
        Some((okur_core::Role::Button, Some(1), Some(1)))
    );

    let ok = node(UIA_BUTTON, "OK", true);
    let cancel = node(UIA_BUTTON, "Cancel", false);
    let mut event = focus_event(vec![ok.clone(), cancel]);
    event.focus_chain = vec![node(UIA_WINDOW, "Dialog", false), ok];
    assert_eq!(
        focus_event_from_trace(&event).and_then(|f| f.item_index),
        None
    );
}

/// A page's menu item keeps its place: on the menu bar, its position, whether it opens a
/// menu (Google Docs' "Help", 9 of 10). A menu item reported through MSAA while UIA has
/// the focus elsewhere in the page is the page moving on, not a menu.
#[test]
fn a_page_menu_item_keeps_its_place() {
    const UIA_MENUBAR: i32 = 50010;
    let mut event = focus_event(Vec::new());
    let help = UiaNode {
        position_in_set: Some(9),
        size_of_set: Some(10),
        aria_properties: Some("haspopup=menu".to_owned()),
        ..node(UIA_MENUITEM, "Help", true)
    };
    event.focus_chain = vec![
        node(UIA_DOCUMENT, "Untitled document - Google Docs", false),
        node(UIA_MENUBAR, "", false),
        help,
    ];
    let focus = focus_event_from_trace(&event);
    assert_eq!(
        focus
            .as_ref()
            .map(|f| (f.in_document, f.in_menu_bar, f.item_index, f.item_count)),
        Some((true, true, Some(9), Some(10)))
    );
    assert_eq!(focus.and_then(|f| f.states.expanded), Some(false));

    event.window_class = Some("Chrome_RenderWidgetHostHWND".to_owned());
    event.object = Some(TraceObject {
        role: Some(MSAA_MENUITEM),
        name: Some("Help".to_owned()),
        value: None,
        states: 0,
        shortcut: None,
    });
    event.focus_chain = vec![
        node(UIA_DOCUMENT, "Untitled document - Google Docs", false),
        node(UIA_EDIT, "Document content", true),
    ];
    assert_eq!(focus_event_from_trace(&event), None);
}

/// A Win32 dialog's control has its access key from MSAA when UIA names none [obs,
/// 541/s8]; UIA's comes first.
#[test]
fn an_access_key_comes_from_msaa_when_uia_has_none() {
    let mut event = focus_event(Vec::new());
    event.object = Some(TraceObject {
        role: None,
        name: Some("Pro".to_owned()),
        value: None,
        states: 0,
        shortcut: Some("Alt+r".to_owned()),
    });
    assert_eq!(
        focus_event_from_trace(&event).and_then(|f| f.access_key),
        Some("Alt+r".to_owned())
    );
    if let Some(pro) = event.focus_chain.last_mut() {
        pro.access_key = Some("alt+P".to_owned());
    }
    assert_eq!(
        focus_event_from_trace(&event).and_then(|f| f.access_key),
        Some("alt+P".to_owned())
    );
}

/// A focus event from a window already gone (Okur's own list dialog as it closes) reads
/// UIA's focus: the page, in the window with the keyboard focus (live11, 256/s4).
#[test]
fn a_focus_from_a_window_gone_is_in_the_focus_window() {
    let mut event = focus_event(Vec::new());
    event.focus_chain = vec![node(UIA_DOCUMENT, "Riverside Gazette", true)];
    assert_eq!(
        focus_event_from_trace(&event).map(|f| f.in_document),
        Some(false)
    );
    event.focus_window_class = Some("Chrome_RenderWidgetHostHWND".to_owned());
    assert_eq!(
        focus_event_from_trace(&event).map(|f| f.in_document),
        Some(true)
    );
}

fn focus_event(siblings: Vec<UiaNode>) -> TraceEvent {
    TraceEvent {
        t_ms: 0,
        event: "focus".to_owned(),
        win_event: 0x8005,
        hwnd: 0,
        id_object: -4,
        id_child: 0,
        window_class: None,
        window_style: None,
        spin_box: false,
        focus_window_class: None,
        focus_elsewhere: false,
        menu_bar: Vec::new(),
        status_bar: Vec::new(),
        window_title: None,
        process: None,
        process_product: None,
        process_version: None,
        object: None,
        focus_chain: vec![node(GROUP, "Plan", false), node(RADIO, "Pro", true)],
        focus_siblings: siblings,
        focus_subtree: None,
        list_position: None,
        column_headers: Vec::new(),
        text_context: None,
        region: None,
        page: None,
        top_window_title: None,
        top_window_class: None,
        app_window_texts: Vec::new(),
        top_window_owner_enabled: None,
        virtual_desktop: None,
        dialog_lines: Vec::new(),
        dialog_text: Vec::new(),
        dialog_page: None,
        group_box: None,
        default_button: None,
        dialog_tree: None,
        notification: None,
        mouse: None,
        point: None,
        msaa_tree: None,
        msaa_siblings: Vec::new(),
        msaa_ancestors: Vec::new(),
        msaa_description: None,
        netui_control: None,
        modal_dialog: None,
    }
}

/// WinForms radio buttons report no PositionInSet; JAWS still says "2 of 3" [obs, 301/s4].
#[test]
fn position_is_counted_among_same_type_siblings() {
    let event = focus_event(vec![
        node(TEXT, "Choose a plan", false),
        node(RADIO, "Basic", false),
        node(RADIO, "Pro", true),
        node(RADIO, "Team", false),
    ]);
    let focus = focus_event_from_trace(&event);
    assert_eq!(
        focus.map(|f| (f.item_index, f.item_count)),
        Some((Some(2), Some(3)))
    );
}

/// A Win32 message box over a WPF dialog window: the window is named plainly before the box
/// ("Manage Notification Rules | Confirm Delete dialog" [obs, 787/s17]); a WPF dialog over
/// another names no outer window (787/s5).
#[test]
fn a_message_box_names_the_wpf_window_it_opened_over() {
    let mut manage = node(UIA_WINDOW, "Manage Notification Rules", false);
    manage.is_dialog = Some(true);
    manage.class_name = Some("Window".to_owned());
    let mut confirm = node(UIA_WINDOW, "Confirm Delete", false);
    confirm.class_name = Some(WIN32_DIALOG_CLASS.to_owned());
    let mut event = focus_event(Vec::new());
    event.focus_chain = vec![manage.clone(), confirm, node(UIA_BUTTON, "Yes", true)];
    let focus = focus_event_from_trace(&event);
    assert_eq!(
        focus.as_ref().map(|f| f.outer_windows.clone()),
        Some(vec!["Manage Notification Rules".to_owned()])
    );
    let mut name_rule = node(UIA_WINDOW, "Name Your Rule", false);
    name_rule.is_dialog = Some(true);
    event.focus_chain = vec![manage, name_rule, node(UIA_BUTTON, "OK", true)];
    let focus = focus_event_from_trace(&event);
    assert_eq!(focus.map(|f| f.outer_windows), Some(Vec::new()));
}

/// A WPF window is read element by element by its window's class, also when the focus is
/// inside it and UI Automation makes the window a dialog of class "Window" (1022/s1).
#[test]
fn a_wpf_window_with_the_focus_inside_is_read_by_elements() {
    let mut window = node(UIA_WINDOW, "SR WPF Controls Fixture", false);
    window.is_dialog = Some(true);
    window.class_name = Some("Window".to_owned());
    let mut event = focus_event(Vec::new());
    event.window_class = Some("HwndWrapper[DefaultDomain;;1]".to_owned());
    event.focus_chain = vec![window, node(UIA_BUTTON, "Save", true)];
    let focus = focus_event_from_trace(&event);
    assert_eq!(focus.map(|f| f.read_by_elements), Some(true));
}

/// A field in a WPF group box: named by the text before it, and its group's line is the
/// header and that text: "Speech or Sound Action Path to Sound File: | Path to Sound File:
/// Edit" [obs, 801/s11].
#[test]
fn a_wpf_group_boxs_line_takes_the_texts_before_the_control() {
    const GROUP: i32 = 50026;
    const EDIT: i32 = 50004;
    const COMBO: i32 = 50003;
    let mut group = node(GROUP, "", false);
    group.name = None;
    group.class_name = Some("GroupBox".to_owned());
    group.label = Some("Speech or Sound Action".to_owned());
    let mut edit = node(EDIT, "", true);
    edit.name = None;
    edit.runtime_id = Some(vec![7, 1, 4]);
    let mut event = focus_event(vec![
        node(TEXT, "_Speech or Sound Action", false),
        node(COMBO, "Speech Action", false),
        node(TEXT, "Path to Sound File:", false),
        edit.clone(),
        node(UIA_BUTTON, "Select Sound...", false),
    ]);
    let mut window = node(UIA_WINDOW, "Create Rule", false);
    window.is_dialog = Some(true);
    event.focus_chain = vec![window, group, edit];
    let focus = focus_event_from_trace(&event);
    assert_eq!(
        focus.as_ref().and_then(|f| f.name.as_deref()),
        Some("Path to Sound File:")
    );
    assert_eq!(
        focus
            .as_ref()
            .and_then(|f| f.groups.last())
            .map(|g| g.name.as_str()),
        Some("Speech or Sound Action Path to Sound File:")
    );
}

/// A traced `mouse_point` is what the mouse points to: the control there and its text by
/// unit, for mouse echo in replay; a `mouse_move` stays a move.
#[test]
fn a_mouse_point_is_what_the_mouse_points_to() {
    let mut event = focus_event(Vec::new());
    event.event = "mouse_point".to_owned();
    event.point = Some(TracePoint {
        at: [16, 87],
        texts: okur_core::UnitTexts {
            line: Some(
                "fn main() {
"
                .to_owned(),
            ),
            ..Default::default()
        },
    });
    let converted = EventConverter::default().convert(&event);
    let point = match converted.as_slice() {
        [okur_core::AccessibilityEvent::MousePoint(point)] => Some(point),
        _ => None,
    };
    assert_eq!(point.map(|p| (p.x, p.y)), Some((16, 87)), "{converted:?}");
    assert_eq!(
        point.and_then(|p| p.texts.line.as_deref()),
        Some(
            "fn main() {
"
        )
    );
    assert_eq!(
        point.and_then(|p| p.control.as_ref()?.name.as_deref()),
        Some("Pro")
    );
    let mut moved = focus_event(Vec::new());
    moved.event = "mouse_move".to_owned();
    moved.mouse = Some([16, 87]);
    assert_eq!(
        EventConverter::default().convert(&moved),
        [okur_core::AccessibilityEvent::MouseMoved { x: 16, y: 87 }]
    );
}

#[test]
fn no_siblings_means_no_position() {
    let focus = focus_event_from_trace(&focus_event(Vec::new()));
    assert_eq!(focus.map(|f| f.item_index), Some(None));
}

fn dialog_node(control_type: i32, name: &str) -> UiaNode {
    UiaNode {
        class_name: Some("#32770".to_owned()),
        ..node(control_type, name, false)
    }
}

/// A button of an Office command bar inside a dialog is read from MSAA, held down where
/// MSAA says so: Insert Hyperlink's "Existing File or Web Page" [obs, 1076/s5]; the bar's
/// own focus is none.
#[test]
fn a_command_bars_button_is_read_from_msaa() {
    let object = |role: i32, name: &str, states: u32| {
        Some(TraceObject {
            role: Some(role),
            name: Some(name.to_owned()),
            value: None,
            states,
            shortcut: Some("x".to_owned()),
        })
    };
    let button = TraceEvent {
        window_class: Some(COMMAND_BAR_CLASS.to_owned()),
        top_window_title: Some("Insert Hyperlink".to_owned()),
        object: object(0x2B, "Existing File or Web Page", 0x10_000C),
        focus_chain: vec![node(UIA_WINDOW, "Insert Hyperlink", false)],
        ..focus_event(Vec::new())
    };
    let focus = command_bar_focus(&button);
    assert!(focus.is_some());
    let Some(focus) = focus else { return };
    assert_eq!(focus.role, okur_core::Role::Button);
    assert_eq!(focus.name.as_deref(), Some("Existing File or Web Page"));
    assert!(focus.states.pressed);
    assert_eq!(focus.window_title.as_deref(), Some("Insert Hyperlink"));
    let bar = TraceEvent {
        object: object(0x16, "MSO Generic Control Container", 0x1000),
        ..button
    };
    assert!(command_bar_focus(&bar).is_none());
}

/// A command in the lower ribbon names what it is in, from its MSAA ancestors: "Lower
/// Ribbon | Home | Ribbon tab | Clipboard Tool bar | Paste Click here for more options
/// such as pasting only plain text." [obs, 970/s4], and it is on the ribbon's menus.
#[test]
fn a_lower_ribbon_command_names_its_groups() {
    let msaa = |role: i32, name: &str, value: &str, description: &str| MsaaNode {
        role: Some(role),
        name: (!name.is_empty()).then(|| name.to_owned()),
        states: 0,
        value: (!value.is_empty()).then(|| value.to_owned()),
        description: (!description.is_empty()).then(|| description.to_owned()),
        children: Vec::new(),
    };
    let event = TraceEvent {
        window_class: Some("NetUIHWND".to_owned()),
        object: Some(TraceObject {
            role: Some(0x3E),
            name: Some("Paste".to_owned()),
            value: Some("Paste (Ctrl+V)".to_owned()),
            states: 0x10_0004,
            shortcut: None,
        }),
        focus_chain: vec![
            node(UIA_WINDOW, "WordPad", false),
            node(50033, "UIRibbonDockTop", false),
        ],
        msaa_ancestors: vec![
            msaa(0x26, "Ribbon", "Ribbon", ""),
            msaa(0x10, "Lower Ribbon", "Lower Ribbon", ""),
            msaa(0x0A, "", "", ""),
            msaa(0x26, "Home", "Ribbon tab", ""),
            msaa(0x16, "Clipboard", "Group", ""),
            msaa(
                0x14,
                "Paste",
                "",
                "Click here for more options such as pasting only plain text.",
            ),
        ],
        ..focus_event(Vec::new())
    };
    let focus = netui_focus(&event);
    assert!(focus.is_some());
    let Some(focus) = focus else { return };
    assert_eq!(focus.value.as_deref(), Some("Paste (Ctrl+V)"));
    assert!(focus.ribbon_menu);
    let groups: Vec<(&str, bool, Option<&str>)> = focus
        .groups
        .iter()
        .map(|g| (g.name.as_str(), g.tool_bar, g.detail.as_deref()))
        .collect();
    assert_eq!(
        groups,
        [
            ("UIRibbonDockTop", false, None),
            ("Lower Ribbon", false, None),
            ("Home", false, Some("Ribbon tab")),
            ("Clipboard", true, None),
            (
                "Paste",
                false,
                Some("Click here for more options such as pasting only plain text.")
            ),
        ]
    );
}

/// Word's ribbon commands, from MSAA and Office's own UIA element: a toggle button's state
/// and ScreenTip title and its own rectangle, a drop-down a menu button (a "sub menu" to
/// JAWS) where Paste is a split button [obs, 1045/s2, s5, s9, s11].
#[test]
fn a_word_ribbon_command_has_its_tip_toggle_and_kind() {
    let command = |role: i32, name: &str, control: Option<NetuiControl>| TraceEvent {
        window_class: Some("NetUIHWND".to_owned()),
        id_object: 103_350_513,
        object: Some(TraceObject {
            role: Some(role),
            name: Some(name.to_owned()),
            value: None,
            states: 0x10_0004,
            shortcut: None,
        }),
        focus_chain: vec![node(UIA_WINDOW, "sr-word-fixture.docx - Word", false)],
        netui_control: control,
        ..focus_event(Vec::new())
    };
    let painter = netui_focus(&command(
        0x2B,
        "Format Painter",
        Some(NetuiControl {
            bounds: Some([55, 132, 184, 159]),
            help_text: Some("Format Painter (Ctrl+Shift+C, Ctrl+Shift+V)".to_owned()),
            toggle_state: Some(0),
            control_type: None,
        }),
    ));
    assert!(painter.is_some());
    let Some(painter) = painter else { return };
    assert_eq!(painter.role, okur_core::Role::Button);
    assert_eq!(painter.states.checked, Some(false));
    assert_eq!(
        painter.help_text.as_deref(),
        Some("Format Painter (Ctrl+Shift+C, Ctrl+Shift+V)")
    );
    assert_eq!(painter.bounds, Some([55, 132, 184, 159]));
    // Without Office's element (traces before format 8), no toggle state and no tip.
    let plain = netui_focus(&command(0x2B, "Copy", None));
    assert!(plain.is_some_and(|p| p.states.checked.is_none() && p.help_text.is_none()));
    let roles: Vec<Option<okur_core::Role>> = [(0x38, "Change Case"), (0x3A, "Cover Page")]
        .iter()
        .chain(&[(0x3E, "Paste")])
        .map(|&(role, name)| netui_focus(&command(role, name, None)).map(|f| f.role))
        .collect();
    assert_eq!(
        roles,
        [
            Some(okur_core::Role::MenuButton),
            Some(okur_core::Role::MenuButton),
            Some(okur_core::Role::SplitButton)
        ]
    );
    // A drop-down that UIA calls a button: a button that opens a menu [obs, 1051/s17].
    let styles = netui_focus(&command(
        0x3A,
        "Styles",
        Some(NetuiControl {
            help_text: Some("More".to_owned()),
            control_type: Some(50000),
            ..NetuiControl::default()
        }),
    ));
    assert!(styles.is_some_and(|s| s.role == okur_core::Role::Button && s.opens_menu));
}

/// Word's Styles gallery: an MSAA list item in a list, named by its description without
/// the preview mark, placed among the list's items, on the ribbon's menus [obs, 1064/s4].
#[test]
fn a_ribbon_gallerys_item_is_a_list_item_with_its_place() {
    let item = |name: &str| MsaaNode {
        role: Some(0x22),
        name: Some(name.to_owned()),
        states: 0,
        value: None,
        description: None,
        children: Vec::new(),
    };
    let button = MsaaNode {
        role: Some(0x2B),
        ..item("More")
    };
    let event = TraceEvent {
        window_class: Some("NetUIHWND".to_owned()),
        object: Some(TraceObject {
            role: Some(0x22),
            name: Some("¶ Heading 2".to_owned()),
            value: None,
            states: 0x30_0000,
            shortcut: None,
        }),
        focus_chain: vec![node(UIA_WINDOW, "sr-word-fixture.docx - Word", false)],
        msaa_siblings: vec![
            button,
            item("¶ Heading 1"),
            item("¶ Heading 2"),
            item("Subtle Reference"),
        ],
        msaa_ancestors: vec![MsaaNode {
            role: Some(0x21),
            ..item("Styles")
        }],
        msaa_description: Some("Heading 2".to_owned()),
        ..focus_event(Vec::new())
    };
    let focus = netui_focus(&event);
    assert_eq!(
        focus.as_ref().map(|f| (
            f.role.clone(),
            f.name.as_deref(),
            f.item_index,
            f.item_count,
            f.in_ribbon && f.ribbon_menu
        )),
        Some((
            okur_core::Role::ListItem,
            Some("Heading 2"),
            Some(2),
            Some(3),
            true
        ))
    );
}

/// A focus in a window whose owner stays available is in a modeless dialog (Word's Find and
/// Replace); one whose owner it disables, or a window without one, is not [obs, 1065/s6].
#[test]
fn a_window_that_leaves_its_owner_available_is_modeless() {
    let modeless = |owner_enabled: Option<bool>| {
        focus_event_from_trace(&TraceEvent {
            top_window_owner_enabled: owner_enabled,
            ..focus_event(Vec::new())
        })
        .map(|f| f.modeless)
    };
    assert_eq!(modeless(Some(true)), Some(true));
    assert_eq!(modeless(Some(false)), Some(false));
    assert_eq!(modeless(None), Some(false));
}

/// Word's Font box, a RichEdit window of its own that UIA reaches inside the ribbon's
/// group: on the ribbon's menus, its group the ribbon's tool bar [obs, 1045/s7].
#[test]
fn a_word_ribbon_box_is_on_the_ribbons_menus() {
    let classed = |control_type: i32, name: &str, class: &str| UiaNode {
        class_name: Some(class.to_owned()),
        ..node(control_type, name, false)
    };
    let mut event = TraceEvent {
        window_class: Some("RICHEDIT60W".to_owned()),
        focus_chain: vec![
            node(UIA_WINDOW, "sr-word-fixture.docx - Word", false),
            classed(50033, "Lower Ribbon", "NetUIPanViewer"),
            classed(50025, "Home", "NetUIOrderedGroup"),
            classed(50026, "Font", NETUI_RIBBON_GROUP_CLASS),
            classed(UIA_COMBOBOX, "Font:", "NetUIComboboxAnchor"),
            UiaNode {
                value: Some("Calibri".to_owned()),
                ..classed(UIA_EDIT, "Font:", "NetUITextbox")
            },
        ],
        ..focus_event(Vec::new())
    };
    let focus = focus_event_from_trace(&event);
    assert!(focus.is_some());
    let Some(focus) = focus else { return };
    assert_eq!(focus.container_role, Some(okur_core::Role::ComboBox));
    assert!(focus.in_ribbon && focus.ribbon_menu);
    assert!(focus.groups.iter().any(|g| g.name == "Font" && g.tool_bar));
    // Outside a ribbon's group, a combo box's field is no ribbon's.
    if let Some(group) = event.focus_chain.get_mut(3) {
        group.class_name = None;
    }
    assert!(focus_event_from_trace(&event).is_some_and(|f| !f.in_ribbon));
}

/// Word's ribbon taking the focus back from its Font box reports the ribbon itself, with
/// none of its tabs focused, then the command that has the focus: the ribbon is a tab panel
/// on the ribbon's menus, so that passing it leaves them not ("Leaving menus" was said
/// there [obs, 1045/s9]); Word's module names it on the way from the search box only
/// [obs, 1051/s12].
#[test]
fn the_ribbon_taking_the_focus_back_is_on_its_menus() {
    let event = TraceEvent {
        window_class: Some("NetUIHWND".to_owned()),
        id_object: OBJID_CLIENT,
        object: Some(TraceObject {
            role: Some(MSAA_PROPERTYPAGE),
            name: Some("Ribbon".to_owned()),
            value: Some("Ribbon".to_owned()),
            states: 0,
            shortcut: None,
        }),
        focus_chain: vec![
            node(UIA_WINDOW, "sr-word-fixture.docx - Word", false),
            node(50033, "", false),
        ],
        ..focus_event(Vec::new())
    };
    let focus = focus_event_from_trace(&event);
    assert!(focus.is_some_and(|f| f.role == okur_core::Role::TabPanel
        && f.name.as_deref() == Some("Ribbon")
        && f.in_ribbon
        && f.ribbon_menu));
}

/// Word's search box beside the ribbon's tabs: in the ribbon, not on its menus, in the menu
/// item that holds it [obs, 1051/s11].
#[test]
fn words_search_box_is_in_its_menu_item() {
    let classed = |control_type: i32, name: &str, class: &str| UiaNode {
        class_name: Some(class.to_owned()),
        ..node(control_type, name, false)
    };
    let event = TraceEvent {
        window_class: Some("RICHEDIT60W".to_owned()),
        focus_chain: vec![
            node(UIA_WINDOW, "sr-word-fixture.docx - Word", false),
            classed(50018, "Ribbon Tabs", NETUI_RIBBON_TABS_CLASS),
            classed(
                50011,
                "Tell me what you want to do...",
                "NetUISearchBoxAnchor",
            ),
            classed(UIA_EDIT, "Tell me what you want to do...", "NetUITextbox"),
        ],
        ..focus_event(Vec::new())
    };
    let focus = focus_event_from_trace(&event);
    assert!(focus.is_some());
    let Some(focus) = focus else { return };
    assert!(focus.in_ribbon && !focus.ribbon_menu);
    assert_eq!(focus.container_role, Some(okur_core::Role::MenuItem));
    assert_eq!(
        focus.container_name.as_deref(),
        Some("Tell me what you want to do...")
    );
}

/// In Word's ribbon, UI Automation's own focus events (passed on as WinEvents, naming no
/// MSAA object) and a command named twice by Word say nothing more: each command was said
/// with "Leaving menus" after it, Cut and the Home tab twice [obs, 1045/s1-s12].
#[test]
fn a_word_ribbon_command_is_one_focus() {
    let command = |id_object: i32, name: Option<&str>| TraceEvent {
        window_class: Some("NetUIHWND".to_owned()),
        id_object,
        object: name.map(|name| TraceObject {
            role: Some(0x2B),
            name: Some(name.to_owned()),
            value: None,
            states: 0x10_0005,
            shortcut: None,
        }),
        focus_chain: vec![
            node(UIA_WINDOW, "sr-word-fixture.docx - Word", false),
            node(50033, "", false),
        ],
        ..focus_event(Vec::new())
    };
    let mut converter = EventConverter::default();
    let focuses = |converted: Vec<okur_core::AccessibilityEvent>| {
        converted
            .into_iter()
            .filter_map(|e| match e {
                okur_core::AccessibilityEvent::Focus(f) => f.name,
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        focuses(converter.convert(&command(101_581_535, Some("Cut")))),
        ["Cut"]
    );
    assert!(focuses(converter.convert(&command(11_080, None))).is_empty());
    assert!(focuses(converter.convert(&command(101_581_535, Some("Cut")))).is_empty());
    assert_eq!(
        focuses(converter.convert(&command(103_350_800, Some("Copy")))),
        ["Copy"]
    );
}

/// Classic RichEdit windows, not Windows 11 Notepad's, which names itself, nor plain edits.
#[test]
fn classic_rich_edits_are_told_apart() {
    for class in ["RICHEDIT50W", "RichEdit20W", "RichEdit20A", "RichEdit"] {
        assert!(rich_edit(Some(class)), "{class}");
    }
    for class in ["RichEditD2DPT", "Edit", "WindowsForms10.EDIT.app.0.1"] {
        assert!(!rich_edit(Some(class)), "{class}");
    }
}

/// WordPad's ribbon given the focus by Alt: its focused tab, with its place among the tabs
/// shown and the ribbon's panes and row of tabs as the groups entered [obs, 970/s1].
#[test]
fn alt_on_a_ribbon_focuses_its_selected_tab() {
    let msaa = |role: i32, name: &str, states: u32, children: Vec<MsaaNode>| MsaaNode {
        role: Some(role),
        name: (!name.is_empty()).then(|| name.to_owned()),
        states,
        value: None,
        description: None,
        children,
    };
    let tabs = msaa(
        0x3C,
        "Ribbon tabs",
        0,
        vec![msaa(
            0x0A,
            "",
            0,
            vec![
                msaa(0x25, "Home", 0x30_0006, Vec::new()),
                msaa(0x25, "View", 0x30_0000, Vec::new()),
                msaa(0x25, "Print preview", 0x8001, Vec::new()),
            ],
        )],
    );
    let pane = |name: &str| node(50033, name, false);
    let event = TraceEvent {
        window_class: Some("NetUIHWND".to_owned()),
        object: Some(TraceObject {
            role: Some(0x26),
            name: Some("Ribbon".to_owned()),
            value: Some("Ribbon".to_owned()),
            states: 0,
            shortcut: None,
        }),
        focus_chain: vec![
            node(UIA_WINDOW, "sr-wordpad-fixture.rtf - WordPad", false),
            pane("UIRibbonDockTop"),
            pane("Ribbon"),
            pane("Ribbon"),
            pane(""),
            pane("Ribbon"),
        ],
        msaa_tree: Some(msaa(0x26, "Ribbon", 0, vec![tabs])),
        ..focus_event(Vec::new())
    };
    let focus = netui_focus(&event);
    assert!(focus.is_some());
    let Some(focus) = focus else { return };
    assert_eq!(focus.role, okur_core::Role::Tab);
    assert_eq!(focus.name.as_deref(), Some("Home"));
    assert_eq!(focus.states.selected, Some(true));
    assert_eq!((focus.item_index, focus.item_count), (Some(1), Some(2)));
    assert!(focus.in_ribbon);
    let groups: Vec<(&str, bool)> = focus
        .groups
        .iter()
        .map(|g| (g.name.as_str(), g.tab_list))
        .collect();
    assert_eq!(
        groups,
        [
            ("UIRibbonDockTop", false),
            ("Ribbon", false),
            ("Ribbon", false),
            ("Ribbon", false),
            ("Ribbon tabs", true)
        ]
    );
}

/// Local Disk (C:) Properties opening, focus on the General page's Label field, as traced
/// on the dev box: the page is not in the focus chain, and the siblings are the page's.
fn property_page_event() -> TraceEvent {
    TraceEvent {
        focus_chain: vec![
            dialog_node(UIA_WINDOW, "Local Disk (C:) Properties"),
            node(UIA_EDIT, "Label:", true),
        ],
        focus_siblings: vec![
            node(UIA_EDIT, "Label:", true),
            node(TEXT, "Type:", false),
            node(TEXT, "Local Disk", false),
        ],
        top_window_title: Some("Local Disk (C:) Properties".to_owned()),
        top_window_class: Some("#32770".to_owned()),
        dialog_page: Some("General".to_owned()),
        ..focus_event(Vec::new())
    }
}

/// A text that names a control beside it is that control's label, not the dialog's text;
/// the group box comes from the trace (JAWS on Okur's settings dialog, dev box,
/// 2026-10-17: "Okur Settings dialog | Speech Synthesizer: Combo box"). Its labels give
/// their combo boxes access keys ("&Voice:").
#[test]
fn a_dialogs_labels_are_not_its_text() {
    let synthesizer = UiaNode {
        access_key: Some("Alt+s".to_owned()),
        ..node(UIA_COMBOBOX, "Synthesizer:", true)
    };
    let settings = TraceEvent {
        focus_chain: vec![dialog_node(UIA_WINDOW, "Okur Settings"), synthesizer],
        focus_siblings: vec![
            node(TEXT, "Synthesizer:", false),
            node(UIA_COMBOBOX, "Synthesizer:", true),
            node(TEXT, "Voice:", false),
            node(UIA_COMBOBOX, "Voice:", false),
            node(TEXT, "Changes apply when you press OK.", false),
        ],
        group_box: Some("Speech".to_owned()),
        ..focus_event(Vec::new())
    };
    let focus = focus_event_from_trace(&settings);
    assert_eq!(
        focus.as_ref().and_then(|f| f.dialog_text.clone()),
        Some("Changes apply when you press OK.".to_owned())
    );
    assert_eq!(focus.and_then(|f| f.group_box), Some("Speech".to_owned()));
}

/// A text that names the control the focus is inside is that control's label, not the
/// dialog's text: Document Regions' "Regions" names its tree [obs, 274/s1].
#[test]
fn the_label_of_the_control_around_the_focus_is_not_the_dialogs_text() {
    const UIA_TREE: i32 = 50023;
    let regions = TraceEvent {
        focus_chain: vec![
            dialog_node(UIA_WINDOW, "Document Regions"),
            node(UIA_TREE, "Regions", false),
            node(UIA_TREEITEM, "Banner", true),
        ],
        focus_siblings: vec![node(UIA_TREEITEM, "Main", false)],
        dialog_text: vec!["Regions".to_owned()],
        ..focus_event(Vec::new())
    };
    let focus = focus_event_from_trace(&regions);
    assert_eq!(focus.and_then(|f| f.dialog_text), Some(String::new()));
}

/// A text naming the focused field is the dialog's prompt when the field takes no access
/// key from it ("Custom Label dialog | Enter a custom label for this item | Enter a custom
/// label for this item edit" [obs, 688/s9]), and the field's label when it does ("Font
/// dialog | Font: edit combo" [obs, 541/s2]).
#[test]
fn a_prompt_naming_the_focused_field_is_the_dialogs_text() {
    let prompt = "Enter a custom label for this item";
    let field = UiaNode {
        runtime_id: Some(vec![42, 7]),
        ..node(UIA_DOCUMENT, prompt, true)
    };
    let tree = UiaNode {
        children: vec![
            node(UIA_TEXT, prompt, false),
            field.clone(),
            node(UIA_BUTTON, "OK", false),
        ],
        ..dialog_node(UIA_WINDOW, "Custom Label")
    };
    let label = TraceEvent {
        focus_chain: vec![dialog_node(UIA_WINDOW, "Custom Label"), field.clone()],
        dialog_text: vec![prompt.to_owned()],
        dialog_tree: Some(tree.clone()),
        ..focus_event(Vec::new())
    };
    assert_eq!(
        focus_event_from_trace(&label).and_then(|f| f.dialog_text),
        Some(prompt.to_owned())
    );
    let keyed = UiaNode {
        access_key: Some("Alt+e".to_owned()),
        ..field
    };
    let labelled = TraceEvent {
        focus_chain: vec![dialog_node(UIA_WINDOW, "Custom Label"), keyed],
        ..label
    };
    assert_eq!(
        focus_event_from_trace(&labelled).and_then(|f| f.dialog_text),
        Some(String::new())
    );
}

/// A message box's text is beside its buttons; a property page's labels are the page's,
/// not the dialog's (JAWS 2025 on the dev box: "Local Disk (C:) Properties dialog",
/// "General page", "Label: Edit").
#[test]
fn a_property_pages_labels_are_not_the_dialogs_text() {
    let message_box = TraceEvent {
        focus_chain: vec![
            dialog_node(UIA_WINDOW, "SR Dialog Fixture"),
            node(UIA_BUTTON, "Yes", true),
        ],
        focus_siblings: vec![
            node(TEXT, "Save changes to the document?", false),
            node(UIA_BUTTON, "Yes", true),
        ],
        ..focus_event(Vec::new())
    };
    assert_eq!(
        focus_event_from_trace(&message_box).and_then(|f| f.dialog_text),
        Some("Save changes to the document?".to_owned())
    );
    assert_eq!(
        focus_event_from_trace(&property_page_event()).and_then(|f| f.dialog_text),
        Some(String::new())
    );
}

/// JAWS 2025 names the page a property sheet opens on (dev box, 2026-10-15).
#[test]
fn a_property_sheet_opens_on_its_page() {
    use okur_core::{AomNode, MockSpeechOutput, Role, ScreenReaderEngine, VirtualBuffer};
    let cursor =
        okur_core::VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let spoken = focus_event_from_trace(&property_page_event())
        .and_then(|focus| engine.handle_focus_event(&focus))
        .unwrap_or_default();
    // Lines as heard: role marks and spacing aside.
    let lines: Vec<String> = spoken
        .lines()
        .map(|l| {
            l.replace(['\u{1d}', '\u{1e}'], " ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    assert_eq!(
        lines,
        vec![
            "Local Disk (C:) Properties dialog",
            "General page",
            "Label: Edit",
            "Type in text."
        ],
        "{spoken:?}"
    );
}

/// A wizard takes its page's title as its own and shows no tabs: its page is not named,
/// and the page's text is the dialog's ("Flexible Web dialog | What would you like to do?"
/// [obs, 705/s3]), where a tabbed sheet's page is named and its labels are not the
/// dialog's text.
#[test]
fn a_wizards_page_is_its_dialog() {
    const UIA_RADIOBUTTON: i32 = 50013;
    let wizard = TraceEvent {
        focus_chain: vec![
            dialog_node(UIA_WINDOW, "Flexible Web"),
            node(UIA_RADIOBUTTON, "Create a new customization", true),
        ],
        focus_siblings: vec![
            node(TEXT, "What would you like to do?", false),
            node(UIA_RADIOBUTTON, "Create a new customization", true),
            node(UIA_BUTTON, "Next >", false),
        ],
        top_window_title: Some("Flexible Web".to_owned()),
        top_window_class: Some("#32770".to_owned()),
        dialog_page: Some("Flexible Web".to_owned()),
        ..focus_event(Vec::new())
    };
    let focus = focus_event_from_trace(&wizard);
    assert_eq!(
        focus.as_ref().map(|f| (
            f.dialog_page.clone(),
            f.wizard_page.clone(),
            f.dialog_text.clone()
        )),
        Some((
            None,
            Some("Flexible Web".to_owned()),
            Some("What would you like to do?".to_owned())
        ))
    );
    let tabbed = focus_event_from_trace(&property_page_event());
    assert_eq!(
        tabbed.map(|f| (f.dialog_page, f.wizard_page)),
        Some((Some("General".to_owned()), None))
    );
}

/// The Run dialog's Open field: an edit inside a combo box, whose access key and position
/// are the combo box's.
#[test]
fn an_edit_in_a_combo_box_is_its_field() {
    let combo = UiaNode {
        access_key: Some("Alt+o".to_owned()),
        ..node(UIA_COMBOBOX, "Open:", false)
    };
    let event = TraceEvent {
        focus_chain: vec![combo, node(UIA_EDIT, "Open:", true)],
        focus_siblings: vec![node(UIA_EDIT, "Open:", true)],
        list_position: Some([0, 6]),
        ..focus_event(Vec::new())
    };
    let focus = focus_event_from_trace(&event);
    assert_eq!(
        focus.as_ref().map(|f| (
            f.container_role.clone(),
            f.access_key.as_deref(),
            f.item_index,
            f.item_count
        )),
        Some((
            Some(okur_core::Role::ComboBox),
            Some("Alt+o"),
            Some(1),
            Some(6)
        ))
    );
}

/// Windows search's result, selected while focus stays in the search box.
#[test]
fn a_uia_selection_is_a_selected_item() {
    let result = UiaNode {
        position_in_set: Some(1),
        size_of_set: Some(5),
        ..node(
            UIA_LISTITEM,
            "Notepad, App, Press right to view options",
            false,
        )
    };
    let event = TraceEvent {
        event: "uia_selected".to_owned(),
        win_event: 20012,
        focus_chain: vec![result],
        ..focus_event(Vec::new())
    };
    let events = EventConverter::default().convert(&event);
    assert!(
        matches!(events.as_slice(), [okur_core::AccessibilityEvent::Selected(item)]
                if item.item_index == Some(1) && item.item_count == Some(5)),
        "{events:?}"
    );
}

/// Focus reported on the browser window while a page button is focused, then the button
/// expanded with Space (GitHub's "Solutions", dev box): the change is the focused
/// control's, so it is passed on. Another object's change is not.
#[test]
fn a_change_of_the_control_focused_inside_a_window_is_passed_on() {
    const BUTTON: i32 = 50000;
    let object = |name: &str| {
        Some(TraceObject {
            role: None,
            name: Some(name.to_owned()),
            value: None,
            states: 0,
            shortcut: None,
        })
    };
    let solutions = UiaNode {
        expand_collapse_state: Some(1),
        ..node(BUTTON, "Solutions", true)
    };
    let focus = TraceEvent {
        object: object("Page - Google Chrome"),
        focus_chain: vec![solutions.clone()],
        ..focus_event(Vec::new())
    };
    let change = |name: &str| TraceEvent {
        event: "state_change".to_owned(),
        hwnd: 7,
        id_child: -16936,
        object: object(name),
        focus_chain: vec![solutions.clone()],
        ..focus_event(Vec::new())
    };
    let mut converter = EventConverter::default();
    assert_eq!(converter.convert(&focus).len(), 1);
    assert!(
        matches!(
            converter.convert(&change("Solutions")).as_slice(),
            [okur_core::AccessibilityEvent::Change { .. }]
        ),
        "the focused button's change"
    );
    assert!(converter.convert(&change("Pricing")).is_empty());
    // Live, the reader decides from the same focus before reading the change further
    // (performance review P5, step 2): the converter's focus is the one it checks.
    let named = |n: &str| TraceObject {
        name: Some(n.to_owned()),
        ..TraceObject::default()
    };
    let identity = (7, focus.id_object, -16936);
    let page = "Chrome_RenderWidgetHostHWND";
    let kept = |event: &str, identity, object: Option<&TraceObject>| {
        super::live_change_kept(event, identity, object, page, false)
    };
    assert!(kept("state_change", identity, Some(&named("Solutions"))));
    assert!(!kept("state_change", identity, Some(&named("Pricing"))));
    // The focus's own object, whatever its name now.
    assert!(kept(
        "state_change",
        (focus.hwnd, focus.id_object, focus.id_child),
        None
    ));
    // UI Automation's own value change in the focused window names no MSAA object; one in
    // another window, or a state change, is not kept.
    assert!(kept("value_change", (focus.hwnd, 414, 0), None));
    assert!(!kept("state_change", (focus.hwnd, 414, 0), None));
    assert!(!kept("value_change", (focus.hwnd + 1, 414, 0), None));
    // ... unless that window has the keyboard focus.
    assert!(super::live_change_kept(
        "value_change",
        (focus.hwnd + 1, 414, 0),
        None,
        page,
        true
    ));
    // A page's progress bar or meter, anywhere in the page.
    let meter = TraceObject {
        role: Some(super::ROLE_SYSTEM_PROGRESSBAR),
        ..named("CPU")
    };
    assert!(kept("value_change", identity, Some(&meter)));
    assert!(!kept("state_change", identity, Some(&meter)));
    assert!(!super::live_change_kept(
        "value_change",
        identity,
        Some(&meter),
        "Notepad",
        false
    ));
}

/// Space on a select with the focus: its state change is read after its list has taken
/// the focus, so the focus chain ends in the list's item. The change is the select's, the
/// event's object, expanded by its MSAA states [obs, 276/s7].
#[test]
fn a_change_read_after_the_focus_moved_on_is_the_focused_controls() {
    const COMBO_BOX: i32 = 50003;
    const EXPANDED: u32 = 0x200;
    let select = node(COMBO_BOX, "Home branch", true);
    let object = |states: u32| {
        Some(TraceObject {
            role: Some(46),
            name: Some("Home branch".to_owned()),
            value: Some("Northside".to_owned()),
            states,
            shortcut: None,
        })
    };
    let focus = TraceEvent {
        object: object(0),
        focus_chain: vec![select.clone()],
        ..focus_event(Vec::new())
    };
    let expanded = TraceEvent {
        event: "state_change".to_owned(),
        object: object(EXPANDED),
        focus_chain: vec![select, node(UIA_LISTITEM, "Northside", true)],
        ..focus_event(Vec::new())
    };
    let mut converter = EventConverter::default();
    assert_eq!(converter.convert(&focus).len(), 1);
    assert!(
        matches!(
            converter.convert(&expanded).as_slice(),
            [okur_core::AccessibilityEvent::Change { control, .. }]
                if control.role == okur_core::Role::ComboBox
                    && control.states.expanded == Some(true)
        ),
        "the select, expanded"
    );
}

/// A file dialog's folder is its address bar's toolbar's name [obs, 524/s3].
#[test]
fn a_file_dialogs_folder_is_its_address_bars_name() {
    let address = UiaNode {
        class_name: Some("ToolbarWindow32".to_owned()),
        automation_id: Some("1001".to_owned()),
        ..node(50021, "Address: C:\\Temp", false)
    };
    let navigation = UiaNode {
        class_name: Some("ToolbarWindow32".to_owned()),
        automation_id: Some("41477".to_owned()),
        ..node(50021, "Navigation buttons", false)
    };
    let band = UiaNode {
        children: vec![navigation, address],
        ..node(50033, "", false)
    };
    let dialog = UiaNode {
        children: vec![band],
        ..node(UIA_WINDOW, "Save As", false)
    };
    assert_eq!(address_band(&dialog).as_deref(), Some("Address: C:\\Temp"));
    assert_eq!(address_band(&node(UIA_WINDOW, "Find", false)), None);
}

/// Tab onto a file dialog's toolbar button: UIA's focus stays on the toolbar, the MSAA
/// event's object is the button. The focus is the button in a toolbar [obs, 524/s6].
#[test]
fn a_toolbar_button_focused_through_msaa_is_the_focus() {
    let toolbar = TraceEvent {
        window_class: Some("ToolbarWindow32".to_owned()),
        id_child: 1,
        object: Some(TraceObject {
            role: Some(0x2B),
            name: Some("Hide Folders".to_owned()),
            value: None,
            states: 0,
            shortcut: None,
        }),
        focus_chain: vec![node(UIA_WINDOW, "Save As", false), node(50021, "", true)],
        ..focus_event(Vec::new())
    };
    let focus = focus_event_from_trace(&toolbar);
    assert!(
        focus
            .as_ref()
            .is_some_and(|f| f.role == okur_core::Role::Button
                && f.name.as_deref() == Some("Hide Folders")
                && f.container_role == Some(okur_core::Role::ToolBar)),
        "{focus:?}"
    );
    // The toolbar's own focus (no child), which the button's follows, is not one; the
    // toolbar is said by its role alone [obs, 3156/s3].
    let own = TraceEvent {
        id_child: 0,
        ..toolbar.clone()
    };
    assert!(focus_event_from_trace(&own).is_none());
    let named = TraceEvent {
        focus_chain: vec![
            node(UIA_WINDOW, "Programs and Features", false),
            node(50021, "Up band toolbar", true),
        ],
        ..toolbar
    };
    assert!(focus_event_from_trace(&named).is_some_and(|f| f.container_name.is_none()));
}

/// Tab onto a list view's column title: a header bar at its place among the titles, not
/// in the list [obs, 3156/s2].
#[test]
fn a_column_title_is_a_header_bar() {
    const SPLIT_BUTTON: i32 = 50031;
    let titles: Vec<UiaNode> = ["Name", "Publisher", "Installed On"]
        .iter()
        .map(|name| node(SPLIT_BUTTON, name, false))
        .collect();
    let header = UiaNode {
        class_name: Some("SysHeader32".to_owned()),
        ..node(UIA_HEADER, "Header Control", false)
    };
    let list = UiaNode {
        class_name: Some("SysListView32".to_owned()),
        ..node(UIA_LIST, "Folder View", false)
    };
    let event = TraceEvent {
        window_class: Some("SysHeader32".to_owned()),
        focus_chain: vec![
            node(UIA_WINDOW, "Programs and Features", false),
            list,
            header,
            node(SPLIT_BUTTON, "Publisher", true),
        ],
        focus_siblings: titles,
        ..focus_event(Vec::new())
    };
    let focus = focus_event_from_trace(&event);
    assert!(
        focus.as_ref().is_some_and(|f| f.header_bar
            && f.item_index == Some(2)
            && f.item_count == Some(3)
            && f.container_role.is_none()),
        "{focus:?}"
    );
}

/// Down in the Font dialog's font field: its open list's item takes the WinEvent focus
/// while the focus stays in the field. It is a selection, with its index [obs, 541/s3].
#[test]
fn an_edit_combos_list_item_is_a_selection() {
    const COMBO_BOX: i32 = 50003;
    const EDIT: i32 = 50004;
    const MSAA_LISTITEM: i32 = 0x22;
    let field = TraceEvent {
        focus_chain: vec![node(COMBO_BOX, "Font:", false), node(EDIT, "Font:", true)],
        ..focus_event(Vec::new())
    };
    let item = TraceEvent {
        window_class: Some("ComboLBox".to_owned()),
        id_child: 39,
        object: Some(TraceObject {
            role: Some(MSAA_LISTITEM),
            name: Some("Constantia".to_owned()),
            value: None,
            states: 0,
            shortcut: None,
        }),
        focus_chain: field.focus_chain.clone(),
        ..focus_event(Vec::new())
    };
    let mut converter = EventConverter::default();
    assert_eq!(converter.convert(&field).len(), 1);
    assert!(
        matches!(
            converter.convert(&item).as_slice(),
            [okur_core::AccessibilityEvent::Selected(selected)]
                if selected.name.as_deref() == Some("Constantia")
                    && selected.item_index == Some(39)
        ),
        "the font, selected"
    );
}

/// Down in a select: the value change carries the focus's text, which a select has none
/// of, and its new choice as its value. It is a value change, not a text change
/// [obs, 205/s9]; a text field's value change stays a text change.
#[test]
fn a_selects_value_change_is_a_change_not_text() {
    const COMBO_BOX: i32 = 50003;
    let select = node(COMBO_BOX, "Home branch", true);
    let object = |value: &str| {
        Some(TraceObject {
            role: Some(46),
            name: Some("Home branch".to_owned()),
            value: Some(value.to_owned()),
            states: 0,
            shortcut: None,
        })
    };
    let focus = TraceEvent {
        object: object("Northside"),
        focus_chain: vec![select.clone()],
        ..focus_event(Vec::new())
    };
    let value_change = |text: &str| TraceEvent {
        event: "value_change".to_owned(),
        object: object("Eastgate"),
        focus_chain: vec![select.clone()],
        text_context: Some(okur_core::TextContext {
            text: text.to_owned(),
            caret: 0,
            selection: None,
            font: None,
            word: None,
            misspelled: Vec::new(),
            structure: None,
        }),
        ..focus_event(Vec::new())
    };
    let mut converter = EventConverter::default();
    assert_eq!(converter.convert(&focus).len(), 1);
    assert!(
        matches!(
            converter.convert(&value_change("")).as_slice(),
            [okur_core::AccessibilityEvent::Change { .. }]
        ),
        "the select's new choice"
    );
    assert!(matches!(
        converter.convert(&value_change("Eastgate")).as_slice(),
        [okur_core::AccessibilityEvent::TextContext(_)]
    ));
}

/// A value change with the focused field's text is the field's only when it is about the
/// focus: Word's ribbon changing its Font box as the caret crosses a style is not the
/// document's text (performance review 2026-10-25, finding 2).
#[test]
fn another_objects_value_change_is_not_the_fields_text() {
    let text = || {
        Some(okur_core::TextContext {
            text: "Library Hours".to_owned(),
            caret: 0,
            selection: None,
            font: None,
            word: None,
            misspelled: Vec::new(),
            structure: None,
        })
    };
    let document = TraceEvent {
        hwnd: 1,
        object: Some(TraceObject {
            role: Some(10),
            name: Some("Microsoft Word Document".to_owned()),
            value: None,
            states: 0,
            shortcut: None,
        }),
        focus_chain: vec![node(UIA_DOCUMENT, "sr-word-fixture.docx", true)],
        text_context: text(),
        ..focus_event(Vec::new())
    };
    let value_change = |hwnd: isize, id_object: i32, name: &str| TraceEvent {
        event: "value_change".to_owned(),
        hwnd,
        id_object,
        object: Some(TraceObject {
            role: Some(42),
            name: Some(name.to_owned()),
            value: Some("Calibri Light".to_owned()),
            states: 0,
            shortcut: None,
        }),
        text_context: text(),
        ..focus_event(Vec::new())
    };
    let mut converter = EventConverter::default();
    assert_eq!(converter.convert(&document).len(), 1);
    assert!(converter
        .convert(&value_change(2, 103_351_241, "Font:"))
        .is_empty());
    assert!(matches!(
        converter
            .convert(&value_change(1, -4, "Microsoft Word Document"))
            .as_slice(),
        [okur_core::AccessibilityEvent::TextContext(_)]
    ));
}

/// Chrome's caption names the profile; its UIA window name does not.
#[test]
fn window_is_named_by_its_caption() {
    const WINDOW: i32 = 50032;
    const DOCUMENT: i32 = 50030;
    let event = TraceEvent {
        focus_chain: vec![
            node(WINDOW, "Page - Google Chrome", false),
            node(DOCUMENT, "Page", true),
        ],
        top_window_title: Some("Page - Google Chrome - Profile".to_owned()),
        ..focus_event(Vec::new())
    };
    let focus = focus_event_from_trace(&event).and_then(|f| f.window_title);
    assert_eq!(focus.as_deref(), Some("Page - Google Chrome - Profile"));
    // Older traces: the UIA window's name.
    let older = TraceEvent {
        top_window_title: None,
        top_window_class: None,
        app_window_texts: Vec::new(),
        ..event
    };
    let focus = focus_event_from_trace(&older).and_then(|f| f.window_title);
    assert_eq!(focus.as_deref(), Some("Page - Google Chrome"));
}

/// The task switcher's items take focus through UIA only (Alt+Tab with Alt held).
#[test]
fn uia_focus_is_a_focus_once() {
    const LIST_ITEM: i32 = 50007;
    let switcher = |name: &str| TraceEvent {
        event: "uia_focus".to_owned(),
        win_event: 20005,
        window_class: Some("XamlExplorerHostIslandWindow".to_owned()),
        focus_chain: vec![node(LIST_ITEM, name, true)],
        ..focus_event(Vec::new())
    };
    let mut converter = EventConverter::default();
    let first = converter.convert(&switcher("Untitled - Notepad"));
    assert!(
        matches!(first.as_slice(), [okur_core::AccessibilityEvent::Focus(f)]
                if f.name.as_deref() == Some("Untitled - Notepad")),
        "{first:?}"
    );
    // A focus UIA repeats changes nothing.
    assert!(converter
        .convert(&switcher("Untitled - Notepad"))
        .is_empty());
    assert_eq!(converter.convert(&switcher("Downloads")).len(), 1);
}

/// Excel's Go To: the dialog's MSAA names its field, "Reference:", where UI Automation has
/// only the field's window, a pane named by its text; the field's own report between the
/// dialog's two is none [obs, 1124/s1, s8].
#[test]
fn an_office_dialog_field_is_its_label_as_msaa_has_it() {
    const PANE: i32 = 50033;
    let dialog = UiaNode {
        control_type: UIA_WINDOW,
        name: Some("Go To".to_owned()),
        class_name: Some("bosa_sdm_XL9".to_owned()),
        is_dialog: Some(true),
        ..UiaNode::default()
    };
    let field = |text: &str| UiaNode {
        control_type: PANE,
        name: (!text.is_empty()).then(|| text.to_owned()),
        class_name: Some("EDTBX".to_owned()),
        has_keyboard_focus: true,
        ..UiaNode::default()
    };
    let reports = |text: &str| {
        let chain = vec![dialog.clone(), field(text)];
        let from_dialog = TraceEvent {
            window_class: Some("bosa_sdm_XL9".to_owned()),
            window_title: Some("Go To".to_owned()),
            id_child: 5,
            object: Some(TraceObject {
                role: Some(MSAA_TEXT),
                name: Some("Reference:".to_owned()),
                value: (!text.is_empty()).then(|| text.to_owned()),
                states: 0,
                shortcut: Some("Alt+R".to_owned()),
            }),
            focus_chain: chain.clone(),
            ..focus_event(Vec::new())
        };
        let from_field = TraceEvent {
            window_class: Some("EDTBX".to_owned()),
            object: Some(TraceObject {
                role: Some(0x0A),
                name: Some(text.to_owned()),
                value: None,
                states: 0,
                shortcut: None,
            }),
            focus_chain: chain,
            ..focus_event(Vec::new())
        };
        (from_dialog, from_field)
    };
    for text in ["", "A1"] {
        let (from_dialog, from_field) = reports(text);
        let converted = EventConverter::default().convert(&from_dialog);
        let value = (!text.is_empty()).then_some(text);
        assert!(
            matches!(converted.as_slice(), [okur_core::AccessibilityEvent::Focus(f)]
                    if f.role == okur_core::Role::Edit
                        && f.name.as_deref() == Some("Reference:")
                        && f.value.as_deref() == value),
            "{converted:?}"
        );
        assert!(EventConverter::default().convert(&from_field).is_empty());
    }
}

/// Delete empties Excel's cell: UI Automation's focus is the grid then, which reports only
/// its value's change; that change is the focus moving to it [obs, 1123/s7].
#[test]
fn a_value_change_that_moves_the_focus_is_a_focus() {
    const CELL: i32 = 50029;
    const TAB: i32 = 50018;
    let grid = UiaNode {
        control_type: TAB,
        name: Some("Book1.xlsx".to_owned()),
        class_name: Some("EXCEL7".to_owned()),
        has_keyboard_focus: true,
        ..UiaNode::default()
    };
    let cell = TraceEvent {
        hwnd: 7,
        id_object: 412,
        window_class: Some("EXCEL7".to_owned()),
        focus_chain: vec![node(CELL, "D1", true)],
        ..focus_event(Vec::new())
    };
    let changed = TraceEvent {
        event: "value_change".to_owned(),
        win_event: 0x800E,
        id_object: 414,
        focus_chain: vec![grid.clone()],
        ..cell.clone()
    };
    let mut converter = EventConverter::default();
    assert_eq!(converter.convert(&cell).len(), 1);
    let moved = converter.convert(&changed);
    assert!(
        matches!(moved.as_slice(), [okur_core::AccessibilityEvent::Focus(f)]
                if f.name.as_deref() == Some("Book1.xlsx")),
        "{moved:?}"
    );
    // The grid's own value changing again moves the focus nowhere.
    assert!(!converter
        .convert(&changed)
        .iter()
        .any(|e| matches!(e, okur_core::AccessibilityEvent::Focus(_))));
}

/// The converter adds what a focus's classes tell to every focus it passes on: a WinUI toggle
/// switch is said by its state, "Night light Button | Off" (JAWS, dev box 2026-10-16).
#[test]
fn a_converted_focus_carries_its_class_facts() {
    let event = TraceEvent {
        focus_chain: vec![UiaNode {
            class_name: Some("ToggleSwitch".to_owned()),
            ..node(UIA_BUTTON, "Night light", true)
        }],
        ..focus_event(Vec::new())
    };
    let converted = EventConverter::default().convert(&event);
    assert!(
        matches!(converted.as_slice(), [okur_core::AccessibilityEvent::Focus(focus)] if focus.switch),
        "{converted:?}"
    );
}
