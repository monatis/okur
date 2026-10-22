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
use crate::messages::msg;

/// Role marker brackets used by JAWS speech history (`\u{1d} role \u{1e}`).
const RS: char = '\u{1d}';
const RE: char = '\u{1e}';
/// A horizontal rule's line, as JAWS shows it [obs, 292/s1].
const SEPARATOR_LINE: &str = "-----";
pub(crate) fn role_marker(name: &str) -> String {
    format!(" {RS} {name} {RE} ")
}

pub(crate) fn state_marker(name: &str) -> String {
    format!(" {RS}{name}{RE} ")
}

/// Where an element said between two marks starts and ends in a run's text: noncharacters,
/// which no page holds, left out of the line's text and said as the marks.
const DELETION_START: char = '\u{FDD0}';
const DELETION_END: char = '\u{FDD1}';
const SUPERSCRIPT_START: char = '\u{FDD2}';
const SUPERSCRIPT_END: char = '\u{FDD3}';
const SUBSCRIPT_START: char = '\u{FDD4}';
const SUBSCRIPT_END: char = '\u{FDD5}';

fn is_flow_mark(c: char) -> bool {
    ('\u{FDD0}'..='\u{FDD5}').contains(&c)
}

/// What a flow mark says, as JAWS brackets it: "\u{1d}Deletion \u{1e}" [obs, 814/s3],
/// "\u{1d}superscript \u{1e}" [obs, 818/s3].
fn flow_marker(mark: char) -> String {
    let name = match mark {
        DELETION_START => msg::role_deletion(),
        DELETION_END => msg::role_deletion_end(),
        SUPERSCRIPT_START => msg::role_superscript(),
        SUPERSCRIPT_END => msg::role_superscript_end(),
        SUBSCRIPT_START => msg::role_subscript(),
        _ => msg::role_subscript_end(),
    };
    format!(" {RS}{} {RE} ", name.as_str())
}

/// The marks an inline element is said between, given the run's text before it: deleted text
/// (IA2's content deletion) between "Deletion" and "end Deletion" [obs, 814/s3]; a raised or
/// lowered number right after a digit between "superscript" or "subscript" and their end,
/// "10 superscript 23", "10 superscript -6", "5 subscript 10" [obs, 818/s3, s4, s9], where
/// read together they would make another number. JAWS leaves a one-digit one unmarked ("2
/// cubed is 23", "1012" for binary 101 [obs, 818/s2, s7]), as its skim reading skips such
/// text; Okur marks it too (accepted divergence). Raised letters ("21st") and a number after
/// letters ("x10", "H2SO4") are read as they are [obs, 818/s5, s6, s8, s10].
fn flow_marks(node: &AomNode, before: &str) -> Option<(char, char)> {
    if attr(node, "aria_role") == Some("deletion") {
        return Some((DELETION_START, DELETION_END));
    }
    let formatting = node.formatting.as_ref()?;
    if node.text.is_some() || node.children.is_empty() {
        return None;
    }
    let after_digit = before
        .trim_end_matches(is_flow_mark)
        .ends_with(|c: char| c.is_ascii_digit());
    let content = text_content(node);
    let digits = content
        .strip_prefix(['-', '+', '\u{2212}'])
        .unwrap_or(&content);
    let number = !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit());
    if !after_digit || !number {
        return None;
    }
    if formatting.superscript {
        Some((SUPERSCRIPT_START, SUPERSCRIPT_END))
    } else if formatting.subscript {
        Some((SUBSCRIPT_START, SUBSCRIPT_END))
    } else {
        None
    }
}

/// A line of a run with its flow marks: its text, without them; its speech, with them said;
/// and the marks that end it, which are said at the next line's start, as JAWS says the "end
/// Deletion" of a paragraph struck through whole [obs, 814/s4-s5].
fn marked_line(line: &str) -> (String, String, String) {
    let text: String = line.chars().filter(|&c| !is_flow_mark(c)).collect();
    if text.len() == line.len() {
        return (text, line.to_owned(), String::new());
    }
    let ends = |c: char| matches!(c, DELETION_END | SUPERSCRIPT_END | SUBSCRIPT_END);
    let body = line.trim_end_matches(|c: char| ends(c) || c.is_whitespace());
    let carried = line
        .get(body.len()..)
        .unwrap_or_default()
        .chars()
        .filter(|&c| ends(c))
        .map(flow_marker)
        .collect();
    let speech = body
        .chars()
        .map(|c| {
            if is_flow_mark(c) {
                flow_marker(c)
            } else {
                c.to_string()
            }
        })
        .collect();
    (text, speech, carried)
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
    let role = button.role_description().unwrap_or(
        match button.role {
            Role::MenuButton => msg::role_button_menu(),
            _ => msg::role_button(),
        }
        .as_str(),
    );
    let state = match button.states.expanded {
        Some(true) => state_marker(msg::state_expanded().as_str()),
        Some(false) => state_marker(msg::state_collapsed().as_str()),
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
    // A control character in a page's text is a space to JAWS: "$3 �{" for "$3\u{2}�{" in a
    // table's cell [obs, 535/s7].
    text.split(|c: char| c.is_whitespace() || c.is_control())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// An editable text area's lines, as the virtual cursor reads them: its value by line breaks.
/// Empty for any other field (a field of one line is read whole).
fn text_area_lines(node: &AomNode) -> Vec<&str> {
    if node.role != Role::Edit
        || attr(node, "multiline") != Some("true")
        || attr(node, "readonly") == Some("true")
    {
        return Vec::new();
    }
    node.value
        .as_deref()
        .map(|v| v.lines().collect())
        .unwrap_or_default()
}

/// Whether `text` is only a list marker such as `•`, `◦`, `▪` or `3.`.
pub(crate) fn is_list_marker(text: &str) -> bool {
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
    /// A horizontal rule: its own line, "-----" said with its role [obs, 292/s1].
    Separator,
    /// A form control rendered as a single line; children are not rendered.
    Control,
    Table,
    List,
    ListItem,
    /// A page's menu bar: its items' lines, then "MenuBar end".
    MenuBar,
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
    /// Attributes the buffer adds to page nodes (a link to the same page, a radio button's
    /// group), as (node id, name, value): set on the page once the visit is over, since the
    /// page is only the buffer's then.
    annotations: Vec<(usize, &'static str, String)>,
    /// A link to a place on the page is a "same page link"; off, a "Link" as any other
    /// ([`PLAIN_SAME_PAGE_LINKS`]).
    same_page_links: bool,
    /// Graphics are read; off, left out of the page ([`NO_GRAPHICS`]).
    graphics: bool,
    headings: crate::settings::HeadingIndication,
    /// Marks said at the start of the next line: the "end Deletion" of deleted text that
    /// ended its line.
    pending_marks: String,
}

/// The page's attribute that says its same page links are read as plain links: JAWS's
/// `IdentifySamePageLinks=0` (724/s1: "Link Riverside Gazette"). Okur's reading choice, set
/// on the page as it loads, so that every buffer built from the page keeps it.
pub const PLAIN_SAME_PAGE_LINKS: &str = "okur_plain_same_page_links";

/// The page's attribute that says its graphics are left out: JAWS's `IncludeGraphics=0`
/// (727/s1: "There are no Graphics on this page."). Set as [`PLAIN_SAME_PAGE_LINKS`] is.
pub const NO_GRAPHICS: &str = "okur_no_graphics";

/// The page's attribute that says how its headings' roles are said, when not with their level
/// ("text" or "heading", [`crate::settings::HeadingIndication`]'s names) [obs, 730, 731].
pub const HEADING_INDICATION: &str = "okur_heading_indication";

/// The heading indication `page` is marked with.
pub fn heading_indication_of(page: &AomNode) -> crate::settings::HeadingIndication {
    use crate::settings::HeadingIndication;
    match attr(page, HEADING_INDICATION) {
        Some("text") => HeadingIndication::Text,
        Some("heading") => HeadingIndication::Heading,
        _ => HeadingIndication::Level,
    }
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
            annotations: Vec::new(),
            same_page_links: true,
            graphics: true,
            headings: crate::settings::HeadingIndication::Level,
            pending_marks: String::new(),
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
        let speech = if self.pending_marks.is_empty() {
            speech
        } else {
            std::mem::take(&mut self.pending_marks) + &speech
        };
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

    /// Splits the last span at `at`: what comes before is the list marker's (its own node, if
    /// it has one), the rest stays the line's. Returns `at`, or the span's start when `at` is
    /// not inside it.
    fn split_off_marker(&mut self, at: usize, marker_node: Option<usize>) -> usize {
        let Some(last) = self.buffer.spans.last_mut() else {
            return at;
        };
        if at <= last.start_offset || at >= last.end_offset {
            return last.start_offset;
        }
        let marker = BufferSpan {
            start_offset: last.start_offset,
            end_offset: at,
            node_id: marker_node.unwrap_or(last.node_id),
            role: Role::Text,
        };
        last.start_offset = at;
        let index = self.buffer.spans.len() - 1;
        self.buffer.spans.insert(index, marker);
        at
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
                // An empty line of it is a line of JAWS's text, a line break in its count
                // [obs, 650/s1: the <pre> that starts with one].
                if line.trim().is_empty() {
                    if line.is_empty() {
                        self.emit(node_id, role.clone(), "", String::new());
                    }
                    continue;
                }
                for part in wrap_lines(&line, MAX_LINE_LENGTH) {
                    self.emit(node_id, role.clone(), part, part.to_owned());
                }
            }
            self.last_text_line = Some(squash(&run.text));
            return;
        }
        let mut text = squash(&run.text);
        if text.chars().all(is_flow_mark) {
            return;
        }
        // The space where a link's line break cuts the text stays with it: "© 2026 Riverside
        // Gazette. " before Contact us, " open in May." after study rooms [obs, 635/s3-s4;
        // 562/s20: JAWS's count of the page has them].
        if run.text.starts_with(char::is_whitespace) {
            text.insert(0, ' ');
        }
        if run.text.ends_with(char::is_whitespace) {
            text.push(' ');
        }
        let node_id = run.node_id.unwrap_or_default();
        let role = run.role.unwrap_or(Role::Text);
        for line in wrap_lines(&text, MAX_LINE_LENGTH) {
            let (plain, speech, carried) = marked_line(line);
            self.emit(node_id, role.clone(), &plain, speech);
            self.pending_marks.push_str(&carried);
        }
        text.retain(|c| !is_flow_mark(c));
        self.last_text_line = Some(text);
    }

    fn push_inline_text(&mut self, node: &AomNode, text: &str) {
        let mut text = text;
        if self.run.text.trim().is_empty() {
            if let Some(idx) = self.absorb_punctuation_into.take() {
                let split = text
                    .find(|c: char| c.is_whitespace() || c.is_alphanumeric() || is_flow_mark(c))
                    .unwrap_or(text.len());
                let (punct, rest) = text.split_at(split);
                if !punct.is_empty() {
                    self.append_to_line(idx, punct);
                    // The space after punctuation the link took is not kept on the next line:
                    // "Maria Lopez," then "published March 14, 2026" (562/s20: JAWS's count
                    // of the page is one less with it).
                    text = rest.trim_start();
                } else {
                    text = rest;
                }
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
            Role::Separator => Class::Separator,
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
                        msg::region_named(name.as_str(), kind).render()
                    }
                };
                let start = match landmark_type.as_str() {
                    "banner" => msg::region_banner().to_string(),
                    "contentinfo" => msg::region_content_info().to_string(),
                    "main" => msg::region_main().to_string(),
                    "navigation" => label(msg::region_navigation().as_str()),
                    "complementary" => label(msg::region_complementary().as_str()),
                    "search" => label(msg::region_search().as_str()),
                    "region" => label(msg::region_region().as_str()),
                    other => label(other),
                };
                Class::Container {
                    end: msg::container_end(start.as_str()).render(),
                    start,
                }
            }
            Role::Dialog | Role::AlertDialog => {
                let name = node.name.as_deref().map(squash).unwrap_or_default();
                Class::Container {
                    start: msg::dialog_start(name).render().trim().to_owned(),
                    end: msg::dialog_end().to_string(),
                }
            }
            Role::BlockQuote => Class::Container {
                start: msg::block_quote().to_string(),
                end: msg::block_quote_end().to_string(),
            },
            Role::Group if aria == "article" => Class::Container {
                start: msg::article().to_string(),
                end: msg::article_end().to_string(),
            },
            // A named group is a fieldset/radio group: its name is the legend (§3).
            Role::Group
                if aria == "group"
                    && node.name.as_deref().is_some_and(|n| !n.trim().is_empty()) =>
            {
                let name = node.name.as_deref().map(squash).unwrap_or_default();
                Class::Container {
                    start: msg::group_start(name.as_str()).render(),
                    end: msg::group_end(name).render(),
                }
            }
            // A named tab list is a group to JAWS, with lines at both ends: Down past its tabs
            // reads "group end Settings", and the focus entering it says "Settings group"
            // [obs, 260/s3, s5; 29/s2].
            Role::TabList if node.name.as_deref().is_some_and(|n| !n.trim().is_empty()) => {
                let name = node.name.as_deref().map(squash).unwrap_or_default();
                Class::Container {
                    start: msg::group_start(name.as_str()).render(),
                    end: msg::group_end(name).render(),
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
            Role::MenuBar => Class::MenuBar,
            Role::MenuItem => Class::Control,
            _ => Class::Block,
        }
    }

    /// Visits `node`; inside a preformatted element (`<pre>`) text keeps its whitespace.
    fn visit(&mut self, node: &AomNode, parent: Option<&AomNode>) {
        let first_line = self.buffer.lines.len();
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
        self.record_focus_stop(node, first_line);
    }

    /// Records `node` as a focus stop when it can take the focus and is not a Tab stop
    /// already, at its first line from `first_line` on.
    fn record_focus_stop(&mut self, node: &AomNode, first_line: usize) {
        if node.states.focusable != Some(true) || node.role == Role::Document {
            return;
        }
        let start = self
            .buffer
            .lines
            .iter()
            .skip(first_line)
            .find(|line| line.node_id == node.id || node.find_by_id(line.node_id).is_some())
            .map(|line| line.start_offset);
        if let Some(start) = start {
            if self.buffer.tab_stops.binary_search(&start).is_err() {
                self.buffer.focus_stops.push((start, node.id));
            }
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
                let marks = flow_marks(node, &self.run.text);
                if let Some((start, _)) = marks {
                    self.push_inline_text(node, &start.to_string());
                }
                self.push_own_text(node);
                self.visit_children(node);
                if let Some((_, end)) = marks {
                    self.push_inline_text(node, &end.to_string());
                }
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
            // JAWS's virtual cursor reads a menu bar's items as lines, "File menu sub menu
            // collapsed", after its name and size, "Main menu MenuBar with 3 items" [obs,
            // 652/s1], and ends it with "MenuBar end" [obs, 218/s1-s4, Auto Forms Mode off].
            Class::MenuBar => {
                self.flush();
                let items = node
                    .children
                    .iter()
                    .filter(|c| c.role == Role::MenuItem)
                    .count();
                if items > 0 {
                    let start = match node.name.as_deref().map(squash).filter(|n| !n.is_empty()) {
                        Some(name) => msg::menu_bar_start_named(name.as_str(), items).render(),
                        None => msg::menu_bar_start(items).render(),
                    };
                    self.emit_boundary(node.id, node.role.clone(), &start);
                }
                let start = self.buffer.lines.len();
                self.visit_children(node);
                self.flush();
                if self.buffer.lines.len() > start {
                    self.emit_boundary(node.id, node.role.clone(), msg::menu_bar_end().as_str());
                }
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
                // A tab list's name is its tabs' group name, as in a list of its own.
                let is_tablist = node.role == Role::TabList;
                if is_tablist {
                    self.tablists.push(legend.clone().unwrap_or_default());
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
                if is_tablist {
                    self.tablists.pop();
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
                let role = self
                    .headings
                    .role(level)
                    .map(|role| role_marker(&role))
                    .unwrap_or_default();
                let speech = format!("{role}{text}{suffix}");
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
            Class::Graphic if !self.graphics => {}
            Class::Graphic => {
                self.flush();
                let alt = node.name.as_deref().map(squash).unwrap_or_default();
                // Its key shortcuts follow its name: "monatis Alt+ArrowUp | Graphic" (JAWS on
                // the dev box, GitHub's avatars).
                let named = match attr(node, "keyshortcuts") {
                    Some(keys) => format!("{alt} {keys}"),
                    None => alt.clone(),
                };
                let speech = format!("{named}\n{}", role_marker(msg::role_graphic().as_str()));
                let off = self.emit(node.id, node.role.clone(), &alt, speech);
                self.buffer.graphics.push(off);
            }
            Class::Separator => {
                self.flush();
                let speech = format!(
                    "{SEPARATOR_LINE}{}",
                    role_marker(msg::role_separator().as_str())
                );
                let off = self.emit(node.id, node.role.clone(), SEPARATOR_LINE, speech);
                self.buffer.separators.push(off);
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
        let (prefix, marker_node) = if is_list_marker(&self.run.text) {
            let run = std::mem::take(&mut self.run);
            (squash(&run.text), run.node_id)
        } else {
            self.flush();
            (String::new(), None)
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
            None if same_page && self.same_page_links => msg::role_link_same_page().as_str(),
            // A mailto: link is a "Send Mail Link" [obs, 293/s1].
            None if href.is_some_and(|h| h.starts_with("mailto:")) => {
                msg::role_link_mailto().as_str()
            }
            None => msg::role_link().as_str(),
        };
        if same_page && self.buffer.contains_node(node.id) {
            self.annotations
                .push((node.id, "same_page", "true".to_owned()));
        }
        // aria-current follows the role: "‹ Link › ‹Current Page› README"; a visited link's
        // state comes before it: "• ‹visited› ‹ Link › City Library Expands Evening Hours"
        // [obs, 689/s5].
        let current = node.current_phrase().map(state_marker).unwrap_or_default();
        let visited = if node.is_visited() {
            state_marker(msg::state_visited().as_str())
        } else {
            String::new()
        };
        let speech = format!("{prefix}{visited}{}{current}{text}", role_marker(kind));
        let line_text = squash(&format!("{prefix} {text}"));
        let line_start = self.emit(node.id, node.role.clone(), &line_text, speech);
        // A list item's bullet is not the link's: the link starts after it and its space,
        // where the cursor goes on it [obs, 635/s10-s12: Left from the link's first letter
        // says nothing (the space), then "•"].
        let off = if prefix.is_empty() {
            line_start
        } else {
            self.split_off_marker(line_start + prefix.len() + 1, marker_node)
        };
        self.buffer.links.push(off);
        self.buffer.tab_stops.push(off);
        self.absorb_punctuation_into = Some(self.buffer.lines.len() - 1);
    }

    /// A read-only text area with text (GitHub's code view): its lines are read with the
    /// virtual cursor, as JAWS reads them (dev box, 2026-10-16: "file content read only edit",
    /// then "resolver = "2"", "members= ["). The first carries its name and role. Returns
    /// false for any other control.
    fn visit_read_only_text(&mut self, node: &AomNode) -> bool {
        let text = node.value.as_deref().unwrap_or_default();
        if node.role != Role::Edit
            || attr(node, "readonly") != Some("true")
            || attr(node, "multiline") != Some("true")
            || text.is_empty()
        {
            return false;
        }
        let name = node.name.as_deref().map(squash).unwrap_or_default();
        let lines = crate::editing::text_lines(text);
        let mut first = None;
        for (i, line) in lines.iter().enumerate() {
            let speech = if i == 0 {
                format!(
                    "{name}{}{line}",
                    role_marker(msg::role_edit_read_only().as_str())
                )
            } else {
                line.clone()
            };
            let off = self.emit(node.id, node.role.clone(), line, speech);
            first.get_or_insert(off);
        }
        if let Some(off) = first {
            self.buffer.edits.push(off);
            self.buffer.form_controls.push(off);
            self.buffer.tab_stops.push(off);
        }
        true
    }

    /// §10: one line per form control, in JAWS's line-reading phrasing.
    fn visit_control(&mut self, node: &AomNode) {
        // Radio buttons remember their enclosing group, which focus speech names when the
        // cursor enters the group (§17).
        if node.role == Role::RadioButton {
            if let Some((group_id, group_name)) = self.groups.last() {
                let (group_id, group_name) = (group_id.to_string(), group_name.clone());
                self.buffer.mark_readable(node.id);
                self.annotations.push((node.id, "group_id", group_id));
                self.annotations.push((node.id, "group_name", group_name));
            }
        }
        // Flush first: an inline label in the same block is still pending in the run.
        self.flush();
        if self.visit_read_only_text(node) {
            return;
        }
        let labelled_by_previous_line = self.last_text_line.clone();
        let labelled = labelled_by_previous_line.is_some()
            && labelled_by_previous_line.as_deref() == node.name.as_deref().map(squash).as_deref();
        let name = node.name.as_deref().map(squash).unwrap_or_default();
        let required = attr(node, "required") == Some("true");
        let checked = |c: Option<bool>| {
            if c == Some(true) {
                msg::state_checked().as_str()
            } else {
                msg::state_not_checked().as_str()
            }
        };

        let speech = match &node.role {
            Role::Edit => {
                let kind = if attr(node, "password") == Some("true") {
                    msg::role_edit_password().as_str()
                } else if attr(node, "readonly") == Some("true") {
                    msg::role_edit_read_only().as_str()
                } else {
                    node.role_description().unwrap_or(msg::role_edit().as_str())
                };
                let mut s = String::new();
                if !labelled {
                    s.push_str(&name);
                }
                if required {
                    s.push_str(&state_marker(msg::state_required().as_str()));
                }
                s.push_str(&role_marker(kind));
                if let Some(v) = node.value.as_deref().filter(|v| !v.trim().is_empty()) {
                    // A text area's first line; its others are lines of their own (below).
                    s.push_str(text_area_lines(node).first().copied().unwrap_or(v));
                }
                s
            }
            Role::ComboBox => {
                let state = if node.states.expanded == Some(true) {
                    msg::state_expanded()
                } else {
                    msg::state_collapsed()
                }
                .as_str();
                let mut s = String::new();
                if !labelled {
                    s.push_str(&name);
                }
                // One typed in is an "edit combo" (JAWS on the dev box).
                let kind = if attr(node, "editable") == Some("true") {
                    msg::role_edit_combo()
                } else {
                    msg::role_combo_box_focus()
                }
                .as_str();
                s.push_str(&role_marker(kind));
                s.push_str(&state_marker(state));
                s.push_str(node.value.as_deref().unwrap_or_default());
                s
            }
            Role::CheckBox => format!(
                "{name}{}{}",
                role_marker(
                    node.role_description()
                        .unwrap_or(msg::role_check_box().as_str())
                ),
                state_marker(checked(node.states.checked))
            ),
            Role::RadioButton => {
                let mut s = format!(
                    "{name}{}{}",
                    role_marker(msg::role_radio_button().as_str()),
                    state_marker(checked(node.states.checked))
                );
                if let (Some(pos), Some(size)) = (attr(node, "posinset"), attr(node, "setsize")) {
                    s.push_str(&format!("\n{}", msg::position(pos, size)));
                }
                s
            }
            // "Platform ‹ Button › ‹collapsed›", "Overview ‹ Button › ‹Current Page›".
            Role::Button => {
                let role = node
                    .role_description()
                    .unwrap_or(msg::role_button().as_str());
                let mut s = format!("{name}{}", role_marker(role));
                match node.states.expanded {
                    Some(true) => s.push_str(&state_marker(msg::state_expanded().as_str())),
                    Some(false) => s.push_str(&state_marker(msg::state_collapsed().as_str())),
                    None => {}
                }
                // Any other popup: "Pick one ‹ Button › ‹Has Popup› listbox" [obs, 210/s15].
                if let Some(popup) = attr(node, "haspopup") {
                    s.push_str(&state_marker(msg::state_has_popup().as_str()));
                    s.push_str(&format!(" {popup}"));
                }
                if let Some(current) = node.current_phrase() {
                    s.push_str(&state_marker(current));
                }
                if node.states.disabled {
                    s.push_str(&state_marker(msg::state_unavailable().as_str()));
                }
                s
            }
            // A button opening a menu: "Actions ‹ button menu › ‹collapsed›" (JAWS on the dev box).
            Role::MenuButton => {
                let role = node
                    .role_description()
                    .unwrap_or(msg::role_button_menu().as_str());
                let mut s = format!("{name}{}", role_marker(role));
                match node.states.expanded {
                    Some(true) => s.push_str(&state_marker(msg::state_expanded().as_str())),
                    Some(false) => s.push_str(&state_marker(msg::state_collapsed().as_str())),
                    None => {}
                }
                s
            }
            // A menu bar's item: "File menu sub menu collapsed", "Help menu" [obs, 218/s1-s3].
            Role::MenuItem => {
                let mut s = format!("{name}{}", role_marker(msg::role_menu().as_str()));
                if attr(node, "submenu") == Some("true") {
                    s.push_str(&state_marker(msg::state_sub_menu().as_str()));
                }
                match node.states.expanded {
                    Some(true) => s.push_str(&state_marker(msg::state_expanded().as_str())),
                    Some(false) => s.push_str(&state_marker(msg::state_collapsed().as_str())),
                    None => {}
                }
                s
            }
            Role::Tab => format!("{name}{}", role_marker(msg::role_tab().as_str())),
            Role::Slider => format!("{name}{}", role_marker(msg::role_slider().as_str())),
            Role::ProgressBar => {
                format!("{name}{}", role_marker(msg::role_progress_bar().as_str()))
            }
            _ => name.clone(),
        };
        // A field's line reads as its label and its text: the role and states between markers
        // are speech only, since JAWS's virtual cursor moves by the field's own words ("lazy"
        // after the focus lands at a text area's start; 213, old-pc, 2026-10-18).
        let text = if node.role == Role::Edit {
            let mut inside = false;
            let words: String = speech
                .chars()
                .filter(|&c| {
                    let marker = c == RS || c == RE;
                    if marker {
                        inside = c == RS;
                    }
                    !marker && !inside
                })
                .collect();
            squash(&words)
        } else {
            squash(&speech.replace([RS, RE], " "))
        };
        let off = self.emit(node.id, node.role.clone(), &text, speech);
        // A text area's lines after its first are lines of the page, each with the field's
        // role: " Edit | Mail ada@example.org ..." [obs, 508/s2-s5; 408's Say All]. Their
        // text is the field's, its leading spaces and tabs kept: JAWS's count of the page has
        // them [obs, 650/s1].
        if node.role == Role::Edit {
            let kind = role_marker(msg::role_edit().as_str());
            for line in text_area_lines(node).into_iter().skip(1) {
                self.emit(
                    node.id,
                    node.role.clone(),
                    line.trim_end(),
                    format!("{kind}{line}"),
                );
            }
        }

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
                // One typed in is an edit too: E stops on it (JAWS on the dev box, GitHub's
                // "Go to file" edit combo).
                if attr(node, "editable") == Some("true") {
                    self.buffer.edits.push(off);
                }
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
        // A menu's items are reached by the page's own Tab and arrows: the buffer cannot tell
        // the one item in the Tab order (tabindex 0) from the others.
        if !node.states.disabled && !matches!(node.role, Role::ProgressBar | Role::MenuItem) {
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
        let nesting = self.list_depth;
        let start = if nesting > 0 {
            msg::list_start_nested(items, nesting).render()
        } else {
            msg::list_start(items).render()
        };
        let start_offset = self.emit_boundary(node.id, node.role.clone(), &start);
        self.buffer.lists.push(start_offset);
        self.list_depth += 1;
        self.visit_children(node);
        self.list_depth -= 1;
        self.flush();
        let end = if nesting > 0 {
            msg::list_end_nested(nesting).render()
        } else {
            msg::list_end().to_string()
        };
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
        let start = msg::table_entry(cols, rows.len()).render();
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
        let end_line = self.emit_boundary(node.id, node.role.clone(), msg::table_end().as_str());
        self.buffer.bounded.push((start_offset, end_line));
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
        if let Some(end_line) = self.buffer.lines.last() {
            self.buffer
                .bounded
                .push((start_offset, end_line.start_offset));
        }
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
        // Colons stay with the word before them, and a letter after them begins a word:
        // "std::" then "io" (213, old-pc, 2026-10-18). A time's colon, between digits, is
        // inside its word: "10:30" [obs, 407/s22].
        let before_colon = i.checked_sub(2).and_then(|p| chars.get(p)).map(|&(_, c)| c);
        let in_time = c.is_ascii_digit() && before_colon.is_some_and(|b| b.is_ascii_digit());
        let after_colons = prev == Some(':') && c.is_alphanumeric() && !in_time;
        if separator {
            if let Some(s) = start.take() {
                words.push((s, offset));
            }
            words.push((offset, offset + c.len_utf8()));
        } else if after_colons {
            if let Some(s) = start.replace(offset) {
                words.push((s, offset));
            }
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
    build_owned(root.clone())
}

/// Builds the virtual buffer for `root`, which the buffer keeps as its page: no copy of it is
/// made. A page spliced after a change was copied twice per part (responsiveness review,
/// 2026-10-19).
pub fn build_owned(root: AomNode) -> VirtualBuffer {
    let (mut buffer, annotations) = visit_page(&root);
    buffer.set_page(root);
    for (id, name, value) in annotations {
        if let Some(node) = buffer.node_mut(id) {
            node.attributes.insert(name.to_owned(), value);
        }
    }
    resolve_flows(&mut buffer);
    buffer
}

/// aria-flowto: each element that flows to others (its `flows-to` uniqueIDs), and where those
/// are, by the lines they start on; the element's line ends "Has Flows To" [obs, 292/s9].
fn resolve_flows(buffer: &mut VirtualBuffer) {
    let line_start = |buffer: &VirtualBuffer, id: usize| {
        let start = buffer.start_of(id)?;
        buffer.line_at_offset(start).map(|l| l.start_offset)
    };
    let mut flows = Vec::new();
    for node in buffer.nodes() {
        let Some(targets) = node.attributes.get("flows-to") else {
            continue;
        };
        let Some(from) = line_start(buffer, node.id) else {
            continue;
        };
        flows.extend(
            targets
                .split_whitespace()
                .filter_map(|object| buffer.node_with_object(object))
                .filter_map(|target| line_start(buffer, target))
                .map(|into| (from, into)),
        );
    }
    // The element a flow comes from ends "Has Flows To" [obs, 292/s9], the one it goes to
    // "Has Flows From" ("heading level 2 Lists Has Flows From" [obs, 563/s8, s9]).
    let marks = flows
        .iter()
        .map(|&(from, _)| (from, msg::flow_has_flows_to().to_string()))
        .chain(
            flows
                .iter()
                .map(|&(_, into)| (into, msg::flow_has_flows_from().to_string())),
        );
    for (start, said) in marks {
        if let Some(line) = buffer.lines.iter_mut().find(|l| l.start_offset == start) {
            if !line.speech.ends_with(&said) {
                line.speech = format!("{}\n{said}", line.speech);
            }
        }
    }
    buffer.flows = flows;
}

/// The buffer's lines and indexes for `root`, and the attributes to add to its nodes.
fn visit_page(root: &AomNode) -> (VirtualBuffer, Vec<(usize, &'static str, String)>) {
    let document_url = attr(root, "url");
    let mut builder = Builder::new(document_url);
    builder.same_page_links = attr(root, PLAIN_SAME_PAGE_LINKS).is_none();
    builder.graphics = attr(root, NO_GRAPHICS).is_none();
    builder.headings = heading_indication_of(root);
    builder.buffer.root_id = Some(root.id);
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
    // Recorded innermost first: of the stops on one line, the innermost stays.
    buffer.focus_stops.sort_by_key(|&(start, _)| start);
    buffer.focus_stops.dedup_by_key(|&mut (start, _)| start);
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
    (buffer, builder.annotations)
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

    /// Ctrl+Right through a text area on old-pc (213, 2026-10-18): "std::" then "io", and the
    /// field's role is not a word of its line.
    #[test]
    fn a_text_area_line_reads_by_its_own_words() {
        assert_eq!(
            words("then reads std::io again"),
            vec!["then", "reads", "std::", "io", "again"]
        );
        let area = AomNode::new(1, Role::Edit).with_value("the lazy dog");
        let root = AomNode::new(0, Role::Document).with_children(vec![area]);
        let buffer = crate::buffer::VirtualBuffer::from_aom(&root);
        let line = buffer.lines.first();
        assert_eq!(line.map(|l| l.text.as_str()), Some("the lazy dog"));
        assert!(line.is_some_and(|l| l.speech.contains(&role_marker("Edit"))));
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

    /// A raised or lowered number right after a number is said between its marks, a one-digit
    /// one too; raised letters and a number after letters are not [obs, 818/s3, s5, s6, s9].
    #[test]
    fn a_raised_number_after_a_number_is_marked() {
        let text = |id: usize, t: &str| AomNode::new(id, Role::Text).with_text(t);
        let positioned = |id: usize, t: &str, superscript: bool| {
            let formatting = crate::aom::TextFormatting {
                superscript,
                subscript: !superscript,
                ..Default::default()
            };
            AomNode::new(id, Role::Text)
                .with_formatting(formatting.clone())
                .with_children(vec![text(id + 1, t).with_formatting(formatting)])
        };
        let paragraph = |id: usize, children: Vec<AomNode>| {
            AomNode::new(id, Role::Paragraph).with_children(children)
        };
        let root = AomNode::new(0, Role::Document).with_children(vec![
            paragraph(
                1,
                vec![
                    text(2, "It holds 10"),
                    positioned(3, "23", true),
                    text(5, " items."),
                ],
            ),
            paragraph(
                6,
                vec![
                    text(7, "Two cubed is 2"),
                    positioned(8, "3", true),
                    text(10, " here."),
                ],
            ),
            paragraph(
                11,
                vec![
                    text(12, "The value 5"),
                    positioned(13, "10", false),
                    text(15, " is decimal."),
                ],
            ),
            paragraph(
                16,
                vec![
                    text(17, "The 21"),
                    positioned(18, "st", true),
                    text(20, " day."),
                ],
            ),
            paragraph(
                21,
                vec![
                    text(22, "The power x"),
                    positioned(23, "10", true),
                    text(25, " grows."),
                ],
            ),
        ]);
        let buffer = crate::buffer::VirtualBuffer::from_aom(&root);
        let read: Vec<(&str, String)> = buffer
            .lines
            .iter()
            .map(|l| {
                (
                    l.text.as_str(),
                    l.speech.split_whitespace().collect::<Vec<_>>().join(" "),
                )
            })
            .collect();
        assert_eq!(
            read,
            vec![
                (
                    "It holds 1023 items.",
                    "It holds 10 \u{1d}superscript \u{1e} 23 \u{1d}end superscript \u{1e} items."
                        .to_owned()
                ),
                (
                    "Two cubed is 23 here.",
                    "Two cubed is 2 \u{1d}superscript \u{1e} 3 \u{1d}end superscript \u{1e} here."
                        .to_owned()
                ),
                (
                    "The value 510 is decimal.",
                    "The value 5 \u{1d}subscript \u{1e} 10 \u{1d}end subscript \u{1e} is decimal."
                        .to_owned()
                ),
                ("The 21st day.", "The 21st day.".to_owned()),
                ("The power x10 grows.", "The power x10 grows.".to_owned()),
            ]
        );
    }

    /// Struck or deleted text is said between "Deletion" and "end Deletion", which are not the
    /// line's text; a paragraph deleted whole says its end at the next line's start [obs,
    /// 814/s3-s5].
    #[test]
    fn deleted_text_is_said_between_its_marks() {
        let text = |id: usize, t: &str| AomNode::new(id, Role::Text).with_text(t);
        let deleted = |id: usize, t: &str| {
            let mut node = AomNode::new(id, Role::Text).with_children(vec![text(id + 1, t)]);
            node.attributes
                .insert("aria_role".to_owned(), "deletion".to_owned());
            node
        };
        let paragraph = |id: usize, children: Vec<AomNode>| {
            AomNode::new(id, Role::Paragraph).with_children(children)
        };
        let root = AomNode::new(0, Role::Document).with_children(vec![
            paragraph(
                1,
                vec![
                    text(2, "The price was "),
                    deleted(3, "twelve dollars"),
                    text(5, ", now ten."),
                ],
            ),
            paragraph(6, vec![deleted(7, "A whole line struck through.")]),
            paragraph(9, vec![text(10, "The meeting is on Tuesday.")]),
        ]);
        let buffer = crate::buffer::VirtualBuffer::from_aom(&root);
        let read: Vec<(&str, String)> = buffer
            .lines
            .iter()
            .map(|l| {
                (
                    l.text.as_str(),
                    l.speech.split_whitespace().collect::<Vec<_>>().join(" "),
                )
            })
            .collect();
        assert_eq!(
            read,
            vec![
                (
                    "The price was twelve dollars, now ten.",
                    "The price was \u{1d}Deletion \u{1e} twelve dollars \u{1d}end Deletion \u{1e} , now ten."
                        .to_owned()
                ),
                (
                    "A whole line struck through.",
                    "\u{1d}Deletion \u{1e} A whole line struck through.".to_owned()
                ),
                (
                    "The meeting is on Tuesday.",
                    "\u{1d}end Deletion \u{1e} The meeting is on Tuesday.".to_owned()
                ),
            ]
        );
    }

    /// The text around inline links as JAWS's holds it [obs, 635; 562/s13, s17, s20]: the space
    /// where a link's line break cuts the text stays ("Gazette. ", " open in May."), but not
    /// after punctuation the link took ("Maria Lopez," then "published"); a list item's link
    /// starts after its bullet and the bullet's space.
    #[test]
    fn the_spaces_beside_an_inline_link_are_kept() {
        let paragraph = |id: usize, children: Vec<AomNode>| {
            AomNode::new(id, Role::Paragraph).with_children(children)
        };
        let text = |id: usize, t: &str| AomNode::new(id, Role::Text).with_text(t);
        let root = AomNode::new(0, Role::Document).with_children(vec![
            paragraph(
                1,
                vec![
                    text(2, "By "),
                    link(3, "#author", "Maria Lopez", None),
                    text(4, ", published March 14"),
                ],
            ),
            paragraph(
                5,
                vec![
                    text(6, "New "),
                    link(7, "#rooms", "study rooms", None),
                    text(8, " open in May."),
                ],
            ),
            AomNode::new(9, Role::List).with_children(vec![AomNode::new(10, Role::ListItem)
                .with_children(vec![
                    text(11, "\u{2022} "),
                    link(12, "#budget", "Council approves library budget", None),
                ])]),
        ]);
        let buffer = crate::buffer::VirtualBuffer::from_aom(&root);
        let lines: Vec<&str> = buffer.lines.iter().map(|l| l.text.as_str()).collect();
        for kept in ["By ", "New ", " open in May.", "published March 14"] {
            assert!(lines.contains(&kept), "{kept:?} in {lines:?}");
        }
        assert!(lines.contains(&"Maria Lopez,"), "{lines:?}");
        let item = buffer
            .lines
            .iter()
            .find(|l| l.text.starts_with('\u{2022}'))
            .map(|l| l.start_offset);
        let bullet = "\u{2022} ".len();
        assert!(
            item.is_some_and(|start| buffer.links.contains(&(start + bullet))),
            "{:?} {:?}",
            buffer.links,
            item
        );
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

    /// With Identify Same Page Links off, the page carries the choice and a link to its own
    /// places is a "Link" like any other [obs, 724/s1].
    #[test]
    fn same_page_links_can_read_as_plain_links() {
        let page = github_tabs().page().map(|p| AomNode::clone(p));
        assert!(page.is_some());
        let Some(mut page) = page else {
            return;
        };
        page.attributes
            .insert(PLAIN_SAME_PAGE_LINKS.to_owned(), "true".to_owned());
        let buffer = crate::buffer::VirtualBuffer::from_aom(&page);
        let speech: Vec<&str> = buffer.lines.iter().map(|l| l.speech.as_str()).collect();
        assert_eq!(
            speech.get(1).copied(),
            Some(format!("{}Skip", role_marker("Link")).as_str())
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

    /// A text area in the virtual cursor: each of its lines is a line of the page with the
    /// field's role, " Edit | Mail ada@example.org" [obs, 508].
    #[test]
    fn a_text_area_is_read_line_by_line() {
        let mut notes = AomNode::new(1, Role::Edit)
            .with_name("Notes")
            .with_value("First line\nSecond line");
        notes
            .attributes
            .insert("multiline".to_owned(), "true".to_owned());
        let root = AomNode::new(0, Role::Document).with_children(vec![notes]);
        let buffer = crate::buffer::VirtualBuffer::from_aom(&root);
        let speech: Vec<String> = buffer
            .lines
            .iter()
            .map(|l| l.speech.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect();
        assert_eq!(
            speech,
            vec![
                "Notes \u{1d} Edit \u{1e} First line".to_owned(),
                "\u{1d} Edit \u{1e} Second line".to_owned()
            ]
        );
        assert!(buffer.lines.iter().all(|l| l.node_id == 1));
    }

    /// The text of a text area's lines keeps their leading spaces and tabs, and preformatted
    /// text its empty lines, as JAWS's count of the page has them [obs, 650/s1].
    #[test]
    fn a_fields_and_a_pres_lines_keep_their_spaces() {
        let mut notes = AomNode::new(1, Role::Edit)
            .with_name("Notes")
            .with_value("First line\n    four spaces\n\tone tab");
        notes
            .attributes
            .insert("multiline".to_owned(), "true".to_owned());
        let mut pre =
            AomNode::new(2, Role::Paragraph)
                .with_children(vec![AomNode::new(3, Role::Text).with_text("\n    indented")]);
        pre.attributes
            .insert("preformatted".to_owned(), "true".to_owned());
        let root = AomNode::new(0, Role::Document).with_children(vec![notes, pre]);
        let buffer = crate::buffer::VirtualBuffer::from_aom(&root);
        let lines: Vec<&str> = buffer.lines.iter().map(|l| l.text.as_str()).collect();
        assert!(lines.contains(&"    four spaces"), "{lines:?}");
        assert!(lines.contains(&"\tone tab"), "{lines:?}");
        let indented = lines.iter().position(|l| *l == "    indented");
        assert_eq!(
            indented
                .and_then(|i| i.checked_sub(1))
                .and_then(|i| lines.get(i)),
            Some(&""),
            "{lines:?}"
        );
    }

    /// Shift+Period and Shift+Comma move to the end and the start of the innermost element
    /// with lines at both ends; a list's item is in its list [obs, 294/s5-s7].
    #[test]
    fn the_element_edges_are_the_innermost_elements_lines() {
        let item = |id: usize, text: &str| {
            AomNode::new(id, Role::ListItem)
                .with_children(vec![AomNode::new(id + 100, Role::Text).with_text(text)])
        };
        let inner = AomNode::new(4, Role::List).with_children(vec![item(5, "Inner")]);
        let root = AomNode::new(0, Role::Document).with_children(vec![
            AomNode::new(1, Role::Paragraph)
                .with_children(vec![AomNode::new(101, Role::Text).with_text("Before")]),
            AomNode::new(2, Role::List).with_children(vec![
                item(3, "First"),
                AomNode::new(6, Role::ListItem).with_children(vec![
                    AomNode::new(106, Role::Text).with_text("Second"),
                    inner,
                ]),
            ]),
        ]);
        let buffer = crate::buffer::VirtualBuffer::from_aom(&root);
        let line_of = |text: &str| buffer.lines.iter().position(|l| l.text.contains(text));
        let lines = (line_of("Before"), line_of("First"), line_of("Inner"));
        assert!(
            matches!(lines, (Some(_), Some(_), Some(_))),
            "{:?}",
            buffer.lines
        );
        let (Some(before), Some(first), Some(inner)) = lines else {
            return;
        };
        let mut cursor = crate::cursor::VirtualCursor::new(buffer);
        // A paragraph outside such elements is one line, at its own edges: its end goes on to
        // the next element's line, its start back [obs, 557/s3, s4].
        cursor.move_to_line(before);
        assert_eq!(
            cursor.move_element_edge(true).as_deref(),
            Some("list of 2 items")
        );
        cursor.move_to_line(inner);
        assert_eq!(
            cursor.move_element_edge(true).as_deref(),
            Some("list end nesting level 1")
        );
        assert_eq!(
            cursor.move_element_edge(false).as_deref(),
            Some("list of 1 items nesting level 1")
        );
        cursor.move_to_line(first);
        assert_eq!(cursor.move_element_edge(true).as_deref(), Some("list end"));
        assert_eq!(
            cursor.move_element_edge(false).as_deref(),
            Some("list of 2 items")
        );
    }

    /// Equals follows a flow to its element and Shift+Equals back; elsewhere "No more flows
    /// found" [obs, 293/s8, s9].
    #[test]
    fn a_flow_is_followed_both_ways() {
        let para = |id: usize, text: &str| {
            AomNode::new(id, Role::Paragraph).with_children(vec![AomNode::new(
                id + 100,
                Role::Text,
            )
            .with_text(text)])
        };
        let root = AomNode::new(0, Role::Document).with_children(vec![
            para(1, "Start here"),
            para(2, "In between"),
            para(3, "Go on here"),
        ]);
        let mut buffer = crate::buffer::VirtualBuffer::from_aom(&root);
        let start = |text: &str| {
            buffer
                .lines
                .iter()
                .find(|l| l.text == text)
                .map_or(0, |l| l.start_offset)
        };
        let flow = (start("Start here"), start("Go on here"));
        let between = buffer.lines.iter().position(|l| l.text == "In between");
        buffer.flows.push(flow);
        let mut cursor = crate::cursor::VirtualCursor::new(buffer);
        cursor.move_to_line(between.unwrap_or_default());
        assert_eq!(
            cursor.move_flow(true).as_deref(),
            Some("No more flows found")
        );
        cursor.move_to_line(0);
        assert_eq!(cursor.move_flow(true).as_deref(), Some("Go on here"));
        assert_eq!(cursor.move_flow(false).as_deref(), Some("Start here"));
    }

    /// An element's aria-flowto, its IA2 flows-to targets by uniqueID, is a flow from its line
    /// to the target's, and its line ends "Has Flows To" [obs, 292/s9].
    #[test]
    fn flows_to_resolve_by_unique_id() {
        let object = |node: AomNode, id: &str| {
            let mut node = node;
            node.attributes
                .insert(crate::aom::UNIQUE_ID_ATTR.to_owned(), id.to_owned());
            node
        };
        let mut from = object(
            AomNode::new(1, Role::Paragraph)
                .with_children(vec![AomNode::new(101, Role::Text).with_text("This part")]),
            "-10",
        );
        from.attributes
            .insert("flows-to".to_owned(), "-30".to_owned());
        let root = AomNode::new(0, Role::Document).with_children(vec![
            from,
            AomNode::new(2, Role::Paragraph)
                .with_children(vec![AomNode::new(102, Role::Text).with_text("Between")]),
            object(
                AomNode::new(3, Role::Heading { level: 2 }).with_name("Lists"),
                "-30",
            ),
        ]);
        let buffer = crate::buffer::VirtualBuffer::from_aom(&root);
        let start = |text: &str| {
            buffer
                .lines
                .iter()
                .find(|l| l.text == text)
                .map(|l| l.start_offset)
        };
        assert_eq!(
            buffer.flows.first().copied(),
            start("This part").zip(start("Lists"))
        );
        assert_eq!(
            buffer.lines.first().map(|l| l.speech.as_str()),
            Some("This part\nHas Flows To")
        );
        // The target ends "Has Flows From" [obs, 563/s8].
        assert!(buffer
            .lines
            .iter()
            .find(|l| l.text == "Lists")
            .is_some_and(|l| l.speech.ends_with("Has Flows From")));
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
                    "Pick one{}{} listbox",
                    role_marker("Button"),
                    state_marker("Has Popup")
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

    /// A page's menu bar in the virtual cursor: its name and size, its items' lines, then
    /// "MenuBar end" [obs, 652/s1; 218/s1-s4].
    #[test]
    fn a_menu_bar_reads_its_items_as_lines() {
        let item = |id: usize, name: &str, submenu: bool| {
            let mut item = AomNode::new(id, Role::MenuItem).with_name(name);
            if submenu {
                item.attributes
                    .insert("submenu".to_owned(), "true".to_owned());
                item.states.expanded = Some(false);
            }
            item
        };
        let bar = AomNode::new(1, Role::MenuBar)
            .with_name("Main menu")
            .with_children(vec![item(2, "File", true), item(3, "Help", false)]);
        let root = AomNode::new(0, Role::Document).with_children(vec![bar]);
        let buffer = crate::buffer::VirtualBuffer::from_aom(&root);
        let speech: Vec<&str> = buffer.lines.iter().map(|l| l.speech.as_str()).collect();
        assert_eq!(
            speech,
            vec![
                "Main menu MenuBar with 2 items".to_owned(),
                format!(
                    "File{}{}{}",
                    role_marker("menu"),
                    state_marker("sub menu"),
                    state_marker("collapsed")
                ),
                format!("Help{}", role_marker("menu")),
                "MenuBar end".to_owned(),
            ]
        );
        assert!(buffer.tab_stops.is_empty());
    }

    /// GitHub's code view is a read-only text area; JAWS reads its lines with the virtual
    /// cursor (dev box, 2026-10-16), the first with its name and role.
    #[test]
    fn a_read_only_text_area_reads_by_lines() {
        let mut code = AomNode::new(1, Role::Edit)
            .with_name("file content")
            .with_value("[workspace]\nresolver = \"2\"\n\nmembers = []");
        for (key, value) in [("readonly", "true"), ("multiline", "true")] {
            code.attributes.insert(key.to_owned(), value.to_owned());
        }
        let root = AomNode::new(0, Role::Document).with_children(vec![code]);
        let buffer = crate::buffer::VirtualBuffer::from_aom(&root);
        let speech: Vec<&str> = buffer.lines.iter().map(|l| l.speech.as_str()).collect();
        assert_eq!(
            speech,
            vec![
                format!("file content{}[workspace]", role_marker("read only edit")),
                "resolver = \"2\"".to_owned(),
                String::new(),
                "members = []".to_owned(),
            ]
        );
        assert_eq!(buffer.edits.len(), 1);
    }
}

#[cfg(test)]
mod focus_stop_tests {
    use super::build;
    use crate::aom::{AomNode, Role};

    fn focusable(mut node: AomNode) -> AomNode {
        node.states.focusable = Some(true);
        node
    }

    /// Elements that take the focus without being controls (WhatsApp's messages: a row and
    /// the group inside it) are focus stops at their first line, the innermost of those
    /// starting on one line; a control there is a Tab stop already.
    #[test]
    fn focusable_plain_elements_are_focus_stops() {
        let page = AomNode::new(0, Role::Document)
            .with_name("Chat")
            .with_children(vec![focusable(
                AomNode::new(1, Role::TableRow).with_children(vec![focusable(
                    AomNode::new(2, Role::Group).with_children(vec![
                        AomNode::new(3, Role::Paragraph).with_text("hello"),
                        focusable(AomNode::new(4, Role::Button).with_name("13:14 Okundu")),
                    ]),
                )]),
            )]);
        let buffer = build(&page);
        let hello = buffer
            .lines
            .iter()
            .find(|l| l.text == "hello")
            .map(|l| (l.start_offset, 2));
        assert!(hello.is_some());
        assert_eq!(buffer.focus_stops, hello.into_iter().collect::<Vec<_>>());
    }
}
