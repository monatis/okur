# Handoff: everyday-use gaps, round 11 (2026-10-21)

Follows [`2026-10-21-everyday-gaps-round-10.md`](2026-10-21-everyday-gaps-round-10.md) and covers
the commits after it (`10ff742` to `9c47259`, and this handoff's), under the plan
[`docs/plans/2026-10-21-everyday-gaps-round-10.md`](../plans/2026-10-21-everyday-gaps-round-10.md)
(the README's "Not yet" list in its order, the owner's decision). The next plan is
[`docs/plans/2026-10-21-everyday-gaps-round-11.md`](../plans/2026-10-21-everyday-gaps-round-11.md).

## The owner's answers and requests

None new in this stretch.

## Decided without the owner (reversible)

- **A visited link is said visited on its line, not as it takes the focus** (`73e9ef3`):
  689/s5, s6 against s3. On the first Tab into 689's page JAWS also says the link's focus
  with "visited" in every trial (689/s1); why is not known, and the step still differs.
- **The Okur cursor in a text field moves over the field's lines** (`71acc64`). Okur has no
  model of the screen's text; JAWS's cursor in Notepad reads lines at the mouse and says
  nothing for characters, the word, End and Home (690). The mouse is not moved by line yet.
- **Okur Settings has a "Settings for" box** (`ac683ef`): Default (All Applications), the
  application in front (chosen when it opens, as JAWS's Settings Center does), and the site of
  a page. Each place shows its own settings; Apply keeps only the changes, in its layer.
- **The common Font dialog's sample is known by its control ids** (`3a0d642`): stc5 and stc6
  of the Windows SDK's dlgs.h, in a dialog with its cmb1. A probe (probe129) showed the sample
  is neither owner-drawn nor hidden.
- **Accepted as JAWS's accidents:** "irections" in Say All by sentence (677/s1, `fc87b50`);
  Say All by sentence or paragraph in a text box starting elsewhere (684, 685); the Page Setup
  preview's pictures named after a label (544, 674, 675 s13).
- **693 and 694 are dropped** (`9c47259`). Following a link in 689 made the news article
  visited on every fixture page (all `file://` pages are one site to Chrome), so 682's page
  with no visited link cannot be had again in the lab's browser.

## What was done

### Features (each checked with `okur parity` against old-pc's evidence)

- **Say All by sentence**: a link is a sentence of its own, and a mark after a line's text
  stays with its last sentence (`6a00167`); 677/s1 then matches with the divergence above.
- **The temporary PlaceMarker** (`08ac2f0`): Ctrl+Windows+K, then K and Shift+K. 687 matches
  in all 14 steps. A focused list-view item that is not selected is "Not Selected" (687/s11).
- **U and V on their keys** (`08ac2f0`): the key map still bound U to any link and V to
  nothing; a test now presses the keys.
- **A visited link's line** (`73e9ef3`): "• ‹visited› ‹ Link › City Library Expands Evening
  Hours". 689 matches in 13 of 14 steps.
- **The Okur cursor in a text field** (`71acc64`): 690 matches in all 14 steps.
- **Say Window**: the Font dialog's sample skipped (`3a0d642`); 665/s3, 672/s3 and 673/s3 now
  differ only in the Size list's name, which JAWS says and the other lists' it does not. The
  probe found nothing in their windows to tell them apart.
- **Okur Settings for one application or one site** (`ac683ef`), from JAWS's documentation.

### Harness

- **Chrome starts without its "Restore pages?" bubble** (`9c47259`), with
  `--hide-crash-restore-bubble`. In round 132 the bubble took every scenario's first trial
  away from the page.
- **Keys in JAWS's own dialogs must be typed by JAWS** (`TypeKey`): round 128's 688 sent plain
  keys and typed text to the elevated Run JAWS Manager list, which refused them, and every
  later key went nowhere. 688 now types through JAWS and walks the list by its arrows.
- **Probes**: probe129 lists a dialog's windows, styles and visibility, without a key. Its
  first run (probe126) opened no dialog: Start-Process joined its quoted command unquoted.

### Imported

Each import had all trials passed, the committed capture script, and was grepped for links
and personal data.

| Commit | Scenarios |
|---|---|
| `5ed63fe` | 684-687 (round 125) |
| `cbe99c9` | 662, 676-682 (round 127), with the colored-fields and visited-links snapshots |
| `dc18ce1` | 689, 690 (round 130), with the followed-links snapshots |

Not imported:
- round 132's 691-694 (the restore bubble; 693 and 694 dropped);
- round 133's 688 (the custom label): its fixture's snapshot is taken in round 135.

## Figures

Each is the named command's output at `9c47259`:

- `okur parity`: "JAWS parity (Ia2): 4942/5132 steps match exactly; 110 accepted divergences
  not counted (0 scenarios could not run)". At the previous handoff: 4844/5017 with 106.
- `okur parity`, spoken layer: "JAWS spoken parity: 4817/5094 steps match exactly".
- `okur invariants`: "Buffer invariants: 0 violation(s)".
- `corpus/CORPUS_MANIFEST.json` statistics: 353 scenarios, 1113 trials, 1113 successful.
- `cargo test --workspace`: 1261 passed, 0 failed, 199 ignored (at `ac683ef`; later commits
  change the harness and the pending scenarios only).

## Learned

- **JAWS's custom labels** (688, round 133): Insert+Ctrl+Tab opens "Custom Label" with the
  text "Enter a custom label for this item" and an edit of that name; Enter closes it
  silently; the field then reads "lamp ‹ Edit ›" and the form fields list "lamp Edit". Right
  after, Insert+Up says the page's title: JAWS refreshed the page and its cursor went to the
  top. Custom Label Delete All asks first: "Delete all Custom Labels? | This will remove all
  custom labels for the current document or web page | OK".
- **Run JAWS Manager's list** (Insert+F2) begins Commands Search, Custom Highlight Assign,
  Custom Label, Custom Label Delete, Custom Label Delete All, of 21.
- **JAWS's cursor** reads Notepad's lines at the mouse (690), unlike the WinForms dialog
  of 516.
- **A visited link**: "visited" before the role on its line; V wraps to the only visited link
  with "wrapping to top", unlike K with the only PlaceMarker.
- **Line with Pauses** only changes how JAWS hands lines to the synthesizer.

## Recorded meanwhile, and queued

On old-pc, in this order:
1. round 134 (running): 695-698, a followed link and the JAWS cursor at Intermediate and
   Advanced (on the harness before the bubble fix);
2. round 135 (waiter135, push `cc154e34`): 691 and 692 again, then the custom-label snapshots.

## Not done, and why

- **Custom labels in Okur**: the next plan's first item, now that 688 shows JAWS's dialog.
- **The PlaceMarker List and saved PlaceMarkers; Flexible Web**: a hand-run exploration of
  their dialogs on old-pc first, so that no recording leaves a PlaceMarker or a rule behind.
- **Line without Pauses** (several lines in one utterance): a speech-pipeline change.
- **The Size list's name in Say Window**: no rule found.
