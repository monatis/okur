# Handoff: everyday-use gaps, round 5 (2026-10-20)

Follows [`2026-10-20-everyday-gaps-round-4.md`](2026-10-20-everyday-gaps-round-4.md) and covers
the 26 commits after it (`4ebc29c` to `165e94a`), under the plan
[`docs/plans/2026-10-21-everyday-gaps-round-4.md`](../plans/2026-10-21-everyday-gaps-round-4.md).
The next plan is [`docs/plans/2026-10-21-everyday-gaps-round-5.md`](../plans/2026-10-21-everyday-gaps-round-5.md).

## The owner's answers and requests

- **Yes:**
  - to the Defender exclusion on old-pc (the owner adds it);
  - to the development certificate (the secure-screens proposal's Q1);
  - to the setup's two defaults (the sign-in Run key; no copy on the secure desktop until M6).
- **Office on old-pc:** checked last, after everything else (`4ebc29c`).
- **README:** brought up to date (`d99a236`).
- **JAWS is Vispero's trademark:**
  - Okur's own text must not use it as Okur's name. AGENTS.md §1.5 states the rule (`ed94e0c`).
  - A subagent made the full pass, merged as `9ddca42` to `0becb02`.
  - Parity reads JAWS's name for itself as Okur's on both sides of every comparison.
  - `no_message_names_jaws` and `no_literal_names_jaws` fail on "JAWS", "Freedom Scientific" or "Vispero" in the catalog or in user-facing literals.
  - Renamed: `--jaws-key` is `--okur-key`; the settings file's `jaws_key` is `okur_key`. No aliases are kept, as the Development Notice asks.

## Decided without the owner (reversible)

- **New accepted divergences:**
  - 522/s7: Say Word in WordPad, where JAWS reads nothing of the document; it matched before only because Okur's laptop layout had no binding there (`db8c087`).
  - "Space run" one column past the end of two lines that have none (614/s23, s27; `2d091fa`).
  - The Intermediate and Advanced copies of Beginner divergences (`7aa5b80`, `bc9725b`):
    - the help keys (601, 611);
    - "Unavailable" after a file dialog (600/s11, 610/s11);
    - the emoji JAWS's bottom line drops (599/s22, 609/s22).
- **Two verbosity output types:** Select (`select`) and JAWS Message, whose settings id is `okur_message` under the trademark rule (`bc9725b`).
- **Not built, with reasons in the plan:**
  - Say Color (needs JAWS's color table: 618, 619 record it);
  - the Word List;
  - 566/s7's "level 3";
  - 567/s12;
  - 610/s6's "Tool bar". Containers in general are kept at Advanced: 20 steps regressed when tried.

## What was done

### Features (each checked with `okur parity` against old-pc's evidence)

- **Verbosity, short forms and Advanced** (`bc9725b`, `7aa5b80`):
  - selection and clipboard messages ("Copied", "Pasted", "cut"; nothing at Advanced; "selected" off at Advanced; "Selection is" left out above Beginner);
  - Insert+E says the button alone;
  - the hot key reads "Find what:, Alt+n";
  - Say Window drops its title line, and at Advanced a group box's name;
  - a table cell's focus is "col 2 row 2".
- **Tables** (`8916763`):
  - "beginning of row" on the first column;
  - JAWS's exact edge sentences ("…first row of the table.", "…first column of the row.", "…last column in the row.");
  - Ctrl+Alt+End reads the cell's link role.
- **Text analysis** (`2d091fa`):
  - pairs brackets, parentheses and quotes over the two lines around;
  - "Unopened ] at column 8";
  - stray punctuation is only clause punctuation and brackets;
  - one "Missing initial cap" per sentence end;
  - abbreviations and numbers are not sentence ends.
- **Say Word in a page field with the laptop layout**, marked when misspelled (567/s8), and Speech On Demand's "Not available in Full Speech mode" (566/s3, s10) (`db8c087`).
- **Rectangles in physical pixels** (`87314da`): a control with its own window gives its window's rectangle, in physical pixels, for 516/s8's PC cursor. Proven only once round 85 records 513 and 516 with this tracer.

### Startup and secure screens

- **`startup-checks.ps1`** (`d1ff9a0`) ran on old-pc between batches. All passed (`6013158`; the log is in the proposal's Status section):
  - an unsigned `okur-sr.exe` with `uiAccess="true"` is refused at start;
  - a second `okur run` replaces the first, and `okur quit` ends it;
  - two `--ease-of-access` starts leave one process;
  - `--sign-in` with both Shift keys held starts none.

### Harness

- `queue-batch.ps1` takes several snapshots as one comma-separated value (`2a43a89`).
- A failed snapshot capture no longer ends a queued batch before it runs (`165e94a`). Round 83's queue process had died that way at 16:37, and the four batches queued after it waited until it was started again by hand at 17:37.
- `cargo run --bin okur --` in the generated files' notices and the tests' hints (`86ec435`): `cargo run -p okur --` fails now that the package builds three programs.

### Recorded and imported

- Round 79 (`7235929`): 593-597 and 599-612, ten scenarios at Intermediate and ten at Advanced. 598 and 613 failed one long step each and wait longer now (round 86).
- Rounds 80 and 81 (`97da82b`): 614 (text analysis rules) and 615-616 (a table's edges).
- 617 (selection sizes, round 82) is recorded but waits in the scratchpad for its page's snapshot (snap87).
- Written and queued: 618-619 (Say Color on 364 colors; `d7ede4e`) and 620 (the selection cutoff; `84fd671`).

## Numbers

`cargo run --bin okur -- parity` at `129c052` (the round-4 handoff's) and at `86ec435`:

```
JAWS spoken parity: 3054/3320 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 3162/3372 steps match exactly; 75 accepted divergences not counted (0 scenarios could not run)
```

```
JAWS spoken parity: 3365/3642 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 3484/3694 steps match exactly; 88 accepted divergences not counted (0 scenarios could not run)
```

At `86ec435`:
- `cargo run --bin okur -- invariants`: `Buffer invariants: 0 violation(s)`;
- `cargo run --bin okur -- corpus-manifest`: `Corpus manifest is current.` (283 scenarios, 903 trials, all succeeded);
- `cargo test --workspace`: 34 test binaries, 1220 passed, 0 failed, 199 ignored, summed from its `test result` lines.

## old-pc when this was written (17:40)

1. Round 83 is recording 618 and 619 (Say Color), started again at 17:37.
2. Then:
   - snap84 (the two color pages' snapshots);
   - round 85 (513 and 516 with the tracer of `87314da`, `okur.exe` f0c4bb20, installed as it starts);
   - round 86 (598 and 613 again);
   - snap87 (the selection pages' snapshots);
   - round 88 (620).
3. No Defender detection since 14:13.

## For the owner

- Your own `%APPDATA%\Okur\settings.toml`: the key `jaws_key` is now `okur_key`, and Okur no longer reads the old name. With the laptop layout, rename that line (or choose the key again in the settings dialog), or CapsLock stops being the Okur key.
- The certificate's step 2 (the plan's §5): `crates\okur-setup\signing\dev-signing.ps1 -NewCertificate` on the dev box. Then send the agent the thumbprint.
