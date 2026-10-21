//! Okur's settings files, in TOML, by layer ([`okur_core::settings_layers`]), each holding only
//! what it changes from the layers below it:
//! - `settings.toml`: for all applications;
//! - `applications/<application>.toml`: one application's, named as
//!   [`okur_core::settings_layers::application_id`] names it (`notepad.toml`);
//! - `sites/<host>.toml`: one web site's (`mail.example.org.toml`), which can be copied to
//!   another user, as JAWS's Personalized Web Settings can.
//!
//! The session's layer is never written. Okur writes only the values that changed, so the
//! comments and order a user gives a file stay.

use std::path::{Path, PathBuf};

use okur_core::settings::SettingsStore;
use okur_core::settings_layers::{SettingsLayers, Values};
use serde_json::Value;

const GLOBAL: &str = "settings.toml";
const APPLICATIONS: &str = "applications";
const SITES: &str = "sites";

/// The folder of Okur's settings files.
#[derive(Debug, Clone)]
pub struct SettingsFolder {
    folder: PathBuf,
}

impl SettingsFolder {
    /// The user's settings folder: `%APPDATA%\Okur` on Windows,
    /// `~/Library/Application Support/Okur` on macOS, and `$XDG_CONFIG_HOME/okur` (else
    /// `~/.config/okur`) elsewhere.
    pub fn user() -> Option<Self> {
        let env = |name: &str| std::env::var_os(name).map(PathBuf::from);
        let folder = if cfg!(windows) {
            env("APPDATA")?.join("Okur")
        } else if cfg!(target_os = "macos") {
            env("HOME")?.join("Library/Application Support/Okur")
        } else {
            env("XDG_CONFIG_HOME")
                .or_else(|| env("HOME").map(|home| home.join(".config")))?
                .join("okur")
        };
        Some(Self::at(folder))
    }

    pub fn at(folder: impl Into<PathBuf>) -> Self {
        Self {
            folder: folder.into(),
        }
    }

    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// The saved layers. A missing file is an empty layer; a file that cannot be read is
    /// logged and left out.
    pub fn load(&self) -> SettingsLayers {
        let mut layers = SettingsLayers {
            global: read_values(&self.folder.join(GLOBAL)).unwrap_or_default(),
            ..SettingsLayers::default()
        };
        for (name, values) in read_folder(&self.folder.join(APPLICATIONS)) {
            layers.applications.insert(name, values);
        }
        for (name, values) in read_folder(&self.folder.join(SITES)) {
            layers.sites.insert(name, values);
        }
        layers
    }

    /// Writes `values` into the file at `path`, changing only what differs, through a
    /// temporary file so that a failure never leaves half a file.
    fn write(path: &Path, values: &Values, heading: &str) -> std::io::Result<()> {
        let existing = std::fs::read_to_string(path).ok();
        if existing.is_none() && values.is_empty() {
            return Ok(());
        }
        let mut document = match existing.as_deref() {
            Some(text) => text
                .parse::<toml_edit::DocumentMut>()
                .map_err(std::io::Error::other)?,
            None => {
                let mut document = toml_edit::DocumentMut::new();
                document.decor_mut().set_prefix(heading);
                document
            }
        };
        sync(document.as_table_mut(), values);
        let text = document.to_string();
        if existing.as_deref() == Some(text.as_str()) {
            return Ok(());
        }
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        let temporary = path.with_extension("toml.tmp");
        std::fs::write(&temporary, text)?;
        std::fs::rename(&temporary, path)
    }
}

impl SettingsStore for SettingsFolder {
    fn save(&self, layers: &SettingsLayers) {
        let mut written = vec![Self::write(
            &self.folder.join(GLOBAL),
            &layers.global,
            "# Okur's settings for all applications: only what differs from Okur's defaults.\n\
             # Okur rewrites only the values it changes; comments stay.\n",
        )];
        for (kind, layer, heading) in [
            (
                APPLICATIONS,
                &layers.applications,
                "# Okur's settings for one application: only what differs from settings.toml.\n",
            ),
            (
                SITES,
                &layers.sites,
                "# Okur's settings for one web site: only what differs from its application's.\n",
            ),
        ] {
            let folder = self.folder.join(kind);
            // A layer that was emptied loses its values; the file and its comments stay.
            for (name, _) in read_folder(&folder) {
                if !layer.contains_key(&name) {
                    written.push(Self::write(
                        &folder.join(format!("{name}.toml")),
                        &Values::new(),
                        heading,
                    ));
                }
            }
            for (name, values) in layer {
                written.push(Self::write(
                    &folder.join(format!("{name}.toml")),
                    values,
                    heading,
                ));
            }
        }
        for result in written {
            if let Err(e) = result {
                tracing::warn!(folder = ?self.folder, error = %e, "settings not saved");
            }
        }
    }
}

/// The custom labels' file: a table per site (or page), each element's label in it.
const LABELS: &str = "labels.toml";
/// The saved PlaceMarkers' file: a list per page address.
const PLACEMARKERS: &str = "placemarkers.toml";
/// The saved Flexible Web rules' file: each with its name, where it applies and its
/// customizations.
const FLEXIBLE_WEB: &str = "flexible-web.toml";
const NOTIFICATION_RULES: &str = "notification-rules.toml";

impl SettingsFolder {
    /// The custom labels kept (Insert+Ctrl+Tab). A missing or unreadable file is none, the
    /// second logged.
    pub fn load_labels(&self) -> okur_core::labels::CustomLabels {
        self.load_own(LABELS)
    }

    /// The PlaceMarkers saved (Ctrl+Shift+K), as [`Self::load_labels`].
    pub fn load_placemarkers(&self) -> okur_core::placemarkers::PlaceMarkers {
        self.load_own(PLACEMARKERS)
    }

    /// The Flexible Web rules saved (Insert+Space, then X), as [`Self::load_labels`].
    pub fn load_flexible_web_rules(&self) -> okur_core::flexible_web::Rules {
        self.load_own(FLEXIBLE_WEB)
    }

    /// The notification rules kept (Create Rule... in the Notification History), as
    /// [`Self::load_labels`].
    pub fn load_notification_rules(&self) -> okur_core::ui::notifications::NotificationRules {
        self.load_own(NOTIFICATION_RULES)
    }

    /// One of the files Okur writes itself; none when missing or unreadable (logged).
    fn load_own<T: serde::de::DeserializeOwned + Default>(&self, name: &str) -> T {
        let path = self.folder.join(name);
        let Ok(text) = std::fs::read_to_string(&path) else {
            return T::default();
        };
        toml::from_str(&text)
            .map_err(|e| tracing::warn!(?path, error = %e, "file not read"))
            .unwrap_or_default()
    }

    /// Writes one of Okur's own files whole, through a temporary one: they are Okur's to
    /// write, `header` their comment.
    fn save_own<T: serde::Serialize>(&self, name: &str, header: &str, value: &T) {
        let write = || -> std::io::Result<()> {
            let text = toml::to_string(value).map_err(std::io::Error::other)?;
            std::fs::create_dir_all(&self.folder)?;
            let path = self.folder.join(name);
            let temporary = path.with_extension("toml.tmp");
            std::fs::write(&temporary, format!("# {header}\n{text}"))?;
            std::fs::rename(&temporary, path)
        };
        if let Err(e) = write() {
            tracing::warn!(folder = ?self.folder, file = name, error = %e, "not saved");
        }
    }
}

impl okur_core::labels::LabelStore for SettingsFolder {
    fn save(&self, labels: &okur_core::labels::CustomLabels) {
        self.save_own(
            LABELS,
            "Okur's custom labels (Insert+Ctrl+Tab), by site, then by element.",
            labels,
        );
    }
}

impl okur_core::placemarkers::PlaceMarkerStore for SettingsFolder {
    fn save(&self, markers: &okur_core::placemarkers::PlaceMarkers) {
        self.save_own(
            PLACEMARKERS,
            "Okur's saved PlaceMarkers (Ctrl+Shift+K), by page address.",
            markers,
        );
    }
}

impl okur_core::flexible_web::RuleStore for SettingsFolder {
    fn save(&self, rules: &okur_core::flexible_web::Rules) {
        self.save_own(
            FLEXIBLE_WEB,
            "Okur's Flexible Web rules (Insert+Space, then X): where each applies, and what it hides.",
            rules,
        );
    }
}

impl okur_core::ui::notifications::NotificationRuleStore for SettingsFolder {
    fn save(&self, rules: &okur_core::ui::notifications::NotificationRules) {
        self.save_own(
            NOTIFICATION_RULES,
            "Okur's notification rules (Insert+Space, then N): the first a notification matches decides what is said of it.",
            rules,
        );
    }
}

/// A settings file's values; `None` when there is no file or it cannot be read (logged).
fn read_values(path: &Path) -> Option<Values> {
    let text = std::fs::read_to_string(path).ok()?;
    let table: toml::Table = toml::from_str(&text)
        .map_err(|e| tracing::warn!(?path, error = %e, "settings file not read"))
        .ok()?;
    match serde_json::to_value(table) {
        Ok(Value::Object(values)) => Some(values),
        _ => None,
    }
}

/// The `.toml` files of `folder`, by name without the extension.
fn read_folder(folder: &Path) -> Vec<(String, Values)> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut layers: Vec<(String, Values)> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "toml"))
        .filter_map(|path| {
            let name = path.file_stem()?.to_str()?.to_lowercase();
            Some((name, read_values(&path)?))
        })
        .collect();
    layers.sort_by(|a, b| a.0.cmp(&b.0));
    layers
}

/// Makes `table` hold exactly `values`, leaving every value that did not change as it was
/// written, with its comments.
fn sync(table: &mut toml_edit::Table, values: &Values) {
    let names: Vec<String> = table.iter().map(|(name, _)| name.to_owned()).collect();
    for (index, name) in names.iter().enumerate() {
        if !values.get(name).is_none_or(Value::is_null) {
            continue;
        }
        // The comments above a removed value go to what follows it.
        let comments = table
            .key(name)
            .and_then(|key| key.leaf_decor().prefix())
            .and_then(|prefix| prefix.as_str())
            .filter(|prefix| prefix.contains('#'))
            .map(str::to_owned);
        table.remove(name);
        let Some(comments) = comments else {
            continue;
        };
        let next = names.iter().skip(index + 1).find(|n| table.contains_key(n));
        if let Some(next) = next {
            if let Some(next_table) = table.get_mut(next).and_then(|i| i.as_table_mut()) {
                let prefix = next_table
                    .decor()
                    .prefix()
                    .and_then(|p| p.as_str())
                    .unwrap_or_default()
                    .to_owned();
                next_table
                    .decor_mut()
                    .set_prefix(format!("{comments}{prefix}"));
            } else if let Some(mut key) = table.key_mut(next) {
                let prefix = key
                    .leaf_decor()
                    .prefix()
                    .and_then(|p| p.as_str())
                    .unwrap_or_default()
                    .to_owned();
                key.leaf_decor_mut()
                    .set_prefix(format!("{comments}{prefix}"));
            }
        }
    }
    for (name, value) in values {
        match value {
            Value::Null => {}
            Value::Object(inner) => {
                if !table.get(name).is_some_and(toml_edit::Item::is_table) {
                    // Implicit: a table holding only tables has no header of its own.
                    let mut new = toml_edit::Table::new();
                    new.set_implicit(true);
                    table.insert(name, toml_edit::Item::Table(new));
                }
                if let Some(inner_table) = table.get_mut(name).and_then(|i| i.as_table_mut()) {
                    sync(inner_table, inner);
                }
            }
            value => {
                let unchanged = table
                    .get(name)
                    .and_then(toml_edit::Item::as_value)
                    .is_some_and(|written| json_of(written) == *value);
                if unchanged {
                    continue;
                }
                let Some(mut new) = toml_of(value) else {
                    continue;
                };
                // In place, so that the key's comments and the value's own stay.
                match table.get_mut(name) {
                    Some(item) => {
                        if let Some(old) = item.as_value() {
                            *new.decor_mut() = old.decor().clone();
                        }
                        *item = toml_edit::Item::Value(new);
                    }
                    None => {
                        table.insert(name, toml_edit::Item::Value(new));
                    }
                }
            }
        }
    }
}

/// A setting's value in TOML (TOML has no null: a null is not written).
fn toml_of(value: &Value) -> Option<toml_edit::Value> {
    Some(match value {
        Value::Null => return None,
        Value::Bool(b) => (*b).into(),
        Value::Number(n) => match n.as_i64() {
            Some(i) => i.into(),
            None => n.as_f64()?.into(),
        },
        Value::String(s) => s.as_str().into(),
        Value::Array(items) => {
            let mut array = toml_edit::Array::new();
            for item in items.iter().filter_map(toml_of) {
                array.push(item);
            }
            toml_edit::Value::Array(array)
        }
        Value::Object(fields) => {
            let mut inline = toml_edit::InlineTable::new();
            for (name, field) in fields {
                if let Some(field) = toml_of(field) {
                    inline.insert(name, field);
                }
            }
            toml_edit::Value::InlineTable(inline)
        }
    })
}

/// A written TOML value as a setting's value, to tell whether it changed.
fn json_of(value: &toml_edit::Value) -> Value {
    match value {
        toml_edit::Value::String(s) => Value::String(s.value().clone()),
        toml_edit::Value::Integer(i) => Value::from(*i.value()),
        toml_edit::Value::Float(f) => Value::from(*f.value()),
        toml_edit::Value::Boolean(b) => Value::Bool(*b.value()),
        toml_edit::Value::Datetime(d) => Value::String(d.value().to_string()),
        toml_edit::Value::Array(items) => Value::Array(items.iter().map(json_of).collect()),
        toml_edit::Value::InlineTable(fields) => Value::Object(
            fields
                .iter()
                .map(|(name, field)| (name.to_owned(), json_of(field)))
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use okur_core::settings::{Punctuation, Settings, TypingEcho};
    use okur_core::settings_layers::Scope;

    fn folder(name: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("okur-settings-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        folder
    }

    /// Each layer is saved in its own file with only its changes, and read back the same.
    #[test]
    fn layers_survive_a_save() {
        let root = folder("layers");
        let files = SettingsFolder::at(&root);
        assert_eq!(files.load(), SettingsLayers::default());
        let mut layers = SettingsLayers::default();
        let before = layers.resolve(None, None);
        let mut after = before.clone();
        after.typing_echo = TypingEcho::None;
        after.voice.rate = 4;
        layers.change(&Scope::Global, &before, &after, None);
        let before = layers.resolve(Some("notepad"), None);
        let mut after = before.clone();
        after.voice.punctuation = Punctuation::All;
        layers.change(&Scope::Application("notepad".into()), &before, &after, None);
        let before = layers.resolve(Some("chrome"), Some("example.org"));
        let mut after = before.clone();
        after.forms_mode = okur_core::settings::FormsMode::Manual;
        layers.change(
            &Scope::Site("example.org".into()),
            &before,
            &after,
            Some("chrome"),
        );
        files.save(&layers);
        assert_eq!(files.load(), layers);
        let global = std::fs::read_to_string(root.join("settings.toml")).unwrap_or_default();
        assert!(global.contains("typing_echo = \"none\""), "{global}");
        assert!(global.contains("[voice]\nrate = 4"), "{global}");
        let notepad =
            std::fs::read_to_string(root.join("applications/notepad.toml")).unwrap_or_default();
        assert!(
            notepad.contains("[voice]\npunctuation = \"all\""),
            "{notepad}"
        );
        assert!(root.join("sites/example.org.toml").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Custom labels are saved in labels.toml and read back the same.
    #[test]
    fn labels_survive_a_save() {
        use okur_core::labels::{CustomLabels, LabelStore};
        let root = folder("labels");
        let files = SettingsFolder::at(&root);
        assert_eq!(files.load_labels(), CustomLabels::default());
        let mut labels = CustomLabels::default();
        labels
            .places
            .entry("library.example".into())
            .or_default()
            .insert("id pickup-branch".into(), "lamp".into());
        LabelStore::save(&files, &labels);
        assert_eq!(files.load_labels(), labels);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Saved PlaceMarkers are kept in placemarkers.toml and read back the same.
    #[test]
    fn placemarkers_survive_a_save() {
        use okur_core::placemarkers::{PlaceMarker, PlaceMarkerStore, PlaceMarkers};
        let root = folder("placemarkers");
        let files = SettingsFolder::at(&root);
        assert_eq!(files.load_placemarkers(), PlaceMarkers::default());
        let mut markers = PlaceMarkers::default();
        markers.add(
            "file:///C:/corpus/fixtures/placemarkers.html",
            PlaceMarker {
                name: "Water in a closed bottle".into(),
                offset: 120,
                anchor: None,
                domain: false,
            },
        );
        PlaceMarkerStore::save(&files, &markers);
        assert!(root.join("placemarkers.toml").exists());
        assert_eq!(files.load_placemarkers(), markers);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Saved Flexible Web rules are kept in flexible-web.toml and read back the same.
    #[test]
    fn flexible_web_rules_survive_a_save() {
        use okur_core::flexible_web::{
            Action, Area, Customization, Kind, Place, Rule, RuleStore, Rules,
        };
        let root = folder("flexible-web");
        let files = SettingsFolder::at(&root);
        assert_eq!(files.load_flexible_web_rules(), Rules::default());
        let hide = |kind, text: Option<&str>| Customization {
            action: Action::Hide,
            kind,
            text: text.map(str::to_owned),
            id: Some("events".into()),
            place: Place::FromTop(1),
        };
        let mut rules = Rules::default();
        rules.save(
            Rule {
                name: "Okur lab rule".into(),
                customizations: vec![
                    hide(Kind::Region("complementary".into()), Some("Advertisement")),
                    hide(Kind::Heading(1), None),
                ],
            },
            Area::Page("file:///C:/corpus/fixtures/flexible-web.html".into()),
        );
        rules.save(
            Rule {
                name: "Everywhere".into(),
                customizations: Vec::new(),
            },
            Area::Global,
        );
        rules.set("Okur lab rule", Area::Site("localhost".into()), false);
        RuleStore::save(&files, &rules);
        assert!(root.join("flexible-web.toml").exists());
        assert_eq!(files.load_flexible_web_rules(), rules);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Notification rules are kept in notification-rules.toml and read back the same.
    #[test]
    fn notification_rules_survive_a_save() {
        use okur_core::ui::notifications::{
            NotificationRuleStore, NotificationRules, Rule, RuleType, SpeechAction,
        };
        let root = folder("notification-rules");
        let files = SettingsFolder::at(&root);
        assert_eq!(
            files.load_notification_rules(),
            NotificationRules::default()
        );
        let rules = NotificationRules {
            enabled: false,
            rules: vec![Rule {
                name: "Mail".into(),
                kind: RuleType::BeginsWith,
                text: "New notification from Mail".into(),
                sender: Some("Mail".into()),
                speech: SpeechAction::Mute,
                braille: 1,
                hidden: true,
                enabled: false,
            }],
        };
        NotificationRuleStore::save(&files, &rules);
        assert!(root.join("notification-rules.toml").exists());
        assert_eq!(files.load_notification_rules(), rules);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A user's comments and order stay when Okur changes a value; a value set back to the
    /// default leaves the file.
    #[test]
    fn comments_stay() {
        let root = folder("comments");
        let _ = std::fs::create_dir_all(&root);
        let _ = std::fs::write(
            root.join("settings.toml"),
            "# mine\ntyping_echo = \"none\" # quiet typing\n\n[voice]\n# fast\nrate = 6\n",
        );
        let files = SettingsFolder::at(&root);
        let mut layers = files.load();
        assert_eq!(layers.resolve(None, None).voice.rate, 6);
        let before = layers.resolve(None, None);
        let mut after = before.clone();
        after.voice.rate = 8;
        after.typing_echo = Settings::default().typing_echo;
        layers.change(&Scope::Global, &before, &after, None);
        files.save(&layers);
        let text = std::fs::read_to_string(root.join("settings.toml")).unwrap_or_default();
        assert_eq!(text, "# mine\n\n[voice]\n# fast\nrate = 8\n");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A file Okur cannot read is left out, and the others still count.
    #[test]
    fn an_unreadable_file_is_left_out() {
        let root = folder("broken");
        let _ = std::fs::create_dir_all(root.join("applications"));
        let _ = std::fs::write(root.join("settings.toml"), "typing_echo = \"none\"\n");
        let _ = std::fs::write(root.join("applications/notepad.toml"), "voice = [\n");
        let layers = SettingsFolder::at(&root).load();
        assert_eq!(layers.resolve(None, None).typing_echo, TypingEcho::None);
        assert!(layers.applications.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }
}
