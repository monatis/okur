# Backlog: what is known and not done yet

A living list, so nothing found along the way is forgotten. Each item says where it was seen:
a scenario and step, a handoff or the parity gap audit. When an item is done, it is deleted
here; its record is the commits and handoffs that did it. An item whose JAWS behavior is not
recorded yet says so: per the owner's rule, the documentation comes first, then a recording on
old-pc, then the code.

Where the JAWS behavior is known, AGENTS.md rule 6 also asks for an `#[ignore = "known gap: ..."]`
test asserting it. Few exist yet (see "Measurement" below).

The active work is in [`2026-10-18-reading-settings.md`](../plans/2026-10-18-reading-settings.md):
punctuation, typing echo, key echo, character reading, words and numbers, capitals, verbosity,
and per-application and per-site settings. Those are not repeated here.

## Everyday use

[`everyday-use-gaps-2026-10-19.md`](everyday-use-gaps-2026-10-19.md) compares JAWS 2025's
help, manual and tutorials with Okur. It lists the gaps still open, ranked for everyday use, the
places where Okur contradicts JAWS's documentation, and the order of work. P0:
- Say All in a dialog reads the last web page;
- Word and Outlook.

## Dialogs and lists

- **Lists not built:** Smart Glance Highlights (Insert+Ctrl+Y), Anchors, Frames and List Items.
  Virtual HTML Features says "Okur has no … yet" for them. Each needs its recording first.
  Smart Glance items were recorded (243: "9 p.m.", "most", "Branch"), but which text JAWS
  counts as highlighted is not known.
- **Virtual HTML Features' PlaceMarkers item** still says "Okur has no PlaceMarkers yet"
  (`Feature::list` in `crates/okur-core/src/ui/lists.rs` has no list for it), though the
  PlaceMarker List exists (`d58d8f71`): it should open that list, as JAWS's does.
- **Flexible Web's View or Change Where Rules are Applied** (751): Space on a rule's check box
  says nothing in the replay, where JAWS says the item before and after ("Okur start rule Site
  not checked 1 of 1 | checked | ... checked 1 of 1", s9): the replay's mirror of Okur's list
  view or the live reader, to tell with a live run; the delete question names the dialog under
  it again (s21); a tree item one level down says "level 1" where JAWS says none (s7).
- **The rest of the helper dialogs plan** ([`2026-10-17-helper-dialogs.md`](../plans/2026-10-17-helper-dialogs.md)):
  the reading highlight and JAWS's Settings Center.
- **The rest of the message catalog groundwork** (M0, plan 2026-10-18 dialog round, step 2):
  - role and state marks still written by hand in `SpeechFormatter` and `linearize.rs`;
  - voice contexts;
  - okur-ui's Virtual Viewer window title and its "Ready";
  - how JAWS's `ACCESS_KEY=a|b|c` fields map (check by changing the setting on old-pc).

## Reading settings

- **Punctuation by word** (the owner, 2026-10-24): JAWS names the punctuation when reading word
  by word, whatever the level. Research and sources:
  [`jaws-punctuation-by-unit-research.md`](../references/jaws-punctuation-by-unit-research.md).
  - **Built 2026-10-24**, recorded in 1037 (classic Notepad) and 1041 (a RichEdit control, the
    family of Windows 11 Notepad), each at Most, None and Some:
    - a word read on its own is given to the synthesizer with every symbol named, in pages and
      web fields as in desktop fields (`f1e3307a`), and by Say Word (`817de792`);
    - a word that is one symbol is that character alone, "em dash —";
    - a plain Win32 edit's word is the whole run around the caret, between spaces and hyphens
      (`f54c4539`); a RichEdit's is its own word unit, ":\", "Users", "\".
    - Say Word in a plain Win32 edit, on a symbol, says the symbol alone (`bd78520f`).
    `okur-lab parity --only` 1037, 1041, 106 and 213: every step matches ("100/100",
    "100/100", "33/33", "19/19 steps match exactly").
  - **Left:** the Okur cursor's words (`cursors.rs`) and the Virtual Viewer's still go through
    `speak` at the level, not recorded by word yet; a live check in Windows 11 Notepad on the
    dev box, with the owner's leave.

## Virtual cursor keys not built

The owner found N unbuilt (2026-10-19). The whole key map was then compared with Okur's:
[`jaws-quick-navigation-keys.md`](../references/jaws-quick-navigation-keys.md) lists every
virtual cursor key of JAWS 2025 (`Default.JKM` on old-pc), what Freedom Scientific documents
for it, and Okur's status. Scenarios 289-299 record the gaps on old-pc; `virtual-keys.html`
is their new fixture.

Order of work, most used first:
1. Y: Smart Glance highlights (292).
2. Slash: clickable elements (293).
3. Not recorded yet:
    - the element information (Insert+Shift+F1);
    - the clickable and mouse-over lists, Insert+F9.

## Forms and browse mode

- **Forms with Up and Down in browse mode** (the owner's request, 2026-10-18).
  - Open: real pages with forms that need no sign-in. GitHub's new issue form needs one: never
    sign in or submit.

## Web applications

- **A focus the buffer has no line for** (219; audit §2.2): in WhatsApp-like message lists Okur
  leaves the virtual cursor on the previous focus and says the page title again. Also two
  dev-box rules that old-pc contradicts: how a plain focusable element and a page's popup menu
  items are said.
- **A page menu opened by the page's own shortcut** (218/s10): the owner decides the hint.
- **Chrome's own controls** (220; audit §2.4): "New tab page", "Close Document Window", the
  page named after leaving Chrome's menu, and Down in the address bar.
- **Docs:** JAWS's "Type in text." after "Leaving menus". **Sheets:** a module for its font
  (handoff 2026-10-17).
- **WhatsApp:** a live check of the partial page reads on the dev box, with the owner's
  permission (`okur latency` before and after a page change).
- The open items of [`2026-10-17-web-apps-round-3.md`](../plans/2026-10-17-web-apps-round-3.md)
  step 2 and [`2026-10-16-web-apps-round-2.md`](../plans/2026-10-16-web-apps-round-2.md).

## Native desktop

- **Dialogs** (323, 322; audit §2.3):
  - the page's name and text on Ctrl+Tab ("Security Page | For special permissions or advanced
    settings, click Advanced." 323/s10): the tracer has the page's name but not its own text.
    Which statics JAWS counts as a page's text needs experiments on old-pc: the tab control
    spans the page, and labels such as "Group or user names:" are not read;
  - Say Window (323/s2): done but for JAWS's "Opens with: Opens with:", which the tree does
    not explain;
  - a list item with its columns and position after the viewer closes (322/s18);
  - the viewer shows a path clipped as on screen ("sr-property-fixtur", 323/s7): a side effect
    of JAWS's screen model, a candidate for accepted-divergences.md.
- **A list view's control view** (322/s15, s16): JAWS shows its last row with its columns, Okur
  another row without them.
- **File Explorer** (dev box, handoff 2026-10-17):
  - the Details view button's help text;
  - Ctrl+A reading every selected item with its columns;
  - "Working on it..." on Alt+Up;
  - "not checked" on pinned Quick Access items.
- **Windows Settings** (handoff 2026-10-17): dialogs of elevated processes ("Rename this PC")
  need an elevated Okur; the search box's doubled "No results".
- **Calculator:** JAWS's digits, expressions and operator names (handoff 2026-10-17). Not built.
- **Word:**
  - A form field's drop-down list opened (Alt+Down: "Open List Box | Context Menu | ...") and
    chosen (Enter: "Leaving menus | Branch: Combo box | Northside | 2 of 3 | To change the
    selection use the Arrow keys. | ..."), and Ctrl+Home onto a field, which JAWS reads three
    times [obs, 1057/s7, s9, s11]: not built; the field after Enter is.
  - Word's other dialogs than Font (Paragraph, Find and Replace...): the module reads any
    `bosa_sdm_msword` dialog, recorded on Font only. The ribbon's galleries.
  - Done: List Fields (`3814d83c`), Say Font's line spacing (`484d7f0d`), form fields as
    controls (`0794d517`), links marked in a line (`0936d6d8`), Read Mode's warning
    (`3c328b76`), the Quick Access Toolbar and title bar (`a77f4ee5`), the Font dialog
    (`6525e534`).
- **Not covered yet:** Task Manager, a desktop mail client.

## Installer

- **A desktop icon that opens Okur's window** (the owner, 2026-10-24): okur-setup creates a
  desktop shortcut, as JAWS's installer does, that starts Okur or, with Okur running, brings its
  main window forward (Insert+J's window, [`../references/okur-settings-redesign.md`](../references/okur-settings-redesign.md)
  §3.2). A choice in setup, on by default (`--no-desktop-icon` for an unattended install);
  uninstall removes it. Today setup creates no shortcut at all, nor a Start menu entry: find
  what JAWS's installer creates (its documentation, then the installed shortcuts on old-pc)
  and do the same with Okur's name.

## Feedback sounds

- **JAWS plays a sound for some actions** (the owner, 2026-10-24): forms mode turned on and off,
  a layered key's first key (Insert+Space, before the key that follows), and more, for example
  the "Top And Bottom Edge Alert with Sound" that Quick Settings lists (General Options), and
  the speech and sounds schemes (customization map rows 117, 124). Okur plays only a
  notification rule's sound file (`crates/okur-core/src/sound.rs`).
  - Find from JAWS's documentation which actions sound and with which setting, then record
    on old-pc when JAWS plays each: the recorder captures speech only, so this needs the
    system's audio output captured beside it (the `wait-silence` loopback) or JAWS's sound
    calls seen another way.
  - JAWS's own sound files are JAWS's: never copied or read (AGENTS.md §1.4). Okur ships its
    own, royalty-free UI effects, for example from
    [Uppbeat's UI sounds](https://uppbeat.io/sfx/category/digital-and-ui/ui) or
    [Pixabay's UI notification sounds](https://pixabay.com/sound-effects/search/ui%20notification/),
    each file's license checked for redistribution in an Apache-2.0 repository (and its
    attribution kept in `NOTICE` where asked) before it is added.
  - A setting per sound (on, off), as JAWS's schemes allow, and the sounds' volume with the
    voice's or on their own.

## A page's first read (UIA, before the IA2 read)

From the audit §4:
- preformatted text keeps its lines (207);
- `haspopup=menu` gives a menu button (210);
- fonts and message groups (209, 219).

Block quotes cannot be seen through UIA. Measure first how long users stay on the UIA buffer
(`okur latency`).

## Responsiveness

- **The 2026-10-19 review** ([`responsiveness-review-2026-10-19.md`](responsiveness-review-2026-10-19.md)):
  measured where possible; its findings 13, 16 and 17 are still open.
- **The 2026-10-25 review** ([`performance-review-2026-10-25.md`](performance-review-2026-10-25.md)):
  findings 13, 14, 18 and 20 open, 6-9 and 19 in part; the okur-ui fixes are not checked live yet.

## Measurement and harness

- **Traces before the field colors and the selection's caret** (510/s14, s15): 510's trace has no
  colors for its WinForms field (928's has them, and Okur says "black on white" as JAWS does);
  the caret a selection leaves is the selection's start in every trace of a Win32 edit
  (`EM_GETSEL` cannot tell its ends apart), where JAWS said the last selected character after
  Shift+End (".").

- **The signup fixture's `help@riverside-library.org`** is not at a reserved example domain, so
  the tracer would blank it out of a trace (`crates/okur-platform/src/privacy.rs`). It has never
  appeared in one; move it to `example.org` the next time `signup-form.html` is re-recorded.
- **Known-gap tests** (AGENTS.md rule 6): `crates/okur-corpus/src/lib.rs` replays the step and
  asserts JAWS's speech (`parity::run_one`). Add one for each gap whose behavior is recorded;
  `cargo test -- --ignored` runs them.
- **Dev-box rules** (audit §5): rules 1 to 3 (a plain focusable element, a page's popup menu
  items, the page after Chrome's menu) came from the owner's real WhatsApp; 219's fixture only
  imitates it, so they wait for a recording that settles them (WhatsApp-like markup, or the
  owner's live check).
- **Live runs:** the run files behind `okur live-parity` numbers are kept on old-pc and in
  `target/live/`, not in the repository. Decide where to keep them so a number can be
  checked again.
- **Re-recordings for evidence questions** (audit §8 item 13):
  - 01 in format 2, or retire it;
  - 305 with a clipboard capture after s13 (JAWS said "selected do" where the keys select "bo");
  - 111's final "end";
  - 207's Insert+T in a region under a heading.
