# Plan: the second review before cross-platform, then JAWS's settings and help (2026-11-03)

Follows [`2026-11-02-windows-ui-round-2.md`](2026-11-02-windows-ui-round-2.md) (its File
Explorer ribbon round done, `a1962fd3` to `9e9af731`) and the public Store submission
([`../references/store-submission-kit.md`](../references/store-submission-kit.md), submission
2, Okur 1.1.0.0, submitted by the owner on 2026-11-03).

The owner, 2026-11-03: "review performance and project structure, identify performance gaps or
naive implementations, anything that can be implemented better, and a second pass for
refactoring before considering to go cross-platform. identify any crate structure that can be
refactored better to support cross-platform functionality. after that work, you can work on
other JAWS features. that you can discover from the settings center or help / user manuals.
I'll let you know when the update goes live."

**Status (2026-11-04,
[`../handoff/2026-11-04-progress-bars-and-tool-tips.md`](../handoff/2026-11-04-progress-bars-and-tool-tips.md)):**
§1 done (the review in
[`../backlog/performance-review-2026-11-03.md`](../backlog/performance-review-2026-11-03.md), the
structure's revision in the proposal's §13); §2.1 begun: P1 (in part), P2, P5, P6, P7, P9, P10,
P11, P14 and T1 to T3 fixed, the macOS stubs deleted. Step 5a done (`30725cf6`, `0553d7f8`, `c2b69712`,
`93709ed9`: engine references to application modules 85 -> 23, no window class left in the
engine); step 4 done (`05bc9c0c` to `573bc638`: `okur-runtime` runs the screen reader through
the ports a platform gives it; the Windows driver is gated at its root; the workspace checks
for Linux with no warning), parity identical throughout, live checks on old-pc. Next: the
review's open items (P4, P8, P3, P1's pieces, P15; Word's and Excel's live checks wait for
Office on old-pc), then JAWS's features from Settings Center and the help (§3); the rest of
the proposal's §13.3 before macOS. §3 begun: JAWS's progress bar announcements recorded (793, 1136)
and built (`29a4c730`), tool tips recorded (1137); the setting as Settings Center has it
(`6292eb7a`); Say Window outside dialog boxes (`13d91c1f`, 548/549/554/1136 recorded again);
comctl32's balloon tips and password edits (`408d2fb4`, `c56173d8`, 1138)
([`../handoff/2026-11-04-say-window-and-balloon-tips.md`](../handoff/2026-11-04-say-window-and-balloon-tips.md)).
Phonetic spelling after a pause recorded and built (`397196fe`, 1140, 1141); Say Blank Lines
During Say All found to change nothing in a text box (1139, recorded again). Next: key labels
and key echo, the quick key delay.

Rules as before: a finding is checked against the code before it is fixed; a fix has a test
that fails without it; a refactor changes nothing a user hears (the proposal's proofs P1 to P5,
§11.1); figures are copied from tools; the dev box is shared (cargo on 4 cores, one heavy job
at a time); old-pc always has something queued.

## 1. The review (second pass)

1. Two read-only reviewers, at once (neither builds): performance and naive implementations;
   structure, layering and cross-platform readiness. Both start from what the earlier reviews
   left open ([`../backlog/performance-review-2026-11-02.md`](../backlog/performance-review-2026-11-02.md))
   and from the accepted crate structure
   ([`../references/crate-structure-proposal.md`](../references/crate-structure-proposal.md),
   steps 0 to 3 done).
2. A live latency baseline on old-pc before any change: Okur reading in JAWS's place, one trial
   each of 1115, 201, 1113, 510, 106 (the 2026-11-02 set) and 3155, 3158, 3159, 3161 (Windows'
   own UI), with `okur-lab latency` over the debug logs. It is also step 4's "before" (P4).
3. The findings, each checked by hand, in
   `docs/backlog/performance-review-2026-11-03.md`, ranked by user impact; the structure's in
   the proposal as its second pass.

## 2. Fixes and the structure

1. The review's fixes, in order of user impact, each with its test and `okur-lab parity`.
2. The crate structure's steps 4 to 6 (the owner's answer 3: after the Store install round,
   before macOS), revised by the second pass: `okur-runtime`, the engine no longer naming
   applications (SDK M2's first half), `okur-apps`. Step 4 needs the live check (P4).
3. Step 7: the rules to blocking.

## 3. Then: JAWS's features, from Settings Center and the help

1. Import round 1135 (Excel's monitor cells, recorded 2026-11-02), and build them.
2. Settings Center's groups beside Okur's: every JAWS option, its default and its place, found in
   JAWS's help ([`../references/jaws-reading-settings-research.md`](../references/jaws-reading-settings-research.md)
   for what is already known); those Okur lacks, ranked by everyday use, recorded on old-pc,
   then built. Leads from the backlog: Say All's capitals and blank lines (row 26), key labels
   and key echo (27), screen echo, key repeat and typing interrupt (39), audio ducking, the
   speech dictionary and language switching (43), the quick key delay (21), progress bars and
   tool tips (33).

## 4. Waiting for the owner

- The Store's certification of 1.1.0.0. When it is live: the README (the Store link, the
  status, "Trying it" from the Store, figures rerun), and on old-pc experiment E6 (the Store
  updates 1.0.0.0 to 1.1.0.0 and Okur restarts by itself).
- Outlook: a mail account on old-pc.
