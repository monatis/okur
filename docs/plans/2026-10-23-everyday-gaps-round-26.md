# Plan: everyday-use gaps, round 26 (2026-10-23)

Follows [`../handoff/2026-10-23-everyday-gaps-round-26.md`](../handoff/2026-10-23-everyday-gaps-round-26.md).
The owner's order stands:
- the README's "Not yet" list, in its order;
- Okur's settings made as customizable as JAWS's
  ([`../references/jaws-customization-map.md`](../references/jaws-customization-map.md)).

Each item is found in JAWS's documentation, recorded on old-pc, then built. JAWS's accidents
become accepted divergences. old-pc always has a round queued behind the running one. Okur's
own words never use JAWS's name (AGENTS.md §1.5).

The workflow, every stretch:
1. this plan;
2. implementation;
3. proof with `okur-lab parity`;
4. a dated handoff and the next plan, with AGENTS.md pointing at them.

A stretch is a few hours or about thirty commits, not more.

## 0. old-pc

Round 282 (999-1003) is recording:
- OCR of a control, routed and clicked;
- clicking recognized text;
- Background OCR;
- the JAWS cursor in a WPF window with each capture method.

Preview and import it. Then queue S0 of the plan below, so a round is always waiting.

## 1. The Okur cursor where the screen has no text

The README's next item; Word, Outlook and Excel wait for Office's activation on old-pc. The
plan is [`2026-10-23-okur-cursor-screen-text.md`](2026-10-23-okur-cursor-screen-text.md), from
the research in
[`../references/okur-cursor-screen-text-research.md`](../references/okur-cursor-screen-text-research.md).
This stretch:
- **S1:**
  - the screen model from UI Automation;
  - a line builder from geometry;
  - the Okur cursor's keys over the model;
  - Numpad Slash and Numpad Star as clicks;
  - the tracer's screen snapshots (`"trace_screen": true`).
- **S2:** Windows.Media.Ocr in place for W, S, C, Q and ?, with the tracer's recognized text
  (`"trace_ocr": true`). Re-record 996 and 997's kind with the tracer's OCR, then compare.

## 2. Smart Navigation and Screen Layout

These take the same line builder:
- Screen Layout (986, 987);
- Smart Navigation's lines (991), and its units by part (992, 993; round 280's preview, 25 of
  66 steps).

Build them once S1's line builder exists.

## 3. Harness

- **A speech clock for the replay:** keys pressed during a Say All arrive where JAWS's speech
  was when they were pressed (505, 916, 988, 989). Use the recorder's per-utterance times or a
  pace model calibrated from them.

## 4. Still open from before

- Typing Interrupt: not observable in the speech history (988, 989).
- The performance review's P4 and C15.
- M3's remainder:
  - `okur-ui`'s lowered token;
  - the owner's sign-out and uninstall checks.
- Windows Terminal's output.

## 5. Owner

- Office on old-pc is not activated (518, 519 and 521 wait for it).
- Code signing for releases, when the owner chooses.
- The OCR plan's decisions O1-O3 (no drawing hooks, no automatic OCR, Windows' engine only)
  were taken as defaults. They are the owner's to change.
