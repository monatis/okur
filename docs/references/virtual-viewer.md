# The Virtual Viewer: Virtualize Window and Virtualize Current Control

JAWS's Virtual Viewer shows what a window or a control holds as plain lines. The user reads
them with the reading keys, selects and copies them, and closes the viewer with Escape. This
page records what JAWS 2025 puts in the viewer, where it fails, and how Okur builds its own.

| Keys | Command | JAWS speaks |
|---|---|---|
| Insert+Alt+W | Virtualize Window | "Virtualizing current window", then the first line |
| Insert+Shift+V | Virtualize Current Control | "Virtualizing current control", then the first line |
| Insert+T in the viewer | Say Window Title | "Virtual Viewer" |
| Ctrl+A, Ctrl+C | select all, copy | "selected" with the text; "Copied selection to clipboard" |
| Escape | close | the focused control again |

In a web page's virtual cursor, or with the viewer already open, JAWS says "The current
location is already in a virtual view" [obs, 311/s32].

## What JAWS shows

Observed with JAWS 2025.2505.43.400 on the dev box (Windows 11), 2026-10-15, unless the
line cites old-pc evidence. Dev-box observations are research, not evidence: scenarios 322
and 323 in `corpus/pending/` will record them on old-pc.

**Classic windows** (Win32 dialogs, WinForms, File Explorer) are shown as they are drawn,
by screen rows. The title comes first. Each row reads left to right: labels beside their
values, tabs and buttons on one row, and text wrapped where the control wraps it. JAWS
captures the screen's text, so what is drawn is what it gets. Local Disk (C:) Properties:

```
Local Disk (C:) Properties
Security Previous Versions Quota
General Tools Hardware Sharing
Type: Local Disk
File system: NTFS
Used space: 440,534,675,456 bytes 410 GB
...
DetailsDrive C:
Compress this drive to save disk space
Allow files on this drive to have contents in dexed in addition to file
properties
OK Cancel Apply
```

A message box gives its title, its text wrapped as drawn, and its buttons on one line
("Yes No Cancel") [obs, 311/s14–s22].

**Modern apps** (XAML, such as Calculator) are shown one element per line in tree order,
with role and state: "Memory recall Button  unavailable", "Display controls group". An
unnamed group is a bare "group" line, and the title bar's buttons are left out.

**Virtualize Current Control** shows the control's screen text. On a WinForms button it
shows "&Save...". On a WinForms list view it shows only the last row ("photo.jpg 1.2 MB"),
twice in a row, while the first row had focus. On a tree view it lists the items with roles
("Documents tree view item"), each followed by a line with "0".

**Text**:
- Windows 11 Notepad shows "paragraph", not its text.
- Classic Notepad on Windows 10 shows its menu bar, its text and its status bar
  [obs, 314/s10–s11].
- An edit control's text is shown by its lines [obs, 312].
- Windows Terminal shows the console's text, opened at the prompt.

**WinForms on old-pc**: Virtualize Window showed only the focused control, "Save... Button"
and a blank line [obs, 311/s1, s26, s37]. The dev box shows the same fixture's whole window
by rows. JAWS falls back to the focused control when its screen capture finds no text in the
window. Scenario 322 will show whether old-pc does so for every WinForms window.

## Where JAWS fails

Its screen capture shows these faults, on the dev box unless cited:

| Fault | Example |
|---|---|
| Separate controls glued into one word | "DetailsDrive C:", "ApplesN&otes:", "DocumentsPlan Pictures" |
| Accelerator marks shown | "&Name:", "OK Cancel &Apply" |
| A symbol font's glyph shown as a letter | a line "S" for a drive's colour square |
| Letters lost or split | "in dexed", "AppDa a", "Hidd n", "3:0 4:05" |
| Text cut off at the control's edge | "C:\Users\Gigabyte\AppDa a\Local\Temp\sr-property-fi" |
| Parts of a window missing | File Explorer's address bar and command bar |
| Unnamed containers listed | "group" |
| A list as its last row | Virtualize Current Control on a list view |
| No text for a text editor | "paragraph" in Windows 11 Notepad |
| Nothing for WinForms (old-pc) | only the focused control [obs, 311/s1] |

## How Okur builds it

Okur reads the window through UI Automation when the key is pressed, on the actions worker
thread, never in the keyboard hook. The viewer opens when the text arrives.
- `okur-platform::window_text` reads the elements and where they are.
- `okur-core::viewer_text` lays them out.
- `okur window-text [--control]` prints the same lines for the window in front, for
  comparing with JAWS.

The layout:
- **Rows or elements.** Modern app hosts are listed by element: `ApplicationFrameWindow`,
  `Windows.UI.Core.CoreWindow`, `WinUIDesktopWin32WindowClass`, or a XAML, WPF or WinUI root.
  Everything else is laid out by rows.
- **Rows.**
  - Lines are grouped by screen position. A row's band is its first line's, so a tall group
    box or a wrapped text never pulls neighbouring rows into one.
  - A one-line label sits at its top, as it is drawn ("Drive C:" beside "Details").
- **Classic labels and buttons.**
  - They show their window text, not their UIA name: UIA names an unlabeled control after
    the label before it ("410 GB" for a colour square).
  - Text is wrapped where the control wraps it, measured with the control's font in the
    window's own DPI units. A message box from a DPI-unaware process wraps where JAWS shows
    it.
  - Win32 owner-drawn controls (`SS_OWNERDRAW`, `BS_OWNERDRAW`) and symbol-font controls show
    no text. WinForms paints its labels from their text, so the owner-drawn rule applies to
    exact Win32 classes only.
- **Names and parts.**
  - A control with a name shows its name, and its parts only when it has none. A column
    header shows "Name", not its "Filter dropdown" button.
  - A list row, tree item or status bar is read through its parts, so a list view's row
    shows its columns.
  - A group's name is a caption only for a group box with its own window. Explorer's "View
    Modes" only names what is inside it.
- **Title.**
  - The title bar is left out, including a modern app's title-bar window.
  - When an app draws its title again at the top (Calculator), it is read once.
- **Text clean-up.** Direction and zero-width marks are removed: Explorer wraps dates in
  left-to-right marks.
- **Fields.** A field shows its value, never a password's. A multi-line field shows its
  lines.
- **Virtualize Current Control** shows the whole control. With focus on a list's row it
  shows the list (its header and every row), and the same for a tree, tab set or menu.

What stays as before:
- A console shows its buffer, opened at the cursor's line.
- A text field's lines are shown when the text is the window's content: a document, or a
  multi-line field outside a dialog box. Windows 11 Notepad shows its text, where JAWS shows
  "paragraph". A one-line field in a form is one row of its window.
- In a page's virtual cursor, there is nothing to virtualize.
- When nothing could be read, the viewer shows what the focus says, as on old-pc: the
  control's name and type, and a blank line.
- Replay (`okur parity`) has no window text in its traces, so it uses that focus-based viewer.
  Parity is unchanged.

## Where Okur differs from JAWS on purpose

| JAWS | Okur | Why |
|---|---|---|
| "DetailsDrive C:" | "Drive C: Details" | separate controls are separate words, read left to right |
| "&Name:" | "Name:" | the mark is for the eye; JAWS speaks the access key elsewhere |
| "S" (a colour square) | nothing | a glyph in a symbol font is not text |
| "in dexed", "AppDa a" | "indexed", "AppData" | the text comes from the control, not from screen capture |
| "sr-property-fi" (cut off) | the whole path | the control's text is complete even where its window clips it |
| "group" | nothing | an unnamed container says nothing |
| last row of a list | the whole list | the control is the list |
| "paragraph" (Notepad) | the text | the user asked for the text |
| focused control only (WinForms, old-pc) | the window by rows | the dev box's JAWS shows the window; Okur reads it through UIA, whatever JAWS's capture sees |

## Opening a property sheet

When a property sheet opens, JAWS names the page. On the dev box: "Local Disk (C:)
Properties dialog", "General page", "Label: Edit", "Type in text." Okur says the same.
- The trace records the page the focus is on (`dialog_page`), because UIA leaves the page
  out of the focus chain.
- The page's labels are not read as the dialog's text. Okur used to read all of them
  ("Type:", "Local Disk", "File system:", ...).

For a file's Properties, JAWS also repeats the file name, which is selected in the name
field, between the lines. Scenario 323 will record it on old-pc.

## Speed

Time from the request to the lines on the worker, live on the dev box, from the debug log's
"viewer text read" (`okur run --debug-log`, 2026-10-15):

| Window | Lines | ms |
|---|---|---|
| Local Disk (C:) Properties | 13 | 47–51 |
| WinForms form (`desktop-forms.ps1`) | 19 | 133 |
| A tree view (Virtualize Current Control) | 2 | 12 |
| File Explorer, 3 files | 20 | 179–193 |

Each element is fetched with one cached UIA call. The walk is bounded at 2,000 elements and
skips page documents.

## Comparing with JAWS

On the dev box, only with the user's permission (verification-loop §7):
- `okur window-text` prints Okur's lines for the window in front.
- To capture JAWS's viewer, send `Insert+Alt+W`, `Ctrl+A`, `Ctrl+C`, `Escape` with
  `okur keys --allow <process> --title <title>`, saving the clipboard before and restoring
  it after.
- `okur jaws-history` shows what JAWS spoke.
- With JAWS running, run Okur with `--jaws-key scrolllock` and send `ScrollLock+Alt+W`, so
  that only Okur takes the command.

On old-pc, record scenarios 322 and 323 (`corpus/pending/`).
