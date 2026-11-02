# Handoff: the Store submission, the reading highlight, and the demo video (2026-11-02)

Follows [`2026-11-02-performance-review-fixes.md`](2026-11-02-performance-review-fixes.md).
Plan: [`../plans/2026-11-02-everyday-gaps-round-43.md`](../plans/2026-11-02-everyday-gaps-round-43.md).

## What was done

1. **The first Store submission** (`6996c926`). The Partner Center draft was filled from
   [`store-submission-kit.md`](../references/store-submission-kit.md); the owner chose the
   customer group, agreed to IARC's terms and submitted. **Certified on 2026-11-02 and
   available to the private group** (the owner's word). Its test on old-pc (the question of §8
   of the research: does the Store's signature give `okur-sr.exe` UI Access) is next.
2. **README** (`4e06a6fc`, `ac9d5838`): a short status, the logo, a privacy badge and section,
   "What works today" in a paragraph with the item-by-item list moved to
   [`what-works-today.md`](../references/what-works-today.md), Outlook as not yet checked, and
   the figures rerun at `6996c926`.
3. **The reading highlight** (`222675de`): JAWS's visual tracking. After each move of the
   virtual cursor (a key, or Say All's next line) its element is scrolled into view
   (`IAccessible2::scrollTo`) and framed in red by `okur-ui`; `visual_tracking = false` turns it
   off. JAWS observed on old-pc framing a table cell and scrolling to it (research, OBS frames).
   Live on old-pc: debug log "reading highlight placed" 83 to 99 ms after each move.
4. **"clickable" in tables** (`5e78bb95` evidence, `ac47ad4b` engine): JAWS says it in a table
   that answers clicks itself, not under a page-wide listener (GitHub's README tables)
   [obs, 3154/s1-s6]. `okur-lab parity`: `JAWS parity (Ia2): 11383/11955 steps match exactly;
   365 accepted divergences not counted (0 scenarios could not run)`; `JAWS spoken parity:
   11125/12021`.
5. **A page still loading** (`e5fc8df0`): twice in five loads of the GitHub page from the
   address bar, Chromium gave the page the focus while it loaded and Okur said "Page has 1
   Region, 1 heading and 2 links", reading a 6-line page. A new page focused while Chromium
   marks it busy now waits (at most 5 s). **Not yet seen to wait:** in five loads since (two
   with an emptied cache), the focus came after the load (`new page focused ... loading=false`
   in the debug log), and every summary was the whole page's.
6. **The demo video** for the owner's LinkedIn post: recorded live on old-pc with OBS and Okur
   reading github.com/monatis/okur (H to "Why re-implement JAWS?", Ctrl+Down through its two
   paragraphs, T and Ctrl+Alt+arrows through the figures table), edited with Remotion (camera
   following the highlight's places, captions timed by the recording's sound, keycaps, chapter
   labels, spoken title and closing cards). The recording tools stay outside the repository:
   OBS and `take.ps1` in `D:\okur-video` on old-pc; the Remotion project, its scripts and the
   takes in the shared Dropbox, `sr-harness\video-project\2026-11-02-github-readme\`.

`cargo test --workspace` at `e5fc8df0`, its 35 `test result:` lines summed: 1612 passed,
0 failed, 201 ignored. Each commit passed the commit gate.

The video: `okur-demo.mp4`, 99.0 s, 1920x1080 H.264 with AAC (ffprobe). Speech levels measured
by `levels.mjs` on the render's sound: title narration -20.8 dBFS, Okur's voice -21.2, closing
narration -21.2 (Okur's recording was raised 9 dB and the narration 3 dB to meet).

## Found and not yet done

- The reading highlight frames the element, not the line's text (a known-gap test); pages read
  through UIA are not tracked; no Settings Center entry until JAWS's own control is observed.
- Chrome's address bar on Ctrl+L: Okur says "Address and search bar Edit about:blank Type in
  text. Ctrl+L Ctrl+L", the accelerator twice; JAWS not yet recorded there.
- F5 on a page Okur reads in the virtual cursor gave no announcement live (once, GitHub's page,
  2026-11-02); `handle_document_loaded` announces a reload, so the load itself did not reach
  the engine. To record and check.
- The page-load wait of item 5 needs a load that catches Chromium busy, and the engine's path
  for a page that arrives after its focus a live check.
- On old-pc, OBS runs hidden in the tray for retakes; close it before long recording rounds.
