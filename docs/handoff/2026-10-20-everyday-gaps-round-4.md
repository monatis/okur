# Handoff: everyday-use gaps, round 4 (2026-10-20)

Follows [`2026-10-20-everyday-gaps-round-3.md`](2026-10-20-everyday-gaps-round-3.md) and covers
the 91 commits after it (`1105631` to `129c052`), done under the plan
[`docs/plans/2026-10-21-everyday-gaps-round-3.md`](../plans/2026-10-21-everyday-gaps-round-3.md).
The next plan is [`docs/plans/2026-10-21-everyday-gaps-round-4.md`](../plans/2026-10-21-everyday-gaps-round-4.md).
The list stays [`docs/backlog/everyday-use-gaps-2026-10-19.md`](../backlog/everyday-use-gaps-2026-10-19.md).

This handoff was written late: the stretch ran from round 55's import to round 78's without
one, which the owner pointed out. Each stretch of work ends with a handoff and a plan.

## Decided without the owner (reversible)

- **JAWS's accidents, accepted divergences** (`docs/references/accepted-divergences.md`):
  - "Unloading JAWS" said before Insert+F4's question (`b241089`, 571/s1): JAWS stays when
    the answer is no.
  - Escape out of the Speech History (`a90380e`, 517/s4, s10; `129c052` adds 570/s10).
  - Two file-dialog accidents (`dd9565e`) and the Font dialog's font said before the dialog
    (`3dd8723`).
  - Up out of 285's text area to the page's top (`7270e97`).
- **A fresh JAWS says no hints on Tab either** (`77f1a76`): 204 recorded again after a JAWS
  restart (`-FreshJaws`) has no "Type in text." on Tab, as the long-running JAWS had none. So
  JAWS's state since its start does not add the hints the verbosity rounds have; their
  settings changes are the next suspect (plan §2).
- **The quit question's words** (`b241089`): JAWS's, with Okur's name, "Quit Okur dialog | Are
  you sure you want to quit Okur? Press Enter to quit now, or Escape to cancel." (571/s1, s2).
- **Short forms only where recorded**: a message keeps its long form at every level until a
  recording shows its short one (`4411754`, `790ae1a`).
- **571 was taken back to pending once** (`77f1a76`): round 71 sent Tab and Escape as keys,
  which JAWS's elevated question never received (no event traced after it opened); JAWS's
  TypeKey sends them now, and round 76 recorded it properly.

## What was done

### Features (each checked with `okur parity` against old-pc's evidence)

- **Verbosity levels.**
  - Settings dialog (`9bee169`); replay runs at the evidence's level (`d9194a2`).
  - Advanced leaves out:
    - a list's position as the arrows move (`91ddd3b`);
    - a page field's description (`98ee840`);
    - a dialog control's group box name, CONTROL_GROUP_NAME `1|2|0` (`0abb49d`);
    - an open combo box list's position (`0abb49d`).
  - Short messages above Beginner (`4411754`, `790ae1a`, `8822802`):
    - "virtual PC" for the virtual PC cursor;
    - a tree level's number alone;
    - "3 columns and 3 rows";
    - "First row", "Last row", "First column", "Last column";
    - no "Open List Box";
    - Insert+T's title without "Title is".
  - A part of an announcement can carry its short form (`Part::short`); the engine's verbosity
    filter applies it. `docs/references/messages-and-verbosity.md` has the model and JAWS's
    shipped `[OutputModes]`.
- **Reading and quick keys.**
  - S and D, Ctrl+Insert+Up, Right and Left by element type (`dbc56b8`, 290 14/14, 558 28/29).
  - Shift+Period and Shift+Comma outside lists (`3704ef4`).
  - J and Shift+J, Jump To Line (`9a4fdca`).
  - "Has Flows From" (`1489e7c`).
  - Misspelled words in field reads (`04ce857`, `c5aab44`).
  - Say Line on a page field with its name (`b6b1b74`).
  - T and Shift+T say when they wrap; Ctrl+Alt+Up at the top says "top of column" (`790ae1a`).
- **Windows and dialogs.**
  - Insert+F11, the system tray list (`d4b5da4`, `1afdf1e`); the Window List switches windows
    (`c01c477`).
  - A Win32 toolbar's buttons (`9f7c218`); an edit spin box (`6a0a7b0`); an edit combo's
    position (`b8d4be3`).
  - The file dialog's folder on Say Focus (`c88b674`); an unlabelled dialog list named by its
    dialog (`dd48218`).
  - The task switcher read only while Alt is held (`4d8beb0`).
- **Cursors.**
  - Route PC to Virtual on a graphic reads it (`b6102e0`, 569 16/16).
  - Screen positions count pixels from 1 and the PC cursor is a middle pixel (`c719d58`,
    516/s2); okur is per-monitor DPI aware (`5ccd78c`).
- **Text analysis** (`caef4cc`): Insert+Tab on a text field names its line's inconsistencies
  as JAWS does (568 29/29, 571 4/4): runs of spaces, a sentence's small first letter, inverted
  case, stray and unspaced punctuation, open brackets, braces and quotes.
  `crates/okur-core/src/text_analysis.rs`.
- **The Speech History layer** (`129c052`): Ctrl+H copies the history, Shift+H clears it (570
  9/9).

### Startup and secure screens (a subagent, merged at `c118272`)

The proposal [`docs/references/startup-and-secure-screens-proposal.md`](../references/startup-and-secure-screens-proposal.md)
(`0dab87c`), M0 to M3, has its own Status section:
- `okur doctor` (`1aa5d29`);
- `okur-sr.exe` with UI Access, one Okur per desktop, the control pipe, `okur quit`;
- Insert+F4 with its question (`dc0fba1`);
- `okur-setup` install and uninstall, and the development signing script, never run
  (`f466ec2`).

Nothing was signed, installed or registered. The rebase on `main` had one conflict
(`commands.ftl`), resolved by hand. After merging, `cargo test --workspace` passed, and
`okur parity` gave the same figure as before the merge.

### Performance (the review `docs/backlog/performance-review-2026-10-20.md`)

Its Status table lists what is done and measured: P1 steps 1-2, P2 step 1, P4 steps 1 and 3,
P5 step 1, P8, P9 steps 1-2, P10. For example, Z on the 25,032-node page went from 559 to
9.7 ms (`crates/okur-core/examples/big_page_keys.rs`). `parity-snapshot.sh` gave identical
output before and after each change.

### Harness and replay

- A step may copy the speech history itself (`"copies_speech_history"`, `3f38983`); capture
  script `1a29afa8` since round 74.
- Replay:
  - knows where the mouse was before step 1 (`c719d58`);
  - knows a recording's clipboard (`c1389db`);
  - leaves out JAWS's Results Viewer and its main window's own focus (`a90380e`,
    `1105631`).
- `lab-fetch.sh` time limits and retries (`7b85ed1`); `okur lines --spans` (`dbc56b8`).

### Recorded and imported

- Imported rounds:
  - 55 to 59, snap60 (`e380b72`, `9c3ae18`, `bac9656`, `a09707a`, `d566f61`);
  - 558 with its snapshot (`c940efd`);
  - rounds 70-72: 569 new; 516, 546 and 204 again (`77f1a76`);
  - 572-591, ten scenarios at Intermediate and ten at Advanced (`96f3a5b`, `90c9984`);
  - 562, 568, 570, 571, 592 (`c838f3a`).
- Written and queued: 592-613 (`b6819bc`), including a Turkish page (613,
  `turkish-text.html`), and 614 (`197346f`).
- Windows Defender quarantined `sr-harness\bin\okur.exe` as `Trojan:Win32/Bearfoos.A!ml` at
  10:52 and 14:13 (`Get-MpThreatDetection` on old-pc). Rounds 61-64, 73 and 74 failed for it;
  65-68 ran empty, queued before their files arrived. Each was queued again and recorded after
  a fresh `okur.exe` was pushed.

## Numbers

`cargo run -p okur -- parity` at `c5c2ba9`, as round 3's handoff quotes it, and at `129c052`:

```
JAWS spoken parity: 2361/2600 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 2463/2652 steps match exactly; 63 accepted divergences not counted (0 scenarios could not run)
```

```
JAWS spoken parity: 3054/3320 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 3162/3372 steps match exactly; 75 accepted divergences not counted (0 scenarios could not run)
```

At `129c052`:
- `cargo run -p okur -- invariants`: `Buffer invariants: 0 violation(s)`;
- `cargo run -p okur -- corpus-manifest`: `Corpus manifest is current.`;
- `cargo test --workspace`: 34 test binaries, 1211 passed, 0 failed, 199 ignored (live and
  known-gap tests), summed from its `test result` lines.

## old-pc when this was written (15:34)

- Round 79 is recording 593-613 (11 of 21 done; 598 failed, not looked at yet).
- Round 80 (614) is queued behind it.
- `okur.exe` 692959fe is installed, pushed at 14:4x after the 14:13 quarantine.
- No Defender detection since 14:13. The exclusion is still the owner's to add.

## Not done, and why

- Recorded but not built yet:
  - 562's "selected N characters up to" rule (5 steps);
  - Say Color (565, 10);
  - the Word List (560, 8);
  - Jump to Line's targets (559, 8; JAWS's dialog keys go through TypeKey);
  - Speech On Demand (566, 3).
- 516/s8 and 513: a WinForms window's rectangle arrives unscaled at 125%. The fix needs a
  probe on old-pc while no batch runs (plan §3).
- The text analysis's open rules (parentheses, how far pairing reaches, leading spaces): wait
  for 614.
- Live checks never made:
  - the performance items P2, P4, P5, P8;
  - the Window List's switch;
  - the secure-screens M1 and M2 checks.

  Each needs a live run on old-pc or the owner's dev box.
