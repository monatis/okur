/// Normalized key representations for screen reader navigation and control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    UpArrow,
    DownArrow,
    LeftArrow,
    RightArrow,

    Home,
    End,
    PageUp,
    PageDown,

    Tab,
    Enter,
    Escape,
    Space,
    Backspace,
    Delete,
    Insert,

    Alpha(char),
    Digit(u8),
    F(u8),
    Semicolon,
    Apostrophe,

    Control,
    Alt,
    Shift,
    Windows,

    Other(u32),
}

/// Modifier keys state held during a keystroke event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
    pub insert: bool,
}

impl Modifiers {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        !self.ctrl && !self.alt && !self.shift && !self.win && !self.insert
    }
}

/// Normalized keyboard event passed to the screen reader engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    pub key: Key,
    pub modifiers: Modifiers,
    pub is_down: bool,
    pub is_repeat: bool,
    /// The character the key types in the active keyboard layout, when it types one
    /// (filled in by the platform hook; used for typing echo).
    pub text: Option<char>,
    /// When the key was pressed, in milliseconds on the OS input clock (0 when unknown).
    /// Tells a command pressed twice quickly from two separate presses.
    pub time_ms: u64,
}

impl KeyEvent {
    pub fn new(key: Key, modifiers: Modifiers, is_down: bool) -> Self {
        Self {
            key,
            modifiers,
            is_down,
            is_repeat: false,
            text: None,
            time_ms: 0,
        }
    }

    /// Translates a key event into a canonical screen reader navigation command string
    /// matching Freedom Scientific JAWS and scenario conventions.
    pub fn to_command_string(&self) -> Option<String> {
        if !self.is_down {
            return None;
        }

        let m = &self.modifiers;

        match self.key {
            // Text selection commands (Shift + Arrows)
            Key::RightArrow if m.shift && !m.ctrl && !m.alt && !m.insert => {
                Some("+{RIGHT}".to_string())
            }
            Key::LeftArrow if m.shift && !m.ctrl && !m.alt && !m.insert => {
                Some("+{LEFT}".to_string())
            }
            Key::DownArrow if m.shift && !m.ctrl && !m.alt && !m.insert => {
                Some("+{DOWN}".to_string())
            }
            Key::UpArrow if m.shift && !m.ctrl && !m.alt && !m.insert => Some("+{UP}".to_string()),

            // Say All (Insert+DownArrow) and Say Line (Insert+UpArrow)
            Key::DownArrow if m.insert && !m.ctrl && !m.alt && !m.shift => {
                Some("SayAll".to_string())
            }
            Key::UpArrow if m.insert && !m.ctrl && !m.alt && !m.shift => {
                Some("SayLine".to_string())
            }

            // Say selected text (JAWSKey+Shift+DownArrow or Shift+Alt+DownArrow)
            Key::DownArrow if (m.insert || m.alt) && m.shift && !m.ctrl => {
                Some("+!{DOWN}".to_string())
            }

            // Directional & reading navigation
            Key::DownArrow if m.ctrl && m.alt && m.shift => Some("^%+DOWN".to_string()),
            Key::UpArrow if m.ctrl && m.alt && m.shift => Some("^%+UP".to_string()),
            Key::RightArrow if m.ctrl && m.alt && m.shift => Some("^%+RIGHT".to_string()),
            Key::LeftArrow if m.ctrl && m.alt && m.shift => Some("^%+LEFT".to_string()),

            Key::DownArrow if m.ctrl && m.alt => Some("^%{DOWN}".to_string()),
            Key::UpArrow if m.ctrl && m.alt => Some("^%{UP}".to_string()),
            Key::RightArrow if m.ctrl && m.alt => Some("^%{RIGHT}".to_string()),
            Key::LeftArrow if m.ctrl && m.alt => Some("^%{LEFT}".to_string()),

            Key::DownArrow if m.alt && !m.ctrl && !m.shift => Some("!{DOWN}".to_string()),
            Key::UpArrow if m.alt && !m.ctrl && !m.shift => Some("!{UP}".to_string()),

            Key::DownArrow if m.ctrl => Some("^{DOWN}".to_string()),
            Key::UpArrow if m.ctrl => Some("^{UP}".to_string()),
            Key::RightArrow if m.ctrl => Some("^{RIGHT}".to_string()),
            Key::LeftArrow if m.ctrl => Some("^{LEFT}".to_string()),

            Key::DownArrow if m.is_empty() => Some("{DOWN}".to_string()),
            Key::UpArrow if m.is_empty() => Some("{UP}".to_string()),
            Key::RightArrow if m.is_empty() => Some("{RIGHT}".to_string()),
            Key::LeftArrow if m.is_empty() => Some("{LEFT}".to_string()),

            // Document & Table boundary navigation
            Key::Home if m.ctrl && m.alt => Some("^%{HOME}".to_string()),
            Key::End if m.ctrl && m.alt => Some("^%{END}".to_string()),
            Key::Home if m.ctrl => Some("^{HOME}".to_string()),
            Key::End if m.ctrl => Some("^{END}".to_string()),

            // Interactive controls and tab stops
            Key::Tab if m.shift => Some("+{TAB}".to_string()),
            Key::Tab if m.is_empty() => Some("{TAB}".to_string()),
            Key::Enter if m.is_empty() => Some("{ENTER}".to_string()),
            Key::Escape if m.is_empty() => Some("{ESC}".to_string()),
            Key::Space if m.is_empty() => Some("{SPACE}".to_string()),

            // Say Window Title (Insert+T)
            Key::Alpha(c)
                if c.eq_ignore_ascii_case(&'T') && m.insert && !m.ctrl && !m.alt && !m.shift =>
            {
                Some("SayWindowTitle".to_string())
            }

            // Say System Time (Insert+F12), with seconds (Ctrl+Insert+F12)
            Key::F(12) if m.insert && !m.alt && !m.shift => Some(if m.ctrl {
                "SaySystemTimeWithSeconds".to_string()
            } else {
                "SaySystemTime".to_string()
            }),

            // Font formatting query (Insert+F)
            Key::Alpha(c)
                if c.eq_ignore_ascii_case(&'F') && m.insert && !m.ctrl && !m.alt && !m.shift =>
            {
                Some("Insert+F".to_string())
            }

            // In-buffer search dialog (Ctrl+F)
            Key::Alpha(c)
                if c.eq_ignore_ascii_case(&'F') && m.ctrl && !m.insert && !m.alt && !m.shift =>
            {
                Some("^f".to_string())
            }

            // Form fields list (Insert+Ctrl+F or JAWSKey+Ctrl+F)
            Key::Alpha(c)
                if c.eq_ignore_ascii_case(&'F') && m.ctrl && m.insert && !m.alt && !m.shift =>
            {
                Some("Insert+Ctrl+F".to_string())
            }

            // Table headers direct query (Ctrl+Alt+Shift+H and Ctrl+Alt+Shift+R)
            Key::Alpha(c) if c.eq_ignore_ascii_case(&'H') && m.ctrl && m.alt && m.shift => {
                Some("^%+H".to_string())
            }
            Key::Alpha(c) if c.eq_ignore_ascii_case(&'R') && m.ctrl && m.alt && m.shift => {
                Some("^%+R".to_string())
            }

            // Function keys (FindNext, FindPrior, Element Lists)
            Key::F(3) if m.shift && !m.ctrl && !m.alt && !m.insert => Some("+{F3}".to_string()),
            Key::F(3) if m.is_empty() => Some("{F3}".to_string()),
            Key::F(3) if m.insert && !m.ctrl && !m.alt && !m.shift => Some("Insert+F3".to_string()),
            Key::F(5) if m.insert && !m.ctrl && !m.alt && !m.shift => Some("Insert+F5".to_string()),
            Key::F(6) if m.insert && !m.ctrl && !m.alt && !m.shift => Some("Insert+F6".to_string()),
            Key::F(7) if m.insert && !m.ctrl && !m.alt && !m.shift => Some("Insert+F7".to_string()),

            // Current table cell query (Ctrl+Alt+5 / Ctrl+Alt+NumPad5)
            Key::Digit(5) if m.ctrl && m.alt => Some("^%5".to_string()),

            // Quick navigation keys
            Key::Alpha(c) => {
                let upper = c.to_ascii_uppercase();
                match upper {
                    'H' | 'T' | 'K' | 'U' | 'F' | 'B' | 'E' | 'X' | 'L' | 'I' | 'R' | 'C' | 'M'
                    | 'G' | 'A' | 'O' => {
                        if m.shift && !m.ctrl && !m.alt {
                            Some(format!("+{}", upper))
                        } else if m.is_empty() {
                            Some(upper.to_string())
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }

            Key::Digit(d) if (1..=6).contains(&d) => {
                if m.shift && !m.ctrl && !m.alt {
                    Some(format!("+{}", d))
                } else if m.is_empty() {
                    Some(d.to_string())
                } else {
                    None
                }
            }
            Key::Semicolon => {
                if m.shift && !m.ctrl && !m.alt {
                    Some("+;".to_string())
                } else if m.is_empty() {
                    Some(";".to_string())
                } else {
                    None
                }
            }
            Key::Apostrophe => {
                if m.shift && !m.ctrl && !m.alt {
                    Some("+'".to_string())
                } else if m.is_empty() {
                    Some("'".to_string())
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Action determining whether the intercepted key event is consumed by the screen reader
/// or passed through to the underlying operating system and application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    /// Consume the key event, suppressing it from reaching other applications.
    Consume,
    /// Pass the key event through to the focused application.
    PassThrough,
}
