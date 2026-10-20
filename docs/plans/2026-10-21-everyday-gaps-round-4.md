# Plan: everyday-use gaps, round 4 (2026-10-21)

Follows [`../handoff/2026-10-20-everyday-gaps-round-4.md`](../handoff/2026-10-20-everyday-gaps-round-4.md).
The list stays [`docs/backlog/everyday-use-gaps-2026-10-19.md`](../backlog/everyday-use-gaps-2026-10-19.md):
the gaps are closed before macOS. Each item is found in JAWS's documentation, recorded on
old-pc, then built; JAWS's accidents become accepted divergences. old-pc always has a round
queued behind the running one.

The workflow, every stretch: this plan, then implementation, then proof with `okur parity`,
then a dated handoff and the next plan, with AGENTS.md pointing at them. A stretch is a few
hours or about thirty commits, not more.

## 0. First: what old-pc recorded meanwhile

| Round | What | Then |
|---|---|---|
| 79 | 593-602 (216, 228, 283, 289, 302, 501, 504, 524, 526, 540 at Intermediate), 603-612 (the same at Advanced), 613 (the Turkish page, `turkish-text.html`) | 598 failed in its first run: read its `598-...-run.log`; import the rest; build what differs from the sources at each level |
| 80 | 614 (text analysis rules, `desktop-proofing-2.ps1`) | settle parentheses, pairing across lines, leading spaces, full stops in words and symbols such as `*` (`crates/okur-core/src/text_analysis.rs`'s open questions) |

For each: fetch with `lab-fetch.sh`, check every trial passed and the capture script is
`1a29afa8`, check each scenario's hash against the committed file (the generated ones are
committed first so their bytes are git's), grep the evidence and traces for links, the lab
account's folders and the owner's name, compare parity before and after, and commit the corpus
alone, with the baseline from the committed code.

Before queueing anything else, check `Get-MpThreatDetection` on old-pc: if `okur.exe` was
quarantined again, push a fresh build and queue the failed rounds again, and remind the owner
of the exclusion (§5).

## 1. Recorded, to build

- 562: a large selection. JAWS reads it whole up to a size, then says "selected N characters up
  to" and the selection's last characters (s13: "selected 1032 characters up to ent information
  end"). Find the size and how the end is cut from s2, s8, s13, s17, s20, and how JAWS counts
  (link roles inside a list item are read in s8).
- 565: Say Color's names (10 steps).
- 560: the Word List (8 steps).
- 566: Speech On Demand ("Not available in Full Speech mode"), and Insert+T's extra context (3).
- 567/s8 (Insert+Numpad5 said "Email"), 567/s12.
- 559 and 299: Jump to Line's targets; replay of its dialog's TypeKey keys.
- 552-555: what still differs at each verbosity level on the forms fixture (7 steps each).

## 2. Verbosity, beyond what is built

- The hints on Tab: the verbosity rounds have "Type in text." on Tab in page forms at every
  level, 204 has none, even on a fresh JAWS (round 72). Compare the JCF files run-batch writes
  for a level (`jaws-settings.ps1`) with old-pc's own: a key it writes may turn the hints on.
- More of JAWS's output types (its shipped `Default.JCF` `[OutputModes]`, listed in
  `docs/references/messages-and-verbosity.md`): add each when 593-612 show what it switches.
- The left edge of a table row (Ctrl+Alt+Left on the first column) still says "edge of
  table", which no recording has: record it.

## 3. Windows and dialogs

- A DPI-unaware window's rectangles at 125% (516/s8, 513/s8). While no batch runs, probe on
  old-pc which windows UI Automation reports unscaled: the dialog fixture (WinForms), Notepad
  and a Win32 dialog, through `okur doctor` or a small probe. Then scale those by the window's
  DPI (`LogicalToPhysicalPointForPerMonitorDPI`) in the platform layer, live and in the tracer,
  and record 513 and 516 again.
- Say Window and group boxes (540/s4, 544/s13); an unlabelled icon's name (537/s8, 544/s13);
  WordPad's ribbon (523); toasts (533): as round 3's plan §4 says.

## 4. Performance and live checks

- The review's open items (`docs/backlog/performance-review-2026-10-20.md`):
  - P3, rebuilds under the engine lock;
  - P5 steps 2, 4, 5;
  - P6, P7 and P11-P15;
  - P2 steps 2-3; P4 steps 2 and 4-6; P9 step 3.
  - Each has replay-equivalence tests and `parity-snapshot.sh` before and after.
- Live checks, never made:
  - P2 (the hook installed again after a slow key);
  - P4 (a big page, a hung application);
  - P5 (305, 313, 400, 403, 504, 522 live);
  - P8 (a web application whose page changes);
  - the Window List's switch;
  - the secure-screens M1 (one Okur per desktop, `okur quit`) and M2 (Insert+F4, Escape,
    Insert+Tab; Insert+F4, Enter).

  On old-pc with `okur live-parity` where a scenario exists. The rest need the owner's
  permission on the dev box.

## 5. Owner questions

- Windows Defender on old-pc quarantines `okur.exe` (10:52 and 14:13 on 2026-10-20). Only the
  owner can add the exclusion, from an elevated PowerShell there:
  `Add-MpPreference -ExclusionPath 'C:\Users\ailabs\sr-harness'`.
- The development certificate (the secure-screens proposal's Q1). With a yes, old-pc's M3
  checks follow the proposal's Status section.
- Two defaults of the setup to confirm: install writes the sign-in Run key (use
  `--no-start-after-sign-in` on old-pc); a copy started on the secure desktop exits silently
  until secure mode (M6) exists.
- Office on old-pc still needs activating; 518, 519 and 521 wait for it.
