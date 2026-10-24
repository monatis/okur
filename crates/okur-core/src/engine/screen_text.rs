//! The Okur cursor's model of the window in front, and recognizing the screen's text into it
//! (Insert+Space, O; `docs/plans/2026-10-23-okur-cursor-screen-text.md`). Built as an
//! experiment, and Okur's way since the owner counted it proven (2026-10-24).
//!
//! JAWS's cursor reads only what its screen capture catches: with Windows 10's driver neither a
//! WinForms label nor GDI+ text, with either method no picture [obs, 994, 995]. Its Convenient
//! OCR (W, S, C) puts the recognized words among its cursor's lines, saying "OCR started using
//! OmniPage.", "finished." and "JAWS Cursor"; the cursor is then unrestricted, and the words go
//! when the PC cursor returns [obs, 996, 997]. Okur's model reads the window's text where the
//! accessibility interfaces place it, and adds what Windows' own recognizer reads in the
//! window's pixels where no accessible text lies.

use super::cursors::{ActiveCursor, OkurPlace, Restriction};
use super::ScreenReaderEngine;
use crate::actions::RecognitionArea;
use crate::input::{Key, KeyAction, KeyEvent};
use crate::messages::msg;
use crate::recognized_file::FileRecognition;
use crate::screen_model::{ScreenLine, ScreenModel, ScreenRun, WindowText};

impl ScreenReaderEngine {
    /// The language the screen is recognized in: the setting's, else Okur's own.
    fn ocr_language(&self) -> String {
        match self.settings.ocr_language.trim() {
            "" => crate::messages::SOURCE_LOCALE.to_owned(),
            tag => tag.to_owned(),
        }
    }

    /// Asks for the window's text for the Okur cursor's model, outside pages, whose lines the
    /// Okur cursor already reads from the page.
    pub(super) fn request_screen(&mut self) {
        if self.app_mode == super::AppMode::VirtualBuffer {
            return;
        }
        if let Some(actions) = &self.actions {
            actions.read_screen();
        }
    }

    /// Whether the Okur cursor reads the window element by element, as JAWS's Scan Cursor reads
    /// an application its screen capture cannot: "JAWS scan cursor" in a WPF window [obs,
    /// 1002/s2]. Words recognized there are read where they are drawn, by the Okur cursor.
    pub(super) fn okur_scans(&self) -> bool {
        self.app_mode != super::AppMode::VirtualBuffer
            && self
                .last_native_focus
                .as_ref()
                .is_some_and(|focus| focus.read_by_elements)
            && !self
                .screen_model
                .as_ref()
                .is_some_and(|model| !model.recognized().is_empty())
    }

    /// The window's text arrived: the model's accessible text, its recognized words kept while
    /// the window is the same.
    pub(super) fn screen_text(&mut self, text: &WindowText) -> Option<String> {
        let window = self.last_window_title.clone();
        self.screen_model = Some(
            match self
                .screen_model
                .take()
                .filter(|model| model.window == window)
            {
                Some(model) => model.with_text(text.clone()),
                None => ScreenModel::new(window, text.clone()),
            },
        );
        // Routed before the window's text came, it is routed again by it [obs, 1010/s3]; made
        // active where nothing placed it, it is where the mouse is.
        if std::mem::take(&mut self.route_awaits_screen) {
            self.okur_place = self.routed_place();
        } else if self.active_cursor == ActiveCursor::Okur && self.okur_place.is_none() {
            self.okur_place = Some(self.place_at_mouse());
        }
        None
    }

    /// The Okur cursor's place where the mouse is: the screen model's line there, else the
    /// window's top.
    fn place_at_mouse(&self) -> OkurPlace {
        let line = self.mouse.and_then(|point| {
            self.okur_screen_lines()?
                .iter()
                .position(|line| line.holds(point))
        });
        line.map_or(OkurPlace::Top, |line| OkurPlace::Text {
            line,
            column: None,
        })
    }

    /// Another window is in front: its model, recognized words and all, is forgotten, as
    /// JAWS's recognized text goes when another window comes to the front (JAWS 2025 help,
    /// Convenient OCR).
    pub(super) fn forget_other_windows_screen(&mut self) {
        if self
            .screen_model
            .as_ref()
            .is_some_and(|model| model.window != self.last_window_title)
        {
            self.screen_model = None;
            // Background OCR stops when another window gets the focus (JAWS 2025 help).
            self.background_ocr = None;
        }
    }

    /// The PC cursor is back: the recognized words go [obs, 996/s15-s19].
    pub(super) fn forget_recognized(&mut self) {
        if let Some(model) = self
            .screen_model
            .as_mut()
            .filter(|model| !model.recognized().is_empty())
        {
            model.set_recognized(Vec::new());
        }
    }

    /// The model's lines, when they are what the Okur cursor reads: for the window in front,
    /// outside pages, whose lines it reads from the page, and outside dialog boxes, whose rows
    /// it reads with their controls' roles as JAWS's cursor does ("Go To Button Cancel Button"
    /// [obs, 755/s10]), unless words were recognized there.
    pub(super) fn okur_screen_lines(&self) -> Option<std::sync::Arc<[ScreenLine]>> {
        if self.app_mode == super::AppMode::VirtualBuffer {
            return None;
        }
        let model = self
            .screen_model
            .as_ref()
            .filter(|model| model.window == self.last_window_title)?;
        if model.recognized().is_empty() && self.okur_dialog_rows().is_some() {
            return None;
        }
        let lines = model.lines();
        (!lines.is_empty()).then_some(lines)
    }

    /// Insert+Space, O: "OCR", and the next key is the layer's [obs, 996/s2].
    pub(super) fn start_ocr_layer(&mut self) {
        self.ocr_layer = true;
        self.speech.speak(msg::ocr_layer().as_str(), true);
    }

    /// The OCR layer's key, when the layer is open; `None` leaves `event` to the rest of the
    /// engine. A modifier pressed on its own keeps the layer open.
    pub(super) fn ocr_layer_key(&mut self, event: &KeyEvent) -> Option<KeyAction> {
        if !self.ocr_layer
            || !event.is_down
            || matches!(
                event.key,
                Key::Shift | Key::Control | Key::Alt | Key::Insert | Key::CapsLock | Key::Windows
            )
        {
            return None;
        }
        self.ocr_layer = false;
        if event.modifiers.is_empty() {
            match event.key {
                Key::Alpha('W') => self.start_recognition(RecognitionArea::Window),
                // OCRAllInOne, in an application without its own case: the window (JAWS 2025
                // help, Convenient OCR).
                Key::Enter => self.start_recognition(RecognitionArea::Window),
                Key::Alpha('B') => self.start_background_ocr(),
                Key::Alpha('S') => self.start_recognition(RecognitionArea::Screen),
                Key::Alpha('C') => {
                    if let Some([left, top, right, bottom]) = self.focus_bounds {
                        self.start_recognition(RecognitionArea::Rect(left, top, right, bottom));
                    }
                }
                Key::Alpha('Q') => self.cancel_recognition(),
                Key::Alpha('F') => self.start_file_recognition(),
                _ => {}
            }
        }
        Some(KeyAction::Consume)
    }

    /// W, S or C: recognition starts, "OCR started using Microsoft OCR." [obs, 997/s3], and
    /// the window's accessible text is read again to lie beside the recognized words.
    fn start_recognition(&mut self, area: RecognitionArea) {
        let language = self.ocr_language();
        if !self
            .actions
            .as_ref()
            .is_some_and(|a| a.recognize(area, &language))
        {
            return;
        }
        self.recognizing = true;
        self.recognition_window = self.last_window_title.clone();
        let started = msg::ocr_started(msg::ocr_engine_windows().to_string()).render();
        self.speech.speak(&started, true);
        self.request_screen();
    }

    /// Q: Background OCR stops, "Background OCR cancelled" [obs, 1001/s14]; a recognition
    /// under way is dropped, "OCR cancelled." [obs, 1006/s3].
    fn cancel_recognition(&mut self) {
        if self.background_ocr.take().is_some() {
            self.speech
                .speak(msg::ocr_background_cancelled().as_str(), true);
            return;
        }
        if std::mem::take(&mut self.recognizing_file) {
            if let Some(actions) = self.actions.as_ref() {
                actions.cancel_file_recognition();
            }
            self.speech.speak(msg::ocr_canceled().as_str(), true);
            return;
        }
        if std::mem::take(&mut self.recognizing) {
            self.speech.speak(msg::ocr_canceled().as_str(), true);
        }
    }

    /// F: the picture or PDF selected in File Explorer is recognized, its text to the Results
    /// Viewer (JAWS 2025 help, Convenient OCR).
    fn start_file_recognition(&mut self) {
        let language = self.ocr_language();
        if !self
            .actions
            .as_ref()
            .is_some_and(|a| a.recognize_selected_file(&language))
        {
            return;
        }
        self.recognizing_file = true;
        let started = msg::ocr_started(msg::ocr_engine_windows().to_string()).render();
        self.speech.speak(&started, true);
    }

    /// The selected file's text arrived: it opens in the Results Viewer, at its first line.
    pub(super) fn file_recognized(&mut self, recognition: &FileRecognition) -> Option<String> {
        if !std::mem::take(&mut self.recognizing_file) {
            return None;
        }
        let FileRecognition::Read { .. } = recognition else {
            return None;
        };
        let viewer =
            crate::viewer::VirtualViewer::new(recognition.lines().map(str::to_owned).collect());
        let speech = viewer.line_text();
        self.viewer = Some(viewer);
        self.show_viewer_window();
        self.speech.speak(&speech, true);
        Some(speech)
    }

    /// B: "Background OCR started" [obs, 1001/s3], and the window is recognized again and again
    /// ([`Self::background_ocr_idle`]), its words read with the Okur cursor as they change.
    /// The cursors stay as they are: the PC cursor keeps the focus [obs, 1001/s4].
    fn start_background_ocr(&mut self) {
        let language = self.ocr_language();
        if !self
            .actions
            .as_ref()
            .is_some_and(|a| a.recognize(RecognitionArea::Window, &language))
        {
            return;
        }
        self.background_ocr = Some((std::time::Instant::now(), true));
        self.recognition_window = self.last_window_title.clone();
        self.speech
            .speak(msg::ocr_background_started().as_str(), true);
        self.request_screen();
    }

    /// While Background OCR runs, the window is recognized again once the last recognition
    /// came back and this long has passed. JAWS says nothing unasked as its text changes
    /// [obs, 1001/s9, 1004/s10, s12].
    const BACKGROUND_OCR_EVERY: std::time::Duration = std::time::Duration::from_secs(2);

    /// The idle tick of Background OCR: the next recognition, when it is due.
    pub(super) fn background_ocr_idle(&mut self) {
        let Some((asked, false)) = self.background_ocr else {
            return;
        };
        // Nor while an edit key's read is pending, which the same worker answers (performance
        // review 2026-10-24, 2).
        if asked.elapsed() < Self::BACKGROUND_OCR_EVERY || self.pending_edit.is_some() {
            return;
        }
        let language = self.ocr_language();
        if self
            .actions
            .as_ref()
            .is_some_and(|a| a.recognize(RecognitionArea::Window, &language))
        {
            self.background_ocr = Some((std::time::Instant::now(), true));
        }
    }

    /// The recognized words arrived: "finished.", and the Okur cursor is active and
    /// unrestricted on them, "Okur Cursor", where JAWS says "JAWS Cursor" [obs, 996/s3, s4],
    /// on the line the mouse is on, else at the window's top.
    pub(super) fn recognized(&mut self, words: &[ScreenRun]) -> Option<String> {
        let window = self.last_window_title.clone();
        // Words of a window no longer in front are not this one's: a recognition queued behind
        // a long read can come back after another window took the focus (performance review
        // 2026-10-24, 9d).
        let same_window = self.recognition_window == window;
        if !std::mem::take(&mut self.recognizing) {
            // Background OCR's words replace the last ones, unsaid; its next recognition is due
            // two seconds from now.
            if let Some((_, true)) = self.background_ocr {
                self.background_ocr = Some((std::time::Instant::now(), false));
                if same_window {
                    match &mut self.screen_model {
                        Some(model) if model.window == window => {
                            model.set_recognized(words.to_vec())
                        }
                        _ => {
                            self.screen_model = Some(
                                ScreenModel::new(window, WindowText::default())
                                    .with_recognized(words.to_vec()),
                            )
                        }
                    }
                }
            }
            return None;
        }
        let finished = msg::ocr_finished().to_string();
        self.speech.speak(&finished, false);
        if !same_window {
            return Some(finished);
        }
        let model = self
            .screen_model
            .take()
            .filter(|model| model.window == window)
            .unwrap_or_else(|| ScreenModel::new(window, WindowText::default()));
        self.screen_model = Some(model.with_recognized(words.to_vec()));
        self.active_cursor = ActiveCursor::Okur;
        self.okur_restriction = Restriction::Unrestricted;
        let line = self.mouse.and_then(|point| {
            self.okur_screen_lines()?
                .iter()
                .position(|line| line.holds(point))
        });
        self.okur_place = Some(line.map_or(OkurPlace::Top, |line| OkurPlace::Text {
            line,
            column: None,
        }));
        let cursor = self.cursor_message(msg::cursor_okur(), msg::cursor_okur_short());
        self.speech.speak(&cursor, false);
        Some(format!("{finished}\n{cursor}"))
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::RecordingActions;
    use crate::aom::{AomNode, Role};
    use crate::buffer::VirtualBuffer;
    use crate::cursor::VirtualCursor;
    use crate::engine::tests::SharedSpeech;
    use crate::engine::ScreenReaderEngine;
    use crate::event::{AccessibilityEvent, FocusEvent};
    use crate::input::{Key, KeyEvent, Modifiers};
    use crate::screen_model::{ScreenRun, Source, WindowText};

    const TITLE: &str = "Drawn Text";

    /// A desktop window with its edit focused.
    fn window() -> (ScreenReaderEngine, SharedSpeech, RecordingActions) {
        let speech = SharedSpeech::default();
        let mut engine = ScreenReaderEngine::new(
            VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document))),
            Box::new(speech.clone()),
        );
        engine.set_settings(crate::settings::Settings::reference_machine());
        let actions = RecordingActions::default();
        engine.set_actions(Box::new(actions.clone()));
        let mut edit = FocusEvent::new(Role::Edit)
            .with_name("Name")
            .with_window_title(TITLE);
        edit.bounds = Some([20, 340, 320, 380]);
        edit.text = Some(crate::event::TextContext {
            text: "Focus here".into(),
            caret: 0,
            selection: None,
            font: None,
            word: None,
            misspelled: Vec::new(),
        });
        engine.handle_accessibility_event(&AccessibilityEvent::Focus(edit));
        let _ = speech.take();
        (engine, speech, actions)
    }

    fn press(
        engine: &mut ScreenReaderEngine,
        speech: &SharedSpeech,
        key: Key,
        modifiers: Modifiers,
    ) -> String {
        let _ = speech.take();
        engine.handle_key_event(&KeyEvent::new(key, modifiers, true));
        speech.take().join("\n")
    }

    fn plain() -> Modifiers {
        Modifiers::none()
    }

    fn insert() -> Modifiers {
        Modifiers {
            insert: true,
            ..Modifiers::none()
        }
    }

    /// What the accessibility interfaces place on the window: the label and the edit; the
    /// panel's painted text and the picture have none.
    fn accessible() -> Vec<ScreenRun> {
        vec![
            ScreenRun::new("Label drawn by GDI", (20, 16, 240, 48), Source::Accessible),
            ScreenRun::new("Focus here", (24, 344, 316, 376), Source::Accessible),
        ]
    }

    fn recognized() -> Vec<ScreenRun> {
        let word = |text: &str, left: i32, right: i32, top: i32| {
            ScreenRun::new(text, (left, top, right, top + 28), Source::Recognized)
        };
        vec![
            word("Painted", 20, 110, 80),
            word("text", 120, 170, 80),
            word("Start", 40, 100, 276),
            word("Focus", 26, 90, 346),
            word("here", 96, 150, 346),
        ]
    }

    /// Until the window's text comes, the Okur cursor reads the field as without it: routed to
    /// it, the field's line [obs, 690/s3].
    #[test]
    fn without_the_windows_text_the_field_is_read() {
        let (mut engine, speech, actions) = window();
        press(&mut engine, &speech, Key::NumpadMinus, insert());
        press(&mut engine, &speech, Key::NumpadMinus, plain());
        assert!(actions.requests().iter().any(|(a, _)| a == "read screen"));
        assert_eq!(
            press(&mut engine, &speech, Key::UpArrow, insert()),
            "Focus here"
        );
    }

    /// On: Numpad Minus reads the window, and the Okur cursor reads its lines from the top,
    /// the label the field's lines never had; the mouse follows each line.
    #[test]
    fn on_the_okur_cursor_reads_the_windows_lines() {
        let (mut engine, speech, actions) = window();
        press(&mut engine, &speech, Key::NumpadMinus, plain());
        assert!(actions.requests().iter().any(|(a, _)| a == "read screen"));
        engine.handle_accessibility_event(&AccessibilityEvent::ScreenText(WindowText::rows(
            accessible(),
        )));
        press(&mut engine, &speech, Key::PageUp, plain());
        assert_eq!(press(&mut engine, &speech, Key::UpArrow, insert()), TITLE);
        assert_eq!(
            press(&mut engine, &speech, Key::DownArrow, plain()),
            "Label drawn by GDI"
        );
        assert_eq!(
            press(&mut engine, &speech, Key::DownArrow, plain()),
            "Focus here"
        );
        assert_eq!(press(&mut engine, &speech, Key::RightArrow, plain()), "F");
        let moved: Vec<String> = actions
            .requests()
            .into_iter()
            .map(|(a, _)| a)
            .filter(|a| a.starts_with("mouse to "))
            .collect();
        // The label's first character, the field's, then the field's "F" again.
        assert_eq!(
            moved,
            ["mouse to 26, 32", "mouse to 38, 360", "mouse to 38, 360"]
        );
    }

    /// Insert+Space, O, W: "OCR", "OCR started using Microsoft OCR.", then with the words
    /// "finished." and "Okur Cursor"; the Okur cursor, unrestricted, reads the recognized words
    /// among the window's lines, none said twice, and they go when the PC cursor returns
    /// [obs, 996/s2-s4, s15-s19; 997/s3].
    #[test]
    fn recognized_words_join_the_okur_cursors_lines_until_the_pc_cursor() {
        let (mut engine, speech, actions) = window();
        press(&mut engine, &speech, Key::Space, insert());
        assert_eq!(press(&mut engine, &speech, Key::Alpha('O'), plain()), "OCR");
        assert_eq!(
            press(&mut engine, &speech, Key::Alpha('W'), plain()),
            "OCR started using Microsoft OCR."
        );
        assert!(actions
            .requests()
            .iter()
            .any(|(a, _)| a == "recognize Window"));
        engine.handle_accessibility_event(&AccessibilityEvent::ScreenText(WindowText::rows(
            accessible(),
        )));
        let _ = speech.take();
        engine.handle_accessibility_event(&AccessibilityEvent::Recognized(recognized()));
        assert_eq!(speech.take().join("\n"), "finished.\nOkur Cursor");
        assert_eq!(
            engine.okur_restriction,
            crate::engine::cursors::Restriction::Unrestricted
        );
        press(&mut engine, &speech, Key::PageUp, plain());
        let lines: Vec<String> = (0..4)
            .map(|_| press(&mut engine, &speech, Key::DownArrow, plain()))
            .collect();
        assert_eq!(
            lines,
            ["Label drawn by GDI", "Painted text", "Start", "Focus here"]
        );
        // Numpad Slash clicks the recognized word the Okur cursor is on.
        press(&mut engine, &speech, Key::UpArrow, plain());
        press(&mut engine, &speech, Key::NumpadSlash, plain());
        let requests: Vec<String> = actions.requests().into_iter().map(|(a, _)| a).collect();
        let clicked = requests.iter().rposition(|a| a.contains("click at"));
        let moved = requests.iter().rposition(|a| a == "mouse to 46, 290");
        assert!(moved.is_some() && clicked > moved, "{requests:?}");
        // The PC cursor, then the Okur cursor again: the window's own lines.
        press(&mut engine, &speech, Key::NumpadPlus, plain());
        press(&mut engine, &speech, Key::NumpadMinus, plain());
        engine.handle_accessibility_event(&AccessibilityEvent::ScreenText(WindowText::rows(
            accessible(),
        )));
        press(&mut engine, &speech, Key::PageUp, plain());
        assert_eq!(
            press(&mut engine, &speech, Key::DownArrow, plain()),
            "Label drawn by GDI"
        );
        assert_eq!(
            press(&mut engine, &speech, Key::DownArrow, plain()),
            "Focus here"
        );
    }

    /// Insert+Space, O, B: "Background OCR started", the PC cursor still active; its words
    /// come unsaid, Alt+Delete says it runs, the window is recognized again once two seconds
    /// passed since the last, and Q stops it, "Background OCR cancelled" [obs, 1001/s3, s4,
    /// s9, s14].
    #[test]
    fn background_ocr_recognizes_again_until_cancelled() {
        let (mut engine, speech, actions) = window();
        let recognitions = |actions: &RecordingActions| {
            actions
                .requests()
                .iter()
                .filter(|(a, _)| a.starts_with("recognize"))
                .count()
        };
        press(&mut engine, &speech, Key::Space, insert());
        press(&mut engine, &speech, Key::Alpha('O'), plain());
        assert_eq!(
            press(&mut engine, &speech, Key::Alpha('B'), plain()),
            "Background OCR started"
        );
        assert_eq!(recognitions(&actions), 1);
        engine.handle_accessibility_event(&AccessibilityEvent::Recognized(recognized()));
        assert!(speech.take().is_empty());
        assert_eq!(
            engine.active_cursor,
            crate::engine::cursors::ActiveCursor::Pc
        );
        let said = press(
            &mut engine,
            &speech,
            Key::Delete,
            Modifiers {
                alt: true,
                ..Modifiers::none()
            },
        );
        assert!(said.ends_with("Background OCR is enabled"), "{said}");
        // Not yet due: no new recognition; due: one.
        let _ = engine.on_idle();
        assert_eq!(recognitions(&actions), 1);
        let long_ago = std::time::Instant::now()
            .checked_sub(std::time::Duration::from_secs(3))
            .unwrap_or_else(std::time::Instant::now);
        engine.background_ocr = Some((long_ago, false));
        let _ = engine.on_idle();
        assert_eq!(recognitions(&actions), 2);
        // The new words replace the old, unsaid.
        let changed = vec![ScreenRun::new(
            "Count 3",
            (20, 80, 120, 108),
            Source::Recognized,
        )];
        engine.handle_accessibility_event(&AccessibilityEvent::Recognized(changed.clone()));
        assert_eq!(
            engine
                .screen_model
                .as_ref()
                .map(|m| m.recognized().to_vec()),
            Some(changed)
        );
        press(&mut engine, &speech, Key::Space, insert());
        press(&mut engine, &speech, Key::Alpha('O'), plain());
        assert_eq!(
            press(&mut engine, &speech, Key::Alpha('Q'), plain()),
            "Background OCR cancelled"
        );
        assert!(engine.background_ocr.is_none());
    }

    /// Routed to the PC cursor before the window's text came, the Okur cursor is routed again
    /// once it comes: on the field's line, as JAWS's is, not on the window's first [obs,
    /// 1010/s1-s3].
    #[test]
    fn a_route_before_the_windows_text_is_routed_again_by_it() {
        let (mut engine, speech, _) = window();
        press(&mut engine, &speech, Key::NumpadMinus, insert());
        press(&mut engine, &speech, Key::NumpadMinus, plain());
        engine.handle_accessibility_event(&AccessibilityEvent::ScreenText(WindowText::rows(
            accessible(),
        )));
        assert_eq!(
            press(&mut engine, &speech, Key::UpArrow, insert()),
            "Focus here"
        );
    }

    /// A WPF window, read element by element as JAWS's Scan Cursor reads it: "Route Okur scan
    /// to PC", "Okur scan cursor", the focused edit with its role, each element a line, a name
    /// ending with its role without it, Up landing on the first character so Right says the
    /// second, and Alt+Delete without "active" [obs, 1002/s1-s13].
    #[test]
    fn a_modern_window_is_read_by_the_scan_cursor() {
        use crate::aom::Role as R;
        let speech = SharedSpeech::default();
        let mut engine = ScreenReaderEngine::new(
            VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document))),
            Box::new(speech.clone()),
        );
        engine.set_settings(crate::settings::Settings::reference_machine());
        engine.set_actions(Box::new(RecordingActions::default()));
        let mut edit = FocusEvent::new(Role::Edit).with_window_title("WPF Text");
        edit.bounds = Some([594, 515, 969, 553]);
        edit.read_by_elements = true;
        engine.handle_accessibility_event(&AccessibilityEvent::Focus(edit));
        let _ = speech.take();
        let run = |text: &str, rect: (i32, i32, i32, i32), role: R| {
            ScreenRun::new(text, rect, Source::Accessible).with_role(role)
        };
        let elements = WindowText::elements(vec![
            run("WPF text block", (594, 353, 1326, 388), R::Text),
            run("Left words", (594, 403, 714, 438), R::Text),
            run("Right words", (764, 403, 903, 438), R::Text),
            run("WPF button", (594, 453, 764, 500), R::Button),
            run("Focus here", (594, 515, 969, 553), R::Edit),
        ]);
        assert_eq!(
            press(&mut engine, &speech, Key::NumpadMinus, insert()),
            "Route Okur scan to PC"
        );
        assert_eq!(
            press(&mut engine, &speech, Key::NumpadMinus, plain()),
            "Okur scan cursor"
        );
        engine.handle_accessibility_event(&AccessibilityEvent::ScreenText(elements));
        assert_eq!(
            press(&mut engine, &speech, Key::UpArrow, insert()),
            "Focus here \u{1d} Edit \u{1e}"
        );
        let said = press(
            &mut engine,
            &speech,
            Key::Delete,
            Modifiers {
                alt: true,
                ..Modifiers::none()
            },
        );
        assert!(said.starts_with("Okur scan cursor\n"), "{said}");
        press(&mut engine, &speech, Key::PageUp, plain());
        let lines: Vec<String> = (0..4)
            .map(|_| press(&mut engine, &speech, Key::DownArrow, plain()))
            .collect();
        assert_eq!(
            lines,
            [
                "WPF text block",
                "Left words",
                "Right words",
                "WPF \u{1d} Button \u{1e}"
            ]
        );
        assert_eq!(
            press(&mut engine, &speech, Key::UpArrow, plain()),
            "Right words"
        );
        assert_eq!(press(&mut engine, &speech, Key::RightArrow, plain()), "i");
        assert_eq!(press(&mut engine, &speech, Key::RightArrow, plain()), "g");
    }

    /// The route takes the Scan Cursor to the element with the focus, though the focus event
    /// is the window's and its middle lies on another line; a list says "list view"
    /// [obs, 1022/s3, s14].
    #[test]
    fn the_scan_cursor_is_routed_to_the_focused_element() {
        use crate::aom::Role as R;
        let speech = SharedSpeech::default();
        let mut engine = ScreenReaderEngine::new(
            VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document))),
            Box::new(speech.clone()),
        );
        engine.set_settings(crate::settings::Settings::reference_machine());
        engine.set_actions(Box::new(RecordingActions::default()));
        let mut window = FocusEvent::new(Role::Window).with_window_title("WPF Controls");
        window.bounds = Some([560, 128, 1360, 903]);
        window.read_by_elements = true;
        engine.handle_accessibility_event(&AccessibilityEvent::Focus(window));
        let run = |text: &str, rect: (i32, i32, i32, i32), role: R| {
            ScreenRun::new(text, rect, Source::Accessible).with_role(role)
        };
        let mut edit = run("Focus here", (594, 215, 969, 253), R::Edit);
        edit.focused = true;
        engine.handle_accessibility_event(&AccessibilityEvent::ScreenText(WindowText::elements(
            vec![
                run("System", (560, 128, 600, 160), R::MenuItem),
                edit,
                run("Colors", (594, 500, 900, 540), R::List),
            ],
        )));
        press(&mut engine, &speech, Key::NumpadMinus, insert());
        press(&mut engine, &speech, Key::NumpadMinus, plain());
        assert_eq!(
            press(&mut engine, &speech, Key::UpArrow, insert()),
            "Focus here  Edit "
        );
        assert_eq!(
            press(&mut engine, &speech, Key::DownArrow, plain()),
            "Colors  list view "
        );
    }

    /// Another window in front: its model, recognized words and all, is not this one's.
    #[test]
    fn another_window_forgets_the_model() {
        let (mut engine, speech, _) = window();
        press(&mut engine, &speech, Key::Space, insert());
        press(&mut engine, &speech, Key::Alpha('O'), plain());
        press(&mut engine, &speech, Key::Alpha('W'), plain());
        engine.handle_accessibility_event(&AccessibilityEvent::Recognized(recognized()));
        assert!(engine.screen_model.is_some());
        let other = FocusEvent::new(Role::Button)
            .with_name("OK")
            .with_window_title("Another");
        engine.handle_accessibility_event(&AccessibilityEvent::Focus(other));
        assert!(engine.screen_model.is_none());
    }
}
