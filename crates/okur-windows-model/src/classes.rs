//! What a window's class says about how it is read, and the facts it tells the engine.

/// Whether a window of class `class` (its top-level window's), drawn by `framework` (UI
/// Automation's FrameworkId, empty when unknown), is read element by element: a modern app's
/// (UWP, WinUI, WPF), as JAWS's viewer lists its elements and its Scan Cursor walks them
/// [obs, 1002/s2]; a classic window by its screen rows.
pub fn reads_by_elements(class: &str, framework: &str) -> bool {
    matches!(
        class,
        "ApplicationFrameWindow" | "Windows.UI.Core.CoreWindow" | "WinUIDesktopWin32WindowClass"
    ) || class.starts_with("HwndWrapper[")
        || matches!(framework, "XAML" | "WPF" | "WinUI")
}

/// Whether a window of class `class` shows files to select: File Explorer's, or the desktop's
/// (its icons are the shell's `Progman` or `WorkerW`).
pub fn is_file_view(class: &str) -> bool {
    matches!(
        class,
        "CabinetWClass" | "ExploreWClass" | "Progman" | "WorkerW"
    )
}

/// A Win32 popup menu's window.
pub const MENU: &str = "#32768";

/// Office's own windows (NetUI): its ribbon, task panes and menus.
pub const NETUI: &str = "NetUIHWND";

/// Whether menu mode starting in a window of class `class` comes from Windows' Win+X launcher,
/// whose menu JAWS reads its own way [obs, 520/s3].
pub fn is_launcher(class: Option<&str>) -> bool {
    class == Some("LauncherTipWnd")
}

/// The kind of popup menu opening in a window of class `class` captioned `title`: an Office
/// command bar's popup (`MsoCommandBarPopup`) is named by its command bar, "Ply" [obs,
/// 1116/s5]; Office's own menus are read their own way [obs, 1122/s2].
pub fn popup_kind(class: Option<&str>, title: Option<&str>) -> okur_core::PopupKind {
    use okur_core::PopupKind;
    let title = title.map(str::trim).filter(|title| !title.is_empty());
    match (class, title) {
        (Some("MsoCommandBarPopup"), Some(bar)) => PopupKind::CommandBar(bar.to_owned()),
        (Some(NETUI), _) => PopupKind::Office,
        _ => PopupKind::Plain,
    }
}

/// The facts a focus's classes and automation id tell the engine, on every focus `event`
/// carries.
pub fn add_class_facts(event: &mut okur_core::AccessibilityEvent) {
    use okur_core::AccessibilityEvent;
    let focus = match event {
        AccessibilityEvent::Focus(focus)
        | AccessibilityEvent::Change { control: focus, .. }
        | AccessibilityEvent::Selected(focus)
        | AccessibilityEvent::WindowShown(focus)
        | AccessibilityEvent::ProgressChanged { bar: focus, .. } => focus,
        AccessibilityEvent::MousePoint(point) => match point.control.as_mut() {
            Some(focus) => focus,
            None => return,
        },
        _ => return,
    };
    let ids = &focus.platform;
    // A WinUI toggle switch: Windows Settings' "Night light Button | Off" (JAWS, dev box
    // 2026-10-16).
    focus.switch = ids.element_class.as_deref() == Some("ToggleSwitch");
    // IDCANCEL: a Win32 control's automation id is its control id.
    focus.cancel_button = ids.automation_id.as_deref() == Some("2");
    focus.on_menu_window = ids.window_class.as_deref() == Some(MENU);
    // Office's own dialogs share the start of their window class, their application's after
    // it (Word's `bosa_sdm_msword`, Excel's `bosa_sdm_XL9`).
    focus.pages_by_control_tab = ids
        .top_window_class
        .as_deref()
        .is_some_and(|class| class.starts_with("bosa_sdm_"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use okur_core::{AccessibilityEvent, FocusEvent, PopupKind, Role};

    fn facts(set: impl Fn(&mut okur_core::PlatformIds)) -> Option<FocusEvent> {
        let mut focus = FocusEvent::new(Role::Button);
        set(&mut focus.platform);
        let mut event = AccessibilityEvent::Change {
            kind: okur_core::ChangeKind::State,
            control: focus,
        };
        add_class_facts(&mut event);
        match event {
            AccessibilityEvent::Change { control, .. } => Some(control),
            _ => None,
        }
    }

    #[test]
    fn a_focus_carries_the_facts_its_classes_tell() {
        let plain = facts(|ids| {
            ids.element_class = Some("Button".into());
            ids.automation_id = Some("1".into());
            ids.window_class = Some("#32770".into());
            ids.top_window_class = Some("#32770".into());
        });
        assert!(plain.is_some_and(|f| {
            !f.switch && !f.cancel_button && !f.on_menu_window && !f.pages_by_control_tab
        }));
        assert!(
            facts(|ids| ids.element_class = Some("ToggleSwitch".into())).is_some_and(|f| f.switch)
        );
        assert!(facts(|ids| ids.automation_id = Some("2".into())).is_some_and(|f| f.cancel_button));
        assert!(
            facts(|ids| ids.window_class = Some("#32768".into())).is_some_and(|f| f.on_menu_window)
        );
        assert!(
            facts(|ids| ids.top_window_class = Some("bosa_sdm_XL9".into()))
                .is_some_and(|f| f.pages_by_control_tab)
        );
        assert!(
            facts(|ids| ids.top_window_class = Some("bosa_sdm_msword".into()))
                .is_some_and(|f| f.pages_by_control_tab)
        );
    }

    #[test]
    fn a_menu_says_its_kind_by_its_window() {
        assert!(is_launcher(Some("LauncherTipWnd")));
        assert!(!is_launcher(Some("#32768")) && !is_launcher(None));
        assert_eq!(
            popup_kind(Some("MsoCommandBarPopup"), Some(" Ply ")),
            PopupKind::CommandBar("Ply".into())
        );
        assert_eq!(
            popup_kind(Some("MsoCommandBarPopup"), Some(" ")),
            PopupKind::Plain
        );
        assert_eq!(popup_kind(Some(NETUI), None), PopupKind::Office);
        assert_eq!(
            popup_kind(Some("#32768"), Some("Context")),
            PopupKind::Plain
        );
    }
}
