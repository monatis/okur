# Assistive AI and remote access: research

**Status:** research only, 2026-10-18. Nothing here is built. The design built on it, for
approval, is [`assistive-ai-and-remote-proposal.md`](assistive-ai-and-remote-proposal.md);
"proposal §n" below points there. It sits beside the extension SDK pair
([`extension-sdk-research.md`](extension-sdk-research.md),
[`extension-sdk-proposal.md`](extension-sdk-proposal.md)) and uses its milestone names (M0 to
M10).

**Code read at:** `75b74fc` (branch `old-pc-round-3`). Another session had uncommitted edits
in the working tree; `path:line` references are lines of `75b74fc`.

**Clean room.** What this page says about JAWS comes from Freedom Scientific's and Vispero's
public documentation, What's New pages, training pages and release notes, from prose by third
parties, and from JAWS 2025's key map as key assignments and command names (AGENTS.md §1.4).
No script source, compiled script, script documentation, message file or other JAWS program
file was opened, Freedom Scientific's or anyone else's. NVDA is GPL-2.0: its documentation
and source were read to learn its design and protocol; nothing of it is copied here.

**How claims are backed.** Web sources are numbered and listed in [Sources](#sources): `[J…]`
JAWS and Vispero, `[N…]` NVDA and its add-ons, `[S…]` other screen readers and tools, `[M…]`
models, runtimes and operating-system OCR, `[C…]` cloud providers, `[O…]` protocols and
security. Local files are cited by path. "(excerpt)" marks a fact seen only in a search
engine's excerpt, because the site refused automated access. "(inferred)" marks the writer's
own reasoning. What could not be confirmed is listed in §13.

## Summary

**Findings.**
- **JAWS's keys are known exactly.** JAWS 2025's key map gives every key and command name of the
  OCR layer (Insert+Space, O), the Picture Smart layer (Insert+Space, P, with Alt, Shift and
  Ctrl variants), Tandem (Insert+Alt+T, Insert+Alt+Tab), the AI Labeler (Insert+G) and Vispero
  Companion (Insert+Space, F1). §2.
- **JAWS's OCR is local; its AI is not.** Convenient OCR uses OmniPage or Microsoft's OCR on the
  computer; results go to the Results Viewer. Picture Smart sends pictures through Freedom
  Scientific's service to OpenAI and Anthropic (a four-hour cache, no training); Companion,
  Page Explorer and the AI Labeler use OpenAI (Companion keeps conversations up to two years).
  In the United States the AI features need an account. §3, §4.
- **JAWS Tandem** forwards speech as text for the controller's synthesizer, through Freedom
  Scientific's server or a direct connection; JAWS's remote desktop support runs JAWS at both
  ends with a virtual channel. §6.
- **NVDA** has Windows OCR built in, withdrew its own AI captioner in January 2026, and leaves AI
  to add-ons that use the user's own keys. Its Remote Access forwards speech sequences and keys,
  but a relay can read and inject everything today; end-to-end encryption is a proposal. §7.
- **Narrator and VoiceOver** describe pictures on the device on new hardware (Copilot+ PCs,
  Apple Intelligence Macs). Windows' new OCR and description interfaces need an NPU and a
  packaged app; Windows.Media.Ocr and Apple Vision do not. §8.
- **Local models are ready enough.** llama.cpp runs many small vision models in GGUF, with
  Vulkan and Metal, and a Rust crate wraps it. Qwen3.5 (2B, 4B) and Gemma 4 (E2B, E4B) describe
  screens; PaddleOCR-VL reads Turkish best in the one Turkish OCR benchmark. Windows' own OCR
  reads Turkish; Apple's is unconfirmed. Encoding a whole screenshot is slow on a laptop. ggmlc
  does not yet run vision-language models. §9.
- **Sign in with ChatGPT is real and new** (2026-09-29): an open-source app running on the
  user's machine can use the user's Plus or Pro plan for image descriptions and follow-up
  questions, through OpenAI's public API. "Must be open source" is too strong: open source is
  one of the groups admitted without a waitlist. Training and retention for these requests are
  not documented. §10.
- **OpenDesk** has the shape a screen reader's remote feature needs: paired peers, an outbound
  relay that cannot read sessions, one controller at a time. Its pairing code can be attacked
  by a party in the middle of the pairing; a PAKE closes that. §11.
- **Okur is close for OCR**: screen capture on three platforms, the Results Viewer, dialogs as
  data and the message catalog exist; layers, an edit control and a protected-field state do
  not. §12.

**Recommendations** (decided in the proposal):
1. OCR first, with the systems' engines and JAWS's keys, in the Results Viewer.
2. Descriptions from local GGUF models in a separate process; cloud only by the user's choice,
   announced every time, with the source always said.
3. Sign in with ChatGPT and an OpenAI-compatible key client in Okur; other providers as
   extensions.
4. Okur Remote on OpenDesk's model, with typed speech, physical keys, a PAKE, a Noise channel,
   consent on the controlled machine, and a relay that learns as little as possible.

**Contents.** §1 what was asked · §2 JAWS 2025's layers · §3 Convenient OCR · §4 Picture
Smart and JAWS's other AI · §5 other JAWS utilities · §6 JAWS remote access · §7 NVDA · §8
Narrator, VoiceOver, Orca and others · §9 local models and the systems' OCR · §10 cloud
providers and Sign in with ChatGPT · §11 OpenDesk · §12 Okur today · §13 not verified ·
Sources.

---

## 1. What was asked

The owner asked (2026-10-18) for a study of JAWS's utilities beyond basic reading, and a
proposal for Okur's own:
1. OCR: JAWS's Convenient OCR, its targets, results, languages and engines.
2. Picture Smart and every other AI feature of JAWS 2025 and 2026.
3. Remote access: JAWS Tandem, JAWS in remote desktop sessions, and NVDA Remote Access as
   the open-source reference.
4. Other JAWS utilities of the same kind, each with a verdict.

With three foundations of the owner's to use and check:
- **Local AI in GGUF.** The owner is an early contributor to ggml and llama.cpp, a co-author
  of the GGUF format, and the author of ggmlc (§9).
- **Sign in with ChatGPT**, which the owner said lets open-source applications use their
  users' ChatGPT plans (§10).
- **OpenDesk**, the owner's remote control program, which this project uses to drive old-pc
  (§11).

How it was researched: five research agents (JAWS's OCR and AI; remote access; other screen
readers; local models; Sign in with ChatGPT); then the writer of this page reopened the
load-bearing sources, read JAWS 2025's key map on the dev box, queried the dev box's installed
OCR languages, read OpenDesk's source and documentation, and read Okur's code.

## 2. JAWS 2025's layers, from its key map

Read on the dev box from `C:\ProgramData\Freedom Scientific\JAWS\2025\Scripts\enu\Default.JKM`,
as key assignments and command names only [J2]. The dev box's JAWS is 2025.2505.43.400, the
same build as old-pc's (Windows' list of installed programs). Every layered key below exists
as Insert+Space and as JAWSKey+Space (CapsLock+Space in the laptop layout).

**Convenient OCR: Insert+Space, then O, then:**
- W `RecognizeRealWindow`; C `RecognizeControl`; S `RecognizeScreen`; Q `CancelRecognition`;
- B `StartBackgroundRecognition`; Ctrl+B `StartBackgroundRecognitionWithSound`;
- Enter `OCRAllInOne`; Ctrl+Enter `OCRAllInOneWithSound`;
- D `OCRDocument`; F `OCRSelectedFile`; R `OCRSelectedFileToWord`; A `RecognizeCamera`;
- ? (Shift+Slash) `OCRLayerHelp`.

**Picture Smart: Insert+Space, then P, then a source letter:** C control, W window, S screen,
F selected file, A camera, Enter `PictureSmartAllInOne`, ? `PictureSmartLayerHelp`. The
modifiers on the letter make the variants:
- none: `PictureSmartWith<Source>`;
- Alt: `…AskPrelim`, a question first;
- Ctrl: `…(4)`, the older Picture Smart (§4.1);
- Shift: `…MultiService`; Shift+Alt and Shift+Ctrl combine them.

**Tandem** (its own section of the key map): Alt+Insert+T `StartOrEndTandemSession`;
Alt+Insert+Tab `ToggleTandemMode`; Ctrl+Shift+Insert+V `PauseTandemVideo`.

**Other layers and keys of the same family:**
- Insert+Space, F1 `LaunchFSCompanion`; Insert+G `UILabeler`; Ctrl+Insert+G
  `AutoLabelGraphics`.
- Insert+Space, F: Face in View (O on or off, D description, L brightness, P
  `FaceInViewPictureSmart`, U confidence, C then arrows for the camera, ? help).
- Insert+Space, E: mouse echo (O on or off; C, W, L, P the unit; D item descriptions).
- Insert+Space, A `ToggleTextAnalyzer`; Insert+Space, R and Insert+Windows+R Research It.
- Insert+Space, H speech history (Shift+H clear, Ctrl+H copy); N notification history
  (Shift+N repeat the last, Ctrl+N toast announcements); C `ViewClipboardText`; J
  `LaunchCommandsSearch`; L `LockKeyboard`; D audio ducking; Q `OpenQuickAccessBar`.
- Insert+Space, F11 and Insert+Space, PrintScreen `ScreenShadeToggle`.

**Letters JAWS's default key map uses after Insert+Space:** A, B, C, D, E, F, H, J, L, M, N,
O, P, Q, R, S, T, V, X, Y, Z and the accent key, plus F1, F4, F11 and PrintScreen. Free in
the default key map: G, I, K, U, W. Application key maps use more (WhatsApp's Insert+Space, I
for image descriptions, per the SDK research §2.9).

**Two keys documented by Freedom Scientific are not in this key map**: Picture Smart's B
(the clipboard) [J18] and Page Explorer's Insert+Shift+E [J21], a JAWS 2026 feature. The
dev box and old-pc run JAWS 2025.

## 3. JAWS: Convenient OCR

**About the sources.** Freedom Scientific's support site refuses automated fetches (HTTP
403). The research agent read its What's New pages, the AI privacy statement and the Picture
Smart pages through a reader service; the writer reopened the privacy statement the same way.

### 3.1 Targets

- **W, S, C**: the window, the whole screen, the focused control [J1].
- **F**: the image file selected in File Explorer (BMP, JPG, GIF, TIF, PNG, PCX since JAWS
  2018) [J3]. File Explorer's context menu also offers "Convenient OCR with JAWS" and
  "Convenient OCR to Word" [J4].
- **D**: the PDF open in Adobe Reader; since 2018 JAWS offers OCR when Reader reports an
  "Empty Document", a scanned PDF [J3].
- **A**: Freedom Scientific's PEARL camera (multi-page capture by motion, a shutter sound,
  page orientation spoken: "sideways", "upside down", "right side up") and TWAIN scanners
  [J3], [J5]. Other cameras: not found.
- **R**: the selected file into a Word document (JAWS 2021; needs Office) [J7], [J8].
- **Enter** (December 2024): the target by context: the selected file in File Explorer, the
  PDF in Reader, the video area in YouTube or the shared screen in Teams and Zoom (as
  Background OCR), otherwise the window [J9].
- **B, Background OCR** (December 2024): recognizes the active window again and again; the
  user reads with the JAWS or Invisible cursor while the text updates; Ctrl+B adds a soft
  clicking sound; Alt+Delete says whether it is on; it stops on Q, on a return to the PC
  cursor, a new page, or a focus change [J9].
- **Q** cancels; **?** speaks the layer's keys [J1].

### 3.2 Results

- Results open in the **Results Viewer**, read with the virtual cursor like a web page;
  Escape closes it [J10], [J11]. Since September 2025 Ctrl+F there opens JAWS Find [J9].
- Background OCR is read in place with the JAWS or Invisible cursor [J9]. In JAWS 13 (2011)
  screen, window and control OCR also put the JAWS cursor on the results [J12] (excerpt);
  today's behavior for W, S and C was not confirmed.
- From File Explorer's context menu, JAWS plays sounds while it works instead of speaking
  [J4]. No documented "recognizing" or "done" message was found: old-pc can record them.

### 3.3 Engines and languages

- **Kofax OmniPage** since JAWS 2020, updated March 2025; Hebrew and Arabic come with the
  English and Western European downloads [J13], [J9].
- **Microsoft's OCR** for screen, window and control since JAWS 2021, through the Settings
  Center option "Use Microsoft OCR For Screen Recognitions"; OmniPage stays the default
  [J7].
- The Settings Center group is now "Convenient OCR and Picture Smart" [J9]; a primary and a
  secondary recognition language were named in a 2015 forum post [J14] (excerpt).
- OCR runs on the computer: Windows' OCR works offline [J15], and the AI privacy statement
  does not cover OCR [J16].

## 4. JAWS: Picture Smart and its other AI features

### 4.1 Picture Smart

**History.** JAWS 2019 sent images to Microsoft and Google services [J17]; JAWS 2021 used
Microsoft by default and Shift for more services [J7]. **Picture Smart AI** came in March
2024 with OpenAI's and Anthropic's models; Google Gemini was used at first and replaced by
Claude in May 2024 for "fewer hallucinations" [J18].

**Keys**, after Insert+Space, then P [J18], [J19], [J2]:
- C the control; W the window; S the screen; F the selected file (BMP, JPEG, GIF, TIF, PNG,
  HEIC); A the camera or scanner; B the clipboard (§2); ? help.
- Enter by context: the slide in Teams or PowerPoint, the image or control on a web page, the
  selected file, a frame of a playing video, the focused image in WhatsApp, the shared screen
  in Zoom, an Outlook image attachment without opening it [J18], [J9].
- Alt + source asks a question before sending; Shift shows both services' descriptions;
  Ctrl uses the older Picture Smart, which "some users find better" for text in images
  [J18].

**Results** [J18], [J19], [J20]:
- JAWS says "Picture Smart is in progress" while it works (a 2026 webinar transcript).
- The Results Viewer opens with a short description, then links: "Get more details", "Ask
  questions about this image", and feedback.
- A warning appears at the top when the image may be cropped.
- With Screen Shade on, JAWS says the image cannot be described [J9].

**Follow-up questions** [J18], [J21]: Enter on the link, type, Enter; a progress sound; the
answer is read; the focus stays in the edit field; H and Shift+H move between questions and
answers; the conversation lasts until the window closes. Since December 2025 a question
asked first is answered at once, without a full description.

**Where pictures go** (AI privacy statement, reopened by the writer) [J16]:
- Pictures, Freedom Scientific's prompts, the user's question and settings go over TLS to
  Freedom Scientific's service, then to OpenAI's and Anthropic's enterprise APIs.
- A temporary cache for the chat is cleared every four hours; no picture is kept.
- Not used for training.
- Administrators can turn Picture Smart off, or limit it to the older version, at install.
- Model names are not published.

**Accounts and age** [J21], [J22]: in the United States, JAWS 2026 needs a Vispero account;
since May 2026, AI features need an account aged 13 or over; younger users get stricter
limits and no chat.

**Languages**: a "Picture Smart Language" choice in Settings Center; 38 languages by machine
translation in 2021, four more in 2025 [J7], [J9].

**Limits**: none published; one user reported a limit of 50 in 2024 [J23] (excerpt).

### 4.2 The other AI features, 2024 to 2026

- **FS Companion**, renamed **Vispero Companion** in March 2026 [J9], [J21]: an assistant for
  questions about JAWS, Windows, the web and Office; Insert+Space, F1; answers in a Results
  Viewer window; dictation with Alt+Shift+D. OpenAI only; queries and answers kept as
  conversation data and telemetry up to two years, with the country if known; dictation audio
  transcribed by OpenAI and not kept [J16].
- **Page Explorer** (JAWS 2026): Insert+Shift+E summarizes a web page (layout, main content,
  important links, navigation tips with keys) in the Results Viewer, with follow-up questions;
  English, Spanish, German, Dutch, French and Italian; OpenAI's model; the page content is
  "not collected or stored" [J21], [J16]. What is sent (text, tree or picture) is not
  documented.
- **AI Labeler** (early adopters May 2025; on by default in JAWS 2026): Insert+G on an
  unlabeled element sends a picture of it to OpenAI and speaks a label, its type and state;
  since July 2025 labels are kept as custom labels, with a choice when one exists; since
  December 2025 a web image gets a one- or two-sentence description; Alt+Insert+G is the older
  Graphics Labeler [J9], [J21], [J16].
- **Automatic language detection** "based on AI predictions" (JAWS 2025), on the computer,
  without the internet [J9].
- **An AI agent** that reads the screen and clicks and types, with a stop button and
  confirmations, was shown in development in March 2026 [J24]; no What's New entry up to
  September 2026 mentions it.

## 5. JAWS: other utilities of the same kind

- **Face in View** (JAWS 2024): checks face position, eye contact, lighting and other things
  in the camera's view before a video call and alerts on changes; it needs the camera to
  itself, so not during a call; its background description goes through Picture Smart [J18],
  [J25].
- **Mouse Echo**: speaks what is under the pointer (character, word, line, paragraph; since
  2022 each control once); local [J1], [J26].
- **Text Analyzer** (by JAWS 2019): unmatched brackets and quotes, extra spaces, stray
  punctuation, font and color changes; Alt+Windows+I and Alt+Shift+Windows+I move between
  them; local [J27], [J2].
- **Research It**: look-ups in web sources, results in the Results Viewer; needs the
  network; an installer option turns it off [J1], [J28], [J7].
- **Notification History**: the last 24 hours of notifications, with rules that speak,
  braille or play a sound; local [J26], [J28].
- **Smart Glance** (JAWS 2023): the least common colors, fonts and attributes on a page,
  moved through with Y and Shift+Y; local [J28].
- **Voice Assistant** ("Sharky", JAWS 2021): the wake word is heard on the computer, then
  five seconds of speech go to Microsoft's cloud [J7], [J29].
- **Split Braille** (JAWS 2024): a second region of the display with another view; local
  [J18], [J9].
- **Screen Shade, speech history, Commands Search, clipboard text, Lock Keyboard**: in the key
  map (§2).

## 6. JAWS: remote access

### 6.1 JAWS Tandem

- **What it is.** One JAWS user reaches another computer running JAWS, for support, training
  or remote scripting. The controller works inside the target user's own desktop session,
  while that user is still at the computer. Two computers per meeting [J30], [J31].
- **Two ways to connect** [J30], [J31]:
  - **Tandem Center**, over the internet: Freedom Scientific's gateway creates meeting IDs,
    checks proper use and passes data between the two machines (port 443).
  - **Tandem Direct**, on a private network: the target listens (port 3000 by default, with a
    firewall exception), and the controller types the target's host name or address and a
    meeting ID the target made up.
- **Starting a Tandem Center session** [J30]: the controller opens JAWS's Utilities menu, JAWS
  Tandem, Tandem Center, "Get Access to a Computer"; a dialog shows a meeting ID (letters and
  digits, not case sensitive, valid 60 minutes until used). The target presses Insert+Alt+T,
  which opens "Allow Access to My Computer" with the focus in the Meeting ID field, types the
  ID and activates Allow Access. The target then says "Tandem connection open".
- **Keys** ([J30]; the same in the key map, §2): Insert+Alt+T starts the target's side or ends
  the session on either side; Insert+Alt+Tab, on the controller, switches between the target's
  desktop and its own; Insert+Ctrl+Shift+V turns video on or off.
- **What travels** [J31]:
  - Both machines must share a synthesizer, or JAWS offers Eloquence to both; only pitch and
    volume can change during a session; both must use the same main language. So speech
    travels as text and commands for the controller's synthesizer (inferred).
  - Sounds are not streamed: the controller plays the same or a similar sound file.
  - Video is optional, for sighted helpers; the mouse works only with video on.
  - The controller's keyboard layout is used on both sides. Ctrl+Alt+Delete acts on the
    controller's own machine; Ctrl+Shift+Escape opens the target's Task Manager.
  - Clipboard: copy on the target, paste on the controller, formatting kept; no files [J30].
- **Security** [J31], [J32]: a proprietary protocol over TLS 1.2 since JAWS 2022; Freedom
  Scientific says it records no session content and that Tandem Center cannot read the data.
  This cannot be checked from outside.
- **Audit**: session start, end, suspend, resume and errors go to the Windows Event Log, with
  meeting ID, role, peer and durations; no user activity [J30].
- **Licensing**: Tandem Center is free on both sides; Tandem Direct is charged per
  controller; a target may run JAWS as a 40-minute demo [J30], [J31].
- **Still there?** Documented on Freedom Scientific's keystroke and help pages and present in
  JAWS 2025's key map; not mentioned in the 2025 and 2026 release notes [J33], [J34].

### 6.2 JAWS in remote desktop sessions

- JAWS, ZoomText and Fusion work over Remote Desktop, Citrix, VMware Horizon and Amazon
  WorkSpaces [J35].
- **Best setup**: the same product on the client and the server. The server's copy sends its
  output over a Freedom Scientific virtual channel, and the client's copy speaks it, shows it
  in braille or magnifies it [J35].
- **Server only**: speech comes back as ordinary redirected audio, without braille [J35].
- A one-time "Remote Access authorization" is added to the license; a dormant server mode
  waits silently for a JAWS client; a Remote Desktop dialog in JAWS's Options menu shows each
  channel's state and repairs it [J35]. JAWS 2026 added Amazon WorkSpaces' DCV protocol
  [J34].
- Without a screen reader at both ends, a local screen reader sees only a picture of the
  remote screen [S21]; Microsoft's advice for a Cloud PC is to run Narrator inside it [S22].

## 7. NVDA

### 7.1 Windows OCR

- **NVDA+R** recognizes the navigator object (by default the focus or the browse-mode cursor)
  with Windows' OCR [N1].
- **The result is a document**: arrows move, Enter or Space clicks the text under the cursor,
  Escape closes it, NVDA+F5 recognizes again [N1]. Since 2026.1 the browse-mode highlighter
  shows on results, and an option starts Say All on the result [N2].
- **Settings** ("Windows OCR"): the recognition language; refresh every 1.5 seconds while the
  result has the focus (off by default); say all on result (off by default); more languages
  come from Windows Settings [N1]. Since 2026.3 beta 1, OCR works with the screen curtain on
  [N3].
- **Design** (read in the GPL source, not copied):
  - an abstract recognizer: one asynchronous job at a time, a callback, cancel; hooks for a
    resize factor, object checks, an auto-refresh interval and auto Say All [N4];
  - the capture records its screen origin and scale, so image positions map back to the
    screen [N4];
  - two result kinds: lines of words with screen rectangles (clickable), or plain text (not
    clickable) [N4];
  - the result is a document object with cursor behavior; messages while working and on
    failure ("Recognizing", "Content is not visible", "Recognition failed", paraphrased from
    the source) [N5];
  - the Windows engine scales images under 100 pixels by 4 and picks its language from NVDA's
    interface language, then the first installed OCR language [N6];
  - add-ons plug other engines into the same framework [N4].

### 7.2 AI descriptions in NVDA itself: tried and withdrawn

- A local captioning model (ONNX, English only, NVDA+G) was merged in September 2025 for
  2026.1 [N7], marked "Could be…" and experimental after reports of invented content [N8],
  then reverted in January 2026: poor captions from an old model, memory and installer cost,
  and a runtime failure on ARM [N9]. A newer model went to a feature branch [N10].
- NV Access's roadmap lists on-device image description as long-term and better OCR,
  including a choice of OCR models, as medium-term [N11].

### 7.3 NVDA add-ons

Popular in the Add-on Store, October 2026 [N12]:
- **AI Content Describer**: cloud providers (one free without a key; OpenAI, Gemini, Claude
  and others with the user's own key), local options (Ollama, llama.cpp, a PaliGemma 2
  daemon). NVDA+Shift+I menu, NVDA+Shift+U navigator object, NVDA+Shift+Y clipboard image,
  NVDA+Alt+C follow-up conversation; the camera; a beta "computer control" mode that asks
  consent. No privacy section [N13].
- **Vision Assistant Pro**: Gemini, OpenAI, Mistral, Groq, any OpenAI-compatible server. A
  layer, NVDA+Shift+V, then O full screen, V object, E a list of on-screen elements to click,
  L label one element, D a document reader, Shift+C chat, Space the last result. Its README
  warns that its AI operator sends a screenshot at every step and that backups include API
  keys [N14].
- **AI-Hub**: chat and descriptions through OpenAI, Anthropic, Gemini, Mistral, OpenRouter and
  others, or Ollama; keeps conversations in NVDA's configuration folder [N15].
- Local captioners (BLIP, English, 1 to 5 seconds) [N16]; Tesseract OCR for files, PDFs and
  scanners [N17].

**What this shows (inferred).** NVDA users mix many providers through their own keys, with
follow-up chat and a layer of commands, and little is said about privacy. Local models are
wanted, but NV Access withdrew a weak one: quality decides.

### 7.4 NVDA Remote Access

**What the user does.**
- Built in since 2025.1, off by default; it began as the NVDA Remote add-on by Tyler Spivey
  and Christopher Toth (2015) [N1], [N18].
- NVDA+Alt+R opens Connect: "Allow this computer to be controlled" or "Control another
  computer"; a server (an existing relay with host and key, or "host locally" on port 6837,
  which may need port forwarding); a key that is both the session's name and its password,
  with a Generate Key button [N1].
- NVDA+Alt+Tab switches between controlling the other computer and one's own. Connect,
  disconnect, copy link, mute, send clipboard and send Ctrl+Alt+Delete have no default keys
  [N1].
- Secure screens work only with an installed NVDA whose settings were copied to the system
  configuration [N1].
- Since: Ctrl+Alt+Delete and auto-mute (2025.3); control returns to the local machine when it
  locks; local messages in braille while controlling (2026.1); a security fix for untrusted
  servers (2026.1.1) [N2].

**How it works** (read in the GPL source, not copied):
- Protocol version 2: one JSON object per line over TLS, each with a type [N19].
- **Speech, not audio**: the controlled machine sends each speech sequence (strings and speech
  commands such as pitch or language) just before it would speak; the controlling machine
  rebuilds the commands it knows and speaks with its own synthesizer. Tones go as frequency
  and length; sound files as a name, played only if the same file exists locally [N20].
- **Braille**: cells are sent only when a controller has a display; the smallest connected
  display sets the width; braille gestures go back [N20].
- **Keys**: while controlling, every key from NVDA's hook is suppressed locally and sent as
  virtual-key code, scan code, extended flag and state; only the toggle and Ctrl+Alt+Delete
  stay local; held modifiers are released when control returns. The controlled side injects
  keys with SendInput and derives the scan code itself [N21].
- **Ctrl+Alt+Delete** uses SendSAS, which needs UIAccess and a system policy, or the secure
  desktop [N20], [S23].
- **Clipboard**: text only, pushed by hand [N21].
- **Secure desktop**: the user's NVDA runs a relay on 127.0.0.1 that the secure-desktop NVDA
  joins as the controlled machine [N22].
- **Hosting locally** makes a self-signed certificate; a key generated there is 7 random
  digits [N22].

**Security.**
- **A relay can read everything.** TLS ends at the relay on both sides, so it sees keystrokes,
  speech, braille and clipboard, and could inject keys. NV Access's own design record says so;
  direct connections are not affected [N23].
- **End-to-end encryption is proposed, not built**: an issue for 2027.1 [N24]; a design record
  (Proposed, awaiting review in September 2026): X25519, HKDF salted with the channel key,
  XSalsa20-Poly1305, only a SHA-256 hash of the key sent to the server [N23]; a prototype
  closed unmerged [N25].
- **Inferred**: hashing a short key does not hide it. Keys are often 7 digits; trying all ten
  million takes a computer seconds, so a relay that wants a key can recover it from its hash
  and then sit in the middle. A password-authenticated key exchange (§11.2) avoids that.
- **Advisories**: a critical one (June 2026): a manually trusted certificate was not checked
  on reconnect, fixed in 2026.1.1 [N26]; two low ones about denial of service [N27].

**The relay.** NV Access's (`nvaccess/remote-server`, AGPL-3.0, Python): a channel exists while
someone has joined with its key, which is the only access check; each line is parsed and
forwarded to the others; generated keys are 7 digits, reserved a day, one per second per
address; it logs addresses and byte counts [N28]. Community relays exist in Python, Go and
Rust [N29]. Who runs nvdaremote.com today was not confirmed [N30].

### 7.5 RDAccess: speech and braille over remote desktop channels

- An NVDA add-on by Leonard de Ruijter, installed at both ends of a Remote Desktop, Citrix,
  Parallels RAS or VMware Horizon session [N31].
- The server's NVDA uses a "remote" synthesizer and braille driver whose output goes over two
  dynamic virtual channels, one for speech and one for braille, with messages modeled on NVDA
  Remote's. The client side is a small Rust library registered as a virtual-channel plugin
  without administrator rights, which exposes a named pipe to the client's NVDA [N31], [N32].
- Remote speech is pitched 10% higher by default, so the user can tell it from local speech;
  caps lock is kept in step [N31].

## 8. Narrator, VoiceOver, Orca and other tools

### 8.1 Windows Narrator and Windows' AI interfaces

- **Narrator+Ctrl+D** describes the focused image. On Copilot+ PCs the description is made on
  the device; on other PCs the image goes to Microsoft. Models arrive through Windows Update;
  the online services can be turned off [S1], [S2].
- **With Copilot** (2026): Narrator+Ctrl+D describes the image and Narrator+Ctrl+S the whole
  screen in Copilot; the image is shared only after the user chooses to; follow-up questions;
  not in the European Economic Area [S3].
- **For other programs**: the Windows App SDK's `TextRecognizer` (lines, words, polygons,
  confidence) and `ImageDescriptionGenerator` (brief, detailed, diagram and "accessible"
  kinds). Both need a Copilot+ PC's NPU; the description API also needs an MSIX-packaged app
  with a special capability; their languages were not found [S4], [S5], [S6].
- **Windows.Media.Ocr**, the engine NVDA uses, is on every Windows 10 and 11 machine; its
  languages are the installed OCR language packs [S7].

### 8.2 VoiceOver and Apple's interfaces

- **VO-Shift-L** describes the image under the VoiceOver cursor; descriptions use Apple
  Intelligence by default, and an "On-Device Mode" keeps processing on the Mac with less
  detail [S8].
- Apple announced in May 2026 a system-wide image explorer with Apple Intelligence, and lists
  Turkish among Apple Intelligence's languages [S9].
- **Remote**: VoiceOver can control another Mac through Screen Sharing; VoiceOver turns on
  temporarily on the remote Mac and the local Mac speaks, with another pitch for the shared
  screen; VO+Shift+Down enters it [S24], [S25]. How speech travels is not documented.
- **For other programs**: Vision's `VNRecognizeTextRequest` (fast or accurate, chosen
  languages, language correction, custom words) [S10]; `RecognizeDocumentsRequest` on macOS
  26 (words, lines, paragraphs, tables, lists) [S11]; VisionKit's Live Text analyzer [S12];
  the Foundation Models framework, an on-device model on Apple Intelligence Macs, which takes
  image attachments from macOS 27 [S13], [S14].
- Whether Vision's OCR reads Turkish was not confirmed: one user's list, on an unknown macOS,
  had none [S15].

### 8.3 Orca

- No OCR, AI or remote feature in Orca's release notes up to 51.1 (October 2026); 51 added
  user extensions [S16].
- `ocrdesktop`, a separate Tesseract tool, recognizes the active window or the desktop into a
  browsable view with positions, and can click [S17]. A fork of Orca has an
  NVDA-Remote-compatible plugin in progress [S28] (excerpt).

### 8.4 Other tools blind people use

- **Be My Eyes for Windows**: the screen and images described, with follow-up questions,
  through OpenAI's models [S18].
- **Copilot Vision**: shares a window or the desktop with Microsoft's cloud [S19].
- **Remote Incident Manager** (Pneuma Solutions): a central service sets up the session, then
  the computers connect directly; it streams stereo audio, so the helper hears the other
  machine's screen reader, whatever it is; voice chat, clipboard and files [S27].
- **Quick Assist** reportedly works with a screen reader on the side receiving help; UAC
  prompts show the helper a black screen [S26] (excerpt).
- **TeamViewer**: users report its own interface became inaccessible in recent versions [S29]
  (excerpt).
- **Research**: Sinter (EuroSys 2016) sent a platform-neutral description of a remote
  application's interface and rebuilt it as a local proxy window, so VoiceOver on a Mac could
  read Windows applications [S30].

## 9. Local models, runtimes and the systems' OCR

### 9.1 The systems' OCR

- **Windows.Media.Ocr** exists on every Windows 10 and 11 machine, offline. Results are lines
  of words, each with a rectangle. Its languages are optional Windows features
  (`Language.OCR~~~xx-XX`) [M2], [M3].
- **Observed on the dev box** (a read-only PowerShell query, 2026-10-18): the installed OCR
  languages are Turkish (`tr`) only, though the user's language list is English and
  Turkish; `MaxImageDimension` is 10,000 pixels; the engine answers an ordinary unpackaged
  process. So Windows' OCR reads Turkish, and a missing English pack is a real case Okur must
  handle (inferred).
- From Rust: `windows::Media::Ocr::OcrEngine` in the `windows` crate [M4], which
  `okur-platform` already uses for OneCore voices (§12).
- **Windows' newer `TextRecognizer`** needs a Copilot+ PC's NPU (40 TOPS or more, 16 GB) and an
  app with package identity; no languages were found [S4], [S6].
- **Apple Vision**: `VNRecognizeTextRequest` is reachable from Rust through `objc2-vision`
  [M5]; `RecognizeDocumentsRequest` (macOS 26) covers 26 languages and has no Objective-C form,
  so it would need a small Swift shim (inferred) [M6]. **Turkish is not confirmed**: macOS's
  Live Text availability list does not show it [M7], and older reports had none [S15]. Okur
  must check the supported languages at run time.
- **Linux** has no system OCR (not found).

### 9.2 The runtime: llama.cpp's multimodal library

- llama.cpp takes images (and audio and video) through `libmtmd`, in `llama-cli`,
  `llama-server` (whose OpenAI-compatible `/chat/completions` endpoint takes images) and
  `llama-mtmd-cli` [M1]. `libmtmd` replaced the older LLaVA code in April 2025 [M8].
- **Each model is two GGUF files**: the language model and a projector (`mmproj`) that turns
  pictures into tokens; the projector runs on the GPU by default [M1].
- **Supported vision families** (docs, October 2026): Gemma 3 and 4, SmolVLM and SmolVLM2,
  Pixtral, Qwen2-VL and Qwen2.5-VL, Mistral Small 3.1, InternVL 2.5 and 3, Llama 4 Scout,
  Moondream 2; the tool's README adds MiniCPM-V and Granite Vision; Qwen3-VL and Qwen3.5 came
  by pull request [M1], [M9], [M10]. **OCR models** have their own list with prompt rules:
  PaddleOCR-VL, GLM-OCR, DeepSeek-OCR, dots.ocr, HunyuanOCR [M1].
- **The C interface** (`mtmd.h`) is marked experimental, with breaking changes expected;
  parameters include GPU use and the minimum and maximum image tokens [M11], [M9].
- **GPU backends**: Metal, CUDA, HIP, Vulkan, SYCL, OpenCL and others; MIT license; prebuilt
  Windows Vulkan, CUDA, CPU and macOS ARM builds with each release [M12].
- **From Rust**: the `llama-cpp-2` crate (0.1.160, October 2026) has features `mtmd`, `vulkan`,
  `metal`, `cuda`, `dynamic-backends` and `dynamic-link`, and wraps the multimodal library
  (context, bitmaps from a buffer, tokenizing, evaluating chunks, image token limits); it
  promises no semantic versioning [M13], [M14].
- **Alternatives**: mistral.rs (Rust, many vision models; CUDA, Metal and CPU, Vulkan not
  listed; which vision models load from GGUF is not stated) [M15]; candle (Hugging Face's Rust
  framework; examples for PaddleOCR-VL, Moondream, Pixtral and others; no Vulkan) [M16]; ONNX
  Runtime through the `ort` crate, with DirectML and CoreML [M17].
- **Field reports** worth knowing: an AMD 780M integrated GPU lost its device running a
  projector on Vulkan, blamed on the driver's timeout [M18]; an Intel Iris Plus took 16 seconds
  per image tile with SmolVLM2 in a regression report [M19]. Model code can fail in ways that
  must not reach speech (inferred: a separate process).

### 9.3 ggmlc

The owner's compiler, read at [M20] and its roadmap [M21]:
- It compiles PyTorch 2 (through `torch.export`) and JAX/Flax models into a graph, optimizes it
  (constant folding, fusion, dead code removal), and writes a GGUF file or a standalone C++
  project that links ggml; a native runner and Python bindings come with it.
- Verified models include CNNs (ResNet, MobileNetV3, EfficientNet, ConvNeXt), SSDLite detection,
  ViT, BERT-class encoders, small language models (SmolLM2, Gemma 3 270M, Qwen 2.5 0.5B),
  Whisper-Tiny and CLIP; Q8_0 and Q4_0 quantization; CPU and CUDA, with Vulkan as a build
  option.
- Its own benchmarks report decoding and prefill at or above llama.cpp's on a laptop RTX 4050
  for most cells (self-reported).
- **Planned, not done**: LLaVA- and Florence-2-style vision-language models, Metal, Vulkan and
  ROCm, Rust bindings (`ggmlc-rs`), mobile.
- **For Okur (inferred)**: ggmlc fits small, fixed vision networks well, the kind classic OCR
  pipelines use (a text detector and a line recognizer, such as PaddleOCR's PP-OCR models, which
  are CNN-based). It is not yet a runtime for the vision-language models that describe
  pictures.

### 9.4 Models

Sizes are GGUF files at Q4 (or Q8 where noted) plus the projector. "In llama.cpp" gives the
pull request that added it.

**Describing pictures and screens**
- **Qwen3.5 2B and 4B** (February 2026; Apache-2.0; vision built in; "201 languages and
  dialects", Turkish not named). 4B: OCRBench 85.0, ScreenSpot Pro 60.3; about 2.7 GB plus a
  0.7 GB projector. 2B: about 1.3 GB plus 0.7 GB. In llama.cpp since February 2026 [M22],
  [M23], [M10]. Qwen3's text models list Turkish among 119 languages [M24].
- **Gemma 4 E2B and E4B** (Apache-2.0; 140 languages in training, 35 supported): the number of
  image tokens is chosen from 70 to 1,120, a direct speed control. E4B about 4.6 GB, E2B about
  2.8 GB, each with a 0.56 GB projector. In llama.cpp since April 2026 [M25], [M1].
- **Qwen3-VL 2B and 4B** (Apache-2.0; OCR in 32 languages, not listed; trained to operate
  computer interfaces): 2B about 1.1 GB plus 0.45 GB; 4B about 2.5 GB plus 0.45 GB. In
  llama.cpp since October 2025 [M26].
- **MiniCPM-V 4.6** (about 1.3B; Apache-2.0): about 0.5 GB plus a 1.1 GB projector, for small
  machines [M27].
- Others: SmolVLM2 (Apache-2.0, English only) [M28]; Qwen2.5-VL 3B (non-commercial license)
  [M29]; Mistral Small 3.1 (lists Turkish, but 24B, too large for a laptop) [M30]; LFM2-VL
  (license limits commercial use) [M31]; Phi-4-multimodal (vision English only; not in
  llama.cpp) [M32].

**OCR**
- **PaddleOCR-VL** (0.9B; Apache-2.0; 109 languages, Turkish listed in its Latin group): the
  1.5 GGUF is about 0.9 GB plus 0.9 GB; in llama.cpp since February 2026 with the prompt `OCR:`,
  one image per call, without Paddle's layout step [M33], [M34], [M35].
- **OCRTurk** (February 2026), a Turkish OCR benchmark of 180 documents and seven systems:
  "PaddleOCR achieves the strongest overall results" [M36]. The research agent read in the
  paper's tables that HunyuanOCR scored best on Turkish characters, and that common errors split
  diacritics from letters ("DOĞAL" as "DO˘ GAL") or misread "İ"; not rechecked by the writer.
- Others in llama.cpp: GLM-OCR (MIT, 0.9B), DeepSeek-OCR (MIT, 3B), dots.ocr (MIT, layout as
  JSON), HunyuanOCR (Tencent's license) [M1], [M37].
- **Classic engines**: Tesseract has Turkish data (`tur`) and wants 300 dpi, dark text on light
  [M38]; PaddleOCR's PP-OCRv5 Latin model covers Turkish [M39] and runs from Rust through
  `oar-ocr` on ONNX Runtime [M40]; `ocrs` (pure Rust) reads only a 96-symbol Latin alphabet, so
  not Turkish's ğ, ş, ı [M41].

### 9.5 What a laptop can do

- **Who uses what**: WebAIM's 2026 survey: Windows 90.7%, JAWS 55.0% of respondents, and 60.1%
  use AI for image descriptions [M42]. No data on blind users' hardware was found; 8 to 16 GB
  of memory and an integrated GPU is the assumption (inferred).
- **Text generation on integrated GPUs** (llama.cpp's Vulkan board, a 7B model): Iris Xe 6 to 8
  tokens a second; Core Ultra 200 about 24; Ryzen 7000 and 8000 about 20 [M43]. Apple M1 to M4:
  14 to 24 [M44]. A 2B to 4B model is roughly two to three times faster (inferred).
- **Encoding a picture is the slow part.** An M2 MacBook Pro took 1.2 s for a 640×488 image
  with Qwen3.5's projector, and about 40 s for a 4096×3123 one [M45]. A 1920×1080 screenshot is
  about 2,000 image tokens for Qwen3.5 and Qwen3-VL (computed from their configurations), 256
  for Gemma 3, 70 to 1,120 for Gemma 4 by choice (inferred from [M23], [M25]).
- **So (inferred)**: a picture of a control or a window, not the whole screen, and a cap on
  image tokens keep a description to a few seconds on an Apple Silicon Mac or a recent
  integrated GPU; tens of seconds on a CPU. Okur must measure before it promises.

## 10. Cloud providers and Sign in with ChatGPT

### 10.1 Sign in with ChatGPT: what is confirmed

A research agent read OpenAI's pages; the writer reopened the key ones on 2026-10-18.

- **It exists, under that name.** OpenAI's developer documentation calls it "Sign in with
  ChatGPT" (SIWC). The user grants two permissions separately: signing in, and "ChatGPT plan
  usage", which lets the app's AI requests count against the user's ChatGPT plan [C1], [C2].
- **It is new**: announced at OpenAI's DevDay on 2026-09-29 [C3].
- **Who may use plan usage**: at launch, open-source projects, personal projects that run
  locally, and selected private apps; paid or remotely hosted apps join a waitlist [C4]. The
  developer pages are written for "open-source and locally hosted apps" [C5].
- **"It must be open source": not as stated.** No OpenAI page found defines open source,
  lists licenses, or describes a check. Open source is one of the groups admitted without a
  waitlist; locally run personal projects are another. Okur, Apache-2.0 and running on the
  user's own machine, fits both as the pages describe them (inferred).
- **Not only for coding**: the examples are coding assistants, research tools and desktop
  utilities; the sample app rewrites copied text [C4].
- **Plans**: Plus and Pro, per the developer pages and OpenAI's user page [C4], [C6]. A help
  center article on the subject refused automated access; search excerpts of it mention Go
  and wider access for open-source tools. Unresolved.
- **The sign-in suits a desktop program** [C7]:
  - the system browser opens OpenAI's authorize page; OAuth 2.0 with PKCE (S256), state and
    nonce; no client secret;
  - the answer comes to a loopback listener on `127.0.0.1` (not `localhost`), any port;
  - the first sign-in registers the app for that user (dynamic registration with the client
    id `dynamic_agent_client`, a per-install opaque host id, the app's name); the user can
    rename it and reviews the permissions in the browser; later sign-ins reuse the issued id;
  - plan usage needs the scopes `offline_access resource.invoke chatgpt.tokens.use.direct`,
    and the app must check that the last was granted;
  - the app receives an ID token, a one-hour access token, and a refresh token that lasts 30
    days and rotates [C8].
- **What the app may call**: the public Responses API (`api.openai.com/v1/responses`) with the
  models the user's account lists. Text, images and files are accepted when the model takes
  them, so describing pictures and follow-up questions work. Requests must set `store: false`
  and `stream: true`; there is no stored conversation over HTTP, so the app sends the history
  with each question. Audio input, file uploads, image generation, computer use and hosted
  tools are not available [C9], [C10].
- **Limits**: use counts against the plan; no extra allowance; the user can cap each app's
  weekly share and decide whether apps may spend credits after the limit [C6]. Over the limit
  the app gets an error it must explain, with a "Manage usage" link; OpenAI never switches to
  another way of paying [C11], [C12].
- **What the app learns about the user**: name, email and profile picture at sign-in. Neither
  permission gives the app the user's ChatGPT conversations or memories [C5], [C6]. The user
  disconnects an app in ChatGPT's settings, under Security and login [C6].
- **OpenAI's DevKit** (Node.js and React) is under a noncommercial license; software that only
  talks to it through an interface is not a modified work [C13]. Okur would implement the
  documented protocol itself, in Rust, without the DevKit's code (inferred).
- **Before DevDay**, some open-source tools reused Codex's own ChatGPT sign-in; OpenAI's SIWC
  pages tell third parties to use the public Responses API instead of Codex's internal route
  [C9], [C14].

### 10.2 Not confirmed

- Whether plan-usage requests are used for training, whether the user's "Improve the model for
  everyone" setting governs them, and how long they are kept: no SIWC page says. `store: false`
  keeps the response out of OpenAI's stored responses; it is not a statement about training
  (inferred).
- Exactly which plans.
- Whether the current Terms of Use say anything about SIWC (the terms page refused automated
  access).
- Whether an open-source desktop app must also fill in OpenAI's interest form.

### 10.3 Other ways to reach a cloud model

- **Anthropic**: an API key is the only route for third-party apps; Anthropic forbids apps
  from offering Claude.ai sign-in or routing requests through users' Free, Pro or Max plans
  [C15].
- **OpenAI API key**: paid per use; not used for training by default; abuse-monitoring logs up
  to 30 days; zero retention only for approved customers [C16].
- **Google Gemini API**: the free tier's data may be used to improve Google's products, with
  human review; paid use is not [C17].
- **Azure OpenAI**: prompts are not shared with OpenAI or used for training [C18].
- **OpenRouter**: one key for many providers; logging is opt-in; each provider's policy also
  applies [C19].
- **Any OpenAI-compatible endpoint** reaches a llama.cpp server or Ollama on the user's own
  network with the same client (inferred from the shared interface; llama.cpp's server has an
  OpenAI-compatible chat endpoint that takes images [M1]).
- **Device-code sign-in** (GitHub's, used by editors for Copilot) is the other common way a
  desktop program signs in [C20]; SIWC uses a loopback redirect instead.

## 11. OpenDesk: what Okur's remote feature builds on

**Read from** the checkout at `C:\Users\Gigabyte\opendesk` (commit `ccbd988`): its `docs/`,
the Rust source under `opendesk-rs/src/`, `opendesk --help` and its subcommands' help, this
repository's `.mcp.json` and [`verification-loop.md`](verification-loop.md) §2. No command
that acts on a peer was run. Paths below are inside that checkout.

### 11.1 What it is and how this project uses it

- Two copies of one program: `opendesk serve` on the machine being controlled, the CLI or
  `opendesk-mcp` on the controller. MIT license, Rust, one binary each (`README.md`).
- This project drives old-pc with it: `opendesk exec --peer old-pc "<powershell>"` runs as the
  lab user in the interactive session; the MCP tools work too (`.mcp.json` starts
  `opendesk-mcp`). If old-pc restarts, `opendesk serve` must be started again, or every call
  fails with `target_offline` ([`verification-loop.md`](verification-loop.md) §2).
- On a peer it can take screenshots, move and click the mouse, press keys by name and type
  Unicode text, read the accessibility tree through xa11y (UI Automation on Windows, AX on
  macOS), use the clipboard, run commands, transfer files, lock the physical input and black
  out the screen (`opendesk privacy`), and it logs every call on the controlled machine
  (`opendesk-rs/src/remote/audit.rs`).
- It reads applications only through the accessibility tree and pixels: no virtual buffer, no
  reading commands, no speech. That is the accessibility limit the owner describes.

### 11.2 Pairing

- `opendesk pair` on the controlled machine prints a 6-digit code; `opendesk pair-with <host>
  <code>` runs on the controller (`docs/protocol/pairing.md`).
- The code becomes a 32-byte pre-shared key through PBKDF2-HMAC-SHA256 (200,000 iterations,
  fixed salt). A three-message handshake of ephemeral X25519 keys exchanges the machines'
  static public keys, each encrypted under keys derived from the pre-shared key and the
  Diffie-Hellman results.
- Each side stores the other's static key in `~/.opendesk/trusted-peers.json`; later
  connections need no code. A pairing accepts one attempt and exits
  (`docs/remote/security.md`).
- Over the internet, `pair` registers on the rendezvous server, and `pair-with` finds the
  target there by name, key prefix, or as the only machine online
  (`opendesk-rs/src/remote/client.rs`, `pair_with`).

**Inferred from the pairing document:** a passive observer cannot test codes, because every
key also needs a Diffie-Hellman secret it does not have. An active party in the middle (a
hostile relay, or someone on the local network) can: it runs the first two messages with the
controlled machine using its own ephemeral key, then tries codes offline against the
encrypted reply, within the pairing's five-minute window. PBKDF2 slows this; it does not stop
it. The document's own estimate is one CPU-month for all codes on one processor, and many
processors or graphics cards shorten that. A password-authenticated key exchange (PAKE) gives
such an attacker one guess per pairing, which is why Magic Wormhole uses SPAKE2 with its
short codes [O1]. SPAKE2 has a Rust crate under RustCrypto, which says it has had no
independent audit [O2]; CPace is the IETF research group's balanced PAKE, still a draft
[O3].

### 11.3 Reconnecting and the encrypted channel

- Reconnecting takes two messages: the client sends an ephemeral and its static public key;
  the server answers with proof that it holds its static private key; an unknown client gets
  an answer that does not say why it failed (`docs/protocol/auth.md`).
- Session keys come from four Diffie-Hellman results through HKDF-SHA256: mutual
  authentication and forward secrecy.
- Frames are ChaCha20-Poly1305 with one key per direction and counter nonces that are never
  sent; any failure ends the session (`docs/protocol/encryption.md`). The documents compare
  the design to Noise; it is OpenDesk's own handshake, not a named Noise pattern (inferred
  from `docs/protocol/`).
- MessagePack frames over binary WebSocket messages: requests, responses and pushes
  (`docs/protocol/transport.md`, `docs/protocol/session.md`).
- One controller at a time; a second one gets "busy" (`opendesk-rs/src/remote/server.rs`).
- A paired controller may call every method; the controlled side has no approval step
  (`docs/protocol/session.md`).

### 11.4 The rendezvous server

- `opendesk rendezvous` is a small relay: the controlled machine connects out and registers;
  the controller asks to connect to a target by public key; the relay forwards binary frames
  between the two legs (`opendesk-rs/src/remote/rendezvous.rs`). No port is opened on the
  controlled machine. An optional shared token guards the relay; TLS is left to a reverse
  proxy.
- The documents describe direct peer-to-peer paths with hole punching
  (`docs/remote/internet-transport.md`); in the code, the relay forwards every frame (no
  candidate exchange was found).
- **What the relay sees**, from the code: each registered machine's public key, name,
  description, IP address and port; which controller connects to which target, and when; the
  size and timing of every frame. With the token, a `list` request returns every registered
  machine's name, description and public key. After authentication it cannot read frames: the
  keys never reach it.
- On the local network, mDNS advertises the machine's static public key and a description
  (`docs/protocol/discovery.md`), so a laptop announces the same identifier on every network
  it joins (inferred).
- `docs/remote/security.md` still calls internet traversal "phase 2"; the rendezvous document
  and the code show it built. The security page is older than the code.
- Encrypted traffic can still reveal typing rhythm; OpenSSH added keystroke timing
  obfuscation in 9.5 for this reason [O4].

### 11.5 What Okur keeps, and what it changes

Keep: two copies of one program; pairing once with a code; trust by stored public keys; an
outbound connection to a relay, so no ports are opened; end-to-end encryption the relay
cannot read; one controller at a time; an audit log on the controlled side; the curtain.

Change for a screen reader (proposal §9):
- forward Okur's speech and braille, not pixels;
- send physical keys with their timing, not key names;
- a PAKE for codes, and words to compare aloud;
- consent on the controlled machine for each session, by default;
- a relay that learns as little as possible: no names, no listing, rotating identifiers,
  padded frames.

## 12. Okur today: what these features build on

- **Screen capture on three platforms.** The vendored xa11y has a `ScreenshotProvider` with
  Windows (GDI BitBlt of the virtual desktop), macOS (ScreenCaptureKit, which needs the
  Screen Recording permission) and Linux implementations, for the whole desktop or a
  rectangle (`crates/xa11y/xa11y-core/src/screenshot/mod.rs:176`;
  `xa11y-windows/src/screenshot.rs:1-9`, `xa11y-macos/src/screenshot.rs:1-4`). Elements carry
  screen bounds (`xa11y-core/src/element.rs:783`). A picture of a control or a window needs no
  new platform code.
- **The Results Viewer.** The Virtual Viewer is Okur's document window: Okur's own lines, read
  with the virtual cursor, shown on screen by `okur-ui` without taking the focus, links
  followed with Enter ([`virtual-viewer.md`](virtual-viewer.md); `ToUi::ViewerShow` in
  `crates/okur-ui-model/src/lib.rs`). JAWS shows OCR and Picture Smart results in its Results
  Viewer, which is this window.
- **Dialogs as data.** `okur-ui` draws dialogs from a `DialogSpec`: group, check box, choice,
  slider, button, list, radio buttons, read-only text (`ControlKind` in `okur-ui-model`).
  Missing for these features: an edit field (a question about a picture, a code), which the
  helper UI decision already lists.
- **The message catalog, part kinds and verbosity** (SDK M0,
  [`messages-and-verbosity.md`](messages-and-verbosity.md)): new words go into Fluent files;
  announcements are typed parts; output types switch parts and messages per level. Remote
  speech can travel as typed parts and be rendered in the listener's language.
- **Speech output** is text with role marks and an interrupt flag (`TtsEngine`,
  `crates/okur-platform/src/tts/mod.rs:32`; `SpeechOutput`, `crates/okur-core/src/speech.rs:898`):
  enough to forward speech now; speech sequences with index marks come with SDK M6.
- **Key routing** has a fixed order of owners
  ([`forms-and-interaction-modes.md`](forms-and-interaction-modes.md), "Who takes a key");
  a remote session becomes its first step. Layers are planned (helper dialogs plan step 10,
  SDK M3).
- **Password fields** are read only by harness tools today (`IsPassword` in
  `crates/okur-platform/src/uia_snapshot.rs:552` and `window_text.rs:185`, which leaves their
  value out); Okur's object model has no protected state yet.
- **WinRT from Rust**: `okur-platform` already uses the `windows` crate's WinRT namespaces for
  OneCore voices (`Media_SpeechSynthesis` in its `Cargo.toml`); Windows.Media.Ocr and
  Windows.Data.Pdf are further namespaces of the same crate (inferred).
- **The lab**: OpenDesk to old-pc, `okur-lab trace`, `okur-lab parity`, `okur-lab live-parity` (in
  progress, helper dialogs round 2 step 1), the speech recorder that logs every fragment JAWS
  speaks, and `okur-lab history` for a run's speech log.

**Missing:** OCR, image description, camera and PDF rendering, a network client, credential
storage, a remote session, sounds (SDK M5), braille (SDK M6), layers (SDK M3).

**Building blocks found outside:**
- Windows.Data.Pdf renders a PDF page to an image, on every Windows 10 and 11 machine [O9].
- The `keyring` crate stores secrets in Windows Credential Manager, the macOS Keychain and the
  Linux Secret Service [O8].
- The `snow` crate implements the Noise Protocol Framework in Rust (X25519, ChaCha20-Poly1305,
  BLAKE2 and SHA-2), under Apache-2.0 or MIT [O5].
- Windows lets a window exclude itself from screen capture; Microsoft says this is not a
  security mechanism [O6].

## 13. What could not be verified

**Sign in with ChatGPT**
- Whether plan-usage requests are used for training, which data controls apply, and how long
  they are kept: no OpenAI page says.
- Exactly which plans may use it: Plus and Pro per the developer pages; Go, and wider access for
  open-source tools, per search excerpts of a help article that refused automated access.
- Whether the current Terms of Use say anything about it.
- Whether an open-source desktop app must also fill in OpenAI's interest form.
- That open source is "required": no OpenAI text says so (§10.1).

**JAWS**
- JAWS's spoken messages for Convenient OCR (no "recognizing" or "done" message is
  documented); whether W, S and C results go to the Results Viewer or the JAWS cursor today; the
  current names of the OCR language settings. All can be recorded on old-pc.
- The model names behind Picture Smart, Companion, Page Explorer and the AI Labeler; what Page
  Explorer sends; Picture Smart's limits; whether Face in View analyzes faces on the computer.
- Whether old-pc's JAWS can reach Picture Smart's service (it needs the network and, in the
  United States, an account).
- Whether Tandem still works in JAWS 2025 and 2026, how it encodes speech, and Freedom
  Scientific's claim that Tandem Center cannot read sessions.
- Picture Smart's B (clipboard) key is documented but not in the dev box's JAWS 2025 key map.
- **A Turkish JAWS exists on the dev box.** Windows' list of installed programs shows "Freedom
  Scientific JAWS 2025 TRK" beside "ENU", and JAWS's shared folder has a `trk` folder with its
  own key maps. The extension SDK research (its §2.4 and §13) found no Turkish JAWS interface.
  Whether this gives Turkish speech messages that Okur's Turkish catalog could be compared
  with is not checked here; it matters for the SDK's D3 and D4.

**Models and platforms**
- Turkish in Apple Vision's OCR (needs a Mac and a run-time check).
- Turkish quality of Qwen3.5, Qwen3-VL and Gemma 4: their cards do not name Turkish; Okur's own
  evaluation must measure it.
- The languages of Windows' `TextRecognizer` and Narrator's descriptions.
- Time to a first description of a screenshot on a typical integrated-GPU laptop: no published
  figure; image-token counts above are computed, not measured.
- The OCRTurk paper's per-system Turkish-character scores (read by the research agent, not
  rechecked).
- Whether macOS still asks again every month to allow screen recording: reported in 2024 for
  macOS Sequoia [O7]; nothing newer found.

**Remote**
- Who runs nvdaremote.com today.
- How VoiceOver's Screen Sharing carries speech between Macs.
- Quick Assist's audio and Remote Incident Manager's details (excerpts only).

## Sources

Read 2026-10-18 by five research sub-agents working under the clean-room rules above, then
checked by the writer where marked "(checked)". Freedom Scientific's support site refused
automated fetches; its pages were read through a reader service or curl with a browser's
user agent, or are marked "(excerpt)".

### JAWS and Vispero

`WN` pages share the prefix `https://support.freedomscientific.com/Downloads/JAWS/`.
- [J1] JAWS keystrokes (hotkeys): https://www.freedomscientific.com/training/jaws/hotkeys/
- [J2] JAWS 2025's key map on the dev box: `C:\ProgramData\Freedom Scientific\JAWS\2025\Scripts\enu\Default.JKM`, sections `[Common Keys]` and `[Tandem keys]` (key assignments only; checked)
- [J3] JAWS 2018 features: https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2018
- [J4] Convenient OCR in File Explorer: https://sastltd.zohodesk.eu/portal/en/kb/articles/jaws-using-convenient-ocr-within-file-explorer
- [J5] Convenient OCR layered keys: https://sastltd.zohodesk.eu/portal/en/kb/articles/jaws-convenient-ocr-layered-hotkeys
- [J7] JAWS 2021 features: https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2021
- [J8] Sharkvember shortcuts roundup: https://training.vispero.com/sharkvember-shortcuts-weekly-roundup-8/
- [J9] JAWS 2025 What's New: `…/JAWSWhatsNew?version=2025`
- [J10] Inside the JAWS 2018 betas, OCR: https://blog.freedomscientific.com/inside-the-jaws-2018-betas-smarter-more-powerful-ocr
- [J11] University of Missouri knowledge base, JAWS OCR: https://tdx.umsystem.edu/TDClient/36/DoIT/KB/ArticleDet?ID=1197
- [J12] JAWS 13 OCR: https://nfb.org/node/1750 (excerpt)
- [J13] JAWS 2020 features: https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2020
- [J14] Convenient OCR languages, forum: https://jfw.groups.io/g/main/topic/using_convenient_ocr_in/212724 (excerpt)
- [J15] Windows 10 OCR, 2016: https://blogs.windows.com/windowsdeveloper/2016/02/08/optical-character-recognition-ocr-for-windows-10/
- [J16] Freedom Scientific AI privacy statement: https://www.freedomscientific.com/aiprivacystatement (checked)
- [J17] JAWS 2019 features: https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2019
- [J18] JAWS 2024 What's New: `…/JAWSWhatsNew?version=2024`
- [J19] Picture Smart page: https://www.freedomscientific.com/?p=12531
- [J20] Webinar transcript, AI features, June 2026: https://doccenter.freedomscientific.com/doccenter/archives/2026_06_04_Digital_Content/Use-AI-Features-in-JAWS-ZoomText-and-Fusion-to-Better-Understand-and-Navigate-Digital-Content.txt
- [J21] JAWS 2026 What's New (to 2026.2609.84): `…/JAWSWhatsNew`
- [J22] AI features: age and account requirements: https://training.vispero.com/upcoming-changes-to-ai-features-in-jaws-and-fusion-age-verification-and-account-requirements/
- [J23] A user's report of a limit: https://applevis.com/comment/164548 (excerpt)
- [J24] FSCast 268, March 2026: https://training.vispero.com/wp-content/uploads/2026/04/FSCast268-march2026Transcript.htm
- [J25] Face in View: https://sastltd.zohodesk.eu/portal/en/kb/articles/jaws-face-in-view
- [J26] JAWS 2022 features: https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2022
- [J27] Text Analyzer: https://training.vispero.com/proof-documents-faster-and-more-efficiently-with-jaws-text-analyzer/
- [J28] JAWS 2023 features: https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2023
- [J29] Voice Assistant FAQ: https://sastltd.zohodesk.eu/portal/en/kb/articles/voice-assistant-faq
- [J30] JAWS Tandem Quick Start: https://support.freedomscientific.com/JAWSHq/JAWSTandemQuickStart
- [J31] JAWS Tandem FAQ: https://support.freedomscientific.com/support/jawsdocumentation/JAWSTandemFAQs
- [J32] FSCast 207, November 2021: https://blog.freedomscientific.com/wp-content/uploads/2021/12/FSCast207Transcript-November2021.html
- [J33] JAWS Headquarters: https://support.freedomscientific.com/JAWSHQ/JAWSHeadquarters01
- [J34] JAWS release features 2024 to 2026: https://support.freedomscientific.com/Downloads/JAWS/PreviousFeatures/?version=2024 (also 2025, 2026)
- [J35] JAWS, ZoomText and Fusion with remote access software: https://www.freedomscientific.com/training/enterprise/using-jaws-zoomtext-and-fusion-with-remote-access-software/

### NVDA and its add-ons

NVDA source files share the prefix `https://github.com/nvaccess/nvda/blob/master/source/`.
- [N1] NVDA User Guide (2026.2): https://download.nvaccess.org/documentation/userGuide.html
- [N2] NVDA What's New: https://download.nvaccess.org/documentation/changes.html
- [N3] NVDA 2026.3 beta 1: https://www.nvaccess.org/?p=33468
- [N4] `contentRecog/__init__.py`
- [N5] `contentRecog/recogUi.py`
- [N6] `contentRecog/uwpOcr.py`
- [N7] Local AI image descriptions: https://github.com/nvaccess/nvda/pull/18475
- [N8] "Could be" prefix: https://github.com/nvaccess/nvda/pull/19327
- [N9] Revert: https://github.com/nvaccess/nvda/pull/19425
- [N10] distilvit branch: https://github.com/nvaccess/nvda/pull/19530
- [N11] NV Access roadmap: https://www.nvaccess.org/roadmap
- [N12] NVDA Add-on Store: https://addonstore.nvaccess.org/
- [N13] AI Content Describer: https://github.com/cartertemm/AI-content-describer
- [N14] Vision Assistant Pro: https://github.com/mahmoodhozhabri/VisionAssistantPro
- [N15] AI-Hub: https://github.com/aaclause/nvda-OpenAI
- [N16] XPose image captioner: https://github.com/cmillsap/XPoseImageCaptioner/
- [N17] Tesseract OCR add-on: https://addons.nvda-project.org/addons/tesseractOCR.ar.html (excerpt)
- [N18] NV Access "In-Process", 19 May 2025: https://www.nvaccess.org/post/in-process-19th-may-2025/
- [N19] `_remoteClient/protocol.py`, `serializer.py`, `transport.py`
- [N20] `_remoteClient/session.py`, `localMachine.py`
- [N21] `_remoteClient/client.py`, `input.py`
- [N22] `_remoteClient/secureDesktop.py`, `bridge.py`, `dialogs.py`, `server.py`
- [N23] Design record, end-to-end encryption: https://github.com/nvaccess/nvda/issues/19868 (checked)
- [N24] End-to-end encryption issue: https://github.com/nvaccess/nvda/issues/17784
- [N25] Prototype, closed: https://github.com/nvaccess/nvda/pull/19857
- [N26] Advisory GHSA-m268-mc77-j2cr: https://github.com/nvaccess/nvda/security/advisories/GHSA-m268-mc77-j2cr
- [N27] Advisories GHSA-42v6-wjv6-h3jj and GHSA-vr27-g5ph-xvq2: https://github.com/nvaccess/nvda/security/advisories/GHSA-42v6-wjv6-h3jj, https://github.com/nvaccess/nvda/security/advisories/GHSA-vr27-g5ph-xvq2
- [N28] NV Access relay: https://github.com/nvaccess/remote-server
- [N29] Community relays: https://github.com/jmdaweb/NVDARemoteServer, https://github.com/tech10/nvdaRemoteServer, https://github.com/ragb/nvda-remote-server-rs
- [N30] nvdaremote.com: https://nvdaremote.com/
- [N31] RDAccess: https://github.com/LeonarddeR/rdAccess
- [N32] RD Pipe: https://github.com/leonardder/rd_pipe-rs

### Other screen readers and tools

- [S1] Narrator complete guide: https://support.microsoft.com/en-us/windows/complete-guide-to-narrator-e4397a0d-ef4f-b386-d8ae-c172f109bdb1
- [S2] Narrator basics: https://support.microsoft.com/en-us/windows/chapter-2-narrator-basics-5ff4591e-7b6d-245e-c95d-ce83c0a1a8d4
- [S3] Windows Insider build 26220.7535: https://blogs.windows.com/windows-insider/2026/01/09/announcing-windows-11-insider-preview-build-26220-7535/
- [S4] Windows AI text recognition: https://learn.microsoft.com/en-us/windows/ai/apis/text-recognition
- [S5] Windows AI image description: https://learn.microsoft.com/en-us/windows/ai/apis/image-description
- [S6] Windows AI APIs and Copilot+ PCs: https://learn.microsoft.com/en-us/windows/ai/apis/, https://learn.microsoft.com/windows/ai/windows-ai-comparison
- [S7] Windows.Media.Ocr.OcrEngine: https://learn.microsoft.com/en-us/uwp/api/windows.media.ocr.ocrengine
- [S8] VoiceOver Recognition: https://support.apple.com/guide/voiceover/voiceover-recognition-category-vocbaf20181f/mac
- [S9] Apple newsroom, May 2026: https://www.apple.com/newsroom/2026/05/apple-unveils-new-accessibility-features-and-updates-with-apple-intelligence/
- [S10] VNRecognizeTextRequest: https://developer.apple.com/tutorials/data/documentation/vision/vnrecognizetextrequest.json
- [S11] RecognizeDocumentsRequest: https://developer.apple.com/tutorials/data/documentation/vision/recognizedocumentsrequest.json
- [S12] VisionKit ImageAnalyzer: https://developer.apple.com/tutorials/data/documentation/visionkit/imageanalyzer.json
- [S13] Foundation Models: https://developer.apple.com/tutorials/data/documentation/foundationmodels.json
- [S14] ImageAttachmentContent: https://developer.apple.com/tutorials/data/documentation/foundationmodels/imageattachmentcontent.json
- [S15] A user's Vision language list: https://www.macscripter.net/t/image-png-to-text-through-applescript/74490?page=2 (excerpt)
- [S16] Orca NEWS: https://gitlab.gnome.org/GNOME/orca/-/raw/main/NEWS
- [S17] ocrdesktop: https://man.archlinux.org/man/ocrdesktop.1
- [S18] Be My Eyes for Windows: https://www.bemyeyes.com/?p=1032
- [S19] Copilot Vision desktop share: https://blogs.windows.com/windows-insider/2025/07/15/copilot-on-windows-vision-desktop-share-begins-rolling-out-to-windows-insiders/
- [S21] Screen readers over RDP, WebAIM list: https://webaim.org/discussion/mail_message?id=45135
- [S22] Narrator in a Cloud PC: https://support.microsoft.com/en-us/accessibility/windows/use-a-screen-reader-to-initiate-connection-to-a-cloud-pc
- [S23] SendSAS: https://learn.microsoft.com/en-us/windows/win32/api/sas/nf-sas-sendsas (checked)
- [S24] VoiceOver with Screen Sharing: https://support.apple.com/guide/voiceover/vodff6b84a32/mac
- [S25] VoiceOver Utility, Screen Sharing: https://support.apple.com/guide/voiceover/vo37c1caa6fe/mac
- [S26] Quick Assist with screen readers: https://vi.ie/?p=893555, https://learn.microsoft.com/en-ie/answers/questions/238459/uac-prompt-is-blocked-when-i-use-quick-assist (excerpts)
- [S27] Remote Incident Manager manual: https://manual.getrim.app/
- [S28] Cthulhu Remote plugin: https://git.stormux.org/storm/cthulhu/commit/cb553d3031694f82a96b9b88b0b6a4065622a7ad (excerpt)
- [S29] TeamViewer and screen readers: https://community.teamviewer.com/English/discussion/135825/teamviewer-and-screen-readers-the-blind-left-out-in-the-cold (excerpt)
- [S30] Sinter, EuroSys 2016: https://www.cs.unc.edu/~porter/pubs/sinter16.pdf

### Models, runtimes and the systems' OCR

- [M1] llama.cpp multimodal: https://github.com/ggml-org/llama.cpp/blob/master/docs/multimodal.md (checked)
- [M2] OcrEngine: https://learn.microsoft.com/en-us/uwp/api/windows.media.ocr.ocrengine
- [M3] OCR language packs (PowerToys Text Extractor): https://learn.microsoft.com/en-us/windows/powertoys/text-extractor
- [M4] `windows` crate, OcrEngine: https://microsoft.github.io/windows-docs-rs/doc/windows/Media/Ocr/struct.OcrEngine.html
- [M5] `objc2-vision`: https://docs.rs/objc2-vision/latest/objc2_vision/
- [M6] RecognizeDocumentsRequest and WWDC 2025 session 272: https://developer.apple.com/documentation/vision/recognizedocumentsrequest.md, https://developer.apple.com/videos/play/wwdc2025/272/
- [M7] macOS feature availability: https://www.apple.com/macos/feature-availability/
- [M8] libmtmd introduced: https://github.com/ggml-org/llama.cpp/pull/12849
- [M9] mtmd README: https://github.com/ggml-org/llama.cpp/blob/master/tools/mtmd/README.md
- [M10] Qwen3.5 in llama.cpp: https://github.com/ggml-org/llama.cpp/pull/19468
- [M11] `mtmd.h`: https://raw.githubusercontent.com/ggml-org/llama.cpp/master/tools/mtmd/mtmd.h
- [M12] llama.cpp repository and releases: https://github.com/ggml-org/llama.cpp
- [M13] `llama-cpp-2` features: https://docs.rs/crate/llama-cpp-2/latest/features (checked)
- [M14] `llama-cpp-rs` and its multimodal wrapper: https://github.com/utilityai/llama-cpp-rs
- [M15] mistral.rs: https://github.com/EricLBuehler/mistral.rs, https://docs.mistralrs.dev/reference/supported-models/
- [M16] candle: https://github.com/huggingface/candle
- [M17] `ort`: https://ort.pyke.io/
- [M18] Radeon 780M device lost: https://github.com/ggml-org/llama.cpp/issues/20081
- [M19] Iris Plus regression: https://github.com/ggml-org/llama.cpp/issues/17438
- [M20] ggmlc: https://github.com/monatis/ggmlc (checked)
- [M21] ggmlc roadmap: https://raw.githubusercontent.com/monatis/ggmlc/main/docs/architecture/future_roadmap.md (checked)
- [M22] Qwen3.5-4B and 2B: https://huggingface.co/Qwen/Qwen3.5-4B (checked), https://huggingface.co/Qwen/Qwen3.5-2B
- [M23] Qwen3.5 GGUF files: https://huggingface.co/unsloth/Qwen3.5-4B-GGUF/tree/main, https://huggingface.co/unsloth/Qwen3.5-2B-GGUF/tree/main
- [M24] Qwen3 languages: https://qwenlm.github.io/blog/qwen3/
- [M25] Gemma 4 E4B and GGUF: https://huggingface.co/google/gemma-4-E4B-it, https://huggingface.co/ggml-org/gemma-4-E4B-it-GGUF/tree/main
- [M26] Qwen3-VL: https://huggingface.co/Qwen/Qwen3-VL-4B-Instruct, https://github.com/ggml-org/llama.cpp/pull/16780
- [M27] MiniCPM-V 4.6: https://huggingface.co/openbmb/MiniCPM-V-4.6
- [M28] SmolVLM2: https://huggingface.co/HuggingFaceTB/SmolVLM2-2.2B-Instruct
- [M29] Qwen2.5-VL 3B license: https://huggingface.co/Qwen/Qwen2.5-VL-3B-Instruct/blob/main/LICENSE
- [M30] Mistral Small 3.1: https://huggingface.co/mistralai/Mistral-Small-3.1-24B-Instruct-2503
- [M31] LFM2-VL license: https://huggingface.co/LiquidAI/LFM2-VL-1.6B-GGUF/blob/main/LICENSE
- [M32] Phi-4-multimodal: https://huggingface.co/microsoft/Phi-4-multimodal-instruct
- [M33] PaddleOCR-VL: https://huggingface.co/PaddlePaddle/PaddleOCR-VL
- [M34] PaddleOCR-VL languages: https://www.paddleocr.ai/main/en/version3.x/algorithm/PaddleOCR-VL/PaddleOCR-VL.html
- [M35] PaddleOCR-VL in llama.cpp and its GGUF: https://github.com/ggml-org/llama.cpp/pull/18825, https://huggingface.co/PaddlePaddle/PaddleOCR-VL-1.5-GGUF/tree/main
- [M36] OCRTurk: https://arxiv.org/abs/2602.03693v1 (checked)
- [M37] OCR model cards: https://huggingface.co/zai-org/GLM-OCR, https://huggingface.co/deepseek-ai/DeepSeek-OCR, https://huggingface.co/rednote-hilab/dots.ocr, https://huggingface.co/tencent/HunyuanOCR
- [M38] Tesseract data and quality: https://tesseract-ocr.github.io/tessdoc/Data-Files-in-different-versions.html, https://tesseract-ocr.github.io/tessdoc/ImproveQuality.html
- [M39] PP-OCRv5 languages: https://www.paddleocr.ai/main/en/version3.x/algorithm/PP-OCRv5/PP-OCRv5_multi_languages.html
- [M40] `oar-ocr`: https://docs.rs/crate/oar-ocr/latest
- [M41] `ocrs` and its alphabet: https://github.com/robertknight/ocrs, https://github.com/robertknight/ocrs/issues/204
- [M42] WebAIM screen reader survey 11: https://webaim.org/projects/screenreadersurvey11/
- [M43] llama.cpp Vulkan performance board: https://github.com/ggml-org/llama.cpp/discussions/10879
- [M44] llama.cpp Apple Silicon performance board: https://github.com/ggml-org/llama.cpp/discussions/4167
- [M45] Qwen3.5 image encoding times: https://github.com/ggml-org/llama.cpp/pull/21443

### Cloud providers

SIWC pages share the prefix `https://developers.openai.com/siwc/token-sharing-open-source/`.
- [C1] Sign in with ChatGPT: https://developers.openai.com/siwc
- [C2] Quickstart: https://developers.openai.com/siwc/quickstart
- [C3] DevDay 2026: https://learn.chatgpt.com/docs/whats-new/devday-2026
- [C4] Cookbook, Sign in with ChatGPT: https://developers.openai.com/cookbook/articles/sign-in-with-chatgpt (checked)
- [C5] Plan usage in open-source apps: https://developers.openai.com/siwc/token-sharing-open-source (checked)
- [C6] Sign in with ChatGPT, for users: https://learn.chatgpt.com/docs/sign-in-with-chatgpt (checked)
- [C7] Sign-in flow: `…/sign-in` (checked)
- [C8] Token reference: `…/token-reference`
- [C9] Models and inference: `…/models-and-inference`
- [C10] Preview limitations: `…/preview-limitations` (checked)
- [C11] Errors and recovery: `…/errors-and-recovery`
- [C12] UI and UX guidelines: https://developers.openai.com/siwc/ui-ux-guidelines
- [C13] DevKit license: https://github.com/openai/sign-in-with-chatgpt-devkit/blob/main/LICENSE (checked)
- [C14] Codex's own sign-in and its earlier reuse: https://github.com/openai/codex (login server), https://www.nxcode.io/resources/news/opencode-blocked-anthropic-2026 (news)
- [C15] Anthropic, third-party apps: https://code.claude.com/docs/en/legal-and-compliance
- [C16] OpenAI API data: https://developers.openai.com/api/docs/guides/your-data
- [C17] Gemini API terms: https://ai.google.dev/gemini-api/terms
- [C18] Azure OpenAI data privacy: https://learn.microsoft.com/en-us/azure/foundry/responsible-ai/openai/data-privacy
- [C19] OpenRouter data collection: https://openrouter.ai/docs/guides/privacy/data-collection
- [C20] GitHub device flow: https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps

### Protocols and security

- [O1] Magic Wormhole: https://magic-wormhole.readthedocs.io/en/latest/welcome.html (checked)
- [O2] `spake2` crate: https://docs.rs/spake2/latest/spake2/ (checked)
- [O3] CPace draft: https://datatracker.ietf.org/doc/draft-irtf-cfrg-cpace/
- [O4] OpenSSH 9.5 release notes: https://www.openssh.com/txt/release-9.5
- [O5] `snow` crate: https://docs.rs/snow/latest/snow/ (checked)
- [O6] SetWindowDisplayAffinity: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowdisplayaffinity
- [O7] macOS Sequoia screen recording prompts, 2024: https://9to5mac.com/2024/08/14/macos-sequoia-screen-recording-prompt-monthly/ (news)
- [O8] `keyring` crate: https://docs.rs/keyring/latest/keyring/ (checked)
- [O9] Windows.Data.Pdf PdfPage: https://learn.microsoft.com/en-us/uwp/api/Windows.Data.Pdf.PdfPage

### Local files read

- This repository: AGENTS.md; `docs/references/extension-sdk-research.md`, `extension-sdk-proposal.md`,
  `extension-sdk-vision.md`, `messages-and-verbosity.md`, `helper-ui-architecture.md`,
  `jaws-dialogs-and-visual-features.md`, `virtual-viewer.md`, `forms-and-interaction-modes.md`,
  `verification-loop.md`; `docs/plans/2026-10-18-helper-dialogs-round-2.md`,
  `2026-10-17-helper-dialogs.md`; `.mcp.json`; the code paths cited in §12.
- OpenDesk at `C:\Users\Gigabyte\opendesk` (`ccbd988`): `README.md`; `docs/protocol/` (pairing,
  auth, encryption, transport, session, discovery); `docs/remote/` (security, internet
  transport); `opendesk-rs/src/remote/rendezvous.rs`, `client.rs`, `server.rs`,
  `computer/privacy.rs`, `remote/audit.rs`; `opendesk --help` and the help of `pair`,
  `pair-with`, `serve`, `connect`, `rendezvous`, `discover`, `peers`, `privacy`.
