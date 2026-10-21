# Handoff: everyday-use gaps, round 13 (2026-10-21)

Follows [`2026-10-21-everyday-gaps-round-12.md`](2026-10-21-everyday-gaps-round-12.md) and covers
the commits after it (`ab2a506` to `922619e`, and this handoff's), under the plan
[`docs/plans/2026-10-21-everyday-gaps-round-12.md`](../plans/2026-10-21-everyday-gaps-round-12.md)
(the README's "Not yet" list in its order, and Okur's settings made as customizable as JAWS's,
the owner's request). The next plan is
[`docs/plans/2026-10-21-everyday-gaps-round-13.md`](../plans/2026-10-21-everyday-gaps-round-13.md).

## The owner's answers and requests

- **Why Windows Defender flags `okur.exe`.** It quarantined it on old-pc as
  `Bearfoos.A!ml`: a machine-learning verdict on an unsigned program that installs a global
  keyboard hook, reads other windows, injects keys and drives the clipboard. Rounds failed in
  seconds. The owner added Defender exclusions for both `sr-harness` folders on old-pc (the
  JAWS rate was lowered for them first, then put back to 90 in every context).
- **Code signing.** The owner will consider it. Prices were researched:
  - Azure Artifact Signing costs $9.99 a month, but its public trust is not offered to
    individuals or organizations in Turkey.
  - Certum's Open Source Code Signing, in the cloud (SimplySign), is open to individuals and
    needs no physical card. It is the suggested route, and the owner will look at it later.
  - SignPath Foundation is free for open source, after a first release.
- **The harness moves out of `okur.exe`** (`a8f1b0a`, the owner's request): `okur.exe` keeps
  `run`, `quit`, `doctor` and `voices`. Every verification command (`parity`, `evidence`,
  `trace`, `snapshot`, `keys`, `jaws-history`, ...) is now `okur-lab.exe`, a crate of its own
  (`crates/okur-lab`), with `okur-lab-keys`. The screen reader a user installs carries none of
  the lab's key injection.
- **The development certificate on old-pc** (the owner's request): the `.cer` and
  `dev-signing.ps1` are in the shared Dropbox's `sr-harness\signing\`. Trusting a certificate
  is a security setting, so the owner runs the import, elevated; not done by the agent.

## Decided without the owner (reversible)

- **One table declares every setting** (`5bb33f5`), the customization map's §7 steps 1 and
  2. The keyboard layout and the Okur key are kept for all applications only: JAWS documents
  that for its key [S4], and a layout brings its key.
- **711/s7 is an accepted divergence** (`e7dd991`). With typing echo set to both, a word
  corrected with Backspace was not said at its end, though words alone say it (710/s7). The
  stale-divergence check now looks at every layer a step is compared on, since typing echo
  is never written to the speech history.
- **`AutoFormsMode=2` is Semi-Auto, built as "Tab brings forms mode, the arrows do not"**
  (`07416cd`). The arrows out of a field with Semi-Auto are not recorded; Okur leaves them
  to the virtual cursor, as with Manual.
- **517 version 2 is not imported** (round 150). The Speech History viewer stays open after
  `TypeKey("Escape")`, so its S and Down steps say nothing (the trace shows no focus back to
  Chrome). It waits for `okur-lab-keys`, which can type into such windows once it is signed.
- **The page load's opening is left as recorded.** JAWS says "Title document" or the title
  twice; which one depends on timing, as 507's trials show both. 713/s1 differs only there.

## What was done

### Features (each checked with `okur-lab parity` against old-pc's evidence)

- **Speech On Demand** (`3511e53`, `05925da`), from 700, 706 and 707:
  - Insert+Space, S goes from full speech to less speech ("Speech On Demand") and back
    ("Full Speech").
  - Shift+S turns less speech from on demand to muted and back ("Speech off set to speech
    mute", "speech off set to speech on demand"). In full speech it says "Not available in
    Full Speech mode".
  - On demand, only the key map's [Speech On Demand] commands speak (Say Line, Say Window
    Title, Say Color, ...); muted, nothing speaks.
  - Either way, what would be said is still written to the speech history, as JAWS writes it.
  - `SpeechMode` and `LessSpeechMode` are settings, replayed from `DEFAULT.JCF`.
- **PlaceMarkers: MoveTo is silent** (`98970d0`); 704's JAWS side effects are accepted.
- **The settings table** (`5bb33f5`): `crates/okur-core/src/settings/registry.rs`.
  - **What it declares:** each of the settings, by its path in the settings file, with its
    group and label, kind, where it may be kept, its `DEFAULT.JCF` key and decoder, and what
    changes with it.
  - **What is made from it:** the settings dialog's controls, the JAWS import replay uses
    (`import_jaws`), `okur run --typing-echo` and the new `okur run --set <id>=<value>`.
  - **Tests:** one declaration per setting; every offered value can be set and read back;
    old-pc's file is imported.
  - **Typing echo** now offers all four of its values in the dialog and on the command line.
- **Typing echo read from JAWS's files** (`e7dd991`): 0 none, 1 characters, 2 words, 3 both,
  as 709-711 record; 709 and 710 match 7/7 on both layers.
- **Forms Mode** (`07416cd`): Auto, Semi-Auto or Manual, in the dialog and read from
  `AutoFormsMode`. 714 matches 13/13 and 715 33/33.
- **Document and Web Pages Automatically Read When Loaded** (`07416cd`, `read_page_on_load`,
  `[HTML] SayAllOnDocumentLoad`): when it is off, a loaded page's title and summary are said
  and the cursor waits at the top. A reading to the page's end says its last word again
  without its period (712/s1).

### Harness

- `quiet_confirmation` for scenarios that turn less speech on with a key (`ab2a506`).
- The harness split (`a8f1b0a`): the scripts take `-OkurLab`. The provenance records
  `okur_lab_sha256`.

## Figures

Each is the named command's output at `922619e`:

- `okur-lab parity`: "JAWS parity (Ia2): 5288/5500 steps match exactly; 129 accepted
  divergences not counted (0 scenarios could not run)". At the previous handoff: 5106/5302
  with 116.
- `okur-lab parity`, spoken layer: "JAWS spoken parity: 5156/5462 steps match exactly".
- `okur-lab invariants`: "Buffer invariants: 0 violation(s)".
- `corpus/CORPUS_MANIFEST.json` statistics: 377 scenarios, 1185 trials, 1185 successful.
- `cargo test --workspace`: 1293 passed, 0 failed, 199 ignored.

## Learned

- **JAWS's settings keys**, from the shared `Default.JCF` (a settings file, which may be read):
  - `[Options]` has `IndicateCaps`, `SayAllIndicateCaps`, `Indentation`, `LinePauses`,
    `SayBlankLineCount`, `SmartWordReading`, `SkimReadingIndication`, `TouchTypingEcho` and
    `AllowTypingEchoWhenMuted`;
  - `[FormsMode]` has `AutoFormsMode`, `SpeakFormsModeChanges`, `IndicateFormsModeWithSounds`
    and `FormsModeAutoOff`;
  - `[HTML]` has `SayAllOnDocumentLoad`, `IgnoreInlineFrames`, `PageRefreshFilter`,
    `MaxLineLength` and `LinesPerPage`.
- **Typing echo with words** (710) says the word at Space and at Enter, then "Enter". With
  both (711) Space says the word, not "Space". Backspace says the deleted letter in every mode.
- **A page load**: with reading on, JAWS reads to the end of a short page, and its cursor is
  left on the last line (712/s2). With it off, the cursor is at the top (713/s2).
- **Flexible Web's wizard is a Win32 property-sheet wizard** (705's trace): a hidden tab
  control, page dialogs named "Choose an Action", "Choose an element" and "Choose a
  customization", and Back, Next, Finish, Cancel and Help.
  - The first page has five radio buttons, those not yet possible disabled: Create a new
    customization, Retry ..., Undo ..., Save temporary customizations as a rule, View or
    change where rules apply.
  - Choose an element lists the element at the cursor and the ones around it ("heading level
    1 Weekend events", "MainRegion").
  - For a region, Choose a customization offers "Hide MainRegion 1 from the top", "... from
    the bottom" and "Hide all MainRegions".
  - Alt+N on the last page and Escape are silent.
  - The guide (Surf's Up, chapter 16) adds temporary customizations, kept until the page is
    left, then saved as a rule for the page, the site or all sites; Undo and Retry; and a
    sound when a page with rules loads.
- **JAWS's Results Viewer takes no keys from the harness**: SendInput is refused, and
  `TypeKey("Escape")` does not close it either.

## Recorded meanwhile, and queued

On old-pc, in this order:
1. round 153 (`waiter153`): 716, Flexible Web hides the advertisement, the page is read, then
   Undo and the page read again;
2. round 154 (`waiter154`): 717-719, 408 with `IndicateCaps=0`, `SayAllIndicateCaps=1` and
   `LinePauses=1`.

## Not done, and why

- **Flexible Web in Okur**: the wizard needs a property-sheet wizard in `okur-ui`, and what a
  customization does is recorded in round 153. It is the next plan's first item.
- **Where a page-load reading leaves the cursor in replay** (712/s2, s3; also 507, 538 and 539
  after their reloads): replay cannot know how far JAWS's reading got. Replay could report
  progress from JAWS's own transcript; this is a harness item in the next plan.
- **Settings Center, its search and Quick Settings**: the map's §7.6 step 3, after Flexible
  Web.
- **PlaceMarkers' duplicate checks and Display For filters; 517; the clipboard keys (627)**:
  unchanged. The last two need `okur-lab-keys` signed and installed, which waits for the
  owner's certificate import.
