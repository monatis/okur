# Performance and reliability review, 2026-11-02

A review of the whole codebase at `f9d7b305`, made at the owner's request before the first
Microsoft Store release ("performance optimizations and code quality optimizations, fixing
edge cases, naive implementations, anything that make okur more reliable and more
responsive"), as the reviews of [2026-10-25](performance-review-2026-10-25.md),
[2026-10-24](performance-review-2026-10-24.md), [2026-10-23](performance-review-2026-10-23.md)
and [2026-10-20](performance-review-2026-10-20.md) were.

**Who reviewed what:** three read-only reviewer agents, one after the other on the shared dev
box: the live input and event path (keyboard hook, WinEvent and UIA handlers, the converter,
the actions worker, Office's object models); the engine core (`okur-core`: engine, buffer,
cursor, Say All, application modules); speech output, Okur's own windows, the process's
lifecycle and its files.

**Method:** the reviewers read code and the corpus's traces; nothing was built or measured by
them. Each finding below was checked against the code before it was fixed; each fix has a test
that fails without it, and `okur-lab parity` was run after each. What replay cannot show was
checked live on old-pc with Okur reading in JAWS's place (`run-scenario.ps1 -Reader okur`, one
trial each, research, never evidence), before and after the fixes.

Statuses: **fixed** with the commit, or **open**.

## Findings, by user impact

Live path (L), engine (E), speech, windows and lifecycle (S).

| # | Kind | Finding | Status |
|---|---|---|---|
| L1 | Bug | Live, the WinEvent thread read no further a value change not concerning the converter's focus, which three rules act on: Excel's grid after Delete, a page's meter or progress bar, Word's and WordPad's own text changes. Replay ran without that filter, so parity and the ARIA-AT report counted them working. | fixed `a146707b` (one predicate, `live_change_kept`; a corpus test fails on any change the converter uses that the filter refuses: 272 in 21 scenarios before the fix). Live: 1123 10/10; ARIA-AT 3113 and 3116 now say the meter's value |
| L2 | Latency | Every place asked of Excel read the active cell's whole row and column (up to 512 cells, a few cross-process calls each) and every workbook name, on every Shift+arrow, empty cell and first move. | fixed `6ac5b6e9` (`SheetCells`: a move reads the cells it says). Live: the place answers 11-15 ms after its focus is read (1120). Open: Insert+Shift+F's formulas list cell by cell (`SpecialCells`), `GetIDsOfNames` per call, Office's object models on the lane every key waits on, no call cancellation |
| L3 | Bug | AltGr (Ctrl with Alt) typed nothing echoed: the at sign on Turkish Q and German; a character beyond the BMP was dropped. | fixed `4bd97729`. Not checked live yet |
| L4 | Bug | Text past 100,000 characters is cut and the caret clamped to the cut: past about 60 pages, arrows speak the last line before it. | open |
| L5 | Bug | A key the busy engine could not take in 150 ms went to the application, the Okur key and its chords included (Insert toggled Word's overtype, Insert+T typed "t"). | fixed `e0bb9a11` |
| L6 | Robustness | Half the event loop's engine calls ran outside the panic guard: one panic ended event speech for the session. | fixed `e0bb9a11` |
| L7 | Latency | Each read after an edit key polled every 15 ms with a whole read, in Word about 60 cross-process calls (font, word, Word's structure). | fixed `30aa6c08` (polls compare text, caret and selection; one whole read once settled). Live, Word's 1113: text read after a key p90 153 → 108 ms (7 reads each) |
| L8 | Bug | Excel's row and column were read from the used range's first cell, 256 at most: a cell past the 256th was not read, a cell before the used range gave no place. | fixed `6ac5b6e9` (256 cells holding the active cell) |
| L9 | Stall | The quick page capture on the WinEvent thread had UIA's default 20 s per answer. | fixed `5f22ae21` (2 s; then the page worker's) |
| L10 | Bug (suspicion) | A text read finished after Tab moved the focus becomes the new field's known text. | open (answers carry no focus identity; heals at the next read) |
| L11 | Latency | Deferred focus events are each read in full where a newer focus of the same window waits. | open (live only: replay would not see what it changes) |
| L12 | Latency | UIA's desktop-wide handlers make cross-process calls per event (no cache request). | open |
| L13 | Robustness, privacy | A clipboard read's thread per press with a hung owner; `ShowWindow` on a hung window on the actions lane; the debug log records what is typed in password fields. | fixed `5f22ae21` (one clipboard read at a time; `ShowWindowAsync`). Open: password fields (native ones do not reach the engine as such yet) |
| E1 | Bug | Say All ended at any page refresh: a news page with changing time stamps or advertisements stopped mid-article. | fixed `19e91ca4` (its lines found again, as the cursor's are) |
| E2 | Bug | Ctrl+R or F5 that reloaded nothing (Google Docs' align right) made the next refresh a reload: forms mode left, place lost. | fixed `eb32e9d2` (not in forms mode; forgotten after 10 s) |
| E3 | Bug | Word's task pane and dialog returns answered other applications' foci (Chrome's page after the Navigation pane: "<name> \| Edit", not taken in); "Edit" after Office's menu for any application. | fixed `eb32e9d2` |
| E4 | Speech | A live region's change that did not keep its old lines at its start was read whole (a log dropping its oldest line re-read every line). | fixed `9bc2ec0b` |
| E5 | Bug | Excel's answers were routed by flags: two quick moves said the second cell with the first's place and title; a reader before a move took the move's answer. | fixed `6ac5b6e9` (asks kept in order; an older cell's answer passed over) |
| E6 | Latency | Excel cell speech waited for the whole place read. | fixed with L2 |
| E7 | Robustness | A panic while speech was set aside (a quiet focus, a report during Say All) left Okur mute for the session. | fixed `eb32e9d2` (`quietly`) |
| E8 | Bug | The cursor's column kept in bytes could land inside a character after a refresh (Turkish text): Right said nothing and did not move. | fixed `19e91ca4` |
| E9 | Bug | Excel's titles are kept per window, not per sheet; titles asked before the first move hide the workbook's named ones. | open |
| E10 | Bug | Excel's readers past the platform's 256-cell window. | fixed with L8 |
| E11 | Behavior | The virtual cursor's Left and Right on tree items and sliders change the buffer only, nothing on the page (no recording). | open (record JAWS first) |
| E12 | Speech | "Leaving menus" is cut off by a cell's deferred answer. | open |
| E13 | Performance | JAWS Find lower-cases the page per search and finds one match per line. | open |
| E14-E16 | Performance | Whole-page scans per key, Word's whole-document copies, the page summary counting hidden elements. | open (under the 6.4 ms of the last sweep) |
| E17 | Robustness | Menu mode has no reset but a focus. | open |
| S1 | Bug | A chosen synthesizer that did not start (OneCore past its 5 s at a cold sign-in) ended the installed reader with no sound and no log. | fixed `e0bb9a11` (the first that starts speaks; the chosen one is not waited for again at once) |
| S2 | Store | The startup task's start was taken for the user's (no Shift escape, the wizard's Never ignored, a running copy replaced); nothing restarted Okur after an update or a crash. | fixed `80f4533e` (startup-task activation is the sign-in's; `RegisterApplicationRestart` without reboot restarts; an update's close logged). To prove live in the Store experiments (E3, E6) |
| S3 | Behavior, privacy | OneCore's speech stayed on the device it first opened (headphones plugged in, speakers kept talking). | fixed `a487b356` (`IMMNotificationClient`). Not checked live yet |
| S4 | Robustness | OneCore's synthesis failures never reached the restart; a hung synthesizer stops all speech, Stop included. | failures fixed `a487b356`. Open: a watchdog for a synthesizer call that never returns |
| S5 | Latency | SAPI enumerates every voice token on each voice switch; Voice Adjustment's Speak Sample is cut off by the settings' synthesizer starting after it. | open |
| S6 | Diagnosis | The installed reader kept no log. | fixed `80f4533e` (`%LOCALAPPDATA%\Okur\okur.log`, warnings and errors, 1 MB, one old file) |
| S7 | Data loss | An own file that did not parse (labels, PlaceMarkers, rules) was replaced by the next save; no `fsync` before the rename. | fixed `80f4533e` (set aside as `.unreadable-<time>`; written to disk before renamed) |
| S8 | Stall | okur-ui attached its input to a hung window's thread; a hung okur-ui is never noticed. | attach fixed `d04218e3`. Open: a liveness ping and restart |
| S9 | Stall | Labels, PlaceMarkers, Flexible Web and notification rules were written on the engine's thread. | fixed `d04218e3` (the settings writer thread) |
| S10 | Speech | Spelling had no stop between characters; OneCore polls every 2-4 ms while speaking; a rule's sound plays on the engine thread. | spelling fixed `d04218e3`. Open: the polls, the sound |
| S11 | Robustness | Exit took the engine with no deadline; the control pipe's read has no timeout; okur-ui's windows shared across its restarts; no restart backoff; okur-ui's threads without panic guards. | exit fixed `5f22ae21` (2 s). Open: the rest |
| S12 | Diagnosis, performance | okur-ui's stderr is null; its notifications clone a list's items. | open |
| S13 | Store | Settings under the package's file virtualization; okur-ui's foreground over an elevated window without UI Access. | open (Store experiments E2, E5) |

**Found by the live check** (not by the reviews):
- Excel's selection grown a cell at a time with Shift pressed for each chord: Shift alone
  cleared the selection read before, and the second Shift+Right said a new selection. Replay
  sends no modifier alone. Fixed `8ece8cd4`; live 1120 8/10 → 10/10, 1121 6/7 → 7/7.
- Word's drop-down form field (1057/s7, s8, s10): Word opens its list as a popup menu with no
  item highlighted and reports no focus until the first arrow; JAWS says the current choice at
  Alt+Down, and its Down moves to the next. Okur says the first item at the first Down. Open.
- A harness fix: `jaws_forget` in a run with `-Reader okur` would have started JAWS beside
  Okur (`798e8bdf`).

## Live measurements (old-pc, one trial each, research)

`okur-lab latency` over Okur's debug logs, the same five scenarios before (`f9d7b305`) and
after (`19e91ca4`), one trial each; columns count, p50, p90, p99, max in ms. Before:

```
1115-desktop-excel-moves: event read: focus                        12      31.4      36.5      43.7      43.7
1113-desktop-word-page-keys: event read: focus                         3      35.8     171.9     171.9     171.9
1113-desktop-word-page-keys: text context read                         7      85.0     153.0     153.0     153.0
510-desktop-laptop-sentence-paragraph-and-cursor-keys: text context read                         4       9.0      32.0      32.0      32.0
```

After:

```
1115-desktop-excel-moves: event read: focus                        13      36.4      81.4     222.4     222.4
1113-desktop-word-page-keys: event read: focus                         7      89.0     252.6     252.6     252.6
1113-desktop-word-page-keys: text context read                         7      89.0     108.0     108.0     108.0
510-desktop-laptop-sentence-paragraph-and-cursor-keys: text context read                         4      14.0      39.0      39.0      39.0
```

The slow focus reads after are the applications' starts and Word's closing (its four focus
events after the last key, which the run before did not log); the other Excel focus reads
were 26-47 ms, as before. One trial is too few for the rest to be more than a direction.

Okur's speech was the same on all 96 steps of the five scenarios before and after (the two
runs' `spoken` compared step by step with a scratch script; no repo tool compares two Okur
runs yet).
`okur-lab live-parity` after the fixes: 1123 10/10, 1132 13/13, 1134 7/7, 1120 10/10, 1121
7/7, 1057 8/11 (3 accepted divergences not counted), 3113 0/1 and 3116 1/2 (the meter's
values are random each run; Okur says one where JAWS said three).

## Earlier reviews' open items, re-checked

- 2026-10-25 #10 (attach to a hung thread): fixed with S8. #14 (OneCore's device): fixed with
  S3. #20 (okur-ui's stderr; the ribbon's sibling positions): open.
- 2026-10-24 #3, #6, #9a, #9c; 2026-10-20 P4 step 5 (no cancellation of a COM call that does
  not return) and T1 (the engine on the hook's thread): open.
