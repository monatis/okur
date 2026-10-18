# Plan: JAWS's helper dialogs, round 2, on the extension groundwork (2026-10-18)

Follows [`docs/handoff/2026-10-18-old-pc-round-3.md`](../handoff/2026-10-18-old-pc-round-3.md).
The user's direction (2026-10-18): with the complex web applications in place, go on with
JAWS's helper dialogs (the Links List and a simple settings dialog exist; the rest of
[`2026-10-17-helper-dialogs.md`](2026-10-17-helper-dialogs.md) waits), and refactor towards the
extension SDK as we go: Okur's words ready for translation, and verbosity settings, first.

## Rules for this round

- New user-facing words go into the message catalog once the groundwork is merged (step 2);
  never as literals.
- Dialogs: record JAWS first (old-pc), then build Okur's, then compare Okur's speech live
  (step 1). No speech is written for a dialog: Okur reads its own window as any other.
- Before committing any trace or evidence: search it for URLs and anything about the owner
  (`grep -lE "https?://"`), as the verification loop now says.
- Remove a worktree once its branch is merged (the user's request).

## Steps

### 1. Measure Okur live on old-pc

Replay cannot measure Okur's own dialogs: the evidence of 222-226 holds JAWS's dialog, which
our tracer sees thinly (JAWS runs elevated). Build a live comparison:
- `run-scenario.ps1 -Reader okur`: stop JAWS (elevated; old-pc is the lab), start `okur run`
  with the reference machine's settings and `--speech-log`, send the scenario's keys as for
  JAWS, then restart JAWS. Its output is Okur's run, kept apart from `corpus/evidence/` (never
  evidence).
- `okur live-parity <run> <evidence>`: utterances cut by each step's times, compared with the
  JAWS transcripts as `okur parity` compares them.
- Done when: 222 (Links List) runs live on old-pc and the comparison lists each step.

Done on 2026-10-18 (`4d144e3`, `00cc733`): the first run measured Okur's Links List at 5/11 steps;
fixed to 10/11 (radio groups by `WS_GROUP`, a link's current state in the list, the hint after
Activate Link). JAWS names its own window first ("JAWS Version 2025.2505.43"): the owner
judged it a side effect, now an accepted divergence, and 222 and 224 match 11/11 live. Run
every new dialog this way; `run-scenario.ps1 -OkurDebugLog` keeps Okur's debug log of a run.

### 2. The rest of M0 (the catalog and verbosity are merged)

Done on 2026-10-18: the catalog, the literal and unused-message tests, the pseudo-locale run,
part kinds and the verbosity model ([`messages-and-verbosity.md`](../references/messages-and-verbosity.md)).
Left: role and state marks still written by hand in `SpeechFormatter` and `linearize.rs` (needs
the structured page line and the spacing decision), voice contexts, `.short` message forms, and
okur-ui's viewer window title and "Ready" (a status text in `ToUi::ViewerShow`, then a live
check). Check on old-pc how JAWS's `ACCESS_KEY=a|b|c` fields map (one per verbosity level, or
speech and braille) by changing the setting in its dialog and reading `DEFAULT.JCF`.

### 3. The list family

From [`2026-10-17-helper-dialogs.md`](2026-10-17-helper-dialogs.md) steps 2-3, with the evidence
of 222-226: the helper session in the engine (the page kept while the dialog is in front, the
result applied on close); Heading List (levels, Alt+1 to Alt+6), Select a Form Field (Enter
turns forms mode on), Insert+F3 (Virtual HTML Features), the regions list with regions, the
PlaceMarker list. Each compared live (step 1).

### 3b. Forms in browse mode, studied again (the user's request, 2026-10-18)

The user suspects Okur diverges from JAWS in how forms and form fields are announced when
moving with Up and Down Arrow in browse mode. Record JAWS on old-pc: the form fixtures
(`forms-interaction.html`, `signup-form.html`, `web-app-patterns.html`) arrowing through every
field, group and form boundary; and real pages with forms that need no sign-in (GitHub's new
issue form needs one: check whether old-pc's Chrome has a session; never sign in or submit).
Real pages need their IA2 snapshot captured at the same time, and a privacy search before
anything is committed. Then compare Okur with `okur parity` and fix what differs.

### 4. Parity left from round 3

- 218/s10: Alt+Shift+F into a page menu ("Main menu menu New menu ...").
- 219 and 220 (in pending until their clean re-record is imported): the chat patterns;
  Chrome's own controls (Escape from Chrome's menu, Ctrl+T, Ctrl+W, Down in the address bar).
- 314: the Notepad window view's menu bar and status bar lines (traces need the window's
  text), and JAWS's "ESCAPE" on the first Down in its font message (s3).
- 315/s16, 316, 312, and the older ones `okur parity` lists (01, 104, 111, 12, 204, 207, 209,
  305, 313).

### 5. With the user

- Live check of the partial page reads on WhatsApp (dev box, with permission): `okur latency`
  before and after a page change.

### 6. Then: punctuation and JAWS's reading settings (the owner's decision, 2026-10-18)

Once the list dialogs are done, the next round is document reading and review: the settings
users change most to fit JAWS to their tasks and habits. For each feature, in this order (the
owner: "do not be satisfied with my description, find them in user manual / tutorials for
jaws, test it in old-pc, and only then can you implement it"):
1. Find it in JAWS's documentation: help, keystroke lists, Freedom Scientific training.
2. Read the setting on old-pc (Settings Center, Quick Settings, `DEFAULT.JCF`, the voice
   profile `.VPF`), change it to each value, and record a scenario per value. Put the old value
   back after recording.
3. Implement it as a configurable Okur setting, with its JAWS name, and compare with
   `okur parity` and a live run.

Features to start with:
- Punctuation levels (None, Some, Most, All), symbol names in running text, and
  per-context punctuation. The parity gap audit puts 59 spoken steps here.
- Typing echo (characters, words, both, none) and key repeat.
- Character reading: Say Character (numpad 5) pressed once, twice (phonetic spelling, "alpha",
  "romeo"), and how 4 and 6 read after it. The owner uses this by habit; record what JAWS
  really does before building anything.
- Word splitting (mixed case, numbers), capitals (pitch, "cap"), and the other reading and
  review options the documentation lists.

Also from this round:
- Accepted divergences are written down and applied by every parity check
  ([`accepted-divergences.md`](../references/accepted-divergences.md)). Two entries are proposals
  waiting for the owner's review: 312's emoji and 313's Say Font.
- Re-record 205: its first trial was recorded with File Explorer in front, and it put old-pc's
  file names into the public corpus. Replace the evidence file; history is not rewritten.

After that, the rest of [`2026-10-17-helper-dialogs.md`](2026-10-17-helper-dialogs.md) (JAWS
Find, the reading highlight, routing, PlaceMarkers, Settings), and the web-application items
left in [`2026-10-17-web-apps-round-3.md`](2026-10-17-web-apps-round-3.md) step 2.
