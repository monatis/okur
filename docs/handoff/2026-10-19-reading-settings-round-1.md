# Handoff: reading settings, round 1: the harness, and JAWS's punctuation levels (2026-10-19)

Follows [`2026-10-18-helper-dialogs-round-2.md`](2026-10-18-helper-dialogs-round-2.md). Plan:
[`2026-10-18-reading-settings.md`](../plans/2026-10-18-reading-settings.md) (still active; its
step 1 has a status note). Open items: [`docs/backlog/open-items.md`](../backlog/open-items.md).

## The owner's decisions (2026-10-19)

- The two divergences the audit proposed (312's emoji, 313's Say Font) are accepted.
- JAWS's `.sbl` symbol files may be looked at. Okur's per-site and per-application settings get
  their own design; the corpus records the settings a scenario asks for, never JAWS's per-site
  files.
- One backlog for everything known and not done: `docs/backlog/open-items.md`.

## What was done

### Harness

- **The spoken capture no longer stops at JAWS's pauses** (`fb2b266`). JAWS queues its "Copy
  speech history to clipboard" confirmation behind the speech it still holds (tried on old-pc:
  three queued utterances, the copy 400 ms in, the confirmation came fourth). A step's recorder
  records now end where that confirmation begins: `observation.spoken_end`.
- **The 17 scenarios of the audit's 46 cut steps re-recorded** (`2fc0771`): 209, 211–220, 306,
  311, 314, 315, 319, 322. Every trial succeeded; all 786 steps ended at the confirmation.
- **Keys** (`4dfa5f5`): the numpad as with Num Lock off (`Numpad5`, `Insert+Numpad5`), and
  `Comma`, `Period`, `Slash` and others by their US places, since JAWS takes keys by scan code
  and old-pc types on Turkish Q.
- **JAWS settings per scenario** (`4dfa5f5`): `jaws_settings` in a scenario, written by
  `jaws-settings.ps1` and applied by `run-batch.ps1` with a JAWS restart; `run-scenario.ps1`
  refuses to record with the wrong settings. Tried on scratch copies only: the first scenario
  that uses it will prove it.
- **Traces keep no e-mail address** (`0c2de10`): re-recording 220 caught Chrome's new tab page
  naming old-pc's signed-in Google account with its address. The tracer now blanks addresses
  outside the reserved example domains. The address is still in the trace committed in
  `f60b3ac` (history is not rewritten).
- **Replay places early-stamped events** (`aed5738`): Windows dates events by a tick count that
  advances every 10 to 16 ms, so an event can be stamped before the key that caused it (306/s2).
- **Live runs record Okur's synthesizer text** (`1b6a581`): the SAPI 5 driver listed only a
  placeholder voice, so Okur could not speak through the recorder voice.
- **`capture-window.ps1`**: a window's UI Automation tree, run elevated, for JAWS's own windows.

### JAWS's punctuation, found, recorded, built

- **Found:** the help documents the levels (research Item 1).
- **Recorded:**
  - JAWS's Customize Punctuation list, read on old-pc through JAWS's own speech
    ([`jaws-punctuation-levels.md`](../references/jaws-punctuation-levels.md)): 75 symbols,
    their names and levels;
  - scenario 400 (`124380c`): Insert+Shift+2 four times, lines 1 to 4 read at each level. The
    level is not saved.
- **Built** (`4e4324e`):
  - `pronounce::spoken` gives the synthesizer what JAWS gives its own: names by level, the
    special places of periods, word splitting, single digits, times, percents, apostrophes,
    and "space Bar";
  - selections and single words name every symbol;
  - Insert+Shift+2 (JAWS's SetPunctuationLevel) cycles the level for the session.

  A prototype of the rules matched 1352 of the corpus's 1363 history lines paired with what JAWS
  sent; the rest are pairing mismatches and emoji.

### Engine

- A field one of JAWS's lists focuses is said as JAWS says it (`91f413d`): its line in forms
  mode, no "Type in text.", and Select a Form Field's control without the page title again.

## Results (pasted tool output)

`okur parity` and `okur parity --api uia` on `main` (`1b6a581`), before this round's first change
and now:

```
before:
JAWS spoken parity: 529/950 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 1184/1512 steps match exactly; 8 accepted divergences not counted (0 scenarios could not run)
JAWS spoken parity: 490/950 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Uia): 1068/1512 steps match exactly; 8 accepted divergences not counted (0 scenarios could not run)
now:
JAWS spoken parity: 645/975 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 1202/1537 steps match exactly; 8 accepted divergences not counted (0 scenarios could not run)
JAWS spoken parity: 602/975 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Uia): 1086/1537 steps match exactly; 8 accepted divergences not counted (0 scenarios could not run)
```

Scenario 400 added 25 steps (and 25 spoken steps): 1512 to 1537, 950 to 975.

`okur live-parity`, Okur run live on old-pc (run files in `target/live/2026-10-19/`):

```
225-web-form-fields-list        Okur live: 11/11 steps match JAWS exactly; 0 accepted divergences not counted
228-web-form-fields-list-edit   Okur live: 12/12 steps match JAWS exactly; 0 accepted divergences not counted
249-web-list-edits-enter        Okur live: 5/5 steps match JAWS exactly; 0 accepted divergences not counted
```

Scenario 400 read live by Okur through the recorder voice: the synthesizer text of every Down
step at the four levels is the same as JAWS's (compared by eye from the run file, not by a
tool: `okur live-parity` compares the history layer only).

## What is next

1. `okur live-parity` should compare the spoken layer too, now that live runs record it.
2. The multi-line text box (backlog, native desktop): Say Line without the name, "top of file",
   "Contains text" on focus. The tracer needs the window's multi-line style.
3. The plan's next steps: settings with scopes (with TOML for Okur's settings file, proposed to
   the owner on 2026-10-19 and not yet answered), then character reading (numpad 5, phonetic),
   typing echo, capitals, words and numbers, verbosity.
4. Punctuation's open parts: per voice context, the Advanced list, repeated characters (line 7),
   and a punctuation control in Okur's settings dialog.
