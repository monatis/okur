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
  end"). JAWS's counts (1032, 1482, 2035) are about a third above the copied text's (775, 1140,
  1523) and below Okur's buffer's (1054, 1512, 2072: Okur has about twice JAWS's line breaks
  there). 617 (round 82: fifteen plain 80-character paragraphs) gives the size, the count and
  the end's length; build then.
- 565, Say Color (10 steps): JAWS names a color from a table of its own ("blue2" for #0000ee,
  "lime green 2" for #008000, "dark slate gray" for #123456, where X11's nearest is midnight
  blue). Its table files are JAWS program files, not to be read: record a page of many colors
  (the CSS names and an RGB grid), Insert+5 on each, and build the table from the recording.
- 560, the Word List (Ctrl+Insert+W, W, Shift+W; 8 steps): JAWS lists the page's words by
  frequency, apparently leaving common ones out ("m (10)" first of 143). Record its list on a
  page of known words (its items and order) before building it.
- 566/s7: Insert+T in a page adds "article | level 3 | heading is City Library Expands Evening
  Hours" after the title; "level 3" is unexplained. Record Insert+T outside and inside articles
  and at other depths. (566/s3 and s10 are built: `db8c087`.)
- 567/s12: Down back onto the field just left by Up is silent in JAWS (audio without text in
  every trial), but 283/s13 reads the field returned to after other lines. The difference is
  not understood; a scenario with both orders on one form would tell. (567/s8 is built:
  `db8c087`.)
- 559 and 299, Jump to Line: in 559 JAWS's `TypeKey("1")` sent the focus back to the page (s2
  traces the document's focus), so the first jump never happened (Shift+J says "No previous
  jump"); the second ("99", Enter) went to the last line, as Okur's does. TypeKey's digits are
  not a reliable way into JAWS's elevated dialog: find another (JAWS's own `TypeString`
  function, or the edit's text set through FSAPI), and record 559 again.
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

## 5. Owner questions, answered 2026-10-20

The owner said yes to each of these and asked that Office be checked last.

- **The Defender exclusion on old-pc**: the owner adds it, from an elevated PowerShell there:
  `Add-MpPreference -ExclusionPath 'C:\Users\ailabs\sr-harness'`. Until then, check
  `Get-MpThreatDetection` before each queue (§0).
- **The development certificate (the secure-screens proposal's Q1).** The steps that change a
  certificate store or the machine are the owner's; the agent builds, signs and checks:
  1. The agent:
     - builds the release programs;
     - runs M0's E1 and M1's checks on old-pc between batches (no certificate needed);
     - writes the results into the proposal's Status section.
  2. The owner, on the dev box: `crates\okur-setup\signing\dev-signing.ps1 -NewCertificate`
     (`-WhatIf` first), which prints the thumbprint and writes `okur-dev-codesign.cer`.
  3. The agent signs the release programs with it (`-Sign -Thumbprint <THUMB>`), checks them
     with `-Verify`, and pushes them and the `.cer` to old-pc.
  4. The owner, on old-pc:
     - elevated: `dev-signing.ps1 -TrustOnLab -CertificateFile <the .cer>`;
     - then, unelevated: `okur-setup.exe install --from <folder> --no-start-after-sign-in`,
       accepting its UAC prompt.
  5. The agent runs the read-only checks of the proposal's M3 list (signature, `okur-sr.exe
     --doctor-exit`, `okur doctor --pid`, `reg query`) and records them.
  6. The owner: signs out and in, then uninstalls (`okur-setup.exe uninstall`) and undoes
     (`-UndoLab`) when the checks are done.
- **The setup's two defaults are confirmed**: install writes the sign-in Run key (old-pc uses
  `--no-start-after-sign-in`); a copy started on the secure desktop exits silently until
  secure mode (M6).
- **Office on old-pc**: the last check, after everything above; 518, 519 and 521 wait for it.
