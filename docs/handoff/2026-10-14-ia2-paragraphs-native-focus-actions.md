# Handoff: IA2 as the Data Source, Paragraphs, Native Focus, Live Page Actions (2026-10-14)

**Branch**: `longer-evidence-and-buffer-fixes`.
**Preceded by**: `2026-10-14-realistic-evidence-buffer-rewrite-ia2-traces.md`.
**Plan**: `docs/plans/2026-10-14-verification-harness-rebuild.md`.

## Numbers (tool output, pasted verbatim)

```
$ sr-cli parity | tail -1
JAWS parity (Ia2): 497/504 steps match exactly (0 scenarios could not run)
$ sr-cli parity --api uia | tail -1
JAWS parity (Uia): 423/504 steps match exactly (0 scenarios could not run)
$ sr-cli invariants | tail -1
Buffer invariants: 0 violation(s)
$ cargo test --workspace  (summed test-result lines)
tests: 801 passed, 0 failed, 199 ignored
```
(`sr-cli` is `target/agent/debug/sr-cli.exe`, built from this commit.)

At the start of this session parity was 296/449, on UIA snapshots.

## What was done

### Buffer (verified by parity)
- **Line wrapping.** JAWS's `Default.JCF` has `[HTML] MaxLineLength=150`. While more than 150 characters remain, a line ends at the first space at or after character 150. This reproduces all four recorded breaks. Chrome's layout is not involved: IA2 and UIA both put these paragraphs on one rendered line. See spec §5.
- **Paragraphs (Ctrl+Up/Down).** A paragraph is the run of lines up to the end of a block. It ends after the title, after container start and end lines, and after the last line of every block element. Inline content that doesn't finish its block continues into the next paragraph. This matches all 23 steps of 112. See spec §25.
- **Block quotes**, through IA2's role. UIA names them only in the Turkish UI locale's words ("blok alıntı").
- **I and Shift+I** skip items of nested lists (108, and the new 114).

### IA2 is the data source
- `ia2_to_aom` maps MSAA/IA2 roles, states and object attributes into the same AOM conventions as `to_aom`.
- Parity ratchets on IA2 (`sr_corpus::snapshots::PARITY_API`). `sr-cli parity --api uia` compares the two.
- Same evidence, same engine: UIA 330/449, IA2 404/449, before the later fixes.
- IA2 snapshots now record `IAccessible2::uniqueID` per node.

### Native applications (desktop scenarios 301–303)
- `sr-core::native_focus` holds the JAWS focus phrasing: container entry, role and state, value, columns, position, hint. Moving within a list drops the hint, and repeat focus events are dropped. Spec: `docs/references/jaws-native-focus-spec.md`.
- Menus (302) are driven by MSAA menu events. WinForms keeps UIA focus on the previous control during menus. Closed popups are announced on idle (`engine.on_idle`).
- The tracer records focus siblings and subtree (format 2), then the Win32 combo or list position (format 3). `run-scenario.ps1` now starts tracing before launch and reset.

### Web focus (204)
- Tab passes through to the browser. The focus event moves the virtual cursor and speaks the F/E/X phrasing plus JAWS's hint.
- Forms mode follows edit focus silently.
- `ROLE_SYSTEM_BUTTONMENU` is "button menu".

### Live use: Enter focuses page fields (user report)
- `sr-core::actions::ApplicationActions`.
- Enter on an edit or combo box requests real focus and turns forms mode on. Enter on buttons, check boxes, radios, tabs and external links clicks them. Space does the same for non-link controls.
- Windows path: the node's IA2 uniqueID goes to `accChild` on the page document (Chromium's IDs are negative and passed as-is), then `accSelect(SELFLAG_TAKEFOCUS)` or `accDoDefaultAction`. These run on a COM worker thread, never in the hook.
- `sr-cli run` and page loads now build the buffer from the IA2 capture plus `ia2_to_aom`, the pipeline parity verifies.
- Live check on old-pc (`sr-cli act`, commit ba814a5): focusing "Full name" moved keyboard focus there, and typed text landed in the field.

### Live use: caret keys in edits read text (user report)
- In an edit, Left/Right read a character, Ctrl+Left/Right a word and Up/Down a line, once the application has moved its caret. Characters use the virtual cursor's phrasing; empty text reads "blank".
- Chromium reads through IA2 (`caretOffset`, `textAtOffset`), because Chromium's UIA selection does not follow the caret in form fields. Other applications read through UIA TextPattern.
- `sr-cli caret --unit char|word|line` is the live check.
- Verified on old-pc (commit 6de2d34, with JAWS's virtual cursor toggled off so that JAWS does not take the arrows): Home "H", Right "e", Ctrl+Right "world", text area Up "first line", Down "second line".
- The page capture picks the visible tab's render window. Chrome keeps one per tab, and the first was a hidden tab with no page. It falls back to UIA when IA2 gives no page.

## Known gaps (ratchet shows them)
- **Tree view:**
  - Moving up to a visited parent drops the position (303/s6).
  - Expand and collapse are state changes, not focus changes, so they need property-change replay (303/s7).
- **Level keys from the top** also speak the enclosing h1 (104/s1, 12/s1). This needs a fixture with two h1 sections.
- **Say All** capture boundaries (01, 111).
- **First Tab into a page** (204/s1): JAWS's page-entry context varies between trials.

## Lab hygiene lessons
- `capture-snapshot.ps1` opened a new Chrome tab per capture and never closed it. With today's diagnostics, old-pc reached about 150 renderer processes. The script now closes its tab with Ctrl+W, and only while the fixture is foreground. `run-scenario.ps1` already closed its own.
- A desktop trial is ruined when a page becomes foreground between trials, because JAWS starts reading it and setup never goes quiet. This happened when the `sr-cli act` check left Chrome on the signup form. Desktop batches should run with no browser window open, or the launcher should make sure the fixture is the only candidate for the foreground.

## Next
1. The user confirms caret reading on their own machine. Then the live differential (phase 5): `sr-cli run` on old-pc with JAWS unloaded.
   - JAWS's elevated `jfw` can't be stopped from the OpenDesk session.
   - The automation needs a JAWS shutdown path, or a run in which JAWS's virtual cursor is toggled off.
2. Record JAWS's caret speech as evidence: a scenario that types into the signup form and moves with arrows. Today's phrasing follows JAWS conventions already verified elsewhere, not a recording of caret moves.
3. Property-change events (expand/collapse, check state), replayed from traces.
4. A fixture with two h1 sections for the level-key experiment.
5. Remove the remaining Explorer/Desktop special cases in `handle_focus_event` (rule 7), with evidence from new desktop scenarios.
