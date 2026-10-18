# Handoff: old-pc round 3, partial page reads, extension groundwork (2026-10-18)

After [`2026-10-17-whatsapp-web.md`](2026-10-17-whatsapp-web.md). The user came back to
Ankara, restarted `opendesk serve` on old-pc and updated OpenDesk on both machines (0.3:
verified file transfer, background jobs, system information). This session recorded the
scenarios waiting for old-pc, fixed what their evidence showed, cut WhatsApp's page-change
stall, and started the extension SDK's groundwork.

## Commits (branch `old-pc-round-3`, not merged into `main`, not pushed)

- `9ccf0ba` harness: deploy to and fetch from old-pc through OpenDesk's verified file transfer
  (`lab-deploy.sh`, `lab-fetch.sh`).
- `3807162` corpus: round 3 recordings (209-222, 306-321; fixtures and snapshots moved in).
- `cbcd429` feat: round 3 parity (settings, partial page reads, desktop and list fixes).
- `6c39353`, `524d660`, `36a84f8` harness: JAWS's window, stuck keys and JAWS dialogs cleared
  before each trial; chosen-scenario batches.
- `b5fc94b` docs: extension SDK research and proposal (accepted by the user as the direction).
- `4a5e57a` feat(web): a page's menu bar in the virtual cursor.
- `1202c4e` fix(hooks): a commit is gated in the repository it is made in (a worktree with its
  own tree), with the user's approval.
- `c64d5f2`..`75b74fc` (14 commits, by a subagent in a worktree, merged by fast-forward and the
  worktree removed): the extension SDK's first milestone, M0. Okur's words come from an English
  Fluent catalog (262 messages, `crates/okur-core/locales/en-US/`, ids checked at compile time);
  tests fail on a word written into code and on an unused message, and a pseudo-locale run
  checks every word the engine says; announcement parts have kinds; `Settings::verbosity` (JAWS's
  levels, output modes per level, message length) replaces `access_keys`. Developer note:
  [`messages-and-verbosity.md`](../references/messages-and-verbosity.md). `parity-snapshot.sh`
  before (1202c4e) and after (75b74fc), built separately: `diff -r` empty.
- `4d144e3` harness: `run-scenario.ps1 -Reader okur` and `okur live-parity`, to measure Okur
  live on old-pc against JAWS's evidence (not run yet; the next plan's step 1).
- `c9eb59c`, `336c5fb` harness: JAWS's own record of a held JAWS key reset each trial; double
  presses kept quick; `run-batch.ps1 -FreshJaws` restarts JAWS before a batch.
- `f60b3ac` corpus: clean re-records (13 scenarios in from pending, 8 recorded again); no file in
  the corpus holds a link now.
- `24bf3fa` fix: JAWS's font size for a plain edit (its ascent: 11, 22 and 33 for Notepad's 12,
  24 and 36 point Consolas, checked on old-pc) and week numbers from Monday.
- This handoff, the next plan, the verification-loop pitfalls and the assistive AI and remote
  research and proposal: a `docs:` commit.

## Where parity stands

`okur parity` on the committed corpus (every scenario recorded, none pending):
`JAWS parity (Ia2): 1123/1220 steps match exactly (0 scenarios could not run)` and
`JAWS spoken parity: 465/647 steps match exactly`. `okur invariants`: 0 violations. 222-226
compare Okur's reading of JAWS's own dialogs (see Known limits).

## What changed

1. **Old-pc's settings in replay.** `Settings` has `auto_forms_mode` and `access_keys`
   (JAWS's Auto Forms Mode and Access Key output, on by default); replay reads both from the
   evidence's `DEFAULT.JCF` (old-pc has both off). Without Auto Forms Mode a field takes
   Enter first, and a page's menu stays the virtual cursor's. With access keys off, a menu
   item's letter and a field's shortcut ("Ctrl+L") are not said either.
2. **Partial page reads (the user's 2 s WhatsApp stall).** A page change no longer re-reads the
   whole page: the changed objects (reorder and IA2 text events) are read again with their
   subtrees and spliced into the buffer by IA2 uniqueID; the whole page only when that fails.
   Verified on old-pc with a mutating fixture; a live check on WhatsApp waits for the user.
3. **Web parity.** The first focus on a link is said as the virtual cursor's line, with the
   page title, and the table it is in is said on the next focus; "Has Popup" wording; a text
   area's line without its role word; "std::" and "io" as words; JAWS's wording for empty
   element lists and PlaceMarkers; Insert+Ctrl+R; a page's menu bar as lines ("File menu sub
   menu collapsed" ... "MenuBar end") and its browse-mode focus.
4. **Desktop parity.** Empty lines and words read with the reading keys go to the synthesizer
   as "blank" while the history keeps nothing, as JAWS's does (`speak_character`); the classic
   console's Virtual Viewer (title first, from the top, its empty rows) and its window named
   alone; a window renamed is followed (`WindowRenamed`); a plain Win32 edit's font from its
   HFONT; the task switcher's repeated reports said once; the laptop reading keys' arrows
   handed back in replay.
5. **The tracer.** Scroll-bar events and other applications' focus and changes are dropped
   before any read (a 5 s focus delay in 319; location data from the taskbar widget); events
   keep the time Windows raised them.
6. **The lab.** `run-scenario.ps1` releases every key whatever failed (a short-circuit left
   Ctrl down and opened JAWS's Research It prompt and regions list), and starts each trial with
   `clear-lab-state.ps1` (elevated) when a key is held, JAWS's window is in the way, or a JAWS
   dialog was left open. `run-batch.ps1` takes several globs. TypeKey steps (`jaws_function`)
   replay as keys.
7. **Extension SDK.** A subagent wrote [`extension-sdk-research.md`](../references/extension-sdk-research.md)
   and [`extension-sdk-proposal.md`](../references/extension-sdk-proposal.md); the user
   accepted them. The SDK waits until Okur is stable on Windows and macOS; the groundwork
   starts now: a second subagent is moving Okur's words into a Fluent catalog (JAWS's English
   exactly), typing announcement parts and adding the verbosity model, in a worktree branch,
   with parity output unchanged.

## Privacy finding

Windows 10's taskbar news and weather widget on old-pc put MSN links with the machine's
location into traces through its name changes. The new traces with them were held back and
re-recorded with the fixed tracer. Six traces committed in earlier sessions (207, 307, 308,
309, 311, 313) have such links in history; the re-record replaces the files, but history keeps
them. The user decided not to rewrite history: clean commits from now on. Before
committing any trace or evidence, search it for links and anything about the owner.

## Lab lessons

- JAWS keeps its own record of a held JAWS key, separate from Windows' key state: after a lost
  Insert release, a plain Ctrl+R opened JAWS's regions list at every trial's start. A bare
  Insert and CapsLock release resets it; trials now start with one.
- After a day of stuck keys JAWS stopped toggling Caps Lock on CapsLock pressed twice until it
  was restarted: long batches start with `-FreshJaws`, and a two-minute canary scenario (221)
  runs first.
- See [`verification-loop.md`](../references/verification-loop.md) §8 for the rest.

## Assistive AI and remote (research, for the next rounds)

The user asked for JAWS's utilities beyond basic accessibility: OCR, Picture Smart's AI
descriptions, and Tandem-style remote help, built on local GGUF models (the user co-authored
GGUF and wrote ggmlc), "Sign in with ChatGPT", and OpenDesk's pairing and rendezvous design.
A subagent wrote [`assistive-ai-and-remote-research.md`](../references/assistive-ai-and-remote-research.md)
and [`assistive-ai-and-remote-proposal.md`](../references/assistive-ai-and-remote-proposal.md)
(decisions A1-A20, milestones V0-V5 and R1-R3 after the SDK milestones). It also found a
Turkish JAWS 2025 installed on the dev box (a possible Turkish reference), and a weakness in
OpenDesk's pairing (offline guessing of the code by an active party; a PAKE would stop it).

## Known limits

- Replaying 222-226 compares Okur's reading of JAWS's own dialog, which our tracer sees thinly
  (JAWS is elevated). Okur's own dialogs need a live comparison (next plan, step 1).
- 314's Notepad window view lacks the menu bar and status bar lines JAWS shows; traces do not
  hold window text.
