# Parity gap audit: where Okur and JAWS differ, and why (2026-10-18)

This audit looks at every step where Okur's speech differs from JAWS's recorded speech, finds
the cause in the code and the evidence, and says what each difference costs a user outside the
corpus. It ends with a proposal in order of real-world impact.

What was audited:
- Branch `helper-dialogs-round-2` at commit `4495382`, with the binary built from it
  (`target/audit/okur.exe`).
- Read-only work: this file is the only change. Nothing was built, and neither old-pc nor the dev
  box's JAWS was touched.
- The branch moved on during the audit (`b2928d9`, `0d4da00`: the Heading List as a real
  dialog). Line numbers below are those of `4495382`. Only `engine/commands.rs`, `engine.rs`,
  `ui/` and the catalog changed since, so `commands.rs` lines are given at `4495382`.

Two words are used with care:
- **Verified** means a command's output shows it. The command is named.
- **Inferred** means it follows from reading the code or the evidence, but no tool run shows it.

Commands below are written as `okur ...` for `target/audit/okur.exe ...`.

## Executive summary

### The numbers

```
$ okur-lab parity --all
JAWS spoken parity: 465/647 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 1123/1220 steps match exactly (0 scenarios could not run)

$ okur-lab parity --all --api uia
JAWS spoken parity: 426/647 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Uia): 1007/1220 steps match exactly (0 scenarios could not run)
```

- History layer (what JAWS keeps in its speech history): 97 steps differ.
- Spoken layer (what the synthesizer is given): 182 steps differ. 81 of them also differ in the
  history layer; 101 differ only in what is spoken.
- UIA replay: 213 steps differ. 116 of them match through IA2; none differs through IA2 only.
- The committed baseline lists the same 97 mismatches (its mismatch keys at `4495382`, compared
  with `diff` against the DIFF list: no difference).

### The history-layer gap by cause

Of the 97 steps:
- **48 are replay or evidence artifacts.** 41 replay JAWS's own dialogs (Links List, Heading
  List, Select a Form Field, Virtual HTML Features; 222-226), which our tracer sees only thinly
  because JAWS runs elevated. The other 7: a Say All read after 800 ms, words JAWS kept in its
  history but did not speak (3), trials that disagree (1), and a window view replay cannot read
  (2).
- **17 are JAWS quirks or unexplained JAWS behavior.** In 15 of them JAWS's answer is wrong or
  poorer and Okur's is kept: a font JAWS cannot find, an emoji JAWS's viewer drops, JAWS's wrong
  selection text, a list view JAWS shows as its last row only. 2 are unexplained.
- **22 are different handling in Okur.** In 12 of them the code follows an observation of
  JAWS on the dev box that old-pc's evidence contradicts.
- **10 are features Okur lacks.**

So 32 steps are real differences in Okur. 15 more are JAWS quirks Okur should not copy; that
decision should be written down, not left in the mismatch count.

### The spoken-layer gap by cause

Of the 101 steps that differ only in what is spoken (one step can have several causes):
- **59 involve symbol and punctuation names** in running text ("Save dot dot dot", "left paren",
  "colon", "dash", "bullet"). For 42 that is the only cause.
- **46 were cut short by the harness**: the recorder's text was read at the first pause of
  500 ms, and JAWS's next part (usually its tutor message or the position, "1 of 3") came after.
  For 30 that is the only cause.
- **13 involve JAWS splitting words** ("MenuBar" spoken "Menu Bar", "System32" as "System 32",
  "spacebar" as "space Bar").
- **2** attribute speech to the wrong step.

### The UIA gap by cause

Of the 116 steps that differ only through UIA, 74 come from one cause: Chrome's UIA tree shows a
block quote only through its localized type name, so the UIA buffer has no "block quote" lines
and every later line of the article read-down and read-up scenarios is off by one or two. This
matters live: Okur reads a new page through UIA first and replaces it with the IA2 read when that
arrives.

### The five most important findings

1. **Most of the history-layer gap is measurement, not Okur.** 48 of 97 steps are artifacts,
   41 of them from replaying JAWS's own elevated dialogs. The real gaps are 32 steps; the
   spoken layer and a live run are what measure the rest.
2. **Two differences cost a user real time every day.**
   - A multi-line text field in a desktop application is read whole each time it takes the focus
     (316/s3). JAWS reads one line and says "Contains text".
   - In web applications, when the focus lands on an element the buffer has no line for (a
     message row that gets its label only when focused), Okur leaves the virtual cursor on the
     previously focused control and repeats the page title on the next focus (219). After
     Shift+Tab to a message, Down reads a button at the top of the page. The user is lost.
3. **Okur does no punctuation processing for running text.** The synthesizer gets the text as
   it is, and the voice decides whether "( ) { } : - ." are heard. JAWS at punctuation Most names
   them. This is the largest cause in the spoken layer (59 steps) and the largest daily-use gap
   for code, command lines, paths and web addresses. Punctuation levels are planned for the
   extension SDK's milestone M5.
4. **Seven rules in the code come from JAWS on the dev box (Windows 11, the owner's settings)
   and old-pc's evidence contradicts them.** They explain 12 of the 22 "different handling"
   steps: Say Window, Say Focus, the Notepad window view, the desktop list, a page's focusable
   elements, a page's popup menu items, and the page after leaving Chrome's menu.
5. **Measurement integrity has gaps.**
   - The spoken layer's 500 ms cut (46 steps) is a harness flaw, not JAWS behavior.
   - Seven steps of 313 changed from match to mismatch on 2026-10-18 with a re-recorded trace,
     and no handoff mentions it.
   - AGENTS.md rule 6 asks for `known gap` tests; there are none, and one test asserts the speech
     JAWS does not give (scenario 12).
   - The one live comparison of Okur's own dialog (222, "5/11 then 10/11") is quoted in a commit
     message, but its run file is not kept, so it cannot be checked again.

### The top of the proposal

1. A multi-line native field on focus: its line and "Contains text" (hours; evidence exists).
2. Web applications: place the virtual cursor inside a focused element that has no line of its
   own, and stop repeating the page title (1-2 days; one new recording).
3. The harness: read the speech recorder's text up to the next step's key, then re-record the
   affected scenarios (hours, one old-pc batch).
4. Punctuation: JAWS's levels and its symbol names for running text, starting with Most in
   English, which 59 recorded steps already measure (3-5 days; new recordings for the other
   levels).
5. Dialogs: the page name on Ctrl+Tab, the tab row in the window view, Say Window's format, the
   access key on Insert+Tab, a list item after the viewer closes (1-2 days; one recording).

The full list is in section 8.

## 1. How the numbers were taken

Commands run, each writing to a scratch file:
- `okur-lab parity --all` (IA2, the ratchet's API), `okur-lab parity --all --spoken`, and
  `okur-lab parity --all --api uia`.
- `okur-lab evidence <scenario> --trace` for every scenario with a DIFF (and `--trial N` for 204).
- `okur-lab lines` on the snapshots of `news-article`, `characters`, `web-chat`, `signup-form` and
  `web-app-patterns`, through IA2 and UIA.
- `git show` of earlier baselines, traces and the code at `4495382`.

DIFF counts per scenario in the history layer (from `okur-lab parity --all`, counted with `grep` and
`uniq -c`):

```
11 225-web-form-fields-list    11 224-web-headings-list    11 222-web-links-list
 8 323-desktop-property-sheet   8 313-desktop-say-font      8 219-web-chat-patterns
 6 322-desktop-forms-viewer     5 226-web-element-lists     5 220-web-chrome-own-controls
 4 312-desktop-virtualize-control  3 314-desktop-notepad-font-and-viewer
 3 223-web-links-list-escape    2 316-desktop-show-desktop  2 209-web-title-and-font
 1 each: 319, 315, 305, 218, 207, 204, 12, 111, 104, 01
```

Each DIFF was grouped by cause, not by scenario. Every step id is in the appendices with its
group and class.

How replay works matters for reading the groups (`crates/okur-corpus/src/parity.rs`):
- A page scenario loads the IA2 snapshot of its fixture, which was captured in a separate
  Chrome session. A desktop scenario starts with no document.
- Each step presses the key, then feeds the engine the accessibility events traced in that
  step of the first traced trial (lines 299-367).
- Okur's spoken layer is its history text with the role marks removed
  (`okur_core::pronounce::text`, parity.rs:527-530).
- Replay cannot re-read the page, read a window's screen, or see JAWS's own dialogs except as
  the tracer saw them.

## 2. The history layer, group by group

The groups are in order of their effect on a user, not their size.

### 2.1 A multi-line text field is read whole when it takes the focus (316/s3, 1 step)

What JAWS says on Alt+Tab back to the editor fixture: "SR Editor Fixture | Code ‹ edit › |
(the field's first line, the fixture's row of ASCII punctuation) | Contains text | Type in
text." That is the field's name, "edit" in lower case, one line, "Contains text", and the
hint.

What Okur says: the name, "Edit", all three lines of the field, and the hint.

Root cause: `crates/okur-core/src/native_focus.rs:179-203`. An edit's focus speech is its name,
the role, its whole value (line 190) and the hint. Only a native document (line 166, Windows 11
Notepad) gets "Contains text". The focus event's value is the field's whole text.

- Class: different handling. No decision about multi-line fields is recorded.
- Verified: `okur-lab parity --all` (316/s3); `okur-lab evidence 316-desktop-show-desktop --trace`.
- Inferred: that JAWS reads the caret's line, not always the first one. The caret was at the
  start in this recording.

Effect beyond the corpus: every multi-line field in a desktop application (a notes field, a mail
body in a desktop client, a log window, a WinForms editor) is read in full each time the user
tabs into it or switches back to its window. With a long text the user has to stop the speech
every time, and the quick "where am I" answer is lost. It does not block, but it is frequent.

Fix: a field whose text holds line breaks says its name, the lower-case "edit"
(`role_edit_multiline`), the caret's line, "Contains text" on arrival, and the hint. The signal
is already used for the window view (`text_is_the_window`, `commands.rs:810-820`).

### 2.2 Web applications: a focus the buffer has no line for (219, 8 steps)

The fixture copies WhatsApp Web's patterns: messages that take the focus and get a label only
while they have it, and a popup menu that appears on demand.

What JAWS says (old-pc, Auto Forms Mode off):
- s8, Shift+Tab onto the first message: "‹ MainRegion › | You:". JAWS moves the virtual cursor
  into the message and reads the line there. A second trial variant read the line before it.
- s9, Down: "Good morning", the message's next line.
- s11, after Insert+3 and Down (the page moves the focus to the next message): "See you at noon".
- s14-s16, the message's menu: "Web Chat - Google Chrome ‹ dialog › | Message info ‹ menu › |
  To move through items press up or down arrow. | Reactions menu | (the reaction buttons)", then
  "Message info ‹ menu ›" and "Reply ‹ menu ›".
- s17, Escape: "Leaving menus | ‹ MainRegion › | See you at noon".

What Okur says:
- s8: the label the page sets on focus, "You Good morning 09:00 Read Press the right or left
  arrow key for more options".
- s9: "Status ‹ Button ›", a button in the banner at the top of the page.
- s11, s13 and s17: the page title "Web Chat" first, as if the page had just been entered.
- s14, s15: "Message info 1 of 5" (a position instead of JAWS's role word). s16: "banner end".

Root causes:
1. **The cursor stays on the previous focus.** The message has no name in the snapshot (the
   page sets it on focus) and no line of its own (its text is in child lines), so
   `page_node_for` finds no node (`engine/page.rs:265-299`: by node id, IA2 unique id, DOM id,
   then name and role). `engine/focus.rs:415-416` has already put the cursor on the line of the
   previous focus (the "Chats" toggle focused in s1), and nothing moves it on. Down reads from
   there.
2. **The page title repeats.** `focus.rs:378` sets `last_page_focus` to the node found, here
   none. The next focus then counts as entering the page (`focus.rs:372`) and gets the title.
3. **A plain element is said by its name.** When the node is not found, the event's name is said
   (`focus.rs:436-449`); when it is found, the plain-element rule says the same
   (`focus.rs:450-469`). That rule cites JAWS on the dev box with WhatsApp (2026-10-17). Old-pc's
   JAWS reads the virtual cursor's line instead.
4. **The popup menu.** It is hidden when the snapshot is captured, so it is not in the buffer
   (verified with `okur-lab lines corpus/snapshots/web-chat.chrome.ia2.json`: no menu item lines).
   Without Auto Forms Mode a page's menu keys stay the virtual cursor's (`focus.rs:104-109`), so
   Down reads the buffer from where the cursor was left. The item wording with a position comes
   from the dev box ("Cevapla 2 of 9", `engine/web_app_tests.rs:304-306`).

- Class: different handling. Causes 1 and 2 are defects. Causes 3 and 4 are dev-box rules that
  old-pc contradicts. Part of it is a replay artifact: live Okur reads a changed part of the page
  again when a focus is not in its buffer (`crates/okur-platform/src/events/windows.rs:316-330`),
  which replay cannot do.
- Verified: parity output; `okur-lab evidence 219-web-chat-patterns --trace`; `okur-lab lines` on the
  snapshot; the fixture (labels set on focus, `corpus/fixtures/web-chat.html:76-81`).
- Inferred: that live Okur leaves the cursor in the same wrong place before the re-read
  arrives. A live run (`run-scenario.ps1 -Reader okur` on 219, then `okur-lab live-parity`) measures
  it.

Effect beyond the corpus: WhatsApp Web, Slack, Teams, Gmail and Discord all use focusable rows
and labels set on focus. After Shift+Tab, or when the page moves the focus, Down reads from
somewhere else on the page. It is like reading a printed page with a finger under the line and
having someone move the finger back to the top. The repeated page title is noise on every such
focus.

Fix: when the focused node has no line, put the virtual cursor on the first line inside it; when
no node is found, keep the previous `last_page_focus` instead of none. Decide the plain-element
and menu-item rules per setting, after recording 219 with Auto Forms Mode on and off.

### 2.3 Dialogs: a property sheet and a WinForms window (323: 8 steps; 322: 6 steps)

Dialogs are where most desktop settings live: file properties, Internet Options, an installer's
pages, Office's options.

**The page after Ctrl+Tab (323/s10, s11).** JAWS: "Security Page | For special permissions or
advanced settings, click Advanced. | Group or user names: ‹ List view › | SYSTEM | 1 of 3 | ...",
and on the way back "General Page | Read-only ...". Okur says only the focused control.
- Root: the page name is said only when the dialog is entered (`engine/focus.rs:653-694`,
  `window_heading`, page at line 678). A page change inside the same dialog, which the trace
  shows as a selection on the tab control, is not announced.
- Class: unimplemented.
- Effect: after Ctrl+Tab the user does not hear which page they reached. They can guess from
  the control, often wrongly.

**The tab row in the window view (323/s4-s7).** JAWS's Virtualize Window shows "General Security
Details Previous Versions" as its second line. Okur's view has no tab row, so the next three
steps are one line off.
- Root: `crates/okur-platform/src/screen_lines.rs:377-455` (`read_layout`) builds a dialog's rows
  from each child window's Win32 text. A tab control has no window text; its tabs are items. The
  tracer uses the same function, so the replayed view is what live Okur shows (inferred).
- The same function keeps the left-to-right marks Windows puts around the dates (U+200E, visible
  in Okur's 323/s2 and s7 text). JAWS's text has none. The marks are silent, but they stay in
  text copied from the viewer and would show in braille. The UIA window view strips them
  (`viewer_text.rs:79-81`); this path does not.
- Class: unimplemented (tabs); different handling (marks).

**Say Window, Insert+B (323/s2).** JAWS: role words in their marks ("‹ Edit ›", "‹ read only
edit ›", "‹ Button ›"), each label once (twice for "Opens with:", as it is drawn), and it stops
after the page's last control, "Advanced... ‹ Button ›". Okur: lower-case type words without
marks, no "read only", most labels two or three times ("Type of file: Type of file: edit Text
Document (.txt) Type of file:"), then OK, Cancel, Apply and the four tab names.
- Root: `native_focus.rs:537-583` (`say_window`). Its comment takes the format from JAWS on the
  dev box's Windows 11 Run dialog, repeated labels and lower-case words included.
- Class: different handling (a dev-box rule).
- Effect: Insert+B is how a user reads a whole dialog, such as an error or a confirmation.
  Okur's version is longer and drops "read only", the words that tell a user a field cannot be
  typed in. A user can be misled.

**Say Focus, Insert+Tab (323/s1).** JAWS adds "alt+R", the check box's access key, although
old-pc's Access Key output is off. The same check box reached by Ctrl+Shift+Tab (s11) has no key.
Okur drops it: `say_focus` (`native_focus.rs:585-596`) leaves out every line said only on
arrival, and access keys are such lines (`native_focus.rs:95`, `116`).
- Class: different handling (a dev-box rule: the Run dialog's Open field, Windows 11).
- Effect: asking "what is this and how do I reach it" does not give the shortcut.

**WinForms window view (322).**
- s3, s4: a replay artifact. In replay `read_window` returns false
  (`crates/okur-core/src/actions.rs:45-48`), so the viewer shows only the title and the field.
  Live Okur reads the window through UIA (`crates/okur-platform/src/window_text.rs`), which is
  not measured. JAWS's own view has screen-capture errors ("DocumentsPlan", and "Documennts" in a
  second trial) that Okur deliberately avoids (`viewer_text.rs:14-16`).
- s15, s16: JAWS's Virtualize Current Control on a list view shows one row, the last one
  ("photo.jpg 1.2 MB"), though the first is selected. Okur shows the focused item's name. A JAWS
  quirk; neither is useful. The useful view is every row with its columns.
- s18: after Escape from the viewer, JAWS says "budget.xlsx | 24 KB | 1 of 3", Okur only
  "budget.xlsx". `speak_focus_again` (`commands.rs:934`) uses `native_focus::focus_line`
  (`native_focus.rs:598-604`), the first line only. Different handling; the user loses the column
  and the position.
- s6: after Escape from the window view to the Name field, JAWS said nothing in all three trials
  (no audio within 800 ms: `heard_audio` is false in the evidence). Okur says "Ada". In 312/s11
  JAWS read the field's line in the same situation. Unexplained.

Verified: parity output; `okur-lab evidence 323-desktop-property-sheet --trace` and
`okur-lab evidence 322-desktop-forms-viewer --trace`; the 322 evidence JSON for s6.

### 2.4 Chrome's own controls (220, 5 steps)

- **Ctrl+T (s10).** JAWS: "New tab page | Adres ve arama çubuğu ‹ Edit ›". It names Chrome's new
  tab page in English rather than by the localized window title, and gives no hint. Okur: the
  window title, the address bar, and "Type in text."
- **Ctrl+W (s11).** JAWS says "Close Document Window", its own confirmation for the key, with the
  address bar. Okur says the window title and the address bar.
- **Escape from Chrome's app menu (s5).** JAWS: "Leaving menus", then the window (twice, with the
  Chrome profile's name), "‹ page ›" and the page title. Okur: "Leaving menus". `focus.rs:291`
  names the page when coming back from the browser's controls, except after leaving menus. No
  observation is cited for that exception.
- **Down in the address bar (s9, s12).** JAWS is silent. Okur reads the address again.

Root: Okur has no Chrome module. `crates/okur-core/src/apps/web/` holds only Google Docs.

- Class: unimplemented (s10, s11); different handling (s5, s9, s12).
- Effect: a JAWS user hears "Close Document Window" as the confirmation that a tab closed, and
  hears where they are after closing Chrome's menu. The rest is a word or two. Low to medium.

### 2.5 JAWS's own dialogs in replay (222-226, 41 steps)

What the evidence holds: JAWS's elevated dialogs as our tracer sees them. Buttons keep their "&"
and have no role ("&Move To Link"), and the first focus is the static text "JAWS Version
2025.2505.43" (trace of 222/s1). Replay types JAWS's TypeKey keys into Okur (`parity.rs:313-320`).
Okur answers with its own list, spoken because replay has no Okur windows (`commands.rs:382-385`),
and then with whatever JAWS's dialog events say. The comparison is fair to neither reader. This
is documented (`docs/references/verification-loop.md:348-351`, the plan's step 1).

Real gaps visible through it:
- **Insert+F3 (Virtual HTML Features) does not exist in Okur.** There is no command for it in
  `command.rs` at `4495382`. Scenario 226 depends on it.
- **Okur's Select a Form Field lists 8 fields; JAWS's lists 10** (225/s1, "1 of 8" against
  "1 of 10"). JAWS's third item is "Outline Button". Which two Okur leaves out was not
  determined; the list comes from `buffer.rs:1109` (`get_form_fields_list`).
- **JAWS names its own window first** ("JAWS Version 2025.2505.43"). The plan leaves this to the
  owner.

- Class: replay artifact, all 41 steps, with two unimplemented features inside.
- Truthful measure: `run-scenario.ps1 -Reader okur`, then `okur-lab live-parity`. The one result so
  far (222: 5 of 11 steps, then 10 of 11) is quoted in commit `00cc733`. The run file is not in
  the repository, so the number cannot be checked again.
- Effect: the lists of links, headings and form fields are among JAWS's most used web
  features. How close Okur's are is unknown until each is run live.

### 2.6 Heading level keys: the section's own heading (104/s1, 12/s1)

JAWS, pressing 2 from the top of the article: "City Library Expands Evening Hours ‹ heading
level 1 › | What changes ‹ heading level 2 ›", in all 5 trials. In 12 it does the same with
"System Documentation". Pressing 2 again (104/s2) says only the next level 2 heading. JAWS's
message at the end is "No more headings at level2 in this section." (104/s4, matched).

Inferred rule: when a level key moves into a new section (the target's nearest higher heading
changes), JAWS first says that higher heading.

Okur: `cursor.rs:909-926` moves to the next heading of the level and says only that heading. The
test `lib.rs:329-345` asserts Okur's speech for scenario 12, which the evidence contradicts.

- Class: unimplemented (the rule is inferred from two steps).
- Effect: reading documentation or a long article by heading level, the user does not hear
  which part a subheading belongs to. Low to moderate, and it is muscle memory.
- Proof: a scenario with level 2 and 3 headings under several level 1 and 2 headings: 3 across
  sections, and Shift+2 backwards.

### 2.7 The window title (209/s1, 207/s87)

JAWS's Insert+T in Chrome: "Title is | (page) - Google Chrome - (the Chrome profile's name)".
Okur says the title without the profile's name.

Root: Okur's title is the window's caption as the tracer records it. Chrome puts the profile name
only in the window's accessible name: in the 209 trace (line 2) the MSAA name has it and the
caption does not. `commands.rs:74-93` (Say Window Title) uses that caption.

- Class: different handling. The cost to a user is nil.
- 207/s87 adds "main region | level 2 | heading is Character review" in the history. The spoken
  audio ends after the title, possibly cut. In Okur's buffer the page's only heading is level 1.
  Unexplained. A recording of Insert+T after moving into a region and under a heading would
  show it.

### 2.8 A page's menu opened with the page's own shortcut (218/s10)

JAWS: "Main menu menu | New ‹ menu › | To move through items press left or right arrow." Okur:
"‹ menu › New | To move through items press up or down arrow."

JAWS names the menu bar ("Main menu") and puts the role after the item. Its hint says left or
right though the open menu goes up and down, which is a JAWS quirk. The spoken audio ends after
"New menu", so the hint was not heard in this recording.

- Root: `focus.rs:114-131` (a page's menu speech by `PageMenuPlace`).
- Class: different handling; the hint is for the owner to decide.

### 2.9 Notepad's window view (314/s10, s11)

JAWS's Virtualize Window in Windows 10's Notepad shows the title, the menu bar ("File Edit Format
View Help"), the text, and the status bar ("Ln 1, Col 1 100% Windows (CRLF) UTF-8"). Okur shows
the title and the text.

Root: `commands.rs:794-798` and `810-820`. A field whose text is the window's (Notepad) is shown
as its lines, after Windows 11 Notepad, where JAWS shows only the text. Live Okur takes the same
branch, so the gap is live too (inferred). The last handoff calls it a trace limit; it is also
this rule.

- Class: different handling (decided in code).
- Effect: low. The status bar is also reachable in other ways.

### 2.10 The desktop list's role marks (316/s1, 319/s9)

JAWS: "desktop ‹ › | FolderView ‹ List view › | ...". Okur says the same words without the
marks.

- Root: `crates/okur-core/src/apps/windows/desktop.rs:114-129` writes the line by hand, after the
  dev box's Windows 11 wording.
- Class: different handling with no audible effect: the spoken text differs only where JAWS's
  was cut and in "Folder View" (word splitting, 3.2). A few minutes' work for 2 steps.

### 2.11 Say Font in a WinForms field (313, 8 steps)

JAWS says "No font", and twice quickly "No font No font"; no viewer opens. Okur says "Font is
Consolas 11 point", and the second press opens the Virtual Viewer. s3 to s9 differ because
Okur's Down, Ctrl+A and Escape act in that viewer.

Root and history:
- The tracer reads a plain edit's font from its HFONT since `cbcd429`. The re-recorded trace
  (`f60b3ac`) carries it (`"font":{"family":"Consolas","size_tenths":110,...}`); the earlier
  trace (`2bc2353`) did not.
- The baseline had 313/s1-s6 and s9 as "match" at `4a5e57a` and as "mismatch" at `24bf3fa`
  (verified with `git show` of both baselines). No handoff mentions the change.
- The comment at `commands.rs:668-672` still expects "No font" for 313.

- Class: JAWS quirk. JAWS finds no font in a WinForms text box but finds one in Notepad's
  (314/s1). Okur's answer is correct and more useful: keep it and record the decision.
- A side effect to check (inferred): to match JAWS in Notepad, `24bf3fa` reports a plain edit's
  font by its ascent (Consolas 12 is "11 point"). Applied to every plain edit, it may give other
  applications' fonts one point smaller than they were set. Check live on a WinForms field.

### 2.12 The emoji JAWS's viewer drops (312, 4 steps)

JAWS's viewer line ends "...≤λ", without the non-breaking space and the grinning face emoji
after it. Okur keeps both. JAWS reads the emoji when reading the field itself (313/s4). Copying
from JAWS's viewer loses the emoji; copying from Okur's does not.

- Class: JAWS quirk; Okur is better. No comment in the code records the decision.

### 2.13 Words JAWS kept in its history but did not speak (209/s8, 314/s3, 315/s16)

On the first Down in JAWS's font message the history says "Keys | Press ESCAPE to close this
message." (209/s8) and "ESCAPE | Press ESCAPE ..." (314/s3); on Ctrl+Home it says "List Font
information:" (315/s16). The spoken layer has only what Okur says: 209/s8 and 314/s3 match, and
315/s16 differs only by "colon". The extra word goes into JAWS's history but not to its
synthesizer. In 209/s8 and 315/s16 it is a word of the message's link line ("JAWS Hot Keys",
"Link List"); where "ESCAPE" comes from in 314/s3 is not known.

- Class: evidence artifact. Okur matches what the user hears.
- This answers the open question in `verification-loop.md` §9 about "ESCAPE": it is not spoken.

### 2.14 Say All (01/s1, 111/s1)

- **01** (an early scenario, `settle_ms: 800`): JAWS's history was read 800 ms after Say All
  began and holds the first three lines. Okur says the whole document. An evidence artifact; 111
  records Say All properly.
- **111**: Okur matches JAWS's text to its last line. JAWS then adds "end", in all 5 trials. It
  is not a line of JAWS's buffer: Down at the bottom repeats "content information end"
  (101/s70, s71). Unexplained. A recording with the speech recorder voice would show whether
  JAWS speaks it, and another page would show whether it always comes.

### 2.15 Two smaller evidence questions (204/s1, 305/s13)

- **204/s1**, the first Tab into the sign-up form. In 4 of 5 trials JAWS says "Create Your
  Account - Riverside Library | Full name ‹ Edit › ‹Required›". Trial 1 adds the regions and
  "blank". Okur says "... ‹Required› | blank" (`speech.rs:399-410`: an empty edit says "blank"),
  which matches neither. With F and E (202, 203) JAWS says "‹Required› | blank" ten times each.
  Re-record with the speech recorder voice.
- **305/s13**, Shift+Home after typing "b", "o" and a space before "d": JAWS says "selected do"
  in all 5 trials. Okur says "selected bo", which is what the keys select (inferred). A JAWS
  quirk. Adding a Ctrl+C clipboard capture after the step would prove the selection.

## 3. The spoken layer

The spoken layer compares what Okur would give the synthesizer with what JAWS sent its
synthesizer, through the speech recorder voice. It exists for the scenarios recorded since the
recorder: 207-226 and 306-323 except 312, 647 steps in all.

```
$ okur-lab parity --all --spoken
JAWS spoken parity: 465/647 steps match exactly (evidence recorded through the speech recorder voice)
```

Of the 182 steps that differ, 81 also differ in the history layer. Their spoken difference
follows the wording and is counted in section 2. The other 101 differ only in what is spoken.
Each of them was read and given one or more causes (appendix B). A prefix test confirmed the
30 steps whose only cause is the cut: JAWS's spoken text is the start of Okur's.

### 3.1 Symbols and punctuation in running text (59 steps; the only cause in 42)

What JAWS sent its synthesizer at punctuation Most, the level old-pc's voice profile uses
(`verification-loop.md:72-75`):
- "Save..." as "Save dot dot dot"; "•" as "bullet"; "Ctrl+N" as "Ctrl plus N".
- "fn main() {" as "fn main left paren right paren left brace".
- "Notes:" as "Notes colon"; "alpha.rs" as "alpha dot rs"; "Read-only" as "Read dash only".
- "document?" as "document question?" in the Virtual Viewer, but as "document?" in a dialog's
  text. JAWS's rules depend on context.

What Okur does:
- Its spoken text is the history text with the role marks removed (`pronounce.rs:62-66`).
- Live, the synthesizer gets the text as it is (`crates/okur-platform/src/tts/mod.rs:183-189`).
- Only a character read alone gets JAWS's names (`pronounce.rs:26-37`, `68-125`, measured in
  307).
- There is no punctuation setting in `settings.rs`.
- Decision: the extension SDK proposal schedules punctuation levels for milestone M5
  (`extension-sdk-proposal.md` §6.3).

Effect beyond the corpus (inferred for the voices): what a user hears for code, command lines,
paths, web addresses, email addresses and prices depends on the voice's own rules, and most
voices skip brackets and many symbols. A blind developer reading code, or anyone checking a
command or an address before pressing Enter, needs the symbols named. This is the largest
daily-use gap the spoken layer shows. Only the Most level has evidence; the other levels have
none.

### 3.2 Words JAWS splits or respells (13 steps; the only cause in 8)

"MenuBar" as "Menu Bar", "SearchRegion" as "Search Region", "ContentInfoRegion" as "Content Info
Region", "PlaceMarkers" as "Place Markers", "PowerShell" as "Power Shell", "ÜstKrktr" as "Üst
Krktr", "System32" as "System 32", "spacebar" as "space Bar". This is JAWS's mixed-case and
number processing, plus one respelling. Okur leaves words to the voice.

- Effect: low, except for identifiers in code, which JAWS splits into words.
- Several of the split words are Okur's own (its role words "MenuBar", "SearchRegion" come from
  JAWS's history spelling). The catalog keeps JAWS's spelling, and the spoken step decides how
  they sound.

### 3.3 Recordings cut short by the harness (46 steps; the only cause in 30)

`run-scenario.ps1:582-584` reads the recorder's records once the audio has been quiet for
500 ms. JAWS sends an announcement as several utterances with pauses between them. In 306/s1,
trial 1, the dialog's name arrives 221 ms after the key and its text 2,553 ms after. The focused
button and its hint, which the history holds, are not in the step's records. The cut parts are
mostly tutor messages and positions ("To activate press Enter.", "1 of 3").

- Class: harness artifact. The history layer matches in these steps, which shows JAWS did queue
  the text.
- Verified: the record times above, from the 306 evidence file; the read at line 584 after the
  500 ms wait at line 582.
- Inferred: that the missing parts reach the recorder during the next step's silent-baseline
  wait (line 576), before that step records where its own speech starts, and so belong to no
  step.
- Fix: read the records up to the moment the next step's key is sent, or cut them by their
  timestamps; then re-record.

### 3.4 Speech in the wrong step (2 steps)

317/s1 and s5 (Alt held through the task switcher): JAWS's audio for one step falls in the
other. The same timing fix applies.

### 3.5 How much of the spoken gap is one cause

Symbol processing touches 59 of the 101 steps (58%). The harness's cut touches 46 (46%).
JAWS's Most level and its word splitting in Okur would resolve up to 53 steps; the harness fix
would resolve 30 more. Both together would resolve 99 of 101. That is an upper bound: it
assumes Okur reproduces JAWS's rules exactly.

## 4. The UIA path: what users hear in a page's first seconds

How live Okur reads a page:
- `load_page_if_new` (`events/windows.rs:398-439`) reads a new page through UIA
  (`capture_foreground_page_quick`, `actions.rs:555-564`). Its comment gives 160 ms through UIA
  against 3 s through IA2 for GitHub's repository page.
- The IA2 read then replaces it.
- When IA2 gives no page, UIA stays (`actions.rs:498-520`).
- Page actions need IA2 unique ids. Text fields in a page are read through IA2 text
  (`text_context.rs:55-67`). Native windows are read through UIA, MSAA and Win32 (dialog rows
  from window text, fonts from HFONT).

So the UIA replay shows what a user hears until the IA2 read lands, and on any page where IA2
fails.

The 116 steps that differ only through UIA (appendix C):
- **Block quotes (74 steps: 101 s49-s71, 102 s21-s71).** Chrome's UIA gives a block quote as a
  group whose only distinct property is its localized type name ("blok alıntı" on old-pc), which
  Okur must not key on. Verified with `okur-lab lines` on both news-article snapshots: the IA2
  buffer's lines 49 and 51 ("block quote", "block quote end") are not in the UIA buffer. Every
  later Down and Up step is off by those lines.
- **Preformatted text (19 steps: 207 s67-s86).** The UIA buffer has the pre block as one line,
  with its line breaks, tab and double space collapsed, so the character positions shift. A code
  block read before the IA2 read lands has no lines.
- **A page's menu bar (7 steps: 218)** and **message groups (4 steps: 219)**.
- **Fonts (6 steps: 209).** UIA gives no formatting runs (`actions.rs:557`), so Okur says "No
  font".
- **Role descriptions and menu buttons (6 steps: 210).** "Button" for "slide control", "Link" for
  "chip", "check box" for "toggle", "Button collapsed" for "button menu". The UIA snapshot has
  `haspopup=menu` in its ARIA properties, but `to_aom` uses haspopup only for combo boxes
  (`uia_snapshot.rs:357-366`). Role descriptions come only as the localized type name, mixed with
  Windows' own localized names.

- Class: different handling, decided in code for speed (`events/windows.rs:394-397`).
- Effect: brief on most pages. It matters on large pages, where the IA2 read takes seconds, and
  it decides everything where IA2 fails.

## 5. Rules taken from the dev box that old-pc contradicts

Seven rules in the code cite JAWS on the dev box (Windows 11, the owner's JAWS settings), and
old-pc's evidence (Windows 10, the reference settings) contradicts them:

1. A plain focusable element is said by its name (`focus.rs:450-469`; WhatsApp). Old-pc reads the
   virtual cursor's line (219/s8, s17).
2. A page's popup menu items say their position (`web_app_tests.rs:304-306`). Old-pc says
   "‹ menu ›" and no position (219/s14, s15).
3. The page is named on coming back from the browser's controls, but not after leaving its
   menus (`focus.rs:286-291`; the dev box is cited for the first part, nothing for the
   exception). Old-pc names it after leaving Chrome's menu too (220/s5).
4. Say Window's format: lower-case words, repeated labels (`native_focus.rs:537-541`). Old-pc
   uses role marks, "read only edit" and single labels (323/s2).
5. Say Focus leaves out the access key (`native_focus.rs:585-588`). Old-pc says it (323/s1).
6. A text field that is its window's text is shown alone in the window view
   (`commands.rs:794-797`). Old-pc's Notepad adds its menu bar and status bar (314/s10, s11).
7. The desktop list's wording, written by hand (`desktop.rs:114-117`). Old-pc has role marks
   (316/s1).

`verification-loop.md` §7 already warns that the dev box's settings are the owner's. What is
missing is a record, with each dev-box rule, of the settings and Windows version it was seen
with, and a way to settle a conflict. The proposal (item 9) suggests one.

## 6. Notes on measurement integrity

- The baseline and the parity output agree: 97 mismatches each.
- 313 lost 7 matching steps on 2026-10-18 (section 2.11). The ratchet allowed it because the
  baseline was regenerated with the re-recorded corpus, and the handoff does not name it. A
  baseline regenerated together with new evidence can hide a change in old steps; a short list
  of steps that changed state, written into the commit message, would show it.
- AGENTS.md rule 6 ("Known gaps are tests") has no tests: `git grep -i "known gap"` at
  `4495382` finds none. The baseline records "mismatch" without a reason. Appendix A could serve
  as that ledger until the tests exist.
- `lib.rs:329-345` asserts Okur's speech for scenario 12, which JAWS's evidence contradicts. It
  will fail when Okur is made to speak as JAWS does, so it works against the fix.
- Live comparisons are not kept. `okur-lab live-parity` output is not evidence, but a committed run
  log, beside the claim, would let a number like "10/11" be checked.
- Scenarios recorded before the speech recorder (01-30, 101-114, 201-206, 301-305, 312) have no
  spoken layer, and the earliest use fixed waits; several of those checked have no trace (01,
  12, 104, 111). 01's 800 ms capture is one result.

## 7. Effect on everyday tasks, and what the corpus does not cover

A blind user's day, task by task. "A word differs" means the user hears a different word, "misled"
means they could act wrongly, and "lost" means they no longer know where they are.

- **Reading articles and documentation.** Lines, headings, lists, tables and regions match in
  depth (101-114: two DIFFs through IA2, 104/s1 and 111/s1). Gaps: the section heading on
  level keys (a word), and in the first seconds of a page the UIA buffer (block quotes missing,
  code blocks flattened). Symbols in code samples are left to the voice (misled for code).
- **Web forms.** Tab order, quick keys and states match (201-205, but for 204/s1). Gaps:
  "blank" after a required field on Tab (a word, unsettled). Not covered: error messages after
  a submit (aria-invalid, error text, alerts), a native select in forms mode, date and number
  fields, and fieldsets read with the arrows (the plan's step 3b).
- **Web applications (WhatsApp, Slack, Gmail).** The cursor left on the previous focus (lost),
  the page title repeated (noise), menu items worded differently (a word). Not covered: a mail
  list grid with a reading pane, and live updates while typing.
- **Email in a desktop client.** Not covered. The multi-line field gap (2.1) would hit a mail
  body.
- **Documents.** Notepad and RichEdit are covered (314, 315, 320). Word processors are not: Word
  and WordPad have no scenario, and Google Docs was studied only on the dev box.
- **Spreadsheets.** Not covered on old-pc. Google Sheets was studied only on the dev box.
- **Dialogs.** Message boxes, property sheets and the Run dialog are covered (306, 311, 319, 322,
  323). Gaps: the page name on Ctrl+Tab (lost), Say Window's noise and missing "read only"
  (misled), the access key on Insert+Tab (a word). Not covered: Save As and Open dialogs, which
  users meet every day.
- **Consoles.** The window view is covered (321). Not covered: new output arriving while a
  command runs, and reading it back. Symbol names matter most here (paths, switches).
- **Browsing.** Chrome only. Gaps in Chrome's own controls (a word, a missing confirmation). Not
  covered: Edge and Firefox.

Scenarios that would cover the gaps that matter most:
1. A multi-line WinForms field with a longer text, the caret in the middle, Tab and Alt+Tab into
   it.
2. 219 again with Auto Forms Mode on, beside the current recording.
3. A punctuation scenario: the characters fixture by line, word and Say All, at each of JAWS's
   four levels (old-pc's voice profile changed for each run).
4. A heading-level scenario across sections (2.6).
5. Insert+Tab and Insert+B on the Run dialog, Save As and a message box (Windows 10), to settle
   the dev-box rules 4 and 5.
6. A web form with validation errors and a native select.
7. A Save As dialog: its folder list, file name field and file type combo box.
8. A console with output arriving after a command.

## 8. Proposal

In order of impact on real use, then effort. Each item says where the fix goes, how long it
takes, what proves it, and whether old-pc must record anything new.

1. **A multi-line native field on focus.**
   - What: say its line and "Contains text" instead of its whole text (2.1).
   - Where: `native_focus.rs:179-203`, using the line-break signal of `commands.rs:815-819`.
   - Effort: hours.
   - Proof: 316/s3 matches; `okur-lab parity` gains 1 step.
   - Recording: optional, scenario 1 of section 7, to settle "caret's line or first line".

2. **Web applications: a focus the buffer has no line for.**
   - What: put the cursor on the first line inside the focused element, keep `last_page_focus`
     when no node is found, then settle the plain-element and menu-item rules by setting (2.2).
   - Where: `focus.rs:372-378`, `415-416`, `436-469`; `page.rs:265-299`.
   - Effort: 1-2 days.
   - Proof: 219/s9 and s13 match in replay after the defect fix, s8, s11 and s17 after the rule
     decision; a live run of 219 with `okur-lab live-parity`; a live check on WhatsApp on the dev
     box, with the owner's permission.
   - Recording: 219 with Auto Forms Mode on.

3. **The harness: spoken text up to the next key.**
   - What: read the recorder's records until the next step's key is sent, or cut by their
     timestamps (3.3, 3.4).
   - Where: `skills/jaws-reference/scripts/run-scenario.ps1:576-584`.
   - Effort: hours, then one batch on old-pc.
   - Proof: `okur-lab parity --spoken`; up to 30 steps should match without any engine change.
   - Recording: re-record 209, 211-220, 306, 311, 314, 315, 317, 319 and 322. Also give live
     runs the recorder voice (`okur run --engine sapi5` with the SR Speech Recorder voice), so
     Okur's own spoken layer can be measured live.

4. **Punctuation and symbols for running text.**
   - What: JAWS's levels (None, Some, Most, All) with a setting, starting with Most in English,
     and the mixed-case and number splitting (3.1, 3.2). Bring the English Most level forward
     from the SDK's M5: the spoken layer already measures it.
   - Where: `pronounce.rs` (a text pass after `text`, line 64), the live speech path
     (`tts/mod.rs:183-189`), `settings.rs`.
   - Effort: 3-5 days for Most; more for the other levels and per-context punctuation.
   - Proof: up to 53 spoken steps; the 14 "symbols, cut short" steps after item 3.
   - Recording: scenario 3 of section 7.

5. **Dialogs.**
   - What: the page name and its text on Ctrl+Tab; the tab row in the window view; marks
     stripped from dialog rows; Say Window's format; the access key on Insert+Tab; a list item
     with its columns and position after the viewer closes (2.3).
   - Where: `focus.rs:653-694`; `screen_lines.rs:377-455`; `native_focus.rs:537-596`;
     `commands.rs:934` with `native_focus.rs:598-604`.
   - Effort: 1-2 days.
   - Proof: 323/s1, s2, s4-s7, s10, s11 and 322/s18 (up to 9 steps).
   - Recording: scenario 5 of section 7, so the dev-box and old-pc rules are settled on several
     dialogs and not fitted to one.

6. **JAWS's own dialogs, measured live.**
   - What: finish the list family (the plan's step 3: the form field count, 8 against 10; Insert+F3,
     which does not exist), run each live, and keep the `okur-lab live-parity` output as a committed
     run log. Report 222-226 apart from the replay headline, so the headline measures what replay
     can measure.
   - Where: `crates/okur-core/src/ui/`, `buffer.rs:1109`, `command.rs`;
     `crates/okur-corpus/src/parity.rs` for the separate report.
   - Effort: several days for the dialogs; hours for the report.
   - Proof: live-parity results per dialog.
   - Recording: none for JAWS (222-226 exist); live Okur runs on old-pc.

7. **Chrome's own controls.**
   - What: "New tab page", "Close Document Window", the page named after leaving Chrome's menu,
     and no speech for Down in the address bar (2.4).
   - Where: a new Chrome module in `crates/okur-core/src/apps/web/`; `focus.rs:291`.
   - Effort: half a day.
   - Proof: 220/s5, s9-s12 (5 steps).
   - Recording: none.

8. **The UIA first read.**
   - What: map `haspopup=menu` to a menu button; read preformatted text with its lines (UIA
     TextPattern); accept that block quotes cannot be seen through UIA. Measure how long users
     stay on the UIA buffer with `okur-lab latency` on real pages before doing more (section 4).
   - Where: `crates/okur-platform/src/uia_snapshot.rs:274-370`.
   - Effort: half a day for haspopup; about a day for preformatted text.
   - Proof: `okur-lab parity --api uia` (210: up to 2 steps; 207: up to 19).
   - Recording: none.

9. **Dev-box rules: record where they come from.**
   - What: next to each dev-box observation in the code, name the JAWS settings and Windows
     version it was seen with. When old-pc's evidence contradicts one, record the scenario on
     old-pc with the dev box's setting changed, to find what decides (section 5).
   - Where: comments and `verification-loop.md` §7.
   - Effort: an hour to start; then part of each fix.
   - Recording: as each conflict is taken up.

10. **The section heading on level keys** (2.6).
    - Where: `cursor.rs:909-948`; correct the test at `lib.rs:329-345`.
    - Effort: half a day.
    - Proof: 104/s1, 12/s1.
    - Recording: scenario 4 of section 7.

11. **Small differences.**
    - Desktop role marks (`desktop.rs:119-129`): minutes, 2 steps.
    - The window title's profile name (2.7): hours, 1 step. It needs the window's accessible name
      in the tracer.
    - Notepad's menu bar and status bar in the window view (2.9): half a day.
    - The menu opened by a page's shortcut (2.8): the owner decides on the hint.

12. **Accept, in writing, where Okur is better.**
    - What: the font in a WinForms field (313), the emoji in the viewer (312), the selection text
      (305/s13), the list view's control view (322/s15, s16). Write each decision in the code and
      in a reviewed list of accepted differences that `okur-lab parity` reports apart. The baseline
      stays generated and is never edited.
    - Also update the stale comment at `commands.rs:668-672`.
    - Effort: half a day (the list and its report).
    - Recording: none.

13. **Re-recordings for evidence questions.**
    - 01 in format 2 with silence settling, or retire it (111 covers Say All).
    - 204 with the speech recorder voice.
    - 305 with a clipboard capture after s13.
    - 111's final "end" on a second page.
    - 207's Insert+T in a region under a heading.
    - Effort: one old-pc batch.

14. **Known gaps as tests.**
    - What: for each real gap above with a known JAWS behavior, an `#[ignore = "known gap: ..."]`
      test asserting it, as AGENTS.md rule 6 asks.
    - Effort: an hour per group.

## Appendix A: every history-layer DIFF step

Class names: "Replay artifact" and "Evidence artifact" are measurement only. "JAWS quirk" means
JAWS's behavior is odd and Okur's is kept. "Different handling" and "Unimplemented" are real
gaps in Okur. The last column says how the step fares in the spoken layer.

| Step | Key | Group | Class | Spoken layer |
|---|---|---|---|---|
| 01-doc-say-all/s1 | SayAll | Say All | Evidence artifact: history read 800 ms into Say All | not recorded |
| 104-article-heading-levels/s1 | 2 | Section heading | Unimplemented | not recorded |
| 111-article-say-all/s1 | Insert+Down | Say All | Unexplained JAWS word ("end") | not recorded |
| 12-struct-heading-level-2/s1 | MoveToNextHeadingLevelTwo | Section heading | Unimplemented | not recorded |
| 204-form-tab-order/s1 | Tab | Evidence questions | Evidence artifact: trials differ | not recorded |
| 207-web-character-review/s87 | Insert+T | Window title | Different handling; JAWS addition unexplained | differs |
| 209-web-title-and-font/s1 | Insert+T | Window title | Different handling | differs |
| 209-web-title-and-font/s8 | Down | History-only words | Evidence artifact: history word not spoken | matches |
| 218-web-aria-menus/s10 | Alt+Shift+F | Page menu shortcut | Different handling; JAWS hint is a quirk | differs |
| 219-web-chat-patterns/s8 | Shift+Tab | Web app focus | Different handling (dev-box rule) | differs |
| 219-web-chat-patterns/s9 | Down | Web app focus | Different handling (defect: cursor left on the previous focus) | differs |
| 219-web-chat-patterns/s11 | Down | Web app focus | Different handling (defect: page title repeated) | differs |
| 219-web-chat-patterns/s13 | Right | Web app focus | Different handling (defect: page title repeated) | differs |
| 219-web-chat-patterns/s14 | Enter | Web app focus | Different handling (menu wording, page title) | differs |
| 219-web-chat-patterns/s15 | Tab | Web app focus | Different handling (menu item wording) | differs |
| 219-web-chat-patterns/s16 | Down | Web app focus | Different handling (menu not in the buffer) | differs |
| 219-web-chat-patterns/s17 | Escape | Web app focus | Different handling (page title, label) | differs |
| 220-web-chrome-own-controls/s5 | Escape | Chrome controls | Different handling (decided in code) | differs |
| 220-web-chrome-own-controls/s9 | Down | Chrome controls | Different handling | differs |
| 220-web-chrome-own-controls/s10 | Ctrl+T | Chrome controls | Unimplemented ("New tab page") | differs |
| 220-web-chrome-own-controls/s11 | Ctrl+W | Chrome controls | Unimplemented ("Close Document Window") | differs |
| 220-web-chrome-own-controls/s12 | Down | Chrome controls | Different handling | differs |
| 222-web-links-list/s1 | Insert+F7 | JAWS dialogs in replay | Replay artifact | differs |
| 222-web-links-list/s2 | TypeKey("Tab") | JAWS dialogs in replay | Replay artifact | differs |
| 222-web-links-list/s3 | TypeKey("Tab") | JAWS dialogs in replay | Replay artifact | differs |
| 222-web-links-list/s4 | TypeKey("Tab") | JAWS dialogs in replay | Replay artifact | differs |
| 222-web-links-list/s5 | TypeKey("Tab") | JAWS dialogs in replay | Replay artifact | differs |
| 222-web-links-list/s6 | TypeKey("Tab") | JAWS dialogs in replay | Replay artifact | differs |
| 222-web-links-list/s7 | TypeKey("Tab") | JAWS dialogs in replay | Replay artifact | differs |
| 222-web-links-list/s8 | TypeKey("DownArrow") | JAWS dialogs in replay | Replay artifact | differs |
| 222-web-links-list/s9 | TypeKey("DownArrow") | JAWS dialogs in replay | Replay artifact | matches |
| 222-web-links-list/s10 | TypeKey("Enter") | JAWS dialogs in replay | Replay artifact | differs |
| 222-web-links-list/s11 | Down | JAWS dialogs in replay | Replay artifact | differs |
| 223-web-links-list-escape/s1 | Insert+F7 | JAWS dialogs in replay | Replay artifact | differs |
| 223-web-links-list-escape/s2 | TypeKey("DownArrow") | JAWS dialogs in replay | Replay artifact | differs |
| 223-web-links-list-escape/s3 | TypeKey("Escape") | JAWS dialogs in replay | Replay artifact | differs |
| 224-web-headings-list/s1 | Insert+F6 | JAWS dialogs in replay | Replay artifact | differs |
| 224-web-headings-list/s2 | TypeKey("Tab") | JAWS dialogs in replay | Replay artifact | differs |
| 224-web-headings-list/s3 | TypeKey("Tab") | JAWS dialogs in replay | Replay artifact | differs |
| 224-web-headings-list/s4 | TypeKey("Tab") | JAWS dialogs in replay | Replay artifact | differs |
| 224-web-headings-list/s5 | TypeKey("Tab") | JAWS dialogs in replay | Replay artifact | differs |
| 224-web-headings-list/s6 | TypeKey("Tab") | JAWS dialogs in replay | Replay artifact | differs |
| 224-web-headings-list/s7 | TypeKey("Tab") | JAWS dialogs in replay | Replay artifact | differs |
| 224-web-headings-list/s8 | TypeKey("DownArrow") | JAWS dialogs in replay | Replay artifact | differs |
| 224-web-headings-list/s9 | TypeKey("DownArrow") | JAWS dialogs in replay | Replay artifact | differs |
| 224-web-headings-list/s10 | TypeKey("Enter") | JAWS dialogs in replay | Replay artifact | differs |
| 224-web-headings-list/s11 | Down | JAWS dialogs in replay | Replay artifact | differs |
| 225-web-form-fields-list/s1 | Insert+F5 | JAWS dialogs in replay | Replay artifact | differs |
| 225-web-form-fields-list/s2 | TypeKey("Tab") | JAWS dialogs in replay | Replay artifact | differs |
| 225-web-form-fields-list/s3 | TypeKey("Tab") | JAWS dialogs in replay | Replay artifact | differs |
| 225-web-form-fields-list/s4 | TypeKey("Tab") | JAWS dialogs in replay | Replay artifact | differs |
| 225-web-form-fields-list/s5 | TypeKey("Tab") | JAWS dialogs in replay | Replay artifact | differs |
| 225-web-form-fields-list/s6 | TypeKey("Tab") | JAWS dialogs in replay | Replay artifact | differs |
| 225-web-form-fields-list/s7 | TypeKey("Tab") | JAWS dialogs in replay | Replay artifact | differs |
| 225-web-form-fields-list/s8 | TypeKey("DownArrow") | JAWS dialogs in replay | Replay artifact | matches |
| 225-web-form-fields-list/s9 | TypeKey("DownArrow") | JAWS dialogs in replay | Replay artifact | matches |
| 225-web-form-fields-list/s10 | TypeKey("Enter") | JAWS dialogs in replay | Replay artifact | differs |
| 225-web-form-fields-list/s11 | Down | JAWS dialogs in replay | Replay artifact | differs |
| 226-web-element-lists/s1 | Insert+F3 | JAWS dialogs in replay | Replay artifact | differs |
| 226-web-element-lists/s2 | TypeKey("h") | JAWS dialogs in replay | Replay artifact | differs |
| 226-web-element-lists/s3 | TypeKey("Enter") | JAWS dialogs in replay | Replay artifact | differs |
| 226-web-element-lists/s4 | TypeKey("DownArrow") | JAWS dialogs in replay | Replay artifact | matches |
| 226-web-element-lists/s5 | TypeKey("Escape") | JAWS dialogs in replay | Replay artifact | differs |
| 305-desktop-edit-review/s13 | Shift+Home | Evidence questions | JAWS quirk (selection text) | not recorded |
| 312-desktop-virtualize-control/s3 | Down | Emoji in JAWS viewer | JAWS quirk | not recorded |
| 312-desktop-virtualize-control/s4 | Down | Emoji in JAWS viewer | JAWS quirk | not recorded |
| 312-desktop-virtualize-control/s9 | Ctrl+A | Emoji in JAWS viewer | JAWS quirk | not recorded |
| 312-desktop-virtualize-control/s14 | Ctrl+A | Emoji in JAWS viewer | JAWS quirk | not recorded |
| 313-desktop-say-font/s1 | Insert+F | Font in WinForms field | JAWS quirk (Okur keeps the font) | differs |
| 313-desktop-say-font/s2 | Insert+F | Font in WinForms field | JAWS quirk (Okur keeps the font) | differs |
| 313-desktop-say-font/s3 | Down | Font in WinForms field | JAWS quirk (Okur keeps the font) | differs |
| 313-desktop-say-font/s4 | Down | Font in WinForms field | JAWS quirk (Okur keeps the font) | differs |
| 313-desktop-say-font/s5 | Down | Font in WinForms field | JAWS quirk (Okur keeps the font) | differs |
| 313-desktop-say-font/s6 | Down | Font in WinForms field | JAWS quirk (Okur keeps the font) | differs |
| 313-desktop-say-font/s7 | Ctrl+A | Font in WinForms field | JAWS quirk (Okur keeps the font) | differs |
| 313-desktop-say-font/s9 | Escape | Font in WinForms field | JAWS quirk (Okur keeps the font) | differs |
| 314-desktop-notepad-font-and-viewer/s3 | Down | History-only words | Evidence artifact: history word not spoken | matches |
| 314-desktop-notepad-font-and-viewer/s10 | Down | Notepad window view | Different handling (decided in code) | differs |
| 314-desktop-notepad-font-and-viewer/s11 | Ctrl+A | Notepad window view | Different handling (decided in code) | differs |
| 315-desktop-richedit-lines-and-font/s16 | Ctrl+Home | History-only words | Evidence artifact: history word not spoken | differs |
| 316-desktop-show-desktop/s1 | Win+M | Desktop role marks | Different handling (no audible effect) | differs |
| 316-desktop-show-desktop/s3 | Alt+Tab | Multi-line field focus | Different handling | differs |
| 319-desktop-run-dialog-viewer/s9 | Escape | Desktop role marks | Different handling (no audible effect) | differs |
| 322-desktop-forms-viewer/s3 | Down | WinForms window view | Replay artifact: no screen read in replay | differs |
| 322-desktop-forms-viewer/s4 | Ctrl+A | WinForms window view | Replay artifact: no screen read in replay | differs |
| 322-desktop-forms-viewer/s6 | Escape | WinForms window view | Unexplained JAWS silence | differs |
| 322-desktop-forms-viewer/s15 | Insert+Shift+V | WinForms window view | JAWS quirk (one row) | differs |
| 322-desktop-forms-viewer/s16 | Ctrl+A | WinForms window view | JAWS quirk (one row) | differs |
| 322-desktop-forms-viewer/s18 | Escape | WinForms window view | Different handling | differs |
| 323-desktop-property-sheet/s1 | Insert+Tab | Property sheet | Different handling (dev-box rule) | differs |
| 323-desktop-property-sheet/s2 | Insert+B | Property sheet | Different handling (dev-box rule) | differs |
| 323-desktop-property-sheet/s4 | Down | Property sheet | Unimplemented (tab row) | differs |
| 323-desktop-property-sheet/s5 | Down | Property sheet | Unimplemented (tab row) | differs |
| 323-desktop-property-sheet/s6 | Down | Property sheet | Unimplemented (tab row) | differs |
| 323-desktop-property-sheet/s7 | Ctrl+A | Property sheet | Unimplemented (tab row) | differs |
| 323-desktop-property-sheet/s10 | Ctrl+Tab | Property sheet | Unimplemented (page name) | differs |
| 323-desktop-property-sheet/s11 | Ctrl+Shift+Tab | Property sheet | Unimplemented (page name) | differs |

## Appendix B: every step that differs only in the spoken layer

Causes: "symbols" (symbol and punctuation names in running text), "cut short" (the harness read
the recorder's text before JAWS finished), "word splitting" (JAWS's mixed-case and number
processing, or a respelling), "speech in the wrong step" (timing).

| Step | Key | Cause |
|---|---|---|
| 209-web-title-and-font/s7 | Insert+F | symbols, cut short |
| 210-web-current-and-described/s4 | Down | symbols |
| 210-web-current-and-described/s5 | Down | symbols |
| 210-web-current-and-described/s6 | Down | symbols |
| 211-web-autocomplete/s6 | Tab | cut short |
| 211-web-autocomplete/s9 | Down | cut short |
| 211-web-autocomplete/s10 | Down | cut short |
| 211-web-autocomplete/s11 | Down | cut short |
| 211-web-autocomplete/s12 | Up | cut short |
| 211-web-autocomplete/s13 | Enter | cut short |
| 212-web-tab-from-virtual-cursor/s7 | Tab | cut short |
| 213-web-textarea-words/s7 | Tab | cut short |
| 213-web-textarea-words/s11 | Ctrl+Right | symbols |
| 213-web-textarea-words/s14 | Ctrl+Right | symbols |
| 214-web-focus-containers/s8 | Tab | symbols, cut short |
| 215-web-application-region/s9 | Tab | cut short |
| 216-web-table-tab/s10 | Tab | symbols, cut short |
| 216-web-table-tab/s11 | Tab | cut short |
| 216-web-table-tab/s12 | Tab | symbols, cut short |
| 216-web-table-tab/s13 | Tab | symbols, cut short |
| 216-web-table-tab/s14 | Tab | cut short |
| 217-web-landmarks-on-focus/s11 | Tab | cut short |
| 217-web-landmarks-on-focus/s12 | Tab | word splitting, cut short |
| 217-web-landmarks-on-focus/s13 | Tab | cut short |
| 217-web-landmarks-on-focus/s14 | Tab | word splitting, cut short |
| 218-web-aria-menus/s1 | Tab | cut short |
| 218-web-aria-menus/s4 | Down | word splitting |
| 218-web-aria-menus/s6 | Escape | cut short |
| 218-web-aria-menus/s12 | Escape | cut short |
| 219-web-chat-patterns/s1 | Tab | cut short |
| 219-web-chat-patterns/s4 | Down | symbols |
| 220-web-chrome-own-controls/s1 | Alt+F | cut short |
| 220-web-chrome-own-controls/s2 | Down | symbols |
| 220-web-chrome-own-controls/s3 | Down | symbols, word splitting |
| 220-web-chrome-own-controls/s4 | Down | cut short |
| 220-web-chrome-own-controls/s7 | Ctrl+L | symbols, cut short |
| 221-web-empty-element-lists/s4 | K | word splitting |
| 221-web-empty-element-lists/s5 | Shift+K | word splitting |
| 306-desktop-dialogs-and-windows/s1 | Enter | cut short |
| 306-desktop-dialogs-and-windows/s2 | Tab | word splitting |
| 306-desktop-dialogs-and-windows/s3 | Escape | symbols, cut short |
| 306-desktop-dialogs-and-windows/s4 | Tab | word splitting |
| 306-desktop-dialogs-and-windows/s5 | Enter | cut short |
| 306-desktop-dialogs-and-windows/s6 | Alt+Tab | cut short |
| 306-desktop-dialogs-and-windows/s7 | Alt+Tab | cut short |
| 306-desktop-dialogs-and-windows/s8 | Alt+F4 | cut short |
| 306-desktop-dialogs-and-windows/s9 | Tab | symbols, word splitting |
| 306-desktop-dialogs-and-windows/s10 | Enter | cut short |
| 306-desktop-dialogs-and-windows/s11 | Enter | symbols, cut short |
| 311-desktop-virtualize-window/s1 | Insert+Alt+W | symbols |
| 311-desktop-virtualize-window/s4 | Ctrl+Home | symbols |
| 311-desktop-virtualize-window/s6 | Ctrl+Right | symbols |
| 311-desktop-virtualize-window/s7 | Insert+Up | symbols |
| 311-desktop-virtualize-window/s9 | Up | symbols |
| 311-desktop-virtualize-window/s10 | Ctrl+A | symbols |
| 311-desktop-virtualize-window/s12 | Escape | symbols |
| 311-desktop-virtualize-window/s13 | Enter | cut short |
| 311-desktop-virtualize-window/s21 | Ctrl+A | symbols |
| 311-desktop-virtualize-window/s24 | Escape | symbols, cut short |
| 311-desktop-virtualize-window/s25 | Alt+A | cut short |
| 314-desktop-notepad-font-and-viewer/s2 | Insert+F | symbols, cut short |
| 314-desktop-notepad-font-and-viewer/s6 | Ctrl+A | symbols |
| 314-desktop-notepad-font-and-viewer/s8 | Escape | symbols |
| 314-desktop-notepad-font-and-viewer/s9 | Insert+Alt+W | symbols |
| 314-desktop-notepad-font-and-viewer/s13 | Escape | symbols |
| 314-desktop-notepad-font-and-viewer/s14 | Insert+Shift+V | symbols |
| 314-desktop-notepad-font-and-viewer/s15 | Down | symbols |
| 314-desktop-notepad-font-and-viewer/s16 | Escape | symbols |
| 315-desktop-richedit-lines-and-font/s1 | Down | symbols |
| 315-desktop-richedit-lines-and-font/s3 | Down | symbols |
| 315-desktop-richedit-lines-and-font/s4 | Down | symbols |
| 315-desktop-richedit-lines-and-font/s5 | Down | symbols |
| 315-desktop-richedit-lines-and-font/s6 | Up | symbols |
| 315-desktop-richedit-lines-and-font/s9 | Ctrl+Home | symbols |
| 315-desktop-richedit-lines-and-font/s15 | Insert+F | symbols, cut short |
| 315-desktop-richedit-lines-and-font/s18 | Escape | symbols |
| 317-desktop-alt-tab-hold/s1 | Alt | speech in the wrong step |
| 317-desktop-alt-tab-hold/s3 | Tab | word splitting |
| 317-desktop-alt-tab-hold/s4 | Tab | symbols |
| 317-desktop-alt-tab-hold/s5 | Alt | speech in the wrong step |
| 319-desktop-run-dialog-viewer/s1 | Win+R | cut short |
| 319-desktop-run-dialog-viewer/s5 | Down | symbols |
| 319-desktop-run-dialog-viewer/s6 | Down | symbols |
| 319-desktop-run-dialog-viewer/s8 | Escape | symbols |
| 320-desktop-notepad-laptop-reading-keys/s1 | CapsLock+I | symbols |
| 320-desktop-notepad-laptop-reading-keys/s3 | CapsLock+O | symbols |
| 320-desktop-notepad-laptop-reading-keys/s4 | CapsLock+I | symbols |
| 320-desktop-notepad-laptop-reading-keys/s9 | CapsLock+U | symbols |
| 320-desktop-notepad-laptop-reading-keys/s10 | Insert+Up | symbols |
| 321-desktop-console-viewer/s5 | Down | symbols, word splitting |
| 321-desktop-console-viewer/s7 | Escape | symbols |
| 322-desktop-forms-viewer/s7 | Tab | symbols |
| 322-desktop-forms-viewer/s8 | Tab | word splitting |
| 322-desktop-forms-viewer/s9 | Tab | word splitting |
| 322-desktop-forms-viewer/s10 | Tab | cut short |
| 322-desktop-forms-viewer/s11 | Tab | symbols, cut short |
| 322-desktop-forms-viewer/s12 | Tab | symbols, cut short |
| 322-desktop-forms-viewer/s13 | Tab | cut short |
| 322-desktop-forms-viewer/s14 | Tab | symbols, cut short |
| 323-desktop-property-sheet/s3 | Insert+Alt+W | symbols |
| 323-desktop-property-sheet/s9 | Escape | symbols |

## Appendix C: every step that differs only through UIA

These steps match through IA2. Steps are listed by number within their scenario.

| Scenario | Steps | Cause |
|---|---|---|
| 101-article-read-down | s49 s50 s51 s52 s53 s54 s55 s56 s57 s58 s59 s60 s61 s62 s63 s64 s65 s66 s67 s68 s69 s70 s71 | No block quote lines through UIA; every later line shifts |
| 102-article-read-up | s21 s22 s23 s24 s25 s26 s27 s28 s29 s30 s31 s32 s33 s34 s35 s36 s37 s38 s39 s40 s41 s42 s43 s44 s45 s46 s47 s48 s49 s50 s51 s52 s53 s54 s55 s56 s57 s58 s59 s60 s61 s62 s63 s64 s65 s66 s67 s68 s69 s70 s71 | No block quote lines through UIA; every later line shifts |
| 207-web-character-review | s67 s68 s69 s71 s72 s73 s74 s75 s76 s77 s78 s79 s80 s81 s82 s83 s84 s85 s86 | Preformatted text loses its line breaks, tab and double space |
| 209-web-title-and-font | s3 s5 s7 s9 s10 s11 | No fonts through UIA ("No font"), so no viewer opens |
| 210-web-current-and-described | s9 s10 s11 s13 s14 s15 | Role descriptions and menu buttons (haspopup) not mapped |
| 218-web-aria-menus | s1 s2 s3 s4 s5 s6 s11 | A page's menu bar is linearized differently |
| 219-web-chat-patterns | s4 s5 s6 s7 | Message containers become group lines |
