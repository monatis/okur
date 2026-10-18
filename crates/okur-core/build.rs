//! Reads Okur's English message catalog (`locales/en-US/*.ftl`) and writes, into `OUT_DIR`,
//! the Rust side of it (`src/messages.rs` includes it):
//! - `MessageId`, one variant per message;
//! - `msg::<id>()`, one constructor per message, whose parameters are the message's
//!   variables, so a call site with a missing message or a missing variable does not compile;
//! - the catalog's files, compiled into the binary.
//!
//! A catalog that does not parse, a message without a value, two messages with one name, or a
//! variable that cannot be a Rust parameter stops the build with the reason.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use fluent_syntax::ast;

/// The language whose catalog defines the messages: JAWS's English.
const SOURCE_LOCALE: &str = "en-US";

/// Words Rust does not take as a parameter or function name.
const KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "crate",
    "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl",
    "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref",
    "return", "self", "static", "struct", "super", "trait", "true", "try", "type", "typeof",
    "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

struct Entry {
    id: String,
    file: String,
    /// The English text, for the documentation of its constructor.
    text: Vec<String>,
    /// Variables the message uses itself, in the order they first appear in its English text.
    variables: Vec<String>,
    /// Messages it refers to, whose variables it takes too.
    references: BTreeSet<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let dir = manifest.join("locales").join(SOURCE_LOCALE);
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "ftl"))
        .collect();
    files.sort();
    if files.is_empty() {
        return Err(format!("no .ftl files in {}", dir.display()).into());
    }

    let mut entries: Vec<Entry> = Vec::new();
    for path in &files {
        println!("cargo:rerun-if-changed={}", path.display());
        let file = file_name(path)?;
        let source = std::fs::read_to_string(path)?;
        let resource = match fluent_syntax::parser::parse(source.as_str()) {
            Ok(resource) => resource,
            Err((_, errors)) => {
                return Err(format!("{file}: the catalog does not parse: {errors:?}").into());
            }
        };
        for entry in &resource.body {
            match entry {
                ast::Entry::Message(message) => entries.push(read_message(message, &file)?),
                ast::Entry::Junk { content } => {
                    return Err(format!("{file}: not a message: {content:?}").into());
                }
                _ => {}
            }
        }
    }

    check_names(&entries)?;
    let variables = all_variables(&entries)?;
    let code = generate(&files, &entries, &variables)?;
    let out = PathBuf::from(std::env::var("OUT_DIR")?).join("messages.rs");
    std::fs::write(out, code)?;
    Ok(())
}

fn file_name(path: &Path) -> Result<String, Box<dyn Error>> {
    Ok(path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| format!("unreadable file name: {}", path.display()))?
        .to_owned())
}

fn read_message(message: &ast::Message<&str>, file: &str) -> Result<Entry, Box<dyn Error>> {
    let id = message.id.name.to_owned();
    let value = message
        .value
        .as_ref()
        .ok_or_else(|| format!("{file}: message {id} has no value"))?;
    let mut entry = Entry {
        id,
        file: file.to_owned(),
        text: Vec::new(),
        variables: Vec::new(),
        references: BTreeSet::new(),
    };
    let mut text = String::new();
    pattern(value, &mut entry, Some(&mut text));
    entry.text = text.lines().map(str::to_owned).collect();
    for attribute in &message.attributes {
        pattern(&attribute.value, &mut entry, None);
    }
    Ok(entry)
}

/// Collects a pattern's variables and message references; writes its text when asked.
fn pattern(pattern: &ast::Pattern<&str>, entry: &mut Entry, mut text: Option<&mut String>) {
    for element in &pattern.elements {
        match element {
            ast::PatternElement::TextElement { value } => {
                if let Some(text) = text.as_deref_mut() {
                    text.push_str(value);
                }
            }
            ast::PatternElement::Placeable { expression } => {
                if let Some(text) = text.as_deref_mut() {
                    text.push_str(&placeable_text(expression));
                }
                expression_names(expression, entry);
            }
        }
    }
}

fn placeable_text(expression: &ast::Expression<&str>) -> String {
    match expression {
        ast::Expression::Inline(ast::InlineExpression::VariableReference { id }) => {
            format!("{{ ${} }}", id.name)
        }
        ast::Expression::Inline(ast::InlineExpression::StringLiteral { value }) => {
            (*value).to_owned()
        }
        ast::Expression::Inline(ast::InlineExpression::MessageReference { id, .. }) => {
            format!("{{ {} }}", id.name)
        }
        _ => "{ … }".to_owned(),
    }
}

fn expression_names(expression: &ast::Expression<&str>, entry: &mut Entry) {
    match expression {
        ast::Expression::Select { selector, variants } => {
            inline_names(selector, entry);
            for variant in variants {
                pattern(&variant.value, entry, None);
            }
        }
        ast::Expression::Inline(inline) => inline_names(inline, entry),
    }
}

fn inline_names(inline: &ast::InlineExpression<&str>, entry: &mut Entry) {
    match inline {
        ast::InlineExpression::VariableReference { id } => {
            if !entry.variables.iter().any(|v| v == id.name) {
                entry.variables.push(id.name.to_owned());
            }
        }
        ast::InlineExpression::MessageReference { id, .. } => {
            entry.references.insert(id.name.to_owned());
        }
        ast::InlineExpression::FunctionReference { arguments, .. } => {
            call_names(arguments, entry);
        }
        ast::InlineExpression::TermReference { arguments, .. } => {
            if let Some(arguments) = arguments {
                call_names(arguments, entry);
            }
        }
        ast::InlineExpression::Placeable { expression } => expression_names(expression, entry),
        ast::InlineExpression::StringLiteral { .. }
        | ast::InlineExpression::NumberLiteral { .. } => {}
    }
}

fn call_names(arguments: &ast::CallArguments<&str>, entry: &mut Entry) {
    for argument in &arguments.positional {
        inline_names(argument, entry);
    }
    for argument in &arguments.named {
        inline_names(&argument.value, entry);
    }
}

/// The Rust name of a catalog name: `quicknav-none-links` is `quicknav_none_links`.
fn snake(name: &str) -> String {
    name.replace('-', "_").to_lowercase()
}

/// `quicknav-none-links` is `QuicknavNoneLinks`.
fn camel(name: &str) -> String {
    name.split(['-', '_'])
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect()
}

fn check_names(entries: &[Entry]) -> Result<(), Box<dyn Error>> {
    let mut seen: BTreeMap<String, &Entry> = BTreeMap::new();
    for entry in entries {
        for name in [snake(&entry.id), camel(&entry.id)] {
            if let Some(other) = seen.insert(name.clone(), entry) {
                return Err(format!(
                    "{} ({}) and {} ({}) both become {name} in Rust: rename one",
                    entry.id, entry.file, other.id, other.file
                )
                .into());
            }
        }
        if KEYWORDS.contains(&snake(&entry.id).as_str()) {
            return Err(format!("{}: message {} is a Rust keyword", entry.file, entry.id).into());
        }
        let mut params = BTreeSet::new();
        for variable in &entry.variables {
            let param = snake(variable);
            if KEYWORDS.contains(&param.as_str()) || !params.insert(param.clone()) {
                return Err(format!(
                    "{}: message {}: variable ${variable} cannot be a Rust parameter ({param}): rename it",
                    entry.file, entry.id
                )
                .into());
            }
        }
    }
    Ok(())
}

/// Each message's variables, with those of the messages it refers to.
fn all_variables(entries: &[Entry]) -> Result<BTreeMap<String, Vec<String>>, Box<dyn Error>> {
    let by_id: BTreeMap<&str, &Entry> = entries.iter().map(|e| (e.id.as_str(), e)).collect();
    let mut all = BTreeMap::new();
    for entry in entries {
        let mut variables: Vec<String> = Vec::new();
        let mut visited = BTreeSet::new();
        let mut pending = std::collections::VecDeque::from([entry.id.as_str()]);
        while let Some(id) = pending.pop_front() {
            if !visited.insert(id) {
                continue;
            }
            let referenced = by_id.get(id).ok_or_else(|| {
                format!(
                    "{}: message {} refers to {id}, which does not exist",
                    entry.file, entry.id
                )
            })?;
            for variable in &referenced.variables {
                if !variables.contains(variable) {
                    variables.push(variable.clone());
                }
            }
            pending.extend(referenced.references.iter().map(String::as_str));
        }
        all.insert(entry.id.clone(), variables);
    }
    Ok(all)
}

fn doc(out: &mut String, indent: &str, lines: &[String]) -> std::fmt::Result {
    writeln!(out, "{indent}#[doc = \"```text\"]")?;
    for line in lines {
        writeln!(out, "{indent}#[doc = {line:?}]")?;
    }
    writeln!(out, "{indent}#[doc = \"```\"]")
}

fn generate(
    files: &[PathBuf],
    entries: &[Entry],
    variables: &BTreeMap<String, Vec<String>>,
) -> Result<String, Box<dyn Error>> {
    let mut out = String::new();
    writeln!(
        out,
        "// Generated by build.rs from locales/{SOURCE_LOCALE}/*.ftl. Do not edit."
    )?;
    writeln!(out)?;
    writeln!(out, "/// The English catalog's files, compiled in.")?;
    writeln!(out, "const ENGLISH_SOURCES: &[(&str, &str)] = &[")?;
    for path in files {
        let name = file_name(path)?;
        let full = path.to_str().ok_or("unreadable path")?;
        writeln!(out, "    ({name:?}, include_str!({full:?})),")?;
    }
    writeln!(out, "];")?;
    writeln!(out)?;

    let empty = Vec::new();
    let vars = |entry: &Entry| variables.get(&entry.id).unwrap_or(&empty);

    writeln!(out, "/// A message of Okur's catalog, by name.")?;
    writeln!(
        out,
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]"
    )?;
    writeln!(out, "pub enum MessageId {{")?;
    for entry in entries {
        writeln!(out, "    /// `{}` ({}):", entry.id, entry.file)?;
        doc(&mut out, "    ", &entry.text)?;
        writeln!(out, "    {},", camel(&entry.id))?;
    }
    writeln!(out, "}}")?;
    writeln!(out)?;

    writeln!(out, "impl MessageId {{")?;
    writeln!(out, "    /// Every message, in catalog order.")?;
    writeln!(out, "    pub const ALL: &'static [MessageId] = &[")?;
    for entry in entries {
        writeln!(out, "        MessageId::{},", camel(&entry.id))?;
    }
    writeln!(out, "    ];")?;
    writeln!(out)?;
    writeln!(out, "    /// The message's name in the catalog.")?;
    writeln!(out, "    pub const fn id(self) -> &'static str {{")?;
    writeln!(out, "        match self {{")?;
    for entry in entries {
        writeln!(
            out,
            "            Self::{} => {:?},",
            camel(&entry.id),
            entry.id
        )?;
    }
    writeln!(out, "        }}")?;
    writeln!(out, "    }}")?;
    writeln!(out)?;
    writeln!(out, "    /// The catalog file the message is in.")?;
    writeln!(out, "    pub const fn file(self) -> &'static str {{")?;
    writeln!(out, "        match self {{")?;
    for entry in entries {
        writeln!(
            out,
            "            Self::{} => {:?},",
            camel(&entry.id),
            entry.file
        )?;
    }
    writeln!(out, "        }}")?;
    writeln!(out, "    }}")?;
    writeln!(out)?;
    writeln!(
        out,
        "    /// The variables the message takes, as the catalog names them."
    )?;
    writeln!(
        out,
        "    pub const fn variables(self) -> &'static [&'static str] {{"
    )?;
    writeln!(out, "        match self {{")?;
    for entry in entries {
        let list: Vec<String> = vars(entry).iter().map(|v| format!("{v:?}")).collect();
        writeln!(
            out,
            "            Self::{} => &[{}],",
            camel(&entry.id),
            list.join(", ")
        )?;
    }
    writeln!(out, "        }}")?;
    writeln!(out, "    }}")?;
    writeln!(out)?;
    writeln!(out, "    /// The name of its constructor in [`msg`].")?;
    writeln!(out, "    pub const fn constructor(self) -> &'static str {{")?;
    writeln!(out, "        match self {{")?;
    for entry in entries {
        writeln!(
            out,
            "            Self::{} => {:?},",
            camel(&entry.id),
            snake(&entry.id)
        )?;
    }
    writeln!(out, "        }}")?;
    writeln!(out, "    }}")?;
    writeln!(out, "}}")?;
    writeln!(out)?;

    writeln!(
        out,
        "/// One constructor per message of the catalog. A message without variables is a"
    )?;
    writeln!(
        out,
        "/// [`StaticMessage`]; one with variables takes their values, in the order they"
    )?;
    writeln!(
        out,
        "/// first appear in the English text, and is a [`Message`]."
    )?;
    writeln!(out, "pub mod msg {{")?;
    writeln!(out, "    #[allow(unused_imports)]")?;
    writeln!(
        out,
        "    use super::{{Arg, Message, MessageId, StaticMessage}};"
    )?;
    for entry in entries {
        writeln!(out)?;
        writeln!(out, "    /// `{}` ({}):", entry.id, entry.file)?;
        doc(&mut out, "    ", &entry.text)?;
        let name = snake(&entry.id);
        let variant = camel(&entry.id);
        let vars = vars(entry);
        if vars.is_empty() {
            writeln!(out, "    pub fn {name}() -> StaticMessage {{")?;
            writeln!(out, "        StaticMessage::new(MessageId::{variant})")?;
        } else {
            let params: Vec<String> = vars
                .iter()
                .map(|v| format!("{}: impl Into<Arg>", snake(v)))
                .collect();
            let args: Vec<String> = vars
                .iter()
                .map(|v| format!("({v:?}, {}.into())", snake(v)))
                .collect();
            writeln!(
                out,
                "    pub fn {name}({}) -> Message {{",
                params.join(", ")
            )?;
            writeln!(
                out,
                "        Message::new(MessageId::{variant}, vec![{}])",
                args.join(", ")
            )?;
        }
        writeln!(out, "    }}")?;
    }
    writeln!(out, "}}")?;
    Ok(out)
}
