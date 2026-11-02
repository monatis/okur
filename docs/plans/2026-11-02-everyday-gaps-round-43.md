# Plan: everyday-use gaps, round 43 (2026-11-02)

Follows [`../handoff/2026-11-02-performance-review-fixes.md`](../handoff/2026-11-02-performance-review-fixes.md).
The owner's order: the first release through the Microsoft Store, after the performance and
reliability review (done, [`../backlog/performance-review-2026-11-02.md`](../backlog/performance-review-2026-11-02.md));
then Outlook (old-pc's needs an account), the settings' long tail, Word's open items, the review
of 2026-10-25's open findings, punctuation by word's rest, and the README's "Not yet" list in its
order.

Rules as before: each item found in JAWS's documentation, recorded on old-pc, then built;
JAWS's accidents become accepted divergences; a figure is copied from the tool's output after
the tool ran; old-pc always has a round queued; the dev box is shared (cargo on 4 cores, one
heavy job at a time, no swarms of subagents). A fix replay cannot reach is checked live with
`run-scenario.ps1 -Reader okur` and `okur-lab live-parity`.

## 1. The first Store release

1. The owner submits the release binaries to Microsoft's malware analysis (the handoff names
   them and their SHA-256), and Defender's flag is settled.
2. With the owner's leave to trust a test certificate on old-pc: the Store experiments E1-E7
   of [`2026-10-31-microsoft-store-release.md`](2026-10-31-microsoft-store-release.md), now with
   the startup task's start and the restart after an update (review S2) among them.
3. The release's text; the owner submits in Partner Center.

## 2. The review's open items, by user impact

1. Text past 100,000 characters (L4): a window of text around the caret, its offset kept.
2. A watchdog for a synthesizer call that never returns (S4), and SAPI's voice switches with
   Speak Sample (S5).
3. Excel's titles per sheet (E9); "Leaving menus" said whole before a cell's deferred answer
   (E12).
4. okur-ui: a liveness ping and restart (S8), its stderr into Okur's log (S12); the control
   pipe's read with a timeout (S11).
5. Password fields: their state to the engine (JAWS says "password edit"; record), then kept
   out of the debug log (L13).
6. Replay sending a chord's modifier alone, as a hand presses it: the live check found a bug
   (Excel's selection) that replay could not see; parity then measures what live does.
7. Live checks still owed: AltGr's echo on Turkish Q, OneCore's device change, a synthesizer's
   fallback at a cold sign-in.

## 3. Open from before

- Word's drop-down form field list at Alt+Down (1057/s7, s8, s10, live): Word highlights no
  item and reports no focus until the first arrow; JAWS says the current choice at once.
- round-1135 (Excel's monitor cells), recorded, not imported; comments; titles kept across
  sessions.
- 1125/s3, 1062/s10-s13, 1129/s11, 1130/s11, 2857/s1.

## 4. Waiting for the owner

- The malware submission; a test certificate trusted on old-pc; Defender's flag; the listing.
- Outlook: a mail account on old-pc.
- Containers on quick keys; a field's selected text on a focus (the ARIA-AT handoffs).
