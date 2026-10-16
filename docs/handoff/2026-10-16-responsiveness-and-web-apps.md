# Handoff: responsiveness and web apps (2026-10-16)

**Branch**: `responsive`, from `main` at `b686c65`, fast-forwarded into `main` at the end.
**Plan**: [`docs/plans/2026-10-16-web-apps-round-2.md`](../plans/2026-10-16-web-apps-round-2.md)
lists what is next. The old-pc work items are still those of
[`docs/plans/2026-10-15-jaws-feature-loop-round-2.md`](../plans/2026-10-15-jaws-feature-loop-round-2.md),
plus the pending scenarios 210–215 written tonight.

## Why

old-pc stayed offline. The user asked, overnight (2026-10-15 to 16), for:
1. the rest of the structure work, without speech regressions;
2. everyday keys (Left and Right, Ctrl+Left and Ctrl+Right, Up and Down) to feel instant,
   measured;
3. complex web apps: GitHub, Google Docs and Sheets first, then Electron IDEs and coding
   agents (Antigravity, Cursor, Codex, Claude).

Everything below was checked against JAWS 2025 on the dev box. That is research, not
evidence: the dev box runs the user's own JAWS settings, which differ from old-pc's (it says
no "banner" on `news-article.html`, where old-pc's evidence does). Each behavior has a pending
scenario for old-pc (see "Next").

## What changed

| Commit | Change |
|---|---|
| `8ab04dc` | `okur latency` times each key to the speech audio it caused ("key to speech audio: RightArrow") |
| `37d986e` | Arrow keys in a text field: the field is compared with what the engine knew, not with a read taken after the key had already acted (p90 280 ms before) |
| `a7ddbf8` | Structure step 4: the window heading and application modules speak `Announcement`s |
| `b9a1910` | A page is read quickly through UIA (GitHub: 160 ms), then in full through IA2 (3 s) without moving the virtual cursor; UIA runtime ids act on the quick capture's controls |
| `e31b7aa` | Space on a page's disclosure button says "expanded" / "collapsed" |
| `a4c7c65` | aria-current ("Link Current Page README"); a link to `#` alone is a plain "Link" |
| `bc9e1f2` | A page field is found once per key and polled by caret, selection and length every 2 ms |
| `644ceed` | A focused multi-line field says "Contains text", not its text (a 278 KB text area was read whole) |
| `b7e9ea8` | With JAWS running, its copies of keys are not handled twice; `okur front <title>` |
| `9029b07` | In a page field, words stop at punctuation: "dog", ","; "std", "::", "collections" |
| `f625140` | aria-roledescription in place of the role |
| `a45e482` | Tab and Shift+Tab move on from the virtual cursor; menu buttons and popup buttons |
| `01c65c3` | Autocompletes: options pointed at with aria-activedescendant are read, forms mode stays |
| `26f1f1c` | A role="application" region takes every key, Escape too, and is named on the way in |
| `04bdac9` | Focus names the lists and the main, form and navigation regions it enters |
| `a8a1a2c` | `corpus/pending`: scenarios 210–215 and `web-app-patterns.html` |
| `205c01b` | aria-label links say their label; aria-keyshortcuts; a heading that is a button |
| `380a13d` | A read-only text area (GitHub's code view) is read by lines |
| `07aa36f` | E stops on an edit combo and says "edit combo" |
| `68eb04f` | The cursor stays on the focus when the page is captured again (Enter clicked the wrong button after the full capture) |
| `0440801` | Chrome's address bar is read as itself, not as the page |
| `054afe4` | Leaving an autocomplete for another app reads its fields again |
| `d48f567` | `okur caret` reads Chrome's address bar as itself too |
| `291ab94` | Focus names every landmark it enters (banner, complementary, search, region, contentinfo) |

## Verification

**Parity is unchanged.** `same-speech.sh` (scratchpad; now in the repo as
`skills/jaws-reference/scripts/parity-snapshot.sh`) reruns `okur parity --all`, `okur parity
--all --api uia` and `okur parity --spoken` and compares their whole output byte for byte with
the reference saved at `5cc13e1`. After every commit above it printed:

```
same: parity-all-ia2.txt (1718 lines)
same: parity-all-uia.txt (1817 lines)
same: parity-spoken.txt (85 lines)
```

The summary lines of that output, at `07aa36f` (and unchanged at `68eb04f`):

```
JAWS parity (Ia2): 818/844 steps match exactly (0 scenarios could not run)
JAWS parity (Uia): 719/844 steps match exactly (0 scenarios could not run)
JAWS spoken parity: 171/172 steps match exactly (evidence recorded through the speech recorder voice)
```

One change made parity differ before it was committed: an editable combo box detected through
UIA turned the `<select>` of 204/205 into an "edit combo" (UIA reports a select's value as
writable). It now comes from IA2's editable state only, and the output is identical again.

**Tests.** `cargo test --workspace` at `d48f567`: 911 passed, 0 failed, 198 ignored (summed
from its "test result" lines). Each feature above has a hermetic test.

### Latency

`okur latency <debug log>`, "key to speech audio", p50 in ms, from `measure-keys.sh`
(scratchpad; now `skills/jaws-reference/scripts/measure-edit-keys.sh`): Ctrl+Home, then 25 Right, 25 Left, 15 Ctrl+Right, 15 Ctrl+Left and 10 Down, with
human spacing (300–350 ms), Okur running next to the user's JAWS.

Until `b7e9ea8`, Okur saw each key twice with JAWS running (JAWS sends its own copy about 5 ms
later), and the tool timed keys from the copy: earlier figures were about 5 ms low. The
before column below is the old build's log with those copies removed
(`lat/drop-copies.js`), so both columns are timed from the original key.

| Field | Key | `a4c7c65` | `07aa36f` |
|---|---|---|---|
| WinForms multi-line edit | Right | 18.8 | 19.7 |
| | Left | 18.6 | 19.1 |
| | Ctrl+Right | 17.7 | 17.7 |
| | Ctrl+Left | 17.3 | 17.7 |
| | Down | 26.6 | 25.7 |
| vscode.dev editor (Monaco) | Right | 34.1 | 30.6 |
| | Left | 34.3 | 30.4 |
| | Ctrl+Right | 40.9 | 24.9 |
| | Ctrl+Left | 21.4 | 20.3 |
| | Down | 43.2 | 44.9 |

The edit speech of the two runs differs only in the new word stops ("std", "::" for
"std::collections::HashMap;"). In vscode.dev what remains is the page's own time: Monaco
rewrites its text area for an arrow key, then Chrome updates its tree.

In a 278 KB web text area each key takes 130–250 ms. The log shows that Chrome takes about
230 ms to report a caret move there, while Okur reads the whole field in about 10 ms; the
earlier note that blamed reading the text was wrong.

Before `37d986e` (session start, same tool): edit keys p50 37–46 ms, p90 about 280 ms.

## What JAWS said on the dev box

The speech history (`okur jaws-history`), on test pages in an isolated Chrome profile and on
GitHub logged out. Okur's live speech (`okur run --speech-log`) matched each after its commit.

- aria-current: "same page link Current Page Page one", "Link Current Page Page remote"; Tab:
  "Page one same page link Current Page", but "README Current Page Link". `href="#"`: "Link".
- Role descriptions: "Next slide slide control", "chip Rust", "Wrap toggle not checked",
  "Code editor fn main() {}". On Tab a link keeps "same page link".
- Popup buttons: "Outline button menu", "Actions button menu collapsed", "Pick one Button Has
  Popup listbox". On Tab a menu button says no state and no hint.
- Autocomplete (Tab, Enter, A, Down×3, Up, Enter): "Fruit edit combo collapsed | blank",
  "expanded", "Fruits list | with 3 items | Apple | 1 of 3 | To move to an item press the Arrow
  keys.", …, "Fruit edit combo collapsed | Apricot | To set the value use the Arrow keys or type
  the value." vscode.dev's command palette reads the same way ("Accounts: Manage Accounts,
  other commands | 2 of 694 | …").
- Tab from the virtual cursor: with focus on "Next slide" and the cursor moved past "Rust" to
  "group end Featured", Tab gave "Code editor Contains text" and Shift+Tab "Rust same page
  link".
- role="application": Tab gave "Workbench | First tool Button"; H, Down, Escape and Tab all
  reached the page (its key log: "h ArrowDown Escape Tab").
- GitHub: "Repository files NavigationRegion | list | with 1 items | README Current Page
  Link"; "Collapse file tree heading level 2 Button expanded"; "monatis Alt+ArrowUp Graphic";
  "Link commits by monatis Alt+ArrowUp"; E: "Go to file edit combo collapsed blank", then
  "file content read only edit" and the whole file; Down: "resolver = "2"", "members= [".
- Words in page fields: ". The quick … dog , then reads"; vscode.dev: "std", "::",
  "collections", "::", "HashMap", ";".
- A text area on Tab: "Notes edit | Contains text | Type in text."

## Not done, and why

- **Google Docs and Sheets** need a signed-in account. The agent does not sign in or create
  accounts. Probing document ids for public ones was refused by the session's safety
  classifier, rightly.
- **Cursor and Antigravity**, run with their own `--user-data-dir` and `--extensions-dir` in
  the scratchpad, both stop at a sign-in screen before the editor. vscode.dev (the same Monaco
  editor and workbench, no sign-in) stood in for them.
- **Claude desktop** shows the user's conversations; it was left alone. **Codex** is not
  installed (only its sandbox service runs).
- **Structure step 4** (`SpeechFormatter` and the buffer's lines as `Announcement`s) waits for a
  structured page-line model; the plan's Status says so.
- **Found and fixed late (`0440801`):** with focus in Chrome's address bar, a text read
  returned the page's focused object ("￼") instead of the URL. Two platform-side guesses were
  reverted first; the engine now says with each read whether the focus is in a page.
- The isolated Chrome profile's windows closed during a connection loss (about 02:50 to
  05:00); vscode.dev restored its scratch file when reopened.

## Pitfalls learned

- JAWS sends its own copy of a navigation key, injected with `dwExtraInfo` 0x45789400 (dev
  box), usually within 5 ms, but 230 ms for Ctrl+Home (JAWS runs a script first). The hook
  learns the mark from the first quick copy.
- With JAWS in browse mode, it takes arrows and letters before the page: put JAWS in forms mode
  on the field first (its own session), then run Okur.
- `okur front <title>` brings a test window forward. WScript's `AppActivate` matched other
  windows by the start or end of their titles.
- In Chrome, a mouse click on a window that is not in front only activates it; click twice.
- JAWS's first H after Ctrl+Home lands on the page's h1 (the H counts of 211–215 assume it).
- Chrome's Tab starts from its focus whatever the page's caret; IA2 `setCaretOffset` did not
  move its starting point. Okur focuses the next tab stop itself, as NVDA does.

## Next

See the plan. In short: record 208 and 210–215 (and round 2's 315–323) on old-pc; Google
Docs, Sheets and the IDEs need the user's accounts, in a session with the user.
