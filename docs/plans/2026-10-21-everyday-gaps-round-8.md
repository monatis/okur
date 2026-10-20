# Plan: everyday-use gaps, round 8 (2026-10-21)

Follows [`../handoff/2026-10-21-everyday-gaps-round-8.md`](../handoff/2026-10-21-everyday-gaps-round-8.md).
The list stays [`docs/backlog/everyday-use-gaps-2026-10-19.md`](../backlog/everyday-use-gaps-2026-10-19.md):
the gaps are closed before macOS. Each item is found in JAWS's documentation, recorded on
old-pc, then built; JAWS's accidents become accepted divergences. old-pc always has a round
queued behind the running one. Okur's own words never use JAWS's name (AGENTS.md §1.5).

The workflow, every stretch: this plan, then implementation, then proof with `okur parity`,
then a dated handoff and the next plan, with AGENTS.md pointing at them. A stretch is a few
hours or about thirty commits, not more.

## 0. What old-pc records meanwhile

| Round | What | Then |
|---|---|---|
| 110 (done, failed) | 613 again: failed its window title check (the fixture read in old-pc's code page) | record again after `f78ec7b` is pulled |
| 111 | 664: Ctrl+Backspace and Ctrl+Delete in Notepad | §3 |
| 112 | 661 and 662 again with a tracer that reads text colors (§1) | §1 |
| 113 | 541 and 524 with `-TraceTimings`, after P6 step 3 (§2) | §2 |

For each: fetch with `lab-fetch.sh`, check every trial passed and the capture script's hash
against a committed version, check each scenario's hash against the committed file, grep the
evidence and traces for links, the lab account's folders and the owner's name, compare parity
before and after, and commit the corpus alone, with the baseline from the committed code.
Before queueing more, check `Get-MpThreatDetection` on old-pc. A waiter that pulls a build is
replaced only before it starts pulling (round 105).

## 1. Say Color with real colors

- **Native edits** (661: "black on white" in the editor fixture's text box). Read UI
  Automation's ForegroundColor and BackgroundColor text attributes with the font
  (`text_context::uia_font`), and for a plain Win32 or WinForms edit without them, the colors
  it draws with by default (the system's window text and window colors). Carry them in
  `FontInfo` (optional, so older traces still load) and say them, by the color table's names.
- **A page's field in forms mode** (662: "black on white" once text is typed, "default on
  default" while empty). Read the IA2 text attributes `color` and `background-color` at the
  field's caret.
- Record 661 and 662 again with the new tracer (round 112): the evidence's speech is right,
  the old traces have no colors to replay.
- A control with no text (513/s3, a button) stays "default on default".

## 2. Performance

- **P6 step 3.** The Font dialog's worst reads still take 673 ms (perf105b, 90th percentile):
  each of the font list's 187 items costs about 3.6 ms through UI Automation's MSAA proxy.
  Read only the items on screen (a tree filter with IsOffscreen false), as JAWS's Say Window
  reads what the window shows; measure with round 113. If that is not enough, read a dialog's
  tree only when something needs it.
- The rest as before: P5 steps 4 and 5, P7, P11-P15, P2 steps 2-3, P4 steps 2 and 4-6, P9 step 3;
  the live checks never made (P2, P3, P4, P5, P8, the Window List's switch, the secure-screens
  M2 with Insert+F4).

## 3. Editing

- Import round 111 (Ctrl+Backspace and Ctrl+Delete in Notepad) and build what JAWS says for
  them (the backlog's row 22). 660/s8 records nothing of the key: the WinForms text box typed
  a DEL character.
- Shift+PageDown and Shift+PageUp were silent in the editor fixture (660/s1, s2); Okur matches.

## 4. Carried over

- **613** after round 110.
- **653's counts** are not text lengths (663); leave them.
- **Spelling marks after typing** (632/s3, s8; 640, 641): Chrome marks a word once the caret
  leaves it, after the tracer read the field. Record a variant that reads the field again
  later, or leave it.
- **The clipboard keys** (627 was inconclusive): record again with `okur-lab-keys` once the
  owner has installed it.
- **The layer's help and Command Search** (628 was inconclusive on the Turkish layout).
- **Notifications**: one recording with Focus Assist off, if the owner agrees.
- **The spoken "blank"** for an empty first line in a large selection's last 20 characters
  (620/s10-s35).

## 5. Owner

- **The development certificate on old-pc:** trust the `.cer` (`dev-signing.ps1 -TrustOnLab`,
  elevated), then `okur-setup.exe install --from C:\Users\ailabs\sr-harness\okur-signed
  --no-start --no-start-after-sign-in`; the agent then checks M3. Then copy `okur-lab-keys.exe`
  to `C:\Program Files\Okur Lab`.
- **Focus Assist off on old-pc**, for one notification recording, if the owner agrees.
- **The Defender exclusion on old-pc**, as before.
- **Office on old-pc**, last; 518, 519 and 521 wait for it.
