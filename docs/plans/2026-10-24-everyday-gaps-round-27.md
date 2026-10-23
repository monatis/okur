# Plan: everyday-use gaps, round 27 (2026-10-24)

Follows [`../handoff/2026-10-24-okur-cursor-screen-text.md`](../handoff/2026-10-24-okur-cursor-screen-text.md).
The owner's order stands:
- the README's "Not yet" list, in its order;
- Okur's settings made as customizable as JAWS's
  ([`../references/jaws-customization-map.md`](../references/jaws-customization-map.md)).

Each item is found in JAWS's documentation, recorded on old-pc, then built. JAWS's accidents
become accepted divergences. old-pc always has a round queued behind the running one. Okur's own
words never use JAWS's name (AGENTS.md §1.5).

The workflow, every stretch:
1. this plan;
2. implementation;
3. proof with `okur-lab parity`;
4. a dated handoff and the next plan, with AGENTS.md pointing at them.

A stretch is a few hours or about thirty commits, not more.

## 0. old-pc

- **Round 287** records 1012 and 1015 with the fixture-only, English snapshots. Check every
  snapshot's text before import (the privacy check), then compare with
  `okur-lab parity --only 1012` and `--only 1015`.
- **Queue next:** the S0 items still open (research §9):
  - OCR under Screen Shade;
  - the Scan Cursor in Calculator and Settings;
  - the OCR voice in the recorder's fragments.

## 1. The experimental screen text

From [`2026-10-23-okur-cursor-screen-text.md`](2026-10-23-okur-cursor-screen-text.md) §0, in
order:

1. **The Scan Cursor's way in modern windows**, where JAWS has one (1002, 1013's 4 of 17).
   - In a window the Virtual Viewer lays out by elements (WPF, UWP, WinUI), the model is one
     element per line, in the window's order, with its role, as `viewer_text::elements`.
   - Okur's words for JAWS's: "Okur scan cursor" and "Route Okur scan to PC".
   - Right and Ctrl+Right move inside the element.
   - Re-measure with 1013.
2. **The layer's ? help and the OCR voice:** an alias of the Okur cursor's voice, its pitch
   lower, until voice aliases exist.
3. **Graphics:** nothing is said of unlabeled pictures, as JAWS's default (1007). With
   `[OSM] IncludeGraphics=2` JAWS says "graphic" and a checksum (1008). Whether Okur offers it
   is the owner's choice, since the number is JAWS's own.
4. **Files and PDFs to the Results Viewer** (S4; the assistive proposal's V2).
5. **The replay's clock for Background OCR:** a refresh due by the replay's time, not the
   machine's, so 1014 measures the refresh rather than passing by chance.

The feature stays experimental, off by default, until the owner decides it is proven.

## 2. Smart Navigation and Screen Layout

On the same line builder (`screen_model::lines`), from plan 26 §2: Screen Layout (986, 987),
Smart Navigation's lines (991) and its units by part (992, 993).

## 3. Harness

- A speech clock for the replay: keys pressed during a Say All arrive where JAWS's speech was
  when they were pressed (505, 916, 988, 989).

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
- The OCR plan's decisions O1-O3, taken as defaults: no drawing hooks, no automatic OCR,
  Windows' engine only.
- When the experimental screen text counts as proven, and goes from Experimental to Reading.
