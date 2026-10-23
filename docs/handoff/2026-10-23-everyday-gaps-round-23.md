# Handoff: everyday-use gaps, round 23 (2026-10-23)

Follows [`2026-10-22-everyday-gaps-round-22.md`](2026-10-22-everyday-gaps-round-22.md) and covers
the commits after it (`14aa484` to `c80d313`, and this handoff's), under the plan
[`docs/plans/2026-10-22-everyday-gaps-round-22.md`](../plans/2026-10-22-everyday-gaps-round-22.md).
The next plan is
[`docs/plans/2026-10-23-everyday-gaps-round-23.md`](../plans/2026-10-23-everyday-gaps-round-23.md).

## The owner's requests

- **A review of the whole code for inefficiencies, lags, stalls, whole reads where part would
  do, naive implementations, memory waste and smelly architecture, fixed** (overnight,
  2026-10-22). Three reviewers read the code at `a0628bf4` (threading and latency, the core
  engine, the platform layer); their 69 findings and what became of each are in
  [`docs/backlog/performance-review-2026-10-23.md`](../backlog/performance-review-2026-10-23.md).
- **The rest of verbosity**, the README's "Not yet" item: rounds 259 and 261 recorded scenarios
  at Intermediate and Advanced; what they showed is built.
- **The UI Access probe's install, as a script to run elevated, with exact steps**: the script
  is `sr-harness\signing\install-notify-probe.ps1` on old-pc (it checks elevation, the probe's
  SHA-256 `21FD0659...C86E` and the development certificate's thumbprint before copying it to
  `C:\Program Files\Okur Lab`); the steps are in the session's last message and in the next
  plan. Not run yet.

## Decided without the owner (reversible)

- **Recordings that hold an Explorer window open on old-pc are not imported.** Old-pc had a
  window open on `C:\Users\ailabs\Dropbox\migration` through rounds 259 and 261; it is in the
  window lists and traces of their desktop scenarios. Those fourteen (843-846, 849-854,
  859-862) stay pending; round 262 records them again by itself once no Explorer window shows
  that folder. Nothing on old-pc was closed.
- **836's first trial is kept as recorded**: JAWS said "No headings" mid-page in it (the page
  not loaded); the other two agree with Okur.
- **"N Smart Glance Highlights"**, Smart Glance's short message, joins its accepted divergence
  (847, 848, 855-858); Escape out of the Speech History at both levels joins its own (863,
  864).

## What was done

### The performance review

Fixed, each with tests where a test can hold it, and the three parity runs byte-identical
before and after every engine change (`skills/jaws-reference/scripts/parity-snapshot.sh`, then
`diff -r`: identical, each time):
- **Crashes and stalls** (`ad0be50`): a WinEvent delivered inside another's read (Windows
  delivers them while a cross-process read waits) aborted the reader; events now wait their
  turn. The clipboard is read on a thread of its own, waited for 150 ms, within its block's
  size. A drag no longer sleeps in the keyboard hook. `EM_GETSEL` through its out-pointers. A
  panic while the engine was held left Okur silent; the lock is recovered, and a key's or an
  event's panic is caught and logged.
- **The engine's hot paths** (`ccf6737`, `b76fab9`, `4a56c7e`): skim reading by attributes
  sweeps the page once (it asked every byte's formatting of every span); the sentence keys make
  only nearby sentences; key bindings looked up by key; Z collects offsets only; a page is
  marked with the reading choices before the host builds it, so a refresh is not built again
  under the engine; typed field text follows its field into a refreshed page; a state change
  keeps the page's indexes; ancestors found through the index; a live region's text compared in
  place; the edit keys find the caret's line without making the field's text a vector.
- **Platform and threading** (`609b381`, `b76fab9`, `4fd2617`): fewer UI Automation calls per
  event (the focus asked for once, the MSAA object read once, the text sent once instead of
  five times, column headers in one call, the page's document found through Chromium's page
  window, one UIA client per capture thread); a stop cuts a long text off between its parts;
  Speech History entries capped at 32 KB; the Virtual Viewer's line starts kept per text;
  Okur's lists refilled with drawing off; mouse moves no longer put off the idle work;
  stopping waits at most 2 s for stuck threads.
- **Not changed, with why**, and **open** (T1, the engine in the keyboard hook, with a plan;
  P3's polls; P4; P5's capture off the WinEvent thread; P6): in the review's file.

**Live check** (live run 260, old-pc): the build before the review and the build at `b76fab9`
read nine scenarios. Speech was the same step for step in eight; in 103 the build before was
off by a heading from its first key (the first run after a recording round), and the build
after matched JAWS 8/8 (`okur-lab live-parity`). `okur-lab latency` over each build's logs:
a state change's read p50/p90 30.0/41.5 ms before, 16.9/27.0 after; a text read after a key
p50 29.0 before, 19.0 after (p90 unchanged, 261-263: the settle time at text boundaries,
P3's polls). The commits after `b76fab9` (C4's line, P14's headers) are not checked live yet.

### Verbosity

- **Short messages above Beginner** (`c24c07a`, round 259): "No more headings at level2 in
  section" and "No more headings at level1" (no "this", no period); "No onMouseOver
  elements"; Insert+Z's "Off" and "On"; the page summary without "Page has"; Insert+Escape's
  "Screen updated" left out at Advanced, a message of JAWS's own.
- **Append Selected Text to Clipboard** (`c80d313`, round 261): "Appended" at Intermediate,
  nothing at Advanced, as "Copied" (its evidence, 859-862, waits for round 262).
- What else the levels change: [`docs/references/messages-and-verbosity.md`](../references/messages-and-verbosity.md).

### Corpus

- Round 259 (`a2a29dc`): 22 web scenarios at Intermediate and Advanced (827-842, 847, 848,
  855-858). Round 261 (`400604d`): 863, 864.

## Figures

Each is the named command's output at `c80d313`:

- `okur-lab parity`: "JAWS parity (Ia2): 6847/7096 steps match exactly; 187 accepted
  divergences not counted (0 scenarios could not run)" (6655/6888 at `a0628bf`; the corpus
  grew by 24 scenarios).
- `cargo test --workspace`: 1376 passed, 0 failed, 199 ignored.

## old-pc

- Round 262 (fourteen desktop scenarios recorded again) waits for no Explorer window to show
  `Dropbox\migration`, for up to 16 hours (`batches\2026-10-21-waiter262.log`).
- `sr-harness\bin\okur.exe` and `okur-ui.exe` are the build at `b76fab9` (live run 260).
- The UI Access probe is not installed yet: `install-notify-probe.ps1`, by the owner.
