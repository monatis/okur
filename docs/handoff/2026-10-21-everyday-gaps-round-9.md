# Handoff: everyday-use gaps, round 9 (2026-10-21)

Follows [`2026-10-21-everyday-gaps-round-8.md`](2026-10-21-everyday-gaps-round-8.md) and covers
the commits after it (`98c0ae0` to `e3eab4f`, and this handoff's), under the plan
[`docs/plans/2026-10-21-everyday-gaps-round-8.md`](../plans/2026-10-21-everyday-gaps-round-8.md).
The next plan is [`docs/plans/2026-10-21-everyday-gaps-round-9.md`](../plans/2026-10-21-everyday-gaps-round-9.md).

## The owner's answers and requests

None new in this stretch. The owner's steps (the development certificate, Focus Assist, the
Defender exclusion, Office) are as before.

## Decided without the owner (reversible)

- **A plain multi-line edit is silent on Home** (`eabf108`, `5de2dbe`). The "document" rule
  (Home reads the character it lands on) came from Windows 11 Notepad's rich edit on the dev
  box. On old-pc JAWS says nothing for Home in Windows 10 Notepad's edit and in the WinForms
  text box, in every recording (504/s5, 664/s9, 406/s8). Ctrl+Home there still says "top of
  file" (660/s4). UI Automation calls Windows 10 Notepad's edit a Document too, so a plain
  edit is known by its class (Edit, or a WinForms edit).
- **Say Color in a native text field** names its colors (`98c0ae0`): UI Automation's
  foreground and background color attributes, or for a Win32 or WinForms edit that reports
  none, the system colors it draws with. A control without text colors is "default on default".
- **A page field's colors are not read yet** (662/s5, s7). Giving them to page fields would
  also change Say Font there, and 662/s7 needs colors in the page's own text.

## What was done

### Features (each checked with `okur parity` against old-pc's evidence)

- **Say All to a page's end says its last word again** (`bbca930`): "content information end
  end" (111/s1), "main region end end" (408/s10), "çğıöşü çğıöşü" (613/s26).
- **A page's last line spells without its space** where the page's text ends without a line
  break (613/s24). That rule (`ends_with_line_break`) is now the buffer's, shared with a
  selection's count.
- **Ctrl+Home and Ctrl+End by verbosity** (`281713d`): "top of file" and "Bottom of file" at
  Beginner, "top" and "Bottom" at Intermediate, the line alone at Advanced (660, 668, 669).
- **A dialog shown after its focus is read again** (`9118ba1`, `a47a17e`). Notepad's Font
  dialog takes the focus while still hidden, so Okur read it empty and Say Window said nothing.
  The tracer now reads the dialog in front again when it is shown (`dialog_shown`), and the
  engine takes its controls silently (`WindowShown`). 665/s3 now has Say Window read the Font
  dialog; its wording still differs from JAWS's (next plan, §1).
- **P6 step 3, again** (`46e1a2c`): a dialog's list items are read where shown. 665 showed
  JAWS's Say Window reads the visible fonts, not all 187, so the revert (`b13f5a7`) was wrong.
  541's scenario description said otherwise; it is left as recorded.

### Performance

- **P6 step 3**: the Font dialog's `window` stage per focus, from `okur latency` over the
  tracer's logs (count, median, 90th percentile, 99th, in ms):

  | Round | Count | Median | 90th | 99th |
  |---|---|---|---|---|
  | 97 | 17 | 1.0 | 843.0 | 852.0 |
  | perf105b | 17 | 1.0 | 673.0 | 713.0 |
  | perf113 | 18 | 0.0 | 257.0 | 475.0 |

### Harness

- **run-scenario and capture-snapshot read files as UTF-8** (`f78ec7b`, `882a89f`, `24c59e6`).
  Every 613 trial had failed its window title check: the fixture's title "Türkçe Metin" was
  read in old-pc's code page. Round 114 recorded 613 with all three trials passed. The earlier
  guesses (a tracer hang, Chrome's translation bubble) were wrong.
- **run-scenario skips a window with no title yet** (`6020b0b`): 670 stopped on one in
  round 118.
- **Waiters on old-pc** pull a build after a round and then queue the next. Two lessons:
  - One waiter was synced half-edited (a failed Write left a sed version) and waited on its
    own output. It had already pulled the right build, so the fix was to stop it and queue the
    round directly (`fix115.ps1`).
  - A waiter is checked by its content before it starts, and replaced only before it pulls.

### Imported

Each import had all trials passed, the committed capture script, and was grepped for links
and personal data.

| Commit | Scenarios |
|---|---|
| `25fcb55` | 664 |
| `99a6028` | 661, 662 again |
| `deeb8c6` | 613, with the snapshots of snap116; 665 |
| `81b325a` | 666-669 |
| `9e5821c` | 665 again |

Not imported:
- round 118's 670 (stopped on a window without a title);
- round 118's 671 (its first trial began with Notepad in front);
- round 110's 613 (the title check). Round 120 records 670 and 671 again.

## Figures

Each is the named command's output at `e3eab4f`:

- `okur parity`: "JAWS parity (Ia2): 4805/4978 steps match exactly; 105 accepted divergences
  not counted (0 scenarios could not run)". At the previous handoff: 4712/4888 with 104.
- `okur parity`, spoken layer: "JAWS spoken parity: 4683/4940 steps match exactly".
- `okur invariants`: "Buffer invariants: 0 violation(s)".
- `corpus/CORPUS_MANIFEST.json` statistics: 334 scenarios, 1056 trials, 1056 successful.
- `cargo test --workspace`: 1252 passed, 0 failed, 199 ignored.

## Learned

- **JAWS's Say Window reads what a window shows**: a list's visible items only (665).
- **JAWS's Say Window reads a combo box** as several parts:
  - its label and "Combo box";
  - its value with the list's shown items;
  - its place ("38 of 187");
  - then its list box, then its edit combo with the value and place again;
  - a drop-down combo box is its label, "Combo box", its value and place.
- **Ctrl+Backspace in Notepad** says the word deleted ("{", "()"); Ctrl+Delete says nothing
  (664).
- **Say All's end** repeats the page's last word.

## Recorded meanwhile, and queued

Round 120 waits on old-pc for its build: 670 and 671 again (111's Say All at Intermediate and
Advanced), Notepad closed first.

## Not done, and why

- **Say Window's wording in a dialog with combo boxes** (665/s3): the next plan's first item.
- **A page field's colors** (662/s5, s7): see above.
- **The clipboard keys, the layer's help and Command Search, notifications, Office**: as before.
