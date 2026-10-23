# Plan: the Okur cursor where the screen has no text (2026-10-23)

The README's "Not yet" item "reading with the Okur cursor where the screen has no text to give
(images, applications that draw their own text)", researched at the owner's request in
[`../references/okur-cursor-screen-text-research.md`](../references/okur-cursor-screen-text-research.md)
(cited as "research §n"). The owner allowed Windows 10 and 11's own OCR, and merging this with
the OCR feature of
[`../references/assistive-ai-and-remote-proposal.md`](../references/assistive-ai-and-remote-proposal.md)
(cited as "proposal §n"). This plan replaces that proposal's V1 OCR milestone and corrects its
§5.2: JAWS puts the screen's recognized text in place, not in the Results Viewer.

## 0. Status (2026-10-24)

**The owner's decision (2026-10-23):** build it, but keep it experimental until it is proven, as
a feature of Okur's own. It is behind Okur Settings' Experimental group
(`experimental.screen_text`, off by default), and the replay runs with it off, so the parity
figures are untouched by it.

**Built:**
- `82d401e1`:
  - the line builder (`screen_model`), shared with the Virtual Viewer's rows;
  - the window's text from UI Automation, a field's lines with their places;
  - the Okur cursor's lines, routing and mouse over the model;
  - Insert+Space, O with W, S, C and Q in place, with Windows.Media.Ocr over xa11y's GDI
    capture;
  - the messages "OCR", "OCR started using Microsoft OCR.", "finished." and "Okur Cursor";
  - removal on the PC cursor and on another window.
- `b1ffeef7`: `okur-lab screen-text`.
- `fce8d727`: a field's misplaced lines; "OCR cancelled.".
- `6e3d477f`: the Alt menu mask, found on the way.

**Proven so far:**
- 11 hermetic tests (`screen_model`, `engine::screen_text`, `ocr`);
- the live run on old-pc in the research's §8: recognition, the lines, and a click on a
  recognized word that reached its button where JAWS's with OmniPage missed (1000).

**Not yet:**
- the replay's screen snapshots and recognized text (§3), which would make the proof a
  ratchet;
- Background OCR (S3), the layer's ? help, the OCR voice, `ocr.language`;
- files to the Results Viewer (S4).

Where JAWS uses its Scan Cursor (WPF, 1002), Okur's experiment reads the window by rows instead
("Left words Right words" on one line, where JAWS reads two). This is the experiment's own
choice, kept while it is experimental.

## 1. What JAWS does, in short

- **The JAWS cursor reads JAWS's model of what applications draw.** It is fed by a display
  driver on Windows 10 or by hooked drawing calls on Windows 11 (research §1).
  - Recorded on old-pc: the old driver gives GDI's painted text, but not a WinForms label, GDI+
    text, pictures or a picture button (994). The Accessibility Monitor adds the label and the
    GDI+ text (995).
- **Where an application draws nothing JAWS can catch, JAWS has two answers.**
  - The Scan Cursor, over UI Automation, in applications with no model (research §1.5).
  - Convenient OCR of the window, screen or control (Insert+Space, O, then W, S or C). It says
    "OCR started using OmniPage." or "… Microsoft OCR.", then "finished.", then "JAWS Cursor".
    The recognized words then stand among the model's text, in place, read with the JAWS cursor
    and clickable with Numpad Slash. They go when the PC cursor returns (996, 997; research §2).
- **Background OCR** (B, and Enter by context) keeps recognizing the window (research §2.3).
- **JAWS's "Microsoft OCR"** is Windows' own engine, the one Okur will use. Recordings with
  `[OCR] UseMicrosoftOcrForScreenRecognitions=1` (997) compare the same engine's text.

## 2. Okur's design

### 2.1 A screen model for the Okur cursor

A platform-neutral model in `okur-core` (`screen_model`):
- **Runs of text**, each with its rectangle in desktop pixels, its source (accessibility or
  recognized) and, for accessibility runs, the element it came from.
- **Lines** are built from the runs' geometry:
  - runs that overlap vertically form a line, left to right;
  - gaps become spaces at JAWS's 8 pixels per space, leading ones too, as JAWS's lines have
    them (research §1.2: "    Painted with GDI text out").
  - The same line builder is what Screen Layout and Smart Navigation need for a page's lines
    (round 280). It is built once, for both.
- **The Okur cursor's keys move over it**, as JAWS's do:
  - Up and Down by line; Left and Right by character; Ctrl+Left and Ctrl+Right by word;
  - Home and End; Page Up and Page Down to the restriction's top and bottom;
  - Insert+Up, Numpad 5 and Insert+Numpad 5.
  - The mouse pointer follows the cursor to the character's rectangle, so Numpad Slash and
    Numpad Star click there. These two are missing outside pages today (backlog row 28) and
    come with this milestone.
- **Routing:**
  - Insert+Numpad Minus finds the run at the PC cursor (the caret, or the focus's rectangle);
  - Insert+Numpad Plus routes the PC cursor to the Okur cursor.
- **Insert+R's restrictions** clip the model to the window, the real window, the focus or
  nothing (`Restriction`, already built).
- **The model is read when needed, not kept up to date.**
  - The actions worker builds it when the Okur cursor needs a window it does not have, or when
    events say the window changed.
  - It answers as an event, as page queries do, so the engine never waits.

### 2.2 Where its text comes from

1. **Accessibility (milestone S1).**
   - **What is read:** UI Automation's visible elements in the restricted window, cached in one
     request, with a count and time budget:
     - names of text, labels and buttons;
     - an edit's or a document's visible text ranges, one line each, with their rectangles
       (`GetVisibleRanges`, `GetBoundingRectangles`, as the console's rows are read).
   - **MSAA** where UI Automation stops: the NetUI ribbon's objects, as the tracer already reads
     them.
   - **What it gives:** text in WPF, UWP, Windows 11 Notepad, Chromium and WinForms labels, where
     JAWS's driver gives nothing. Where JAWS uses its Scan Cursor instead (1002, 1003 will say
     which), Okur's cursor takes the Scan Cursor's keys and names: "Okur Scan Cursor" for
     JAWS's "JAWS Scan Cursor" (AGENTS.md §1.5).
   - **What it misses:** text painted with no accessible name (994's panels). Recognition
     covers that on request, as below.
2. **Recognition (S2).**
   - **Engine:** Windows.Media.Ocr over a capture of the window, the screen or the focused
     control.
   - **Merging:** the recognized words go into the model as runs. A recognized word whose
     rectangle lies on accessibility text is dropped, as JAWS says nothing twice (996).
3. **Drawing hooks: not now** (decision O1 below).

### 2.3 Convenient OCR

The layer, as JAWS's key map has it (research §2.1).

| Key | Command | Okur | Milestone |
|---|---|---|---|
| W | `RecognizeRealWindow` | the real window, in place | S2 |
| S | `RecognizeScreen` | every monitor, in place | S2 |
| C | `RecognizeControl` | the focused control's rectangle, in place | S2 |
| Q | `CancelRecognition` | stops recognition and Background OCR | S2 |
| ? | `OCRLayerHelp` | the layer's keys, in the Results Viewer | S2 |
| B, Ctrl+B | `StartBackgroundRecognition`, `…WithSound` | the window again and again, in place | S3 |
| Enter, Ctrl+Enter | `OCRAllInOne`, `…WithSound` | a selected file or open PDF to the viewer, otherwise the window in place (video and shares come with their application modules) | S3, S4 |
| F | `OCRSelectedFile` | the image or PDF selected in File Explorer, to the Results Viewer | S4 |
| D | `OCRDocument` | the open PDF, to the Results Viewer | S4 |
| A, R | camera, Word | not planned (proposal §5.1) | |

**In place**, after W, S or C:
- **Messages:**
  - "OCR" for the layer key;
  - "OCR started using Microsoft OCR.", which names Windows' engine truthfully, as JAWS's
    option does;
  - "finished.", then "Okur Cursor";
  - the "no text" and "cancelled" messages are to be recorded.
- **The cursor:** the Okur cursor becomes active and unrestricted, on the first recognized
  line. Where JAWS leaves it is to be confirmed (998 left it where it was).
- **The voice:** recognized runs are spoken with an OCR voice, an alias of the Okur cursor's
  voice, as JAWS's `OCRVoice` is. Until Okur has voice aliases, it is the Okur cursor's voice
  with a pitch change.
- **Removal:** the recognized runs go when the PC cursor is activated or another window comes
  to the front. They stay across switches between the Okur and Invisible cursors.
- **Clicking:** Numpad Slash on a recognized word clicks its centre (1000 will show JAWS's
  click).

### 2.4 Background OCR (S3)

- **Repeats:** the window is recognized again at an interval, which 1001 will show. Each time,
  it re-recognizes only when the captured pixels changed (a hash), and replaces the recognized
  runs.
- **Speaking:** whether new text is spoken by itself, Alt+Delete's report and the clicking sound
  of Ctrl+B are all as 1001 records them.
- **It stops** on Q, the PC or Virtual PC cursor, a page reload, or another window taking the
  focus (research §2.3).

### 2.5 The platform (`okur-platform`, Windows; macOS with the port)

- **`ScreenCapture`:**
  - a rectangle of the desktop through the screen's device context (`BitBlt`), per-monitor DPI
    aware (old-pc runs at 125%);
  - Windows Graphics Capture later, when Okur has a screen curtain (JAWS's Screen Shade; Okur
    has none yet).
  - A window that excludes itself from capture gives nothing: Okur says JAWS's "no text"
    message.
- **`TextRecognizer` for Windows.Media.Ocr:**
  - a BGRA bitmap in;
  - lines of words with rectangles mapped back to desktop pixels out;
  - pictures under 100 pixels on a side enlarged four times (NVDA's practice, for C on small
    controls);
  - `MaxImageDimension` respected by scaling.
  - Features `Media_Ocr`, `Graphics_Imaging` and `Globalization` of `windows` 0.62.
  - It runs on its own thread: 184 ms for 1280×720 on the dev box (research §5).
- **The accessibility screen source:** the UI Automation walk of §2.2.1, on the actions worker.
- **Mocks** for all three, so `cargo test` stays hermetic.
- The crates follow proposal §4.2: the engine traits in a platform-neutral crate
  (`okur-vision`, or `okur-core` until a second engine exists). Picture Smart later reuses the
  capture, the guard and the Results Viewer.

### 2.6 Settings and messages

- **`ocr.language`:**
  - the default is Okur's speech language when an OCR pack has it, else the system's, else
    English;
  - it is imported from `[OCR] MicrosoftOcrLanguage`, a locale number;
  - a missing language is said, with Windows' page that adds it (proposal §5.3).
- **`[OCR] UseMicrosoftOcrForScreenRecognitions`** has no Okur setting while Windows' engine is
  the only one. Recordings set it to 1.
- **The customization map** gains the OCR group's rows. The messages go into
  `recognition.ftl` (proposal §11).

## 3. Verification

- **Recordings** (each 3 trials):
  - 994-998, imported;
  - 999-1003 queued (round 282): OCR of a control routed and clicked, clicking recognized text,
    Background OCR, the JAWS cursor in WPF with each capture method;
  - the S0 list below.
- **Parity** compares Okur with JAWS's Microsoft OCR recordings (997 and its kind). OmniPage's
  recordings (996) keep the messages and cursor behavior; their text is another engine's.
- **Hermetic replay:**
  - **Screen snapshots:** a scenario marked `"trace_screen": true` has the tracer record the
    window's accessibility runs after each step. The replay answers the engine's screen-model
    reads from them, as it answers text reads now.
  - **Recognized text:** a scenario marked `"trace_ocr": true` also has the tracer capture the
    window and run Windows.Media.Ocr on old-pc. Its lines, words and rectangles go into the
    trace, and the replay's recognizer answers with them. This needs no pictures in the
    repository.
  - **Privacy:** what is captured is the fixture's window. The privacy check before import
    (memory) also reads every recognized line, as round 281's import did.
- **Unit tests:**
  - lines from runs: overlap, order, gaps as spaces, a recognized word over accessibility text;
  - the Okur cursor's keys over a model;
  - removal on the PC cursor;
  - mock recognizers.
- **Live check on the dev box** (Windows 11, with the owner's permission each time): the
  installed okur-sr's W in Windows 11 Notepad and in a picture-only installer page.

## 4. Milestones, in order

- **S0, observe** (old-pc, between other rounds), after round 282:
  - the Scan Cursor in Calculator and Settings (a fixture that starts them);
  - a picture button of 10 to 64 pixels at each `[OSM] IncludeGraphics` value ("graphic" and
    its number);
  - OCR under Screen Shade;
  - W over a window with no text at all (the "no text" message);
  - Q during a full-screen recognition (the "cancelled" message);
  - whether the recorder's per-utterance parameters show the OCR voice.
- **S1, the screen model from accessibility:**
  - the line builder;
  - the Okur cursor's keys and routing over it;
  - Numpad Slash and Numpad Star as clicks;
  - Scan Cursor naming where JAWS uses it;
  - the tracer's screen snapshots.
  - Proof: 994, 995, 1002 and 1003's steps that need no OCR, and 690, 752 and 757 kept.
- **S2, Convenient OCR in place:**
  - the capture and the recognizer;
  - W, S, C, Q and ?;
  - merging, the OCR voice, removal and clicking;
  - the tracer's recognized text.
  - Proof: 996-1000.
- **S3, Background OCR:** B, Ctrl+B, and Enter's window case; Alt+Delete. Proof: 1001.
- **S4, files to the Results Viewer:**
  - F, D and Enter's file cases;
  - Windows.Data.Pdf for a PDF's pages (proposal V2).
- **Later:**
  - Picture Smart and the local models (proposal V3 onward);
  - labels for small graphics by checksum, with the Graphics Labeler;
  - YouTube, Teams and Zoom areas in their application modules.

## 5. Decisions taken without the owner (reversible)

- **O1, no drawing hooks now.** An NVDA-style display model hooks GDI's calls by injecting code
  into every process, with 32- and 64-bit helpers.
  - Against it:
    - Defender already flags `okur.exe` (memory);
    - Microsoft ended display-driver capture for assistive technology;
    - such hooks miss DirectWrite, Direct2D and GDI+, which are where applications are going
      (research §1, §4.1).
  - OCR covers what is visible, on request.
  - **Revisit** if recordings show everyday applications whose text only JAWS's model has, and
    OCR reads them too slowly or badly.
- **O2, no automatic OCR.** The Okur cursor does not recognize the screen by itself where it
  finds no text, since JAWS does not. A later Okur addition could offer it as a setting, off by
  default.
- **O3, Windows' engine only** until the local models of proposal V3. Okur names it as JAWS's
  option does: "Microsoft OCR".

## 6. Risks

- **Two accessibility walks.** The UI Automation walk over a large window (a page in Chrome)
  can be slow. The budget, the cache request and reading only the visible part keep it bounded;
  `okur-lab latency` measures it.
- **Pixels vs. DPI.** Desktop pixels and DPI scaling: rectangles are per-monitor DPI aware
  throughout, tested at old-pc's 125%.
- **Windows OCR languages.** Each language needs Windows' pack, and installing one needs an
  administrator. Okur says what is missing and does not install anything itself.
