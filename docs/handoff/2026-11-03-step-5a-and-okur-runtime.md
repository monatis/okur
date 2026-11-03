# Handoff: step 5a done, and okur-runtime through step 4.4 (2026-11-03)

Follows [`2026-11-03-review-round-2-first-fixes.md`](2026-11-03-review-round-2-first-fixes.md).
Plan: [`../plans/2026-11-03-review-round-2-before-cross-platform.md`](../plans/2026-11-03-review-round-2-before-cross-platform.md).
Branch `windows-ui-round-2`; `main` was at `3158cfbb` when this stretch began.

## What the owner did

- Said Okur 1.1.0.0 is still in the Store's certification (step 3 of 4), and to go on with the
  plan meanwhile. The README waits until it is live.

## Built

Each row is a refactor: what a user hears does not change. Its proofs are the proposal's
(§11.1): P1, `skills/jaws-reference/scripts/parity-snapshot.sh` before and after with
`diff -r` (identical every time, against the snapshot taken before step 5a); P2, the names
`cargo test --workspace -- --list` gives (the same, plus the new tests named); P5, the
structure counts (`check-structure.ps1 -Report`).

| What | Commit | Proof beyond P1 and P2 |
|---|---|---|
| Word's renderers into the core's `crate::document` (step 5a) | `30725cf6` | engine references to application modules 85 -> 45 |
| Excel's renderers into the core's `crate::spreadsheet` (step 5a) | `0553d7f8` | references 45 -> 24 |
| The engine knows no window class (T7): `FocusEvent`'s `switch`, `cancel_button`, `on_menu_window` and `pages_by_control_tab`, the menus' `launcher` and `PopupKind`, supplied by the converter (`okur-windows-model/src/classes.rs`) for every focus it passes on | `c2b69712` | `platform_literals_in_core` 5 -> 0, now blocking; references 24 -> 23; three new tests (`a_focus_carries_the_facts_its_classes_tell`, `a_menu_says_its_kind_by_its_window`, `a_converted_focus_carries_its_class_facts`) |
| `AppMode`'s `NativeDesktop`, `NativeShell` and `NativeExplorer` become `Shell` and `Application`, each named by its module (`module_path!()`) (step 5a) | `93709ed9` | |
| 4.1: the program chooses the hook and the listener; the program says where the settings and the kept log are (`okur::folders`); a snapshot's AOM loads beside the snapshot format (`okur_windows_model::uia_snapshot::load_aom`, not the core as §13.1 said: the core cannot read a UIA snapshot); one `com::init_mta()` for 16 copies | `05bc9c0c` | `platform_cfg_sites` 186 -> 185; live, below |
| 4.2: the synthesizer is made on the speech thread, which waits for it; a start that panics leaves speech silent; `TtsManager`'s unused speech log removed | `47514a2b` | 185 -> 184; two new tests (`the_synthesizer_is_made_on_the_speech_thread`, `a_synthesizer_whose_start_panicked_leaves_speech_silent`); live, below |
| 4.3: `okur-runtime`, neutral, depending on `okur-core` alone: `KeyboardHook`, `EventSource` with an `EventSink`, `TtsEngine`, `SynthesizerFactory`, `RuntimeError`, with their mocks; Windows' synthesizers a `SynthesizerFactory` (`WindowsSynthesizers`); the portable CI job tests it | `94439938` | 184 -> 182 |
| 4.4: speech (`TtsManager`, `SpeechThread`, the speech log) into `okur-runtime`, making its synthesizers through the factory it is given | `36b9d672` | |
| 4.4: the settings files and the UI host into `okur-runtime`; the UI host starts `okur-ui` through a `UiSpawner` port (§13.1 had put it in 4.3; it came with the code it serves), Windows' starting it lowered below UI Access as before | `13a02546` | 182 -> 178; live, below |

## Figures

At `13a02546`, from `skills/jaws-reference/scripts/parity-snapshot.sh` (identical to the
snapshot taken before step 5a):

```
JAWS spoken parity: 11221/12114 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 11463/12048 steps match exactly; 370 accepted divergences not counted (0 scenarios could not run)
```

`okur-lab invariants`: `Buffer invariants: 0 violation(s)`. `cargo test --workspace
--no-fail-fast` (4 threads), summed: 1627 passed, 0 failed, 201 ignored.

`check-structure.ps1 -Report`: `platform_literals_in_core 0`, `engine_app_references 23`,
`platform_cfg_sites 178`, `neutral_platform_cfg 0`.

## Live checks on old-pc

Okur reading in JAWS's place (`run-scenario.ps1 -Reader okur -OkurDebugLog`), one trial each,
research, never evidence.

**4.1 and 4.2** (release build of `47514a2b`): every scenario spoke through the speech thread's
own synthesizer. `okur-lab latency`'s focus reads (count, p50, p90, p99/max in ms), the last
run before today's changes then this one:

```
106   before  4  27.9  48.5    48.5    after  4  26.2  72.1   72.1
201   before  1  60.6  60.6    60.6    after  4  22.9  33.8   33.8
3155  before 18  49.9  56.5  1557.2    after 18  53.3  64.7  782.4
3158  before 14  40.9  53.1   109.5    after 14  37.4  57.6  103.0
3161  before 13  96.2 130.6   211.6    after 13  92.3 126.7  187.3
510   before  3  46.1  79.7    79.7    after  3  49.4  91.4   91.4
3159  before 13 118.3 158.5   334.0    after 11  36.2  90.3   97.8
```

Within noise: nothing got slower. 3155's trial failed two steps (s6, s18) on the wait for
silence: a second voice, not Okur's, said the list's announcement again 5.7 s after each Tab,
with a bookmark between every word. It was heard before these changes too (3159, 1113), and is
now a pitfall in [`verification-loop.md`](../references/verification-loop.md) §8; who speaks is
not found.

**4.4** (release build of `13a02546`): 221 and 222 passed, every step: Okur's own Links List,
"no links", "no form fields were found" and "There are no PlaceMarkers on this page." came
through the moved UI host, which started `okur-ui` once in each.

Not checked: Word and Excel (Office's Activation Wizard on old-pc, as before).

## Next

1. Step 4.5, the event loop: `okur_runtime::run(Platform, Config)` takes `run.rs`'s loop (about
   620 lines, 13 calls into `okur-windows`), the hook's callback with its key deadlines, the
   exit order and `init_logging` (which removes the known edge `okur-lab -> okur`); a new
   `crates/okur/src/windows.rs` keeps Windows' start-up. It needs the live check (P4).
2. 4.6 (the driver gated once at its root) and 4.7 (`bridge.rs` and the non-Windows start
   branch deleted).
3. The review's open items: P4, P8 (Word and Excel, waiting for Office on old-pc), P3, P1's
   pieces, P15.
4. Then JAWS's features from Settings Center and the help (the plan's §3).
