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

P0 blocks daily use, P1 is expected by a JAWS user within days, P2 is later. Closed gaps, and
the contradictions closed with them, were removed on 2026-10-24 and are in git history; the
open ones keep their numbers.

| # | Area | Feature and JAWS keys | JAWS (source) | Okur | P | Recording |
|---|---|---|---|---|---|---|
| 1 | Reading | Reading keys on native controls: Insert+Up, Insert+Down, Numpad 5, Insert+Numpad 5 | They work in every cursor and read at the active cursor [H1] | **Done but for Say All** (2026-10-19, B1): Say All in a dialog is still the page | P0 | 503 |
| 2 | Reading | Say All as a reading position: Insert+Down; Left and Right rewind and fast-forward; PageUp and PageDown set the rate; Ctrl stops | The cursor follows the reading and stays where it stopped [H1, H9, S1] | **Done but for the Shift keys** (2026-10-19). Live on old-pc (okur live-parity, live4), where Ctrl stopped it: 500/s2 a line short of JAWS's earlier trial, s5 between JAWS's two trials, s8 and s11 behind both (the voice is at SAPI's top rate, which PageUp cannot pass, and JAWS joins short lines into one utterance); 505/s2 matches | P0 | 500, 505 |
| 7 | Applications | Word: arrows, Say All, misspellings, Insert+F7, Alt+Shift+L, Insert+Shift+R, Insert+F6, Ctrl+Insert+T | [P2] | **Done** but for the Spelling pane's suggestion said twice (1069/s4) and Insert+Alt+R, which JAWS let through to Word (2026-10-24: lines, Say All with its structure, tables and their titles, the Heading List, Alt+Shift+L, Say Font, the views; 2026-10-25: Web Layout's and Read Mode's names, the ribbon; to 2026-10-29: Insert+F7, List Fields, form fields, Say Font's line spacing from Word's object model, Read Mode's warning, comments, footnotes, endnotes and bookmarks; 2026-10-31: the Virtual Viewer of notes and revisions, Insert+Shift+R and Ctrl+Insert+R, deleted text unread, pages and sections, Insert+Delete, the status bar) | P0 | 518, 519, 1034-1113 |
| 8 | Applications | Outlook: message list, opening a message, reading it, attachments (Insert+A), header keys (Alt+1-9) | [P3] | **Not covered** | P0 | Needs Outlook on old-pc |
| 11 | Reading | The laptop layout's other keys: CapsLock+A, Shift+A, Y, N, H, Ctrl+U, O, I, Shift+J, L, Y, N, 5, Semicolon, P | [H17, W1] | **Partial** (2026-10-19): the JAWS and PC cursors are missing | P1 | 510 |
| 14 | Reading | Top and bottom line of the window: Insert+End, Insert+PageDown | The bottom line is usually the status bar [H2] | **Partial** (2026-10-31): Word's status bar read as JAWS reads it (1113/s9); WordPad's ("100%", 522/s20) is not collected yet, and other windows need the screen's text | P1 | 503, 504, 1113 |
| 15 | Help | Keyboard Help (Insert+1), hot key help (Insert+H), Windows keys help (Insert+W), screen-sensitive help (Insert+F1), the help layer (Insert+Space, J) | [H3, H8] | **Partial** (2026-10-19): the help layer is missing (628 was inconclusive on the Turkish layout) | P1 | 509, 634 |
| 18 | Speech | Voice rate on the fly: Alt+Ctrl+PageUp and PageDown, with Win to save | Says "faster" or "slower" [H9] | **Done but for Win to save** (2026-10-19) | P1 | 512 |
| 20 | Web | When a page loads: a summary, then reading starts by itself | Both on by default [S4, W2] | **Partial** (2026-10-19): JAWS's Smart Glance sentence is not Okur's; not yet run live | P1 | 507 |
| 21 | Web | Forms mode: Numpad Plus leaves it; semi-auto mode; the quick key delay; Numpad Slash on a check box | [S5, W3] | **Partial.** The quick key delay is missing | P1 | 507, 534 |
| 26 | Reading settings | Say All unit, capitals during Say All, blank lines, and the rest of [R]'s options | [S1, R] | **Partial** (2026-10-21): capitals during Say All, blank lines and the rest of [R]'s options are not yet | P1 | [R] §13, 677-686 |
| 27 | Echo | Typing echo choices in the dialog, key labels, key echo | [S2]; [R] item 3 | **Partial.** Key labels and key echo are missing | P2 | [R] item 3 |
| 29 | Windows | Window List (Insert+F10), system tray (Insert+F11) | [H2, H16] | **Partial** (2026-10-19): Okur's Window List: JAWS lists one window more, in another order; Delete to close a window is missing | P1 | 511 |
| 31 | Web | The remaining quick keys and first and last keys | [`jaws-quick-navigation-keys.md`](../references/jaws-quick-navigation-keys.md) | **Partial** (2026-10-20): clickable elements (Slash) wait for a page source that exposes click handlers [290, 293, 299, 559, 560] | P1/P2 | 289-299 |
| 33 | Notifications | Toasts, notification history, progress bars, tool tips | [H15, S2, S3] | **Partial.** Progress bars and tool tips remain | P1 | to write |
| 34 | Applications | Excel | [P4] | **Partial** (2026-10-31): a cell by its value and coordinates, Insert+C, Ctrl+Home, Ctrl+End and the Ctrl arrows, Say Line and the row and column readers, the lists of cells with formulas, of a column's cells and of worksheets, from Excel's object model. Not yet: selections (1117), the sheet tabs' menu (Insert+Shift+S), editing a cell | P1 | 521, 1114-1117 |
| 36 | Applications | Office ribbons | [S6] | **Partial** (2026-10-29). Word's ribbon as JAWS reads it (its tabs, the lower ribbon as a menu, key tips silent, leaving for the document), its check boxes, the Styles gallery and the Font and Size boxes' lists. Not yet: its other galleries, Excel's and Outlook's | P1 | WordPad, 1042, 1043, 1046, 1064, 1066, 1067 |
| 37 | Voice | Voice profiles, contexts, pitch; punctuation per context | [R] items 1, 8.8 | **Done but for** tutor hints in the message voice and a Say All's parts at the context's pitch (2026-10-24: Voice Adjustment, Select a Voice Profile) | P2 | the walks of the settings redesign |
| 38 | Information | Color (Insert+5), battery (Insert+Shift+B), version (Ctrl+Insert+V), configuration (Insert+Q) | [H1, H2, H4] | **Partial** (2026-10-19): battery is not recorded | P2 | 513, 530, 565, 618, 619 |
| 39 | Screen | Screen echo, key repeat, typing interrupt | [S2, H4] | **Partial** (2026-10-23): Typing Interrupt and key repeat not recorded so that a trial settles | P2 | [R] item 3 |
| 41 | Web | Element information, link source, F8, frames list, visited links, live-region filter, mouse over | [P1, W1] | **Partial** (2026-10-19): element information, the live-region filter and mouse over are missing | P2 | 536 |
| 42 | Web | PlaceMarkers, custom labels, Flexible Web, Smart Navigation | [P1, S4, W2] | **Partial** (2026-10-21): Smart Navigation's units of the character and word keys are not built | P2 | 691, 699, 705-742, 913 |
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

- **B1.** Insert+Down on a native control that is not a text field reads the virtual buffer
  of the last page.
- **B9.** Okur's Ctrl+Alt+Shift+H and R (column and row headers) are in neither key map.
  JAWS documents Alt+Insert+Shift+C and R in Word and Excel [P2, P4].

## Where the sources disagree

- **Phonetic reading after Say Character twice.** [H1] says Left and Right go on spelling
  phonetically until another key; [R] item 4 found the teacher lesson and Settings Center
  disagree. Old-pc's 402 shows plain "A" on the next Right. The cause may be the harness's
  history copy after each step.
- **Laptop Hot Key Help.** [W1] gives CapsLock+Ctrl+Shift+H, since CapsLock+H is Say
  Sentence in the laptop layout [H17]; [H3] lists only Insert+H.

## Order of work

1. B1 and the native reading keys (rows 1, 14).
2. Say All as a reading position (row 2).
3. The virtual keys queued in rounds 9-11.
4. Page load (row 20, 507) and the help layer.
5. Word and Outlook.
