# Okur's words and verbosity

A developer note on milestone M0 of the extension SDK proposal
([`extension-sdk-proposal.md`](extension-sdk-proposal.md) §6.1, §6.2, §6.5, §12): where
Okur's own words live, how to add one, how announcement parts carry kinds, and how verbosity
switches output types. The SDK itself is not built.

## The message catalog

Everything Okur says or shows in its own words comes from one catalog, in
[Project Fluent](https://projectfluent.org) syntax, compiled into the binary:

| File (`crates/okur-core/locales/en-US/`) | What it holds |
|---|---|
| `roles.ftl` | role words: "Button", "check box", "List view" |
| `states.ftl` | state words: "checked", "expanded", "Current Page" |
| `hints.ftl` | usage hints: "To activate press Enter." |
| `navigation.ftl` | quick navigation: "There are no links on this page." |
| `page.ftl` | the virtual cursor's container lines: "list of 3 items", "main region end" |
| `tables.ftl` | tables: "Column 2, Row 3", "Not in a table." |
| `lists.ftl` | the element lists' openings: "Links List" |
| `commands.ftl` | what commands say: "Caps lock on", "Title is", the Virtual Viewer |
| `speech.ftl` | other messages: positions, Say Font, selections, native controls |
| `apps.ftl` | the application modules' words: "desktop", "Not Selected { $name }" |
| `dialogs.ftl` | Okur's own dialogs' labels: "&Voice:", "&Move To Link" |

What an application gives (names, values, page text) is never translated: it is spoken as
given, as an argument of a message or a part of its own.

### Adding a message

1. Write it into the file of its area, with an id that names the situation, not the words:
   `quicknav-none-links = There are no links on this page.` A value in it is a variable:
   `position = { $index } of { $total }`.
2. Keep it whole. Never build "There are no " + element + " on this page.": write one message
   per element kind.
3. Write JAWS's English exactly, quirks included: the frames message has no period
   (`quicknav-none-frames`), "level1" has no space (`quicknav-heading-level-none-top`).
4. Put a translator comment (`# …`) above it when the id and words do not say where it is
   heard. A `##` comment heads a group.
5. Call its constructor: `msg::quicknav_none_links()`, `msg::position(3, 7)`. `build.rs` writes
   one constructor per message; its parameters are the message's variables in the order they
   first appear in the English text. A misspelled id or a missing variable does not compile.

A message without variables is a `StaticMessage`: `.as_str()` gives `&'static str` (rendered
once, kept), and `Display` writes it. One with variables is a `Message`: `.render()` or
`Display`. Numbers are `Arg::Number`, so a language's plural rules can choose words.

Rules the catalog keeps:
- A message never starts or ends with spaces (Fluent drops them) and never holds JAWS's role
  and state marks (`\u{1d}`, `\u{1e}`): the code that renders speech adds both.
- Role and state words take a suffix where JAWS's English differs by situation:
  `role-button` ("Button", focus speech), `role-button-say-window` ("button").
- Fluent's bidirectional isolation marks are off: they would reach the synthesizer.

### The guards

`crates/okur-core/tests/catalog.rs`:
- **No word outside the catalog.** It reads the non-test Rust of `okur-core` and `okur-ui` as
  tokens and fails on a string literal with words that is neither in the catalog nor listed
  in `crates/okur-core/tests/literals.txt`. A failure prints the line to add when the literal
  is an identifier (an attribute key, a class name, a name matched against what Windows
  reports). The list has four sections: `identifiers`, `locale-data` (the English locale
  `clock.rs` falls back to), `symbols` (symbol names, which become per-language data with
  punctuation levels, proposal §6.3) and `pending` (words still to move). A listed literal
  that no longer exists fails too.
- **No unused message.** A message whose constructor no code outside tests calls fails.
- **The pseudo-locale run.** Under `messages::with_pseudo_locale` every message comes out as
  `⟦…⟧`. The test reads a page and native controls and fails on any word that is neither
  wrapped nor the page's own.

`crates/okur-core/src/messages.rs` tests that every message renders without error and holds
no mark.

### Proving English did not change

Moving words into the catalog changes no speech. Before and after a change:
`bash skills/jaws-reference/scripts/parity-snapshot.sh <dir>` (with the release build's
`okur.exe` as its second argument when it is not under `target/agent/`), then
`diff -r before after`, which must be empty. The committed parity baselines must stay valid
without regeneration.

## Announcement parts and their kinds

`crates/okur-core/src/announcement.rs`: an `Announcement` is lines of `Part`s; a part is a
`PartKind` and its words. Kinds are those of proposal §6.1: Name, Value, Role, State,
Position, Level, Description, Hint, Shortcut, AccessKey, Container, Message, Text. Roles and
states render with JAWS's marks; every other kind renders as its words.

Where parts have kinds today:
- native focus speech (`native_focus.rs`), the new window's heading (`engine/focus.rs`) and
  File Explorer's list entry (`apps/windows/explorer.rs`);
- the web focus phrasing: `SpeechFormatter::control_focus`, `edit_combo` and `link_focus`
  build the same text from typed parts. A line whose spacing parts cannot give exactly (a
  check box's state without a trailing space, a button's states) is one `Text` part.

`Announcement::retain` keeps parts by kind. A test shows that dropping the AccessKey and
Shortcut parts of a native focus says what the focus says without those keys.

## Verbosity

`crates/okur-core/src/settings.rs`, `Settings::verbosity`, modeled on JAWS (proposal §6.5):
- `level`: Beginner (default), Intermediate or Advanced;
- `output_modes`: per output type, one mode (Off, On) per level, Beginner first; a type not
  listed is on at every level, JAWS's default;
- `message_length`: Long or Short per level, Long at Beginner and Short above (`SpeechLevel`
  in JAWS's shipped `Default.JCF` `[OutputModes]`: `1|2|2`). A message's short form is a
  catalog entry of its own, `<id>-short`, learned from the Intermediate recordings (572-581):
  "virtual PC" for "virtual PC Cursor", a tree level's number for "level 1", "3 columns and
  3 rows" for "table with 3 columns and 3 rows", "Last row" for "You are on the last row in
  the table."; "Open List Box" and Insert+T's "Title is" have none (they are left out).
  Round 259 (827-858, the web keys at both levels) added "No more headings at level2 in
  section" (no "this", no period), "No onMouseOver elements", Insert+Z's "Off" and "On", and
  the page's summary without "Page has" ("5Regions, 5headings and 12links"); Insert+Escape's
  "Screen updated" is a message of JAWS's own, left out at Advanced.
  Round 263 (865-912, 24 more web scenarios at both levels) added the quick keys'
  nothing-found messages, "No PlaceMarkers", "No clickable elements" (every "There are no ... on
  this page." of the family has its `-short`, reached through `StaticMessage::short`),
  Insert+F5's "no form fields" (Insert+F7's and Insert+Ctrl+R's keep their long forms), a
  table's edge left out at Advanced ("end of row"), and, at Advanced, a toolbar entered said as
  a group name and left out with it. In an
  announcement a part carries its short form (`Part::short`, `Line::short`), and the engine's
  verbosity filter says it ([`Announcement::shorten`]); text built outside announcements
  (the virtual cursor's table keys) asks `message_length` itself. A message without a recorded
  short form keeps its long one at every level.

Output types are Okur's ids (`OutputType`), each mapped to JAWS's `DEFAULT.JCF`
`[OutputModes]` key where one is known:

| Okur id | JAWS key | Parts | Today |
|---|---|---|---|
| `access_key` | `ACCESS_KEY` (`0\|0\|0\|Access Key`) | AccessKey, Shortcut | off on old-pc (shipped `1\|1\|1`) |
| `hint` | not known yet | Hint | on |
| `list_position` | `ANNOUNCE_POSITION_AND_COUNT` (shipped `1\|2\|0`) | ListPosition | off at Advanced: a list's item, a combo box's open list (548-549, 590) |
| `description` | not known yet | Description | off at Advanced (552-553) |
| `control_group_name` | `CONTROL_GROUP_NAME` (shipped `1\|2\|0`) | GroupName | off at Advanced: a dialog control's group box (585) |
| `select` | `SELECT` (shipped `1\|2\|0`) | (strings, `editing::Wording`) | off at Advanced: "selected" and "unselected" as a selection changes (608, 609) |
| `okur_message` | `JAWS_MESSAGE` (shipped `1\|2\|0`) | (strings, `editing::Wording`) | short above Beginner ("Copied", "Pasted", "cut", "Appended"), off at Advanced, Undo too (599, 608, 609, 859-862); also off at Advanced: Insert+Escape's "Screen updated" (842/s13) and a table's edge, "end of row", "bottom of column" (870/s4, s7) |

JAWS's shipped `Default.JCF` (`C:\ProgramData\Freedom Scientific\JAWS\2025\SETTINGS\enu`)
lists the other types with their modes per level (0 off, 1 long, 2 short): `CONTROL_NAME`,
`CONTROL_TYPE`, `DIALOG_NAME`, `DIALOG_TEXT`, `ITEM_STATE`, `POSITION`, `STATUS`, `TUTOR`
and more are `1|2|2` or `1|1|1`, on at every level; `SMART_HELP` and `APP_START` are `1|0|0`,
`JAWS_MESSAGE` and `SELECT` `1|2|0`. A type joins Okur's when a recording shows what it
switches.

How it is applied:
- the engine says the parts of a native focus (and an application module's announcement)
  that `Verbosity::says(kind)` keeps;
- access keys also leave the focus event before speech is built, since menus speak strings,
  not parts;
- parity replay (`crates/okur-corpus/src/parity.rs`, `replay_settings`) fills the model from
  each evidence file's recorded `DEFAULT.JCF` `[OutputModes]`; `Settings::reference_machine()`
  turns `access_key` off at every level.

Adding an output type: a variant of `OutputType`, its JCF key in `jcf_key` when known, the
part kinds it governs in `of_part`, and nothing else: it is on until JAWS's settings say
otherwise. JAWS's list of types and their defaults is learned on the reference machine
(proposal §6.5): read the items of JAWS's verbosity dialog, switch each, and compare
`DEFAULT.JCF` before and after.

Open question: the proposal reads `ACCESS_KEY=0|0|0` as one mode per level; an earlier comment
in `parity.rs` read the fields as speech, braille and more. Both give the same behavior for
`0|0|0` and at the Beginner level. Which is right is to be confirmed on old-pc.

## What M0 still has to do

- `okur-ui` shows two words of its own: the Virtual Viewer's window title at creation
  (replaced by the engine's catalog title when it shows) and its status bar's "Ready". Moving
  them needs the status text in `ToUi::ViewerShow` and a live check of the window; they are
  listed as pending in `tests/literals.txt`.
- Role marks are still written by hand in `SpeechFormatter`, `linearize.rs` and a few engine
  lines; the virtual buffer's lines keep finished strings (`BufferLine::speech`). M0's "no role
  mark written outside the renderer" needs the structured page line and a decision on JAWS's
  spacing (structure plan step 4).
- Voice contexts (§6.4) are not started.
- Symbol names (`pronounce.rs`) and the clock's English fallback stay in code, by design, until
  per-language data exists (§6.3).
- Application modules still match Windows' English names ("Taskbar", "Items View", "File
  Explorer"); identifying them by what they are is a separate task.
