# Performance and hardening review, 2026-10-25

A read-only review of the whole codebase at `9a103571`, made at the owner's request after the
settings and okur-ui rounds, as the reviews of
[2026-10-24](performance-review-2026-10-24.md), [2026-10-23](performance-review-2026-10-23.md)
and [2026-10-20](performance-review-2026-10-20.md) were.

**Who reviewed what:**
- one reviewer agent: the settings area and its engine glue;
- two more, in parallel: okur-ui and okur-ui-model, and the event tracer, the speech thread
  and the Word ribbon module.

**Method:** benchmarks of their own outside the repository, in this session's scratchpad, on
the dev box (Windows 11, i5-12500H, release builds). Nothing ran live. The figures below are
theirs; each fix was checked against the code before it was made.

Statuses are this session's: **fixed** with the commit, or **open**. Fixed findings stay here
until the next review's pass, then go to git history.

## Findings, by user impact

| # | Kind | Finding | Status |
|---|---|---|---|
| 1 | Bug | A list chosen in Virtual HTML Features opens over it and so is owned by it (`dialog::show` infers the owner from the window in front), and Windows destroys it when Virtual HTML Features closes: the list vanishes, the focus is lost, Okur believes a dialog is open. | fixed `611c43a4` (a closing dialog passes the dialogs it owns to its own owner); not checked live yet |
| 2 | Latency | A value change of any object in the application in front was read as the focused field's text: in Word, the ribbon's Font boxes change as the caret crosses a style, and each read the whole document. 2,715 of 3,861 foreground value changes in the corpus's traces were not the focus's; 1,978 carried a full text read, 1,024,002 characters. | fixed `c86b6804` (converter and live pre-check); parity's baseline file unchanged |
| 3 | Latency, speech | A voice context going back to the synthesizer's own voice (after the keyboard's voice spoke) started a new synthesizer each time; OneCore's can take 5 s to start, and the old one talked on. | fixed `7748d63d` (each synthesizer tells its default voice; voices listed once; OneCore's dropped synthesizer drops its queue) |
| 4 | Robustness | One value no setting takes, in any settings file, reset every setting to Okur's defaults for that place (`SettingsLayers::resolve`). | fixed `7748d63d` (a layer that does not read is merged value by value, the wrong value left out with a warning naming it) |
| 5 | Robustness | A speech batch that panicked was never counted out: Okur was "speaking" forever, live regions and exit waiting on it; pitch and rate sums could overflow. | fixed `611c43a4` |
| 6 | Performance | okur-ui copied a control's whole item list on every WM_NOTIFY before looking at its code; a list cleared sent one LVN_DELETEITEM per row. 0.4 s per refill of 2,000 links, 2.4 s of 5,000. | partly fixed `c86b6804` (only handled codes look up; LVN_DELETEALLITEMS answered). Open: a slim control kind without items; `LVS_OWNERDATA` |
| 7 | Stall | Apply and OK wrote every settings file on the engine's thread, reading every application's and site's file to list them: 100 ms median with 300 site files. | fixed `2c12e6ce` (only changed files written, names listed unread). Open: the write off the engine's thread |
| 8 | Performance, behavior | Settings Center sends every row and a new pane on each Space, and okur-ui walks the whole tree per row (O(N²)); a tree's own TVM_EXPAND came back as the user's opening, so groups a search showed stayed open. | behavior fixed `42c68661`; performance fixed `988e9fad` (only changed rows sent), `95518883` (an item index per tree in okur-ui); not checked live yet |
| 9 | Robustness | A dialog destroyed other than by `close`, or an okur-ui restart, left Okur believing its dialogs open. | partly fixed `611c43a4` (WM_NCDESTROY reports DialogClosed). Open: the host's restart telling the engine |
| 10 | Behavior | A lowered okur-ui was allowed the foreground only for dialogs, not the Startup Wizard, the main window, the viewer, file dialogs and menus. | fixed `c86b6804`. Open: try SetForegroundWindow before attaching to a hung thread |
| 11 | Robustness | The WinEvent pump's own reads ran outside the panic guard: one panic would end event speech for the session. | fixed `c86b6804` |
| 12 | Undefined behavior | An 8-byte write through a 4-byte-aligned RECT (`dialog.rs`, TVM_GETITEMRECT). | fixed `c86b6804` |
| 13 | Robustness | Every synthesizer call's result is discarded: a dead OneCore worker is silent forever. | fixed `bda32db1` (logged), `00290780` (a new synthesizer after five failures in a row) |
| 14 | Behavior | OneCore's audio stays on the device it first opened. | open (needs a live check) |
| 15 | Stall | The voices list was locked while a new synthesizer's voices were enumerated. | fixed `c86b6804` |
| 16 | Bug | Choosing another place in Settings Center kept the previous place's rows (Last Changed Settings). | fixed `c86b6804`. Open: what JAWS does with unapplied changes then (record first) |
| 17 | Bug | Okur's main window's text was a 0 by 0 control: the window was known only after it showed. | fixed `c86b6804` |
| 18 | Performance | Settings by name serialize the whole Settings per get (8.3 µs) and set (18.5 µs); about 58 gets per Space; `StaticMessage::suffixed` scans 1,417 ids. Fine today. | open |
| 19 | Design | `DialogKey::ChangeBack` maps to `change(false)`, a dead branch; Tutor and Access Key logic duplicated in Basics and the Startup Wizard; application knowledge (process names, the NetUI policy) in the platform's event source. | partly fixed `c86b6804` (`strip_prefix` for menu ids), `e62e9513` (Tutor and Access Key choices shared, `Verbosity::chosen`, `choose`); the NetUI policy open |
| 20 | Small | The deferred WinEvent queue drops its newest event on overflow at debug level; Word's ribbon reads 21 sibling positions it never speaks; UIA notification text logged before the foreground filter; okur-ui's diagnostics go to a null stderr; wizard pages accumulate per Next and Back. | open |

## Checked and found fine

- `pane_update`'s clone and compare: 0.1 µs.
- Settings Center's message handling in the engine: at most 0.63 ms a message.
- `SettingsLayers::resolve`: 10.4 µs, once per application or site switch.
- Bounded: `settings_history` (100), the speech history, the start options.
- No RefCell borrow held across a Win32 call in okur-ui; DLGTEMPLATEEX alignment is right;
  menus, brushes and fonts are freed.
- The converter: 3.3 µs an event on average.
