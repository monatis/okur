# Handoff: everyday-use gaps, round 21 (2026-10-22)

Follows [`2026-10-22-everyday-gaps-round-20.md`](2026-10-22-everyday-gaps-round-20.md) and covers
the commits after it (`79b429a` to `a09021c`, and this handoff's), under the plan
[`docs/plans/2026-10-22-everyday-gaps-round-20.md`](../plans/2026-10-22-everyday-gaps-round-20.md).
The next plan is
[`docs/plans/2026-10-22-everyday-gaps-round-21.md`](../plans/2026-10-22-everyday-gaps-round-21.md).

## The owner's answers and requests

- A network problem interrupted the session once, and the owner restarted the dev box for
  another task, then asked to resume. Nothing was lost; old-pc was idle after round 228.

## Decided without the owner (reversible)

- **The Word List's Summary shows what JAWS's help describes**, the sentences holding the word,
  each a link, in a viewer "Word List Summary"; JAWS's own opened nothing (808), an accepted
  divergence.
- **One-character superscripts and subscripts count**: JAWS's skim reading passes over them
  (E = mc², H₂O, x sub i) and reads 2³ as "23"; Okur finds them and marks a one-digit number
  after a number ("2 superscript 3"). Accepted divergence, six steps.
- **An application's notification that only echoes what was typed** is neither said nor kept
  in the history (the echo filters Okur had for live announcements, now on notifications).

## What was done

### Features (each checked with `okur-lab parity` against old-pc's evidence)

- **Select Sound...** opens Windows' Open dialog in Okur's window process (`79b429a`, 802).
- **Skim reading's color rules** (`7415b8a`, 805): Text Color and Background Color by JAWS's
  color names, alone or with attributes.
- **Skim reading by Strikeout, Superscript and Subscript** (`0bddf8f`, 809 to 813): IA2's
  `text-line-through-style` and `text-position`. After an attribute rule's lines and count,
  the word said is the one the last line's found text starts in ("on", "1st"); a rule that finds
  nothing says nothing.
- **Deleted, inserted and highlighted text between their marks** (`0bddf8f`, `015e5c6`, 814,
  819): "Deletion … end Deletion", "Insertion … end Insertion", "mark … end mark"; a paragraph
  marked whole says its end at the next line's start.
- **Raised and lowered numbers after a number** (`1d4d0aa`, 817, 818): "10 superscript 23 end
  superscript", "5 subscript 10 end subscript"; raised letters (21st) and numbers after
  letters (x10, H2SO4) as they are.
- **Words with their marks** (`015e5c6`, 819): Ctrl+Right, Ctrl+Left and Say Word say the last
  mark before the word, then the word with the marks inside it ("Insertion ten", "10
  superscript 23").
- **The Word List's Summary** (`f93453c`).
- **Applications' own notifications in the Notification History** (`8d8f9ab`, 820): a UI
  Automation notification is kept for Shift+N and in the history under its program's name
  ("powershell"), the rules decide for it, and Ctrl+N's toast switch leaves it said.
- **Notification History**: Manage Rules... works after the first rule (`c612260`); a message
  box names the WPF window it opened over (`ced40f6`, 787/s17).
- **Mouse echo** says nothing where a text control has no word under the mouse (`0b717f8`).

### Harness

- The IA2 snapshot keeps `text-position` and `text-line-through-type` (`0bddf8f`); the
  attributes page's snapshots were taken again with it (snap239).
- The buffer invariant on merged words lets a word run through an element of inline text only
  ("mc2", "1st").
- The tracer records the program a UI Automation notification came from (`8d8f9ab`).

### Accepted divergences

- JAWS's Word List Summary opens nothing (808/s3 to s8).
- JAWS passes over raised and lowered text of one character (811/s15, 812/s16, 815/s15,
  816/s16, 818/s2, 818/s7).
- JAWS's speech before its own dialogs: 809/s1 to 813/s1, 815/s1, 816/s1.

## Figures

Each is the named command's output at `a09021c`:

- `okur-lab parity`: "JAWS parity (Ia2): 6540/6776 steps match exactly; 181 accepted
  divergences not counted (0 scenarios could not run)". At the previous handoff: 6347/6587.
- `okur-lab parity`, spoken layer: "JAWS spoken parity: 6403/6738 steps match exactly".
- `okur-lab invariants`: "Buffer invariants: 0 violation(s)".
- `corpus/CORPUS_MANIFEST.json` statistics: 459 scenarios, 1383 trials, 1377 successful.
- `cargo test --workspace`: 1357 passed, 0 failed, 199 ignored.

## Learned

- **Chromium's IA2 for text attributes**: `<s>` and a double strikeout are both
  `text-line-through-style:solid` (no line-through type); `<sup>` and `<sub>` are
  `text-position:super` and `sub`; `<s>` and `<del>` have IA2 role 0x42F (content deletion),
  `<ins>` 0x430, `<mark>` 0x432, `<sup>` and `<sub>` 0x429 (text frame).
- **JAWS's skim reading by attributes**: Strikeout finds a double strikeout too; Double
  Strikeout and Shadow find nothing in Chrome; raised and lowered text of one character is
  passed over; with nothing found, nothing at all is said.
- **JAWS's marks in a line**: Deletion, Insertion and mark around their text; the end mark of
  text that ends a paragraph opens the next line; "superscript" and "subscript" only around a
  number of two or more characters right after a digit. By word, only the last mark before the
  word is said.
- **JAWS and an application's UI Automation notification** (820): it speaks it, lists it as
  "Upload finished: report.pdf powershell 4:53 PM" (the process's name, lower case), and
  Insert+Space, Shift+N repeats it; its details dialog reads as a toast's.
- **A UIA snapshot capture can fail** when Chrome is not in front ("Foreground window '' does not
  show fixture title", round 241); the next round captured it again.
- **The commit gate judges the tree before the command runs**: a new snapshot needs
  `invariants --write-baseline` and a new fixture `corpus-manifest --write`, each in a command of
  its own, before the commit.

## Recorded meanwhile, and queued

Rounds 229 to 246 are imported. 820 was recorded three times (rounds 247 to 249; 248 and 249
with every trial successful) and is **not imported**: no trace holds the notification JAWS
spoke, so its replay could not show Okur's reading of it. An experiment queued after round 250
traces the fixture with a debug log (`sr-harness\exp\uianotify`). Round 250 (821, the edits page
by character) is recording.

## Not done, and why

- **820's traces** (above): the first item of the next plan.
- **787/s16 and s18** in Okur's mirrored dialogs, and judging "Contains text" and "_Rules".
- **The clipboard keys** wait for the development certificate on old-pc; **Office** for its
  activation; **toasts Focus Assist hides** need research that leaves old-pc's notification
  settings alone.
