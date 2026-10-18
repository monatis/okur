# Assistive AI and remote access: proposal

**Status:** proposal for the owner to approve or amend, 2026-10-18. Nothing here is built.

**Builds on:**
- [`assistive-ai-and-remote-research.md`](assistive-ai-and-remote-research.md), cited below as
  "research §n": what JAWS, NVDA, Narrator, VoiceOver and others do, local models, Sign in
  with ChatGPT and OpenDesk, with every source;
- [`extension-sdk-proposal.md`](extension-sdk-proposal.md), cited as "SDK §n": its
  milestones M0 to M10, the settings registry, verbosity, drivers and permissions;
- [`helper-ui-architecture.md`](helper-ui-architecture.md): dialogs drawn by `okur-ui`, the
  Virtual Viewer as the Results Viewer;
- [`messages-and-verbosity.md`](messages-and-verbosity.md): the catalog and output types.

**Clean room.** Unchanged from AGENTS.md §1.4. Everything about JAWS here comes from its
documentation, its key map and its behavior. JAWS's engines and providers are not copied:
Okur matches JAWS's keys, messages and results, and chooses its own engines.

## Contents

1. The decisions asked of you
2. Goals and non-goals
3. Privacy and consent
4. Architecture: one path from a picture to words
5. OCR: Convenient OCR
6. Descriptions: Picture Smart
7. Local models
8. Cloud providers
9. Remote access: Okur Remote
10. Other JAWS utilities: verdicts
11. Messages, settings and verbosity
12. Verification
13. Roadmap
14. Risks
15. Open decisions in detail

---

## 1. The decisions asked of you

Each has a recommended default; §15 gives the reasons. "A" numbers keep them apart from the
SDK's D1 to D18.

| # | Decision | Recommended default |
|---|---|---|
| A1 | OCR with nothing installed | The operating system's engine: Windows.Media.Ocr, Apple Vision |
| A2 | Keys and messages | JAWS's layers exactly, with JAWS's command names; Okur's additions on letters JAWS leaves free |
| A3 | Where results go | The Results Viewer, as JAWS; OCR laid over the window as a second step |
| A4 | Local models | Not bundled; downloaded on request from a list Okur tests, the size said first |
| A5 | The first local models | Describing: Qwen3.5 4B (2B on 8 GB machines), against Gemma 4 E2B/E4B in V3's evaluation; OCR: PaddleOCR-VL 1.5 |
| A6 | The local runtime | llama.cpp's libraries in a separate process, `okur-models`; ggmlc when it runs vision models with Metal, Vulkan and a C interface |
| A7 | Cloud providers in Okur itself | Sign in with ChatGPT, and one client for any OpenAI-compatible API key; others as extensions |
| A8 | Each cloud request | Announced before it goes, always; asking Yes or No each time is a setting, off |
| A9 | Saying where a result came from | Always, as the result's first line; not an output type |
| A10 | Remote: who carries the traffic | Directly on a local network; otherwise a relay that only forwards ciphertext |
| A11 | Remote: a public relay run by the project | Not until the protocol has had an outside review |
| A12 | Remote: pairing and consent | One-time codes through a PAKE; stored keys for one's own machines; consent on the controlled machine every session |
| A13 | Remote: one transport with OpenDesk | Extract OpenDesk's protocol and rendezvous into a crate both use, moved to a PAKE and a Noise pattern |
| A14 | Remote: what crosses | Announcements as typed parts, braille, physical keys; never audio or pixels |
| A15 | Remote: keyboard layout | The controlled machine's (scan codes), with a per-session option for the controller's |
| A16 | Working with NVDA Remote or JAWS Tandem | No |
| A17 | Remote desktop sessions (RDP, Citrix) | Later, a virtual channel carrying Okur Remote's messages |
| A18 | An assistant for Okur's own help (Vispero Companion's role) | Later, as an extension, local first |
| A19 | The SDK's non-goal "no language model inside Okur" | Amend: models run only in `okur-models`, only on the user's command |
| A20 | Timing | V0 with SDK M1; V1 after M3a; V3 after M6; R1 after M4; V5 with the macOS port |

## 2. Goals and non-goals

### Goals

1. **JAWS's commands.** A JAWS user presses the keys they know for OCR, Picture Smart and
   remote help, hears the messages they know, and finds results where they expect them.
2. **Useful with nothing to download and no account.** The operating system's OCR works on
   the first day, on Windows and macOS.
3. **Private by default.** Nothing Okur reads leaves the machine unless the user chose a cloud
   service, and the user hears it each time.
4. **Local AI a laptop can run.** Descriptions and hard OCR from small GGUF models, offline, in
   Turkish as well as English.
5. **The user's own accounts when they want more.** A ChatGPT plan through Sign in with
   ChatGPT, or an API key.
6. **Help from a distance.** One blind person helps another, or controls their own second
   computer, hearing it in their own voice and language, without opening ports and without a
   server that can read the session.
7. **Windows and macOS equally**, Linux where it is cheap.
8. **Provable.** Hermetic tests, live checks and, where JAWS has the feature, old-pc
   scenarios.

### Non-goals

1. A general chat assistant. Descriptions and questions are about a picture the user chose.
2. A model in the installer.
3. An Okur cloud service, account system or billing.
4. Audio or video of the screen in remote sessions.
5. Joining JAWS Tandem or NVDA Remote sessions (A16).
6. Reading what a window asks not to be captured, or secure screens.
7. JAWS's engines. Okur matches JAWS's keys, messages and results, not its OCR engine or AI
   providers.

## 3. Privacy and consent

A screen reader sees everything on the screen: bank pages, medical letters, private
messages. A sighted person glances at a picture before sending it to a service; a blind user
cannot. So Okur never sends what the user has not chosen to send, and always says where an
answer came from.

### 3.1 OCR and descriptions

1. **Local first.** With settings untouched, nothing Okur reads leaves the machine: OCR uses
   the system's engine, descriptions a local model once one is installed.
2. **Cloud only by choice.** A cloud provider works only after the user turns it on in Settings
   Center and accepts Okur's one-page explanation: what is sent, to whom, when, and where the
   provider's data terms are. An application or site scope, a settings preset or an extension
   cannot turn it on.
3. **Every cloud request is said before it goes**: "Sending the window to ChatGPT." Verbosity
   cannot silence it.
4. **The smallest picture.** The control, the window or the screen: the key decides, never a
   larger area "for context".
5. **The source is always said**, as the first line of every result: "Windows OCR, Turkish",
   "Described by <model>, on this computer", "Described by ChatGPT".
6. **Protected fields are masked.** Before a picture goes to any engine, local or cloud, the
   rectangles of password fields in it are painted over, and their text never enters a
   prompt. This needs a protected state in Okur's object model (UI Automation's
   `IsPassword`, AX's secure text field), which only harness tools read today (research §12).
7. **Secure screens never.** No capture on the sign-in screen, UAC prompts or macOS secure
   input; the command says why.
8. **Windows that exclude themselves from capture stay excluded.** Microsoft says this flag is
   not a security guarantee [research O6], so Okur treats it as a request it honors, not a
   protection it promises.
9. **Nothing kept.** Pictures are not stored. Results live in the Results Viewer until
   replaced; a history exists only if the user turns it on.
10. **Keys and tokens in the system's credential store** (Windows Credential Manager, the macOS
    Keychain, the Secret Service on Linux, through the `keyring` crate [research O8]); never in
    a settings file, a log, a trace, a replay file or the developer interface.
11. **Local models are local.** `okur-models` opens no connection except to download a model
    the user chose, from the address in Okur's model list, checked by hash.

### 3.2 Remote sessions

1. **The controlled machine's user decides.** Every session asks them by default, names the
   controller, and can be ended by them with one key.
2. **Help sessions start view-only**: the helper hears; the controlled user gives control.
3. **End-to-end encryption**: no relay, Okur's or anyone's, can read or change a session.
4. **Protected fields stay protected**: typing in a password field is never echoed to the
   controller, whatever the controlled user's typing echo says.
5. **Nothing recorded** beyond a list of sessions (who, when, how long), as JAWS Tandem logs
   (research §6.1).
6. **The relay learns as little as possible**: no names, no list of machines, padded key
   frames.

## 4. Architecture: one path from a picture to words

### 4.1 The steps

Every OCR and description command takes the same five steps:
1. **Source**: the focused control, the window, the screen, a file selected in File Explorer
   or Finder, an open PDF, the clipboard's picture, the camera.
2. **Capture**: pixels (or a page rendered to pixels) and, for screen sources, where each pixel
   is on the desktop.
3. **Guard**: the rules of §3 decide whether the request may go to the chosen engine, and what
   to mask.
4. **Engine**: an OCR engine returns lines and words with rectangles; a describer returns text
   and takes follow-up questions.
5. **Result**: the Results Viewer, or OCR laid over the window as a virtual document, with
   its source said.

### 4.2 Crates and programs

- **`okur-vision`** (new, platform-neutral): requests and results, the two engine traits, the
  guard, the engine registry, mocks. No platform or network code.
- **`okur-platform`** gains capture traits, each with Windows and macOS drivers and a mock
  that returns fixture images:
  - `ScreenCapture`, over xa11y's `ScreenshotProvider` (research §12) and the element and
    window rectangles Okur already reads;
  - `DocumentRender`: a PDF page to pixels (Windows.Data.Pdf [research O9]; PDFKit on macOS);
  - `CameraCapture`: one frame (Media Foundation; AVFoundation).
- **OS OCR drivers** in `okur-platform`: Windows.Media.Ocr, Apple Vision.
- **`okur-models`** (new program): the process host for local models (§7.2).
- **`okur-providers`** (new, V4): the cloud clients, Sign in with ChatGPT, the credential
  store, the consent records. The only crate that opens connections for descriptions.
- **`okur-core`**: the commands and layers, the Results Viewer's content, the in-place OCR
  document, the remote session in key routing, the messages.
- **`okur-ui`**: the question dialog, the setup and consent pages, the model download dialog,
  the remote dialogs. It needs the edit control the helper UI decision already lists.
- **`okur-remote`** and the shared transport (§9.4).

**Platforms.** Every trait gets a Windows and a macOS driver against the same mocks and tests;
the macOS drivers are written with the macOS port (V5), as the SDK places it. Linux, best
effort, has no system OCR: its OCR is a local model or Tesseract (research §9.1, §9.4).

### 4.3 The engine traits (a sketch, not an implementation)

```
trait TextRecognizer {                      // OCR
    fn id(&self) -> EngineId;               // "windows-ocr", "apple-vision", "local:<model>"
    fn locality(&self) -> Locality;         // OnDevice | Cloud(ProviderId)
    fn languages(&self) -> Vec<LanguageTag>;
    fn recognize(&self, image: Image, hints: OcrHints) -> RequestId;   // answer as an event
}

trait Describer {                           // Picture Smart
    fn id(&self) -> EngineId;
    fn locality(&self) -> Locality;
    fn describe(&self, images: Vec<Image>, prompt: Prompt, language: LanguageTag) -> RequestId;
    fn ask(&self, conversation: ConversationId, question: String) -> RequestId;
}

struct OcrResult {
    engine: EngineId,                       // said to the user
    language: Option<LanguageTag>,
    lines: Vec<OcrLine>,                    // words with rectangles in desktop pixels
}
```

- Requests are asynchronous, as Okur's page queries are (SDK §4.4): the engine thread never
  waits. Results arrive as events; Q in the layer, or a newer request, cancels.
- `Locality` drives the guard and the spoken source.
- Engines are drivers in a registry, like speech drivers (SDK §5.6, M6). For the SDK this adds
  a driver kind, the **recognizer driver**: it runs in a process host, and a cloud one needs
  the SDK's network permission for its host only (SDK §9.3).
- NVDA's content recognition framework has the same shape (research §7.1): one job at a time,
  a capture that remembers where it was on screen, results as lines of words with rectangles.

## 5. OCR: Convenient OCR

### 5.1 Keys

Insert+Space, O, then (JAWS 2025's key map; research §2, §3):

| Key | Command | Recognizes | Milestone |
|---|---|---|---|
| W | `RecognizeRealWindow` | the window in front | V1 |
| C | `RecognizeControl` | the focused control | V1 |
| S | `RecognizeScreen` | every screen | V1 |
| Q | `CancelRecognition` | stops a recognition | V1 |
| ? | `OCRLayerHelp` | the layer's keys, in the Results Viewer | V1 |
| F | `OCRSelectedFile` | the image or PDF selected in File Explorer or Finder | V2 |
| D | `OCRDocument` | the PDF open in front | V2 |
| Enter | `OCRAllInOne` | by context: the selected file, the open PDF, otherwise the window | V2 |
| B | `StartBackgroundRecognition` | the window again and again, read in place | V2 |
| Ctrl+B, Ctrl+Enter | `…WithSound` | as B and Enter, with a soft sound while working | after SDK M5 |
| A | `RecognizeCamera` | a frame from the camera | V5 |
| R | `OCRSelectedFileToWord` | the file into a Word document | not planned; copy from the viewer |

- Command names are JAWS's, so the key file Okur reads (SDK §7.2) keeps them.
- JAWS's own words (while working, when done, nothing found, cancelled, no file selected) are
  recorded on old-pc before Okur's are written, and go into the catalog as `ocr-*` messages.
- Background OCR in JAWS also reads YouTube's video area and shared screens in Teams and Zoom
  (research §3.1); Okur's B starts with the window, and those application cases come with
  their application modules.

**Okur's additions**, on letters free in JAWS's OCR layer: **L** chooses the recognition
language for this command only; **E** recognizes again with the next engine (the system's,
then a local model), saying which.

### 5.2 Results

- **The Results Viewer** (Okur's Virtual Viewer window): the first line says the engine and the
  language; then the text by lines, read with the virtual cursor; Ctrl+F finds, as in JAWS
  since 2025; Enter on a word clicks it on screen (rectangles are kept); Escape closes.
- **In place** (B): the text becomes a virtual document over the window, as NVDA's results are
  (research §7.1); the virtual cursor reads it, Enter clicks, and it is refreshed while it
  stays open; any key that leaves it (a return to the PC cursor, a new page, the focus moving)
  ends it, as JAWS's does.
- **Nothing found**: JAWS's message, no viewer.
- **Many pages**: progress by page, as an output type the user can silence.

### 5.3 Engines

1. **Windows.Media.Ocr** (V1): on every Windows 10 and 11 machine, offline. Its languages are
   the installed OCR packs; the dev box has the Turkish one, and images may be up to 10,000
   pixels on a side (research §9). When the user's language is missing, Okur says so and
   offers the Windows Settings page that adds it.
2. **Apple Vision** (V5): `RecognizeDocumentsRequest` on macOS 26 and later,
   `VNRecognizeTextRequest` before. Turkish is to be checked on a Mac (research §8.2).
3. **Windows' `TextRecognizer`** on Copilot+ PCs: only if Okur is ever packaged as it
   requires (research §8.1).
4. **A local model** (V3) for what the system's engine reads badly: dense layouts, small text,
   tables, handwriting (§7).
5. **A cloud model** (V4), only by the user's choice (§8).

`ocr.engine` chooses the default; `ocr.language` defaults to Okur's speech language when the
engine has it, else the system's language, else English.

## 6. Descriptions: Picture Smart

### 6.1 Keys

Insert+Space, P, then a source (research §2, §4.1):

| Key | Source |
|---|---|
| C | the focused control (an image on a page, an unlabeled button) |
| W | the window in front |
| S | the screen |
| F | the image selected in File Explorer or Finder |
| B | the picture on the clipboard |
| A | the camera (V5) |
| Enter | by context: the focused image, the selected file, otherwise the window |
| ? | the layer's keys |

Modifiers, as JAWS's:
- **Alt + source**: ask first. A dialog with one edit field ("Question:") opens; Enter sends
  the picture with the question.
- **Shift + source** (JAWS: both services): every enabled engine answers, one after another,
  each under its source.
- **Ctrl + source** (JAWS: the older Picture Smart, which some users find better for text in
  pictures): Okur has no older service, so Ctrl gives OCR of the same source, which reads the
  text exactly (an Okur choice, for the same need).

### 6.2 Results and questions

- The answer opens in the Results Viewer: the source line ("Described by <model>, on this
  computer"), a short description, then links as JAWS's: "Get more details", "Ask questions
  about this image".
- **A question**: Enter on the link opens an edit field; Enter sends; the answer is read and
  added below, so the viewer holds the conversation; H and Shift+H move between questions
  and answers, as in JAWS (research §4.1). The picture is not taken again.
- The conversation ends when the viewer closes. Nothing is kept unless the user turns on a
  history.

### 6.3 Engines

- A local model (V3), the default once installed.
- A cloud model (V4), by the user's choice.
- With neither, Insert+Space, P says that no description engine is set up and offers the setup
  dialog (§7.1). It never falls back to a cloud service by itself.

### 6.4 The AI Labeler, later

Insert+G on an unlabeled control or image (JAWS 2026's AI Labeler, research §4.2): the
describer proposes a short label, Okur speaks it with the role and state, and the custom
labels store (helper dialogs plan step 8) keeps it, marked as generated. Built after V3 and
the custom labels.

## 7. Local models

### 7.1 How a user gets one

- Nothing is bundled. The first Picture Smart command without an engine opens a setup dialog:
  "Describe pictures on this computer" (download a model, with its size and needs said in
  words: "about 3 GB; works best with 16 GB of memory") or "Use an online service" (§8).
- Okur keeps a **model list**: models its maintainers have tested, each with its download
  address, file sizes, SHA-256 checksums, license, languages, memory needed, and the error
  rates and fact-list scores Okur's evaluation measured (§12). The list is data, updated with
  Okur.
- Downloads go to Okur's data folder and are checked before use; settings remove them. A user
  may add a GGUF file of their own, marked "not tested by Okur".

### 7.2 The process host, `okur-models`

- A separate program, as `okur-ui` is: a crash, a hang or running out of memory in model code
  ends it, never speech. Okur restarts it on the next request.
- It links llama.cpp's libraries (the language model and the multimodal library, `libmtmd`
  [research M1]) with the platform's GPU backend: Metal on macOS, Vulkan on Windows (any GPU
  vendor), the CPU otherwise. From Rust, through the `llama-cpp-2` crate's `mtmd`, `vulkan` and
  `metal` features, pinned to exact versions, since llama.cpp marks its multimodal interface
  experimental (research §9.2).
- It loads a model on first use and unloads it after a few idle minutes, so memory is free
  while the user works.
- It speaks a small protocol over a pipe: load, describe (images, instruction, history),
  recognize (image, language), cancel, progress, result.
- **ggmlc** (research §9) can later supply small compiled models (a text detector and
  recognizer, an image encoder) to the same host, once it has the GPU backends and an
  interface the host can call.

### 7.3 What a local model does

- **Describer**: the picture, Okur's instruction in Okur's language ("Describe this screen for
  a blind user: the window, its controls and any text"), the user's question if any. The
  instructions are catalog messages, translated with everything else.
- **OCR engine**: the same or an OCR model, told to transcribe exactly in reading order. Its
  output may have no word rectangles; then the Results Viewer cannot click, and says so.
- **With the accessibility tree** (to measure before it becomes a default): Okur can add the
  names and roles it knows to the instruction, so the model describes what the tree lacks.

### 7.4 The models

From research §9.4. All are Apache-2.0 and in llama.cpp today; the first list ships after V3's
evaluation measures them in English and Turkish on the dev box and a Mac.

- **Describing, 16 GB machines: Qwen3.5 4B** (about 3.4 GB with its projector). Strong on text in
  pictures (OCRBench 85) and on screens (ScreenSpot Pro 60). Its card claims 201 languages but
  does not name Turkish: measured before it is the default.
- **Describing, 8 GB machines: Qwen3.5 2B** (about 2 GB).
- **The alternative to measure: Gemma 4 E2B and E4B** (about 3.4 and 5.2 GB). Their image-token
  budget (70 to 1,120) is the cleanest speed control on a weak GPU.
- **Very small machines: MiniCPM-V 4.6** (about 1.6 GB), if its quality holds.
- **Local OCR: PaddleOCR-VL 1.5** (about 1.8 GB): Turkish listed among 109 languages, and the
  best overall in the one Turkish OCR benchmark (OCRTurk, 2026).
- **Left out**: Qwen2.5-VL 3B (non-commercial license), LFM2-VL (license limits), SmolVLM
  (English only), Phi-4-multimodal (not in llama.cpp), and larger models such as Mistral
  Small 3.1, which names Turkish but needs a desktop GPU.

**Speed.** Encoding the picture is the slow part: a whole 1080p screenshot is about 2,000 image
tokens for Qwen3.5 (research §9.5). Okur sends the control or the window the key chose, and caps
image tokens (a setting, default to be measured). The time budget V3 must meet is set from the
dev box's and a Mac's measurements, with `okur latency`.

## 8. Cloud providers

### 8.1 Which, and where they live

- **In Okur itself (V4):**
  - **Sign in with ChatGPT**, for users with a ChatGPT plan. Okur fits the groups OpenAI's
    pages admit without a waitlist (open source; runs on the user's machine; research §10.1).
    Okur writes its own client from OpenAI's protocol pages and uses none of the DevKit's
    code, whose license is noncommercial.
  - **An API key for any OpenAI-compatible endpoint**: OpenAI, OpenRouter, or a llama.cpp
    server or Ollama on the user's own network. One client covers them.
- **As extensions, later (SDK M8 onward):** other providers (Anthropic, Google, Azure), each a
  recognizer driver with the network permission for its host only. Anthropic allows only API
  keys in third-party apps (research §10.3).

### 8.2 Signing in with ChatGPT, as the user hears it

1. Settings Center, Picture Smart, "Use ChatGPT": Okur's consent page first (§3.1, rule 2):
   what is sent (the picture and the question), when (only when a Picture Smart key is
   pressed with ChatGPT chosen), that it counts against the ChatGPT plan, and that OpenAI's
   terms apply. Buttons "Sign in with ChatGPT" and "Cancel".
2. Okur opens OpenAI's page in the user's browser, which Okur reads like any page, and waits
   on a loopback address for the answer.
3. Back in Okur: "Signed in to ChatGPT as <name>. Plan usage allowed." Or, if the plan
   permission was not granted, it says so and offers to try again.
4. Tokens go to the system's credential store; signing out revokes them.
5. Over the limit, Okur says OpenAI's reason in words and offers the "Manage usage" link OpenAI
   asks apps to show.

### 8.3 What Okur sends

- The picture, cropped to the chosen source, at the smallest size that keeps text legible.
- Okur's instruction and the user's question; for follow-up questions, the earlier questions
  and answers again, and the picture (plan usage keeps no conversation).
- `store: false` on every request, as OpenAI requires.
- Never by default: the accessibility tree's text, file paths, window titles beyond what is in
  the picture, settings.

**Not confirmed** (research §10.2): whether plan-usage requests are used for training and how
long they are kept. The consent page says so plainly until OpenAI documents it.

## 9. Remote access: Okur Remote

### 9.1 What the user does

Two uses, one feature:
- **Getting or giving help.** The person being helped presses Insert+Alt+T (JAWS Tandem's
  key) and chooses "Get help"; Okur speaks a one-time code in groups ("4 8 1, 2 2 7, 9 0
  3"), which they read to the helper by phone. The helper presses Insert+Alt+T, chooses "Give
  help" and types it. The helped machine asks its user: "<helper's name> wants to hear this
  computer. Allow, Deny." The helper hears; control comes when the helped user gives it.
- **My other computer.** A user pairs two of their own machines once, with the same kind of
  code; afterwards Insert+Alt+T, "Connect to", and the machine's name.

In JAWS, Insert+Alt+T opens the target's "Allow Access to My Computer" dialog and ends a
session, while the controller starts from JAWS's Utilities menu (research §6.1). Okur puts both
sides behind Insert+Alt+T, in one small dialog: an addition, since Okur has no Utilities menu.

While connected:
- The controller hears the other machine's Okur in their own voice, rate, verbosity and
  language, with a pitch change so they always know which machine is talking (as VoiceOver
  and RDAccess do; research §7.5, §8.2).
- **Insert+Alt+Tab** (Tandem's key) switches the keyboard between the two machines; Okur says
  which ("Remote computer", "Local computer"; placeholders until Tandem's words are recorded).
- **Insert+Alt+T** ends the session on either side.
- The controlled machine's Okur keeps reading for whoever sits there; either side can mute.
- Braille (after SDK M6): the controller's display shows the other machine's braille; its keys
  act there.
- Clipboard text goes across only on an explicit command. No files.
- Ctrl+Alt+Delete for the other machine is a menu command (a keyboard hook never sees that
  chord); the other side performs it through Okur's service if its policy allows (research
  §7.4).
- The other machine's screen can be blacked out for privacy, as OpenDesk's curtain.

### 9.2 What crosses the link

- **Speech as typed announcements.** The controlled Okur sends each announcement after
  interceptors and before verbosity (SDK §6.1); the controller renders it with its own
  catalog, verbosity and synthesizer. A Turkish helper hears Okur's own words in Turkish on an
  English user's machine; the application's words stay as they are. Until all speech is typed
  (M0's remaining work), the rendered text with its role marks.
- **Keys as physical keys**: scan code, extended flag, down or up, modifiers, time. The
  controller's hook consumes every key while controlling, except Insert+Alt+Tab and
  Insert+Alt+T. The controlled Okur puts each key through its own key routing first, so its
  commands work (Insert+F7 opens its Links List), and injects the keys it passes on by scan
  code, so its own keyboard layout applies (A15).
- **Braille cells and braille input** (after M6); **sound ids** (after M5), played from the
  controller's own sounds, as Tandem and NVDA do.
- **Never**: audio, pixels, files.

### 9.3 Security

The design assumes:
- **The relay is not trusted.** Anyone may run it; it may log or be compromised.
- **The network is not trusted**, the local one included.
- **The controlled machine's user is the authority.**

| Threat | Answer |
|---|---|
| The relay reads or changes the session | End-to-end encryption; the relay never holds a key |
| The relay or a LAN attacker sits in the middle of pairing | A PAKE: one guess per code; codes expire in minutes; words to compare aloud |
| Someone guesses a session code | Nine digits, one use, ten minutes, and the consent dialog |
| A stolen laptop keeps a pairing | Pairings listed in settings, removable from either side; "allow without asking" off by default |
| The relay learns typing rhythm | Key frames padded and sent at a steady rate while controlling (OpenSSH does the same; research §11.4) |
| A bug in frame parsing | A small decoder of typed messages, fuzzed; frames never carry commands to run |
| The helper does too much | View-only first; the controlled user gives and takes control; the session list |
| Ctrl+Alt+Delete, UAC | Only through Okur's service and the machine's policy; secure screens read only when Okur runs there |

**What the relay sees**, in this design: the two machines' IP addresses, when they connect and
for how long, the padded sizes of frames, and an identifier that changes with every code or
pairing. Not names, not descriptions, not keys, not content. OpenDesk's relay today also sees
names, descriptions and stable public keys, and lists them (research §11.4); NVDA's relay sees
everything (research §7.4).

### 9.4 Architecture

- **`okur-remote`** (new, platform-neutral): the session state machine, frames, consent
  records, mocks.
- **The transport, shared with OpenDesk (A13):** connection to a relay, direct connections,
  discovery, the handshake. Extracted from OpenDesk into one crate both use, with these
  changes (research §11):
  1. pairing and session codes through a PAKE (CPace or SPAKE2);
  2. the channel as a named Noise pattern through the `snow` crate [research O5]: IK for
     paired machines, XX with the PAKE's key as pre-shared key on first contact; so the design
     can be checked against a published framework;
  3. the relay sees an opaque identifier derived from the code or the pairing, never a name or
     a description, and has no "list" request;
  4. mDNS only while waiting for a session, without a stable key;
  5. TLS to the relay as well, so a network observer learns only the relay's address.
- **`okur relay`**: the relay as a subcommand of Okur's binary, the same code as OpenDesk's
  rendezvous, so one relay can serve both programs.
- **`okur-core`**: the remote session as step 0 of key routing ("Who takes a key"); forwarding
  announcements; rendering remote ones with a "remote" voice context (SDK §6.4).
- **`okur-platform`**: key injection by scan code (SendInput on Windows, CGEvent on macOS);
  SendSAS through Okur's service; the curtain.
- **`okur-ui`**: Get help, Give help, Allow, Paired computers.

Frames, a sketch (MessagePack, as OpenDesk's):

```
Hello { protocol, okur_version, platform, language, capabilities }
Speech { announcement: TypedParts | Text, context: VoiceContext, interrupt: bool }
CancelSpeech
Sound { id }                                   // after SDK M5
Braille { cells, cursor }                      // after SDK M6
BrailleInput { routing: Option<u16>, gesture: Option<GestureId> }
Key { scan_code, extended, down, modifiers, at_ms }
ReleaseHeldKeys
Clipboard { text }                             // on an explicit command only
Control { kind: Request | Grant | Revoke }
Curtain { on: bool }
SecureAttention                                // Ctrl+Alt+Delete, through Okur's service
End { reason }
Padding { bytes }
```

### 9.5 How this differs from JAWS Tandem and NVDA Remote

- **Speech**: Tandem needs the same synthesizer and language on both sides; NVDA sends speech
  sequences for the controller's synthesizer; Okur sends typed announcements, rendered in the
  controller's language and verbosity.
- **Keys**: Tandem uses the controller's layout on both sides; NVDA sends virtual-key codes and
  the controlled side makes up scan codes; Okur sends scan codes into the controlled Okur's
  own routing first.
- **The server**: Tandem's is Freedom Scientific's, said to be unable to read sessions; NVDA's
  relay can read and inject today, by NV Access's own account; Okur's relay forwards only
  ciphertext.
- **The secret**: Tandem's meeting ID comes from its server; NVDA's key, often 7 digits, goes to
  the relay; Okur's code goes through a PAKE and is never sent.
- **Consent**: Tandem's target types the ID; NVDA's joining is consent; Okur asks on the
  controlled machine every session by default.
- **One's own machines**: Tandem has none; NVDA auto-connects with a stored key in plain text;
  Okur stores public keys, as OpenDesk.
- **Ports**: Tandem Direct and NVDA's local hosting open a port; Okur connects out.
- **Cost**: Tandem Center is free and Tandem Direct is paid; NVDA and Okur are free, with a
  relay anyone can run.

### 9.6 Remote desktop sessions, later

When Okur runs inside an RDP or Citrix session and the user's own machine runs Okur too, the
same messages go over a virtual channel, as JAWS's remote access and RDAccess do (research
§6.2, §7.5). One Okur speaks, with its own voice; the session's Okur reads. After R2.

## 10. Other JAWS utilities: verdicts

- **Screen Shade** (Insert+Space, F11): **yes, soon, core.** Privacy in public, and the curtain
  of remote sessions.
- **Mouse Echo** (Insert+Space, E): **yes, core**, with routing and the mouse (helper dialogs
  step 6). Local.
- **Speech History** (Insert+Space, H): **yes, core.** Okur keeps speech logs already; a user
  command shows the history in the Results Viewer.
- **Notification History** (Insert+Space, N): **yes, core**, with notification events (SDK M2).
- **Clipboard text** (Insert+Space, C) and **Commands Search** (Insert+Space, J): **yes,
  core**; Commands Search from the key table (SDK M3).
- **AI Labeler** (Insert+G): **yes, after V3** and custom labels (§6.4).
- **Page Explorer** (JAWS 2026, Insert+Shift+E): **later**: a page summary from the virtual
  buffer's text by a local model, cloud by choice; keys and messages recorded once old-pc has
  JAWS 2026.
- **Text Analyzer** (Insert+Space, A; Alt+Windows+I): **later, core**, for word processors.
  Local, no AI.
- **Research It** (Insert+Space, R): **extension**, after SDK M9, with network permission per
  source.
- **Vispero Companion** (Insert+Space, F1): **extension, later**, over Okur's own documentation,
  local model first (A18).
- **Face in View** (Insert+Space, F): **later, extension**: the camera, a local face detector, a
  few messages. Useful before video calls, small, no cloud.
- **Smart Glance, Skim Reading, Flexible Web**: in the helper dialogs plan, step 11.
- **Lock Keyboard, audio ducking, volume layer**: **yes, core**, small platform features.
- **Voice Assistant** ("Sharky"): **no.** Speech recognition is another product.
- **Split Braille**: with braille, after the first braille driver.
- **The AI agent** JAWS is developing (research §4.2): **no, for now.** Okur's agents work from
  outside, through the developer interface (SDK §4.6).

## 11. Messages, settings and verbosity

### 11.1 Messages

- New catalog files: `recognition.ftl` (OCR and descriptions) and `remote.ftl`.
- Where JAWS has the command, the English is JAWS's, recorded on old-pc first. Where it does not
  (sources, consent, downloads, pairing), Okur writes its own, short, in JAWS's style.
- Sources are messages with the engine's name as an argument:
  `recognition-source-local = Described by { $model }, on this computer`. Model names are data,
  never translated.
- The instructions Okur gives models are catalog messages too, so a Turkish user's model is
  asked in Turkish.

### 11.2 Settings (the registry, SDK M1)

| Id | Default | Notes |
|---|---|---|
| `ocr.engine` | the system's engine | global or per application |
| `ocr.language` | Okur's language if the engine has it | global or per application |
| `ocr.say_all_on_result` | off | as NVDA's option |
| `describe.engine` | none until set up | global |
| `describe.language` | Okur's speech language | global |
| `describe.detail` | brief | brief or detailed |
| `describe.ask_before_cloud` | off | Yes or No before each cloud request (A8) |
| `cloud.<provider>.enabled` | off | saved only; Settings Center only |
| `models.unload_after_minutes` | 5 | |
| `models.gpu` | auto | auto or off |
| `results.history` | off | |
| `remote.incoming` | ask | off, ask, or paired machines without asking |
| `remote.relay` | none | local network only until set |
| `remote.distinct_pitch` | on | remote speech at another pitch |

`cloud.*` and `remote.incoming` cannot be set by an application or site scope, a settings
preset or an extension: only by the user in Settings Center.

### 11.3 Verbosity

New output types, on at every level unless JAWS's dialog shows otherwise:
`recognition_progress` ("Recognizing", page progress) and `remote_status` (connections,
switching). The source line, consent and "Sending … to …" are not output types: verbosity
cannot silence them.

## 12. Verification

AGENTS.md §3 holds: `cargo test` stays hermetic; live tests are `#[ignore]`d; claims quote
tools; JAWS evidence comes only from old-pc's capture tooling.

### 12.1 OCR

- **Hermetic.** A mock capture returns committed fixture images; a mock recognizer returns
  results recorded once from the real engines. Tests cover everything after recognition: the
  Results Viewer's lines and source line, the in-place document's order and rectangles, the
  point a click goes to, the messages, cancellation, the guard (masking, secure screens).
- **Engine quality, live.** A new `okur ocr-eval <engine> <set>` prints the character error rate
  per image and overall; a live test fails when it rises above the committed figure, a ratchet
  like parity's. The set: rendered text in English and Turkish (ç, ğ, ı, İ, ö, ş, ü), the
  corpus fixtures' screenshots, a scanned page, a photographed page.
- **Against JAWS** (old-pc runs JAWS 2025 with Convenient OCR): scenarios of W, C, S, F, D and
  Q on fixtures whose text is only pixels (a window drawing text as an image, an image file,
  an image-only PDF). Compared: JAWS's spoken messages exactly; the recognized text against
  the known text by error rate, since JAWS's engine is its own.

### 12.2 Descriptions

- **Hermetic.** A mock describer returns fixed text. Tests: the command flow, the question
  dialog, the conversation (history resent), consent (no request without it), the announcement
  before a cloud request, the source line, cancellation, errors (over the limit, offline, model
  missing) and their messages. A mock OpenAI server (in memory) covers sign-in, token refresh,
  limits and errors.
- **Model quality, live.** Each test image has a list of facts a description must contain ("a
  button labeled Send", "the chart's title") and phrases it must not ("I cannot see"),
  checked by keywords; local models at temperature 0 with a fixed seed. Run on the dev box
  with the owner's permission, and on CI machines with a GPU if one is available.
- **Against JAWS**: old-pc scenarios record Picture Smart's messages and the Results Viewer's
  links on a fixture image, never the description's words (they come from JAWS's providers).
  Picture Smart needs JAWS's online service and, in the United States, an account (research
  §4.1); whether old-pc's JAWS can reach it is checked first.

### 12.3 Remote

- **Hermetic.** The protocol over in-memory pipes: pairing with the right and the wrong code, a
  tampered frame, a replayed frame, a relay that changes bytes, a second controller refused,
  consent denied. Two engines in one test, each with a mock hook and mock speech: keys typed on
  one reach the other's routing, and the other's speech is spoken by the first in its own
  language. The frame decoder is fuzzed.
- **Live**: the dev box controls old-pc's Okur through a relay, with the owner's permission;
  `okur latency` reports key to speech across the link.
- **Against JAWS**: Tandem needs JAWS on both machines. With the owner's permission, the dev
  box's JAWS and old-pc's could record Tandem's messages once ("Tandem connection open" and
  the rest); they become Okur's English where the features match.

## 13. Roadmap

V is for OCR and descriptions, R for remote. Each milestone ends with tool output that proves
it. None is in the current plan,
[`2026-10-18-helper-dialogs-round-2.md`](../plans/2026-10-18-helper-dialogs-round-2.md); they
follow it and fit between the SDK's milestones.

**V0. Groundwork** (with SDK M1)
- `okur-vision` types, traits and mocks; capture traits with Windows drivers; the protected
  state in Okur's object model; the edit control in `okur-ui`.
- Done when: a hermetic test captures a fixture element's rectangle through the mock; the
  guard masks a protected field; `okur-ui`'s edit field is read by Okur (live conformance
  test, helper UI §5).

**V1. Convenient OCR with the system's engine** (after SDK M3a, which brings layers)
- Windows.Media.Ocr; W, C, S, Q and ?; JAWS's messages from old-pc; the Results Viewer with its
  source line; `ocr.language` in the registry.
- Done when: `okur ocr-eval windows-ocr` passes its committed error rate; the old-pc scenarios'
  messages match in `okur live-parity`; the owner uses it live.

**V2. Files, PDFs, Enter, Background OCR**
- F, D, Enter, B; Windows.Data.Pdf; the in-place document with clicking.
- Done when: an image-only PDF of two pages reads page by page, hermetically from a recorded
  result and live with the real engine; B's document refreshes and ends as JAWS's does.

**V3. Local models** (after SDK M6, whose driver registry it shares)
- `okur-models` with llama.cpp; the model list, download dialog and checksums; the describer
  and the local OCR engine; Picture Smart's C, W, S, F, B, Enter, Alt and Shift; the question
  dialog and the conversation in the Results Viewer.
- Done when: on the dev box the default model passes its fact lists within a time budget
  measured by `okur latency`; killing `okur-models` mid-request leaves Okur speaking.

**V4. Cloud providers**
- Sign in with ChatGPT; the OpenAI-compatible key client; consent pages; announcements;
  credentials in the system store.
- Done when: the mock-server tests pass; one live description through the owner's own ChatGPT
  plan, with the owner's permission.

**V5. macOS** (with the macOS port)
- Apple Vision OCR, PDFKit, AVFoundation camera, Metal in `okur-models`, the Keychain.
- Done when: V1 to V4's live checks pass on a Mac.

**R1. Okur Remote, Windows to Windows** (after SDK M4, whose speech-history and event channel it
resembles)
- The shared transport (A13); `okur-remote`; `okur relay`; speech and key forwarding; consent;
  Insert+Alt+T and Insert+Alt+Tab; session codes and pairing.
- Done when: the hermetic protocol and two-engine tests pass; live, the owner controls old-pc's
  Okur from the dev box through a relay, and `okur latency` reports key to speech across it.

**R2. Remote, complete**
- Braille (after SDK M6), clipboard, Ctrl+Alt+Delete, the curtain, view-only sessions, macOS
  on either side.
- Done when: a Mac controls a Windows machine and the reverse, live.

**R3. Remote desktop sessions** (A17)
- A virtual channel for RDP, then Citrix, carrying R1's messages.
- Done when: Okur inside an RDP session speaks through the client's Okur, live.

**How it fits the SDK.** V0 and V1 need only M1 and M3a. V3 adds the recognizer driver kind to
M6's registries; V4's providers stay in Okur until M8, then others come as extensions. R1
reuses M4's local channel design for its own link. None of it waits for the extension runtime
(M7).

## 14. Risks

- **Model quality.** NVDA withdrew its built-in captioner over invented content (research §7.2).
  Mitigation: tested models only, fact-list scores in the model list, the source always said,
  a description never presented as certain.
- **Model size and speed on a blind user's laptop.** Mitigation: the system's OCR as the
  default for text; small models; GPU through Vulkan and Metal; unload when idle; the time
  budget in V3's proof.
- **Cloud terms change.** Sign in with ChatGPT is weeks old (research §10). Mitigation: one
  client, a provider registry, an API-key fallback, and the consent page dated and revised.
- **Training and retention are unknown for plan usage.** Mitigation: the consent page says so;
  local models first.
- **A custom handshake in a program that forwards everything typed.** Mitigation: a PAKE and a
  named Noise pattern, an outside review before any public relay (A11), fuzzing.
- **macOS asks again for screen recording** (reported monthly in 2024; research §13).
  Mitigation: capture only on command, and say what the system prompt is when it appears.
- **JAWS's messages are unknown until recorded.** Mitigation: old-pc scenarios before each V and
  R milestone writes its words.
- **Scope.** These are large features. Mitigation: V1 is small and useful alone; each later
  milestone stands on its own.

## 15. Open decisions in detail

**A1. OCR with nothing installed.** The system's engines cost nothing, need no download, and
work offline; the dev box's Windows has the Turkish OCR pack (research §9). Default:
Windows.Media.Ocr and Apple Vision.

**A2. Keys and messages.** JAWS 2025's key map gives every layer key and command name (research
§2). Default: exactly those; Okur's additions (L, E in the OCR layer) on letters free in JAWS's
layer, checked against each application key map Okur adopts.

**A3. Results.** JAWS puts OCR and Picture Smart results in its Results Viewer, which Okur's
Virtual Viewer window already is. Default: the Results Viewer; in-place OCR in V2.

**A4. Bundled or downloaded models.** Good vision models are gigabytes. Default: downloaded on
request from a tested list, the size said first.

**A5. The first local models.** Qwen3.5 is Apache-2.0, strong on screens and text in pictures,
and small enough for a laptop; Gemma 4's adjustable image budget may suit weak GPUs better;
PaddleOCR-VL is the only small OCR model with Turkish both listed and measured (research §9.4).
None of the general models names Turkish. Default: Qwen3.5 4B and 2B, compared with Gemma 4
E2B and E4B in V3's evaluation before the list is fixed; PaddleOCR-VL 1.5 for local OCR.

**A6. The runtime.** llama.cpp's libraries run the widest set of GGUF vision models, with GPU
backends on every platform and a C interface (research §9). ggmlc today compiles PyTorch and JAX
models to GGUF and C++, runs on the CPU and CUDA, and lists vision-language models, Metal,
Vulkan and Rust bindings as planned (research §9). Default: llama.cpp in `okur-models` now;
ggmlc for small compiled models when those land.

**A7. Cloud providers in Okur.** Default: Sign in with ChatGPT and one OpenAI-compatible key
client; others as extensions.

**A8. Each cloud request.** Announcing is the minimum. Asking Yes or No each time is safer and
slower. Default: announce; asking is a setting, off.

**A9. The source.** Default: always the first line; not an output type.

**A10. The relay's software.** Default: Okur's own, the same code as OpenDesk's rendezvous after
the changes in §9.4, self-hostable.

**A11. A public relay run by the project.** It makes help work without setup, but someone must
run it, pay for it, and answer for its uptime and logs. Default: not before an outside review
of the protocol; until then, local networks and self-hosted relays.

**A12. Pairing and consent.** Default: PAKE codes; stored keys for one's own machines; consent
every session unless the user marks a paired machine otherwise.

**A13. One transport with OpenDesk.** Both need two peers, a code, and a relay that cannot read.
Default: extract OpenDesk's protocol and rendezvous into a shared crate, move it to a PAKE and a
Noise pattern, and use it in both. You own both projects, so this is your call on OpenDesk too.

**A14. What crosses.** Default: typed announcements, braille, keys; no audio, no pixels.

**A15. Keyboard layout.** Tandem uses the controller's layout on both sides; scan codes give the
controlled machine's. A Turkish Q user helping someone on a US layout will find letters moved.
Default: scan codes, with a per-session option to send characters in the controller's layout.

**A16. NVDA Remote and Tandem.** Tandem is proprietary; NVDA Remote's relay can read sessions
today and its end-to-end design is not final (research §7.4). Default: no; reconsider when
NVDA's version 3 protocol ships.

**A17. RDP and Citrix.** Default: after R2, a virtual channel carrying Okur Remote's messages.

**A18. A help assistant.** Default: later, an extension over Okur's documentation, local model
first.

**A19. The SDK's non-goal 7** says no language model inside Okur, so that agents work from
outside. That stays true for agents. Default: amend it to allow models in the separate
`okur-models` process, used only by the user's command, never in Okur's own process.

**A20. Timing.** Default: V0 with SDK M1; V1 after M3a; V2 next; V3 after M6; V4 after V3; R1
after M4; V5 and macOS remote with the macOS port.

## Glossary

- **OCR**: reading text from pixels.
- **Results Viewer**: JAWS's window for results; in Okur, the Virtual Viewer's window.
- **Layer**: a key such as Insert+Space, O, after which the next key picks a command.
- **GGUF**: the file format llama.cpp loads models from. **Projector (`mmproj`)**: the second file
  of a vision model, which turns a picture into tokens.
- **Image tokens**: how much of the model's work one picture takes; fewer is faster.
- **Process host**: a separate program that runs risky code, so a crash cannot silence Okur.
- **Sign in with ChatGPT (SIWC)**: OpenAI's sign-in that lets an app's requests use the user's
  ChatGPT plan.
- **Relay**: a server that passes encrypted frames between two machines that cannot reach each
  other directly.
- **PAKE**: a password-authenticated key exchange; turns a short code into a strong key, giving an
  attacker one guess per attempt.
- **Noise**: a published framework of handshake patterns for encrypted channels.
- **Pairing**: two machines exchanging and storing each other's public keys once.
- **Curtain**: the controlled screen blacked out while its screen reader keeps working.
