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
    /// The numeric keypad's 5 with Num Lock off (VK_CLEAR): JAWS's desktop Say Character.
    /// The keypad's other keys arrive as the arrows and Home, End, Page Up and Page Down.
    Numpad5,

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
            caps_lock_on: None,
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
