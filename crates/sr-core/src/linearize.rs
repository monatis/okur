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

fn role_marker(name: &str) -> String {
    format!(" {RS} {name} {RE} ")
}

fn state_marker(name: &str) -> String {
    format!(" {RS}{name}{RE} ")
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
    /// Enclosing named groups (fieldsets), innermost last: (node id, name).
    groups: Vec<(usize, String)>,
    /// Names of enclosing tab lists, innermost last.
    tablists: Vec<String>,
    document_url: Option<&'a str>,
}

impl<'a> Builder<'a> {
    fn new(document_url: Option<&'a str>) -> Self {
        Self {
            buffer: VirtualBuffer::default(),
            run: Run::default(),
            absorb_punctuation_into: None,
            last_text_line: None,
            list_depth: 0,
            groups: Vec::new(),
            tablists: Vec::new(),
            document_url,
        }
    }

    fn offset(&self) -> usize {
        self.buffer.text.len()
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
            speech: speech.clone(),
        });
        self.buffer.paragraphs.push(BufferParagraph {
            paragraph_index: self.buffer.paragraphs.len(),
            start_offset: start,
            end_offset: end,
            node_id,
            role: role.clone(),
            text: text.to_owned(),
        });
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
        let text = squash(&run.text);
        if text.is_empty() {
            return;
        }
        let node_id = run.node_id.unwrap_or_default();
        let role = run.role.unwrap_or(Role::Text);
        let lines = wrap_lines(&text, MAX_LINE_LENGTH);
        let first_paragraph = self.buffer.paragraphs.len();
        for line in &lines {
            self.emit(node_id, role.clone(), line, (*line).to_owned());
        }
        // A wrapped run is still one paragraph.
        if lines.len() > 1 {
            let end = self.offset();
            self.buffer.paragraphs.truncate(first_paragraph + 1);
            if let Some(p) = self.buffer.paragraphs.last_mut() {
                p.end_offset = end;
                p.text = text.clone();
            }
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
        if let Some(p) = self.buffer.paragraphs.last_mut() {
            p.text.push_str(suffix);
            p.end_offset = end;
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

    fn visit(&mut self, node: &AomNode, parent: Option<&AomNode>) {
        if is_hidden(node) {
            return;
        }
        self.buffer.node_map.insert(node.id, node.clone());
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
            }
            Class::Container { start, end } => {
                self.flush();
                let start_offset = self.emit(node.id, node.role.clone(), &start, start.clone());
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
                self.emit(node.id, node.role.clone(), &end, end.clone());
                self.record_container(node, start_offset);
            }
            Class::Heading(level) => {
                self.flush();
                let text = match text_content(node) {
                    t if t.is_empty() => node.name.as_deref().map(squash).unwrap_or_default(),
                    t => t,
                };
                let speech = format!("{}{text}", role_marker(&format!("heading level {level}")));
                let off = self.emit(node.id, node.role.clone(), &text, speech);
                self.buffer.headings.push((off, level));
            }
            Class::Link => self.visit_link(node),
            Class::Graphic => {
                self.flush();
                let alt = node.name.as_deref().map(squash).unwrap_or_default();
                let speech = format!("{alt}\n{}", role_marker("Graphic"));
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
                if let Some(line) = self.buffer.lines.get(first_line) {
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

    fn is_same_page(&self, href: Option<&str>) -> bool {
        let Some(href) = href else { return false };
        if href.starts_with('#') {
            return true;
        }
        match (href.split_once('#'), self.document_url) {
            (Some((base, _)), Some(doc)) => doc.split('#').next() == Some(base),
            _ => false,
        }
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
        let text = match text_content(node) {
            t if t.is_empty() => node.name.as_deref().map(squash).unwrap_or_default(),
            t => t,
        };
        let href = match &node.role {
            Role::Link { href } => href.as_deref(),
            _ => None,
        };
        let kind = if self.is_same_page(href) {
            "same page link"
        } else {
            "link"
        };
        let speech = format!("{prefix}{}{text}", role_marker(kind));
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
                let mut annotated = node.clone();
                annotated
                    .attributes
                    .insert("group_id".to_owned(), group_id.to_string());
                annotated
                    .attributes
                    .insert("group_name".to_owned(), group_name.clone());
                self.buffer.node_map.insert(node.id, annotated);
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
                    "Edit"
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
                s.push_str(&role_marker("Combo box"));
                s.push_str(&state_marker(state));
                s.push_str(node.value.as_deref().unwrap_or_default());
                s
            }
            Role::CheckBox => format!(
                "{name}{}{}",
                role_marker("check box"),
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
            Role::Button => {
                let mut s = format!("{name}{}", role_marker("Button"));
                if node.states.disabled {
                    s.push_str(&state_marker("Unavailable"));
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
            Role::Button if node.states.disabled => {}
            Role::Button => {
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
        let start_offset = self.emit(node.id, node.role.clone(), &start, start.clone());
        self.buffer.lists.push(start_offset);
        self.list_depth += 1;
        self.visit_children(node);
        self.list_depth -= 1;
        self.flush();
        let end = format!("list end{nesting}");
        self.emit(node.id, node.role.clone(), &end, end.clone());
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
        let start_offset = self.emit(node.id, node.role.clone(), &start, start.clone());

        let mut caption = None;
        // UIA names a table by its caption; without a caption element, the name is the caption.
        if node.children.iter().all(|c| c.role == Role::TableRow) {
            if let Some(name) = node.name.as_deref().map(squash).filter(|n| !n.is_empty()) {
                self.emit(node.id, Role::Text, &name, name.clone());
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
            self.buffer.node_map.insert(child.id, child.clone());
            let row_index = grid.len() + 1;
            let mut row_cells = Vec::new();
            for cell in child.children.iter().filter(|c| c.role == Role::TableCell) {
                self.flush();
                let before = self.buffer.lines.len();
                self.buffer.node_map.insert(cell.id, cell.clone());
                self.push_own_text(cell);
                for c in &cell.children {
                    self.visit(c, Some(cell));
                }
                self.flush();
                let offset = match self.buffer.lines.get(before) {
                    Some(line) => line.start_offset,
                    None => self.emit(cell.id, Role::TableCell, "", String::new()),
                };
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
        self.emit(
            node.id,
            node.role.clone(),
            "table end",
            "table end".to_owned(),
        );
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

/// Builds the virtual buffer for `root` (a document, or any subtree).
pub fn build(root: &AomNode) -> VirtualBuffer {
    let document_url = attr(root, "url");
    let mut builder = Builder::new(document_url);
    builder.buffer.node_map.insert(root.id, root.clone());

    // §1: line 0 is the page title.
    if root.role == Role::Document {
        let title = root.name.as_deref().map(squash).unwrap_or_default();
        if !title.is_empty() {
            builder.emit(root.id, Role::Document, &title, title.clone());
        }
        builder.visit_children(root);
    } else {
        builder.visit(root, None);
    }
    builder.flush();

    let mut buffer = builder.buffer;
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
}
