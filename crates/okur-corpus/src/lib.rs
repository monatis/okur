pub mod invariants;
pub mod keys;
pub mod latency;
pub mod live;
pub mod manifest;
pub mod parity;
pub mod scenario;
pub mod snapshots;

pub use scenario::{Scenario, ScenarioReset, ScenarioStep};

pub fn init() {
    tracing::info!("okur-corpus initialized");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn corpus_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
    }

    /// Engine speech vs. recorded JAWS speech, over UIA snapshots captured from Chrome on
    /// the reference machine. Fails on any regression and whenever the generated baseline
    /// is stale; run `cargo run -p okur -- parity` for the per-step diff.
    #[test]
    fn jaws_parity_matches_baseline() {
        let dir = corpus_dir();
        let report = parity::run(&dir);
        assert!(
            report.errors.is_empty(),
            "scenarios could not run: {:?}",
            report.errors
        );
        assert_eq!(parity::check_baseline(&dir, &report), Ok(()));
    }

    /// Universal buffer invariants over every captured snapshot. Known violations are listed
    /// in the generated baseline; any new one fails, and fixing one requires regenerating it.
    #[test]
    fn buffer_invariants_match_baseline() {
        let dir = corpus_dir();
        let results = invariants::run_all(&dir);
        assert!(results.is_ok(), "{results:?}");
        if let Ok(results) = results {
            assert_eq!(invariants::check_baseline(&dir, &results), Ok(()));
        }
    }

    fn snapshot_aom(name: &str, api: snapshots::Api) -> okur_core::AomNode {
        let path = snapshots::path(&corpus_dir(), &format!("{name}.html"), api);
        let loaded = snapshots::load(&path);
        assert!(loaded.is_ok(), "{:?}", loaded.as_ref().err());
        loaded
            .map(|l| l.aom)
            .unwrap_or_else(|_| okur_core::AomNode::new(0, okur_core::Role::Generic))
    }

    fn find<'a>(
        node: &'a okur_core::AomNode,
        pred: &dyn Fn(&okur_core::AomNode) -> bool,
    ) -> Vec<&'a okur_core::AomNode> {
        let mut out = Vec::new();
        if pred(node) {
            out.push(node);
        }
        for c in &node.children {
            out.extend(find(c, pred));
        }
        out
    }

    /// The UIA→AOM mapping on real Chrome data: semantics come from control types, ARIA
    /// roles and UIA properties (old-pc's localized names are Turkish and must not matter).
    #[test]
    fn snapshot_mapping_preserves_chrome_semantics() {
        for api in [snapshots::Api::Uia, snapshots::Api::Ia2] {
            mapping_preserves_chrome_semantics(api);
        }
    }

    fn mapping_preserves_chrome_semantics(api: snapshots::Api) {
        use okur_core::Role;
        let doc = snapshot_aom("structural-navigation", api);
        let levels: Vec<u32> = find(&doc, &|n| matches!(n.role, Role::Heading { .. }))
            .iter()
            .filter_map(|n| match n.role {
                Role::Heading { level } => Some(level),
                _ => None,
            })
            .collect();
        assert_eq!(levels, vec![1, 2, 3, 2, 2]);
        let hrefs: Vec<Option<String>> = find(&doc, &|n| matches!(n.role, Role::Link { .. }))
            .iter()
            .map(|n| match &n.role {
                Role::Link { href } => href.clone(),
                _ => None,
            })
            .collect();
        assert_eq!(hrefs.len(), 3);
        assert!(hrefs.iter().all(|h| h
            .as_deref()
            .is_some_and(|h| h.contains("structural-navigation.html#"))));
        let landmarks: Vec<String> = find(&doc, &|n| matches!(n.role, Role::Landmark { .. }))
            .iter()
            .filter_map(|n| match &n.role {
                Role::Landmark { landmark_type } => Some(landmark_type.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            landmarks,
            vec![
                "banner",
                "navigation",
                "main",
                "complementary",
                "contentinfo"
            ]
        );

        let forms = snapshot_aom("forms-interaction", api);
        let checkbox = find(&forms, &|n| n.role == Role::CheckBox);
        assert_eq!(checkbox.len(), 1);
        assert_eq!(checkbox.first().and_then(|c| c.states.checked), Some(false));
        let edits = find(&forms, &|n| n.role == Role::Edit);
        assert_eq!(
            edits.iter().map(|e| e.value.as_deref()).collect::<Vec<_>>(),
            vec![Some("Jane Doe"), Some("jane@example.com")]
        );

        let tables = snapshot_aom("tables-navigation", api);
        let col_headers = find(&tables, &|n| {
            n.attributes.get("scope").map(String::as_str) == Some("col")
        });
        assert_eq!(col_headers.len(), 4);
        let row_headers = find(&tables, &|n| {
            n.attributes.get("scope").map(String::as_str) == Some("row")
        });
        assert_eq!(row_headers.len(), 3);
    }

    /// E moves to an edit; Enter must put real focus in it (the user reported typing went
    /// nowhere). The request names the IA2 object, so the platform can resolve it.
    #[test]
    fn enter_on_an_edit_focuses_it_in_the_page() {
        use okur_core::actions::RecordingActions;
        use okur_core::{KeyEvent, MockSpeechOutput, Modifiers, ScreenReaderEngine};
        let page = snapshot_aom("signup-form", snapshots::Api::Ia2);
        let mut cursor = okur_core::VirtualCursor::new(okur_core::VirtualBuffer::from_aom(&page));
        cursor.move_to_start();
        let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
        engine.app_mode = okur_core::engine::AppMode::VirtualBuffer;
        let actions = RecordingActions::default();
        engine.set_actions(Box::new(actions.clone()));

        let press = |engine: &mut ScreenReaderEngine, key: okur_core::Key| {
            engine.handle_key_event(&KeyEvent::new(key, Modifiers::none(), true))
        };
        press(&mut engine, okur_core::Key::Alpha('e'));
        let action = press(&mut engine, okur_core::Key::Enter);
        assert_eq!(action, okur_core::input::KeyAction::Consume);
        assert!(engine.forms_mode);

        let requests = actions.requests();
        assert_eq!(requests.len(), 1, "{requests:?}");
        let Some((kind, node_id)) = requests.first() else {
            return;
        };
        assert_eq!(kind, "focus");
        let node = engine.cursor.buffer.node(*node_id);
        assert_eq!(node.and_then(|n| n.name.as_deref()), Some("Full name"));

        // In forms mode, typing reaches the page.
        let typed = press(&mut engine, okur_core::Key::Alpha('a'));
        assert_eq!(typed, okur_core::input::KeyAction::PassThrough);

        // Caret keys reach the field too; each asks the platform for the field's new text, and
        // the speech comes from the text the field then reports (the user reported arrow keys
        // read nothing in an edit).
        use okur_core::{AccessibilityEvent, TextContext};
        let report = |engine: &mut ScreenReaderEngine, text: &str, caret: usize| {
            engine.handle_accessibility_event(&AccessibilityEvent::TextContext(TextContext {
                text: text.to_owned(),
                caret,
                selection: None,
                font: None,
                word: None,
            }))
        };
        report(&mut engine, "Zed", 3);
        let left = press(&mut engine, okur_core::Key::LeftArrow);
        assert_eq!(left, okur_core::input::KeyAction::PassThrough);
        assert_eq!(report(&mut engine, "Zed", 2).as_deref(), Some("d"));
        let mut ctrl = Modifiers::none();
        ctrl.ctrl = true;
        engine.handle_key_event(&KeyEvent::new(okur_core::Key::LeftArrow, ctrl, true));
        assert_eq!(report(&mut engine, "Zed", 0).as_deref(), Some("Zed"));
        press(&mut engine, okur_core::Key::UpArrow);
        assert_eq!(report(&mut engine, "Zed", 0).as_deref(), Some("Zed"));
        // A report without a pending key (the application changed the text) is not spoken.
        assert_eq!(report(&mut engine, "Zed!", 4), None);
        let kinds: Vec<String> = actions.requests().into_iter().map(|(k, _)| k).collect();
        assert_eq!(kinds.first().map(String::as_str), Some("focus"));
        assert_eq!(kinds.len(), 4);
    }
    /// Every kind of text field maps to an edit through both APIs, so Enter enters forms
    /// mode and caret keys read text: input, textarea, a WhatsApp-style contenteditable with
    /// role=textbox, and a plain contenteditable (the user's WhatsApp Web report).
    #[test]
    fn every_editing_shape_is_an_edit() {
        for api in [snapshots::Api::Ia2, snapshots::Api::Uia] {
            let page = snapshot_aom("editing", api);
            let edits: Vec<(Option<String>, Option<String>)> =
                find(&page, &|n| n.role == okur_core::Role::Edit)
                    .iter()
                    .map(|n| (n.name.clone(), n.value.clone()))
                    .collect();
            let names: Vec<&str> = edits.iter().filter_map(|(n, _)| n.as_deref()).collect();
            assert_eq!(
                names,
                vec!["Subject", "Body", "Type a message", "Notes"],
                "{api:?}"
            );
            let notes = edits.iter().find(|(n, _)| n.as_deref() == Some("Notes"));
            assert_eq!(
                notes.and_then(|(_, v)| v.as_deref()),
                Some("Plain editable notes"),
                "{api:?}"
            );
        }
    }

    /// Actions resolve nodes through their IA2 uniqueID, so every form control must carry one.
    #[test]
    fn ia2_snapshots_carry_unique_ids() {
        let form = snapshot_aom("signup-form", snapshots::Api::Ia2);
        let controls = find(&form, &|n| {
            matches!(
                n.role,
                okur_core::Role::Edit | okur_core::Role::CheckBox | okur_core::Role::Button
            )
        });
        assert!(!controls.is_empty());
        assert!(controls
            .iter()
            .all(|n| okur_platform::actions::unique_id(n).is_some()));
    }

    /// Semantics only IA2 carries without localization: block quotes (UIA gives only a
    /// localized control type), inline versus block elements, and the IA2 form states.
    #[test]
    fn ia2_mapping_adds_semantics_uia_lacks() {
        use okur_core::Role;
        let article = snapshot_aom("news-article", snapshots::Api::Ia2);
        let quotes = find(&article, &|n| n.role == Role::BlockQuote);
        assert_eq!(quotes.len(), 1);
        let articles = find(&article, &|n| {
            n.attributes.get("aria_role").map(String::as_str) == Some("article")
        });
        assert_eq!(articles.len(), 1);

        let form = snapshot_aom("signup-form", snapshots::Api::Ia2);
        let required: Vec<_> = find(&form, &|n| {
            n.attributes.get("required").map(String::as_str) == Some("true")
        })
        .iter()
        .filter_map(|n| n.name.clone())
        .collect();
        assert_eq!(required, vec!["Full name", "Password"]);
        let passwords = find(&form, &|n| n.attributes.contains_key("password"));
        assert_eq!(passwords.len(), 1);
        let radios = find(&form, &|n| n.role == Role::RadioButton);
        assert_eq!(
            radios.iter().map(|r| r.states.checked).collect::<Vec<_>>(),
            vec![Some(true), Some(false), Some(false)]
        );
        let described = find(&form, &|n| n.attributes.contains_key("description"));
        assert_eq!(
            described
                .first()
                .and_then(|n| n.attributes.get("description")),
            Some(&"We never share your email.".to_owned())
        );
    }

    /// Focus events built from a chain of real Chrome UIA nodes, as trace replay and the
    /// live listener build them: the radio button inside the "Membership type" fieldset.
    #[test]
    fn focus_event_from_chrome_focus_chain() {
        use okur_platform::uia_snapshot::UiaNode;
        let path = corpus_dir().join("snapshots/signup-form.chrome.uia.json");
        let snapshot = okur_platform::uia_snapshot::UiaSnapshot::load(&path);
        assert!(snapshot.is_ok(), "{snapshot:?}");
        let Ok(snapshot) = snapshot else { return };

        // Path from the document to the first radio button.
        fn path_to<'a>(node: &'a UiaNode, pred: &dyn Fn(&UiaNode) -> bool) -> Vec<&'a UiaNode> {
            if pred(node) {
                return vec![node];
            }
            for c in &node.children {
                let mut p = path_to(c, pred);
                if !p.is_empty() {
                    p.insert(0, node);
                    return p;
                }
            }
            Vec::new()
        }
        let chain: Vec<UiaNode> = path_to(&snapshot.root, &|n| n.control_type == 50013)
            .into_iter()
            .cloned()
            .collect();
        assert!(chain.len() >= 3, "radio button not found");

        let event = okur_platform::event_trace::focus_event_from_chain(
            &chain,
            Some("Chrome_RenderWidgetHostHWND"),
            None,
        );
        assert!(event.is_some());
        let Some(event) = event else { return };
        assert_eq!(event.role, okur_core::Role::RadioButton);
        assert_eq!(event.name.as_deref(), Some("Adult"));
        assert_eq!(event.states.checked, Some(true));
        assert_eq!((event.item_index, event.item_count), (Some(1), Some(3)));
        assert_eq!(event.container_role, Some(okur_core::Role::Group));
        assert_eq!(event.container_name.as_deref(), Some("Membership type"));
    }

    /// `corpus/CORPUS_MANIFEST.json` must be exactly what the generator derives from disk.
    #[test]
    fn manifest_is_current() {
        assert_eq!(manifest::check(&corpus_dir()), Ok(()));
    }
}
