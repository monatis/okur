# Okur

[![CI status](https://github.com/monatis/okur/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/monatis/okur/actions/workflows/ci.yml)
[![License: Apache 2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust 1.99.0](https://img.shields.io/badge/rust-1.99.0-orange.svg?logo=rust)](rust-toolchain.toml)
[![Platforms: Windows; macOS in progress](https://img.shields.io/badge/platforms-Windows%20%7C%20macOS%20%28in%20progress%29-lightgrey.svg)](#platforms)
[![JAWS parity: 9051 of 9625 recorded steps](https://img.shields.io/badge/JAWS%20parity-9051%2F9625%20steps-brightgreen.svg)](#where-it-stands)

**An open screen reader that keeps your JAWS muscle memory.**

Okur ("reader" in Turkish) is an open-source screen reader written in Rust. It speaks,
navigates and behaves like **JAWS 2025**: the same keystrokes, the same phrasing, the same
reading of web pages and Windows applications. It is built by AI agents in a **clean room**:
they never see JAWS's code, only what JAWS says when keys are pressed on a separate lab
machine, and nothing they build counts until it matches recorded JAWS speech.

## Why re-implement JAWS?

Blind people use the computer with the keyboard. With daily use, the keystrokes become
instant: Insert+F7 for the list of links, H for the next heading, Insert+T for the window
title. Nobody thinks about them, just as sighted people don't think about using the mouse.

Now suppose someone changed your mouse overnight, and it behaved differently from what you
were used to. That is how changing screen readers feels after years, often decades, of daily
use. The alternatives are good, but they use other keys, other phrasing and other ideas of
where the cursor is. So people stay with what they know, and for many that is JAWS: a
commercial product, three decades old, that runs only on Windows.

Okur keeps the muscle memory and replaces everything underneath:
- **Open source**, under the Apache License 2.0.
- **Memory-safe and fast**, written in Rust.
- **Built to run on Windows and macOS** as first-class platforms, with Linux as a second tier.

The keystrokes, the announcements and the navigation model are the interface people rely on,
so they follow JAWS, checked step by step against recordings of it. Everything behind them is
our own.

Where to go from here:
- **If you use a screen reader** and want to try Okur, go to
  [For screen reader users](#for-screen-reader-users).
- **If you are a developer** and want to contribute, go to [For developers](#for-developers).
- **If you are curious** how AI agents can build a screen reader, read
  [How AI agents build it](#how-ai-agents-build-it) further down.

## Where it stands

> **Status (October 2026): a preview on Windows.** Web pages in Chrome (the Virtual PC
> Cursor, the lists of links, headings and other elements, tables, selecting and copying),
> Microsoft Word's documents, dialog boxes and menus, Notepad, File Explorer, Windows
> Settings, Windows Terminal, the desktop, Alt+Tab and Windows search are read as JAWS reads
> them, and the Okur cursor reads any window's screen, pictures included, recognized with
> Windows' own OCR. It is not yet a replacement for your screen reader: Outlook and Excel are
> the largest part still missing (see [Not yet](#not-yet)). Those everyday gaps on Windows are being closed first; macOS, whose
> accessibility layer is in place, comes after them.
>
> **Okur speaks in its own name.** Where JAWS names itself, Okur says Okur: the Okur key
> (JAWS's JAWS key), the Okur cursor, Okur Find. The words around the name stay the ones you
> know.

Every figure is tool output, rerun for this README at commit `80a78aeb` (2026-10-25).

| Measure | Result | Command |
|---|---|---|
| Steps matching JAWS's speech history, word for word | **9051 of 9625** | `okur-lab parity` |
| Steps matching what JAWS sent its synthesizer, pitch included | **8963 of 9686** | `okur-lab parity` |
| Steps left out on purpose (accepted divergences, each a written decision) | **259** | `okur-lab parity` |
| Recorded JAWS scenarios, trials | **680, 2046** (2029 trials succeeded) | `corpus/CORPUS_MANIFEST.json` |
| Tests | **1510 passed, 0 failed** (200 ignored: they need a live desktop, or assert a known gap) | `cargo test --workspace` |
| Virtual buffer invariant violations | **0** | `okur-lab invariants` |
| `unwrap`, `expect`, `panic!` in the screen reader's own crates | **0**, tests included | the commit gate and CI |

The 251 steps left out are where Okur differs from JAWS on purpose. Some are JAWS's own
accidents, which Okur does not copy: in a data grid, JAWS adds "Column 65535, Row 65535", a
place it failed to find. Others are JAWS's help texts, which are Freedom Scientific's writing;
Okur says its help in its own words. Each one is listed, with the reason and who decided it,
in [accepted divergences](docs/references/accepted-divergences.md).

---

## For screen reader users

### What works today (Windows)

- **Web pages in browse mode** with the Virtual PC Cursor:
  - reading by character, word, line, sentence and paragraph; Home, End, PageUp and
    PageDown; Say All by line, sentence or paragraph (Say All Reads, in Okur Settings), where
    Right and Left skip and go back by that unit, PageUp and PageDown change the rate, and the
    cursor stays where you stopped it; skim reading (Ctrl+Insert+Down), each paragraph by its
    first line or its first sentence, the lines, sentences or paragraphs a text rule finds, or
    the lines with bold, italic, underlined, struck, raised or lowered text or text in a color,
    as chosen in Skim Reading (Ctrl+Shift+Insert+Down), whose Create Summary lists them, each a
    link to its place;
  - deleted text said between "Deletion" and "end Deletion", and a raised or lowered number
    after a number between "superscript" or "subscript" and their end ("10 superscript 23");
  - the Word List (Ctrl+Insert+W): the page's words, most used first, W and Shift+W to the
    next and prior line with the word chosen, and its Summary: the sentences holding the word,
    each a link to its place;
  - quick navigation keys: headings and their levels, links, form fields, edits, buttons,
    check boxes, radio buttons, combo boxes, lists and list items, tables, graphics, regions,
    articles, divisions, separators, tabs and mail links; N past a run of links; the first
    and last heading or form field; V and U to the next visited and unvisited link; a
    temporary PlaceMarker (Ctrl+Windows+K) and saved ones to come back to with K and Shift+K,
    and 7 to 0 for the first four; Shift+Comma and Shift+Period
    to the start and end of a list, table or region; Equals to where the page says reading goes
    next (aria-flowto); S and D to the next element of the same or of another type (and
    Ctrl+Insert with Up, Right and Left); Jump to Line (J, and Shift+J back);
  - the lists of links, headings and form fields (Insert+F7, Insert+F6, Insert+F5), Virtual
    HTML Features (Insert+F3), the element lists on Ctrl+Insert and a letter (buttons, edits,
    check boxes, radio buttons, combo boxes, graphics, lists, articles, paragraphs, block
    quotes, tables, divisions and tabs), and Document Regions (Insert+Ctrl+R);
  - the PlaceMarker List (Ctrl+Shift+K): PlaceMarkers added with the line at the cursor as
    their name, kept for the page (or every page of its site) from one visit to the next,
    moved to, renamed and removed; custom labels (Insert+Ctrl+Tab): a name of your own for a
    field, button, link or graphic, read in its place on the site's pages;
  - tables: Ctrl+Alt and the arrows, a whole row or column with Windows+Alt and the arrows,
    and the table layer (Insert+Space, then T);
  - selecting with Shift and the arrows, Ctrl+A or F8, and copying with Ctrl+C;
  - Okur Find (Ctrl+F or Insert+Ctrl+F, then F3 and Shift+F3), with its history kept per
    site, as JAWS Find keeps it;
  - forms mode, turned on by Tab or the arrows (Auto Forms Mode), by Tab only (Semi-Auto) or
    with Enter (Manual), as Okur Settings chooses, and off with Escape or Numpad Plus; Space
    clicks the control at the cursor, as JAWS does;
  - routing the cursors (Insert+Delete, Ctrl+Insert+Delete), a click with Numpad Slash, a
    link's address (Ctrl+Insert+F7), and the virtual cursor and the quick keys turned off and
    on (Insert+Z, Insert+N);
  - a page that loads says its title and a summary of its regions, headings and links, then
    is read from the top, as JAWS does by default (Document and Web Pages Automatically Read
    When Loaded, in Okur Settings, turns that reading off);
  - frames, and single-page apps changing route; Tab moves on from where you are reading.
- **Web apps:** autocompletes and command palettes read their suggestions ("Apple, 1 of 3");
  regions with role "application" (such as VS Code's) get every key; GitHub's code view reads
  line by line; Google Docs (with its braille support turned on) reads as a document, Google
  Sheets by cell, and WhatsApp Web's chat list and messages as lists.
- **Live regions** announced as JAWS announces them. A streamed chat reply is read once, as
  it arrives, not again from the start on every update; `--live-region-whole-text` gives
  JAWS's own behavior.
- **Windows applications:**
  - dialog boxes: Say Window (Insert+B) with their group boxes, the default button (Insert+E),
    a control's hot key (Shift+Numpad 5), a property sheet's page on Ctrl+Tab;
  - menus, the menu bar and context menus; lists, list views and their columns (Ctrl+Insert
    and a number), trees, combo boxes and data grids;
  - Notepad and its dialogs, WordPad's and Paint's ribbons (Alt to the tabs, their places and
    the lower ribbon's groups, as JAWS reads them), File Explorer, Windows Settings,
    Windows Terminal, the desktop, the Alt+Tab switcher and Windows search;
  - the Window List (Insert+F10), to switch to another window, and the system tray list
    (Insert+F11), to open or click an icon.
- **Editing** in text fields: characters, words, lines, sentences and paragraphs; selections
  said as JAWS says them, and Say Selected Text (Insert+Shift+Down); cut, paste and undo; a
  field of several lines read as a document; the window's top and bottom line (Insert+End,
  Insert+PageDown); Say Font; misspelled words marked; and text analysis on Insert+Tab, as
  JAWS does it (runs of spaces, stray or unspaced punctuation, a missing capital, brackets and
  quotes left open, each with its column).
- **Microsoft Word:** a document read as JAWS reads it: each line as Word lays it out, a
  heading with its level ("heading level 1 Library Hours"), a list entered and left ("Entering
  List", "exit list"), a table entered, its cells and left ("table 1, uniform Table, new
  cell"). In a table, JAWS's table keys (Ctrl+Alt+arrows, Home and End) go from cell to cell
  and say its column's or row's title, Tab moves and selects as Word does, and Ctrl+Alt+Numpad
  5 and Insert+Tab say the cell and where it is. A table's column titles come from its first
  row, its row titles from its first column where the table marks it ("Row 1 no title" where
  not). Say All (Insert+Down) reads the document paragraph by paragraph and says what changes
  as it goes: a heading's level, "Entering List", "exit list", "table 1, uniform Table", "Row
  2", "out of table". Insert+F6 lists the document's headings to move to; Say Font gives bold,
  italic and underline ("bolded", "italicized"), the size, colors, font, style, alignment,
  outline level and line spacing ("Line spacing: Double"); Ctrl+Alt+N, O and P say the view they switch to ("Draft View"), and Insert+T
  the title and the view. Its ribbon is read as JAWS reads it: Alt to its tabs ("Upper Ribbon,
  Ribbon, Home Tab"), Down into a tab's commands with their group's name, each command's
  ScreenTip where it says more than the name ("Grow Font Button, Increase Font Size"), toggle
  buttons and check boxes with their state ("Format Painter Button not checked"), and Escape
  back ("Leaving ribbons, Edit"). The Links List (Insert+F7) lists the document's hyperlinks,
  Move To Link moves to one and Activate Link follows it; List Fields (Insert+F5) lists its
  fields ("HYPERLINK opening hours", "FORMTEXT") and OK moves to one; with none, each says so
  in Word's words. The line spacing and the fields come from Word's own object model.
- **The cursors:** Numpad Minus to the Okur cursor at the mouse, Numpad Plus back to the PC
  cursor, Insert+Numpad Minus to bring the Okur cursor to the PC cursor, Insert+Numpad Plus
  the other way, Insert+R for where the Okur cursor may go, and Alt+Delete for where the
  active cursor is. The Okur cursor reads a page as its screen shows it, a dialog's rows, and
  any other window by its screen rows, where UI Automation places its text. In WPF, UWP and
  WinUI windows it is the Okur scan cursor, one element a line with its role and state, as
  JAWS's Scan Cursor reads them. It moves by line, word and character, with Page Up and Page
  Down to the window's top and bottom, and the mouse goes with it.
- **The screen's pictures, recognized** (Convenient OCR): Insert+Space, O, then W, S or C
  recognizes the window, the screen or the focused control with Windows' own OCR. Okur says
  "OCR started using Microsoft OCR.", "finished." and "Okur Cursor". The recognized words then
  stand among the window's lines where they are drawn, Numpad Slash clicks them, and they go
  when the PC cursor comes back. B keeps recognizing the window as it changes (Background
  OCR), and Q stops it.
- **Pictures and scanned PDFs, recognized:** in File Explorer, Insert+Space, O, F (or Enter)
  reads the selected picture or PDF, page by page, into the Results Viewer, titled by the file;
  nothing selected, or a file that is neither, gets JAWS's message box. ? lists the layer's
  keys.
- **The mouse:** Numpad Slash and Numpad Star click the left and right buttons, with Ctrl,
  Shift or Alt for a click with that key held; Insert+Numpad Slash and Star hold a button down
  and let it go; Ctrl+Insert+Numpad Slash drags and drops; Alt+Shift and the arrows move the
  mouse a little. Mouse echo says what the mouse rests on, by character, word, line or
  paragraph, set in Okur Settings or with Insert+Space, E (O turns it on and off, C, W, L and P
  choose the unit).
- **Information keys:** the time and date (Insert+F12), the settings in use (Insert+Q), the
  application's version (Ctrl+Insert+V), and the colors of a page's text or a text field by the
  names JAWS gives them (Insert+5: "blue2 on white").
- **Help** in Okur's own words: Insert+F1 for the control or the page element you are on,
  Insert+H for Okur's keys there, Insert+W for Windows' keys, and Keyboard Help (Insert+1),
  where each key is named and described instead of run.
- **The classic console** (Command Prompt, PowerShell): a command's output is read as the
  console writes it (Screen Echo, Insert+S, as JAWS reads it); Windows Terminal's not yet.
- **The Virtual Viewer** (Insert+Alt+W) on screen as JAWS shows it, for windows, controls and
  consoles, Windows Terminal included; the Speech History (Insert+Space, then H; Ctrl+H
  copies it and Shift+H clears it).
- **The clipboard keys**: Windows+Insert+X says the clipboard's text, Insert+Space, then C
  shows it in a viewer, and Insert+Windows+C adds the selection to it; Ctrl+C or Ctrl+X after
  that asks before overwriting it, as JAWS does.
- **Windows' notifications** (toasts), and applications' own (as Teams sends them), said as
  they come, whatever is in front; Insert+Space, then Shift+N says the last one again,
  Insert+Space, then Ctrl+N stops and starts saying toasts, and Insert+Space, then N opens the
  Notification History, each listed with the program it came from, with the rules that mute a
  notification, say a shorter text or play a sound in its place, or leave it out of the
  history (Create Rule..., Manage Rules...).
- **Okur's own dialogs** are real Windows dialogs: the lists above, Okur Find and the Window
  List, with the same controls and labels as JAWS's, and Okur's settings, below.
- **Settings, as JAWS's:**
  - Settings Center (Insert+6) and Quick Settings (Insert+V) are trees of JAWS's groups with a
    search box (Ctrl+E). Space changes a setting and says its new value. Changes are kept for
    all applications, the application in front or the page's site. Settings Center asks
    before Cancel drops a change; Quick Settings can keep one only until Okur exits or the
    focus moves (Shift+F10). Their last group, Last Changed Settings, holds the latest changes
    from one run to the next (Shift+F10: Remove, Remove All).
  - Okur's window (Insert+J) has JAWS's menus: Options for Basics, Voices, Manage Application
    Settings (an application's settings turned off or removed) and Restore to Factory
    Settings; Utilities for Settings Center and the Notification History; Help for the Startup
    Wizard, the most common settings a page at a time, which also opens the first time Okur
    starts.
- **Speech:** punctuation levels and symbol names as JAWS has them (Insert+Shift+2); typing
  echo by characters, words, both or none (Insert+2); a reading key pressed twice quickly
  spells, with capitals at a higher pitch; the rate on the fly (Ctrl+Alt+PageUp and
  PageDown); Speech On Demand (Insert+Space, then S), where only the reading commands speak,
  and muted speech (Insert+Space, then Shift+S), as JAWS has them, what is not spoken still
  written to the Speech History.
- **Settings** declared in one table, so the settings dialog, the command line (`okur run --set
  <setting>=<value>`) and the import of JAWS's own settings files offer the same values. Among
  them, as JAWS has them: Caps Indicate, mixed case, numbers read as digits, dollar amounts
  left to the voice, same page links, graphics shown or not, how a heading is said, and which
  parts of a control verbosity says (its type, its state, its position, its hints).
- **Flexible Web** (Insert+Space, then X): JAWS's wizard, page for page, hides an element of a
  page (a region, a heading, a list) found by its kind, its text, its id or its place; Undo
  takes it back. Customizations are saved as named rules for the page, its site or every
  site, and hold whenever such a page loads; leaving a page with unsaved ones asks first.
  View or Change Where Rules are Applied turns each rule on or off for an area and deletes
  rules. Starting the reading at an element, and renaming a rule, are not done yet.
- **Verbosity levels:** Beginner, Intermediate and Advanced, as JAWS has them. Above
  Beginner Okur says the short messages JAWS says there ("Last row", "3 columns and 3 rows",
  "virtual PC", "Copied", "Appended", "Off", "No more headings at level2 in section", the
  page's summary without "Page has"); at Advanced it leaves out what JAWS leaves out, such as a
  list's positions, a field's description, a group box's name and JAWS's own messages
  ("Copied", "Screen updated"). Recorded at both levels on the web, in desktop dialogs, with
  the Okur cursor, in Notepad and in the Notification History. Each level's spoken items and
  message length are chosen in Okur Settings' Verbosity Levels, as JAWS's Settings Center
  customizes a level.
- **Quitting with Insert+F4,** after asking first, as JAWS does (Enter quits, Escape stays).
- **Both keyboard layouts,** desktop and laptop, with Insert or CapsLock as the Okur key
  (JAWS's JAWS key).
- **OneCore and SAPI 5 voices,** with JAWS's voice contexts and profiles: the PC cursor, the
  Okur cursor, the keyboard and Okur's own messages each speak with their own person, rate,
  punctuation and pitch (Voice Adjustment); a voice is saved as a profile by name, and Select a
  Voice Profile (Insert+Ctrl+S) chooses one for all applications or for the one in front.

The [reference documentation](docs/references/) describes these behaviors in detail.

### Not yet

The [everyday-use gaps](docs/backlog/everyday-use-gaps-2026-10-19.md) list what JAWS's
documentation describes and Okur lacks, most important first. Among them:
- Outlook and Excel, not yet supported or checked; in Word, a form field is read as text, not
  as the edit or check box JAWS reads it as ("Name: Edit");
- starting by itself at sign-in and on Windows' sign-in screen: the installer and a build
  with UI Access are written and are being tested with a development signature on the lab
  machine, before a signed release;
- on macOS, the keyboard and speech.

Braille and Picture Smart come later, and from Convenient OCR: a PDF open in a reader (D), a
scanner or camera (A), and the Results Viewer's Open in Word.

### Trying it

There are no ready-made downloads yet: Okur is built from its source code.

1. Install [Rust](https://www.rust-lang.org/tools/install) and the Visual Studio C++ build
   tools it asks for, on Windows 10 or 11.
2. In a command prompt in this folder, build it:

   ```
   cargo build --release --bin okur-lab --bin okur-ui
   ```

   `okur-ui` draws Okur's dialogs, such as the Links List and the settings. Okur starts it
   from its own folder, where the build puts both; without it Okur still speaks, but its
   windows do not show.

3. Run it from the same prompt. Okur speaks as soon as it starts:

   ```
   target\release\okur run
   ```

   With a laptop keyboard (CapsLock as the Okur key):

   ```
   target\release\okur run --keyboard-layout laptop
   ```

4. To stop it, press Insert+F4 and Enter (Escape keeps it running), or run `okur quit` in
   another command prompt. Starting Okur again while one runs replaces it: one Okur speaks
   on a desktop at a time.

Run it from a command prompt opened as administrator to hear applications that run as
administrator. Windows keeps their windows from programs that do not, so otherwise Okur is
silent there (Windows Settings' "Rename this PC" dialog is one). JAWS runs that way too.
Okur says so when it starts without administrator rights.

Two screen readers speaking at once is hard to follow. Unload your usual screen reader first,
or if you keep it running, give Okur another key as its Okur key so the two do not both act
on the same command: `--okur-key scrolllock` (then ScrollLock+T says the window title, and so
on).

Okur's settings are in Settings Center (Insert+6) and Quick Settings (Insert+V), and in Okur's
window (Insert+J): Basics, Voice Adjustment and Select a Voice Profile (Insert+Ctrl+S) among
them. OK or Apply uses a change at once and saves it in `%APPDATA%\Okur\settings.toml`, or for
one application or site in `applications\` and `sites\` beside it.

Options of `okur run` change a setting for that run only:

| Option | What it does |
|---|---|
| `--engine onecore`, `sapi5` or `no-speech` | which speech engine to use (by default OneCore, else SAPI 5) |
| `--voice <name>`, `--rate <-10 to 10>`, `--volume <0 to 100>` | the voice |
| `--keyboard-layout desktop` or `laptop` | the keyboard layout, as JAWS's |
| `--okur-key insert`, `capslock` or `scrolllock` | the Okur key, Okur's modifier (JAWS's JAWS key) |
| `--live-region-whole-text` | read a growing live region whole each time, as JAWS does |
| `--typing-echo none`, `characters`, `words` or `both` | what typing says, as JAWS's Typing Echo: nothing, each character (the default), each word, or both. Insert+2 goes through the four while Okur runs |
| `--set <setting>=<value>` | any setting, by its name in the settings file: `--set say_all_unit=sentence`, `--set voice.punctuation=all`; repeat it for more |
| `--debug-log <file>` | record keys, events and timings, for reporting a problem |

### Reporting a problem

Please [open an issue](https://github.com/monatis/okur/issues). The most useful reports say:
- what you did, key by key;
- what JAWS says there, and what Okur said instead;
- the application or web page.

A debug log helps: run Okur with `--debug-log okur.log`, repeat the problem, stop Okur, and
attach the file. It records keys and what is on screen, so check it before you share it.

---

## For developers

### Build and test

Requires Rust; [`rust-toolchain.toml`](rust-toolchain.toml) pins the version, and rustup
installs it on the first build. The screen reader runs on Windows 10 and 11; the engine
(`okur-core`) builds and tests on any platform.

```
cargo build --release --bin okur-lab --bin okur-ui
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::indexing_slicing -D clippy::string_slice
cargo test --workspace
cargo run --bin okur-lab -- parity
```

`cargo test` is hermetic: no live UI, speech, audio or network. It includes the parity replay
of every recorded JAWS scenario, the corpus manifest check and the ratcheted baselines, so a
step that matched JAWS cannot silently stop matching.

### Quality gates

A screen reader that crashes leaves a blind user without a computer, so crashing is treated
as the worst possible bug.

- **No panics in the screen reader's own crates.** `unwrap`, `expect`, `panic!`, `todo!`,
  `unimplemented!` and `dbg!` are rejected, in tests too. Unchecked indexing and string
  slicing are clippy errors (`indexing_slicing`, `string_slice`).
- **The vendored accessibility layer is held to the same standard** through each crate's
  `[lints]` table; only its tests may panic, and its remaining panicking calls may only
  decrease (a ratchet).
- **Every commit passes the gate** ([`.agents/scripts/check-pre-commit.ps1`](.agents/scripts/check-pre-commit.ps1)):
  the panic scan, clippy with warnings denied, the full test suite, and a check that recorded
  JAWS evidence never lands in the same commit as engine code.
- **CI runs the same checks** on every push and pull request
  ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)), on Windows. It also runs the
  engine's tests on macOS and Linux, reported without failing the build until those ports are
  built there.
- **Responsive by construction.** The keyboard hook never waits more than 150 ms for the
  engine; slow work (building a page's buffer, reading text, writing logs) runs off the engine
  lock and the event loop.

### Performance

Measured on the development machine with `okur-lab latency` (from a live run's debug log),
release build, Okur running next to JAWS:

| What | Result |
|---|---|
| Arrow key to speech in a Windows text field (median) | **19 ms** |
| Ctrl+Right to speech in VS Code's editor on the web (median) | **25 ms**, from 41 ms |
| A GitHub repository page readable after it loads | **160 ms**, from 3 s |
| Reading a focus change in a 300-item list (median) | **55 ms**, from 1,792 ms |
| Building a 9,866-node page's virtual buffer | **14 ms**, from 59 ms |
| The engine handling a key (90th percentile) | **0.1 ms** or less |
| Z (next division) on a page of 25,032 elements | **9.7 ms**, from 559 ms |

Most gains came from measuring first: every stage of an accessibility event read and every
key's way to speech is timed in the debug log. The last row is the engine's own benchmark
([`crates/okur-core/examples/big_page_keys.rs`](crates/okur-core/examples/big_page_keys.rs)),
from the [performance review](docs/backlog/performance-review-2026-10-20.md) of 2026-10-20.

### Repository map

- [`crates/okur-core`](crates/okur-core): the engine. The accessibility object model, the
  virtual buffer, keys, focus, live regions, speech phrased as JAWS phrases it, settings,
  and what Okur's dialogs hold. Platform-neutral.
- [`crates/okur-platform`](crates/okur-platform): Windows bindings (and the future macOS and
  Linux ones): the event tracer, keyboard hook, speech engines, application actions and the
  settings files.
- [`crates/okur-corpus`](crates/okur-corpus): parity replay, accepted divergences,
  invariants, the corpus manifest and the latency summary.
- [`crates/okur`](crates/okur): the screen reader (`okur.exe`: `run`, `quit`, `doctor`,
  `voices`; and `okur-sr.exe`, the build installed with UI Access).
- [`crates/okur-lab`](crates/okur-lab): the verification harness, never installed for users
  (`okur-lab.exe`: `parity`, `live-parity`, `evidence`, `trace`, `snapshot`, `latency`, `keys`,
  `jaws-history`, and more), with `okur-lab-keys.exe`, the lab machine's key sender with UI
  Access for JAWS's own dialogs.
- [`crates/okur-setup`](crates/okur-setup): installs and uninstalls Okur (its start at
  sign-in, its Ease of Access registration) and the development signing script; not yet for
  users.
- [`crates/okur-ui`](crates/okur-ui): draws Okur's own windows (its dialogs and the Virtual
  Viewer on screen) natively, in a process of its own.
- [`crates/okur-ui-model`](crates/okur-ui-model): what Okur and `okur-ui` say to each other.
- [`crates/okur-speech-recorder`](crates/okur-speech-recorder): the SR Speech Recorder voice,
  which records what JAWS sends its synthesizer.
- [`crates/xa11y`](crates/xa11y): the cross-platform accessibility layer, vendored from
  [xa11y](https://github.com/xa11y/xa11y) and extended here
  ([`VENDORED.md`](crates/xa11y/VENDORED.md)).
- [`corpus/`](corpus): fixtures, scenarios, recorded JAWS evidence, event traces, snapshots,
  the generated manifest and baselines. [`corpus/pending/`](corpus/pending) holds scenarios
  waiting to be recorded.
- [`skills/jaws-reference/`](skills/jaws-reference): the recording toolkit for the lab
  machine.
- [`docs/`](docs): plans, handoffs and references.

### How the work is organized

The project is developed by AI coding agents, session after session.
[`AGENTS.md`](AGENTS.md) is their guide and points to the current plan and the latest
handoff. The [verification loop](docs/references/verification-loop.md) describes the lab
machines, recording JAWS evidence and live checks.

---

## How AI agents build it

Clean-room design is an old idea: one team describes what a system does, and another, which
has never seen its insides, builds a new one from that description. Here the description is
**JAWS itself, observed as a black box**, and the builders are AI coding agents (Claude Code,
among others) working in a loop with a verification harness.

That only works if the agents' claims can be checked. An agent can confidently report
"parity achieved" about code that does nothing of the kind, and on 2026-10-14 an audit of
this project found exactly that: parity claims no tool had ever computed. Since then the rule
is simple: **talk is cheap; the harness decides.** No behavior counts until a tool compares it
with recorded JAWS evidence, and every number in a commit or handoff is pasted tool output.

### The lab

- **The development machine** builds and tests the code, replays recorded evidence, and runs
  the screen reader live.
- **`old-pc`, the reference lab,** a separate Windows 10 machine running **JAWS
  2025.2505.43.400** with Chrome, used only as a black box. The harness presses keys there and
  records what JAWS says.

The agents reach the lab through **[OpenDesk](https://github.com/monatis/opendesk)**, our
open-source remote-desktop harness for AI agents. Files move between the machines through a
shared folder, and every transfer is checked by hash.

JAWS's speech history keeps text before JAWS processes symbols, so it is not quite what a
user hears. The **SR Speech Recorder** is a SAPI 5 voice JAWS speaks through on the lab
machine: it logs every utterance, with pitch changes such as JAWS's higher pitch for capital
letters, and hands the text on to a real voice. A tracer also records the accessibility events
Windows raised during each trial, so replay feeds the engine the same events JAWS saw. The
tracer runs with administrator rights, as JAWS does, so it sees JAWS's own dialogs whole.
Web pages open in a signed-out Chrome profile of the lab's own, so no person's name shows in
window titles or recordings.

Okur can also run live on the lab machine through the same scenarios, and
`okur-lab live-parity` compares what it said with JAWS, step by step. That measures what replay
cannot, such as Okur's own dialogs.

### The loop

1. **Discover** a gap, from a user report, a JAWS manual or keystroke list, or JAWS's
   observed behavior.
2. **Record the reference:** a fixture (a web page or a small Windows app) and a scenario of
   keystrokes, run on `old-pc` several times, keeping JAWS's speech, synthesizer text, event
   traces and provenance.
3. **Implement** the behavior from scratch, with per-application behavior in named modules,
   as JAWS keeps it in application scripts.
4. **Test:** replay every recorded scenario through the engine (`okur-lab parity`) and compare
   each step with JAWS. Results are ratcheted. Where JAWS errs by accident, the step is
   written down as an accepted divergence instead of copied.
5. **Prove it live:** run the screen reader against real applications with a debug log, and
   measure its responsiveness (`okur-lab latency`).

### Integrity rules

From [`AGENTS.md`](AGENTS.md), enforced by tests, the commit gate and CI where a machine can
enforce them:
1. Evidence is written only by capture tooling on the lab machine; never by hand.
2. Evidence and engine code never land in the same commit.
3. Generated artifacts (the corpus manifest, the parity and invariant baselines) are
   regenerated, never edited.
4. Every parity number or "verified" claim quotes the command that produced it.
5. `cargo test` is hermetic. Live checks are separate.
6. A known but unimplemented JAWS behavior becomes a test asserting it, marked as a known gap.
7. No fixture-shaped special cases: code never branches on a test page's text or a machine's
   window titles.

## Platforms

| Platform | Tier | Status |
|---|---|---|
| Windows 10 and 11 | 1 | Runs today: UI Automation, IAccessible2, MSAA and WinEvents; low-level keyboard hook; OneCore and SAPI 5 speech; Okur's own dialogs as native Win32 dialogs |
| macOS | 1 | The accessibility layer (AXUIElement) is in place; the keyboard event tap, event listener and speech come once the everyday gaps on Windows are closed |
| Linux | 2 | The accessibility layer (AT-SPI2) is in place; best effort |

Every OS-dependent part is a trait in the core (keyboard hook, speech output, accessibility
events, application actions), with a mock for tests. The engine never knows which platform it
runs on. Platform patterns were learned from NVDA's open source code; everything the user
hears follows JAWS.

## Clean room and interoperability

- No source code, scripts, binaries or decompiled code from Freedom Scientific are copied,
  bundled or examined. Everything here was written from scratch.
- JAWS is observed only from the outside: keystrokes in, speech out, on fixtures this project
  wrote.
- The project's position is that keystrokes, announcement phrasing and navigation models are
  functional methods of operation, an interoperability interface users depend on, and that
  observing a program's behavior to build a compatible one is legitimate.

JAWS, Freedom Scientific and Vispero are trademarks of Vispero. Okur is independent and is
not affiliated with or endorsed by them. Okur never uses these names as its own: its
messages, dialogs, settings, command-line options and installer say "Okur" where JAWS names
itself, and name JAWS only to compare with it.

## License

Okur is licensed under the [Apache License, Version 2.0](LICENSE); see also [`NOTICE`](NOTICE).
The vendored xa11y crates in [`crates/xa11y`](crates/xa11y) keep their MIT license
([`crates/xa11y/LICENSE`](crates/xa11y/LICENSE)).
