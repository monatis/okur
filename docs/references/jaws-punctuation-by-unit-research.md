# JAWS's punctuation by reading unit: research

Date: 2026-10-24. Research only: nothing was recorded on old-pc and no code was changed. The
backlog entry is in [`../backlog/open-items.md`](../backlog/open-items.md), "Reading settings".
The levels themselves and the symbol table are in
[`jaws-punctuation-levels.md`](jaws-punctuation-levels.md); the settings research is
[`jaws-reading-settings-research.md`](jaws-reading-settings-research.md), Item 1.

Marks, as in the settings research: **Doc** a document says it; **Obs** recorded evidence shows
it; **Inf** my inference, to be confirmed; **Open** neither documented nor recorded.

## The owner's report (2026-10-24)

- JAWS's punctuation level seems not to apply when reading word by word (Ctrl+Left and
  Ctrl+Right): JAWS names the punctuation then.
- Reading `C:\users\gigabyte\Downloads` word by word, JAWS says "C:", "\", "users", "\",
  "gigabyte", "\", "Downloads".
- Okur says "C", then nothing while the cursor moves, "Users", nothing, "gigabyte", nothing,
  "downloads".
- The owner's Okur runs at Some: `%APPDATA%\Okur\settings.toml` has `punctuation = "some"` under
  `[voice]` (read on the dev box, 2026-10-24). The level of the owner's JAWS is not known.
- The application is not named.

## Sources

| Id | Source | What it gives | Strength |
|---|---|---|---|
| [S1] | JAWS 2025 help, `SettingsCenter.chm`, Text Processing, Smart Word Reading (`TextProcessing/text_processing_dialog.htm`), read in the dev box's installed help | Say Word follows the application's word units ("application dependent"). Without Smart Word Reading, the caret on the colon of "2:00 P.M." gives "colon". | Doc, official, current build. About Say Word, not the level |
| [S2] | `SettingsCenter.chm`, Customize Punctuation (`Punctuation/customize_punctuation.htm`) | Levels are cumulative. The levels of blank, colon, comma, dot, point and space cannot be changed; JAWS uses them in special places. Nothing about reading units | Doc, official |
| [S3] | `SettingsCenter.chm`, Graphics and Symbols, Announce Special Symbols When (`GraphicsAndSymbols/graphics_and_symbols_dialog.htm`) | JAWS treats special symbols by reading unit: typing, Say Character and Spell Word; also Say Word; also lines and Say All | Doc, official. Shows that JAWS's symbol handling depends on the unit; about a symbol's value, not the level |
| [H1] | `JFW.chm`, Voice Adjustment | Punctuation per voice context; Most by default | Doc, official |
| [W1] | University of Houston, JAWS Tutorial + Accessibility & Math Tips, §10 (https://online.math.uh.edu/jaws-tutorial/, updated 2025-12-05) | At None, symbols such as `#`, `$`, `%`, `&`, `*` are left out unless they are on a line by themselves or read one character at a time | Third party, no JAWS version. Medium-weak |
| [W2] | JFW mailing list, "punctuation when reading by word", January 2021 (https://jfw.groups.io/g/main/topic/punctuation_when_reading_by/80021211), read in the browser pane | Where a word ends depends on the application: in Notepad commas and periods stay with their words; in Word they are words of their own; Ted Notepad stops at every bracket. A reply calls it the application's doing, not JAWS's | User reports. Medium for "the units come from the application"; nothing about the level |
| [W3] | JFW mailing list, Matthew Horspool, 2018-05-23 (https://jfw.groups.io/g/main/message/59744) | A tab is announced when moving by character, perhaps by word, not by line | One user, hedged. Weak |
| [W4] | Microsoft, ComboBoxEx Control Extended Styles, `CBES_EX_PATHWORDBREAKPROC` (https://learn.microsoft.com/en-us/windows/win32/controls/comboboxex-control-extended-styles) | In such a combo box, slash, backslash and period are word delimiters for Ctrl+arrow movement in paths and URLs | Official Windows documentation. Explains a "C:", "\" stop pattern; whether the owner's field was one is Inf |
| [N1] | NVDA, `source/speech/speech.py`, `getTextInfoSpeech` (https://github.com/nvaccess/nvda/blob/master/source/speech/speech.py) | NVDA's: moving by character or word, a unit of one character is spelled, so a lone symbol is named whatever the symbol level; a longer word is spoken at the user's level | NVDA's source; contrast only, not a parity target |
| [O] | The corpus, old-pc, JAWS 2025.2505.43.400 at Most | The steps below | Obs |

Searches that found nothing more (2026-10-24): the web for JAWS punctuation by word, Say Word
and paths; JFW list searches for "back slash" with punctuation, "say word" with punctuation,
"punctuation level" with "word by word". No Freedom Scientific page says how the level applies
by unit. `groups.io` refuses the fetch tool (HTTP 402); its pages were read in the browser pane.

## What the documents say

- **Nothing official says the level is ignored by word.** The help describes the level per
  symbol and per voice context only [S2], [H1].
- **JAWS does handle symbols by unit** for one other setting: the numeric value of special
  symbols is announced by typing, character and spelling, then optionally by word, then by line
  and Say All [S3]. Inf: JAWS has the notion of reading by unit in its symbol processing.
- **A punctuation mark read alone is named.** The help's Smart Word Reading example says the
  colon alone as "colon" under Say Word [S1]. A third-party tutorial says symbols dropped at None
  are still read on a line by themselves or by character [W1].
- **The word itself is the application's.** Smart Word Reading is "application dependent" [S1];
  users report punctuation stops differing between Notepad, Word and other editors [W2]. A path
  combo box breaks words at `\`, `/` and `.` [W4], which would give exactly the owner's stops
  "C:", "\", "users" (Inf: the colon is no delimiter there, so it stays with "C").

## What the corpus shows

Every step below was recorded at Most, old-pc's level. **No word or character step has been
recorded at any other level**: the only scenario that changes the level, 400, reads lines.

What JAWS sent its synthesizer (the speech recorder's `spoken`), from a scratch script over the
evidence files:

| Step | Where | History | Spoken |
|---|---|---|---|
| 106/s4 | virtual cursor, Ctrl+Right | `month,` | month comma, |
| 106/s6, s8, s14, s21, s27, s33 | virtual cursor, a period alone | `.` | period. |
| 106/s23 | virtual cursor | `—` | em dash — |
| 106/s18, s19 | virtual cursor | `help@riverside`, `-` | help at riverside, dash |
| 106/s30 | virtual cursor | `3.5%` | 3.5% |
| 213/s11, s14 | a page's text area, Ctrl+Right (Okur's replay: the virtual cursor) | `dog,`, `std::` | dog comma, / std colon colon |
| 510/s7 | WinForms edit, Ctrl+Right | `world.` | world period. |
| 529/s12, s13 | WinForms edit, Ctrl+Shift+Right | `Hello,`, `world.` | Hello comma, / world period. |
| 405/s25 | WinForms edit, Ctrl+Right | `P.M.` | P dot M period. |
| 407/s20, s21, s22 | virtual cursor | `2.5`, `1,000,000`, `10:30` | 2.5 / 1,000,000 / 10:30 |
| 504/s12 | WinForms edit, Insert+Right | `1.5;` | 1.5 semi colon |
| 311/s6 | Virtual Viewer, Ctrl+Right | `...` | dot dot dot |
| 664/s4 | Notepad, Ctrl+Backspace | `()` | left paren right paren |

So, at Most:
- **A word names symbols of level All**: the comma, the period and the em dash, which a line at
  Most leaves unnamed (400/s1: "Hello, world. Is it done? Yes!"). It does so in every context
  recorded: the virtual cursor, a page's text area, a native edit, a selection by word.
- **Numbers keep the line's rules**: a decimal point, thousands separators and a time's colon
  stay as they are (407), not "2 point 5" as at All. Okur's `pronounce::spoken_symbols` encodes
  exactly this ("render at All, numbers as at the lower levels", `pronounce.rs:154-160`).
- **The virtual cursor's word units** (106, 213): a trailing comma, semicolon or colon stays with
  its word ("month,", "adults;", "std::"); a period and a dash are words of their own except
  inside a number; `@`, `$` and `%` stay inside. How a backslash splits a word in the virtual
  cursor is **Open**.

What is not known:
- **None and Some by word** (the owner's case): **Open**. The owner's report says JAWS names `\`,
  which is named from Most, below Most. Inf: JAWS reads a word at All whatever the level, as the
  Most evidence shows for All's symbols. NVDA instead names only a one-character word [N1].
- **A character read alone at None**: `pronounce.rs` claims JAWS names it at every level; only
  Most is recorded (settings research, Item 1). **Open**; the scenario below covers it too.
- **Old-pc's Notepad is Windows 10's** (Win32 edit, `os_build` 10.0.19045 in the provenance),
  the owner's is Windows 11's (RichEdit). Their word stops may differ;
  `desktop-richedit.ps1` (a RichTextBox) is the Windows 11 family.

## Okur today: where the difference comes from

Read from the code at `38055533` (2026-10-24).

**Two speech calls.** `speak` gives the synthesizer the text at the punctuation level
(`crates/okur-platform/src/tts/mod.rs:219-246`, `pronounce::spoken_with`); `speak_symbols` gives
it every symbol named (`tts/mod.rs:224-231`, `tts/thread.rs:353-356`, `pronounce::spoken_symbols`
at `pronounce.rs:158-160`).

**Why `\` is silent and `C:` loses its colon at Some.** `:` and `\` are named from Most
(`pronounce.rs:433`, `:441`). Below Most a colon stays as a pause mark (`pronounce.rs:252-255`)
and a backslash becomes a space (`pronounce.rs:257`). So a word spoken with `speak` at Some or
None goes to the voice as `C:` (heard "C"), `\` as nothing, `:\` as a lone `:`. This rendering is
right for a line: `okur-lab parity --spoken --only 400-desktop-punctuation-cycle` gives "JAWS
spoken parity: 25/25 steps match exactly" (every level, the path line included). The fault is
using it for a word.

Word reading, by path:

1. **A native edit or a page field in forms mode, Ctrl+Left/Right** (`EditKey::Word`): spoken
   with `speak_symbols` (`crates/okur-core/src/engine/text_feedback.rs:355-363`), so symbols are
   named whatever the level, as JAWS does at Most (510/s7 matches). The word read runs from the
   caret to the end of the field's own word, or to the next space in a Win32 edit read by
   window messages, which reports no word unit (`crates/okur-core/src/editing.rs:434-451`;
   `crates/okur-platform/src/text_context.rs:381` for page fields, `:939-943` for UIA fields).
   Inf: where the application stops at `\` but reports no word unit, Okur reads
   `\Users\Ada\Downloads` to the next space where JAWS reads `\`. Not silent, so this is
   probably not the owner's path.
2. **The virtual cursor, Ctrl+Left/Right and Insert+Left/Right** (`Command::SayNextWord`,
   `SayPriorWord`, `crates/okur-core/src/engine/commands.rs:338-339`) and **Say Word**
   (`commands.rs:581`): spoken with `speak` (`commands.rs:687-688`), at the level. Recorded at
   Most, `okur-lab parity --spoken --only 106-article-words --only 213-web-textarea-words`
   (`target/debug/okur-lab.exe`, 2026-10-24):

   ```
   [DIFF ] spoken 106-article-words/s4  (Ctrl+Right)
       ours: "month,"
       jaws: "month comma,"
   [DIFF ] spoken 106-article-words/s6  (Ctrl+Right)
       ours: "."
       jaws: "period."
   (s8, s14, s21, s27 and s33 the same)
   [DIFF ] spoken 106-article-words/s23  (Ctrl+Right)
       ours: ""
       jaws: "em dash —"
   [DIFF ] spoken 213-web-textarea-words/s11  (Ctrl+Right)
       ours: "dog,"
       jaws: "dog comma,"
   JAWS spoken parity: 43/52 steps match exactly (evidence recorded through the speech recorder voice)
   JAWS parity (Ia2): 52/52 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
   ```

   The history layer matches; the spoken layer misses every All-level symbol. The comment at
   `text_feedback.rs:355` cites 213/s11 for `speak_symbols`, but the replay reads 213 through
   this virtual cursor path. The virtual buffer's words also keep a path whole: a backslash
   does not split a word (`crates/okur-core/src/linearize.rs:1494-1533`, `segment_words`), so
   `C:\Users\Ada\Downloads` is one word.
3. **Say Word in a text field** (Insert+Numpad 5, CapsLock+K): `word_around`
   (`editing.rs:458-482`) spoken with `speak` (`commands.rs:1142-1157`), at the level.
4. **The Okur cursor, Ctrl+Left/Right and Insert+Left/Right** (`cursors.rs:485-486`,
   `:594-638`; Say Word `:712-726`): words from `web_word_at` (`cursors.rs:76-87`), spoken with
   `say` (`crates/okur-core/src/engine/toggles.rs:140-142`), which is `speak`, at the level.
5. **The Virtual Viewer, Ctrl+Left/Right** (`crates/okur-core/src/viewer.rs:295`, `move_word`;
   `:221`, `word_text`): spoken by `ViewerAction::Speak` with `speak`
   (`crates/okur-core/src/engine.rs:974-976`), at the level.

**Word units with a colon.** `web_word_at` (`editing.rs:391-425`) and the viewer's `class`
(`viewer.rs:78-86`) put every run of non-letters in one word, so `C:\Users` is "C", ":\",
"Users", where JAWS's virtual cursor keeps a colon with the word before it ("std::", 213/s14)
and the owner heard "C:", "\". Even with symbols named, Okur would say "C" and then "colon back
slash" where JAWS says "C colon" and then "back slash".

**Which path the owner used** is not known. The report matches paths 4 and 5 exactly at Some
(units "C", ":\", "Users", "\": "C", nothing, "Users", nothing), and path 3 would say the same
word by word. It does not match path 1 (symbols named) nor path 2's units (the whole path in
one word). Inf; a live debug log of the
owner's reading (`RUST_LOG=okur_core=debug`, the "edit speech" line) would settle it.

## What to record first

**Fixture.** `desktop-notepad.ps1` is pinned by the manifest, so a new one,
`corpus/fixtures/desktop-notepad-paths.ps1`, made the same way (writes a text file to `%TEMP%`,
opens it in Notepad), ASCII, with one line:

```
C:\Users\Ada\Downloads or https://example.org/a-b or ada@example.org
```

Window title `sr-notepad-paths.txt - Notepad`. If Notepad stops at every run of punctuation, the
line has 23 word stops: C, `:\`, Users, `\`, Ada, `\`, Downloads, or, https, `://`, example, `.`,
org, `/`, a, `-`, b, or, ada, `@`, example, `.`, org.

Reserved domain and the fixtures' "Ada" only, so the tracer keeps them (the owner's own path is
not used).

**Scenario** (for `corpus/pending/`, the next free id; 1037 on 2026-10-24). Most first; then
Insert+Shift+2 twice (All, None), as 400 shows the cycle from Most is All, None, Some, Most; then
Some, the owner's Okur level; the last press brings Most back, so every trial starts at Most.
Each level: the line, two characters alone (`:` and `\`, the open character claim), Ctrl+Right
along the line, Say Word (Insert+Numpad 5 works in old-pc's laptop layout, 567/s8), Ctrl+Left
back.

```json
{
  "id": "1037-desktop-notepad-words-with-punctuation",
  "name": "Notepad: a path, a web address and an email address read by word (Ctrl+Right, Insert+Numpad5, Ctrl+Left) and two symbols by character, at punctuation Most, None and Some",
  "category": "desktop",
  "version": "1.0.0",
  "fixture": "file:///C:/Users/ailabs/corpus/fixtures/desktop-notepad-paths.ps1",
  "target_app": "desktop",
  "launch": { "window_title": "sr-notepad-paths.txt - Notepad" },
  "reset": { "keys": ["Ctrl+Home"] },
  "steps": [
    { "id": "s1", "type": "key", "command": "Insert+Up" },
    { "id": "s2", "type": "key", "command": "Right" },
    { "id": "s3", "type": "key", "command": "Right" },
    { "id": "s4", "type": "key", "command": "Ctrl+Home" },
    { "id": "s5", "type": "key", "command": "Ctrl+Right" },
    { "id": "s6", "type": "key", "command": "Insert+Numpad5" },
    { "id": "s7", "type": "key", "command": "Ctrl+Right" },
    { "id": "s8", "type": "key", "command": "Ctrl+Right" },
    { "id": "s9", "type": "key", "command": "Ctrl+Right" },
    { "id": "s10", "type": "key", "command": "Ctrl+Right" },
    { "id": "s11", "type": "key", "command": "Ctrl+Right" },
    { "id": "s12", "type": "key", "command": "Ctrl+Right" },
    { "id": "s13", "type": "key", "command": "Ctrl+Right" },
    { "id": "s14", "type": "key", "command": "Ctrl+Right" },
    { "id": "s15", "type": "key", "command": "Ctrl+Right" },
    { "id": "s16", "type": "key", "command": "Ctrl+Right" },
    { "id": "s17", "type": "key", "command": "Ctrl+Right" },
    { "id": "s18", "type": "key", "command": "Ctrl+Right" },
    { "id": "s19", "type": "key", "command": "Ctrl+Right" },
    { "id": "s20", "type": "key", "command": "Ctrl+Right" },
    { "id": "s21", "type": "key", "command": "Ctrl+Right" },
    { "id": "s22", "type": "key", "command": "Ctrl+Right" },
    { "id": "s23", "type": "key", "command": "Ctrl+Right" },
    { "id": "s24", "type": "key", "command": "Ctrl+Right" },
    { "id": "s25", "type": "key", "command": "Ctrl+Right" },
    { "id": "s26", "type": "key", "command": "Ctrl+Right" },
    { "id": "s27", "type": "key", "command": "Ctrl+Right" },
    { "id": "s28", "type": "key", "command": "Ctrl+Right" },
    { "id": "s29", "type": "key", "command": "Ctrl+Left" },
    { "id": "s30", "type": "key", "command": "Ctrl+Left" },
    { "id": "s31", "type": "key", "command": "Ctrl+Left" },
    { "id": "s32", "type": "key", "command": "Ctrl+Home" },
    { "id": "s33", "type": "key", "command": "Insert+Shift+2" },
    { "id": "s34", "type": "key", "command": "Insert+Shift+2" },
    { "id": "s35", "type": "key", "command": "Insert+Up" },
    { "id": "s36", "type": "key", "command": "Right" },
    { "id": "s37", "type": "key", "command": "Right" },
    { "id": "s38", "type": "key", "command": "Ctrl+Home" },
    { "id": "s39", "type": "key", "command": "Ctrl+Right" },
    { "id": "s40", "type": "key", "command": "Insert+Numpad5" },
    { "id": "s41", "type": "key", "command": "Ctrl+Right" },
    { "id": "s42", "type": "key", "command": "Ctrl+Right" },
    { "id": "s43", "type": "key", "command": "Ctrl+Right" },
    { "id": "s44", "type": "key", "command": "Ctrl+Right" },
    { "id": "s45", "type": "key", "command": "Ctrl+Right" },
    { "id": "s46", "type": "key", "command": "Ctrl+Right" },
    { "id": "s47", "type": "key", "command": "Ctrl+Right" },
    { "id": "s48", "type": "key", "command": "Ctrl+Right" },
    { "id": "s49", "type": "key", "command": "Ctrl+Right" },
    { "id": "s50", "type": "key", "command": "Ctrl+Right" },
    { "id": "s51", "type": "key", "command": "Ctrl+Right" },
    { "id": "s52", "type": "key", "command": "Ctrl+Right" },
    { "id": "s53", "type": "key", "command": "Ctrl+Right" },
    { "id": "s54", "type": "key", "command": "Ctrl+Right" },
    { "id": "s55", "type": "key", "command": "Ctrl+Right" },
    { "id": "s56", "type": "key", "command": "Ctrl+Right" },
    { "id": "s57", "type": "key", "command": "Ctrl+Right" },
    { "id": "s58", "type": "key", "command": "Ctrl+Right" },
    { "id": "s59", "type": "key", "command": "Ctrl+Right" },
    { "id": "s60", "type": "key", "command": "Ctrl+Right" },
    { "id": "s61", "type": "key", "command": "Ctrl+Right" },
    { "id": "s62", "type": "key", "command": "Ctrl+Right" },
    { "id": "s63", "type": "key", "command": "Ctrl+Left" },
    { "id": "s64", "type": "key", "command": "Ctrl+Left" },
    { "id": "s65", "type": "key", "command": "Ctrl+Left" },
    { "id": "s66", "type": "key", "command": "Ctrl+Home" },
    { "id": "s67", "type": "key", "command": "Insert+Shift+2" },
    { "id": "s68", "type": "key", "command": "Insert+Up" },
    { "id": "s69", "type": "key", "command": "Right" },
    { "id": "s70", "type": "key", "command": "Right" },
    { "id": "s71", "type": "key", "command": "Ctrl+Home" },
    { "id": "s72", "type": "key", "command": "Ctrl+Right" },
    { "id": "s73", "type": "key", "command": "Insert+Numpad5" },
    { "id": "s74", "type": "key", "command": "Ctrl+Right" },
    { "id": "s75", "type": "key", "command": "Ctrl+Right" },
    { "id": "s76", "type": "key", "command": "Ctrl+Right" },
    { "id": "s77", "type": "key", "command": "Ctrl+Right" },
    { "id": "s78", "type": "key", "command": "Ctrl+Right" },
    { "id": "s79", "type": "key", "command": "Ctrl+Right" },
    { "id": "s80", "type": "key", "command": "Ctrl+Right" },
    { "id": "s81", "type": "key", "command": "Ctrl+Right" },
    { "id": "s82", "type": "key", "command": "Ctrl+Right" },
    { "id": "s83", "type": "key", "command": "Ctrl+Right" },
    { "id": "s84", "type": "key", "command": "Ctrl+Right" },
    { "id": "s85", "type": "key", "command": "Ctrl+Right" },
    { "id": "s86", "type": "key", "command": "Ctrl+Right" },
    { "id": "s87", "type": "key", "command": "Ctrl+Right" },
    { "id": "s88", "type": "key", "command": "Ctrl+Right" },
    { "id": "s89", "type": "key", "command": "Ctrl+Right" },
    { "id": "s90", "type": "key", "command": "Ctrl+Right" },
    { "id": "s91", "type": "key", "command": "Ctrl+Right" },
    { "id": "s92", "type": "key", "command": "Ctrl+Right" },
    { "id": "s93", "type": "key", "command": "Ctrl+Right" },
    { "id": "s94", "type": "key", "command": "Ctrl+Right" },
    { "id": "s95", "type": "key", "command": "Ctrl+Right" },
    { "id": "s96", "type": "key", "command": "Ctrl+Left" },
    { "id": "s97", "type": "key", "command": "Ctrl+Left" },
    { "id": "s98", "type": "key", "command": "Ctrl+Left" },
    { "id": "s99", "type": "key", "command": "Ctrl+Home" },
    { "id": "s100", "type": "key", "command": "Insert+Shift+2" }
  ]
}
```

Each level block (Most s1-s32, None s35-s66, Some s68-s99): the line (Insert+Up); Right twice
from the line's start, onto `:` and `\`; Ctrl+Home, Ctrl+Right once, then Say Word on that stop;
Ctrl+Right 22 times more, to the line's end even with all 23 stops (presses at the end record
what JAWS says there: "blank" at a line's end in 320/s11); Ctrl+Left three times; Ctrl+Home. s33 and s34 say "All" and "None", s67 "Some", s100
"Most". 100 steps; about six minutes for three trials at old-pc's pace (Inf).

**Then, in this order:**
- the same line in `desktop-richedit.ps1`'s family (Windows 11 Notepad's RichEdit), which the
  owner uses;
- the virtual cursor and the Virtual Viewer at the same levels: `reading-settings.html` lines 2
  and 3 already hold an email address, a web address and two paths (no new fixture); the
  viewer by Insert+Alt+W over the Notepad fixture, as 311 does over a dialog;
- a path field that breaks words at `\` [W4] (File Explorer's address bar or the Run dialog):
  the owner's stops; it needs a fixture the harness can launch. **Open.**

## What 1037 recorded (2026-10-24, `e4e88597`)

Old-pc's Notepad is Windows 10's classic one (an Edit control), not the owner's Windows 11
Notepad (RichEdit), so its word stops differ from the owner's report. In the speech history, the
same at Most, None and Some:
- Ctrl+Right stops three times inside `C:\Users\Ada\Downloads`, and JAWS says the whole token at
  each stop: "C:\Users\Ada\Downloads". Okur says from the caret on: "\Users\Ada\Downloads",
  "\Ada\Downloads", "\Downloads".
- A hyphen ends JAWS's word: "https://example.org/a", then "b". Okur says
  "https://example.org/a-b".
- Say Word on the first stop says "\"; Okur says ":\".
- At the line's end, Ctrl+Right says nothing.

`okur-lab parity --only 1037`: 85 of 100 steps match; the other 15 are these three. What the
synthesizer was given at each level (the spoken layer) is still to read. The owner's Notepad
needs the same line in `desktop-richedit.ps1`'s family, recorded next.

**Built since (2026-10-24):**
- A plain edit's word is the whole run around the caret, between spaces and hyphens, and
  nothing at the text's end (`f54c4539`). 1037's word steps match, all but Say Word on the
  first stop ("\" where Okur says ":\").
- In pages and web fields, a word read on its own names every symbol, as desktop fields' did,
  and a word that is one symbol is that character alone, "em dash —" (`f1e3307a`). 106/s4, s6,
  s23 and 213/s11 match in the spoken layer.
- 1041 records the same line in a RichEdit control (`desktop-richedit-paths.ps1`), the family
  of the owner's Windows 11 Notepad, whose own word unit stops at punctuation.

## What Okur should do, once recorded

Not decided before the recordings. If None and Some show what Most shows:
- read every word (paths 2 to 5 above, and Say Word) with `speak_symbols`, as Ctrl+Left/Right in
  a native edit already does;
- split words as JAWS does where Okur splits them itself: a colon with the word before it
  (`web_word_at`, the viewer's `class`), a backslash as a word of its own in the virtual buffer
  if the web recording shows it (`segment_words`);
- add known-gap tests (AGENTS.md rule 6) for 106/s4, s6, s23 and 213/s11 on the spoken layer
  now: their JAWS behavior is recorded.
