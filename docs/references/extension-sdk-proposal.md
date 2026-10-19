# Extension SDK: proposal

**Status:** proposal for the owner to approve or amend, 2026-10-18. Nothing here is built.

**Builds on:**
- [`extension-sdk-vision.md`](extension-sdk-vision.md): the goal and the principles;
- [`extension-sdk-research.md`](extension-sdk-research.md), cited below as "research §n": the
  study of JAWS scripting, NVDA add-ons and Okur's code, with every source;
- [`docs/plans/2026-10-15-structure-for-macos-and-extensions.md`](../plans/2026-10-15-structure-for-macos-and-extensions.md)
  and [`docs/plans/2026-10-17-helper-dialogs.md`](../plans/2026-10-17-helper-dialogs.md):
  the plan steps this proposal absorbs (§12).

**Code references** are lines of commit `9ccf0ba`. Another session is editing the working tree
now: `settings.rs` (adding `auto_forms_mode` and `access_keys`), `engine.rs`,
`engine/focus.rs`, `engine/page.rs`, `native_focus.rs`, `event.rs`, `buffer.rs`, the Windows
event listener and `okur/src/main.rs`. Line numbers in those files will move.

**Clean room.** Unchanged from AGENTS.md §1.4. Everything about JAWS here comes from its
documentation, its behavior, and the reference machine's user settings files as Okur's
capture tooling records them.

## Contents

1. The decisions asked of you
2. Goals and non-goals
3. How the SDK serves the vision
4. Architecture
5. Extension kinds: what an extension can and cannot do
6. Speech output: announcements, messages, voices, verbosity
7. Keys and the hook thread
8. Settings
9. Runtime, permissions, isolation and failure
10. Packaging, versions, distribution and trust
11. Testing and proof
12. Roadmap and milestones
13. Risks
14. Open decisions in detail
15. Glossary

---

## 1. The decisions asked of you

Each has a recommended default. §14 gives the reasons.

| # | Decision | Recommended default |
|---|---|---|
| D1 | The code runtime for outside extensions | WebAssembly components on Wasmtime, confirmed by a measured spike; QuickJS as the fallback |
| D2 | Okur's own nine application modules | Stay Rust, in the repository, on the same contract |
| D3 | Okur's speech language | English by default, as JAWS on the reference machine; other languages chosen in settings |
| D4 | The first translation | Turkish, reviewed by you |
| D5 | Verbosity defaults | JAWS's defaults (Beginner); replay uses the reference machine's recorded settings |
| D6 | Keys on non-US layouts | By physical key, as JAWS binds them |
| D7 | Who may take a key | Application and site extensions may rebind keys inside their scope; global extensions may not take Okur's keys |
| D8 | The chord that turns every extension off | Chosen after checking JAWS's key map for a free chord |
| D9 | Protected (password) fields | No extension can read them; there is no permission for it |
| D10 | Network for extensions | Off; a per-host permission you approve |
| D11 | Global extensions | Allowed, with your explicit approval, and without keys of Okur's |
| D12 | Distribution | A source-required registry, built and signed by its CI; local extensions approved by hash |
| D13 | Porting third-party JAWS scripts | Not now; AGENTS.md §1.4 stays as it is |
| D14 | Python | Not in Okur's process; an out-of-process host only if there is demand |
| D15 | The read-only developer interface | Build it early (milestone M4), off by default |
| D16 | Timing | M0 to M4 now, before the macOS port; M7 onwards after Okur is stable on Windows and macOS |
| D17 | Sounds and braille | A sound output with verbosity (M5); braille driver interface in M6, first driver after macOS |
| D18 | Reading JAWS's Speech and Sounds scheme files | Your call: AGENTS.md lists `.jcf` and `.vpf` only |

## 2. Goals and non-goals

### Goals

1. **Application support without changing Okur.** Behavior for one application or one web
   site, written by Okur's maintainers, by agents, or by users.
2. **The vision's loop.** An agent reproduces a problem, compares with JAWS, writes a core fix
   or an extension, proves it by replay, and proposes it.
3. **JAWS's behavior stays the target.** Okur's English is JAWS's English, checked by parity.
   Verbosity works the way JAWS's does, with JAWS's defaults.
4. **Okur speaks other languages.** Its own words come from translated catalogs. Turkish first.
5. **More than application behavior.** Extensions add commands and keys, settings, dialogs,
   speech synthesizers and braille displays.
6. **Safety.** No extension can silence Okur, slow a key, or read what it was not allowed to.
   One chord turns them all off.
7. **One extension, every platform.** Windows, macOS and Linux run the same extension.

### Non-goals

1. A JAWS script language or a runtime for JAWS scripts (research §2.10; vision).
2. Reading or translating Freedom Scientific's scripts; porting third-party scripts until D13
   changes.
3. A stable public SDK before AGENTS.md's "no backward compatibility" notice is lifted and
   before Okur is stable on Windows and macOS. Until then the contract is internal and changes
   freely.
4. Python in Okur's process, or native plugin libraries from third parties (research §9.2).
5. Extensions as a place for workarounds of Okur's own defects (vision principle 8).
6. Raw UI Automation, IAccessible2, AX or AT-SPI objects in extensions.
7. A language model inside Okur. Agents work from outside, through the developer interface.

## 3. How the SDK serves the vision

The vision's example: "Okur does not read new messages in this chat app."

1. **Reproduce.** The user turns the developer interface on (§4.6). An agent drives the
   application with OpenDesk and records the session with Okur: keys, accessibility events and
   the speech Okur produced, as one replay file (§11.1).
2. **Compare.** On old-pc the agent records JAWS in the same application, as today
   (`skills/jaws-reference/`).
3. **Decide where the fix goes.** If Okur misreads something any application could produce
   (an ARIA pattern, a UIA event), it is a core fix, tested by parity. If the behavior belongs
   to this application (its own keys, its own message layout), it is an extension.
4. **Write the extension.** Most often data: an activation rule, two key bindings, a message,
   an announcement rule. Code only when data cannot express it (§5.3).
5. **Prove it.** The replay file plus the speech the extension should produce become the
   extension's test. `okur ext test` replays it on every platform (§11).
6. **Propose it.** A pull request to the registry carries the source, the manifest, the replay
   tests and their results (§10.4).

The same loop serves people who never write code: the agent does steps 1 to 6 from a sentence
of theirs.

## 4. Architecture

### 4.1 The parts

- **The engine** (`okur-core`): key routing, cursors, the virtual buffer, focus reading, the
  rendering of speech. Unchanged in role.
- **The contract**: one set of Rust types and traits that every extension uses, Okur's own
  modules first. It lives in a new crate, `okur-ext`, that depends on nothing platform-specific.
  `okur-core` depends on it; nothing in it depends on `okur-core`'s internals.
- **Hosts**: the places extensions run.
  - The **built-in host**: Rust modules compiled into Okur (today's `apps/`).
  - The **data host**: extensions made only of data (key bindings, rules, messages, settings).
  - The **Wasm host**: code extensions as WebAssembly components, in Wasmtime (§9).
  - The **process host**, later: a separate program for what cannot be Wasm (a Python
    ecosystem, a GPL synthesizer, a braille device driver).
- **Drivers**: speech synthesizers and braille displays, behind registries instead of enums
  (§5.6).
- **`okur-ui`**: draws every dialog, Okur's and extensions', from data (research §7.6).
- **The developer interface**: a local channel into a running Okur, off by default (§4.6).

### 4.2 Threads

- **The hook thread** runs the keyboard hook. It decides whether a key is Okur's. No extension
  code ever runs there (§7).
- **The engine thread** updates state and renders speech. Built-in commands run there.
- **The extension worker** runs Wasm and data extensions, with deadlines (§9.4). Results come
  back to the engine as events.
- **`okur-ui`** and any process host are separate processes.

Today the engine runs inside the hook callback under one mutex
(`crates/okur/src/main.rs:853-902`). §7.3 says how this changes, in two stages.

### 4.3 The contract, first used by Okur's own modules

The contract is proven on the nine modules before anyone else uses it (vision principle 1).
Each of today's `AppModule` hooks (`crates/okur-core/src/apps/mod.rs:57-131`) has a place in
it:

| Today's hook | In the contract |
|---|---|
| `owns`, `owns_page` | the extension's activation: applications, sites, object kinds (§5.2) |
| `focus` returning `Pass`, `Silent` or `Speak` | a focus interceptor returning a decision (§5.4) |
| `names_window`, `window_name` | an announcement filter on the window heading (§5.4) |
| `reads_key`, `types_into`, `edit_key` | an edit behavior the module declares for its fields |
| `virtualizes_text`, `viewer_lines`, `line_after_viewer` | a Virtual Viewer provider |
| `font_elements`, `font_from_elements` | an asynchronous page query and a formatting provider |
| the application modes in `AppMode` (`engine/state.rs:182-189`) | state owned by the module, with a "keys go to the application" answer (§7.2) |

Nothing a user hears changes in this move; parity proves it (§12, M2).

### 4.4 What crosses the contract

- **Objects as snapshots**, never live platform objects: the focus, nodes of the page, window
  facts. Each carries a stable id for later queries.
- **Queries are asynchronous.** An extension asks; the answer arrives as an event. This is how
  `ApplicationActions` works today (`crates/okur-core/src/actions.rs:11-90`), so no extension
  ever blocks the engine on a call into another process.
- **Platform identifiers** (window classes, automation ids) are readable in a `platform`
  section, marked non-portable. Okur's own Windows modules use them, as JAWS's application
  scripts know window classes (research §2.2).

### 4.5 Order of precedence

For any event or key, Okur asks in this order, the most specific first:
1. the user's own overrides (key maps, labels);
2. object-kind rules for the object, then the extension for the site in front, then the
   extension for the application in front;
3. global extensions;
4. Okur's default behavior.

A handler can do the default first and add to it, as a JAWS application script calls the
default script's version and an NVDA handler calls `nextHandler` (research §2.2, §3.4). Two
extensions for one application keep a fixed order, shown in the extensions dialog.

### 4.6 The developer interface

- A local channel to the running Okur: a named pipe on Windows, a Unix socket on macOS and
  Linux, readable only by the user's account.
- Off by default. Turning it on is a setting with a lifetime (until Okur exits by default),
  and Okur says so when it starts.
- An MCP front end, `okur mcp`, a stdio server that connects to that channel, so any agent can
  use it.
- First version read-only: state (mode, focus, application, site), speech history, events,
  the virtual buffer, the Virtual Viewer's text, recording a replay file.
- Later: loading and reloading a development extension, running a command. Each needs its own
  switch.
- Protected fields are never sent; their text is replaced by a mark.

## 5. Extension kinds: what an extension can and cannot do

### 5.1 The kinds

1. **Application extension.** Active while its application is in front.
2. **Site extension.** Active while a page of its site is in front, in any browser.
3. **Object-kind extension.** Active for objects of one kind (a role and properties) inside its
   application or site, as NVDA's overlay classes (research §3.3).
4. **Global extension.** Always active. Needs your approval (D11) and cannot take Okur's keys.
5. **Speech synthesizer driver.**
6. **Braille display driver**, and braille translation tables.
7. **Language pack.** Message catalogs and symbol names for one language (§6.3).
8. **Settings presets.** Key maps, verbosity profiles, labels, as data.

Dialogs are not a kind: any extension may show dialogs through `okur-ui`.

### 5.2 Activation

An extension declares what it is for, in its manifest (TOML, §10.2). Two examples: a site,
and an application with made-up names.

```
[[activation.sites]]
host = "web.whatsapp.com"
path_prefix = "/"

[[activation.applications]]
windows = "examplechat.exe"          # executable name
macos = "com.example.chat"           # bundle identifier
linux = "examplechat"                # desktop file or process name

[[activation.objects]]               # optional, inside the site or application
role = "list_item"
inside_role = "list"
```

Okur matches by identity only, never by window titles or other display text (research §7.1
and §10.1).

### 5.3 Data first, code when needed

Many fixes are data, as most of JAWS's user customizations are (research §2.6):
- key bindings to existing commands;
- custom labels for unnamed controls;
- announcement rules: "for a button named like X, drop the role" or "for a list item in this
  list, say the second line first";
- new quick-navigation kinds: "messages are list items in the list named Y";
- settings presets and messages.

Code is for what data cannot express: counting, remembering, reading several objects and
combining them. Code extensions use the same declarations plus handlers.

### 5.4 Interfaces, sketched

These are sketches of the contract's types, in a Rust-like notation, not an implementation.

**Events an extension can receive:**

```
enum Event {
    ApplicationActivated, ApplicationDeactivated,
    SiteActivated, SiteDeactivated,
    Focus(FocusSnapshot),
    ObjectChanged { object: NodeId, what: Name | Value | State | Description },
    Selection(FocusSnapshot),
    TextChanged { caret, selection, edit: Option<EditKind> },
    LiveRegion { region: NodeId, text, politeness },
    Notification { text, kind },              // UIA notifications, macOS announcements
    PageLoaded, PageChanged, PagePartReloaded { part: NodeId },
    WindowOpened, WindowClosed,
    MenuStarted, MenuEnded,
    QueryAnswer { request: RequestId, nodes },
    DialogEvent { dialog: DialogId, event: FromUi },
    SettingChanged { id },
    Idle,
}
```

**Two kinds of handler:**
- An **observer** gets an event after Okur has spoken. It cannot change speech. It runs on the
  extension worker with a 50 ms deadline.
- An **interceptor** gets the announcement Okur is about to speak for that event, and returns
  a decision. It has a 2 ms deadline (§9.4).

```
enum Decision {
    Default,                       // say what Okur would say
    Amend(Vec<Edit>),              // change parts
    Replace(Announcement),         // say this instead
    Silent,                        // say nothing
}

enum Edit {
    Drop(PartKind),
    Replace { kind: PartKind, with: Part },
    InsertBefore { kind: PartKind, part: Part },
    InsertAfter { kind: PartKind, part: Part },
    AddLine(Line),
}
```

**Commands and bindings:**

```
struct CommandSpec {
    id: CommandId,                 // "whatsapp.read-message-1"
    label: MessageId,              // for key help and the key map dialog
    description: MessageId,
    category: MessageId,
    cursors: [Virtual | Pc | Forms | Any],
}

struct Binding {
    key: PhysicalKey,              // §7.4
    modifiers: Modifiers,          // JAWS key, Ctrl, Alt, Shift, Windows
    layout: Desktop | Laptop | Any,
    scope: Application | Site | Global,
    command: CommandId,
    pressed_twice: Option<CommandId>,
}
```

A command handler gets a context (the focus, the page, settings, the extension's state) and
may speak, query and act within its permissions.

**Actions an extension may request**, each a permission (§9.3): move the virtual cursor, focus
an object, activate it, send keys to the application, speak, play a sound, show a dialog,
copy text.

**A query:**

```
enum Query {
    Node(NodeId),
    Children(NodeId), Parent(NodeId),
    Find { within: NodeId, role, name_matches },
    PageElementsById(Vec<String>),     // today's read_page_elements
    FocusedText,                       // text, caret, selection of the focused field
}
```

### 5.5 Dialogs

An extension describes a dialog with `okur-ui-model`'s `DialogSpec` (research §7.6) and
implements a controller:

```
trait DialogController {
    fn spec(&self) -> DialogSpec;
    fn handle(&mut self, event: FromUi) -> DialogOutcome;
}
enum DialogOutcome { Nothing, Send(Vec<ToUi>), Close(DialogResult) }
```

The engine keeps a table of open dialogs by id and routes events to their owners, replacing
today's two dialog fields (`engine.rs:117-120`). The control set grows by edit, tree and tabs
controls, which the helper UI decision already lists. Okur reads an extension's dialog like
any other window, through the same code that reads its own dialogs.

### 5.6 Drivers

**Speech synthesizer drivers** replace the closed enum (`crates/okur-platform/src/tts/mod.rs:83-89`):

```
trait SpeechDriver {
    fn id(&self) -> &str;
    fn available(&self) -> bool;
    fn voices(&self) -> Vec<Voice>;               // name, language
    fn capabilities(&self) -> Capabilities;       // pitch, rate, volume, index marks, languages
    fn speak(&mut self, sequence: SpeechSequence);
    fn cancel(&mut self);
    fn settings(&self) -> Vec<SettingSpec>;       // into the settings registry
}
// SpeechSequence: text segments and commands: voice context, pitch, rate,
// language, character mode, pause, index mark. Index marks come back as events.
```

**Braille display drivers**: cell count, write cells, routing keys and input gestures, device
detection. Translation through liblouis tables (LGPL, research §9.4).

Drivers that talk to devices, or carry a GPL license, run in the process host. A synthesizer
that only computes audio may be a Wasm component that returns sound samples, which Okur plays.

### 5.7 What an extension cannot do

- Run on the hook thread, or see keys it did not bind.
- See typed characters outside its own scope, or at all without the "typing" permission.
- Read protected fields. No permission grants it.
- Reach raw platform accessibility objects.
- Read or write files outside its own storage folder.
- Use the network without a per-host permission.
- Change another extension, or Okur's settings other than its own. It can offer a settings
  preset the user applies.
- Run on secure screens (sign-in, UAC): no outside extension loads there.
- Keep running after it is disabled, or when the kill switch is used.

## 6. Speech output: announcements, messages, voices, verbosity

### 6.1 Announcements as typed data

Today a part is `Text`, `Role` or `State`, and roles and states carry English words
(`crates/okur-core/src/announcement.rs:13-31`; research §7.4). The target:

```
struct Announcement {
    context: VoiceContext,           // §6.4
    lines: Vec<Line>,
}
struct Line { parts: Vec<Part> }
struct Part {
    kind: PartKind,
    content: Content,
    language: Option<LanguageTag>,   // from the page or the application
    object: Option<NodeId>,
}
enum PartKind {
    Name, Value, Role, State, Position, Level, Description,
    Hint, Shortcut, AccessKey, Container, Message, Text,
}
enum Content {
    Text(String),                    // the application's words, as given
    Role(Role, RoleContext),         // rendered from the catalog
    State(State),
    Position { index: u32, total: u32 },
    Message { id: MessageId, args: Vec<Arg> },
}
```

- The engine builds announcements. Words come only at the end, from the catalog.
- `RoleContext` exists because JAWS's own English differs by place: "Button" in focus speech,
  "button" in Say Window (`native_focus.rs:486-569`; research §7.4).
- The virtual buffer's lines become the same parts (the structured page line of structure
  step 4), so filters and verbosity work there too.

**The pipeline**, for every utterance:
1. the engine builds the announcement;
2. interceptors amend it (§5.4);
3. verbosity drops or changes parts by kind and output type (§6.5);
4. the catalog renders words in Okur's language (§6.2);
5. the voice context and role and state marks become speech commands (§6.4);
6. braille renders the same parts with its own profile, when braille exists.

Interceptors run before verbosity, so the user's verbosity governs what extensions add too.

### 6.2 The message catalog

**Format: Project Fluent** (`.ftl` files, the `fluent-rs` crates; research §9.5). It has a
mature Rust implementation, plurals and selectors by CLDR rules, and attributes. MessageFormat
2 is the alternative once its Rust implementations mature.

**Where:** `crates/okur-core/locales/<language>/` (for example `en-US/speech.ftl`,
`en-US/dialogs.ftl`), compiled into the binary. Extensions carry their own `locales/` folder.

**Ids name the situation, not the words.** Fluent allows letters, digits, `-` and `_`:

```
quicknav-none-links = There are no links on this page.
quicknav-none-frames = There are no frames on this page
table-not-in-table = Not in a table.
hint-activate-enter = To activate press Enter.
role-button = Button
role-button-say-window = button
position = { $index } of { $total }
pass-key-through = Enter key to pass through
```

- The frames message keeps JAWS's missing period (`cursor.rs:970`). English is JAWS's English,
  exactly, including its quirks.
- Each message is whole. Okur never builds "There are no " + element + " on this page."
- **Long and short forms** use a Fluent attribute, `.short`, on the messages where JAWS has
  both (research §2.4). Which of JAWS's messages have a short form is learned by switching
  the message length setting on old-pc.
- **Plurals** use selectors. English can keep JAWS's exact wording even where it is not
  grammatical; Turkish gets its own rules (CLDR: `one` and `other`; research §9.5).
- Fluent's bidirectional isolation marks are turned off: they would reach the synthesizer
  (research §9.5).
- Translator comments (`#` lines) go above each message whose meaning is not obvious, as NVDA
  requires (research §3.7).

**How English stays JAWS's English:**
1. The move from literals to the catalog changes no speech: `parity-snapshot.sh` before and
   after, `diff -r` empty, every test unchanged in what it asserts.
2. A test fails on any user-facing literal outside the catalog (an allow-list for identifiers
   such as class names and attribute keys).
3. A test fails on catalog entries nothing uses.
4. Parity keeps comparing English output with JAWS's English transcripts, as today.

**How Turkish is added:**
1. `locales/tr/` with every id, translated by an agent and reviewed by you (D4).
2. A completeness test: every English id exists in Turkish, or is listed as falling back.
3. A pseudo-locale (every message wrapped in marks) runs the hermetic tests and finds words
   still outside the catalog.
4. Turkish has no JAWS to compare with (no Turkish JAWS interface was found, research §2.4),
   so its proof is review and hermetic tests, not parity.

**Choosing the language:** a setting, `speech.language`, English by default (D3). Okur's words
use it; the application's words are spoken as the application gives them. Dates and times keep
coming from the system's locale, as Say System Time does now
(`crates/okur-platform/src/clock.rs:44-68`).

### 6.3 Symbols and punctuation

- Symbol names move from `pronounce.rs` into per-language data (a language pack, §5.1).
- Punctuation levels None, Some, Most, All, as JAWS's (research §2.5).
- English names stay exactly what JAWS sends its synthesizer (`pronounce.rs`, checked by the
  spoken layer of parity).

### 6.4 Voice contexts

The reference machine's voice profiles show that JAWS has one voice setting per context:
`Message`, `PCCursor`, `JAWSCursor`, `Keyboard`, `MenuAndDialog`, inheriting from `Global`,
each with voice, rate, pitch, punctuation, spelling rate and capital pitch (research §2.5,
recorded in evidence provenance).

Okur adopts the same model:
- every announcement has a context;
- the voice settings are per context, inheriting from a global one;
- punctuation is per context, as JAWS stores it;
- role and state parts get JAWS's separate voice treatment through speech commands, replacing
  today's `\u{1d}` … `\u{1e}` marks inside strings.

Which context JAWS uses for the virtual cursor, forms mode and its own messages is not known.
It can be measured without reading anything forbidden: give each context a different pitch in
a test voice profile on old-pc, and read the pitch of each fragment JAWS sends the
synthesizer, which the speech recorder already logs
(`crates/okur-speech-recorder/src/engine.rs:140-141`).

### 6.5 Verbosity, modeled on JAWS

**What JAWS has** (research §2.4, §2.5):
- three user verbosity levels, Beginner (the default), Intermediate and Advanced;
- per level, a list of message types that are spoken or not, and a long or short message
  length;
- output types: every message JAWS speaks belongs to one;
- Speech and Sounds Manager schemes: per control type, state and attribute, speech, a sound,
  another voice, or replacement text;
- a separate web verbosity level per application;
- punctuation levels per voice context.

**What the reference machine's files show** (research §2.5):
- `DEFAULT.JCF` `[OutputModes] ACCESS_KEY=0|0|0|Access Key`: one output type with three
  values, presumably one per level, all off, and its display name;
- `[FormsMode] AutoFormsMode=0`, `[Options] TypingEcho=0`, `KeyboardType=Laptop`,
  `LanguageDetection=0`;
- `Chrome.JCF` `[VirtualCursorVerbosity] VirtualCursorVerbosityLevel=2` and
  `[options] AllowWebAppReservedKeystrokes=0`;
- voice profiles per context (§6.4).

**Okur's model:**

```
struct Verbosity {
    level: Beginner | Intermediate | Advanced,       // default Beginner
    output_modes: Map<OutputType, [OutputMode; 3]>,  // one per level
    message_length: [Long | Short; 3],               // Long at Beginner, Short above
    scheme: SpeechScheme,                            // per role, state, attribute
    web_level: per application,                      // VirtualCursorVerbosityLevel
}
enum OutputMode { Off, On }        // more values only if JAWS's files show them
```

- **Output types are Okur's ids** (`access_key`, `position`, `hint`, …), each mapped to the JCF
  key where one is known (`access_key` ↔ `ACCESS_KEY`).
- **The list of types is learned by observation**, never from JAWS's header files (research
  §2.4): read the item names in JAWS's verbosity customization dialog, then switch each item
  and compare `DEFAULT.JCF` before and after. Both steps are allowed: JAWS's UI and user
  settings.
- **Every part kind maps to an output type** (`PartKind::AccessKey` to `access_key`,
  `PartKind::Hint` to the help type…), and every message declares its type. Extensions'
  messages declare types too, so your verbosity governs them.
- **Defaults are JAWS's** (D5): Beginner, every type on unless JAWS's default says otherwise.
  JAWS's defaults are learned the same way, from a fresh user profile's dialogs.
- **Sounds**: the scheme's sound choice needs a sound output, which Okur lacks (research §7.7).
  It comes with M5; until then a scheme entry can only be speech or silence.

**How old-pc's settings map:**

| JAWS setting on old-pc | Okur setting | State |
|---|---|---|
| `KeyboardType=Laptop` | `keyboard.layout = laptop` | exists (`settings.rs:12-83`) |
| `TypingEcho=0` | `typing_echo = none` | exists (`settings.rs:99-122`) |
| `AutoFormsMode=0` | `forms.auto_forms_mode = false` | being added in the working tree as `auto_forms_mode` |
| `ACCESS_KEY=0\|0\|0` | `verbosity.output_modes.access_key = [Off, Off, Off]` | exists: `Settings::verbosity`, filled by parity replay ([`messages-and-verbosity.md`](messages-and-verbosity.md)) |
| `LanguageDetection=0` | `speech.language_switching = off` | new, M5 |
| Chrome `VirtualCursorVerbosityLevel=2` | `web.verbosity_level`, application scope | new; the meaning of 2 to confirm on old-pc |
| Chrome `AllowWebAppReservedKeystrokes=0` | an application-scope web setting | new; meaning to confirm from JAWS's help |
| voice profile "SAPI 5X 64", punctuation Most | `voice.contexts.*`, `punctuation = most` | new, M5 |

The flat `auto_forms_mode` and `access_keys` the other session is adding are the right first
step: they make replay match old-pc now. M1 moves them into the registry under these ids, and
the JCF import fills them (§8.4).

## 7. Keys and the hook thread

### 7.1 The rule

The keyboard hook decides whether a key is Okur's from a table and a small state, at once. No
extension code runs on the hook thread, and no cross-process call happens there (vision
principle 2). Microsoft documents that a slow low-level hook is silently removed, and advises
handing work to a worker thread (research §9.1). JAWS decides by key maps before any script
runs (research §2.3).

### 7.2 One key table

The key table is built from:
1. JAWS's key map as Okur has it (`command.rs:404-578`), moved into a data file in JAWS's own
   notation (`Command::from_name` already reads it, `command.rs:143-294`);
2. extension bindings, by scope;
3. the user's overrides.

The table answers, for a key, the modifiers, the layout, the current layer and the current
scope (mode, application, site): consume and run this command, or pass the key on. Modes that
belong to applications (File Explorer's, the desktop's) answer through their module's state,
not through new engine modes.

**Conflicts:**
- An application or site extension may rebind a key inside its scope, as JAWS's application
  key maps take precedence over the default key map (research §2.3; D7).
- A global extension may not take a key Okur binds, as Orca's extensions may not
  (research §4.1). Its command is registered but unbound, and the extensions dialog says so.
- Between two extensions for the same scope, the first loaded keeps the key; the other is
  reported.
- The user's overrides beat everything.

**Key help** (JAWS's Insert+H and Insert+W) lists the commands of the application and site in
front, extensions' included, from the table.

### 7.3 Decide first, run after: two stages

Today, in the virtual cursor, Okur runs the command and consumes the key if it produced speech
or is a navigation key (`engine.rs:455-477`). Deciding needs running. That has to change.

**Stage 1 (M3a): split decision from execution inside the engine.**
- Each command gets a "consumes when" rule that reads state but changes nothing and speaks
  nothing (for Enter: the line under the cursor is a link, a field or a control).
- `handle_key` asks the table and the rule, then executes.
- Extension commands are never executed inline: the engine posts them to the extension worker
  and returns. Their speech arrives as an event.
- Proof: a unit test that the decision never calls `execute`; parity unchanged; `okur latency`
  shows the same `held_us`.

**Stage 2 (M3b, if measurements ask for it): the engine leaves the hook thread.**
- The engine thread publishes a routing snapshot after every key and event: mode, forms mode,
  menus, viewer, layer, scope, keyboard settings.
- The hook decides from the latest snapshot. If an earlier key is still being processed and
  might change the mode (Enter turning forms mode on), the hook waits for that key, bounded as
  today (150 ms), never for an extension.
- Done when `okur latency` shows hook time independent of engine work, measured live.

Stage 1 is required for extensions. Stage 2 follows Microsoft's advice and is done when
`okur latency` shows keys waiting on engine work.

### 7.4 Physical keys, and the Turkish Q layout

- **Bindings name physical keys**, as JAWS binds logical key names that correspond to physical
  keys. On Turkish Q the `;` key types "ş", and JAWS ran its onMouseOver command for it
  ([`jaws-quick-navigation-keys.md`](jaws-quick-navigation-keys.md), evidence 110/s8).
- Okur already maps Windows virtual keys, not typed characters (`VK_OEM_1` becomes `Semicolon`,
  `crates/okur-platform/src/hook/windows.rs:415`). The key file names keys as JAWS does
  (`SemiColon`, `Apostrophe`, `Comma`).
- On macOS, key codes are positions, which fits the same model.
- Key help says the character the user's layout produces for each key, in the user's language.
- Open: CapsLock+Comma and CapsLock+Period typed into the document on the dev box
  ([`verification-loop.md`](verification-loop.md) §9). Record JAWS on old-pc before deciding
  those two.

### 7.5 Layers

JAWS's layered keys (Insert+Space, then a letter) become a step in key routing (helper dialogs
plan step 10). Extensions may add layer keys inside their scope, with the same conflict rules.

## 8. Settings

### 8.1 One registry

Every setting, Okur's and extensions', is declared once:

```
struct SettingSpec {
    id: SettingId,                      // "verbosity.level", "whatsapp.read-incoming"
    owner: Okur | Extension(ExtensionId),
    label: MessageId,
    description: MessageId,
    kind: Bool | Choice(options) | Range(min, max, step) | Text,
    default: Value,                     // JAWS's default where JAWS has the setting
    scopes: [Global, Application, Site],
    lifetimes: [Saved, UntilExit, UntilFocusMoves],
    group: GroupPath,                   // place in Settings Center's tree
    quick: bool,                        // also in Quick Settings
    jcf: Option<JcfKey>,                // section and key for the JAWS import
}
```

This is the registry the helper UI decision describes
([`helper-ui-architecture.md`](helper-ui-architecture.md) §4, "Settings").

### 8.2 Scopes and layers

A value is looked up in this order: until the focus moves, until Okur exits, the site, the
application, global, then the default. Quick Settings changes the application or site in
front, as JAWS's does; Settings Center chooses the scope with an Application combo box
(research §2.5; [`jaws-dialogs-and-visual-features.md`](jaws-dialogs-and-visual-features.md)).

Storage: one TOML file per scope under Okur's settings folder (`settings.toml`,
`applications/<id>.toml`, `sites/<host>.toml`), built in the reading-settings round
(`crates/okur-core/src/settings_layers.rs`, `crates/okur-platform/src/settings_file.rs`);
extensions' values will go under their own id. The format can change freely while AGENTS.md's
notice stands.

### 8.3 How okur-ui shows them

Quick Settings and Settings Center are dialog descriptions generated from the registry: a
search field, a tree of groups, each setting as a standard control, a description, the
lifetime choice. Extensions' settings appear under the extension's name. No extension draws its
own settings window. This needs the tree and edit controls (§5.5).

### 8.4 The JAWS import

One mapping table reads JAWS's user settings files (`.jcf`, `.vpf`) into registry values, by
each setting's `jcf` key. Replay uses it for every evidence file's recorded settings, replacing
today's two hand-picked options (`crates/okur-corpus/src/parity.rs:568-588`). A user moving
from JAWS could use it too.

## 9. Runtime, permissions, isolation and failure

### 9.1 The choice

**Recommended (D1): WebAssembly components on Wasmtime for code extensions**, after a spike
confirms the numbers (§12, M7). Data extensions need no runtime. Okur's own modules stay Rust.

Why (research §9.2, §9.3):
- a sandbox: an extension reaches only the functions Okur gives it, so permissions are
  enforced, not just declared;
- deadlines: epoch interruption stops a handler at its deadline, at about 10% speed cost;
- memory limits per extension;
- one artifact for every platform;
- a language-neutral interface (WIT) that can mirror the Rust contract;
- Rust guests today; JavaScript guests through an engine compiled to Wasm; Python later if
  wanted;
- a trap ends the extension, not Okur.

### 9.2 Rejected, and why

| Alternative | Why not (research section) |
|---|---|
| A JAWS script language | Adds nothing a modern language lacks; would need much of JAWS's runtime (vision; §2.10) |
| Native plugin libraries | A crash or abort ends Okur; no budget, no permission enforcement (§9.2) |
| Python in Okur's process | Cannot be sandboxed; NVDA's experience (§3.10, §3.12, §3.13) |
| RustPython | Its authors say it is not production-ready (§9.2) |
| V8 | Running out of heap ends the whole process (§9.2) |
| Boa | No time limit for a running script (§9.2) |
| QuickJS in Okur's process | Kept as the fallback if the Wasm spike fails: small and fast, with interrupts and memory limits, but its C code shares Okur's process |
| Lua (Luau) or Rhai | Workable sandboxes, but another language for authors and agents, and weaker isolation than Wasm |
| A process per extension | Strong isolation, but an IPC round trip on every focus filter; kept for drivers and Python (§9.4) |

### 9.3 Permissions

An extension declares permissions in its manifest. You approve them at install. In Wasm each
permission is a set of imported functions: without the permission, the function is not there.

| Permission | Gives |
|---|---|
| (always) | its own scope's focus, page and events; speaking; its own settings and storage |
| `read.text-fields` | the text, caret and selection of fields in its scope |
| `read.typing` | typed characters in its scope |
| `read.clipboard` / `write.clipboard` | the clipboard |
| `act.focus`, `act.activate` | move the focus, press buttons and links |
| `act.keys` | send keys to the application |
| `ui.dialogs` | show dialogs |
| `network:<host>` | that host only |
| `scope.global` | global activation (D11) |

There is no permission for protected fields (D9), for other extensions' data, or for files
outside the extension's folder.

### 9.4 Time budgets

| Where | Budget | When it is missed |
|---|---|---|
| The key decision | no extension code at all | (cannot happen) |
| A focus or announcement interceptor | 2 ms per call | Okur speaks its default; the miss is logged |
| An observer | 50 ms | the handler is stopped; logged |
| A command | 250 ms to start speaking; stopped at 2 s | stopped; logged |
| Loading an extension | 50 ms, on first activation, not at Okur's start | the extension stays off this session |

These are starting values. The spike (M7) measures real handlers with `okur latency` and sets
the final numbers with you.

### 9.5 Failure

- **A trap or a stopped handler**: Okur continues with its default behavior.
- **Three failures within a minute**: the extension is turned off for the session. Okur says
  once: "Extension WhatsApp Web stopped." The extensions dialog says why.
- **Memory over its limit**: the same.
- **The kill switch** (D8): one chord turns every extension off at once; pressed again, back on.
- **Safe start**: `okur run --no-extensions`, and a way to restart with extensions off from
  Okur's own dialogs, as NVDA's "Restart with add-ons disabled" (research §3.1).
- **A process host that exits**: restarted, with a growing delay, three times; then off.
- **Okur's own modules** are not subject to this: they are Okur, tested by parity.

## 10. Packaging, versions, distribution and trust

### 10.1 The package

An extension is a folder, and a zip of it for distribution:
- `manifest.toml`;
- `extension.wasm`, only for code extensions;
- `data/`: key bindings, rules, labels, presets;
- `locales/<language>/*.ftl`: its messages;
- `tests/`: replay files and the speech each should produce (§11);
- `README.md` and `LICENSE`.

### 10.2 The manifest

```
id = "org.example.whatsapp-web"
name = "WhatsApp Web"
version = "0.3.1"
license = "Apache-2.0"
authors = ["…"]
source = "https://…"
sdk = ">=0.4, <0.5"
platforms = ["windows", "macos", "linux"]
kinds = ["site"]
[activation]        … (§5.2)
permissions = ["act.focus", "read.text-fields"]
[[commands]]        … (§5.4)
[[bindings]]        …
[[settings]]        … (§8.1)
output_types = ["whatsapp.message"]
```

### 10.3 Versions and compatibility

- The SDK has a version. Until AGENTS.md's notice is lifted it stays below 1.0 and any minor
  version may break. Okur's own modules are updated in the same commit as any change.
- An extension declares the SDK range it supports. Okur refuses one outside its range and says
  why, in words, not a code.
- After 1.0:
  - breaking changes only in a major version, at most once a year;
  - the Wasm interface is versioned, and Okur keeps an adapter for the previous major version,
    as Zed keeps its older extension interfaces (research §9.2);
  - each extension's replay tests, run against the new version in the registry's CI, tell
    authors and agents exactly what broke. NVDA's yearly cliff (28% of store add-ons
    compatible on release day of 2026.1, research §3.12) is what this avoids.

### 10.4 Distribution and review

- **A registry**: a git repository of manifests, with each version's hash, source link,
  license, permissions and replay results (research §3.8 for NVDA's model).
- **Source required.** The registry's CI builds the Wasm from the published source. No
  binary-only extensions in the registry (research §2.9 on JAWS's move to binary-only).
- **Submission is a pull request**, which an agent can open as easily as a person. NVDA's
  store takes submissions through an issue form (research §3.8).
- **Automatic checks**: the manifest, the build, the permissions, the replay tests on
  Windows, macOS and Linux.
- **Human review** for a first-time publisher, for any permission beyond the default, and for
  global extensions.
- **Signing.** The registry's CI signs what it builds; Okur checks the signature and the hash
  before loading (D12).
- **Local extensions** (development, or private): Okur asks you to approve each by its hash,
  and again after any change, as Orca does (research §4.1).
- **Clean room in the registry**: no extension may contain or translate Freedom Scientific's
  scripts; review checks this.

## 11. Testing and proof

### 11.1 The replay file

- One file records the engine's input: keys with their times, and accessibility events as
  Okur's own types (`AccessibilityEvent` is already serializable, `event.rs:393-394`).
- It is platform-neutral: recorded on Windows, replayed on Linux CI.
- It sits beside the platform trace that parity uses today
  (`crates/okur-platform/src/event_trace.rs`), and is written by the developer interface or
  by `okur trace`.
- Personal content (messages, names) never enters a test committed to a public registry; the
  developer interface offers to replace text with placeholders before saving.

### 11.2 An extension's tests

- Each test is a replay file and the speech expected, step by step, with the settings it ran
  with.
- `okur ext test <folder>` replays every test hermetically, with the extension on, and prints
  matched and mismatched steps, as `okur parity` does.
- The registry's CI runs them on every platform and on every SDK version the extension claims.

### 11.3 JAWS parity stays clean

- `okur parity` runs with outside extensions off, always (vision principle 6).
- Okur's own nine modules are part of Okur and stay in parity.
- A change to the contract or the runtime must leave `parity-snapshot.sh` output unchanged.

### 11.4 Fault tests

The Wasm host gets tests with extensions that loop, trap, run out of memory and miss
deadlines. Each must leave Okur speaking its default, hermetically, and once live (`#[ignore]`
live test).

## 12. Roadmap and milestones

Each milestone ends with tool output that proves it. A refactor changes no speech: before and
after, `skills/jaws-reference/scripts/parity-snapshot.sh` with `diff -r` empty, `cargo test`
passing with assertions unchanged, and the pre-commit gate. Behavior changes are separate
commits with their own tests. Live checks on the dev box need your permission each time, as
today; old-pc work follows [`verification-loop.md`](verification-loop.md).

### Now, before the macOS port

**M0. Groundwork: typed speech and messages.**
- Finish announcements as data: `SpeechFormatter`, then the structured page line.
- Typed parts and voice contexts (§6.1).
- Every user-facing literal into the English catalog (§6.2).
- Absorbs: structure plan step 4 and its spacing decision.
- Done when: no role mark is written outside the renderer; the literal-outside-catalog test
  passes; parity output unchanged.
- State (2026-10-18): the catalog, typed parts, the verbosity model and the catalog tests are
  in; what is left is in [`messages-and-verbosity.md`](messages-and-verbosity.md).

**M1. The settings registry and the JAWS import.**
- The registry with scopes and lifetimes (§8).
- `auto_forms_mode`, `access_keys` and the existing settings move into it.
- The JCF import; replay reads every recorded setting.
- Quick Settings and Settings Center generated from the registry, with tree and edit controls
  in `okur-ui`.
- Capture records the Windows display language, JAWS's language folder, and every target
  application's JCF.
- Absorbs: helper dialogs plan step 9.
- Done when: parity unchanged with the imported reference settings; a hermetic test per scope
  layer; a live check of Quick Settings over Chrome.

**M2. The contract, version 0, on Okur's own modules.**
- `okur-ext` with activation, events beyond focus, commands, interceptors, settings, dialogs,
  lifecycle (§4.3, §5.4).
- The nine modules move onto it; application modes leave `AppMode`.
- A table of open dialogs replaces the two dialog fields.
- Absorbs: structure plan step 3's "command handling".
- Done when: the engine has no application branches; dispatcher tests with mock extensions;
  parity unchanged.

**M3. Keys decided before they run.**
- M3a: the decision and execution split (§7.3), the key table as data, scopes in bindings,
  user overrides.
- M3b, when measurements ask for it: the engine leaves the hook thread.
- Absorbs: helper dialogs plan step 10 (layers) as a table feature.
- Done when: a test proves the decision never executes; `okur latency` before and after shows
  hook time no worse; a user key file overrides a binding in a test.

**M4. Replay and the developer interface.**
- The platform-neutral replay file (§11.1) and `okur replay`.
- The read-only developer interface and `okur mcp` (§4.6).
- Done when: a recorded live session replays to the same speech on Linux CI; an agent reads
  the speech history of a live run through MCP.

### Next, when M1 is in

**M5. Verbosity, voices and a second language.**
- Verbosity levels, output modes, message lengths, the speech scheme (§6.5), with JAWS's
  defaults and old-pc's recorded settings.
- Voice contexts with the synthesizer (§6.4); a sound output for scheme sounds.
- The Turkish catalog and language setting (§6.2); symbols per language.
- Language switching, off as on old-pc.
- Old-pc work: the verbosity dialog's items, the JCF keys behind them, the voice contexts by
  pitch.
- Done when: parity unchanged with the reference profile; a hermetic test per output type;
  Turkish hermetic tests pass; the pseudo-locale run finds no literal.

**M6. Driver registries.**
- Speech drivers behind a registry, with speech sequences and index marks (§5.6).
- The braille driver interface, with a test driver.
- Done when: adding a driver touches one registration; Say All moves the cursor by index
  marks in a hermetic test.

### After Okur is stable on Windows and macOS

**M7. The runtime spike.**
- The same focus filter as a Rust call, a Wasm component with a Rust guest, a Wasm component
  with a JavaScript guest, and QuickJS.
- Measured with `okur latency` on the dev box and old-pc: p50 and p99 per call, load time,
  memory per extension, binary size.
- Done when: the measurements and the decision are written into this page; proposed pass
  marks are p99 under 1 ms per filter call and under 50 ms to load.

**M8. The Wasm host, data extensions and safety.**
- WIT generated from or checked against `okur-ext`; the data host; permissions; budgets;
  failure handling; kill switch; safe start (§9).
- Done when: the fault tests pass (§11.4); one live check with a looping test extension
  shows Okur still speaking.

**M9. The first outside extension and the developer skill.**
- WhatsApp Web commands, proven by replay.
- The `okur-developer` agent skill: SDK documentation, examples, the replay workflow, OpenDesk
  and old-pc workflows; the developer interface gains reload.
- Done when: the extension's replay tests pass on three platforms; you use it live; an agent
  writes, tests and proposes a small extension from a sentence.

**M10. The registry.**
- The registry repository, CI builds, signing, review rules (§10.4).
- Done when: an extension goes from pull request to signed install without manual steps
  beyond review.

### How the existing plans map

| Existing plan step | Milestone |
|---|---|
| Structure plan step 3, command handling in application modules | M2 |
| Structure plan step 4, announcements as data, the structured page line | M0 |
| Structure plan, "Later": the read-only developer MCP server | M4 |
| Helper dialogs step 9, settings registry, Quick Settings, Settings Center | M1 |
| Helper dialogs step 10, layers | M3 |
| Research roadmap phases 0 to 4 (research §11) | M0 to M10, regrouped here |

The macOS port should start after M2 and M3a, so it is built on the contract and on the
decide-then-run key path.

## 13. Risks

- **Mode races when the engine leaves the hook thread.** A key's decision may depend on the
  previous key's effect. Mitigation: stage 1 keeps one thread; stage 2 waits for pending keys,
  bounded, and only when measured necessary (§7.3).
- **Catalog migration changes speech by accident** (spacing, quirks). Mitigation: parity
  snapshot diff on every commit, and a per-id comparison with the old literal during the move.
- **JAWS's output types are not documented in prose.** Mitigation: observation of JAWS's
  dialog and settings files on old-pc (§6.5); until then output types are Okur's guesses,
  marked so.
- **Wasm component tooling changes fast**, and Wasmtime has monthly releases and security
  fixes (research §9.2). Mitigation: follow a long-term-support line; keep the contract in Rust
  so the runtime can change.
- **JavaScript guests may be too large or slow.** Mitigation: Rust guests and data first; the
  spike decides; QuickJS as the fallback.
- **Scope creep before stability.** The vision puts the SDK after Windows and macOS are
  stable. Mitigation: M0 to M4 are refactors and tools that pay for themselves now; M7 onwards
  waits.
- **The developer interface exposes private content.** Mitigation: off by default, local only,
  protected fields never sent, an announcement when it is on (§4.6).
- **Clean room through extensions.** Someone may submit translated Freedom Scientific script
  logic. Mitigation: registry policy and review (§10.4).
- **Three hosts to maintain.** Mitigation: one contract; the process host only when a real
  need arrives (D14).

## 14. Open decisions in detail

**D1. The runtime.** Wasm components give enforced permissions, deadlines and one artifact
(§9.1). The cost is a young toolchain and large JavaScript guests. Default: Wasm, decided by
the M7 spike; QuickJS in-process if Wasm misses the pass marks.

**D2. Okur's own modules.** JAWS ships its application scripts with JAWS (research §2.9). Keeping
Okur's modules in Rust keeps them in parity and fast. Default: Rust, in the repository.

**D3. Speech language.** The reference JAWS speaks English on a Turkish Windows (research
§10.1). Default: English; a setting chooses another catalog.

**D4. First translation.** Default: Turkish, translated by an agent and reviewed by you.

**D5. Verbosity defaults.** Default: JAWS's own defaults for a new install; replay always uses
the recorded settings of each evidence file.

**D6. Physical keys.** JAWS binds physical keys (§7.4). Default: physical, as JAWS. Two laptop
keys need evidence first.

**D7. Key precedence.** Default: application and site extensions may rebind inside their scope;
global ones may not take Okur's keys; your overrides win.

**D8. The kill switch chord.** It must be free in JAWS's key map, which Okur may read for its
key assignments (AGENTS.md §1.4). Default: choose after reading `Default.JKM`'s assignments on
old-pc; announce the chord in key help.

**D9. Protected fields.** Default: no permission exists; the developer interface masks them too.

**D10. Network.** Default: none; per-host permission approved at install.

**D11. Global extensions.** Default: allowed with your approval, without Okur's keys.

**D12. Distribution and trust.** Default: a source-required registry, CI-built and signed;
local extensions approved by hash.

**D13. Porting third-party scripts.** Default: not now. AGENTS.md §1.4 forbids reading any
script source, and the most prolific free script author has shipped most scripts without
source since 2019 (research §2.9). If an author offers source and permission, ask for legal
advice first.

**D14. Python.** Default: no Python in Okur's process; a process host after M10 if people ask.

**D15. The developer interface early.** It helps develop Okur itself. Default: M4, read-only.

**D16. Timing.** Default: M0 to M4 now and before the macOS port; M5 and M6 when M1 is in; M7
onwards after stability.

**D17. Sounds and braille.** Default: a sound output with verbosity in M5; the braille driver
interface in M6; the first braille driver after the macOS port.

**D18. Speech and Sounds scheme files.** JAWS stores schemes in their own files, `.smf`
(research §2.5). AGENTS.md lists `.jcf` and `.vpf` as readable user settings. Default: treat
a user's own scheme file as user settings once you agree; otherwise learn schemes from JAWS's
Speech and Sounds Manager dialog only.

## 15. Glossary

- **Announcement**: what Okur says for one event, as lines of typed parts.
- **Part**: one piece of an announcement: a name, a role, a state, a position, a message…
- **Output type**: the category of a message or part, which verbosity turns on or off (JAWS's
  term).
- **Output mode**: whether an output type is spoken at a verbosity level.
- **Verbosity level**: Beginner, Intermediate or Advanced, as in JAWS.
- **Voice context**: the voice settings an utterance uses: message, PC cursor, JAWS cursor,
  keyboard, menu and dialog (JAWS's voice profile sections).
- **Message catalog**: the file of Okur's words for one language, by message id.
- **Fluent**: Mozilla's message format and its Rust crates.
- **Key table**: the merged bindings that decide whether a key is Okur's.
- **Routing snapshot**: the few facts the hook needs to decide a key, published by the engine.
- **Interceptor**: a handler that can change what Okur is about to say, under a 2 ms deadline.
- **Observer**: a handler told about an event after the fact.
- **Contract**: the Rust types and traits every extension uses (`okur-ext`).
- **Host**: where extensions run: built-in, data, Wasm, process.
- **WebAssembly component, WIT**: a portable sandboxed module, and the language that describes
  its interface.
- **Epoch deadline**: Wasmtime's way to stop a running handler at a time limit.
- **Permission**: what an extension may do, approved at install.
- **Registry**: the repository of reviewed, signed extensions.
- **Replay file**: a recorded session (keys and events) that a test plays back without the
  application.
- **Parity**: comparing Okur's speech with JAWS's recorded speech (`okur parity`).
- **Reference machine**: old-pc, where JAWS is recorded.
- **JCF, VPF**: JAWS's user settings files: configuration and voice profiles.
