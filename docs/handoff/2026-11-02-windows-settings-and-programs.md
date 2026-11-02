# Handoff: Windows' own UI, round 1 — Settings and Programs and Features (2026-11-02)

Follows [`2026-11-02-store-submission-reading-highlight-demo.md`](2026-11-02-store-submission-reading-highlight-demo.md).
Branch `windows-ui-round-1`: `4b8b197d` (corpus), `4210edfd` (engine), `1d9f44f0` (a move of
one rule), and this handoff.

## What the owner asked

"work more on windows' own UI. I tried okur on "add or remove programs" screen, and it was not
performing very well there, I had difficulty in navigating on that screen. we can start from
there, but continue with the rest." And a review of the crate structure by a read-only
subagent, for a refactoring before any cross-platform work.

## What was wrong

Okur said nothing at all in Windows 10's Settings (old-pc, `side-by-side.ps1`, first run):
JAWS read every control, Okur's speech log held only "stop" entries. Settings is a Store app:
its controls are a core window of `SystemSettings.exe` inside a frame of
`ApplicationFrameHost.exe`, and the frame is the window in front. Since 2026-10-18 the tracer
took a focus from any process but the front window's for an application behind (a fix for
Settings speaking from behind Chrome, 221), so every focus in every Store app was dropped. Now
a window is the front application's when its process is the front window's, the Store app's in
that frame, or when the window lies inside the one in front (`in_front_application`).

On Windows 11 the owner's "Add or remove programs" opens Settings' Installed apps, the same
kind of window: the same cause is likely what they met. Not checked on Windows 11: the dev box
needs the owner's permission.

## Built

| What | Scenario |
|---|---|
| A Store app's focus is read (the cause above) | 3155 |
| An app said once: a XAML item is named after its texts, which are not its columns | 3155/s7 |
| "collapsed"/"expanded" on entering a list and on Insert+Tab, not on the arrows | 3155/s6, s7, s11 |
| Space on an app: " selected expanded" | 3155/s10 |
| An opened app's buttons are not its columns | 3155/s11, s15 |
| Column titles are header bars: "Name Header bar, 1 of 5, Select column title to sort by that category." | 3156/s2 |
| A Win32 toolbar by its role alone; its own focus before its button's not said | 3156/s3, s4 |
| A Win32 list view outside a dialog box named by its window's text ("FolderView") | 3156/s10 |
| Columns without the direction marks around dates | 3156/s11 |
| File Explorer's search box says its prompt (from the platform, as the field's description) | 3156/s5 |
| Insert+Tab on a list item: its state, place and hints, with a list view's name | 3156/s14, 3155/s11 |
| The desktop module no longer takes Programs and Features' "FolderView" for the desktop | 3156 |

Harness:
- `run-scenario.ps1` launch options `app_frame` (a Store app's window: closed through its
  frame, ApplicationFrameHost never killed), `focus_selected: false` (a Control Panel page),
  `settle_ms` (Settings fills its apps list for seconds after it shows).
- The tracer's privacy rules: a list's rows drawn (neither focused nor selected) are not
  recorded, and UIA selections are taken only from the processes in front. The first
  recordings of 3155 held old-pc's installed programs and the taskbar news feed's MSN address;
  they were discarded and recorded again. Live, the news feed's rotating headline is no longer
  read while Settings is in front.
- `skills/jaws-reference/scripts/side-by-side.ps1`: Okur silent beside JAWS on old-pc, each
  key's speech from both (research).

## Figures

`okur-lab parity --only 3155` at `4210edfd`:
```
JAWS spoken parity: 16/20 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 7/20 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
```
`okur-lab parity --only 3156` at `1d9f44f0`:
```
JAWS spoken parity: 15/17 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 15/17 steps match exactly; 0 accepted divergences not counted (0 scenarios could not run)
```
`okur-lab parity` (all) at `4210edfd`:
```
JAWS spoken parity: 11156/12058 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 11405/11992 steps match exactly; 365 accepted divergences not counted (0 scenarios could not run)
```
Against the previous `corpus/parity-baseline.json`, step by step: 0 steps that matched
mismatch now. The commit gate (`cargo fmt --check`, clippy, `cargo test --workspace`, the
reference separation) passed for each commit.

## Open

- **Windows 11's Installed apps**, the owner's own screen: check live on the dev box, with the
  owner's permission.
- 3156/s1, s14's list: a list view with the focus itself and a focused item not selected; JAWS
  says the item ("Not Selected 7-Zip ..."). The tracer must read the list view's focused item
  (MSAA `accFocus`), then record again.
- 3156/s6: Tab from the search box passes through the list for about 30 ms before the link.
  JAWS says the link alone; Okur says the list, its item and the link twice. Focus events that
  follow one another that closely need coalescing, live and in replay alike.
- 3155/s20: Insert+T says "Apps", the category above Settings' navigation list, not the
  window's title.
- 3155/s13, s14: JAWS's "Uninstall Button, 2 of 2" and "Advanced options Link, 1 of 1" inside an
  opened app; UI Automation reports no position there.
- 3155's history layer (7/20): JAWS's history keeps the direction marks in a XAML item's name;
  Okur removes them from every name (after [obs, 323], a property sheet's, which had none).
  Inaudible; which names keep them is not known yet.
- The Print dialog's list view is "Folder View" (its label) and Programs and Features' is
  "FolderView" (its window's text): the rule taken is "outside a dialog box, the window's
  text"; one more list view outside a dialog would confirm it.

## The crate structure

The subagent's review is [`../references/crate-structure-proposal.md`](../references/crate-structure-proposal.md)
(read-only for the code; every figure names its command). In short: `okur-core` holds 13
application modules and calls Word and Excel directly in 84 places; `okur-platform` mixes the
live Windows driver with pure converters that parity needs, platform-neutral runtime code,
lab-only tools and installer plumbing, and it defines the OS traits a macOS crate would
implement. Proposed: `okur-apps`, `okur-runtime`, `okur-windows-model` (pure converters, so
parity runs on macOS and Linux CI), `okur-platform` renamed `okur-windows`, lab code into
`okur-lab`, and a dependency-layer check in the gate and CI. Seven steps, each proved by an
unchanged `parity-snapshot.sh` output. Its §12 has six questions for the owner. One finding
was acted on at once: this round's search box rule had named a Win32 class in the engine; it
moved to the platform (`1d9f44f0`).
