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
    CapsLock,
    ScrollLock,

    Alpha(char),
    Digit(u8),
    F(u8),
    Semicolon,
    Apostrophe,
    /// The keys at the US comma and period places, whatever the layout prints on them: JAWS
    /// takes keys by their place (the laptop layout's CapsLock+Comma, Say Character).
    Comma,
    Period,
    /// The keys at the US minus, equals, slash and backslash places, by their place as JAWS takes
    /// them (Default.JKM: Dash, Equals, Slash, Backslash): quick keys for separators, flow to,
    /// clickable elements and mail links.
    Minus,
    Equals,
    Slash,
    Backslash,
    /// The keys at the US left and right bracket places, by their place (Default.JKM:
    /// LeftBracket, RightBracket): the laptop layout's Route JAWS Cursor to PC Cursor is
    /// CapsLock+Left Bracket.
    LeftBracket,
    RightBracket,
    /// The numeric keypad's 5 with Num Lock off (VK_CLEAR): JAWS's desktop Say Character.
    /// The keypad's other keys arrive as the arrows and Home, End, Page Up and Page Down.
    Numpad5,
    /// The keypad's operator keys, whatever the Num Lock state: JAWS's PC cursor and leaving
    /// forms mode (Plus), the JAWS cursor (Minus), and its mouse buttons (Slash, Star).
    NumpadPlus,
    NumpadMinus,
    NumpadSlash,
    NumpadStar,

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
    /// The JAWS key is held. Platform hooks report Insert here; the engine resolves which
    /// held keys act as the JAWS key ([`crate::settings::KeyboardSettings::resolve`]).
    pub insert: bool,
    /// CapsLock is held (a JAWS key in the laptop layout).
    pub caps_lock: bool,
    /// ScrollLock is held.
    pub scroll_lock: bool,
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
    /// Whether Caps Lock was toggled on when the key was pressed, when the platform knows.
    pub caps_lock_on: Option<bool>,
    /// The platform's own code for the key, to press it again exactly as it came (Windows: the
    /// virtual key in its low byte, the scan code from bit 8, which a VK_PACKET's character
    /// is, the extended flag at bit 31; X11: the keycode);
    /// 0 when unknown. A key held for one of Okur's dialogs is pressed again by it, in the
    /// layout the application has, whatever the key's place.
    pub native: u32,
}

impl KeyEvent {
    /// The chord as the corpus's scenarios write it ("Down", "Shift+H", "Ctrl+Insert+F"), the
    /// modifiers in that order before the key: what a runner presses for it (the Linux lab
    /// plays a JAWS script's step by the key Okur binds to the command).
    pub fn chord(&self) -> String {
        let m = &self.modifiers;
        let held = [
            (m.ctrl, "Ctrl"),
            (m.alt, "Alt"),
            (m.shift, "Shift"),
            (m.insert, "Insert"),
            (m.caps_lock, "CapsLock"),
            (m.scroll_lock, "ScrollLock"),
            (m.win, "Win"),
        ];
        let key = match self.key {
            Key::UpArrow => "Up".to_owned(),
            Key::DownArrow => "Down".to_owned(),
            Key::LeftArrow => "Left".to_owned(),
            Key::RightArrow => "Right".to_owned(),
            Key::Alpha(c) => c.to_ascii_uppercase().to_string(),
            Key::Digit(d) => d.to_string(),
            Key::F(n) => format!("F{n}"),
            Key::Other(code) => format!("0x{code:x}"),
            other => format!("{other:?}"),
        };
        held.iter()
            .filter(|(on, _)| *on)
            .map(|(_, name)| *name)
            .chain(std::iter::once(key.as_str()))
            .collect::<Vec<_>>()
            .join("+")
    }

    pub fn new(key: Key, modifiers: Modifiers, is_down: bool) -> Self {
        Self {
            key,
            modifiers,
            is_down,
            is_repeat: false,
            text: None,
            time_ms: 0,
            caps_lock_on: None,
            native: 0,
        }
    }

    /// The keys pressed for this event: its Ctrl, Alt, Shift and Windows keys, then the key.
    pub fn chord_keys(&self) -> Vec<Key> {
        let m = &self.modifiers;
        [
            (m.ctrl, Key::Control),
            (m.alt, Key::Alt),
            (m.shift, Key::Shift),
            (m.win, Key::Windows),
        ]
        .into_iter()
        .filter_map(|(held, key)| held.then_some(key))
        .chain(std::iter::once(self.key))
        .collect()
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
