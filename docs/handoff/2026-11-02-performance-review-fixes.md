# Handoff: the pre-release performance and reliability review, fixed and checked live (2026-11-02)

Follows [`2026-11-01-excel-named-titles-and-totals.md`](2026-11-01-excel-named-titles-and-totals.md)
and covers `a146707b` to this one, made at the owner's request before the first Store release:
"performance optimizations and code quality optimizations, fixing edge cases, naive
implementations, anything that make okur more reliable and more responsive".

Where things stand: three review passes, 14 commits (13 of fixes, one of the harness), each fix
with a test that fails without it; the fixes that replay cannot reach checked live on old-pc. The review and every
finding's status: [`../backlog/performance-review-2026-11-02.md`](../backlog/performance-review-2026-11-02.md).
The release binaries for the owner's malware submission are built (below).

## What was fixed

The live path:
- **The live filter starved three rules** (`a146707b`): Excel's blank cell after Delete, a page's
  meter, Word's own text changes. Replay ran without the filter, so parity counted them working;
  a corpus test now fails on any change the converter uses that the filter refuses.
- **Excel reads what it says** (`6ac5b6e9`): a move asks for the active cell, its titles' cells and
  the selection's edges, not 512 cells and every name. The engine keeps its asks in order, so two
  quick moves no longer say a cell with the other's title.
- **Word's polls after a key** compare the place alone, then read it whole once (`30aa6c08`).
- **AltGr's characters are echoed** (`4bd97729`): the at sign on Turkish Q.
- **A busy engine keeps the Okur key from the application**; the event loop is panic-guarded
  (`e0bb9a11`).

The engine:
- **Say All reads on across a page refresh**; the cursor stays on a character's start
  (`19e91ca4`).
- **Ctrl+R in forms mode is the page's**; a reload asked and not come is forgotten. Word's
  modules answer only Word's foci. Speech comes back after a panic while it was set aside
  (`eb32e9d2`).
- **A live region says what it added** (`9bc2ec0b`).

Speech, windows and lifecycle:
- **A synthesizer that does not start gives way** to the first that does; it had ended Okur
  silently (`e0bb9a11`).
- **The installed reader keeps a log** (`%LOCALAPPDATA%\Okur\okur.log`); **own files that do not
  parse are set aside**; **the Store's startup task is a sign-in start** and Windows restarts Okur
  after a crash or an update (`80f4533e`).
- **OneCore follows the default output** and its failures count toward its restart (`a487b356`).
- **Own files are written off the engine**, spelling stops between characters, okur-ui does not
  attach to a hung window (`d04218e3`).
- **Bounded waits** for a busy browser's first capture (2 s), a hung window's restore, the
  clipboard and the exit (`5f22ae21`).

Found by the live check:
- **Excel's selection a cell at a time** with Shift pressed for each chord (`8ece8cd4`): Shift alone
  had cleared the selection read before. Replay sends no modifier alone, so parity could not see
  it.
- A harness fix: `jaws_forget` in a run with `-Reader okur` would have started JAWS beside Okur
  (`798e8bdf`).

## Proof

`okur-lab parity` at `5f22ae21` (unchanged through every fix; the baseline file never moved):

```
JAWS spoken parity: 11119/12015 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 11377/11949 steps match exactly; 365 accepted divergences not counted (0 scenarios could not run)
```

`cargo test --workspace --no-fail-fast` (4 test threads): 1608 passed, 0 failed, 200 ignored
(summed from cargo's 35 result lines; 1587 at the last handoff). `okur-lab invariants`: "Buffer
invariants: 0 violation(s)". `okur-lab corpus-manifest`: "Corpus manifest is current."

Live on old-pc (`okur-lab live-parity`, Okur reading in JAWS's place, one trial each, research):

```
1123-desktop-excel-typing-in-cells: Okur live: 10/10 steps match JAWS exactly; 0 accepted divergences not counted
1132-desktop-excel-titles-and-totals: Okur live: 13/13 steps match JAWS exactly; 0 accepted divergences not counted
1134-desktop-excel-named-titles: Okur live: 7/7 steps match JAWS exactly; 0 accepted divergences not counted
1120-desktop-excel-selection-from-a1: Okur live: 10/10 steps match JAWS exactly; 0 accepted divergences not counted
1121-desktop-excel-selection-from-b2: Okur live: 7/7 steps match JAWS exactly; 0 accepted divergences not counted
1057-desktop-word-protected-form-change: Okur live: 8/11 steps match JAWS exactly; 3 accepted divergences not counted
```

1120 and 1121 were 8/10 and 6/7 before `8ece8cd4`. The latency before and after, and the
meter's live runs (3113, 3116), are in the review document.

Not checked live yet: AltGr's echo, OneCore's device change, the Store's start and restart (the
Store experiments), a synthesizer's fallback at a cold sign-in.

## The release binaries

`packaging/msix/build-msix.ps1 -Version 1.0.0.0`, release profile, built from `5f22ae21` (the
package without UI Access, as the Store experiments have not proven it), on the dev box. SHA-256
(`sha256sum`):

```
d175d71eb47a50d6c659c5c6f251c45d78e9bd9727814433c3d7c89dd3a40c99 *target/msix/okur-1.0.0.0-x64.msix
193cf9214c2d95f3ab70ba63d7acd7e03b307f93e01c31d14007cfb5c4eae543 *target/msix/stage/okur-sr.exe
27cfef661baa0837d8fcf14497465d0a5ed6b55070ddda6c96bb79e0db1450aa *target/msix/stage/okur-ui.exe
d0dc69c302648fa7d60b917e11cfa771753bd4282e930ee7ff2f2efe072a9601 *target/release/okur-sr.exe
```

The package's `okur-sr.exe` is `target/release/okur-sr-no-uiaccess.exe` under the reader's
name; `target/release/okur-sr.exe` is the build with UI Access, for the unpackaged installer.
These are the files for Microsoft's malware analysis; they are unsigned (the Store signs what
it publishes). A later commit changes the binaries: submit these, or build again at the commit
submitted and note its hashes.

## Open

From the review (statuses in its table): text past 100,000 characters (L4); Excel's titles per
sheet (E9); the virtual cursor's Left and Right on trees and sliders (E11, record first);
"Leaving menus" cut off by a cell (E12); a watchdog for a synthesizer call that never returns
(S4); SAPI's voice switches and Speak Sample (S5); okur-ui's liveness and stderr (S8, S12); the
control pipe's timeout (S11); password fields in the debug log (L13). From the live check:
Word's drop-down form field list at Alt+Down (1057/s7, s8, s10). From before: round-1135
(Excel's monitor cells) recorded, not imported; 1125/s3, 1062/s10-s13, 1129/s11, 1130/s11,
2857/s1.

## Waiting for the owner

- The malware submission of the release binaries (Microsoft's), which the owner makes.
- A test certificate trusted on old-pc for the Store package's experiments (E1-E7); Defender's
  flag; the privacy policy's URL and whether the listing may compare Okur with JAWS.

## old-pc

Idle after the third live check; JAWS running, its settings as before every round, volume 1%,
Rate 90. Okur's latest debug build (`8ece8cd4`) in `sr-harness\bin`.

## Next

[`../plans/2026-11-02-everyday-gaps-round-43.md`](../plans/2026-11-02-everyday-gaps-round-43.md).
