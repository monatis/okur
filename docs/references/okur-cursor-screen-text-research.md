# The Okur cursor where the screen has no text: research

**Status:** research, 2026-10-23, for the README's "Not yet" item "reading with the Okur cursor
where the screen has no text to give (images, applications that draw their own text)". The
plan built on it is
[`../plans/2026-10-23-okur-cursor-screen-text.md`](../plans/2026-10-23-okur-cursor-screen-text.md).
It corrects one point of [`assistive-ai-and-remote-research.md`](assistive-ai-and-remote-research.md)
§3.2 and [`assistive-ai-and-remote-proposal.md`](assistive-ai-and-remote-proposal.md) §5.2 (§3
below).

**Clean room.** What this page says about JAWS comes from the following, as AGENTS.md §1.4
allows. No script source, compiled script or other JAWS program file was opened. NVDA is
GPL-2.0: its source was read to learn how it uses the operating system's interfaces, and
nothing of it is copied.
- JAWS 2025's installed help (`JFW.chm`, `SettingsCenter.chm`, decompiled with `hh.exe` in the
  session scratchpad, not in the repository).
- Its key map (`Default.JKM`).
- Its shipped settings file (`DEFAULT.JCF`, its `[OSM]` and `[OCR]` sections).
- Freedom Scientific's public What's New pages, FSCast transcripts, and users on mailing
  lists and forums.
- Recordings on old-pc (994-997, round 281).

**Tags.**
- **[help]**: JAWS 2025's installed help.
- **[jcf]**: the shipped `DEFAULT.JCF`.
- **[jkm]**: the key map.
- **[wn N]**: Freedom Scientific's What's New for version N, at
  `support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=N`.
- **[staff]**: Freedom Scientific, NV Access or a vendor speaking.
- **[user]**: a user's report.
- **[code]**: NVDA's source.
- **[obs]**: recorded on old-pc.
- **[measured]**: measured on the dev box.
- **[inf]**: this writer's reasoning.

## Summary

1. **JAWS's cursor reads a model of what applications draw**, the off-screen model (OSM), not
   the accessibility interfaces.
   - Two capture methods feed it, chosen by `[OSM] HookingMode` [jcf], [help]:
     - 0: the old Accessibility Driver, the default on Windows 10.
     - 1: the Accessibility Monitor, which watches drawing calls; the default on Windows 11.
   - Where an application draws with neither method's calls, the JAWS cursor reads nothing
     ("blank"). This happens in Chromium, WPF, UWP, DirectWrite, Qt, Java, games and video.
2. **JAWS has two answers where the model is empty.**
   - **The Scan Cursor** (JAWS 2020): it takes over the JAWS cursor's keys in applications that
     give no OSM, and walks UI Automation elements [wn 2020], [staff].
   - **Convenient OCR on the screen** (Insert+Space, O, then W, S or C; JAWS 13): the recognized
     words go into the screen model where the picture is. The JAWS cursor is activated, reads
     them among the screen's other text in an OCR voice, and can click them. They go away when
     the PC cursor returns or the window changes [help], [wn 13], [obs 996, 997].
   - **Background OCR** (JAWS 2025, O then B or Enter) recognizes the window again and again for
     the same cursor [help], [wn 2025].
3. **Recorded on old-pc** (round 281; the fixture's six rows are a GDI label, a GDI+ painted
   text, a GDI painted text, a picture of text, a button showing a picture of a word, and an
   edit):
   - **The old driver** gives the title, the GDI painted text and the edit [obs 994].
   - **The Accessibility Monitor** also gives the label and the GDI+ text [obs 995].
   - **Neither** gives the picture or the button.
   - **OCR of the window**, with either engine, adds "Picture of text forty two" and "Start" in
     their places, among the lines the model already had [obs 996, 997].
4. **For Okur.**
   - **Windows' own OCR** (`Windows.Media.Ocr`) is on every Windows 10 and 11 machine. On
     old-pc it has English and Turkish, with images up to 10,000 pixels a side [obs].
   - **It is fast:** 184 ms for a 1280×720 picture on the dev box [measured].
   - **JAWS's "Use Microsoft OCR for screen recognitions" uses it too**
     (`[OCR] UseMicrosoftOcrForScreenRecognitions`), so recordings made with that option give
     the same engine's text to compare with [jcf], [obs 997].
   - **An Okur screen model** can be built from three sources:
     - UI Automation and MSAA, which Okur reads already;
     - OCR in place, which is JAWS's own answer;
     - GDI hooks, NVDA's way, which needs code injected into every process: the costliest, and
       the one JAWS itself is moving away from.

## 1. JAWS's off-screen model

### 1.1 How it is captured

- **The driver era** [wn 11], [wn 14]: until Windows 7, a mirror display driver ("Video
  Intercept"). Windows 8 ended mirror drivers. Microsoft gave assistive technology an
  accessibility display driver model instead, which it later stopped recommending
  (learn.microsoft.com, "Mirror drivers").
- **JAWS 14** said that on Windows 8 the JAWS and Invisible cursors lost unrestricted mode and
  cannot read title bars [wn 14].
- **The Accessibility Monitor** (JAWS 2018) [help, Settings Center: Miscellaneous], [wn 2018]:
  - Settings Center's "Use Accessibility Driver for Screen Capture" chooses between the driver
    and a newer "API Accessibility Monitor".
  - The help says the driver "no longer works in many cases".
  - It is on by default on Windows 10 and off on Windows 11.
  - The setting is `[OSM] HookingMode` (0 the driver, 1 the monitor), shown only when
    `AllowAccessibilityMonitor=1` [jcf].
  - A keystroke can be assigned to the `ToggleScreenCaptureMethod` script [help].
  - Freedom Scientific does not say how the monitor works. Its name, and the end of driver
    support, suggest the drawing calls are intercepted in user mode, as NVDA does [inf].
- **Windows 11** [user, reseller knowledge bases]:
  - JAWS 2022 and later do not install the driver there.
  - Microsoft ended driver support in 24H2.
  - A JAWS 2026 user on Windows 11 found no such option at all (jfw.groups.io topic 119971224,
    2026-06).

### 1.2 What the model holds

`DEFAULT.JCF`'s `[OSM]` comments describe more than text [jcf]:
- **Graphics** of 10 to 64 pixels a side, identified by a checksum (`GraphicMinX` to
  `GraphicMaxY`, `GraphicsMode`).
  - `IncludeGraphics` (0 none, 1 labeled only, the default, 2 all) chooses which are spoken.
  - An unlabeled one is "graphic" with a number [help, Graphics Labeler].
- **Carets** found from drawn lines of given sizes (`VertCaret*`, `HorizCaret*`,
  `CaretTimeOut`).
- **Focus rectangles** (`TrackFocusRect`), and line segments when `IncludeLines` is on.
- **Blank space made into spaces:** 8 pixels per space (`PixelsPerSpace`), 13 per blank line
  (`PixelsPerBlankLine`).
  - The JAWS cursor's lines hold leading spaces where text starts further right: "    Painted
    with GDI text out" [obs 996/s9].
- **Where a click lands:** offsets from the JAWS cursor (`MouseClickFudgeX`, `-Y`).
- **When text is new:** text written to the screen is "new text" for Screen Echo
  (`TextOutDelay`).

### 1.3 What the model gets, recorded

The fixture `desktop-drawn-text.ps1`, read with Insert+Numpad Minus, Numpad Minus, Page Up,
Insert+Up and Down (994, 995):

| Row | How it reaches a screen reader | Driver (994) | Monitor (995) |
|---|---|---|---|
| Window title | the window's text | "SR Drawn Text Fixture" | same |
| Label | GDI text, and a name in UI Automation | not read | "Label drawn by GDI" |
| Panel | GDI+ `DrawString`, no name | not read | "Painted with GDI plus" |
| Panel | GDI `DrawText`, no name | "Painted with GDI text out" | same |
| Picture | pixels | not read | not read |
| Button | a picture of "Start", no name | not read | not read |
| Edit | a Win32 edit | "Focus here" | same |

- Down past the last line says it again ("Focus here").
- With the monitor, one line came as "Pain nted with GDI plus" (995/s7), a model caught while
  the panel was drawing [inf].

Earlier recordings agree:
- In a WinForms dialog the JAWS cursor read nothing at the Save button (516/s4-s6).
- In Windows 10 Notepad it read lines by Down but said nothing for Right and Left
  (690/s7-s9, 752/s9-s10).

### 1.4 Where the model is empty

From users and Freedom Scientific; each claim is to be recorded before it is built on.

| Application kind | JAWS cursor |
|---|---|
| UWP/XAML (Calculator, Settings) | "blank" until JAWS 2020; since then the Scan Cursor [wn 2020], [user, jfw topic 71134211, 2020-02] |
| Windows 11 Notepad (DirectWrite) | "blank" on its status bar; only the Touch Cursor reads it [user, jfw topic 90636443, 2022-04] |
| Qt (VLC) | Scan Cursor; unlabeled buttons only with the Touch Cursor [user, same topic] |
| Java Swing | not available; JAWS 2024's early option exposes Swing to the Touch Cursor [wn 2024], [user] |
| Chromium, Electron, Gecko | the virtual cursor and accessibility interfaces instead; Mozilla's staff said in 2015 that graphics scraping no longer works [staff, Mozilla bug 1181156] |
| Windows Terminal | reaches text scrolled off screen, so it reads UI Automation's text there [user, jfw topic 120025211, 2026-07], [inf] |
| Games, video, shared screens | OCR (§2) |

- **The model learns text only when a window paints.** Users hear "blank" in logging
  programs until they restore the window or switch away and back [user, jfw topic 114474995,
  2025-08]. NVDA forces a repaint before reviewing (§4.1).

### 1.5 The Scan Cursor and the Touch Cursor

- **The Scan Cursor** (JAWS 2020) [wn 2020], [staff, FSCast 175 and 177, 2019]:
  - JAWS uses it by itself when the application gives no OSM. It says "JAWS Scan Cursor", or
    "Invisible Scan Cursor".
  - Each UI Automation element is a line:
    - Left and Right move by character; Ctrl with them, by word.
    - Up and Down move by element; Home and End go to an element's ends; Ctrl+Home and Ctrl+End
      to the first and last element.
    - Insert+Up reads the element.
  - The mouse follows it, so Numpad Slash clicks.
  - No setting turns it on or off [user, jfw topic 71134211].
  - A December 2025 report says it reads only "blank" in two applications under JAWS 2025 and
    2026, where 2024 worked [user, AppleVis].
- **The Touch Cursor** (JAWS 15; Shift+Numpad Plus) [help, Using the Touch Cursor]:
  - It moves through UI Automation's elements: Left and Right in order, Up and Down by a type
    chosen with Page Up and Page Down.
  - It has its own quick keys (Insert+Z), and a routing key (Insert+Numpad Plus).
  - It is not the JAWS cursor and does not move the focus.

## 2. JAWS: recognizing the screen

### 2.1 The layer

Insert+Space, O, then one of these keys [jkm], [help]:

| Key | Command | Recognizes | Result |
|---|---|---|---|
| W | `RecognizeRealWindow` | the window in front | in place, JAWS cursor |
| S | `RecognizeScreen` | the whole screen | in place, JAWS cursor |
| C | `RecognizeControl` | the focused control | in place, JAWS cursor |
| B | `StartBackgroundRecognition` | the window, again and again | in place, JAWS or Invisible cursor |
| Ctrl+B | `StartBackgroundRecognitionWithSound` | as B, with a soft click while it works | as B |
| Enter | `OCRAllInOne` | by context: a selected file, an open PDF, YouTube's video or a Teams or Zoom share (Background OCR limited to it, the Invisible cursor put there), otherwise the window | as its target's |
| Ctrl+Enter | `OCRAllInOneWithSound` | as Enter, with the click | |
| Q | `CancelRecognition` | stops recognition, and Background OCR | |
| D | `OCRDocument` | the PDF open in Adobe Reader | Results Viewer |
| F | `OCRSelectedFile` | the image or PDF selected in File Explorer | Results Viewer |
| A | `RecognizeCamera` | a camera or scanner | Results Viewer |
| R | `OCRSelectedFileToWord` | the selected file, into Word | Word |
| ? | `OCRLayerHelp` | the layer's keys | |

### 2.2 Results in place

The help, for W, S and C [help, Convenient OCR]:
- The text is placed where the picture is.
- The JAWS cursor is activated.
- Recognized text is read in a different voice.
- Activating the PC cursor, or moving to another application or dialog, removes it.
- The help's example is an installer's welcome screen: recognize it, then click its buttons
  with the JAWS cursor and Numpad Slash.

Users have done exactly that: in an installer, Skype's "Quit", Quick Assist (jfw topics
5823912 2017, 24942125 2018, 91677276 2022) [user].

Recorded (996 with OmniPage, 997 with Microsoft OCR; three trials each):
- **The layer key O** says "OCR".
- **W** says "OCR started using OmniPage." (or "… Microsoft OCR."), then "finished.", then
  "JAWS Cursor".
- **Alt+Delete** then says "JAWSCursor | x, y | Unrestricted": the cursor without a space, and
  no restriction left. The JAWS cursor's own Alt+Delete says "JAWS Cursor active | x, y |
  Application window restriction" (994/s4).
- **The lines** are the window's, recognized text and model text together, in screen order:
  - "SR Drawn Text Fixture" (with Microsoft OCR, "SR Drawn Text Fixture x", its close button
    read as a letter);
  - "Label drawn by GDI", "Painted with GDI plus", "Painted with GDI text out";
  - "Picture of text forty two", "Start";
  - "Focus here".
  - Recognized lines start with one space; model lines with their leading spaces.
  - Text the model had is not said twice.
- **After the PC cursor and the JAWS cursor again** (996/s15-s19), Page Up and Down say
  nothing.
- **Trials differ**: one trial's window recognition read "Windows PowerShell" from a window
  behind the fixture (996/s6 variant). The lines then start one row earlier.

The voice is the `OCRVoice` alias [wn 13]. The speech history does not show voices; the
recorder's per-utterance parameters may (to check).

### 2.3 Background OCR

JAWS 2025, December 2024 update [help], [wn 2025]:
- **Recognition repeats** in the window. The JAWS or Invisible cursor reads the text as it
  updates; braille follows a line live.
- **Alt+Delete** reports it after the cursor's own information.
- **It stops** on O then Q, on the PC or Virtual PC cursor, when a page reloads or a new video
  loads, or when another window takes the focus.
- **For video and screen shares,** Enter limits it to the video area or the shared screen and
  restricts the Invisible cursor there.
- **Not documented:** its refresh interval, and whether new text is spoken by itself.

### 2.4 Engines and languages

- **OmniPage** (Kofax, formerly Nuance) is the default [help, What's New 2025], [wn 2020].
- **Microsoft's OCR** is chosen for C, S and W with
  `[OCR] UseMicrosoftOcrForScreenRecognitions=1` [jcf], [help]:
  - its language is `MicrosoftOcrLanguage`, a locale number (1033);
  - the languages offered are those of Windows' Preferred Languages [help].
- **OmniPage's languages** are `PrimaryRecognitionLanguage` and `SecondaryRecognitionLanguage`
  [jcf].
- **Accuracy:** users find the two engines about even, depending on the document [user, jfw
  topic 92189429, 2022-07].

### 2.5 Graphics and descriptions

- **Unlabeled graphics:** "Graphic" and a number; labels are made by hand or by the Auto
  Graphics Labeler (Ctrl+Insert+G, from tooltips) [help].
- **Graphics verbosity** speaks labeled graphics only by default [jcf `IncludeGraphics=1`],
  [wn 2021].
- **JAWS 2026's Insert+G** is the AI Labeler; the old labeler moved to Alt+Insert+G [wn 2026].
- **Picture Smart describes pictures; it does not place text.** Since 2025 it refuses while
  Screen Shade is on [help]. See
  [`assistive-ai-and-remote-research.md`](assistive-ai-and-remote-research.md) §4.

## 3. Correction to the assistive AI research and proposal

Research §3.2 and proposal §5.2 send OCR results to the Results Viewer, with in-place text for
B only. For W, S and C, JAWS 2025's help and the recordings (996, 997) put the text in place,
read with the JAWS cursor. The Results Viewer is for files, PDFs and the camera. The plan
follows JAWS.

## 4. NVDA, for the interfaces

### 4.1 Its display model

- **Hooks** (`nvdaHelper/remote/gdiHooks.cpp`) [code], run inside every process with
  Microsoft Detours:
  - text: `TextOut`, `ExtTextOut`, `PolyTextOut`, Uniscribe's `ScriptStringOut` and
    `ScriptTextOut`;
  - pictures, so text drawn off screen and copied in is kept: `BitBlt`, `StretchBlt`,
    `GdiTransparentBlt`, `PatBlt`;
  - painting and windows: `FillRect`, `DrawFocusRect`, `BeginPaint`, `ScrollWindow`, device
    contexts.
  - Direct2D, DirectWrite, GDI+ and Direct3D are not hooked.
- **The model is kept per window** (`displayModel.cpp`).
- **Read through** `source/displayModel.py`: gaps of 8 pixels become word separators, and of 32
  a line's.
- **Screen review forces a repaint first** (`RedrawWindow` with invalidate and update now),
  because the model may be stale.
- **The user guide warns** that newer applications draw text NVDA cannot see.
- **It needs no driver**, so it works on Windows 8 and later.

### 4.2 Its Windows OCR

Files: `source/contentRecog/uwpOcr.py`, `recogUi.py`, `_wgcCapture.py`, `screenBitmap.py`;
`nvdaHelper/localWin10/uwpOcr.cpp`, `wgcCapture.cpp` [code].
- **Capture:** the screen's device context with `StretchBlt`. When the screen curtain or the
  magnifier is on, Windows Graphics Capture, one capture per monitor, cropped and scaled.
- **Size:** pictures under 100 pixels on a side are enlarged four times.
- **Language:** the chosen one; else NVDA's language when an OCR pack has it; else its primary
  subtag; else the first installed.
- **The result is a document** of lines, each word keeping its rectangle:
  - Enter or Space clicks a word, when its window is still in front;
  - Escape closes it;
  - refreshing is optional (every 1.5 s, since 2023.3; NVDA+F5 since 2025.1).
- **Its messages** include "Recognizing", "Content is not visible" and "Recognition failed".

## 5. Windows.Media.Ocr

- **`OcrEngine`** (Windows 10 build 10240 and later) [learn.microsoft.com, OcrEngine]:
  - creation: `TryCreateFromUserProfileLanguages`, which returns null when no profile language
    has an OCR pack, and `TryCreateFromLanguage`;
  - languages: `AvailableRecognizerLanguages`, `IsLanguageSupported`;
  - limits and use: `MaxImageDimension`, `RecognizeAsync(SoftwareBitmap)`.
- **The result:** lines of words, each with its `BoundingRect`, and a text angle. It gives no
  confidence and no paragraphs.
- **Languages** are Windows capabilities named `Language.OCR~~~<tag>~0.0.1.0`, installed by an
  administrator.
- **Packaging:** it works from unpackaged programs (NVDA; a Windows PowerShell run on old-pc and
  the dev box).
- **Capture:**
  - The screen's device context (`BitBlt`) is the simplest.
  - Windows Graphics Capture works under a magnification screen curtain, where the device
    context gives black. An unpackaged program's capture may show a yellow border [inf].
  - A window with display affinity `WDA_MONITOR` is captured empty;
    `WDA_EXCLUDEFROMCAPTURE` is captured absent (SetWindowDisplayAffinity).
- **Timing on the dev box** (i5-12500H, warm engine, Turkish, dense text) [measured]:

  | Picture | Time | Words |
  |---|---|---|
  | 600×400 | 57 ms | 126 |
  | 1280×720 | 184 ms | 323 |
  | 1920×1080 | 359 ms | 487 |

- **Rust, `windows` 0.62:** features `Media_Ocr`, `Graphics_Imaging` and `Globalization`. For
  Windows Graphics Capture also `Graphics_Capture`, `Graphics_DirectX_Direct3D11` and the
  Direct3D and DXGI ones.
- **Windows AI's `TextRecognizer`** is for Copilot+ PCs with an NPU and the Windows App SDK.
  It is an extra, not a base.

## 6. Other screen readers

- **SuperNova** (Dolphin) has a review cursor that turns on by itself where text cannot be
  read, and OCR of files and the clipboard. The screen reaches OCR through the clipboard. It
  has no OCR in place [Dolphin's help, SuperNova 25].
- **Narrator** describes images (Narrator+Ctrl+D), online or on Copilot+ PCs. It has no OCR in
  place [support.microsoft.com].
- **ZDSR** users report an OCR mode with clicking (paid). There is no English documentation
  [user, AppleVis 2025-02].
- **No screen reader documents hooking DirectWrite.** Chromium's glyph runs carry no strings,
  so text would have to be rebuilt from glyph indices [inf].

## 7. Rounds 282 and 283

Recorded on old-pc, three trials each.

- **WPF gets the Scan Cursor** (1002, 1003):
  - Numpad Minus says "JAWS scan cursor", and Insert+Numpad Minus says "Route JAWS scan to
    PC", with either capture method.
  - Each element is a line, in the window's order: "System" (the title bar's menu), "WPF
    text block", "Left words", "Right words" (two lines, though side by side), "WPF
    ‹Button›".
  - Right moves by character in an element ("i", "g"), and Ctrl+Right by word ("words").
- **C on a control** (998, 999):
  - It says the same three messages as W.
  - It leaves the JAWS cursor where it was. Routed to the PC cursor afterwards, the cursor's
    line said nothing.
- **Clicking recognized text** (1000, 1005):
  - With OmniPage, Numpad Slash on the recognized "Start" said "left mouse button", and a
    PowerShell window came to the front: the click missed the fixture's button.
  - With Microsoft's engine, the same click reached the button: "SR Drawn Text Fixture -
    started | Button".
  - OmniPage's miss is an accident, not a behavior: Okur's clicks follow the recognized
    word's physical pixels.
- **Background OCR** (1001, 1004):
  - B says "Background OCR started".
  - Alt+Delete adds "Background OCR is enabled" after the cursor's place, here "edit cursor
    | 1, 1 | 9% | Background OCR is enabled" for the PC cursor in an edit.
  - Nothing is said unasked while the picture changes.
  - The counter's recognized line stayed "Count 9" (or "Count 11") over ten seconds while
    the picture counted on every two: JAWS's refresh is much slower than the picture, or
    keeps the line the cursor is on.
  - Q says "Background OCR cancelled".
- **Cancelling** (1006): S, then Insert+Space, O, Q while it works, says "OCR started using
  OmniPage.", "OCR", "OCR cancelled.".

## 8. Okur's experiment on old-pc

Built behind Okur Settings' Experimental group (`experimental.screen_text`, off by default),
then run on old-pc with `okur-lab screen-text` and the lab Okur (research, never evidence;
the probe `screen-probe.ps1` in the lab's Dropbox, 2026-10-24).

- **`okur-lab screen-text`** over the drawn-text fixture:
  - UI Automation's text alone gives "Label drawn by GDI" and "Focus here", in 61 to 80 ms.
  - With the window recognized (101 to 133 ms), the lines are those of JAWS's Microsoft OCR
    (997):
    - "SR Drawn Text Fixture x";
    - "Label drawn by GDI";
    - "Painted with GDI Plus";
    - "Painted with GDI text out";
    - "Picture of text forty two";
    - "Start";
    - "Focus here".
  - Each word kept its place, and nothing was read twice.
- **WPF:** UI Automation placed the text box's line 40 pixels above the text box, over the
  button, so a field's lines placed outside it are now read at the field's own place. The
  window then reads "WPF text block", "Left words Right words", "WPF button", "Focus here".
  The first read, a second after the window appeared, came back empty; the next was whole.
- **The lab Okur, with the feature on**, keys sent by `okur-lab keys`, JAWS stopped:
  - Before recognition: "Route Okur to PC", "Okur Cursor", then the title, "Label drawn by
    GDI" and "Focus here" by Page Up, Insert+Up and Down, and "F", "o" by Right.
  - Recognition: "OCR", "OCR started using Microsoft OCR.", "finished.", "Okur Cursor". Then
    the seven lines above by Down, and "Okur Cursor active | 604, 714 | Unrestricted" for
    Alt+Delete.
  - Numpad Slash on "Start" clicked the button: "left mouse button", and the fixture's title
    became "SR Drawn Text Fixture - started".
  - After Numpad Plus and Numpad Minus, the window's own lines came back without the
    recognized words.
- **A bug found on the way:** Alt+Delete opened the fixture's system menu. Okur's hook took
  Delete after the application had seen Alt's press. The hook now taps an unassigned key
  (vkE8) to the application when it takes a key with Alt or Windows held (`6e3d477f`).

## 9. Open, to record

1. **The OCR voice:** whether the recorder's per-utterance parameters show it.
2. **The Scan Cursor elsewhere:** Calculator, Settings, Windows 11 Notepad, Chrome; what it says
   and how its keys move.
3. **Graphics:** the JAWS cursor over small picture buttons at each `IncludeGraphics` value,
   "graphic" and its number (1007-1009, round 284).
4. **OCR under Screen Shade** (Insert+Space, F11): a message, or nothing recognized.
5. **Title bars:** the JAWS cursor read the fixture's title (994/s6), which JAWS 14 said it
   cannot. Restriction and window kinds to check.
6. **Background OCR's refresh:** how often, and whether the line the cursor is on is kept.
7. **Time** from W to the first recognized line, on a full-screen window.
