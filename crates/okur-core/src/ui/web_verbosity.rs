//! Configure Web Verbosity Levels (Settings Center, Web / HTML / PDFs, Reading): one level's
//! choices of which structures' starts and ends the virtual cursor says, as JAWS's dialog lists
//! them [obs, the walk of Configure Web Verbosity Levels, web-verbosity-walk.ps1]:
//! - "Medium Virtual Cursor Verbosity Preferences dialog | Announce Start and End For: Tree view |
//!   Block Quotation checked | 1 of 17": the structures, Regions a group of eleven region kinds;
//! - a read-only text under the tree, the selected item's description; OK and Cancel.
//!
//! Space turns the selected item; on Regions, every region kind at once. OK gives Settings
//! Center's draft the choices ([`crate::settings::WebStructureChange`]), kept when Settings
//! Center's own OK or Apply is pressed, as JAWS keeps them in the application's settings file
//! [obs, web-verbosity-store-probe.ps1].

use okur_ui_model::{Control, ControlKind, DialogKey, DialogSpec, FromUi, ToUi, TreeItem, Value};

use crate::messages::msg;
use crate::settings::{WebStructure, WebStructureChange, WebVerbosity};

const TREE: &str = "structures";
const DESCRIPTION: &str = "description";

/// What the engine does after a message from the dialog.
#[derive(Debug, Clone, PartialEq)]
pub enum WebLevelOutcome {
    Nothing,
    Send(Vec<ToUi>),
    /// OK: these are the choices now.
    Done(Vec<WebStructureChange>),
    Closed,
}

/// A row of the tree: a structure, or the Regions group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    Structure(WebStructure),
    Regions,
}

/// The region kinds, under Regions.
fn is_region(structure: WebStructure) -> bool {
    use WebStructure::*;
    matches!(
        structure,
        ApplicationRegion
            | Article
            | Banner
            | Complementary
            | ContentInformation
            | Document
            | Form
            | Main
            | Navigation
            | Region
            | Search
    )
}

/// The tree's rows in JAWS's order: the structures, Regions with its kinds after Note.
fn rows() -> Vec<Row> {
    let mut rows = Vec::new();
    for structure in WebStructure::ALL {
        if structure == WebStructure::ApplicationRegion {
            rows.push(Row::Regions);
        }
        rows.push(Row::Structure(structure));
    }
    rows
}

/// A structure's name in the tree.
fn name(structure: WebStructure) -> String {
    use WebStructure::*;
    match structure {
        BlockQuotation => msg::web_structure_block_quotation(),
        ClickableElements => msg::web_structure_clickable(),
        DescribedBy => msg::web_structure_described_by(),
        Description => msg::web_structure_description(),
        EditableSection => msg::web_structure_editable_section(),
        Feed => msg::web_structure_feed(),
        Figure => msg::web_structure_figure(),
        FlashObject => msg::web_structure_flash(),
        Frame => msg::web_structure_frame(),
        GroupBox => msg::web_structure_group_box(),
        List => msg::web_structure_list(),
        Log => msg::web_structure_log(),
        Note => msg::web_structure_note(),
        ApplicationRegion => msg::web_structure_application(),
        Article => msg::web_structure_article(),
        Banner => msg::web_structure_banner(),
        Complementary => msg::web_structure_complementary(),
        ContentInformation => msg::web_structure_content_information(),
        Document => msg::web_structure_document(),
        Form => msg::web_structure_form(),
        Main => msg::web_structure_main(),
        Navigation => msg::web_structure_navigation(),
        Region => msg::web_structure_region(),
        Search => msg::web_structure_search(),
        TabPanel => msg::web_structure_tab_panel(),
        TableOrGrid => msg::web_structure_table(),
        Toolbar => msg::web_structure_toolbar(),
    }
    .to_string()
}

/// One level's choices while its dialog is open.
#[derive(Debug, Clone)]
pub struct WebLevelDialog {
    pub id: u32,
    level: WebVerbosity,
    changes: Vec<WebStructureChange>,
    selected: usize,
}

impl WebLevelDialog {
    /// The dialog for `level` on `changes`, Settings Center's draft's choices.
    pub fn new(id: u32, level: WebVerbosity, changes: Vec<WebStructureChange>) -> Self {
        Self {
            id,
            level,
            changes,
            selected: 0,
        }
    }

    fn announced(&self, structure: WebStructure) -> bool {
        !self.level.left_out(&self.changes).contains(&structure)
    }

    fn item(&self, row: Row) -> TreeItem {
        match row {
            Row::Structure(structure) => TreeItem {
                text: name(structure),
                level: usize::from(is_region(structure)),
                open: true,
                checked: Some(self.announced(structure)),
            },
            // The group is checked while all its kinds are (JAWS says "partially checked"
            // while some are).
            Row::Regions => TreeItem {
                text: msg::web_structure_regions().to_string(),
                level: 0,
                open: true,
                checked: Some(
                    WebStructure::ALL
                        .into_iter()
                        .filter(|s| is_region(*s))
                        .all(|s| self.announced(s)),
                ),
            },
        }
    }

    fn description(&self) -> String {
        match rows().get(self.selected) {
            Some(Row::Regions) => msg::web_structure_regions_description().to_string(),
            Some(Row::Structure(structure)) => {
                msg::web_structure_description_of(name(*structure)).render()
            }
            None => String::new(),
        }
    }

    pub fn spec(&self) -> DialogSpec {
        let control = |id: &str, label: String, kind: ControlKind| Control {
            id: id.to_owned(),
            label,
            kind,
            enabled: true,
            hidden: false,
        };
        DialogSpec {
            id: self.id,
            title: msg::web_level_title(super::settings_tree::level_name(self.level)).render(),
            controls: vec![
                control(
                    TREE,
                    msg::web_level_tree().to_string(),
                    ControlKind::Tree {
                        items: rows().into_iter().map(|r| self.item(r)).collect(),
                        selected: self.selected,
                    },
                ),
                control(
                    DESCRIPTION,
                    String::new(),
                    ControlKind::Note {
                        text: self.description(),
                    },
                ),
                control(
                    "ok",
                    msg::web_level_ok().to_string(),
                    ControlKind::Button { default: true },
                ),
                control(
                    "cancel",
                    msg::web_level_cancel().to_string(),
                    ControlKind::Button { default: false },
                ),
            ],
            keys: Vec::new(),
        }
    }

    /// `structure` said at this level or not, its change kept where it differs from the
    /// default.
    fn set(&mut self, structure: WebStructure, on: bool) {
        let key = structure.key();
        let current = |level: WebVerbosity| !level.left_out(&self.changes).contains(&structure);
        let (mut high, mut medium, mut low) = (
            current(WebVerbosity::High),
            current(WebVerbosity::Medium),
            current(WebVerbosity::Low),
        );
        match self.level {
            WebVerbosity::High => high = on,
            WebVerbosity::Medium => medium = on,
            WebVerbosity::Low => low = on,
        }
        self.changes.retain(|c| c.key != key);
        let default = (
            WebVerbosity::High.announces(structure),
            WebVerbosity::Medium.announces(structure),
            WebVerbosity::Low.announces(structure),
        );
        if (high, medium, low) != default {
            self.changes.push(WebStructureChange {
                key: key.to_owned(),
                high,
                medium,
                low,
            });
        }
    }

    /// Space on the selected row: it turned, every region kind with Regions.
    fn turn(&mut self) -> WebLevelOutcome {
        let Some(row) = rows().get(self.selected).copied() else {
            return WebLevelOutcome::Nothing;
        };
        let before: Vec<TreeItem> = rows().into_iter().map(|r| self.item(r)).collect();
        match row {
            Row::Structure(structure) => {
                let on = !self.announced(structure);
                self.set(structure, on);
            }
            Row::Regions => {
                let kinds: Vec<WebStructure> = WebStructure::ALL
                    .into_iter()
                    .filter(|s| is_region(*s))
                    .collect();
                let on = !kinds.iter().all(|s| self.announced(*s));
                for kind in kinds {
                    self.set(kind, on);
                }
            }
        }
        // Each row's item set again where it changed (the tree filled again would move its
        // selection to the top as it fills).
        let sent = rows()
            .into_iter()
            .enumerate()
            .zip(before)
            .filter_map(|((index, row), was)| {
                let item = self.item(row);
                (item != was).then(|| ToUi::DialogSet {
                    id: self.id,
                    control: TREE.to_owned(),
                    value: Value::TreeItem(index, item),
                })
            })
            .collect();
        WebLevelOutcome::Send(sent)
    }

    pub fn handle(&mut self, message: &FromUi) -> WebLevelOutcome {
        match message {
            FromUi::DialogChanged {
                id,
                control,
                value: Value::Selected(row),
            } if *id == self.id && control == TREE => {
                self.selected = *row;
                WebLevelOutcome::Send(vec![ToUi::DialogSet {
                    id: self.id,
                    control: DESCRIPTION.to_owned(),
                    value: Value::Text(self.description()),
                }])
            }
            FromUi::DialogKey {
                id,
                control,
                key: DialogKey::Change | DialogKey::ChangeBack,
            } if *id == self.id && control == TREE => self.turn(),
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                "ok" => WebLevelOutcome::Done(self.changes.clone()),
                "cancel" => WebLevelOutcome::Closed,
                _ => WebLevelOutcome::Nothing,
            },
            FromUi::DialogClosed { id } if *id == self.id => WebLevelOutcome::Closed,
            _ => WebLevelOutcome::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree_of(spec: &DialogSpec) -> Vec<TreeItem> {
        spec.controls
            .iter()
            .find_map(|c| match &c.kind {
                ControlKind::Tree { items, .. } => Some(items.clone()),
                _ => None,
            })
            .unwrap_or_default()
    }

    /// Medium's tree as JAWS's lists it: 17 items at the top, Regions with its eleven kinds,
    /// each checked as the level says it [jaws-reading-settings-research.md §14].
    #[test]
    fn a_level_lists_its_structures_checked_as_it_says_them() {
        let dialog = WebLevelDialog::new(3, WebVerbosity::Medium, Vec::new());
        let spec = dialog.spec();
        assert_eq!(spec.title, "Medium Virtual Cursor Verbosity Preferences");
        let items = tree_of(&spec);
        assert_eq!(items.iter().filter(|i| i.level == 0).count(), 17);
        assert_eq!(items.iter().filter(|i| i.level == 1).count(), 11);
        let checked = |text: &str| {
            items
                .iter()
                .find(|i| i.text == text)
                .and_then(|i| i.checked)
        };
        assert_eq!(checked("Block Quotation"), Some(true));
        assert_eq!(checked("Banner"), Some(false));
        assert_eq!(checked("Main Region"), Some(true));
        assert_eq!(checked("Regions"), Some(false));
    }

    /// Space turns the selected structure, and OK gives the changes as JAWS keeps them.
    #[test]
    fn space_turns_a_structure_and_ok_gives_the_change() {
        let mut dialog = WebLevelDialog::new(3, WebVerbosity::Medium, Vec::new());
        let banner = rows()
            .iter()
            .position(|r| *r == Row::Structure(WebStructure::Banner))
            .unwrap_or_default();
        dialog.handle(&FromUi::DialogChanged {
            id: 3,
            control: TREE.into(),
            value: Value::Selected(banner),
        });
        dialog.handle(&FromUi::DialogKey {
            id: 3,
            control: TREE.into(),
            key: DialogKey::Change,
        });
        assert_eq!(
            dialog.handle(&FromUi::DialogPressed {
                id: 3,
                control: "ok".into(),
            }),
            WebLevelOutcome::Done(vec![WebStructureChange {
                key: "BannerRegion".into(),
                high: true,
                medium: true,
                low: false,
            }])
        );
        // Turned back, nothing differs from the default.
        dialog.handle(&FromUi::DialogKey {
            id: 3,
            control: TREE.into(),
            key: DialogKey::Change,
        });
        assert!(matches!(
            dialog.handle(&FromUi::DialogPressed { id: 3, control: "ok".into() }),
            WebLevelOutcome::Done(changes) if changes.is_empty()
        ));
    }
}
