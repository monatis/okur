//! Okur's settings in layers, as JAWS keeps its own (`docs/references/jaws-reading-settings-research.md`
//! Item 9): Okur's defaults; the user's settings for all applications; one application's; one
//! web site's; and changes that last only until Okur exits (Insert+Shift+2). A setting is looked
//! up from the most specific layer down: the session, the site, the application, all
//! applications, then the default. Each layer keeps only what it changes, as a JAWS JCF holds only
//! the options that differ from the defaults.
//!
//! The layers are kept as setting values by name (the shape [`Settings`] serializes to), so a
//! layer can hold any one setting; where and how they are stored is the platform's concern
//! (`okur-platform`'s settings files).

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::settings::Settings;

/// Setting values by name, nested as [`Settings`] is: `{"voice": {"punctuation": "all"}}`.
pub type Values = Map<String, Value>;

/// Where a change is kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// For all applications (JAWS: Settings Center after Ctrl+Shift+D, `Default.jcf`).
    Global,
    /// For one application, by [`application_id`] (JAWS: Settings Center or Quick Settings
    /// on the application in front).
    Application(String),
    /// For one web site, by [`site_of`] (JAWS: Personalize Web Settings).
    Site(String),
    /// Until Okur exits (JAWS's session toggles: Set Punctuation Level).
    Session,
}

/// The layers, each with only the values it changes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SettingsLayers {
    pub global: Values,
    pub applications: BTreeMap<String, Values>,
    pub sites: BTreeMap<String, Values>,
    /// Until the focus moves to another application (Quick Settings' "restore when the focus
    /// changes"), above the site, under the session.
    pub until_focus_moves: Values,
    pub session: Values,
}

impl SettingsLayers {
    /// Layers whose only values are `settings`' differences from Okur's defaults, for all
    /// applications.
    pub fn with_global(settings: &Settings) -> Self {
        Self {
            global: changes(settings, &Settings::default()),
            ..Self::default()
        }
    }

    /// The settings in force for `application` and `site`, from the most specific layer down.
    /// An application whose settings are disabled (Manage Application Settings) has those for
    /// all applications.
    pub fn resolve(&self, application: Option<&str>, site: Option<&str>) -> Settings {
        let mut merged = to_values(&Settings::default());
        merge(&mut merged, &self.global);
        let disabled = |application: &str| {
            merged
                .get("disabled_applications")
                .and_then(Value::as_array)
                .is_some_and(|names| names.iter().any(|n| n.as_str() == Some(application)))
        };
        if let Some(values) = application
            .filter(|a| !disabled(a))
            .and_then(|a| self.applications.get(a))
        {
            merge(&mut merged, values);
        }
        if let Some(values) = site.and_then(|s| self.sites.get(s)) {
            merge(&mut merged, values);
        }
        merge(&mut merged, &self.until_focus_moves);
        merge(&mut merged, &self.session);
        serde_json::from_value(Value::Object(merged)).unwrap_or_else(|e| {
            tracing::warn!(error = %e, "settings layers not read; Okur's defaults apply");
            Settings::default()
        })
    }

    /// Keeps in `scope` what changed from `before` to `after` (a settings dialog's changes),
    /// and nothing else of the settings in force: a session's level is not saved with them. The
    /// layer then holds only what differs from the layers below it. `application`: the
    /// application a site's pages are read in.
    pub fn change(
        &mut self,
        scope: &Scope,
        before: &Settings,
        after: &Settings,
        application: Option<&str>,
    ) {
        let mut delta = changes(after, before);
        // A setting kept for all applications only (the Okur key) is not kept elsewhere.
        for id in crate::settings::registry::refused(&delta, scope) {
            tracing::warn!(
                setting = id,
                ?scope,
                "not kept there: for all applications only"
            );
            remove(&mut delta, id);
        }
        if delta.is_empty() {
            return;
        }
        let below = to_values(&self.below(scope, application));
        let mut layer = self.layer(scope);
        merge(&mut layer, &delta);
        let mut resolved = below.clone();
        merge(&mut resolved, &layer);
        let values = difference(&resolved, &below);
        match scope {
            Scope::Global => self.global = values,
            Scope::Application(id) => set_or_remove(&mut self.applications, id, values),
            Scope::Site(host) => set_or_remove(&mut self.sites, host, values),
            Scope::Session => self.session = values,
        }
    }

    /// A layer's values (empty when it has none).
    fn layer(&self, scope: &Scope) -> Values {
        match scope {
            Scope::Global => self.global.clone(),
            Scope::Application(id) => self.applications.get(id).cloned().unwrap_or_default(),
            Scope::Site(host) => self.sites.get(host).cloned().unwrap_or_default(),
            Scope::Session => self.session.clone(),
        }
    }

    /// The settings the layers under `scope` give.
    fn below(&self, scope: &Scope, application: Option<&str>) -> Settings {
        match scope {
            Scope::Global => Settings::default(),
            Scope::Application(_) => Self {
                global: self.global.clone(),
                ..Self::default()
            }
            .resolve(None, None),
            Scope::Site(_) | Scope::Session => Self {
                global: self.global.clone(),
                applications: self.applications.clone(),
                ..Self::default()
            }
            .resolve(application, None),
        }
    }

    /// Forgets `application`'s own settings (Manage Application Settings' Remove Selected).
    pub fn remove_application(&mut self, application: &str) {
        self.applications.remove(application);
    }

    /// Forgets every saved setting (Restore to Factory Settings): Okur's defaults again,
    /// everywhere.
    pub fn restore_factory(&mut self) {
        self.global.clear();
        self.applications.clear();
        self.sites.clear();
        self.until_focus_moves.clear();
        self.session.clear();
    }

    /// Forgets the voice profile saved as `name` (Voice Adjustment's Delete Profile): a map's
    /// entry, which a change of the settings cannot remove.
    pub fn remove_voice_profile(&mut self, name: &str) {
        if let Some(Value::Object(profiles)) = self.global.get_mut("voice_profiles") {
            profiles.remove(name);
            if profiles.is_empty() {
                self.global.remove("voice_profiles");
            }
        }
    }

    /// Sets one value in the session layer, by its path in [`Settings`]
    /// (`["voice", "punctuation"]`).
    pub fn set_for_session(&mut self, path: &[&str], value: Value) {
        set_in(&mut self.session, path, value);
    }

    /// Sets one value until the focus moves to another application, by its path.
    pub fn set_until_focus_moves(&mut self, path: &[&str], value: Value) {
        set_in(&mut self.until_focus_moves, path, value);
    }
}

/// Sets the value at `path` in `values`, making the tables on the way.
fn set_in(values: &mut Values, path: &[&str], value: Value) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };
    let mut table = values;
    for name in parents {
        let entry = table
            .entry((*name).to_owned())
            .or_insert_with(|| Value::Object(Map::new()));
        if !entry.is_object() {
            *entry = Value::Object(Map::new());
        }
        let Value::Object(next) = entry else {
            return;
        };
        table = next;
    }
    table.insert((*last).to_owned(), value);
}

/// Removes the value at `path` ("keyboard.okur_key"), and the tables it leaves empty.
fn remove(values: &mut Values, path: &str) {
    match path.split_once('.') {
        None => {
            values.remove(path);
        }
        Some((name, rest)) => {
            if let Some(Value::Object(inner)) = values.get_mut(name) {
                remove(inner, rest);
                if inner.is_empty() {
                    values.remove(name);
                }
            }
        }
    }
}

fn set_or_remove(layers: &mut BTreeMap<String, Values>, key: &str, values: Values) {
    if values.is_empty() {
        layers.remove(key);
    } else {
        layers.insert(key.to_owned(), values);
    }
}

fn to_values(settings: &Settings) -> Values {
    match serde_json::to_value(settings) {
        Ok(Value::Object(values)) => values,
        _ => Values::new(),
    }
}

/// What `settings` changes from `base`, nested: only the values that differ.
pub fn changes(settings: &Settings, base: &Settings) -> Values {
    difference(&to_values(settings), &to_values(base))
}

fn difference(values: &Values, base: &Values) -> Values {
    let mut out = Values::new();
    for (name, value) in values {
        match (value, base.get(name)) {
            (Value::Object(inner), Some(Value::Object(below))) => {
                let inner = difference(inner, below);
                if !inner.is_empty() {
                    out.insert(name.clone(), Value::Object(inner));
                }
            }
            (value, Some(below)) if value == below => {}
            (value, _) => {
                out.insert(name.clone(), value.clone());
            }
        }
    }
    out
}

/// `layer`'s values over `values`: tables merge, any other value replaces.
fn merge(values: &mut Values, layer: &Values) {
    for (name, value) in layer {
        match (values.get_mut(name), value) {
            (Some(Value::Object(inner)), Value::Object(over)) => merge(inner, over),
            _ => {
                values.insert(name.clone(), value.clone());
            }
        }
    }
}

/// An application's settings name, from its executable as the platform reports it: the file
/// name without its extension, in small letters ("chrome.exe" is `chrome`), as JAWS names an
/// application's JCF after its executable. On macOS the platform reports the bundle identifier,
/// which is used whole.
pub fn application_id(app: &str) -> String {
    let file = app.rsplit(['\\', '/']).next().unwrap_or(app);
    let base = file
        .strip_suffix(".exe")
        .or_else(|| file.strip_suffix(".EXE"))
        .unwrap_or(file);
    base.to_lowercase()
}

/// An application's name as the settings dialog shows it: its id with a capital first letter,
/// as JAWS's Settings Center names it ("Notepad", "Chrome").
pub fn application_name(id: &str) -> String {
    let mut chars = id.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// A page's site: the host of its address, in small letters. JAWS keeps Personalized Web
/// Settings per domain or subdomain (shopping.yahoo.com's do not apply to mail.yahoo.com), so
/// the whole host is the key. A page without a host (`file:///`) has no site.
pub fn site_of(url: &str) -> Option<String> {
    let (_, rest) = url.split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = authority.rsplit('@').next().unwrap_or_default();
    let host = if host.starts_with('[') {
        host.split_inclusive(']').next().unwrap_or_default()
    } else {
        host.split(':').next().unwrap_or_default()
    };
    (!host.is_empty()).then(|| host.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Punctuation, TypingEcho};
    use serde_json::json;

    fn values(value: Value) -> Values {
        match value {
            Value::Object(values) => values,
            _ => Values::new(),
        }
    }

    /// A setting is looked up from the most specific layer down: the session, the site, the
    /// application, all applications, then Okur's default.
    #[test]
    fn the_most_specific_layer_wins() {
        let mut layers = SettingsLayers {
            global: values(json!({"typing_echo": "none", "voice": {"punctuation": "some"}})),
            ..SettingsLayers::default()
        };
        layers.applications.insert(
            "notepad".into(),
            values(json!({"voice": {"punctuation": "all"}})),
        );
        layers.sites.insert(
            "example.org".into(),
            values(json!({"forms_mode": "manual"})),
        );
        let everywhere = layers.resolve(None, None);
        assert_eq!(everywhere.voice.punctuation, Punctuation::Some);
        assert_eq!(everywhere.typing_echo, TypingEcho::None);
        assert_eq!(everywhere.forms_mode, crate::settings::FormsMode::Auto);
        let notepad = layers.resolve(Some("notepad"), None);
        assert_eq!(notepad.voice.punctuation, Punctuation::All);
        assert_eq!(notepad.typing_echo, TypingEcho::None);
        let site = layers.resolve(Some("chrome"), Some("example.org"));
        assert_eq!(site.forms_mode, crate::settings::FormsMode::Manual);
        assert_eq!(site.voice.punctuation, Punctuation::Some);
        layers.set_for_session(&["voice", "punctuation"], json!("none"));
        assert_eq!(
            layers.resolve(Some("notepad"), None).voice.punctuation,
            Punctuation::None
        );
    }

    /// A layer keeps only what changed, and only what differs from the layers below it; a
    /// session's value is not saved with a dialog's change.
    #[test]
    fn a_layer_keeps_only_its_changes() {
        let mut layers = SettingsLayers::default();
        layers.set_for_session(&["voice", "punctuation"], json!("none"));
        let before = layers.resolve(None, None);
        let mut after = before.clone();
        after.typing_echo = TypingEcho::None;
        layers.change(&Scope::Global, &before, &after, None);
        assert_eq!(layers.global, values(json!({"typing_echo": "none"})));
        // Back to the default: nothing kept.
        let before = layers.resolve(None, None);
        let mut after = before.clone();
        after.typing_echo = TypingEcho::default();
        layers.change(&Scope::Global, &before, &after, None);
        assert!(layers.global.is_empty());
        // Notepad's own level; the same as all applications' is not kept.
        let before = layers.resolve(Some("notepad"), None);
        let mut after = before.clone();
        after.voice.punctuation = Punctuation::All;
        layers.change(&Scope::Application("notepad".into()), &before, &after, None);
        assert_eq!(
            layers.applications.get("notepad"),
            Some(&values(json!({"voice": {"punctuation": "all"}})))
        );
        let before = layers.resolve(Some("notepad"), None);
        let mut after = before.clone();
        after.voice.punctuation = Punctuation::Most;
        layers.change(&Scope::Application("notepad".into()), &before, &after, None);
        assert!(layers.applications.is_empty());
        assert_eq!(
            SettingsLayers::with_global(&Settings::default()),
            SettingsLayers::default()
        );
    }

    /// The Okur key is kept for all applications only: an application's change of it is not
    /// kept, while the rest of the change is.
    #[test]
    fn an_application_does_not_keep_the_okur_key() {
        let mut layers = SettingsLayers::default();
        let before = layers.resolve(Some("notepad"), None);
        let mut after = before.clone();
        after.keyboard.okur_key = crate::settings::OkurKey::ScrollLock;
        after.typing_echo = TypingEcho::None;
        layers.change(&Scope::Application("notepad".into()), &before, &after, None);
        assert_eq!(
            layers.applications.get("notepad"),
            Some(&values(json!({"typing_echo": "none"})))
        );
        layers.change(&Scope::Global, &before, &after, None);
        assert_eq!(
            layers.global,
            values(json!({"keyboard": {"okur_key": "scroll_lock"}, "typing_echo": "none"}))
        );
    }

    #[test]
    fn applications_and_sites_are_named_as_jaws_names_them() {
        assert_eq!(application_id("chrome.exe"), "chrome");
        assert_eq!(application_id("C:\\Windows\\notepad.EXE"), "notepad");
        assert_eq!(application_id("com.apple.Safari"), "com.apple.safari");
        assert_eq!(
            site_of("https://Mail.Example.org:8443/inbox?x=1#top").as_deref(),
            Some("mail.example.org")
        );
        assert_eq!(
            site_of("http://user@shop.example.com/").as_deref(),
            Some("shop.example.com")
        );
        assert_eq!(site_of("file:///C:/fixtures/page.html"), None);
        assert_eq!(site_of("about:blank"), None);
    }

    /// Delete Profile: a saved profile goes from the settings for all applications, the others
    /// stay; with the last one, the table goes too.
    #[test]
    fn a_saved_voice_profile_is_removed() {
        let mut saved = Settings::default();
        for (name, rate) in [("Fast", 6), ("Slow", -4)] {
            saved.voice_profiles.insert(
                name.to_owned(),
                crate::settings::VoiceSettings {
                    rate,
                    ..crate::settings::VoiceSettings::default()
                },
            );
        }
        let mut layers = SettingsLayers::with_global(&saved);
        layers.remove_voice_profile("Fast");
        let profiles = layers.resolve(None, None).voice_profiles;
        assert_eq!(profiles.keys().collect::<Vec<_>>(), ["Slow"]);
        layers.remove_voice_profile("Slow");
        assert!(!layers.global.contains_key("voice_profiles"));
    }

    /// Manage Application Settings: an application whose settings are disabled has those for
    /// all applications, its own kept; Restore to Factory Settings forgets everything.
    #[test]
    fn disabled_application_settings_are_kept_but_not_used() {
        use crate::settings::Punctuation;
        let mut layers = SettingsLayers::default();
        let mut notepad = Settings::default();
        notepad.voice.punctuation = Punctuation::All;
        layers.change(
            &Scope::Application("notepad".into()),
            &Settings::default(),
            &notepad,
            None,
        );
        assert_eq!(
            layers.resolve(Some("notepad"), None).voice.punctuation,
            Punctuation::All
        );
        let disabled = Settings {
            disabled_applications: vec!["notepad".into()],
            ..Settings::default()
        };
        layers.change(&Scope::Global, &Settings::default(), &disabled, None);
        assert_eq!(
            layers.resolve(Some("notepad"), None).voice.punctuation,
            Punctuation::Most
        );
        assert!(layers.applications.contains_key("notepad"));
        layers.restore_factory();
        assert_eq!(layers.resolve(Some("notepad"), None), Settings::default());
        assert!(layers.applications.is_empty());
    }
}
