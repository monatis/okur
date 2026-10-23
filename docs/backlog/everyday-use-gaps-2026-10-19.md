# Everyday use: JAWS 2025's documentation against Okur (2026-10-19)

The owner asked for a comparison of JAWS's help, manual and tutorials with Okur, to find what
is still missing for everyday use. In scope: reading settings, verbosity, reading and reviewing
documents, selecting, copying and pasting, skimming, reading by character, word, sentence and
line, Say All, reading from or to the cursor, and spelling. OCR, Picture Smart and the other
assistive utilities come later
([`assistive-ai-and-remote-proposal.md`](../references/assistive-ai-and-remote-proposal.md)).

A review subagent compared them on `main` at `be1e9a6`. It read JAWS's documentation and
Okur's code, ran hermetic replays, and probed `okur-core` in a scratch crate. It changed no
repository file. Parity at that commit, from `cargo run -p okur -- parity`:

```
JAWS spoken parity: 1640/1993 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 1714/2045 steps match exactly; 33 accepted divergences not counted (0 scenarios could not run)
```

The owner's decision (2026-10-19): these gaps are closed before the macOS port or any public
announcement.

Per the owner's rule, each gap is found in the documentation, then recorded on old-pc, then
built. The "Recording" column names the scenario that records it. Scenarios 500-507 were
queued on old-pc on 2026-10-19 as round 12, behind rounds 9-11; 508-512 followed as
round 13.

## Sources

**JAWS 2025's own help.** It is installed with JAWS 2025.2505.43.400 under
`C:\ProgramData\Freedom Scientific\JAWS\2025\Locale\enu\Help\`. Topics are cited by `.chm`
file and topic, with each topic's notes page:
- **[H1]** `JFW.chm`: keystrokes/jaws_keystrokes/jaws_keystrokes_for_reading_text.
- **[H2]** `JFW.chm`: …/navigation_information_keystrokes.
- **[H3]** `JFW.chm`: …/jaws_keystrokes_for_accessing_help.
- **[H4]** `JFW.chm`: …/miscellaneous_jaws_keystrokes.
- **[H5]** `JFW.chm`: …/jaws_keystrokes_for_cursors_and_mouse_simulation.
- **[H6]** `JFW.chm`: …/jaws_keystrokes_for_working_in_dialog_boxes.
- **[H7]** `JFW.chm`: …/keystrokes_for_working_in_tables.
- **[H8]** `JFW.chm`: …/Layered_Keystrokes.
- **[H9]** `JFW.chm`: …/additional_say_all_features ("Change Voice Rate on the Fly").
- **[H10]** `JFW.chm`: keystrokes_for_using_the_clipboard and using_jaws/Using_the_Clipboard.
- **[H11]** `JFW.chm`: …/windows_keystrokes_for_working_with_text.
- **[H12]** `JFW.chm`: using_jaws/controlling_speech/using_the_virtual_pc_cursor.
- **[H13]** `JFW.chm`: using_jaws/Advanced_Reading_of_Screens_and_Documents and
  Reading_Screens_and_Documents.
- **[H14]** `JFW.chm`: customizing_jaws_for_windows/Speech_History.
- **[H15]** `JFW.chm`: customizing_jaws_for_windows/Notification_History.
- **[H16]** `JFW.chm`: …/jaws_dialogs/window_list_dialog and select_a_system_tray_icon_dialog.
- **[H17]** `JFW.chm`: keystrokes/laptop_jaws_keystrokes/laptop_keystrokes_for_reading_text_and_information.
- **[S1]** `SettingsCenter.chm`: SayAll/say_all_options.
- **[S2]** `SettingsCenter.chm`: User/user_options_dialog.
- **[S3]** `SettingsCenter.chm`: SpeechVerbosity/general_verbosity_dialog and speech_output_types.
- **[S4]** `SettingsCenter.chm`: Web-HTML-PDF/html_options_dialog.
- **[S5]** `SettingsCenter.chm`: FormsMode/Forms_Mode_Options_Dialog.
- **[S6]** `SettingsCenter.chm`: Miscellaneous/Miscellaneous_Group.
- **[P1]** `popapps.chm`: the JAWS commands for HTML (JAWS_specific_keystrokes_ie5).
- **[P2]** `popapps.chm`: Word (JAWS_specific_keystrokes_wd, spelling_grammar_wd, track_changes_wd).
- **[P3]** `popapps.chm`: Outlook (JAWS_specific_keystrokes_olk, helpful_hints_olk).
- **[P4]** `popapps.chm`: Excel (JAWS_specific_keystrokes_xl).

**Freedom Scientific on the web:**
- **[W1]** <https://www.freedomscientific.com/training/jaws/hotkeys/>. The same rows are in
  <https://support.freedomscientific.com/Content/Documents/Manuals/JAWS/Keystrokes.txt>.
- **[W2]** Surf's Up, <https://support.freedomscientific.com/SurfsUp/5-Navigating.htm>.
- **[W3]** Surf's Up, <https://support.freedomscientific.com/SurfsUp/9-Forms.htm>.

**Other sources:**
- **[K]** `Default.JKM` and `Chrome.jkm`: key names and command names only (AGENTS.md §1.4).
- **[R]** [`jaws-reading-settings-research.md`](../references/jaws-reading-settings-research.md).

## Gaps, most important for everyday use first

P0 blocks daily use, P1 is expected by a JAWS user within days, P2 is later.

| # | Area | Feature and JAWS keys | JAWS (source) | Okur | P | Recording |
|---|---|---|---|---|---|---|
| 1 | Reading | Reading keys on native controls: Insert+Up, Insert+Down, Numpad 5, Insert+Numpad 5 | They work in every cursor and read at the active cursor [H1] | **Done for Say Line, Say Word and the word keys** (2026-10-19, B1): on a native control that is not a text field they read the control without its hint [503]. Say All in a dialog is still the page | P0 | 503 |
| 2 | Reading | Say All as a reading position: Insert+Down; Left and Right rewind and fast-forward; PageUp and PageDown set the rate; Ctrl stops | The cursor follows the reading and stays where it stopped [H1, H9, S1] | **Done but for the unit** (2026-10-19): the virtual cursor, or a text field's caret, follows the line being spoken and stays where reading stopped; Right and Left skip and rewind a line, PageUp and PageDown change the rate, a quick key moves and reading goes on. Live on old-pc (okur live-parity, live4), where Ctrl stopped it: 500/s2 a line short of JAWS's earlier trial, s5 between JAWS's two trials, s8 and s11 behind both (the voice is at SAPI's top rate, which PageUp cannot pass, and JAWS joins short lines into one utterance); 505/s2 matches. Say All by sentence or paragraph, and the Shift keys, are missing | P0 | 500, 505 |
| 3 | Selecting | Selecting in the virtual cursor: Shift+arrows, Ctrl+Shift+Left and Right, Shift+Home and End, Ctrl+Shift+Home and End, Shift+PageUp and PageDown, Ctrl+A, F8; then Ctrl+C | Selects and copies page text [H12, K] | **Done but for spelling** (2026-10-19, B3; F8 2026-10-20): Shift and Ctrl+Shift with the arrows, Home, End and the page keys, and Ctrl+A select as JAWS reads them; Ctrl+C copies the selection (and is the browser's without one) [501]. F8 selects the element at the cursor [515]; Spell Selected Text is missing; a large selection is said as JAWS does not yet ("selected 2035 characters up to", 562; the size where it starts from 617 and 620); Ctrl+A with nothing selected is not recorded | P0 | 501 |
| 4 | Web | JAWS Find: Ctrl+F, Insert+Ctrl+F, F3, Shift+F3 | A find dialog that searches from the virtual cursor [H4, P1, W1] | **Done** (2026-10-20, `2be518d`): Ctrl+F and Insert+Ctrl+F open Okur Find with JAWS's controls, history per site, and Find moving to the match (B2 and B4 fixed); F3 and Shift+F3 go to the next and previous match and say its line. 506 matches 9 of 9 steps and 633 11 of 11; JAWS's F3 right after its Find says nothing and lands on lines without the word, an accepted divergence (506/s8-s11, 633/s9-s14) | P0 | 506, 633 |
| 5 | Selecting | Say Selected Text in native edits: Insert+Shift+Down; twice quickly spells it | Reads or spells the selection [H2, W1] | **Done** (2026-10-19, B5): "Selection is" and the field's selection, or "Nothing selected" [503/s8, 504/s2]; twice quickly spells it, in a field and in the virtual cursor [501/s4, 504/s18] | P0 | 504 |
| 6 | Reading | Home, End, PageUp and PageDown in the virtual cursor | Bound to the virtual cursor; a page is 24 lines by default [K, S4] | **Done** (2026-10-19): Home and End stay in the virtual cursor (silent there, as in JAWS), PageUp and PageDown move it by a page of lines and read where it lands; every step of 502 matches | P0 | 502 |
| 7 | Applications | Word: arrows, Say All, misspellings, Insert+F7, Alt+Shift+L, Insert+Shift+R, Insert+F6, Ctrl+Insert+T | [P2] | **Not covered.** No Word module; not checked in Word | P0 | Office on old-pc is not known. Start with WordPad |
| 8 | Applications | Outlook: message list, opening a message, reading it, attachments (Insert+A), header keys (Alt+1-9) | [P3] | **Not covered** | P0 | Needs Outlook on old-pc |
| 9 | Reading | A multi-line native field on focus | The caret's line, then "Contains text" (316/s3, 400/s1, s5) | **Done** on 2026-10-19 (`6c4b81d`, with the task switcher and apostrophe fixes after it) | P0 | recorded |
| 10 | Reading | Prior and next word with the desktop layout: Insert+Left and Insert+Right | [H1, W1] | **Done** (2026-10-19): bound in the desktop layout | P1 | 503, 504 |
| 11 | Reading | The laptop layout's other keys: CapsLock+A, Shift+A, Y, N, H, Ctrl+U, O, I, Shift+J, L, Y, N, 5, Semicolon, P | [H17, W1] | **Partial** (2026-10-19): Say All, sentences, paragraphs, Say To and From Cursor, Say Selected Text and the top line are bound too [510]; the bottom line in an editor too; color and the JAWS and PC cursors are missing | P1 | 510 |
| 12 | Reading | Sentences and paragraphs: Alt+Numpad 5, Alt+Numpad Minus and Plus, Ctrl+Numpad 5; P and Shift+P | [H1, W1] | **Done** (2026-10-19) in pages and text fields; a text field's next and prior sentence move its caret (Win32 edits) [291, 510] | P1 | 291, 504 |
| 13 | Reading | Say To and From Cursor: Insert+Home, Insert+PageUp; twice quickly spells | [H1] | **Done** (2026-10-21): in text fields, spelled twice quickly [504/s9-s11], and with the virtual cursor with JAWS's link marks: 291 matches 15 of 15 steps and 504 23 of 23 | P1 | 291, 504 |
| 14 | Reading | Top and bottom line of the window: Insert+End, Insert+PageDown | The bottom line is usually the status bar [H2] | **Partial** (2026-10-19): the top line is the window's title [503/s10, 504/s23]; the bottom line is the status bar where the platform reads one (Notepad's), else an editor's last line [504/s22, 510/s13]; WordPad's status bar ("100%", 522/s20) is not collected yet, and other windows need the screen's text | P1 | 503, 504 |
| 15 | Help | Keyboard Help (Insert+1), hot key help (Insert+H), Windows keys help (Insert+W), screen-sensitive help (Insert+F1), the help layer (Insert+Space, J) | [H3, H8] | **Partial** (2026-10-19): Insert+F1, Insert+H and Insert+W open help in the Virtual Viewer as JAWS shows its own, with the hot keys link and the closing line; the words are Okur's own, Insert+H lists Okur's keys and Enter on one uses it; Escape reads the focus as JAWS does [526, 527]. Keyboard Help (Insert+1) is built (2026-10-20, `e42430d`): a key named as JAWS names it, then described in Okur's words, at more length when pressed twice quickly; JAWS waits for the second press before describing the first, Okur does not yet [634/s3]. The help layer is missing (628 was inconclusive on the Turkish layout) | P1 | 509, 634 |
| 16 | Speech | Speech on and off: Insert+Space then S; Shift+S for Speech On Demand | [H1, H8, K] | **Done** (2026-10-21): Insert+Space, S turns speech off and on; Insert+Space, Shift+S turns Speech On Demand on and off, where only the reading commands of its list speak (566, 707) | P1 | 517, 566, 707 |
| 17 | Speech | Speech History: Insert+Space then H | Results Viewer with the recent speech [H14] | **Done** (2026-10-20): Insert+Space, H opens the last 50 announcements in the Virtual Viewer, saying "Speech History"; Ctrl+H copies them and Shift+H clears them [570] | P1 | 517 |
| 18 | Speech | Voice rate on the fly: Alt+Ctrl+PageUp and PageDown, with Win to save | Says "faster" or "slower" [H9] | **Done but for Win to save** (2026-10-19): "Faster", "Slower", one step each, until Insert+Escape [512] | P1 | 512 |
| 19 | General | Quit (Insert+F4), refresh the screen (Insert+Escape) | [H4, H9] | **Done** (2026-10-20): Insert+Escape says "Screen updated", reads the page again and puts the rate back [298, 512]; Insert+F4 asks JAWS's question in Okur's name, Enter quits and Escape stays [571] | P1 | 298 (Insert+Escape) |
| 20 | Web | When a page loads: a summary, then reading starts by itself | Both on by default [S4, W2] | **Partial** (2026-10-19): a page loaded, or loaded again by Ctrl+R or F5, says its title, JAWS's summary ("Page has 5Regions, 5headings and 12links", "Page has 1heading and no links") and is read from the top; JAWS's Smart Glance sentence is not Okur's; not yet run live; no setting turns the reading off | P1 | 507 |
| 21 | Web | Forms mode: Numpad Plus leaves it; semi-auto mode; the quick key delay; Numpad Slash on a check box | [S5, W3] | **Partial.** Numpad Plus leaves forms mode (2026-10-19, B6 fixed; its speech checked by 507); Numpad Slash on a check box and a radio button with the virtual cursor matches 534's 10 steps (one an accepted divergence); semi-auto mode and the quick key delay are missing | P1 | 507, 534 |
| 22 | Editing | Cut, paste, undo, Ctrl+Delete, Shift+PageDown | Command names in [K]; what JAWS says is not known | **Done** (2026-10-21): Ctrl+X, Ctrl+V and Ctrl+Z say "cut selection to clipboard", "Pasted from clipboard" and "Undo" [504/s6, s7, s19, s20]; Ctrl+Backspace says the word it deleted, Ctrl+Delete and Home say nothing in a plain edit [664]; Shift+PageDown and Shift+PageUp say nothing [660/s1, s2]. 504 matches 23 of 23 steps, 664 13 of 13, 660 10 of 11 (its s8 records nothing: the WinForms text box typed a DEL character for Ctrl+Backspace) | P1 | 504, 660, 664 |
| 23 | Clipboard | Say the clipboard (Win+Insert+X), view it (Insert+Space, C), append to it (Insert+Win+C) | [H10, K] | **Done** (2026-10-22): Say Clipboard (Windows+Insert+X), View Clipboard Text (Insert+Space, C) and Append (Insert+Windows+C), with the Overwrite Clipboard question and the edges (825, 826); its short messages above Beginner (2026-10-23, `c80d313`) | P1 | 825, 826, 859-862 |
| 24 | Verbosity | Beginner, Intermediate and Advanced, and what each says; long and short messages | [S3] | **Partial** (2026-10-20): the settings dialog sets the level; each level's output types are JAWS's shipped Default.JCF's ([OutputModes]); above Beginner the short messages, and at Advanced what JAWS leaves out there (list positions, descriptions, group names, selection words), are built from 548-555 and 573-616; the hints on Tab in a page form and forms mode on Tab are built (`3c32b39`: once the virtual cursor has moved since the page loaded, 629-631), and match at Intermediate and Advanced too (644-649); Insert+T's short messages (`2bc4130`, 636-639); the web keys at both levels (round 259, `c24c07a`: heading levels, onMouseOver, Insert+Z, the page summary, Refresh Screen silent at Advanced) and Append's "Appended" (`c80d313`, round 261); the desktop's at both levels (843-854, 859-862) are being recorded again | P1 | 548-555, 629-649, 827-864 |
| 25 | Settings | Quick Settings for the application in front (Insert+V); settings per site | [H4]; [R] §9 | **Partial (B13).** The dialog saves for every application | P1 | [R] §9.5 |
| 26 | Reading settings | Say All unit, capitals during Say All, blank lines, and the rest of [R]'s options | [S1, R] | **Partial** (2026-10-21): Say All by line, sentence or paragraph is a setting (`say_all_unit`, 677-686); capitals during Say All, blank lines and the rest of [R]'s options are not yet | P1 | [R] §13, 677-686 |
| 27 | Echo | Typing echo choices in the dialog, key labels, key echo | [S2]; [R] item 3 | **Partial.** All 4 modes work by Insert+2; the dialog offers 2 | P2 | [R] item 3 |
| 28 | Cursors | JAWS cursor and mouse: Numpad Minus and Plus, Insert+Numpad Minus and Plus, Numpad Slash and Star, Alt+Delete, Insert+R | [H5] | **Partial** (2026-10-20): Numpad Minus and Plus, Insert+Numpad Minus (the mouse moves to the PC cursor) and Alt+Delete with the active cursor and its place [516, 513/s2; the coordinates need round 57's traces]; the JAWS cursor reads nothing (no screen model), Insert+Numpad Plus, Numpad Slash and Star as mouse buttons outside pages, and Insert+R are missing | P1 | 513, 516 |
| 29 | Windows | Window List (Insert+F10), system tray (Insert+F11) | [H2, H16] | **Partial** (2026-10-19): Insert+F10 opens Okur's Window List, JAWS's controls (Running Applications with each window's state, Only Include On Top Windows, Switch To, Cancel), and Switch To brings the window chosen to the front; live on old-pc (live7) it opened on the windows and Down moved through them. Its list is a list box as JAWS's (after live7); JAWS lists one window more, in another order; Delete to close a window is missing. Insert+F11 (2026-10-20, `d4b5da4`) opens Okur's Select a System Tray Icon with JAWS's controls (the icons, said by the dialog's name as JAWS does [511/s4]; Right Single Click, the default, Left Single Click, Left Double Click, Cancel) and clicks the icon chosen; its list (the taskbar's TrayNotifyWnd buttons, left to right) is not yet checked on a machine (`okur tray`, round 62) | P1 | 511 |
| 30 | Web | Virtual cursor off and on, routing: Insert+Z, Insert+Delete, Ctrl+Insert+Delete | [H12, S6] | **Done** (2026-10-19): Insert+Z turns the virtual cursor off and on [298]; Insert+Delete and Ctrl+Insert+Delete route the cursors [536] | P1 | 298 |
| 31 | Web | The remaining quick keys and first and last keys | [`jaws-quick-navigation-keys.md`](../references/jaws-quick-navigation-keys.md) | **Partial** (2026-10-20): the first and last heading, heading of a level and form field (Alt+Insert+Home and End, Ctrl+Alt+Insert and a level, Ctrl+Insert+Home and End) [295, 296], separators (Minus), divisions (Z) and mail links (Backslash) [292, 293], a table's rows and columns (Windows+Alt) [297], an element's start and end (Shift+Comma, Shift+Period, Alt+Windows+Home and End) [294]; S and D by element type, flow to (Equals) and Jump to Line (J, Shift+J) (2026-10-20); clickable elements (Slash) wait for a page source that exposes click handlers, and words from a list (the Word List) for 560's next recording [290, 293, 299, 559, 560] | P1/P2 | 289-299 |
| 32 | Web tables | Rows and columns; Alt+Ctrl+Numpad 5; the table layer | [H7, H8, P1] | **Partial** (2026-10-20): cells and headers; the table layer (Insert+Space, T: the arrows by cell, Numpad5 the cell, Ctrl+Home and End, Ctrl and an arrow to an edge, Shift+Up the row, Shift+Numpad5 the column, Escape) [535]; rows and columns read whole with Windows+Alt and an arrow, Numpad5, Comma or Period [297]; Ctrl+Alt+Numpad 5 says the cell (B8) | P1 | 297 |
| 33 | Notifications | Toasts, notification history, progress bars, tool tips | [H15, S2, S3] | **Partial.** Toasts said as their banner shows and kept in the Notification History, with its rules (770, 773-775). A toast Focus Assist hides is neither said nor kept by JAWS: in 533 the two trials whose banner did not show added nothing to the history (26 items, its newest from an earlier run), the one whose banner showed said it and added it (27) (re-read 2026-10-23, `docs/references/notifications-research.md`); Okur does the same. Progress bars and tool tips remain | P1 | to write |
| 34 | Applications | Excel | [P4] | **Not covered** | P1 | needs Excel on old-pc |
| 35 | Dialogs | Default button (Insert+E), control's hot key (Shift+Numpad 5), list view columns (Ctrl+Insert+1-0) | [H6] | **Done** (2026-10-20): Insert+E says a dialog's default button wherever the focus is (DM_GETDEFID), and "Not in a dialog box" outside one; Shift+Numpad 5 says the control's hot key; Ctrl+Insert with a number a list view's column [513, 514, 532, 537, 540-544]. Insert+B and Insert+Tab are built | P1/P2 | 513, 514, 532 |
| 36 | Applications | Office ribbons | [S6] | **Partial** (2026-10-23): WordPad's ribbon (Windows' ribbon framework, NetUI as Office's) as JAWS reads it from its MSAA objects: Alt and F10 to the selected tab ("Upper Ribbon | ... | Ribbon tabs Tab | Home Tab selected | 1 of 2"), the lower ribbon's groups ("Lower Ribbon | Home | Ribbon tab | Clipboard Tool bar | ..."), a command's place and state, "Leaving menus" and "Leaving ribbons" (523, 970-973). Not yet: the font gallery's list, key tips, Office itself | P1 | WordPad |
| 37 | Voice | Voice profiles, contexts, pitch; punctuation per context | [R] items 1, 8.8 | **Missing** | P2 | [R] |
| 38 | Information | Color (Insert+5), battery (Insert+Shift+B), version (Ctrl+Insert+V), configuration (Insert+Q) | [H1, H2, H4] | **Partial** (2026-10-19): Insert+Q says "Default settings are loaded" and the application, spelled [513/s5]; Ctrl+Insert+V says the application's product and version (2026-10-20) [513/s4, replayed once traced again]; Insert+5 (Say Color) says a page's text and background colors by JAWS's names, "blue2 on white", "default on default" off the text (2026-10-20, `af4fb10`, `3cf36df`): X11's names as JAWS says them with JAWS's own entries, the nearest by the redmean distance, every step of 565, 618 and 619 matching; outside pages it says nothing yet (JAWS reads the screen there); battery is not recorded | P2 | 513, 530, 565, 618, 619 |
| 39 | Screen | Screen echo, key repeat, typing interrupt | [S2, H4] | **Partial** (2026-10-23): Screen Echo (Insert+S) reads a console's new output (962-964); Reading Interrupt is a setting (982, 983). Typing Interrupt and key repeat not recorded so that a trial settles | P2 | [R] item 3 |
| 40 | Reading | Skim reading, Word Index | [H13] | **Done** (2026-10-21, 2026-10-22): skim reading by first line, first sentence, text rules and attributes, the Skim Reading dialog and its summary; the Word List (Ctrl+Insert+W) | P2 | 760-818, 560, 764-766, 808 |
| 41 | Web | Element information, link source, F8, frames list, visited links, live-region filter, mouse over | [P1, W1] | **Partial** (2026-10-19): the link's address (Ctrl+Insert+F7), routing (Insert+Delete, Ctrl+Insert+Delete) and the frames list's "No frames found in page" (Insert+F9) [536]; element information, visited links, the live-region filter and mouse over are missing | P2 | 536 |
| 42 | Web | PlaceMarkers, custom labels, Flexible Web, Smart Navigation | [P1, S4, W2] | **Partial** (2026-10-21): PlaceMarkers, custom labels and Flexible Web are built (the README lists them); Smart Navigation's toggle (Insert+X) is built (913, 2026-10-23), its units of the character and word keys are not | P2 | 691, 699, 705-742, 913 |
| 43 | Other | Audio ducking, keyboard lock, schemes, dictionary, language switching | [H4, H8] | **Missing** | P2 | — |

**Later, not in this list:**
- OCR and Picture Smart;
- braille, and the Braille and Text Viewers;
- the Touch Cursor;
- Tandem and FSCompanion;
- Research It, Face in View and the Voice Assistant;
- Screen Shade;
- the Math Viewer and Text Analyzer;
- Graphics Labeler, Frame Viewer and Prompt Creator.

## Where Okur contradicts JAWS's documentation

- **B1.** Insert+Up, Insert+Down, Numpad 5 and Insert+Numpad 5 on a native control that is not
  a text field read the virtual buffer of the last page. In
  `engine/commands.rs`'s `execute`, only the desktop and taskbar modes are excluded.
- **B2.** Insert+Ctrl+F opens the form fields list. [K] binds it to JAWS Find, and [H4] and
  [W1] agree.
- **B3.** In the virtual cursor, Ctrl+Shift+Left and Right and Ctrl+Shift+Home and End move
  without selecting: Ctrl+Right's binding matches them. Ctrl+C never copies Okur's selection,
  because no action sets a selection in the page.
- **B4.** Ctrl+F is taken without opening a dialog, which also blocks Chrome's own find.
- **B5.** Insert+Shift+Down in a native edit reads the page's selection.
- **B6.** Numpad Plus in forms mode types "+"; [S5] and [W3] say it leaves forms mode.
- **B7.** Alt+Shift+Down is bound to Say Selected Text; in [K] it is a JAWS cursor key.
- **B8.** Alt+Ctrl+Numpad 5, the documented Say Current Cell [H7], is not bound. Okur binds
  Ctrl+Alt with the number row's 5.
- **B9.** Okur's Ctrl+Alt+Shift+H and R (column and row headers) are in neither key map.
  JAWS documents Alt+Insert+Shift+C and R in Word and Excel [P2, P4].
- **B10.** Home, End, PageUp and PageDown go to the browser in the virtual cursor (row 6).
- **B11.** Say All does not leave the cursor where it stopped and has no keys during it
  (row 2).
- **B12.** A page load neither gives a summary nor starts reading (row 20).
- **B13.** Quick Settings saves for every application; JAWS's acts on the one in front
  [H4].

## Where the sources disagree

- **Insert+F3.** [P1] and [W1] list it both as Virtual HTML Features and as Find Next. [K]
  settles it by context: Find Next in Common Keys, Virtual HTML Features in the virtual
  cursor.
- **A table's row and column.** [H7], [P1] and [P2] give Alt+Win+Comma and Alt+Win+Period.
  [W1] gives them without Alt. The key map has Alt.
- **Phonetic reading after Say Character twice.** [H1] says Left and Right go on spelling
  phonetically until another key; [R] item 4 found the teacher lesson and Settings Center
  disagree. Old-pc's 402 shows plain "A" on the next Right. The cause may be the harness's
  history copy after each step.
- **Insert+Space, S.** [H1]'s table cycles three speech modes; its notes and [K] give mute,
  and Shift+S for Speech On Demand.
- **Speech history size.** [S3] says 50 items, [H14] says up to 500.
- **The copy message.** [S3]'s long form is "Copied selected text to clipboard"; old-pc says
  "Copied selection to clipboard" (311, 313).
- **Forms mode's default.** [S5] says Auto. Old-pc runs Manual, as most of the web evidence
  does.
- **Laptop Hot Key Help.** [W1] gives CapsLock+Ctrl+Shift+H, since CapsLock+H is Say
  Sentence in the laptop layout [H17]; [H3] lists only Insert+H.

## Not known yet

Each needs a recording before anything is built:
- what JAWS says for Ctrl+X, Ctrl+V and Ctrl+Z (504);
- what it says for Home and End in a page (502);
- the wording of the page-load summary (507);
- whether Office is installed on old-pc;
- how Okur handles toasts.

## Round 12 on old-pc

| Scenario | Records | Fixture |
|---|---|---|
| 500 | Say All stopped by Ctrl, moved by Right and Left, rate keys, H during it, and Insert+Up after each | `news-article.html` |
| 501 | Selecting in the virtual cursor, Say and Spell Selected Text, Ctrl+C and Ctrl+A with the clipboard captured | `doc-reading.html` |
| 502 | Home, End, PageDown and PageUp in the virtual cursor | `news-article.html` |
| 503 | The reading keys on a check box and in a list box, Insert+Right and Left, the window's top and bottom line, Say All in a dialog | `desktop-forms.ps1` |
| 504 | A native edit: Say and Spell Selected Text, copy, paste, cut, undo, Ctrl+Delete, Say To and From Cursor, sentence, paragraph, top and bottom line, Win+Insert+X | `desktop-editor.ps1` |
| 505 | Say All in a native edit stopped by Ctrl and moved by Right and Left | `desktop-reading.ps1` |
| 506 | JAWS Find: Ctrl+F, a word typed in its dialog, Enter, F3, Shift+F3, Insert+Ctrl+F | `news-article.html` |
| 507 | A page load stopped by Ctrl after 7 s, then Numpad Plus in forms mode | `signup-form.html` |

Keys pressed while JAWS still speaks use the new step field `during` in `run-scenario.ps1`;
`okur parity` replays them at their times.

## Order of work

1. B1, B5 and the native reading keys (rows 1, 5, 10, 13, 14), once 503 and 504 are
   recorded.
2. Say All as a reading position (row 2), once 500 and 505 are recorded. Speech must report
   which utterance it reached, so the cursor can follow it. This is platform work in
   `okur-platform`'s speech thread.
3. Selecting and copying in the virtual cursor (row 3, 501). Ctrl+C needs an action that sets
   the page's selection (IA2 `setSelection`, UIA TextRange `Select`), or Okur puts the text on
   the clipboard itself.
4. Home, End and the page keys (row 6, 502). Then JAWS Find (row 4, 506) with its dialog in
   okur-ui, and Ctrl+F handed back to Chrome until that dialog exists.
5. Sentences, paragraphs and the virtual keys queued in rounds 9-11.
6. Page load (row 20, 507), Numpad Plus (B6), Keyboard Help and the Insert+Space layer.
7. Word and Outlook: first find out what old-pc has.
