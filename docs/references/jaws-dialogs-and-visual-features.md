# JAWS's Helper Dialogs, Visual Tracking and Cursor Routing

JAWS users rely on more than reading keys. They open lists of links, headings and fields, search
with JAWS Find, mark places, change settings in Quick Settings, and route the mouse and the
virtual cursor to each other. Sighted helpers follow a red rectangle that JAWS draws around
what it reads. This page records what these features look like and do in JAWS 2025, as the
basis for Okur's own ([`helper-ui-architecture.md`](helper-ui-architecture.md) decides how Okur
builds them).

**Sources.**
- Freedom Scientific's *Surf's Up* web guide, every chapter, read on 2026-10-17. It is summarized
  here, never copied.
- JAWS's key map (`Default.JKM`, `Chrome.jkm`), for keys and script names.
- Live observation of JAWS 2025.2505.43 with Chrome on the dev box, 2026-10-17, on the guide's
  sample pages. These are research, not evidence: each needs an old-pc scenario before a test
  may rely on it.

**How the dialogs were captured.** From the sample page, with the virtual cursor at the top:
send the keys (`okur keys`), or ask JAWS to run the script by name when a layered key would not
take (FSAPI `RunScript`). Then save a picture of the window, JAWS's speech on opening, and what
`ReadBoxInTabOrder` (Insert+B) reads; close it with JAWS's own Escape (FSAPI
`TypeKey("Escape")`), since JAWS's windows refuse our keys. The pictures are in
[`jaws-dialogs/`](jaws-dialogs/).

---

## 1. What all the dialogs share

- **They are ordinary Windows dialogs.** Class `#32770`, standard controls: list views, radio
  buttons in group boxes, check boxes, combo boxes, edit fields, tree views, push buttons.
  Every label has a mnemonic (Alt+letter), one button is the default (Enter), and Escape
  cancels.
- **JAWS reads them as it reads any dialog.** On opening it says the dialog's name, the
  focused control, its position and the usage hint. Insert+B reads the dialog, Insert+E the
  default button, Insert+F1 helps on a control. Nothing about them is special-cased speech.
- **They are real windows.** A sighted helper sees them; the mouse works in them; the focus
  is in them, so a stray key lands in the dialog, not in the page behind. Settings Center
  stays open in the Alt+Tab list while the user returns to the page.
- **Owner and integrity.** The list dialogs belong to `jfw.exe` itself (the Links List: owner
  `jfw.exe`, extended style `0x10109`, so they are topmost). JAWS runs elevated, so its
  dialogs refuse input from normal-integrity processes and show only their title to a
  normal-integrity UI Automation client (`okur window-text` on the open Links List returned
  "Links List" and nothing else). The Virtual Viewer is a separate process, `VIRTVIEW.EXE`.
- **Content comes from the virtual buffer.** A list shows the page as JAWS's virtual cursor
  sees it, custom labels included, when the key is pressed.
- **The first item selected** is the first in the list, except the graphics list, which
  selects the graphic nearest the reading position (guide, chapter 5).
- **An empty list opens no dialog.** JAWS says a message instead: "no form fields were found",
  "No regions were found on the page", "There are no PlaceMarkers on this page." (K), "There
  are no Check boxes on this page." (Shift+X), "A Custom Label cannot be created for this
  item."
- **Opening speech varies with timing.** When focus moves into JAWS's own process, JAWS
  sometimes first says "JAWS Version 2025.2505.43" and the page's window title, and sometimes
  says the focused list before the dialog's name. Compare the dialog's lines, not their order.

## 2. The dialogs

Speech on opening is as JAWS's speech history recorded it, joined with " | ". "Insert" stands
for the JAWS key.

### Lists of page elements

| Keys | Script | Title | Controls in tab order | Default | Speech on opening |
|---|---|---|---|---|---|
| Insert+F7 | `SelectALink` | Links List | list view "links"; Move To Link, Activate Link, Cancel; Sort Links (In Tab Order, Alphabetically); Display (All Links, Visited Links Only, Unvisited Links); the link's address in a read-only edit | Activate Link | Links List dialog \| links List view \| Reputation for Quality Service \| 1 of 13 \| To move to items use the Arrow keys. |
| Insert+F6 | `SelectAHeading` | Heading List | list view "headings" ("text : level"); Move To Heading, Cancel; Sort Headings (In Tab Order, Alphabetically); Display (All Headings, Level 1 to Level 6 Headings) | Move To Heading | Heading List dialog \| headings List view \| Jim's Cafe at Innisport : 1 \| 1 of 8 \| To move ... |
| Insert+F5 | `SelectAFormField` | Select a Form Field | list view "List1"; OK, Cancel | OK | Select a Form Field dialog \| List1 List view \| Contact Information First name. This is a required field. Edit \| 1 of 26 \| To move ... |
| Insert+F9 | `SelectAFrame` | Frames List | list view "List1"; OK, Cancel | OK | Frames List dialog \| List1 List view \| Contact Information \| 1 of 3 \| To move ... |
| Insert+Ctrl+G | `SelectAGraphic` | Select a Graphic | list view "List1"; OK, Cancel | OK | Select a Graphic dialog \| List1 List view \| Jim's Cafe at Innisport \| 1 of 4 \| To move ... |
| Insert+Ctrl+L | `SelectAList` | Select a List | list view "List1" (each list's items run together on one line); OK, Cancel | OK | Select a List dialog \| List1 List view \| (the first list) \| 1 of 3 \| To move ... |
| Insert+Ctrl+R | `SelectaRegion` | (regions list) | not seen: the sample page has no regions | | "No regions were found on the page" |
| Insert+F3 | `VirtualHTMLFeatures` | Virtual HTML Features | list view "List1" of 22 lists (Anchors, Articles, Block Quotes, Buttons, Check Boxes, Controls, Divisions, Edit Boxes, Form Fields, Frames, Graphics, Headings, Links, List Items, Lists, Paragraphs, PlaceMarkers, Radio Buttons, Regions, Smart Glance Highlight, Tables, Tabs); OK, Cancel | OK | Virtual HTML Features dialog \| List1 List view \| Anchors List \| 1 of 22 \| To move ... |

Notes:
- A form field's entry joins its group, label, description, type and value or state:
  "Ordered Products JAWS® check box not checked", "Account Information Order Number Edit 357".
  The dialog widens to its longest entry (1359 pixels wide on WebTrack).
- In the Heading List, Alt+1 to Alt+6 choose a level and Alt+L all levels (guide, chapter 5):
  the radio buttons' mnemonics.
- Enter on a form field moves there and turns forms mode on (guide, chapter 9).
- The Insert+Ctrl+letter family (`Chrome.jkm`): A radio buttons, B buttons, C combo boxes,
  E edits, G graphics, L lists, O objects, P paragraphs, Q block quotes, R regions,
  T tables, X check boxes, Y spans, Z divisions. Each is the list dialog of that quick key.
- Pictures: [links](jaws-dialogs/links.png), [headings](jaws-dialogs/headings.png),
  [frames](jaws-dialogs/frames.png), [graphics](jaws-dialogs/graphics.png),
  [HTML features](jaws-dialogs/htmlfeatures.png).

### PlaceMarker List (Ctrl+Shift+K, `SelectAPlaceMarker`)

- List view "PlaceMarker" with two columns, PlaceMarker and Generalized to Domain ("0 items"
  when empty).
- Buttons Add..., MoveTo, Change..., Remove, Remove All, Cancel. Only Add and Cancel are
  enabled while the list is empty.
- Display PlaceMarkers For: URL, Domain, All (All checked).
- Speech: "PlaceMarker List dialog | PlaceMarker List view | 0 items | To move to items use
  the Arrow keys."
- From the guide (chapter 15): Add is the default button until an item is selected, then
  MoveTo. Add proposes the text at the virtual cursor as the name, selected so typing replaces
  it. Moving to a marker returns to the exact character. K and Shift+K move between markers
  and wrap ("wrapping to top"). Ctrl+Windows+K sets one temporary marker per page. Markers are
  kept per URL in a file and can be shared. A page setting, Custom Page Summary, can speak the
  markers' text, or list it in the Results Viewer, when the page loads.
- [Picture](jaws-dialogs/placemarkers.png).

### JAWS Find (Ctrl+F in a virtual document, Insert+Ctrl+F elsewhere, `JAWSFind`)

- Find what: an editable combo box holding the last 20 searches (Alt+W).
- Check boxes: Context Specific History (history kept per site), Maintain History of Recent
  Finds, Ignore Case; all checked by default.
- Direction: Forward, Reverse. Buttons: Find (default), Delete, Clear Find History, Cancel.
- Speech: "JAWS Find dialog | Find what: edit combo | To set the value use the Arrow keys or
  type the value. | Alt+w".
- From the guide (chapter 13): the search starts at the virtual cursor; Enter moves to the
  match and reads its line, with the cursor at the match's first character; F3 and Shift+F3
  repeat; "wrapping to top" when the search passes the end.
  In a browser, Ctrl+F is JAWS's; Insert+3 then Ctrl+F reaches the browser's own find.
- [Picture](jaws-dialogs/find.png).

### Custom labels (Insert+Ctrl+Tab, `CreatePrompt`)

From the guide (chapter 19): a small dialog with one edit field labels the element at the
virtual cursor (links, graphics, image map areas, form fields, buttons). The label replaces
the element's name everywhere, the lists included, on every page of the domain, and is kept in
the domain's settings file. JAWS Manager has Custom Label Delete and Custom Label Delete All.
Live: on a paragraph JAWS said "A Custom Label cannot be created for this item." and opened
nothing.

### Select a Scheme (Insert+Alt+S, `SelectAScheme`)

List view "List1" of 21 speech and sound schemes, the current one selected ("Classic, 7 of
21"); OK, Cancel. [Picture](jaws-dialogs/scheme.png).

### Run JAWS Manager (Insert+F2, `RunJAWSManager`)

List view "List1" of 21 managers, opened on the first (Commands Search, Custom Highlight
Assign, Custom Label, Custom Label Delete, Custom Label Delete All, Customize ListView,
Dictionary Manager, Flexible Web, Frame Viewer, Graphics Labeler, Keyboard Manager, Mark Colors
in Braille, Message Center, Navigation Quick Keys, Notification History, Quick Settings,
Research It, Script Manager, Settings Center, Skim Reading Tool, Window Class Reassign); OK,
Cancel. Speech: "Run JAWS Manager dialog | List1 List view | Commands Search | 1 of 21 | To
move ...". [Picture](jaws-dialogs/managers.png).

### Quick Settings (Insert+V, `QuickSettings`)

- Title "QuickSettings - Chrome": the settings of the application in front, its own groups
  first (Virtual Cursor Options, Forms Options, General Options, Personalized Web Settings...),
  the general ones after.
- A search box (focused on opening, Ctrl+E) filters the tree as one types.
- The tree view's groups open and close with the arrows; Space cycles an item's value.
- A pane beside the tree shows the selected group's settings as ordinary controls (combo
  boxes, check boxes), for sighted helpers and the mouse.
- A read-only text describes the selected item; Tab reaches it.
- "Expand Tree View" check box; OK.
- Speech: "QuickSettings - Chrome dialog | SearchBox Edit | Type in text. | CTRL+E".
- From the guide (chapter 6): each item's context menu (Shift+F10) chooses how long a change
  lasts: saved (the default, shown by an icon), until JAWS exits, or until the focus moves to
  another window. On a group, the menu sets every item in it. A "Last Changed Settings" group
  keeps the last 25 changes, each removable. Personalized Web Settings apply to the site's
  domain and are kept in its settings file.
- [Picture](jaws-dialogs/quicksettings.png).

### Settings Center (Insert+6, `SettingsCenter`)

- Title "JAWS Settings Center - Chrome". An Application combo box chooses which settings are
  edited ("Chrome, 17 of 180"; Default (All Applications) at the top; Ctrl+Shift+D switches to
  it).
- Then a search box, the tree of groups (User, Web / HTML / PDFs, Forms Mode, Text Processing,
  Speech Verbosity, ... Braille, Keyboard, Custom Highlight...), a configuration pane (F6
  between tree and pane), a description, and Apply, OK, Cancel.
- Speech: "Chrome - JAWS Settings Center dialog | SearchBox Edit | Type in text. | control+E".
- Some items open their own dialogs: Configure Web Verbosity Levels opens a tree of check
  boxes, one per structural element (guide, chapter 7).
- [Picture](jaws-dialogs/settingscenter.png).

### Skim Reading (Ctrl+Shift+Insert+Down, `SkimReadDialog`)

- Use Rule Type: four radio buttons (first line or first sentence of every paragraph, text
  rules, attribute and colour rules).
- Rules History combo box (`<New>`), Text Search Pattern, colour and attribute rules (text and
  background colour combo boxes with "*" for any, nine attribute check boxes), Delete Rule,
  Delete All Rules, Speak Unit combo box, Use regular expression, Skim Reading Indication.
- Buttons Start Skim Reading, Create Summary (default), Close, Help.
- Controls that don't apply to the chosen rule type are disabled.
- From the guide (chapter 14): Create Summary lists the matching lines in the Results Viewer,
  numbered, each a link back to its place in the page. The summary stays until the next one,
  even after switching applications.
- [Picture](jaws-dialogs/skim.png).

### Flexible Web (Insert+Space then X, `DoFlexibleWebDialog`)

- A wizard (Back, Next, Cancel). The first page asks what to do: Create a new customization,
  Retry..., Undo..., Save temporary customizations as a rule, View or change where rules
  apply. Options not yet possible are disabled. An Enable Flexible Web check box sits below.
- From the guide (chapter 16): later pages choose an action (hide an element, or start reading
  at it), the element (a list of the elements around the virtual cursor) and the rule ("Hide
  all frames with text ...", "Start at heading level 2 1 from the top"). A customization holds
  until the page is left; JAWS offers to save the unsaved ones as a rule for the page, the
  site or every site. A sound plays when a page with rules loads. In the rules view, F2
  renames and Delete removes.
- [Picture](jaws-dialogs/flexweb.png).

### The Results Viewer and the Virtual Viewer

- Lists that are text (hot key help, skim reading summaries, PlaceMarker summaries, Insert+F1
  help) open in JAWS's Virtual Viewer. JAWS reads it with its virtual cursor; links in it are
  followed with Enter; Escape or Alt+F4 closes it.
- With "Show Virtual Viewer on Screen" checked (Settings Center, User group; checked on the dev
  box), the text is also shown in a window of `VIRTVIEW.EXE` (class `VIRTVIEWHELP`, title
  "Virtual Viewer"): plain text with a status bar. That window accepts normal-integrity input,
  unlike JAWS's dialogs. [Picture](jaws-dialogs/virtualviewer.png).
- Insert+H in Chrome opened the hot key help in the Virtual Viewer, shown on screen, and JAWS
  read it ("Go Back a page, ALT+LEFT ARROW. ...").

## 3. Visual tracking: the reading highlight

JAWS draws a red rectangle around what the virtual cursor last read, so a sighted person
(teacher, parent, tester) can follow along (guide, chapters 5 and 9).

- **Unit.** After a line move, the rectangle surrounds the line's text, across the screen lines
  it wraps to; after Insert+Right, the word; after Right, still the word holding the character
  (the character itself is spoken). After H, the heading; after a quick key generally, the
  element. Pictures: [word](jaws-dialogs/highlight-word.png),
  [heading](jaws-dialogs/highlight-heading.png).
- **Scrolling.** The page scrolls so the highlighted text is on screen.
- **How it is drawn.** One window of `jfw.exe` covers the whole screen: class
  `OverlayWindowClass_<GUID>`, extended style `0x0A0800A8`, that is layered, transparent to
  the mouse, topmost, a tool window, never activated, composited. The rectangle is painted into
  it; the page is not changed.
- **It stays on top.** The rectangle is drawn over JAWS's own dialogs too, and keeps the page's
  last position while a dialog is open.
- **Settings.** Quick Settings has Smart Glance Highlight Indication; the Insert+F3 list has a
  Smart Glance Highlight list. Visual tracking's own options were not explored.

## 4. Routing between the virtual cursor, the PC cursor and the mouse

Key map (`Default.JKM`, Common Keys), with JAWS's script names:

| Keys | Script | What it does (by its name, unless observed below) |
|---|---|---|
| NumPad Plus | `PCCursor` | the PC cursor (and the virtual cursor in a document) |
| NumPad Minus | `JAWSCursor` | the JAWS cursor, which is the mouse pointer |
| Insert+NumPad Plus | `RoutePCCursorToJAWS` | brings the PC or virtual cursor to the mouse pointer |
| Insert+NumPad Minus | `RouteJAWSCursorToPc` | moves the mouse pointer to the PC or virtual cursor |
| Insert+Delete | `RouteVirtualToPc` | moves the virtual cursor to the PC cursor |
| Ctrl+Insert+Delete | `RoutePCToVirtual` | moves the PC cursor to the virtual cursor |
| NumPad Slash, NumPad Star | `LeftMouseButton`, `RightMouseButton` | click at the pointer |
| Insert+NumPad Slash, Insert+NumPad Star | `LeftMouseButtonLock`, `RightMouseButtonLock` | hold a button |
| Ctrl+Insert+NumPad Slash | `DragAndDrop` | drag and drop |
| Ctrl+Insert+NumPad Minus | `TetherJAWSToPC` | the JAWS cursor follows the PC cursor |
| Alt+Shift+Arrows | `MouseUp`, `MouseDown`... | move the pointer |

Live, in Chrome, with the virtual cursor on a heading:
- Insert+NumPad Minus said "Route JAWS to PC" and moved the pointer onto the heading.
- After Ctrl+Home, Insert+NumPad Plus said "Route PC to JAWS"; Insert+Up then read that
  heading. The virtual cursor had moved to the element under the pointer.

The guide (chapter 5) suggests this to sighted mixed use: point at text with the mouse, press
Insert+NumPad Plus, read from there.

## 5. Layers

Insert+Space starts a layered keystroke: the next key chooses a layer or a command (X Flexible
Web, T the table layer, E mouse echo, O OCR, V volume, B the braille and text viewers, P
Picture Smart). A layer such as the table layer stays active until a key outside it (Escape,
Space, Tab, Enter); ? or H lists its keys (guide, chapter 8). Layers are part of key routing:
they belong in Okur's key state machine, beside menu mode and forms mode.

## 6. Where settings and marks are kept

- Settings, per application or for all (`.jcf`).
- Personalized settings and custom labels, per domain (`.jsi`).
- PlaceMarkers, per URL (`.pmi`, shareable).
- Flexible Web rules, for a page, a site or all sites.
- Find history, per site when Context Specific History is checked.
- Skim reading rules.
- Quick Settings changes, saved, or kept until JAWS exits, or until the focus changes.

## 7. Okur today

| Feature | Okur | Gap |
|---|---|---|
| Links, headings and form fields lists | speech only (`ActiveElementList`): the engine speaks a dialog that has no window; arrows, Enter and Escape | no window; Tab, mnemonics, sorting, filtering, the other buttons, first-letter moves, mouse and braille missing; keys Okur doesn't take go to the page; empty-list messages differ ("There are no form fields on this page." against JAWS's "no form fields were found") |
| Frames, graphics, lists, regions, Insert+Ctrl+letter, Insert+F3 lists | none | all |
| JAWS Find | speaks the dialog's opening line, no field (`Command::VirtualFind`); F3 and Shift+F3 search | the dialog |
| PlaceMarkers, custom labels, Flexible Web, skim reading | none | all |
| Quick Settings, Settings Center | none; settings come from the command line and JAWS's `.jcf` (`TypingEcho::from_jcf`) | all |
| Virtual Viewer | speech only, read with the reading keys | no window, so stray keys reach the application behind |
| Visual tracking | none | all |
| Routing and mouse keys | none | all |
| Layers | none | all |
