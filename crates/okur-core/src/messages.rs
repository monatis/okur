//! Okur's own words: the message catalog.
//!
//! Everything Okur says or shows in its own words comes from here: hints ("To activate press
//! Enter."), messages ("There are no links on this page."), role and state words ("Button",
//! "checked"), dialog labels. What an application gives (names, values, page text) is spoken
//! as given and never goes through the catalog.
//!
//! The catalog is [Project Fluent](https://projectfluent.org): `locales/en-US/*.ftl`,
//! compiled into the binary. English is JAWS's English, exactly, quirks included: a message
//! JAWS ends without a period ends without one here.
//!
//! # Adding a message
//!
//! 1. Write it into the right file under `locales/en-US/`, with an id that names the
//!    situation, not the words: `quicknav-none-links = There are no links on this page.`
//!    Values in it are variables: `position = { $index } of { $total }`.
//! 2. When its meaning is not obvious from the id and the words, put a translator comment
//!    (`# ...`) right above it.
//! 3. Call its constructor: `msg::quicknav_none_links()`, `msg::position(3, 7)`. `build.rs`
//!    writes one constructor per message, with the message's variables as parameters, so a
//!    misspelled id or a missing variable does not compile.
//!
//! Rules:
//! - A message is whole. Never build "There are no " + element + " on this page.": write one
//!   message per element kind.
//! - A message never starts or ends with spaces (Fluent drops them) and never holds JAWS's
//!   role and state marks (`\u{1d}`, `\u{1e}`): the code that renders speech adds both.
//! - A message nothing uses fails a test, and so does a word written into Rust code instead
//!   of the catalog (`tests/catalog.rs`).
//! - Where JAWS names itself, Okur names itself, JAWS's words kept around the name: "Okur
//!   Cursor" for "JAWS Cursor" (AGENTS.md §1.5). A message naming JAWS, Freedom Scientific or
//!   Vispero fails a test; parity reads JAWS's name as Okur's.
//!
//! Fluent's bidirectional isolation marks are off: they would reach the synthesizer.

use std::borrow::Cow;
use std::cell::Cell;
use std::fmt;
use std::sync::OnceLock;

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};

include!(concat!(env!("OUT_DIR"), "/messages.rs"));

/// The value of a message's variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arg {
    /// Words: an application's text, said as given, or Okur's own already rendered.
    Text(String),
    /// A count or a position, which a language's plural rules can choose words by.
    Number(i64),
}

impl From<&str> for Arg {
    fn from(text: &str) -> Self {
        Arg::Text(text.to_owned())
    }
}

impl From<String> for Arg {
    fn from(text: String) -> Self {
        Arg::Text(text)
    }
}

impl From<&String> for Arg {
    fn from(text: &String) -> Self {
        Arg::Text(text.clone())
    }
}

impl From<Cow<'_, str>> for Arg {
    fn from(text: Cow<'_, str>) -> Self {
        Arg::Text(text.into_owned())
    }
}

impl From<StaticMessage> for Arg {
    fn from(message: StaticMessage) -> Self {
        Arg::Text(message.as_str().to_owned())
    }
}

impl From<Message> for Arg {
    fn from(message: Message) -> Self {
        Arg::Text(message.render())
    }
}

macro_rules! number_args {
    ($($t:ty),*) => {$(
        impl From<$t> for Arg {
            fn from(n: $t) -> Self {
                Arg::Number(i64::try_from(n).unwrap_or(i64::MAX))
            }
        }
    )*};
}
number_args!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);

/// A message without variables: its words do not change while Okur runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StaticMessage(MessageId);

impl StaticMessage {
    const fn new(id: MessageId) -> Self {
        Self(id)
    }

    pub fn id(self) -> MessageId {
        self.0
    }

    /// The message's words in Okur's language.
    pub fn as_str(self) -> &'static str {
        catalog().plain(self.0)
    }
}

impl fmt::Display for StaticMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<StaticMessage> for String {
    fn from(message: StaticMessage) -> Self {
        message.as_str().to_owned()
    }
}

impl AsRef<str> for StaticMessage {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// A message and the values of its variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    id: MessageId,
    args: Vec<(&'static str, Arg)>,
}

impl Message {
    fn new(id: MessageId, args: Vec<(&'static str, Arg)>) -> Self {
        Self { id, args }
    }

    pub fn id(&self) -> MessageId {
        self.id
    }

    /// The value given for `variable`.
    pub fn arg(&self, variable: &str) -> Option<&Arg> {
        self.args
            .iter()
            .find(|(name, _)| *name == variable)
            .map(|(_, arg)| arg)
    }

    /// The message's words in Okur's language.
    pub fn render(&self) -> String {
        if self.args.is_empty() {
            return catalog().plain(self.id).to_owned();
        }
        let mut args = FluentArgs::with_capacity(self.args.len());
        for (name, arg) in &self.args {
            match arg {
                Arg::Text(text) => args.set(*name, FluentValue::from(text.as_str())),
                Arg::Number(n) => args.set(*name, FluentValue::from(*n)),
            }
        }
        catalog().format(self.id, Some(&args))
    }
}

impl From<StaticMessage> for Message {
    fn from(message: StaticMessage) -> Self {
        Self::new(message.0, Vec::new())
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

impl From<Message> for String {
    fn from(message: Message) -> Self {
        message.render()
    }
}

/// The pseudo-locale's marks around every message.
const PSEUDO_OPEN: char = '\u{27e6}';
const PSEUDO_CLOSE: char = '\u{27e7}';

thread_local! {
    static PSEUDO: Cell<bool> = const { Cell::new(false) };
}

/// Runs `f` with the pseudo-locale on this thread: every message comes out between `⟦` and
/// `⟧`, so a test can tell Okur's words from an application's in what the engine says. A
/// word outside the marks that the application did not give was written into code, not the
/// catalog.
pub fn with_pseudo_locale<R>(f: impl FnOnce() -> R) -> R {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            PSEUDO.with(|p| p.set(self.0));
        }
    }
    let _restore = Restore(PSEUDO.with(|p| p.replace(true)));
    f()
}

fn pseudo() -> bool {
    PSEUDO.with(Cell::get)
}

fn wrap(text: &str) -> String {
    format!("{PSEUDO_OPEN}{text}{PSEUDO_CLOSE}")
}

struct Catalog {
    bundle: FluentBundle<FluentResource>,
    /// Each message without variables, rendered once, by `MessageId as usize`.
    plain: Vec<String>,
    /// The same in the pseudo-locale.
    plain_pseudo: Vec<String>,
}

fn catalog() -> &'static Catalog {
    static ENGLISH: OnceLock<Catalog> = OnceLock::new();
    ENGLISH.get_or_init(Catalog::english)
}

impl Catalog {
    fn english() -> Self {
        let language = "en-US"
            .parse::<unic_langid::LanguageIdentifier>()
            .unwrap_or_default();
        let mut bundle = FluentBundle::new_concurrent(vec![language]);
        bundle.set_use_isolating(false);
        for (file, source) in ENGLISH_SOURCES {
            // build.rs parsed these same files and stops the build on any error.
            let resource = match FluentResource::try_new((*source).to_owned()) {
                Ok(resource) => resource,
                Err((resource, errors)) => {
                    tracing::error!(file, ?errors, "message catalog does not parse");
                    resource
                }
            };
            if let Err(errors) = bundle.add_resource(resource) {
                tracing::error!(file, ?errors, "message catalog has errors");
            }
        }
        let mut catalog = Self {
            bundle,
            plain: Vec::new(),
            plain_pseudo: Vec::new(),
        };
        let plain: Vec<String> = MessageId::ALL
            .iter()
            .map(|id| {
                if id.variables().is_empty() {
                    catalog.english_text(*id, None)
                } else {
                    String::new()
                }
            })
            .collect();
        catalog.plain_pseudo = plain.iter().map(|text| wrap(text)).collect();
        catalog.plain = plain;
        catalog
    }

    fn plain(&self, id: MessageId) -> &str {
        let plain = if pseudo() {
            &self.plain_pseudo
        } else {
            &self.plain
        };
        plain.get(id as usize).map_or(id.id(), String::as_str)
    }

    /// The message in the current locale: English, or the pseudo-locale on this thread.
    fn format(&self, id: MessageId, args: Option<&FluentArgs>) -> String {
        let text = self.english_text(id, args);
        if pseudo() {
            wrap(&text)
        } else {
            text
        }
    }

    /// The message in English, whatever the thread's locale: what the cache keeps.
    fn english_text(&self, id: MessageId, args: Option<&FluentArgs>) -> String {
        let Some(pattern) = self.bundle.get_message(id.id()).and_then(|m| m.value()) else {
            tracing::error!(id = id.id(), "message missing from the catalog");
            return id.id().to_owned();
        };
        let mut errors = Vec::new();
        let text = self.bundle.format_pattern(pattern, args, &mut errors);
        if !errors.is_empty() {
            tracing::error!(id = id.id(), ?errors, "message did not render cleanly");
        }
        text.into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every message renders without an error, given a value for each variable.
    #[test]
    fn every_message_renders() {
        let catalog = catalog();
        for id in MessageId::ALL {
            let mut args = FluentArgs::new();
            for variable in id.variables() {
                args.set(*variable, FluentValue::from(2));
            }
            let pattern = catalog.bundle.get_message(id.id()).and_then(|m| m.value());
            assert!(pattern.is_some(), "{} has no value", id.id());
            if let Some(pattern) = pattern {
                let mut errors = Vec::new();
                let text = catalog
                    .bundle
                    .format_pattern(pattern, Some(&args), &mut errors);
                assert!(errors.is_empty(), "{}: {errors:?}", id.id());
                assert!(!text.trim().is_empty(), "{} is empty", id.id());
                assert!(
                    !text.contains(['\u{1d}', '\u{1e}', '\u{2068}', '\u{2069}']),
                    "{} holds a mark: {text:?}",
                    id.id()
                );
            }
        }
    }

    /// A count picks the English plural: "1 item", "2 items" [obs, 274/s1, 303/s7].
    #[test]
    fn counts_take_their_plural() {
        assert_eq!(msg::tree_items(1usize).render(), "1 item");
        assert_eq!(msg::tree_items(2usize).render(), "2 items");
    }

    /// A catalog made under the pseudo-locale still keeps English for everyone else.
    #[test]
    fn the_english_cache_is_english_whatever_thread_builds_it() {
        let catalog = with_pseudo_locale(Catalog::english);
        assert!(catalog
            .plain
            .iter()
            .all(|text| !text.contains([PSEUDO_OPEN, PSEUDO_CLOSE])));
        assert!(catalog
            .plain_pseudo
            .iter()
            .filter(|text| !text.is_empty())
            .all(|text| text.starts_with(PSEUDO_OPEN) && !text.starts_with("\u{27e6}\u{27e6}")));
    }

    #[test]
    fn pseudo_locale_marks_every_message_on_this_thread_only() {
        let id = MessageId::ALL.iter().find(|id| id.variables().is_empty());
        if let Some(id) = id {
            let message = StaticMessage::new(*id);
            let english = message.as_str();
            let marked = with_pseudo_locale(|| message.as_str());
            assert_eq!(marked, wrap(english));
            assert_eq!(message.as_str(), english);
        }
    }
}
