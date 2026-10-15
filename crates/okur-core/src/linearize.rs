//! Linearization of the AOM into JAWS-style virtual buffer lines.
//!
//! Implements `docs/references/jaws-virtual-buffer-spec.md` (section numbers cited as §n):
//! the page title is line 0 (§1); containers produce start and end lines (§3); links,
//! graphics and form controls are lines of their own (§4, §7, §10); inline text flows
//! into one line per block (§4). Every line carries the exact speech JAWS produces when
//! the virtual cursor lands on it, so Down/Up/Say All speak `BufferLine::speech` verbatim.
//!
//! Content is rendered from text leaves, not from element names: an element's name is
//! used only for leaf controls or when it has no text content (§8, list markers come from
//! the marker text leaf).

use crate::aom::{AomNode, Role};
use crate::buffer::{
    BufferLine, BufferParagraph, BufferSpan, BufferWord, DialogInfo, TabInfo, TableCellInfo,
    TableGrid, VirtualBuffer,
};

/// Role marker brackets used by JAWS speech history (`\u{1d} role \u{1e}`).
const RS: char = '\u{1d}';
const RE: char = '\u{1e}';

pub(crate) fn role_marker(name: &str) -> String {
    format!(" {RS} {name} {RE} ")
}

pub(crate) fn state_marker(name: &str) -> String {
    format!(" {RS}{name}{RE} ")
}

/// A heading's only control, when it is a button named as the heading is: GitHub's "Collapse
/// file tree" heading is one button.
fn sole_button<'a>(heading: &'a AomNode, text: &str) -> Option<&'a AomNode> {
    fn controls<'a>(node: &'a AomNode, out: &mut Vec<&'a AomNode>) {
        for child in &node.children {
            if matches!(child.role, Role::Button | Role::MenuButton) {
                out.push(child);
            } else {
                controls(child, out);
            }
        }
    }
    let mut found = Vec::new();
    controls(heading, &mut found);
    match found.as_slice() {
        [only] if only.name.as_deref().map(squash).as_deref() == Some(text) => Some(*only),
        _ => None,
    }
}

/// What a heading that is a button says after its name: "Collapse file tree heading level 2
/// ‹ Button › ‹expanded›" (JAWS on the dev box, GitHub's code view).
pub(crate) fn button_suffix(button: &AomNode) -> String {
    let role = button.role_description().unwrap_or(match button.role {
        Role::MenuButton => "button menu",
        _ => "Button",
    });
    let state = match button.states.expanded {
        Some(true) => state_marker("expanded"),
        Some(false) => state_marker("collapsed"),
        None => String::new(),
    };
    format!("{}{state}", role_marker(role))
}

/// The longest a line of paragraph text may be before it wraps (§5). JAWS's default
/// `[HTML] MaxLineLength=150` in `Default.JCF`.
pub const MAX_LINE_LENGTH: usize = 150;

/// Splits paragraph text into JAWS lines (§5). While more than `max` characters remain,
/// a line ends at the first space at or after character `max`, so the word crossing the
/// limit is kept whole. Text without such a space stays on one line.
pub fn wrap_lines(text: &str, max: usize) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut rest = text;
    while let Some((limit, _)) = rest.char_indices().nth(max) {
        let Some(space) = rest.get(limit..).and_then(|tail| tail.find(' ')) else {
            break;
        };
        let (line, tail) = rest.split_at(limit + space);
        lines.push(line);
        rest = tail.trim_start();
    }
    if !rest.is_empty() {
        lines.push(rest);
    }
    lines
}

/// Collapses runs of whitespace to single spaces and trims.
fn squash(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Whether `text` is only a list marker such as `•`, `◦`, `▪` or `3.`.
fn is_list_marker(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    let mut chars = t.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => !c.is_alphanumeric(),
        _ => t
            .strip_suffix('.')
            .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())),
    }
}

/// Concatenated text content of a subtree (text leaves, or the name of a leaf without
/// text content).
fn text_content(node: &AomNode) -> String {
    let mut out = String::new();
    collect_text(node, &mut out);
    squash(&out)
}

fn collect_text(node: &AomNode, out: &mut String) {
    // `text` is a node's own text content, whatever its role.
    if let Some(t) = &node.text {
        out.push_str(t);
    }
    if node.children.is_empty() && node.text.is_none() {
        if let Some(n) = &node.name {
            out.push_str(n);
        }
    }
    for c in &node.children {
        collect_text(c, out);
    }
}

fn attr<'a>(node: &'a AomNode, key: &str) -> Option<&'a str> {
    node.attributes.get(key).map(String::as_str)
}

fn is_hidden(node: &AomNode) -> bool {
    node.states.hidden
        || attr(node, "aria-hidden") == Some("true")
        || node.attributes.contains_key("hidden")
}

/// How a node participates in linearization.
enum Class {
    /// Start/end lines around the children (§3).
    Container {
        start: String,
        end: String,
    },
    Heading(u32),
    Link,
    Graphic,
    /// A form control rendered as a single line; children are not rendered.
    Control,
    Table,
    List,
    ListItem,
    /// Line break before and after, no lines of its own.
    Block,
    /// Flows into the current line.
    Inline,
}

/// An in-progress line of inline text.
#[derive(Default)]
struct Run {
    text: String,
    node_id: Option<usize>,
    role: Option<Role>,
    /// The run is preformatted text (`<pre>`): its whitespace and line breaks are kept.
    preformatted: bool,
}

struct Builder<'a> {
    buffer: VirtualBuffer,
    run: Run,
    /// Index of the last emitted line, when it was a link line that may absorb punctuation
    /// immediately following the link (§4: "Maria Lopez,").
    absorb_punctuation_into: Option<usize>,
    /// Text of the most recent plain text line; a following edit/combo whose name equals it
    /// is labelled by it and omits its name (§10).
    last_text_line: Option<String>,
    list_depth: usize,
    /// How many preformatted elements (`<pre>`) the visit is inside.
    preformatted: usize,
    /// Enclosing named groups (fieldsets), innermost last: (node id, name).
    groups: Vec<(usize, String)>,
    /// Names of enclosing tab lists, innermost last.
    tablists: Vec<String>,
    document_url: Option<&'a str>,
    /// Per line: whether a paragraph ends after it (§25).
    paragraph_ends: Vec<bool>,
}

impl<'a> Builder<'a> {
    fn new(document_url: Option<&'a str>) -> Self {
        Self {
            buffer: VirtualBuffer::default(),
            run: Run::default(),
            absorb_punctuation_into: None,
            last_text_line: None,
            list_depth: 0,
            preformatted: 0,
            groups: Vec::new(),
            tablists: Vec::new(),
            document_url,
            paragraph_ends: Vec::new(),
        }
    }

    fn offset(&self) -> usize {
        self.buffer.text.len()
    }

    /// §25: a paragraph ends after the last line emitted since `first_line`, when a block
    /// element finishes. Inline content that does not finish its block (a link directly in
    /// a banner, a graphic directly in an article) continues into the next paragraph.
    fn end_paragraph_since(&mut self, first_line: usize) {
        if self.paragraph_ends.len() > first_line {
            if let Some(end) = self.paragraph_ends.last_mut() {
                *end = true;
            }
        }
    }

    /// Emits a line that is a paragraph of its own: the title and container boundaries.
    fn emit_boundary(&mut self, node_id: usize, role: Role, text: &str) -> usize {
        let first_line = self.paragraph_ends.len();
        let offset = self.emit(node_id, role, text, text.to_owned());
        self.end_paragraph_since(first_line);
        offset
    }

    /// Appends a line. `text` goes into the flat buffer (word/character navigation);
    /// `speech` is what the virtual cursor speaks on it. Returns the line's start offset.
    fn emit(&mut self, node_id: usize, role: Role, text: &str, speech: String) -> usize {
        if !self.buffer.text.is_empty() {
            self.buffer.text.push('\n');
        }
        let start = self.offset();
        self.buffer.text.push_str(text);
        let end = self.offset();
        self.buffer.lines.push(BufferLine {
            line_index: self.buffer.lines.len(),
            start_offset: start,
            end_offset: end,
            node_id,
            role: role.clone(),
            text: text.to_owned(),
            speech,
        });
        self.paragraph_ends.push(false);
        self.buffer.spans.push(BufferSpan {
            start_offset: start,
            end_offset: end,
            node_id,
            role,
        });
        self.absorb_punctuation_into = None;
        self.last_text_line = None;
        start
    }

    /// Emits the pending inline run, if it has any visible text. A flush is a block
    /// boundary, so punctuation after it never attaches to a preceding link line.
    fn flush(&mut self) {
        self.absorb_punctuation_into = None;
        let run = std::mem::take(&mut self.run);
        if run.preformatted {
            // Preformatted text keeps its tabs and runs of spaces, and its own line breaks
            // [obs, 207: a <pre> line reads "tab", "x", "space", "space", "equals"].
            let node_id = run.node_id.unwrap_or_default();
            let role = run.role.unwrap_or(Role::Text);
            for line in crate::editing::text_lines(&run.text) {
                if line.trim().is_empty() {
                    continue;
                }
                for part in wrap_lines(&line, MAX_LINE_LENGTH) {
                    self.emit(node_id, role.clone(), part, part.to_owned());
                }
            }
            self.last_text_line = Some(squash(&run.text));
            return;
        }
        let text = squash(&run.text);
        if text.is_empty() {
            return;
        }
        let node_id = run.node_id.unwrap_or_default();
        let role = run.role.unwrap_or(Role::Text);
        for line in wrap_lines(&text, MAX_LINE_LENGTH) {
            self.emit(node_id, role.clone(), line, line.to_owned());
        }
        self.last_text_line = Some(text);
    }

    fn push_inline_text(&mut self, node: &AomNode, text: &str) {
        let mut text = text;
        if self.run.text.trim().is_empty() {
            if let Some(idx) = self.absorb_punctuation_into.take() {
                let split = text
                    .find(|c: char| c.is_whitespace() || c.is_alphanumeric())
                    .unwrap_or(text.len());
                let (punct, rest) = text.split_at(split);
                if !punct.is_empty() {
                    self.append_to_line(idx, punct);
                }
                text = rest;
            }
        }
        if text.is_empty() {
            return;
        }
        if self.run.node_id.is_none() && !text.trim().is_empty() {
            self.run.node_id = Some(node.id);
            self.run.role = Some(node.role.clone());
        }
        if self.preformatted > 0 {
            self.run.preformatted = true;
        }
        self.run.text.push_str(text);
    }

    /// Appends `suffix` to the last line (only valid for the most recently emitted line).
    fn append_to_line(&mut self, idx: usize, suffix: &str) {
        let is_last = idx + 1 == self.buffer.lines.len();
        if !is_last {
            return;
        }
        self.buffer.text.push_str(suffix);
        let end = self.offset();
        if let Some(line) = self.buffer.lines.get_mut(idx) {
            line.text.push_str(suffix);
            line.speech.push_str(suffix);
            line.end_offset = end;
        }
        if let Some(s) = self.buffer.spans.last_mut() {
            s.end_offset = end;
        }
    }

    fn classify(&self, node: &AomNode, parent: Option<&AomNode>) -> Class {
        let aria = attr(node, "aria_role").unwrap_or_default();
        match &node.role {
            Role::Heading { level } => Class::Heading(*level),
            Role::Link { .. } => Class::Link,
            Role::Image => Class::Graphic,
            Role::Edit
            | Role::CheckBox
            | Role::RadioButton
            | Role::ComboBox
            | Role::Button
            | Role::MenuButton
            | Role::Slider
            | Role::ProgressBar
            | Role::Tab => Class::Control,
            Role::Table => Class::Table,
            Role::List => Class::List,
            Role::ListItem => Class::ListItem,
            Role::Landmark { landmark_type } => {
                let name = node.name.as_deref().map(squash).unwrap_or_default();
                let label = |kind: &str| {
                    if name.is_empty() {
                        kind.to_owned()
                    } else {
                        format!("{name} {kind}")
                    }
                };
                let start = match landmark_type.as_str() {
                    "banner" => "banner".to_owned(),
                    "contentinfo" => "content information".to_owned(),
                    "main" => "main region".to_owned(),
                    "navigation" => label("navigation region"),
                    "complementary" => label("complementary information"),
                    "search" => label("search region"),
                    "region" => label("region"),
                    other => label(other),
                };
                Class::Container {
                    end: format!("{start} end"),
                    start,
                }
            }
            Role::Dialog | Role::AlertDialog => {
                let name = node.name.as_deref().map(squash).unwrap_or_default();
                Class::Container {
                    start: format!("{name} dialog").trim().to_owned(),
                    end: "dialog end".to_owned(),
                }
            }
            Role::BlockQuote => Class::Container {
                start: "block quote".to_owned(),
                end: "block quote end".to_owned(),
            },
            Role::Group if aria == "article" => Class::Container {
                start: "article".to_owned(),
                end: "article end".to_owned(),
            },
            // A named group is a fieldset/radio group: its name is the legend (§3).
            Role::Group
                if aria == "group"
                    && node.name.as_deref().is_some_and(|n| !n.trim().is_empty()) =>
            {
                let name = node.name.as_deref().map(squash).unwrap_or_default();
                Class::Container {
                    start: format!("group start {name}"),
                    end: format!("group end {name}"),
                }
            }
            Role::Text if node.text.is_none() && !node.children.is_empty() => {
                // Inline wrappers Chrome exposes as Text (strong, em, time), except the
                // caption wrapper directly inside a table, which is its own line.
                if matches!(parent.map(|p| &p.role), Some(Role::Table)) {
                    Class::Block
                } else {
                    Class::Inline
                }
            }
            Role::Text | Role::Generic => Class::Inline,
            _ => Class::Block,
        }
    }

    /// Visits `node`; inside a preformatted element (`<pre>`) text keeps its whitespace.
    fn visit(&mut self, node: &AomNode, parent: Option<&AomNode>) {
        let preformatted = node.attributes.contains_key("preformatted");
        if preformatted {
            self.flush();
            self.preformatted += 1;
        }
        self.visit_node(node, parent);
        if preformatted {
            self.flush();
            self.preformatted -= 1;
        }
    }

    fn visit_node(&mut self, node: &AomNode, parent: Option<&AomNode>) {
        if is_hidden(node) {
            return;
        }
        self.buffer.mark_readable(node.id);
        let first_line = self.buffer.lines.len();

        match self.classify(node, parent) {
            Class::Inline => {
                self.push_own_text(node);
                self.visit_children(node);
            }
            Class::Block => {
                self.flush();
                self.push_own_text(node);
                let is_tablist = node.role == Role::TabList;
                if is_tablist {
                    self.tablists
                        .push(node.name.as_deref().map(squash).unwrap_or_default());
                }
                self.visit_children(node);
                if is_tablist {
                    self.tablists.pop();
                }
                self.flush();
                self.end_paragraph_since(first_line);
            }
            Class::Container { start, end } => {
                self.flush();
                let start_offset = self.emit_boundary(node.id, node.role.clone(), &start);
                match &node.role {
                    Role::Landmark { landmark_type } => {
                        self.buffer.landmarks.push(start_offset);
                        if landmark_type == "main" && self.buffer.main_region.is_none() {
                            self.buffer.main_region = Some(start_offset);
                        }
                    }
                    Role::Group if attr(node, "aria_role") == Some("article") => {
                        self.buffer.articles.push(start_offset);
                    }
                    _ => {}
                }
                let legend = node.name.as_deref().map(squash);
                let is_group = node.role == Role::Group;
                if is_group {
                    self.groups
                        .push((node.id, legend.clone().unwrap_or_default()));
                }
                for child in &node.children {
                    // The legend repeats the fieldset name and is not read again.
                    if node.role == Role::Group
                        && legend.as_deref().is_some_and(|l| text_content(child) == l)
                    {
                        continue;
                    }
                    self.visit(child, Some(node));
                }
                if is_group {
                    self.groups.pop();
                }
                self.flush();
                self.emit_boundary(node.id, node.role.clone(), &end);
                self.record_container(node, start_offset);
            }
            Class::Heading(level) => {
                self.flush();
                let text = match text_content(node) {
                    t if t.is_empty() => node.name.as_deref().map(squash).unwrap_or_default(),
                    t => t,
                };
                // A heading that is a single button (GitHub's "Collapse file tree") says the
                // button, and its line is the button's: Tab, B and Enter reach it.
                let button = sole_button(node, &text);
                let suffix = button.map(button_suffix).unwrap_or_default();
                let speech = format!(
                    "{}{text}{suffix}",
                    role_marker(&format!("heading level {level}"))
                );
                let line_node = button.map_or(node.id, |b| b.id);
                let off = self.emit(line_node, node.role.clone(), &text, speech);
                if let Some(button) = button {
                    let end = self.offset();
                    self.buffer.spans.push(BufferSpan {
                        start_offset: off,
                        end_offset: end,
                        node_id: button.id,
                        role: button.role.clone(),
                    });
                    self.buffer.mark_readable(button.id);
                    self.buffer.buttons.push(off);
                    self.buffer.form_controls.push(off);
                    self.buffer.tab_stops.push(off);
                }
                self.end_paragraph_since(first_line);
                self.buffer.headings.push((off, level));
            }
            Class::Link => self.visit_link(node),
            Class::Graphic => {
                self.flush();
                let alt = node.name.as_deref().map(squash).unwrap_or_default();
                // Its key shortcuts follow its name: "monatis Alt+ArrowUp | Graphic" (JAWS on
                // the dev box, GitHub's avatars).
                let named = match attr(node, "keyshortcuts") {
                    Some(keys) => format!("{alt} {keys}"),
                    None => alt.clone(),
                };
                let speech = format!("{named}\n{}", role_marker("Graphic"));
                let off = self.emit(node.id, node.role.clone(), &alt, speech);
                self.buffer.graphics.push(off);
            }
            Class::Control => self.visit_control(node),
            Class::Table => self.visit_table(node),
            Class::List => self.visit_list(node),
            Class::ListItem => {
                self.flush();
                self.push_own_text(node);
                self.visit_children(node);
                self.flush();
                self.end_paragraph_since(first_line);
                // §14: I and Shift+I stop only on items of lists that are not nested in
                // another list [obs, 108/s8, 114/s6–s7, s15–s19].
                let top_level = self.list_depth <= 1;
                if let Some(line) = self.buffer.lines.get(first_line).filter(|_| top_level) {
                    self.buffer.list_items.push(line.start_offset);
                }
            }
        }

        if let Some(id) = attr(node, "id") {
            if let Some(line) = self.buffer.lines.get(first_line) {
                self.buffer
                    .target_anchors
                    .insert(id.to_owned(), line.start_offset);
            }
        }
    }

    /// Flows a node's own `text` (its text content, for any role) into the current line.
    fn push_own_text(&mut self, node: &AomNode) {
        if let Some(t) = node.text.as_deref() {
            self.push_inline_text(node, t);
        }
    }

    fn visit_children(&mut self, node: &AomNode) {
        for child in &node.children {
            self.visit(child, Some(node));
        }
    }

    /// A link to a place in this document: a fragment, alone or after this document's
    /// address. An empty fragment (`href="#"`, a link a script handles) is a plain "Link".
    fn is_same_page(&self, href: Option<&str>) -> bool {
        let Some((base, fragment)) = href.and_then(|h| h.split_once('#')) else {
            return false;
        };
        !fragment.is_empty()
            && (base.is_empty()
                || self.document_url.and_then(|d| d.split('#').next()) == Some(base))
    }

    /// §4: a link is its own line. Text already on the line is flushed first, except a
    /// list marker, which prefixes the link line ("•  ‹ same page link › News").
    fn visit_link(&mut self, node: &AomNode) {
        let prefix = if is_list_marker(&self.run.text) {
            squash(&std::mem::take(&mut self.run).text)
        } else {
            self.flush();
            String::new()
        };
        // A link named by aria-label says that name, not its text, and then its key
        // shortcuts: "Link commits by monatis Alt+ArrowUp" (JAWS on the dev box, GitHub).
        let label = node
            .name
            .as_deref()
            .map(squash)
            .filter(|n| !n.is_empty() && attr(node, "labelled") == Some("true"));
        let mut text = match label {
            Some(label) => label,
            None => match text_content(node) {
                t if t.is_empty() => node.name.as_deref().map(squash).unwrap_or_default(),
                t => t,
            },
        };
        if let Some(keys) = attr(node, "keyshortcuts") {
            text = format!("{text} {keys}");
        }
        let href = match &node.role {
            Role::Link { href } => href.as_deref(),
            _ => None,
        };
        let same_page = self.is_same_page(href);
        // aria-roledescription takes the role's place in reading ("‹ chip › Rust").
        let kind = match node.role_description() {
            Some(description) => description,
            None if same_page => "same page link",
            None => "Link",
        };
        if same_page {
            if let Some(n) = self.buffer.node_mut(node.id) {
                n.attributes
                    .insert("same_page".to_owned(), "true".to_owned());
            }
        }
        // aria-current follows the role: "‹ Link › ‹Current Page› README".
        let current = node.current_phrase().map(state_marker).unwrap_or_default();
        let speech = format!("{prefix}{}{current}{text}", role_marker(kind));
        let line_text = squash(&format!("{prefix} {text}"));
        let off = self.emit(node.id, node.role.clone(), &line_text, speech);
        self.buffer.links.push(off);
        self.buffer.tab_stops.push(off);
        self.absorb_punctuation_into = Some(self.buffer.lines.len() - 1);
    }

    /// §10: one line per form control, in JAWS's line-reading phrasing.
    fn visit_control(&mut self, node: &AomNode) {
        // Radio buttons remember their enclosing group, which focus speech names when the
        // cursor enters the group (§17).
        if node.role == Role::RadioButton {
            if let Some((group_id, group_name)) = self.groups.last() {
                let (group_id, group_name) = (group_id.to_string(), group_name.clone());
                self.buffer.mark_readable(node.id);
                if let Some(annotated) = self.buffer.node_mut(node.id) {
                    annotated.attributes.insert("group_id".to_owned(), group_id);
                    annotated
                        .attributes
                        .insert("group_name".to_owned(), group_name);
                }
            }
        }
        // Flush first: an inline label in the same block is still pending in the run.
        self.flush();
        let labelled_by_previous_line = self.last_text_line.clone();
        let labelled = labelled_by_previous_line.is_some()
            && labelled_by_previous_line.as_deref() == node.name.as_deref().map(squash).as_deref();
        let name = node.name.as_deref().map(squash).unwrap_or_default();
        let required = attr(node, "required") == Some("true");
        let checked = |c: Option<bool>| {
            if c == Some(true) {
                "checked"
            } else {
                "not checked"
            }
        };

        let speech = match &node.role {
            Role::Edit => {
                let kind = if attr(node, "password") == Some("true") {
                    "password edit"
                } else {
                    node.role_description().unwrap_or("Edit")
                };
                let mut s = String::new();
                if !labelled {
                    s.push_str(&name);
                }
                if required {
                    s.push_str(&state_marker("Required"));
                }
                s.push_str(&role_marker(kind));
                if let Some(v) = node.value.as_deref().filter(|v| !v.is_empty()) {
                    s.push_str(v);
                }
                s
            }
            Role::ComboBox => {
                let state = if node.states.expanded == Some(true) {
                    "expanded"
                } else {
                    "collapsed"
                };
                let mut s = String::new();
                if !labelled {
                    s.push_str(&name);
                }
                // One typed in is an "edit combo" (JAWS on the dev box).
                let kind = if attr(node, "editable") == Some("true") {
                    "edit combo"
                } else {
                    "Combo box"
                };
                s.push_str(&role_marker(kind));
                s.push_str(&state_marker(state));
                s.push_str(node.value.as_deref().unwrap_or_default());
                s
            }
            Role::CheckBox => format!(
                "{name}{}{}",
                role_marker(node.role_description().unwrap_or("check box")),
                state_marker(checked(node.states.checked))
            ),
            Role::RadioButton => {
                let mut s = format!(
                    "{name}{}{}",
                    role_marker("radio button"),
                    state_marker(checked(node.states.checked))
                );
                if let (Some(pos), Some(size)) = (attr(node, "posinset"), attr(node, "setsize")) {
                    s.push_str(&format!("\n{pos} of {size}"));
                }
                s
            }
            // "Platform ‹ Button › ‹collapsed›", "Overview ‹ Button › ‹Current Page›".
            Role::Button => {
                let role = node.role_description().unwrap_or("Button");
                let mut s = format!("{name}{}", role_marker(role));
                match node.states.expanded {
                    Some(true) => s.push_str(&state_marker("expanded")),
                    Some(false) => s.push_str(&state_marker("collapsed")),
                    None => {}
                }
                // Any other popup: "Pick one ‹ Button › ‹Has Popup listbox›" (dev box).
                if let Some(popup) = attr(node, "haspopup") {
                    s.push_str(&state_marker(&format!("Has Popup {popup}")));
                }
                if let Some(current) = node.current_phrase() {
                    s.push_str(&state_marker(current));
                }
                if node.states.disabled {
                    s.push_str(&state_marker("Unavailable"));
                }
                s
            }
            // A button opening a menu: "Actions ‹ button menu › ‹collapsed›" (JAWS on the dev box).
            Role::MenuButton => {
                let role = node.role_description().unwrap_or("button menu");
                let mut s = format!("{name}{}", role_marker(role));
                match node.states.expanded {
                    Some(true) => s.push_str(&state_marker("expanded")),
                    Some(false) => s.push_str(&state_marker("collapsed")),
                    None => {}
                }
                s
            }
            Role::Tab => format!("{name}{}", role_marker("Tab")),
            Role::Slider => format!("{name}{}", role_marker("slider")),
            Role::ProgressBar => format!("{name}{}", role_marker("progress bar")),
            _ => name.clone(),
        };
        let text = squash(&speech.replace([RS, RE], " "));
        let off = self.emit(node.id, node.role.clone(), &text, speech);

        match &node.role {
            Role::Button | Role::MenuButton if node.states.disabled => {}
            Role::Button | Role::MenuButton => {
                self.buffer.buttons.push(off);
                self.buffer.form_controls.push(off);
            }
            Role::Edit => {
                self.buffer.edits.push(off);
                self.buffer.form_controls.push(off);
            }
            Role::CheckBox => {
                self.buffer.checkboxes.push(off);
                self.buffer.form_controls.push(off);
            }
            Role::RadioButton => {
                self.buffer.radio_buttons.push(off);
                self.buffer.form_controls.push(off);
            }
            Role::ComboBox => {
                self.buffer.comboboxes.push(off);
                self.buffer.form_controls.push(off);
            }
            Role::Slider => self.buffer.sliders.push(off),
            Role::ProgressBar => self.buffer.progress_bars.push(off),
            Role::Tab => {
                let position = |key: &str| attr(node, key).and_then(|v| v.parse::<usize>().ok());
                let index = position("posinset").unwrap_or(self.buffer.tabs.len() + 1);
                self.buffer.tabs.push(TabInfo {
                    node_id: node.id,
                    offset: off,
                    index,
                    total: position("setsize").unwrap_or(index),
                    group_name: self.tablists.last().filter(|n| !n.is_empty()).cloned(),
                });
            }
            _ => {}
        }
        if !node.states.disabled && node.role != Role::ProgressBar {
            self.buffer.tab_stops.push(off);
        }
    }

    /// §3/§8: "list of N items" … "list end", with "nesting level K" for nested lists.
    fn visit_list(&mut self, node: &AomNode) {
        self.flush();
        let items = node
            .children
            .iter()
            .filter(|c| c.role == Role::ListItem)
            .count();
        let nesting = if self.list_depth > 0 {
            format!(" nesting level {}", self.list_depth)
        } else {
            String::new()
        };
        let start = format!("list of {items} items{nesting}");
        let start_offset = self.emit_boundary(node.id, node.role.clone(), &start);
        self.buffer.lists.push(start_offset);
        self.list_depth += 1;
        self.visit_children(node);
        self.list_depth -= 1;
        self.flush();
        let end = format!("list end{nesting}");
        self.emit_boundary(node.id, node.role.clone(), &end);
        self.record_container(node, start_offset);
    }

    /// §9: "table with C columns and R rows", caption, one line per cell, "table end".
    fn visit_table(&mut self, node: &AomNode) {
        self.flush();
        let rows: Vec<&AomNode> = node
            .children
            .iter()
            .filter(|c| c.role == Role::TableRow)
            .collect();
        let cols = rows
            .iter()
            .map(|r| {
                r.children
                    .iter()
                    .filter(|c| c.role == Role::TableCell)
                    .count()
            })
            .max()
            .unwrap_or(0);
        let start = format!("table with {cols} columns and {} rows", rows.len());
        let start_offset = self.emit_boundary(node.id, node.role.clone(), &start);

        let mut caption = None;
        // UIA names a table by its caption; without a caption element, the name is the caption.
        if node.children.iter().all(|c| c.role == Role::TableRow) {
            if let Some(name) = node.name.as_deref().map(squash).filter(|n| !n.is_empty()) {
                self.emit_boundary(node.id, Role::Text, &name);
                caption = Some(name);
            }
        }
        let mut grid: Vec<Vec<TableCellInfo>> = Vec::new();
        for child in &node.children {
            if child.role != Role::TableRow {
                let before = self.buffer.lines.len();
                self.visit(child, Some(node));
                if caption.is_none() {
                    caption = self.buffer.lines.get(before).map(|l| l.text.clone());
                }
                continue;
            }
            self.buffer.mark_readable(child.id);
            let row_index = grid.len() + 1;
            let mut row_cells = Vec::new();
            for cell in child.children.iter().filter(|c| c.role == Role::TableCell) {
                self.flush();
                let before = self.buffer.lines.len();
                self.buffer.mark_readable(cell.id);
                self.push_own_text(cell);
                for c in &cell.children {
                    self.visit(c, Some(cell));
                }
                self.flush();
                let offset = match self.buffer.lines.get(before) {
                    Some(line) => line.start_offset,
                    None => self.emit(cell.id, Role::TableCell, "", String::new()),
                };
                self.end_paragraph_since(before);
                let text = self
                    .buffer
                    .lines
                    .get(before..)
                    .unwrap_or_default()
                    .iter()
                    .map(|l| l.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                let speech = self
                    .buffer
                    .lines
                    .get(before..)
                    .unwrap_or_default()
                    .iter()
                    .map(|l| l.speech.as_str())
                    .collect::<Vec<_>>()
                    .join(
                        "
",
                    );
                self.buffer.spans.push(BufferSpan {
                    start_offset: offset,
                    end_offset: self.offset(),
                    node_id: cell.id,
                    role: Role::TableCell,
                });
                let scope = attr(cell, "scope");
                row_cells.push(TableCellInfo {
                    node_id: cell.id,
                    offset,
                    row: row_index,
                    col: row_cells.len() + 1,
                    row_span: 1,
                    col_span: 1,
                    is_header: attr(cell, "tag") == Some("th"),
                    is_col_header: scope == Some("col"),
                    is_row_header: scope == Some("row"),
                    col_header: None,
                    row_header: None,
                    text,
                    speech,
                });
            }
            grid.push(row_cells);
        }
        // Resolve each data cell's column and row headers.
        let col_headers: Vec<Option<String>> = (0..cols)
            .map(|c| {
                grid.iter()
                    .filter_map(|r| r.get(c))
                    .find(|cell| cell.is_col_header)
                    .map(|cell| cell.text.clone())
            })
            .collect();
        for row in &mut grid {
            let row_header = row
                .iter()
                .find(|cell| cell.is_row_header)
                .map(|cell| cell.text.clone());
            for (c, cell) in row.iter_mut().enumerate() {
                if !cell.is_col_header {
                    cell.col_header = col_headers.get(c).cloned().flatten();
                }
                if !cell.is_row_header {
                    cell.row_header = row_header.clone();
                }
            }
        }

        self.flush();
        self.emit_boundary(node.id, node.role.clone(), "table end");
        let end_offset = self.offset();
        self.buffer.table_offsets.push(start_offset);
        self.buffer.tables.push(TableGrid {
            table_id: node.id,
            rows: grid.len(),
            cols,
            caption,
            start_offset,
            end_offset,
            cells: grid,
        });
        self.buffer.spans.push(BufferSpan {
            start_offset,
            end_offset,
            node_id: node.id,
            role: Role::Table,
        });
    }

    /// Records a container span (and dialog range) from its start line to the current end.
    fn record_container(&mut self, node: &AomNode, start_offset: usize) {
        let end_offset = self.offset();
        self.buffer.spans.push(BufferSpan {
            start_offset,
            end_offset,
            node_id: node.id,
            role: node.role.clone(),
        });
        if matches!(node.role, Role::Dialog | Role::AlertDialog) {
            self.buffer.dialogs.push(DialogInfo {
                node_id: node.id,
                name: node.name.clone().unwrap_or_default(),
                is_modal: node.states.modal,
                start_offset,
                end_offset,
            });
        }
    }
}

/// §23: JAWS word boundaries within one line of text.
///
/// Whitespace separates words. Within a whitespace-delimited token, `.` and `-` are words of
/// their own unless they sit between two digits (`$5.99`, `3.5%`, `555-0142`); other
/// punctuation (`,`, `;`, `$`, `%`, `@`, `'`) stays attached to the adjacent word.
pub fn segment_words(text: &str) -> Vec<(usize, usize)> {
    let mut words = Vec::new();
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut start: Option<usize> = None;
    for (i, &(offset, c)) in chars.iter().enumerate() {
        let prev = i.checked_sub(1).and_then(|p| chars.get(p)).map(|&(_, c)| c);
        let next = chars.get(i + 1).map(|&(_, c)| c);
        if c.is_whitespace() {
            if let Some(s) = start.take() {
                words.push((s, offset));
            }
            continue;
        }
        let separator = (c == '.' || c == '-')
            && !(prev.is_some_and(|p| p.is_ascii_digit())
                && next.is_some_and(|n| n.is_ascii_digit()));
        if separator {
            if let Some(s) = start.take() {
                words.push((s, offset));
            }
            words.push((offset, offset + c.len_utf8()));
        } else if start.is_none() {
            start = Some(offset);
        }
    }
    if let Some(s) = start {
        words.push((s, text.len()));
    }
    words
}

/// §25: groups consecutive lines into paragraphs, each ending after a marked line.
fn paragraphs(lines: &[BufferLine], ends: &[bool]) -> Vec<BufferParagraph> {
    let mut out: Vec<BufferParagraph> = Vec::new();
    let mut open = false;
    for (line, &end) in lines.iter().zip(ends) {
        match out.last_mut().filter(|_| open) {
            Some(p) => {
                p.end_offset = line.end_offset;
                p.text.push(' ');
                p.text.push_str(&line.text);
                p.speech.push(' ');
                p.speech.push_str(&line.speech);
            }
            None => out.push(BufferParagraph {
                paragraph_index: out.len(),
                start_offset: line.start_offset,
                end_offset: line.end_offset,
                node_id: line.node_id,
                role: line.role.clone(),
                text: line.text.clone(),
                speech: line.speech.clone(),
            }),
        }
        open = !end;
    }
    out
}

/// Builds the virtual buffer for `root` (a document, or any subtree).
pub fn build(root: &AomNode) -> VirtualBuffer {
    let document_url = attr(root, "url");
    let mut builder = Builder::new(document_url);
    builder.buffer.set_page(root.clone());
    builder.buffer.mark_readable(root.id);

    // §1: line 0 is the page title.
    if root.role == Role::Document {
        let title = root.name.as_deref().map(squash).unwrap_or_default();
        if !title.is_empty() {
            builder.emit_boundary(root.id, Role::Document, &title);
        }
        builder.visit_children(root);
    } else {
        builder.visit(root, None);
    }
    builder.flush();
    builder.end_paragraph_since(0);

    let mut buffer = builder.buffer;
    buffer.paragraphs = paragraphs(&buffer.lines, &builder.paragraph_ends);
    let mut words = Vec::new();
    for line in &buffer.lines {
        for (s, e) in segment_words(&line.text) {
            let text = line.text.get(s..e).unwrap_or_default().to_owned();
            words.push(BufferWord {
                word_index: words.len(),
                start_offset: line.start_offset + s,
                end_offset: line.start_offset + e,
                text,
            });
        }
    }
    buffer.words = words;
    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<&str> {
        segment_words(text)
            .into_iter()
            .filter_map(|(s, e)| text.get(s..e))
            .collect()
    }

    /// Recorded JAWS behavior, corpus/evidence/106-article-words (Ctrl+Right).
    #[test]
    fn word_segmentation_matches_recorded_jaws() {
        assert_eq!(
            words("costs $5.99 a month, e.g. for adults; children join free."),
            vec![
                "costs", "$5.99", "a", "month,", "e", ".", "g", ".", "for", "adults;", "children",
                "join", "free", "."
            ]
        );
        assert_eq!(
            words("help@riverside-library.org — or call 555-0142. Attendance rose 3.5%"),
            vec![
                "help@riverside",
                "-",
                "library",
                ".",
                "org",
                "—",
                "or",
                "call",
                "555-0142",
                ".",
                "Attendance",
                "rose",
                "3.5%"
            ]
        );
    }

    /// Line breaks recorded from JAWS on `news-article.html` [obs, 101/s18, s22, s52–s53].
    #[test]
    fn long_text_wraps_after_the_word_crossing_the_limit() {
        let p = "The library board will review attendance in the fall. A long-standing \
                 complaint among students and working parents was that the reading rooms \
                 closed before most people finished their day, so the extended schedule is \
                 intended to make the collections, the public computers, and the quiet \
                 study spaces available at the times when the community actually needs \
                 them, according to the library director.";
        let lines = wrap_lines(p, MAX_LINE_LENGTH);
        let lengths: Vec<usize> = lines.iter().map(|l| l.chars().count()).collect();
        assert_eq!(lengths, vec![155, 152, 87]);
        assert!(lines.first().is_some_and(|l| l.ends_with("closed before")));
        assert!(lines.get(1).is_some_and(|l| l.starts_with("most people")));

        let p = "Membership costs $5.99 a month, e.g. for adults; children join free. \
                 Questions go to help@riverside-library.org — or call 555-0142. Attendance \
                 rose 3.5% last year, and staff say they can't wait.";
        let lines = wrap_lines(p, MAX_LINE_LENGTH);
        assert_eq!(lines.first().map(|l| l.chars().count()), Some(152));
        assert_eq!(
            lines.get(1).copied(),
            Some("last year, and staff say they can't wait.")
        );
    }

    #[test]
    fn short_or_unbreakable_text_stays_on_one_line() {
        assert_eq!(wrap_lines("short", MAX_LINE_LENGTH), vec!["short"]);
        let word = "x".repeat(200);
        assert_eq!(wrap_lines(&word, MAX_LINE_LENGTH), vec![word.as_str()]);
        let exact = "a ".repeat(75);
        assert_eq!(wrap_lines(exact.trim_end(), MAX_LINE_LENGTH).len(), 1);
    }

    #[test]
    fn list_markers() {
        assert!(is_list_marker("• "));
        assert!(is_list_marker("◦"));
        assert!(is_list_marker("12."));
        assert!(!is_list_marker("New"));
        assert!(!is_list_marker(""));
    }

    fn link(id: usize, href: &str, name: &str, current: Option<&str>) -> AomNode {
        let mut node = AomNode::new(
            id,
            Role::Link {
                href: Some(href.to_owned()),
            },
        )
        .with_name(name);
        if let Some(current) = current {
            node.attributes
                .insert("current".to_owned(), current.to_owned());
        }
        node
    }

    fn github_tabs() -> crate::buffer::VirtualBuffer {
        let mut root = AomNode::new(0, Role::Document).with_children(vec![
            link(
                1,
                "https://github.com/monatis/okur#",
                "README",
                Some("page"),
            ),
            link(2, "https://github.com/monatis/okur#start", "Skip", None),
            link(3, "#b", "Step two", Some("step")),
            link(4, "https://example.com/", "Today", Some("true")),
        ]);
        root.attributes.insert(
            "url".to_owned(),
            "https://github.com/monatis/okur".to_owned(),
        );
        crate::buffer::VirtualBuffer::from_aom(&root)
    }

    /// JAWS on Chrome, observed on the dev box (GitHub's README tab and a test page,
    /// 2026-10-16): aria-current is said after the link's role, and a link to `#` alone is a
    /// plain "Link", not a same page link.
    #[test]
    fn a_current_link_says_so_after_its_role() {
        let buffer = github_tabs();
        let speech: Vec<&str> = buffer.lines.iter().map(|l| l.speech.as_str()).collect();
        let (link, same_page) = (role_marker("Link"), role_marker("same page link"));
        assert_eq!(
            speech,
            vec![
                format!("{link}{}README", state_marker("Current Page")),
                format!("{same_page}Skip"),
                format!("{same_page}{}Step two", state_marker("Current Step")),
                format!("{link}{}Today", state_marker("Current")),
            ]
        );
    }

    /// Tab to a current link (dev box, as above): a same page link says the state after its
    /// role, a plain link before it.
    #[test]
    fn a_focused_current_link_says_so() {
        let mut cursor = crate::cursor::VirtualCursor::new(github_tabs());
        assert_eq!(
            cursor.focus_node(1).as_deref(),
            Some("README \u{1d}Current Page\u{1e}   \u{1d} Link \u{1e} \nTo activate press Enter.")
        );
        assert_eq!(
            cursor.focus_node(3).as_deref(),
            Some(
                "Step two  \u{1d} same page link \u{1e}  \u{1d}Current Step\u{1e} \nTo activate press Enter."
            )
        );
    }

    /// Tab into a text area with text (JAWS on the dev box, 2026-10-16: "Notes edit |
    /// Contains text | Type in text."): its text is not read, a one-line field's is.
    #[test]
    fn a_focused_text_area_says_it_contains_text() {
        let mut notes = AomNode::new(1, Role::Edit)
            .with_name("Notes")
            .with_value("1. The quick brown fox\n2. jumps over the lazy dog");
        notes
            .attributes
            .insert("multiline".to_owned(), "true".to_owned());
        let name = AomNode::new(2, Role::Edit)
            .with_name("Name")
            .with_value("Ada");
        let root = AomNode::new(0, Role::Document).with_children(vec![notes, name]);
        let mut cursor =
            crate::cursor::VirtualCursor::new(crate::buffer::VirtualBuffer::from_aom(&root));
        assert_eq!(
            cursor.focus_node(1).as_deref(),
            Some("Notes \u{1d} edit \u{1e} \nContains text")
        );
        assert_eq!(
            cursor.focus_node(2).as_deref(),
            Some("Name \u{1d} Edit \u{1e} \nAda")
        );
    }

    /// aria-roledescription on the dev box (JAWS, 2026-10-16): said in place of a button's,
    /// link's, check box's and edit's role in reading; on focus, a link keeps its own role.
    #[test]
    fn a_role_description_takes_the_roles_place() {
        let described = |node: AomNode, description: &str| {
            let mut node = node;
            node.attributes
                .insert("roledescription".to_owned(), description.to_owned());
            node
        };
        let root = AomNode::new(0, Role::Document).with_children(vec![
            described(
                AomNode::new(1, Role::Button).with_name("Next slide"),
                "slide control",
            ),
            described(link(2, "#x", "Rust", None), "chip"),
            described(AomNode::new(3, Role::CheckBox).with_name("Wrap"), "toggle"),
        ]);
        let buffer = crate::buffer::VirtualBuffer::from_aom(&root);
        let speech: Vec<&str> = buffer.lines.iter().map(|l| l.speech.as_str()).collect();
        assert_eq!(
            speech,
            vec![
                format!("Next slide{}", role_marker("slide control")),
                format!("{}Rust", role_marker("chip")),
                format!(
                    "Wrap{}{}",
                    role_marker("toggle"),
                    state_marker("not checked")
                ),
            ]
        );
        let mut cursor = crate::cursor::VirtualCursor::new(buffer);
        assert!(cursor
            .focus_node(1)
            .is_some_and(|s| s.starts_with("Next slide  \u{1d} slide control \u{1e}")));
        assert!(cursor
            .focus_node(2)
            .is_some_and(|s| s.starts_with("Rust  \u{1d} same page link \u{1e}")));
    }

    /// Buttons with popups, JAWS on the dev box (2026-10-16): a menu's says "button menu" and,
    /// in reading, its state; any other popup is a Button that "Has Popup".
    #[test]
    fn a_menu_button_and_a_popup_button() {
        let mut actions = AomNode::new(1, Role::MenuButton).with_name("Actions");
        actions.states.expanded = Some(false);
        let mut pick = AomNode::new(2, Role::Button).with_name("Pick one");
        pick.attributes
            .insert("haspopup".to_owned(), "listbox".to_owned());
        let root = AomNode::new(0, Role::Document).with_children(vec![actions, pick]);
        let buffer = crate::buffer::VirtualBuffer::from_aom(&root);
        let speech: Vec<&str> = buffer.lines.iter().map(|l| l.speech.as_str()).collect();
        assert_eq!(
            speech,
            vec![
                format!(
                    "Actions{}{}",
                    role_marker("button menu"),
                    state_marker("collapsed")
                ),
                format!(
                    "Pick one{}{}",
                    role_marker("Button"),
                    state_marker("Has Popup listbox")
                ),
            ]
        );
        assert_eq!(buffer.tab_stops.len(), 2);
        let mut cursor = crate::cursor::VirtualCursor::new(buffer);
        assert_eq!(
            cursor.focus_node(1).as_deref(),
            Some("Actions  \u{1d} button menu \u{1e} ")
        );
    }

    /// GitHub's code view, JAWS on the dev box (2026-10-16): a link named by aria-label says
    /// that name and its key shortcuts; an avatar says its shortcuts; a heading that is a
    /// button says the button, and its line is the button's.
    #[test]
    fn labels_shortcuts_and_a_heading_that_is_a_button() {
        let mut commits = link(1, "https://github.com/x", "commits by monatis", None);
        commits.text = Some("monatis".to_owned());
        commits
            .attributes
            .insert("labelled".to_owned(), "true".to_owned());
        commits
            .attributes
            .insert("keyshortcuts".to_owned(), "Alt+ArrowUp".to_owned());
        let mut avatar = AomNode::new(2, Role::Image).with_name("monatis");
        avatar
            .attributes
            .insert("keyshortcuts".to_owned(), "Alt+ArrowUp".to_owned());
        let mut button = AomNode::new(4, Role::Button).with_name("Collapse file tree");
        button.states.expanded = Some(true);
        let heading = AomNode::new(3, Role::Heading { level: 2 })
            .with_name("Collapse file tree")
            .with_children(vec![button]);
        let root = AomNode::new(0, Role::Document).with_children(vec![commits, avatar, heading]);
        let buffer = crate::buffer::VirtualBuffer::from_aom(&root);
        let speech: Vec<&str> = buffer.lines.iter().map(|l| l.speech.as_str()).collect();
        assert_eq!(
            speech,
            vec![
                format!("{}commits by monatis Alt+ArrowUp", role_marker("Link")),
                format!("monatis Alt+ArrowUp\n{}", role_marker("Graphic")),
                format!(
                    "{}Collapse file tree{}{}",
                    role_marker("heading level 2"),
                    role_marker("Button"),
                    state_marker("expanded")
                ),
            ]
        );
        let heading_line = buffer.lines.get(2).map(|l| (l.node_id, l.start_offset));
        assert_eq!(heading_line.map(|(node, _)| node), Some(4));
        assert!(heading_line.is_some_and(|(_, off)| buffer.tab_stops.contains(&off)));
    }
}
