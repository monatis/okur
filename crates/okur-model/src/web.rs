//! A web page's element as Chromium gives it on any platform, and its conversion into the
//! engine's AOM (docs/plans/2026-11-08-structure-for-three-platforms.md, step 2). Chromium gives a
//! page through IAccessible2 on Windows, AT-SPI on Linux and AX on macOS; each platform's model
//! turns its objects into [`WebNode`]s (`ia2_to_web` in okur-windows-model), and [`web_to_aom`]
//! makes the AOM the same way for all of them.
//!
//! A node has a main role ([`WebRole`], what MSAA calls it on Windows), a finer one where Chromium
//! gives it ([`WebMark`]: a heading, a block quote, a toggle button, a generic section, an edit's
//! insertion and deletion, a mark), named states ([`WebStates`]), and Chromium's object
//! attributes (`tag`, `display`, `xml-roles`, `level`, ...), which carry most of what JAWS reads.

use std::collections::{BTreeMap, HashMap};

use okur_core::aom::TextFormatting;
use okur_core::{AomNode, NodeStates, Role};

/// What an element is, as Chromium's main role for it says (MSAA's on Windows).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WebRole {
    Document,
    Dialog,
    MenuBar,
    MenuItem,
    Link,
    Image,
    Separator,
    /// A text leaf.
    StaticText,
    /// A text object (an input, a text area), an edit when editable.
    Text,
    Button,
    /// A button with a popup.
    MenuButton,
    CheckBox,
    RadioButton,
    ComboBox,
    ProgressBar,
    Slider,
    List,
    ListItem,
    Tree,
    TreeItem,
    TabList,
    Tab,
    TabPanel,
    Table,
    Row,
    Cell,
    ColumnHeader,
    RowHeader,
    Math,
    /// A grouping: a fieldset, a group, and (with [`WebMark::Section`]) a generic `div`.
    Grouping,
    /// Any other.
    #[default]
    Other,
}

/// A finer role Chromium gives beside the main one (IA2's own roles on Windows).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebMark {
    Heading,
    BlockQuote,
    ToggleButton,
    /// A generic container (`div`, `span`): a grouping only to MSAA.
    Section,
    ContentDeletion,
    ContentInsertion,
    Mark,
}

/// An element's states, by name.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WebStates {
    pub checked: bool,
    pub mixed: bool,
    pub pressed: bool,
    pub selected: bool,
    pub expanded: bool,
    pub collapsed: bool,
    pub focused: bool,
    pub focusable: bool,
    pub unavailable: bool,
    pub invisible: bool,
    pub protected: bool,
    pub readonly: bool,
    /// A visited link.
    pub traversed: bool,
    pub editable: bool,
    pub horizontal: bool,
    pub vertical: bool,
    pub invalid_entry: bool,
    pub modal: bool,
    pub multi_line: bool,
    pub required: bool,
    pub supports_autocompletion: bool,
}

/// One element of a page.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WebNode {
    pub role: WebRole,
    pub mark: Option<WebMark>,
    pub name: Option<String>,
    pub value: Option<String>,
    pub description: Option<String>,
    /// The default action's name: Chromium's "click", "click ancestor".
    pub default_action: Option<String>,
    pub value_min: Option<f64>,
    pub value_max: Option<f64>,
    pub states: WebStates,
    /// The platform's id of the element, which relations name and actions find it by.
    pub unique_id: Option<i32>,
    /// Chromium's object attributes.
    pub attributes: BTreeMap<String, String>,
    /// Level, set size and position within its set.
    pub group_position: Option<[i32; 3]>,
    pub text: Option<String>,
    /// The text attributes of its text's first run (font, size, weight, color).
    pub text_attributes: Option<BTreeMap<String, String>>,
    /// The elements it is related to, by unique id: `error`, `labelFor`, `descriptionFor`,
    /// `flowsTo`.
    pub relations: BTreeMap<String, Vec<i32>>,
    pub children: Vec<WebNode>,
}

/// ARIA landmark roles, as Chromium reports them in `xml-roles`.
const LANDMARKS: [&str; 8] = [
    "banner",
    "complementary",
    "contentinfo",
    "form",
    "main",
    "navigation",
    "region",
    "search",
];

/// Whether Chromium reports `node` as a grouping of its own (a fieldset, `role=group`, a
/// log): MSAA calls every generic `div` and `span` a grouping too, and only the finer role
/// (a section) tells them apart. WhatsApp names its message wrappers with `aria-label`, and
/// JAWS reads them as plain text, not as groups.
fn is_grouping(node: &WebNode) -> bool {
    node.role == WebRole::Grouping && node.mark != Some(WebMark::Section)
}

/// Whether `node` is a generic element (`div`, `span`) with an `aria-label` and nothing
/// inside: it reads as its label (WhatsApp's "Siz:" before each of the user's messages,
/// which JAWS reads as a line of its own).
fn is_labelled_empty_generic(node: &WebNode) -> bool {
    node.mark == Some(WebMark::Section)
        && node.children.is_empty()
        && node.attributes.get("name-from").map(String::as_str) == Some("attribute")
        && node.name.as_deref().is_some_and(|n| !n.trim().is_empty())
}

/// Whether a CSS `display` value makes a block-level box (anything but inline content).
fn is_block_display(display: Option<&str>) -> bool {
    display.is_some_and(|d| !d.is_empty() && !d.starts_with("inline") && d != "contents")
}

/// A font size from text attributes in points: IA2 specifies points (`12pt`), and CSS pixels
/// (`16px`) are three quarters of a point.
fn font_size_points(raw: &str) -> Option<f32> {
    let raw = raw.trim();
    if let Some(px) = raw.strip_suffix("px") {
        return px.trim().parse::<f32>().ok().map(|px| px * 0.75);
    }
    raw.strip_suffix("pt").unwrap_or(raw).trim().parse().ok()
}

/// Say Font's and Say Color's formatting from a text run (`font-family`, `font-size`,
/// `font-weight`, `font-style`, `text-underline-style`, `color`, `background-color`), and
/// what skim reading finds text by (`text-line-through-style`, `text-position`).
fn text_formatting(attributes: &BTreeMap<String, String>) -> Option<TextFormatting> {
    let get = |k: &str| attributes.get(k).map(|v| v.trim().trim_matches('"'));
    let font_family = get("font-family")
        .filter(|f| !f.is_empty())
        .map(str::to_owned);
    let font_size_pt = get("font-size").and_then(font_size_points);
    if font_family.is_none() && font_size_pt.is_none() {
        return None;
    }
    let bold =
        get("font-weight").is_some_and(|w| w == "bold" || w.parse::<u32>().is_ok_and(|n| n >= 600));
    Some(TextFormatting {
        font_family,
        font_size_pt,
        bold,
        italic: get("font-style").is_some_and(|s| s == "italic" || s == "oblique"),
        underline: get("text-underline-style").is_some_and(|s| !s.is_empty() && s != "none"),
        strikeout: get("text-line-through-style").is_some_and(|s| !s.is_empty() && s != "none"),
        superscript: get("text-position") == Some("super"),
        subscript: get("text-position") == Some("sub"),
        alignment: None,
        color: get("color").and_then(okur_core::color::Rgb::parse_css),
        background: get("background-color").and_then(okur_core::color::Rgb::parse_css),
    })
}

/// Okur's role for a page's element.
fn map_role(node: &WebNode) -> Role {
    let attr = |k: &str| node.attributes.get(k).map(String::as_str);
    let xml_role = attr("xml-roles")
        .and_then(|r| r.split_whitespace().next())
        .unwrap_or_default();
    let named = node.name.as_deref().is_some_and(|n| !n.trim().is_empty());

    if node.mark == Some(WebMark::Heading) {
        let level = attr("level")
            .and_then(|l| l.parse().ok())
            .or_else(|| {
                node.group_position
                    .and_then(|[l, _, _]| u32::try_from(l).ok())
            })
            .unwrap_or(2);
        return Role::Heading { level };
    }
    if node.mark == Some(WebMark::BlockQuote) {
        return Role::BlockQuote;
    }
    match xml_role {
        "dialog" => return Role::Dialog,
        "alertdialog" => return Role::AlertDialog,
        "tabpanel" => return Role::TabPanel,
        // Forms and generic regions are landmarks only when named.
        "form" | "region" if !named => {}
        r if LANDMARKS.contains(&r) => {
            return Role::Landmark {
                landmark_type: r.to_owned(),
            }
        }
        _ => {}
    }
    let editable = node.states.editable;
    match node.role {
        WebRole::Document if attr("tag") == Some("article") => Role::Group,
        // An iframe, which holds its page's own document [obs, 1175's snapshot].
        WebRole::Document if matches!(attr("tag"), Some("iframe" | "frame")) => Role::IFrame,
        WebRole::Document => Role::Document,
        WebRole::Dialog => Role::Dialog,
        WebRole::MenuBar => Role::MenuBar,
        WebRole::MenuItem => Role::MenuItem,
        WebRole::Link => Role::Link {
            href: node.value.clone(),
        },
        WebRole::Image => Role::Image,
        WebRole::Separator => Role::Separator,
        WebRole::StaticText => Role::Text,
        WebRole::Text if editable => Role::Edit,
        WebRole::Button => Role::Button,
        // Chromium makes a button with any popup but a dialog a "button menu"; JAWS says
        // "button menu" only for a menu ("Pick one Button Has Popup listbox", dev box).
        WebRole::MenuButton if matches!(attr("haspopup"), None | Some("menu")) => Role::MenuButton,
        WebRole::MenuButton => Role::Button,
        WebRole::CheckBox => Role::CheckBox,
        WebRole::RadioButton => Role::RadioButton,
        WebRole::ComboBox => Role::ComboBox,
        WebRole::ProgressBar => Role::ProgressBar,
        WebRole::Slider => Role::Slider,
        WebRole::List => Role::List,
        WebRole::ListItem => Role::ListItem,
        WebRole::Tree => Role::Tree,
        WebRole::TreeItem => Role::TreeItem,
        WebRole::TabList => Role::TabList,
        WebRole::Tab => Role::Tab,
        WebRole::TabPanel => Role::TabPanel,
        WebRole::Table => Role::Table,
        WebRole::Row => Role::TableRow,
        WebRole::Cell | WebRole::ColumnHeader | WebRole::RowHeader => Role::TableCell,
        WebRole::Math => Role::Math,
        // A fieldset; its name is the legend.
        // An editing host (contenteditable without a role): editable and focusable.
        // Its text children are editable too, but not focusable.
        _ if editable && node.states.focusable => Role::Edit,
        WebRole::Grouping if named && is_grouping(node) => Role::Group,
        // Any other element reads by its CSS box: block boxes break lines, inline
        // elements (strong, em, time, label) flow with the surrounding text.
        _ if is_block_display(attr("display")) => Role::Group,
        _ => Role::Text,
    }
}

/// A page's elements as the engine's AOM.
///
/// Pure and platform-independent, with the AOM's conventions: text leaves carry `text`, inline
/// elements are `Text` nodes with children, articles and fieldsets are `Group` with an
/// `aria_role`, and header cells carry `tag`=`th` plus `scope`.
///
/// Chromium's object attributes carry what JAWS reads beyond a role: the HTML `tag` (block
/// quotes), the CSS `display` (block versus inline), visited links.
pub fn web_to_aom(root: &WebNode) -> AomNode {
    let mut next_id = 0;
    let mut by_id = HashMap::new();
    index_by_id(root, &mut by_id);
    convert(root, &mut next_id, &by_id)
}

/// The tree's nodes by their unique id, which relations name.
fn index_by_id<'a>(node: &'a WebNode, out: &mut HashMap<i32, &'a WebNode>) {
    if let Some(id) = node.unique_id {
        out.insert(id, node);
    }
    for child in &node.children {
        index_by_id(child, out);
    }
}

/// A related node's words: its name, or its text without embedded objects' marks.
fn related_text(node: &WebNode) -> Option<String> {
    let text = node
        .name
        .as_deref()
        .filter(|n| !n.trim().is_empty())
        .or(node.text.as_deref())?;
    let words = text.replace('\u{fffc}', " ");
    let words = words.split_whitespace().collect::<Vec<_>>().join(" ");
    (!words.is_empty()).then_some(words)
}

fn convert(node: &WebNode, next_id: &mut usize, by_id: &HashMap<i32, &WebNode>) -> AomNode {
    let id = *next_id;
    *next_id += 1;
    let role = map_role(node);
    let attr = |k: &str| node.attributes.get(k).cloned();
    let s = node.states;

    let mut aom = AomNode::new(id, role.clone());
    aom.name = node.name.clone().filter(|n| !n.is_empty());
    match role {
        Role::Text if node.role == WebRole::StaticText => {
            aom.text = node.name.clone();
        }
        Role::Text | Role::Group if is_labelled_empty_generic(node) => {
            aom.text = node.name.clone();
        }
        Role::Edit | Role::ComboBox | Role::Slider | Role::ProgressBar => {
            aom.value = node.value.clone();
            aom.value_min = node.value_min;
            aom.value_max = node.value_max;
        }
        // The document value is its URL; same-page links are resolved against it.
        Role::Document => {
            if let Some(url) = &node.value {
                aom.attributes.insert("url".to_owned(), url.clone());
            }
        }
        _ => {}
    }
    aom.level = attr("level").and_then(|l| l.parse().ok());
    aom.formatting = node.text_attributes.as_ref().and_then(text_formatting);
    aom.states = NodeStates {
        // A toggle button (`aria-pressed`) is checked when pressed.
        checked: if node.mark == Some(WebMark::ToggleButton) {
            Some(s.pressed)
        } else {
            matches!(role, Role::CheckBox | Role::RadioButton).then_some(s.checked)
        },
        // A tri-state check box between checked and not (`aria-checked="mixed"`).
        mixed: role == Role::CheckBox && s.mixed,
        selected: s.selected.then_some(true),
        expanded: if s.expanded {
            Some(true)
        } else if s.collapsed {
            Some(false)
        } else {
            None
        },
        focused: s.focused,
        disabled: s.unavailable,
        hidden: s.invisible,
        modal: s.modal,
        focusable: Some(s.focusable),
        pressed: false,
    };

    let mut set_attr = |key: &str, value: String| {
        aom.attributes.insert(key.to_owned(), value);
    };
    match (&role, node.role) {
        (Role::Group, WebRole::Document) => set_attr("aria_role", "article".to_owned()),
        // A fieldset: a named grouping. An unnamed one (a form, a section) keeps its xml-roles.
        // A feed is one too, which JAWS says as a feed ("Updates feed" [obs, 1172/s5]).
        (Role::Group, WebRole::Grouping)
            if aom.name.is_some()
                && is_grouping(node)
                && attr("xml-roles").as_deref() == Some("feed") =>
        {
            set_attr("aria_role", "feed".to_owned())
        }
        (Role::Group, WebRole::Grouping) if aom.name.is_some() && is_grouping(node) => {
            set_attr("aria_role", "group".to_owned())
        }
        (_, WebRole::ColumnHeader) => {
            set_attr("tag", "th".to_owned());
            set_attr("scope", "col".to_owned());
        }
        (_, WebRole::RowHeader) => {
            set_attr("tag", "th".to_owned());
            set_attr("scope", "row".to_owned());
        }
        _ => {
            if let Some(r) = attr("xml-roles") {
                set_attr("aria_role", r);
            }
        }
    }
    // An editing host (contenteditable): JAWS reads it in the
    // page as a section, "Notes field | Editable Section Start | Editable text | Editable
    // Section End" [obs, 1172/s23-s26].
    // A section (a div) made editable: not a text field, nor a spin box or a combo box's edit.
    if role == Role::Edit && node.mark == Some(WebMark::Section) {
        set_attr("editing_host", "true".to_owned());
    }
    // An ARIA radio group (`role="radiogroup"`), a group as a fieldset is, which JAWS says
    // its own way as the focus comes into it [obs, round-aria4, 2080/s1].
    if role == Role::Group && attr("xml-roles").as_deref() == Some("radiogroup") {
        set_attr("radiogroup", "true".to_owned());
    }
    // An ARIA group (`role="group"` on another element, not a fieldset), whose name a focus
    // coming into it does not say [obs, round-aria9, 2272/s1, 2286/s1].
    if role == Role::Group && attr("xml-roles").as_deref() == Some("group") {
        set_attr("aria_group", "true".to_owned());
    }
    // Struck, deleted, inserted and highlighted text (<s>, <del>, <ins>, <mark>), which JAWS
    // reads between "Deletion" and "end Deletion" and the like [obs, 814/s3, 819/s2, s4].
    match node.mark {
        Some(WebMark::ContentDeletion) => set_attr("aria_role", "deletion".to_owned()),
        Some(WebMark::ContentInsertion) => set_attr("aria_role", "insertion".to_owned()),
        Some(WebMark::Mark) => set_attr("aria_role", "mark".to_owned()),
        _ => {}
    }
    if s.required {
        set_attr("required", "true".to_owned());
    }
    // Chrome keeps the whitespace of preformatted elements in their text (tabs, runs of
    // spaces, line breaks), and so does the virtual buffer.
    if matches!(
        attr("tag").as_deref(),
        Some("pre" | "listing" | "xmp" | "plaintext")
    ) {
        set_attr("preformatted", "true".to_owned());
    }
    // Paragraphs and divisions are blocks like any other in the AOM; JAWS lists them apart
    // (Select a Paragraph, Select a Division) by their tag [obs, 239, 244]. A label's text is
    // a unit of its own to Smart Navigation ("City" [obs, 955/s2]).
    if let Some(tag) = attr("tag").filter(|t| matches!(t.as_str(), "p" | "div" | "label")) {
        set_attr("tag", tag);
    }
    // A read-only field (GitHub's code view is a read-only text area).
    if role == Role::Edit && s.readonly {
        set_attr("readonly", "true".to_owned());
    }
    if s.protected {
        set_attr("password", "true".to_owned());
    }
    // A combo box one types in (an autocomplete), JAWS's "edit combo". Chrome's UIA reports
    // a select's value as writable too, so only IA2's editable state tells them apart.
    // Clicks a table answers: a cell, row or table with a click listener of its own
    // (Chromium's default action "click") is clickable; a cell under a listener elsewhere
    // ("click ancestor") is clickable when its row or table is, which the reading decides.
    // JAWS says "clickable" in a table answering clicks itself, not in one under a listener
    // on the whole page, as GitHub's [obs, round-aria24, 2695/s2; 3154/s4, s1].
    if matches!(role, Role::TableCell | Role::TableRow | Role::Table) {
        match node.default_action.as_deref() {
            Some("click") => set_attr("clickable", "true".to_owned()),
            Some("click ancestor") if role == Role::TableCell => {
                set_attr("click_ancestor", "true".to_owned())
            }
            _ => {}
        }
    }
    // An ARIA meter, a progress bar to MSAA: "CPU Usage meter 36.1665".
    if role == Role::ProgressBar && attr("xml-roles").as_deref() == Some("meter") {
        set_attr("meter", "true".to_owned());
    }
    if role == Role::ComboBox && s.editable {
        set_attr("editable", "true".to_owned());
    }
    // One that completes what is typed (aria-autocomplete): "Supports AutoComplete" [obs,
    // round-aria32, 2925/s1].
    if role == Role::ComboBox && s.supports_autocompletion {
        set_attr("autocomplete", "true".to_owned());
    }
    if s.multi_line {
        set_attr("multiline", "true".to_owned());
    }
    if s.traversed {
        set_attr("visited", "true".to_owned());
    }
    // aria-describedby text, spoken after the value in focus phrasing.
    if let Some(d) = node.description.as_deref().filter(|d| !d.is_empty()) {
        set_attr("description", d.to_owned());
        // A description from the element's title (HTML's `title`), read in the page after its
        // first line: "Titled text Tooltip title" [obs, 1178/s12].
        if attr("description-from").as_deref() == Some("tooltip") {
            set_attr(okur_core::linearize::TITLE_ATTRIBUTE, d.to_owned());
        }
    }
    // A frame named by its title says it after its first line too: "First frame line News
    // frame" [obs, 1178/s4].
    if role == Role::IFrame && attr("name-from").as_deref() == Some("tooltip") {
        if let Some(name) = aom.name.clone().filter(|n| !n.is_empty()) {
            set_attr(okur_core::linearize::TITLE_ATTRIBUTE, name);
        }
    }
    if let Some(dom_id) = attr("id") {
        set_attr("id", dom_id);
    }
    // aria-current: "page", "step", "location", "date", "time" or "true".
    if let Some(current) = attr("current").filter(|c| c != "false") {
        set_attr("current", current);
    }
    // A menu item that opens a menu of its own: "File menu sub menu collapsed" [obs, 218/s1].
    if role == Role::MenuItem && attr("haspopup").is_some_and(|p| p != "false") {
        set_attr("submenu", "true".to_owned());
    }
    // A button's popup other than a menu ("listbox", "dialog"); a menu makes a menu button.
    if let Some(popup) =
        attr("haspopup").filter(|p| !matches!(p.as_str(), "false" | "true" | "menu"))
    {
        set_attr("haspopup", popup);
    }
    // A slider's orientation, which its role word says: "left right slider" [obs,
    // round-aria26, 2741/s1].
    if role == Role::Slider {
        if s.horizontal {
            set_attr("orientation", "horizontal".to_owned());
        } else if s.vertical {
            set_attr("orientation", "vertical".to_owned());
        }
    }
    // An ARIA spin button (role="spinbutton"), a text field JAWS calls an "edit spin box"
    // [obs, round-aria25, 2718/s1].
    if role == Role::Edit && attr("xml-roles").as_deref() == Some("spinbutton") {
        set_attr("spinbutton", "true".to_owned());
    }
    // A name given by aria-label (or title), which a link says in place of its text.
    if attr("name-from").as_deref() == Some("attribute") {
        set_attr("labelled", "true".to_owned());
    }
    // A table named from outside it (aria-labelledby, aria-label), not by a caption of its
    // own: JAWS reads no caption line for it [obs, round-aria24, 2695/s2].
    if role == Role::Table
        && matches!(
            attr("name-from").as_deref(),
            Some("related-element" | "attribute")
        )
    {
        set_attr("named_outside", "true".to_owned());
    }
    // aria-keyshortcuts, said after a link's or graphic's name ("Alt+ArrowUp").
    if let Some(keys) = attr("keyshortcuts").filter(|k| !k.trim().is_empty()) {
        set_attr("keyshortcuts", keys);
    }
    // aria-roledescription, said in place of some roles.
    if let Some(description) = attr("roledescription") {
        set_attr("roledescription", description);
    }
    // A field whose value is invalid (aria-invalid), and one with an error message
    // (aria-errormessage): "Adults edit spin box | Has Error | invalid entry | 9" [obs,
    // round-aria25, 2732/s1].
    if s.invalid_entry {
        set_attr("invalid", "true".to_owned());
    }
    if let Some(targets) = node.relations.get("error").filter(|t| !t.is_empty()) {
        set_attr("has_error", "true".to_owned());
        // The error message's words, "Must be between 1 and 8".
        let message: Vec<String> = targets
            .iter()
            .filter_map(|id| by_id.get(id).and_then(|target| related_text(target)))
            .collect();
        if !message.is_empty() {
            set_attr("error_message", message.join(" "));
        }
    }
    // Text that labels or describes a control (a label, an aria-describedby target): a
    // dialog's text leaves it out [obs, round-aria23, 2687/s1].
    if node
        .relations
        .get("labelFor")
        .is_some_and(|t| !t.is_empty())
        || node
            .relations
            .get("descriptionFor")
            .is_some_and(|t| !t.is_empty())
    {
        set_attr("labels_control", "true".to_owned());
    }
    // aria-flowto: the objects this one flows to, by their unique ids (Equals follows them).
    if let Some(targets) = node.relations.get("flowsTo").filter(|t| !t.is_empty()) {
        let ids: Vec<String> = targets.iter().map(i32::to_string).collect();
        set_attr("flows-to", ids.join(" "));
    }
    // The live object behind the node, for actions (focus, default action).
    if let Some(unique_id) = node.unique_id {
        set_attr(okur_core::aom::UNIQUE_ID_ATTR, unique_id.to_string());
    }
    if let (Some(pos), Some(size)) = (attr("posinset"), attr("setsize")) {
        if size != "0" {
            set_attr("posinset", pos);
            set_attr("setsize", size);
        }
    }

    aom.children = node
        .children
        .iter()
        .map(|c| convert(c, next_id, by_id))
        .collect();
    aom
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Struck-through, raised and lowered text, as Chromium marks its runs
    /// (`text-line-through-style:solid`, `text-position:super` or `sub`): what skim reading's
    /// Strikeout, Superscript and Subscript find.
    #[test]
    fn a_run_says_struck_raised_and_lowered_text() {
        let run = |pairs: &[(&str, &str)]| {
            let attributes = [("font-family", "Times New Roman"), ("font-size", "12pt")]
                .iter()
                .chain(pairs)
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect();
            text_formatting(&attributes).unwrap_or_default()
        };
        assert!(run(&[("text-line-through-style", "solid")]).strikeout);
        assert!(!run(&[("text-line-through-style", "none")]).strikeout);
        let raised = run(&[("text-position", "super")]);
        assert!(raised.superscript && !raised.subscript);
        let lowered = run(&[("text-position", "sub")]);
        assert!(lowered.subscript && !lowered.superscript);
        assert!(!run(&[("text-position", "baseline")]).superscript);
    }

    /// A generic container named by aria-label with nothing inside reads as its label; a
    /// named grouping that is not a section is a group.
    #[test]
    fn a_labelled_section_reads_as_text_and_a_fieldset_is_a_group() {
        let mut label = WebNode {
            role: WebRole::Grouping,
            mark: Some(WebMark::Section),
            name: Some("Siz:".to_owned()),
            ..WebNode::default()
        };
        label
            .attributes
            .insert("name-from".to_owned(), "attribute".to_owned());
        let aom = web_to_aom(&label);
        assert_eq!(aom.text.as_deref(), Some("Siz:"));
        let fieldset = WebNode {
            role: WebRole::Grouping,
            name: Some("Shipping".to_owned()),
            ..WebNode::default()
        };
        let aom = web_to_aom(&fieldset);
        assert_eq!(aom.role, Role::Group);
        assert_eq!(
            aom.attributes.get("aria_role").map(String::as_str),
            Some("group")
        );
    }
}
