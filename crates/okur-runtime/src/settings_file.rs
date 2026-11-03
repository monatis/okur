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

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

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
    /// What each settings file held when it was last read or written, by path: a save writes
    /// only the files whose values changed. It read every application's and site's file to
    /// list them, then read and rewrote each one, on the engine's thread: 100 ms with 300 site
    /// files (performance review 2026-10-25, finding 7).
    known: Arc<Mutex<HashMap<PathBuf, Values>>>,
}

impl SettingsFolder {
    /// The settings files in `folder` (the program says where).
    pub fn at(folder: impl Into<PathBuf>) -> Self {
        Self {
            folder: folder.into(),
            known: Arc::default(),
        }
    }

    /// Notes that the file at `path` holds `values`.
    fn know(&self, path: &Path, values: &Values) {
        if let Ok(mut known) = self.known.lock() {
            known.insert(path.to_owned(), values.clone());
        }
    }

    /// Whether the file at `path` is known to hold `values` already.
    fn holds(&self, path: &Path, values: &Values) -> bool {
        self.known
            .lock()
            .is_ok_and(|known| known.get(path) == Some(values))
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
        self.know(&self.folder.join(GLOBAL), &layers.global);
        for (kind, layer) in [
            (APPLICATIONS, &mut layers.applications),
            (SITES, &mut layers.sites),
        ] {
            let folder = self.folder.join(kind);
            for (name, values) in read_folder(&folder) {
                self.know(&folder.join(format!("{name}.toml")), &values);
                layer.insert(name, values);
            }
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
        write_whole(path, &text)
    }
}

/// Writes `text` as the file at `path` through a temporary file, on the disk before it takes
/// the file's name: renamed with its data still in the cache, a power cut could leave the new
/// name on a file that does not parse, which the next save then replaced (performance review
/// 2026-11-02, speech finding 7).
fn write_whole(path: &Path, text: &str) -> std::io::Result<()> {
    use std::io::Write;
    let temporary = path.with_extension("toml.tmp");
    {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
    }
    std::fs::rename(&temporary, path)
}

/// Moves a file that does not parse aside, beside it with `.unreadable` and the time in its
/// name, so that the next save does not write over what it holds: one hand-edit's typo, then
/// one PlaceMarker added, and every PlaceMarker saved was gone. The name is returned.
fn set_aside(path: &Path) -> Option<PathBuf> {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    let mut name = path.file_name()?.to_os_string();
    name.push(format!(".unreadable-{seconds}"));
    let aside = path.with_file_name(name);
    std::fs::rename(path, &aside).ok()?;
    Some(aside)
}

impl SettingsStore for SettingsFolder {
    fn save(&self, layers: &SettingsLayers) {
        let mut written = Vec::new();
        // A file is written only when its values changed, and is then known to hold them.
        let mut save = |path: PathBuf, values: &Values, heading: &str| {
            if self.holds(&path, values) {
                return;
            }
            let result = Self::write(&path, values, heading);
            if result.is_ok() {
                self.know(&path, values);
            }
            written.push(result);
        };
        save(
            self.folder.join(GLOBAL),
            &layers.global,
            "# Okur's settings for all applications: only what differs from Okur's defaults.\n\
             # Okur rewrites only the values it changes; comments stay.\n",
        );
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
            for name in file_names(&folder) {
                if !layer.contains_key(&name) {
                    save(folder.join(format!("{name}.toml")), &Values::new(), heading);
                }
            }
            for (name, values) in layer {
                save(folder.join(format!("{name}.toml")), values, heading);
            }
        }
        for result in written {
            if let Err(e) = result {
                tracing::warn!(folder = ?self.folder, error = %e, "settings not saved");
            }
        }
    }

    fn save_history(&self, history: &okur_core::ui::settings_tree::SavedHistory) {
        self.save_own(
            LAST_CHANGED,
            "Okur's Last Changed Settings: each change's place and setting, newest first.",
            history,
        );
    }
}

/// The settings folder written on a thread of its own: Apply and OK hand it what to keep and go
/// on, and the files are written behind them, the newest of what waits only. They were written
/// on the engine's thread, which the keyboard hook waits on: 100 ms with 300 site files
/// (performance review 2026-10-25, finding 7).
#[derive(Clone)]
pub struct BackgroundStore {
    writes: std::sync::mpsc::Sender<Write>,
    /// The folder itself, for the writes the thread could not take (it could not start).
    folder: SettingsFolder,
}

/// What the writer thread is handed.
enum Write {
    Layers(SettingsLayers),
    History(okur_core::ui::settings_tree::SavedHistory),
    /// One of Okur's own files, by its name, written by the job: custom labels, PlaceMarkers,
    /// Flexible Web and notification rules, which were serialized and written on the
    /// engine's thread too (performance review 2026-11-02, speech finding 9).
    Own(&'static str, OwnWrite),
    /// Answered once everything handed before it is written.
    Flush(std::sync::mpsc::Sender<()>),
}

/// Writes one of Okur's own files into the folder.
type OwnWrite = Box<dyn FnOnce(&SettingsFolder) + Send>;

/// How long Okur's exit waits for the settings to be written.
const FLUSH_WAIT: std::time::Duration = std::time::Duration::from_secs(2);

impl BackgroundStore {
    pub fn new(folder: SettingsFolder) -> Self {
        let (writes, waiting) = std::sync::mpsc::channel::<Write>();
        let writer = folder.clone();
        let started = std::thread::Builder::new()
            .name("okur-settings".into())
            .spawn(move || {
                while let Ok(first) = waiting.recv() {
                    let (mut layers, mut history, mut flushes) = (None, None, Vec::new());
                    let mut own: Vec<(&str, OwnWrite)> = Vec::new();
                    for write in std::iter::once(first).chain(waiting.try_iter()) {
                        match write {
                            Write::Layers(newer) => layers = Some(newer),
                            Write::History(newer) => history = Some(newer),
                            // The newest of each file only.
                            Write::Own(name, job) => {
                                own.retain(|(waiting, _)| *waiting != name);
                                own.push((name, job));
                            }
                            Write::Flush(done) => flushes.push(done),
                        }
                    }
                    if let Some(layers) = layers {
                        writer.save(&layers);
                    }
                    if let Some(history) = history {
                        writer.save_history(&history);
                    }
                    for (_, job) in own {
                        job(&writer);
                    }
                    for done in flushes {
                        let _ = done.send(());
                    }
                }
            });
        if let Err(e) = started {
            tracing::warn!(error = %e, "settings writer not started: settings written at once");
        }
        Self { writes, folder }
    }

    /// Hands one of Okur's own files to the writer: `value`, written by `write`; at once
    /// where the writer did not start.
    fn own<T: Send + 'static>(&self, name: &'static str, value: T, write: fn(&SettingsFolder, &T)) {
        let job: OwnWrite = Box::new(move |folder| write(folder, &value));
        if let Err(std::sync::mpsc::SendError(Write::Own(_, job))) =
            self.writes.send(Write::Own(name, job))
        {
            job(&self.folder);
        }
    }
}

impl okur_core::labels::LabelStore for BackgroundStore {
    fn save(&self, labels: &okur_core::labels::CustomLabels) {
        self.own(LABELS, labels.clone(), |folder, labels| {
            okur_core::labels::LabelStore::save(folder, labels)
        });
    }
}

impl okur_core::placemarkers::PlaceMarkerStore for BackgroundStore {
    fn save(&self, markers: &okur_core::placemarkers::PlaceMarkers) {
        self.own(PLACEMARKERS, markers.clone(), |folder, markers| {
            okur_core::placemarkers::PlaceMarkerStore::save(folder, markers)
        });
    }
}

impl okur_core::flexible_web::RuleStore for BackgroundStore {
    fn save(&self, rules: &okur_core::flexible_web::Rules) {
        self.own(FLEXIBLE_WEB, rules.clone(), |folder, rules| {
            okur_core::flexible_web::RuleStore::save(folder, rules)
        });
    }
}

impl okur_core::ui::notifications::NotificationRuleStore for BackgroundStore {
    fn save(&self, rules: &okur_core::ui::notifications::NotificationRules) {
        self.own(NOTIFICATION_RULES, rules.clone(), |folder, rules| {
            okur_core::ui::notifications::NotificationRuleStore::save(folder, rules)
        });
    }
}

impl SettingsStore for BackgroundStore {
    fn save(&self, layers: &SettingsLayers) {
        if self.writes.send(Write::Layers(layers.clone())).is_err() {
            self.folder.save(layers);
        }
    }

    fn save_history(&self, history: &okur_core::ui::settings_tree::SavedHistory) {
        if self.writes.send(Write::History(history.clone())).is_err() {
            self.folder.save_history(history);
        }
    }

    fn flush(&self) {
        let (done, written) = std::sync::mpsc::channel();
        if self.writes.send(Write::Flush(done)).is_ok() && written.recv_timeout(FLUSH_WAIT).is_err()
        {
            tracing::warn!("settings not written before exit");
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
/// Last Changed Settings' file: each change's place and setting, newest first.
const LAST_CHANGED: &str = "last-changed.toml";

impl SettingsFolder {
    /// The settings files in `folder` (the program says where).
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

    /// Last Changed Settings as kept, as [`Self::load_labels`].
    pub fn load_settings_history(&self) -> okur_core::ui::settings_tree::SavedHistory {
        self.load_own(LAST_CHANGED)
    }

    /// One of the files Okur writes itself; none when missing or unreadable. One that does not
    /// parse is set aside, logged, and kept ([`set_aside`]).
    fn load_own<T: serde::de::DeserializeOwned + Default>(&self, name: &str) -> T {
        let path = self.folder.join(name);
        let Ok(text) = std::fs::read_to_string(&path) else {
            return T::default();
        };
        toml::from_str(&text).unwrap_or_else(|e| {
            let aside = set_aside(&path);
            tracing::warn!(?path, ?aside, error = %e, "file not read; set aside");
            T::default()
        })
    }

    /// Writes one of Okur's own files whole, through a temporary one: they are Okur's to
    /// write, `header` their comment.
    fn save_own<T: serde::Serialize>(&self, name: &str, header: &str, value: &T) {
        let write = || -> std::io::Result<()> {
            let text = toml::to_string(value).map_err(std::io::Error::other)?;
            std::fs::create_dir_all(&self.folder)?;
            write_whole(&self.folder.join(name), &format!("# {header}\n{text}"))
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

/// The names of the `.toml` files of `folder`, without the extension, unread.
fn file_names(folder: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "toml"))
        .filter_map(|path| Some(path.file_stem()?.to_str()?.to_lowercase()))
        .collect()
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

    /// One of Okur's own files that does not parse is set aside as it is read, and the next
    /// save writes a new one beside it.
    #[test]
    fn an_unreadable_own_file_is_kept_aside() {
        let root = folder("unreadable");
        let files = SettingsFolder::at(&root);
        assert!(std::fs::create_dir_all(&root).is_ok());
        let labels = root.join(LABELS);
        assert!(std::fs::write(&labels, "[sites\nbroken = ").is_ok());
        assert_eq!(
            files.load_labels(),
            okur_core::labels::CustomLabels::default()
        );
        okur_core::labels::LabelStore::save(&files, &okur_core::labels::CustomLabels::default());
        let entries: Vec<String> = std::fs::read_dir(&root)
            .map(|dir| {
                dir.flatten()
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        let aside: Vec<&String> = entries
            .iter()
            .filter(|name| name.starts_with("labels.toml.unreadable-"))
            .collect();
        assert_eq!(aside.len(), 1, "{entries:?}");
        let kept = aside
            .first()
            .map(|name| std::fs::read_to_string(root.join(name)).unwrap_or_default());
        assert_eq!(kept.as_deref(), Some("[sites\nbroken = "));
        assert!(entries.contains(&LABELS.to_owned()), "{entries:?}");
        assert!(
            !entries.iter().any(|name| name.ends_with(".tmp")),
            "{entries:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Okur's own files handed to the background store are written off the caller's thread,
    /// the newest of each, and are in the folder once it is flushed.
    #[test]
    fn the_background_store_writes_own_files() {
        use okur_core::labels::LabelStore;
        let root = folder("background-own");
        let store = BackgroundStore::new(SettingsFolder::at(&root));
        let empty = okur_core::labels::CustomLabels::default();
        LabelStore::save(&store, &empty);
        LabelStore::save(&store, &empty);
        SettingsStore::flush(&store);
        assert!(root.join(LABELS).exists());
        assert_eq!(SettingsFolder::at(&root).load_labels(), empty);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The settings handed to the background store are in the files once it is flushed, the
    /// newest of them.
    #[test]
    fn the_background_store_writes_the_newest_settings() {
        let root = folder("background");
        let store = BackgroundStore::new(SettingsFolder::at(&root));
        let mut layers = SettingsLayers::default();
        let before = layers.resolve(None, None);
        for rate in [3, 6] {
            let mut after = before.clone();
            after.voice.rate = rate;
            let mut changed = layers.clone();
            changed.change(&Scope::Global, &before, &after, None);
            store.save(&changed);
            layers = changed;
        }
        store.flush();
        assert_eq!(SettingsFolder::at(&root).load(), layers);
        assert_eq!(
            SettingsFolder::at(&root)
                .load()
                .resolve(None, None)
                .voice
                .rate,
            6
        );
        let _ = std::fs::remove_dir_all(&root);
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

    /// A save writes only the files whose values changed: one application's file is not
    /// written again for a change to all applications', and an emptied one is still emptied.
    /// Every file was read and rewritten at each Apply (performance review 2026-10-25,
    /// finding 7).
    #[test]
    fn a_save_writes_only_what_changed() {
        let root = folder("only-changed");
        let files = SettingsFolder::at(&root);
        let mut layers = SettingsLayers::default();
        let before = layers.resolve(Some("notepad"), None);
        let mut after = before.clone();
        after.voice.punctuation = Punctuation::All;
        layers.change(&Scope::Application("notepad".into()), &before, &after, None);
        files.save(&layers);
        let notepad = root.join("applications/notepad.toml");
        // Notepad's file edited by hand while Okur runs: a save for another layer leaves it as
        // it is, where it wrote Okur's value back over the user's.
        let mine = "# my Notepad\n[voice]\npunctuation = \"none\"\n";
        let _ = std::fs::write(&notepad, mine);
        let before = layers.resolve(None, None);
        let mut after = before.clone();
        after.voice.rate = 4;
        layers.change(&Scope::Global, &before, &after, None);
        files.save(&layers);
        assert_eq!(
            std::fs::read_to_string(&notepad).unwrap_or_default(),
            mine,
            "Notepad's layer did not change, and its file was written again"
        );
        let global = std::fs::read_to_string(root.join("settings.toml")).unwrap_or_default();
        assert!(global.contains("rate = 4"), "{global}");
        // Notepad's layer emptied: its file, listed without being read, loses the value.
        layers.applications.remove("notepad");
        files.save(&layers);
        let emptied = std::fs::read_to_string(&notepad).unwrap_or_default();
        assert!(!emptied.contains("punctuation"), "{emptied}");
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
                shortened: String::new(),
                sound: String::new(),
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

    /// Last Changed Settings is read back as it was written, newest first.
    #[test]
    fn the_settings_history_is_kept() {
        use okur_core::ui::settings_tree::{SavedChange, SavedHistory};
        let root = folder("history");
        let files = SettingsFolder::at(&root);
        assert_eq!(files.load_settings_history(), SavedHistory::default());
        let history = SavedHistory {
            changed: vec![
                SavedChange {
                    application: Some("notepad".into()),
                    site: None,
                    setting: "typing_echo".into(),
                },
                SavedChange {
                    application: None,
                    site: None,
                    setting: "voice.rate".into(),
                },
            ],
        };
        files.save_history(&history);
        assert_eq!(files.load_settings_history(), history);
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
