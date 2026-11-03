# Handoff: the crate structure's steps 0 to 3, and Windows' own UI round 2 begun (2026-11-03)

Follows [`2026-11-02-windows-settings-and-programs.md`](2026-11-02-windows-settings-and-programs.md).
Branch `windows-ui-round-2`, fast-forwarded into `main` (not pushed: the owner pushes):
`8915de56`, `9d8c2aa2`, `484aec21`, `99fee058`, `b4c5dcf7`, `60b04085`, `e835c366`, `5d26f8cc`
and this handoff.

## What the owner said

"agree on the plan and defaults answers to the questions. go for it" (2026-11-03): the plan
[`2026-11-02-windows-ui-round-2.md`](../plans/2026-11-02-windows-ui-round-2.md), and the default
answer to each of the six questions of
[`crate-structure-proposal.md`](../references/crate-structure-proposal.md) §12 (recorded there):
the names `okur-apps`, `okur-runtime`, `okur-windows`, `okur-windows-model`; Word's and Excel's
queries as document and spreadsheet capabilities; steps 1 to 3 now, 4 to 6 after the Store
install round and before macOS, step 5 as the SDK's M2; macOS's accessibility layer decided with
its plan; the size ratchet reports until step 5 ends; a metadata script, not `cargo-deny`.

## old-pc went offline

old-pc answered until about 01:45 (2026-11-03, its time), then OpenDesk says `target_offline`
on every try (two hours of tries, the last at the end of this stretch). The deploy of this
round's build failed half-way: old-pc's `okur.exe` and fixtures are from before it. Nothing was
recorded this round. When old-pc answers again: `lab-deploy.sh --okur target/release/okur.exe`,
push `corpus/pending/3157-*.json` and `3158-*.json` to its `corpus/scenarios`, and record 3157,
3158 and 3156 again (the plan's next steps). The two scenarios are in `corpus/pending/` and their
fixtures (`desktop-settings-home.ps1`, `desktop-settings-display.ps1`) in `corpus/fixtures/`, in
the manifest (`okur-lab corpus-manifest --write`).

## The crate structure: steps 0 to 3 built

Each step proved, before and after, by `bash skills/jaws-reference/scripts/parity-snapshot.sh`
(`diff -r` printed nothing every time) and by `cargo test --workspace -- --list` (the same
tests by name):

| Step | Commit | What |
|---|---|---|
| 0 | `9d8c2aa2` | `.agents/crate-layers.json` (each crate's kind and allowed dependencies) and `check-crate-layers.ps1` on `cargo metadata` (rules R0-R3); `check-structure.ps1` with `.agents/structure-baseline.json` (R4-R6); both run by `check-sources.ps1`, so by the gate and CI's Windows job |
| 1 | `484aec21` | the harness's own tools out of the reader's crate into `okur-lab`'s new library: `harness`, `uia_probe`, `audio`, `probes` (the lab-only reads of `actions.rs`), `trace` (the trace file writer; the platform keeps `run_recording`) |
| 2 | `99fee058` | `okur-platform` renamed `okur-windows`; its re-exports of `okur-core`'s types gone; the debug log's filter follows (`okur_windows=debug`) |
| 3 | `b4c5dcf7`, `e835c366` | `okur-windows-model`: the trace format and its conversion (`event_trace`, in parts: `focus`, `converter`, `office`, `controls`, `tests`), the UIA and IA2 snapshot formats and their AOM mapping, `privacy`, `node_ids`, and the class and text helpers the conversion uses. `okur-windows` keeps the live reads: `event_capture`, `uia_capture`, `ia2_capture`. `okur-corpus` no longer depends on the driver, and CI's portable job tests `okur-windows-model` and `okur-corpus` on macOS and Linux |

The checks now (`check-crate-layers.ps1 -Report`, `check-structure.ps1 -Report`, at the end of
this stretch):

```
[report] known edge okur-lab -> okur, until step 4: logging and the AOM loader in okur-runtime
[report] neutral_platform_cfg 0 (baseline 0)
[report] platform_literals_in_core 4 (baseline 4)
[report] engine_app_references 82 (baseline 82)
[report] uia_outside_xa11y 250 (baseline 250)
[report] engine_fields 200 (baseline 200)
[report] platform_cfg_sites 191 (baseline 191)
```

The `uia_outside_xa11y` ratchet stopped its first commit this round: a new UI Automation walk
in `okur-windows` added three lines naming `IUIAutomation`. The walk went into
`xa11y-windows` (`uia_element::nth_list_item`) instead, as VENDORED.md's phase 4 asks.

Steps 4 to 6 (`okur-runtime`, the engine no longer naming Word and Excel, `okur-apps`) wait for
the Store install round, as the owner answered.

## Windows' own UI, round 2

Found side by side on old-pc (`side-by-side.ps1`, research) before it went offline:

| Page | JAWS and Okur |
|---|---|
| Settings' home (13 categories in a grid) | differ: JAWS says a category's second text after its position, "Devices \| 2 of 13 \| Bluetooth, printers, mouse"; Okur said it before |
| Settings > Time & Language > Language | the same on 9 keys of 9 |
| Settings > System > Display | the same on 12 keys of 13: JAWS's history keeps "1920 × 1080" where Okur says "x"; the spoken layer, once recorded, tells what JAWS's voice got |

Built (each with a unit test; none recorded yet):

| What | Commit | Proof so far |
|---|---|---|
| An item's further texts outside a list view are its description, said after its position | `8915de56` | parity unchanged; 3157 records it |
| A focus that moved on before it was read is not said (the tracer's new `focus_elsewhere`, trace format 10) | `60b04085` | parity unchanged (old traces lack the field); 3156/s6 recorded again shows it |
| A list view with the focus itself is read at its focused item | `5d26f8cc` | parity unchanged; 3156/s1 recorded again shows it |

A rule tried for 3156/s6 on window classes alone (the event's against the keyboard focus's)
took 305 recorded steps with it (`parity --all`: 11100/11992 against 11405/11992): Notepad's
window and a Word dialog both come to the front with the focus in a child of another class.
Only the tracer can tell "inside" from "elsewhere", hence the new field.

Figures at the end of this stretch, from `parity-snapshot.sh` (unchanged from the start):

```
JAWS spoken parity: 11156/12058 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 11405/11992 steps match exactly; 365 accepted divergences not counted (0 scenarios could not run)
JAWS parity (Uia): 10199/11886 steps match exactly; 365 accepted divergences not counted (15 scenarios could not run)
```

`cargo test --workspace -- --list`: 1818 tests (1817 at the start, and the moved-on focus test).

## Open

- Record 3157, 3158, and 3156 again (old-pc).
- 3155/s20: Insert+T in Settings says the category, "Title is Apps"; the trace holds no such
  text yet (the tracer would read the page's category).
- 3155/s1: Insert+Tab on a combo box in Settings names its group ("Choose where to get apps"),
  where Okur's Insert+Tab names none; 3158/s8 will show whether JAWS does so in general.
- 3155/s13, s14: the positions JAWS gives an opened app's button and link ("2 of 2", "1 of 1",
  but none on the first Tab): no rule yet.
- Windows 11's Installed apps on the dev box, with the owner's permission.
- The rest of the plan's §2 (Start menu, Action Center, Task Manager, Device Manager, Services,
  File Explorer's ribbon).
