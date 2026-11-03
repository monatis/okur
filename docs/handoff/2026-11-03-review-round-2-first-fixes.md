# Handoff: File Explorer's ribbon, the public Store submission, the second review's first fixes (2026-11-03)

Follows [`2026-11-03-windows-ui-round-2-recordings.md`](2026-11-03-windows-ui-round-2-recordings.md).
Plan: [`../plans/2026-11-03-review-round-2-before-cross-platform.md`](../plans/2026-11-03-review-round-2-before-cross-platform.md).
Branch `windows-ui-round-2`; `main` was fast-forwarded to `ceb00ead` and pushed by the owner.

## What the owner did

- Submitted Okur 1.1.0.0 to the Store for certification as **public** (submission 2), after the
  agent filled its draft on old-pc's Chrome. The owner says when it is live; then the README
  and experiment E6 (the Store updates old-pc's 1.0.0.0 by itself).
- Asked for a second review of performance and structure before cross-platform work, then
  JAWS's features from Settings Center and the help.

## Built

| What | Commit | Proof |
|---|---|---|
| File Explorer's ribbon (the Windows Ribbon framework's): a split button with a drop-down grid, no tool bar or place outside a ribbon menu, the minimized ribbon's page passed over; Insert+Tab on a file; no direction marks given to the voice | `a1962fd3` | 3161 recorded (round-ui2j) and imported `221f3dca`; its tab places a JAWS accident like WordPad's, accepted `9e9af731`; 3161/s8 open (JAWS gives the New folder button no hint) |
| Submission 2's package and texts: 1.1.0.0 from `9e9af731`, Public, "What's new", the listing's line on Windows | `137bbaf3`, `ceb00ead` | Partner Center validated the package; Defender on the dev box: no threats; [kit](../references/store-submission-kit.md) |
| The second review: two read-only reviewers, findings checked by hand | `b10ac35f` | [`../backlog/performance-review-2026-11-03.md`](../backlog/performance-review-2026-11-03.md); the structure's revisions in [`crate-structure-proposal.md`](../references/crate-structure-proposal.md) §13 |
| Structure quick wins: `okur` built off Windows only by accident (an ungated `set_sounds`); `TtsEngineType` gone for the core's `Synthesizer`; `EXE_SUFFIX`; two doc comments; CI tests `okur-ui-model` and reports whether the workspace builds off Windows; ratchets that count calls through imported module names (85, not 82) and literals outside the app modules (5) | `153a5c4d` | the gate's structure check |
| The macOS stubs that logged "Initializing macOS AXObserver" and did nothing, deleted | `f776720a` | |
| Live, a window come to the front recorded as its handle, class and title, without the focus read the converter threw away | `8ad52a3e` | live after-run below |
| Word's Say All withdrawn by a key while its paragraphs are on their way | `8ad52a3e` | `say_all_in_word_reads_by_its_paragraphs` fails without it |
| Whole-document and whole-sheet reads on the bulk lane; Word's paragraphs from the caret's on | `8ad52a3e`, `5f3d00c0` | live after-run (1038) |
| The reading highlight: an object in view framed at once, no fixed 80 ms sleep | `8ad52a3e` | live after-run (106, 201) |
| Cached walks: a list view's focused item, a Store app's window texts, a list's header; the focus's window and rectangle with its properties | `8ad52a3e`, `3a26d63d` | live after-run (3155, 3158) |
| Skim reading's text rule and notification rules compile their patterns once; Settings Center's search finds messages by name | `8ad52a3e`, `59e7d068` | their tests |

## Figures

At `3a26d63d`, `okur-lab parity` (unchanged by every fix of the review):

```
JAWS spoken parity: 11221/12114 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 11463/12048 steps match exactly; 370 accepted divergences not counted (0 scenarios could not run)
```

`okur-lab invariants`: `Buffer invariants: 0 violation(s)`. `cargo test --workspace
--no-fail-fast` (4 threads), summed: 1622 passed, 0 failed, 201 ignored.

The live baseline before the fixes is in the review document. The after-run: the same scenarios
and 1038, one trial each, release build of `5f3d00c0` (without `3a26d63d`), `okur-lab latency`
over each debug log, the focus reads (count, p50, p90, p99, max in ms), before then after:

```
106   before  4  34.1  60.6    60.6    60.6    after  4  27.9  48.5    48.5    48.5
201   before  1  61.4  61.4    61.4    61.4    after  1  60.6  60.6    60.6    60.6
3155  before 18  50.7  64.5  1568.2  1568.2    after 18  49.9  56.5  1557.2  1557.2
3158  before 14  38.1  60.2   116.3   116.3    after 14  40.9  53.1   109.5   109.5
3161  before 13  96.5 129.1   206.2   206.2    after 13  96.2 130.6   211.6   211.6
510   before  3  37.5  62.7    62.7    62.7    after  3  46.1  79.7    79.7    79.7
3159  before 10  35.8  92.6    95.9    95.9    after (3 trials) 31  43.3  94.1  97.3  97.3
```

No "event read: foreground" line is left after: a window come to the front is no longer read
(before: 1 to 8 per scenario, up to 1544 ms in 1115). One trial of 3159 after gave 118 ms p50;
three trials gave 43.3, as before, so that trial was noise. The single trials say no more than a
direction: nothing got slower beyond the noise.

**Not measured: Word and Excel.** Office on old-pc is not activated, and its Activation Wizard
now opens a few seconds after every start of Word or Excel. It blocked the fixtures from closing,
put the focus on its link, and spoiled 1113, 1115 and 1038 both before and after (1038 again on
its own: the wizard opened at its first keys). Word's Say All from the caret (`5f3d00c0`) is
therefore not checked live yet. Recording Word or Excel scenarios on old-pc waits for the owner:
activating Office or stopping the wizard is theirs to do (verification-loop.md §8).

## Next

1. The review's open items, in its order: Excel's readers by arrays (P4), Word's per-key reads
   (P8), the list view's item in O(1) (P3), Say All in pieces (P1), Office's slow NetUI MSAA read
   (P15).
2. The structure (proposal §13.4): step 5a (Word's and Excel's renderers into the core's
   document and spreadsheet modules), then step 4 as 4.1 to 4.7 with its live checks, then the
   key model and `FocusEvent`.
3. Then JAWS's features from Settings Center and the help (plan §3): round 1135 (Excel's monitor
   cells) imported and built first; the customization map refreshed against the 44 settings Okur
   declares today.
4. When the owner says 1.1.0.0 is live: the README, and E6 on old-pc.
5. Waiting for the owner: Office on old-pc activated, or its Activation Wizard stopped, before
   any Word or Excel recording or live check; Document Recovery's files in Word and Excel,
   kept, are the owner's to remove.
