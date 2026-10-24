# Plan: everyday-use gaps, round 28 (2026-10-24)

Follows [`../handoff/2026-10-24-scan-cursor-and-file-ocr.md`](../handoff/2026-10-24-scan-cursor-and-file-ocr.md).
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

- **Round 298** records 1033 (F with two files selected) with the tracer at `d63882d7`.
  - Check every text in its traces and file snapshots before import (the privacy check).
  - Expect "OCR Failed to start. Multiple files currently selected." in Okur's message box.
- **Queue next:** the recordings §1 needs, one round at a time.

## 1. Convenient OCR, what is left

From [`2026-10-23-okur-cursor-screen-text.md`](2026-10-23-okur-cursor-screen-text.md) S4 and the
handoff's open steps, in order:

1. **The Results Viewer's "Open in Word..." link** (known gap,
   `the_results_viewer_ends_with_open_in_word`; 1024/s7-s9, 1025/s6-s12).
   - Record first what JAWS does on Enter there: whether Word opens with the text, and what JAWS
     says. Word 2016 is on old-pc; check its first-run state before a round.
   - Then the viewer's last line, a same page link. Okur opens the text in Word when Word is
     installed. Without Word, the line is left out rather than promise what Okur cannot do.
2. **D in a PDF reader.** Okur recognizes the PDF open in front.
   - Edge's viewer: the file's path from its address.
   - Adobe Acrobat: needs a working Acrobat Reader on old-pc, which is the owner's to install
     (§5).
   - Then Okur says JAWS's sentences where no PDF is open (1031/s3), and its divergence goes.
3. **A scanned page's paragraphs.** JAWS's engine joins a page's lines into paragraphs
   (1025/s4). Try grouping Windows' recognized lines by their spacing and indent. Keep it only
   if a ground-truth set of scanned letters reads better by paragraph than by line, measured,
   not by look.
4. **1032/s22:** back from the help, Okur names a WPF field after its window ("SR WPF Text
   Fixture Edit"); JAWS gives it no name.
5. **The OCR voice:** an alias of the Okur cursor's voice, its pitch lower, until voice
   aliases exist.
6. **The replay's clock for Background OCR:** a refresh due by the replay's time, so 1014
   measures the refresh rather than passing by chance.
7. **Research §9's open items:**
   - OCR under Screen Shade;
   - the Scan Cursor in Calculator and Settings. Old-pc has the owner's Calculator and
     Settings windows open: a fixture must not close them. Use Windows' own WPF or UWP sample
     apps instead, or ask.

## 2. Smart Navigation and Screen Layout

On the same line builder (`screen_model::lines`), from plan 26 §2: Screen Layout (986, 987),
Smart Navigation's lines (991) and its units by part (992, 993).

## 3. Harness

- A speech clock for the replay: keys pressed during a Say All arrive where JAWS's speech was
  when they were pressed (505, 916, 988, 989).

## 4. Still open from before

- The hardening review's open findings
  ([`../backlog/performance-review-2026-10-24.md`](../backlog/performance-review-2026-10-24.md)):
  1 (labels and Flexible Web rebuilt in the hook), 3 (Chromium live regions read on the
  WinEvent thread), 4 (attribute runs), 6 (page changes dropped), 9a-c and 9e.
- Typing Interrupt: not observable in the speech history (988, 989).
- M3's remainder:
  - `okur-ui`'s lowered token;
  - the owner's sign-out and uninstall checks.
- Windows Terminal's output.

## 5. Owner

- Old-pc's Adobe Acrobat Reader is broken (no `AcroRd32.exe`). D in Acrobat can be recorded once
  it is reinstalled, which is a Program Files install and so the owner's decision.
- The README's next "Not yet" item after OCR of files: Microsoft Word, Outlook and Excel.
