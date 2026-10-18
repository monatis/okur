# Extension SDK: research and recommendations

**Status:** research only, 2026-10-18. Nothing here is built. It extends
[`extension-sdk-vision.md`](extension-sdk-vision.md) (the goal and principles) and feeds the
structure work in
[`docs/plans/2026-10-15-structure-for-macos-and-extensions.md`](../plans/2026-10-15-structure-for-macos-and-extensions.md).
The design built on it, for approval, is
[`extension-sdk-proposal.md`](extension-sdk-proposal.md); "proposal §n" below points there.

**Code read at:** `9ccf0ba` (branch `old-pc-round-3`). Another session had uncommitted edits in
the working tree while this was written; every `path:line` below is a line of `9ccf0ba`
(`git show 9ccf0ba:<path>`), not of the working tree.

**Clean room.** What this page says about JAWS comes from Freedom Scientific's public
documentation, help, training pages and release notes, and from prose written by third
parties. No script source, compiled script, script documentation file, key map or other JAWS
program file was opened, Freedom Scientific's or anyone else's (AGENTS.md §1.4). NVDA is
GPL-2.0: its documentation and source were read to learn its architecture; no NVDA code is
copied here.

**How claims are backed.** Web sources are numbered `[J1]`, `[N1]`, `[R1]` and listed in
[Sources](#sources). Code is cited by path and line. Where a claim could not be confirmed it
says "unverified" and how to check it.

## Summary

**Findings.**
- JAWS and NVDA solve the same problem in the same shape: code per application that loads by
  itself, layered over default behavior, reacting to events, adding keys and changing speech.
  JAWS adds a mature data side (key maps, message files with long and short forms and
  categories, per-application and per-site settings, labels, PlaceMarkers, Flexible Web).
  NVDA adds open drivers (speech, braille, vision), typed extension points and a store. §2,
  §3, §5.
- NVDA runs add-ons with full privilege, on its main thread, with no time limit; a watchdog can
  notice a frozen core but cannot stop a Python loop; NV Access now plans a separate "secure
  add-on runtime". No source describes any isolation of JAWS scripts either. Orca added user
  extensions in 2026, approved by hash, without a sandbox. §2.10, §3.10, §3.13, §4.1.
- JAWS's key maps already decide whether a key is consumed before any script runs. Windows
  silently removes a low-level hook that is too slow, and Microsoft advises handing the work to
  another thread. The vision's "keys decided by table" is the design these constraints leave.
  §2.3, §9.1.
- Okur is closer than it looks: an `AppModule` trait with 13 hooks and nine modules, commands as
  values with JAWS's names, one key table, announcements as parts, data-described dialogs, an
  asynchronous request model for reading applications, and a replay harness. §7.
- The gaps: application modules see only focus and cannot add keys; the command set and key
  map are closed; whether a key is consumed depends on running the command, on the hook thread;
  announcement parts hold English words; about 500 user-facing English literals; settings have
  no scopes, verbosity or language; drivers are closed enums; no braille; no
  platform-neutral replay. §7.1-§7.10.

**Recommendations.**
1. Start the groundwork now, with no change in speech (§11 phase 0): finish announcements as
   data, give parts kinds, move every message into a catalog with ids, decide keys before
   running them, record languages in evidence.
2. Build the internal contract next (phase 1), on the nine built-in modules, before macOS is
   finished: events beyond focus, commands, announcement filters, settings with scopes,
   generic dialogs, a key map file, a driver registry, platform-neutral replay, a read-only
   developer interface.
3. Model verbosity on JAWS (levels, message categories, long and short messages, speech and
   sounds per role and state, punctuation) with JAWS's defaults, and translate Okur's words
   through Fluent catalogs with context ids; English stays JAWS's English and parity-verified
   (§10).
4. For outside extensions: a declarative layer first (keys, settings, labels, rules,
   catalogs); then WebAssembly components on Wasmtime for code, with epoch deadlines, imports
   as permissions, Rust and JavaScript guests; an out-of-process host for Python, GPL drivers
   and anything native. Decide after measuring (§9.6).
5. Prove every extension by replaying recorded interactions with expected speech; keep JAWS
   parity measured with extensions off (R26).

The owner's decisions needed are in §12.

**Contents.** §1 what was asked · §2 JAWS scripting · §3 NVDA add-ons · §4 other screen
readers · §5 side by side · §6 patterns · §7 Okur today · §8 requirements · §9 runtime options ·
§10 languages and verbosity · §11 roadmap · §12 open questions · §13 not verified · Sources.

---

## 1. What was asked, and what was already decided

The owner asked for a study of JAWS scripting and NVDA add-ons, of how far Okur's code is from
an extension system, and of the refactoring it needs, covering verbosity settings and the
translation of Okur's own messages, and extension points such as additional speech engines,
navigation keys and dialogs (session of 2026-10-18).

Already decided ([`extension-sdk-vision.md`](extension-sdk-vision.md)):
- The SDK comes after Okur is stable on Windows and macOS. Until then the code is shaped for
  it.
- No JAWS script language: Okur copies the capabilities, not the language.
- One contract, defined in Rust; Okur's own application modules use it first.
- Keys are decided by table on the hook; extension code runs afterwards, off the hook thread.
- Announcements are data; extensions change parts, never strings.
- Objects are platform-neutral.
- Budgets: a slow handler is skipped; one chord turns every extension off.
- Parity with JAWS is measured with extensions off; an extension is proven by replaying a
  recorded interaction.
- The developer interface is off by default and local; extensions declare what they need.
- Core defects get core fixes.

The ideation exchange the owner pasted on 2026-10-15 (session transcript) proposed a Rust SDK
with JavaScript (Boa) and Python bindings, a built-in developer MCP server, an
`okur-developer` agent skill and agent-assisted porting of third-party scripts. The vision page
narrowed the runtime choice to "one script runtime, chosen by measuring handler latency":
JavaScript (QuickJS or Boa) or WebAssembly components, Python later. This page goes deeper on
that choice in §9.

The WhatsApp Web work of 2026-10-17 is the concrete case: the owner said WhatsApp-specific
commands should wait for the SDK and become an extension
([`docs/handoff/2026-10-17-whatsapp-web.md`](../handoff/2026-10-17-whatsapp-web.md), first
paragraph).

## 2. JAWS scripting

**About the sources.** Freedom Scientific's own pages (the Script Manual, FSDN, Surf's Up, the
Doc Center) answered automated fetches with HTTP 403. Statements below that cite them come from
search engines' excerpts of those exact pages, marked "(excerpt)"; several were confirmed by
more than one excerpt, none was read in full. They should be checked in a browser before
anyone relies on a detail. Vispero, Freedom Scientific GmbH, Doug Lee, Hartgen and other pages
were read directly. Function, event and file names appear only as names.

### 2.1 The language and its model

- JAWS has had a scripting language since 1996 [J18]. It is proprietary and compiled; the
  compiled form also protects the source [J33].
- A script file holds **scripts** and **functions**. A script can be bound to a key; a
  function cannot. Script Manager's "Can be Attached to Key" check box makes the difference
  [J1], [J2] (excerpts).
- **Event functions** are called by JAWS when something happens in Windows. Freedom
  Scientific writes them; users "can modify these functions, but they cannot create new ones"
  [J3] (excerpt). Documented names include `AutoStartEvent` (an application starts or becomes
  active), `FocusChangedEvent` (focus moves between applications, dialogs and controls),
  `NewTextEvent` (new text is written to the screen) and `BottomEdgeEvent` [J1] (excerpt).
  Third-party tutorials and tools name many more: `AutoFinishEvent`, `WindowActivatedEvent`,
  `ScreenStabilizedEvent`, `KeyPressedEvent`, `ValueChangedEvent`, `DocumentLoadedEvent`
  [J6], [J7]; Doug Lee's BX tool can log about 245 event and "say" functions [J7].
- **"Say" functions** such as `SayHighlightedText` are functions JAWS calls to speak something;
  application scripts override them to change what is said [J8] (excerpt).
- **Built-in functions**: "over 600" in 2003 [J4]; the current count was not found. FSDN, a
  compiled help file updated April 2026, is the reference [J5] (excerpt).
- **UI Automation from scripts**: an object created through `FreedomSci.UIA` gives elements,
  tree walkers and event handler registration (automation, focus, property and structure
  events); it dates from JAWS 15, updated in JAWS 18 [J9] (excerpt).
- **Types**: integer, string, handle and object; no floating point [J32] (excerpt). Arrays
  and collections came in JAWS 11 update 1 [J15] (excerpt). "No built-in error handling" and a
  compiler that checks syntax only [J29] (excerpt).
- **A new compiler**: the Modern Script Compiler appeared for early adopters in December 2024
  [J26] and is the default in JAWS 2026, whose first new language feature is a bracket and dot
  syntax for nested collections, presented as groundwork for future language improvements
  [J27], [J28]. No sign of a move to a general-purpose language was found.

### 2.2 Layers: default, application, domain

- **Two layers at least.** The default script file loads at startup and stays; an
  application's script file loads when that application starts or gains the focus and unloads
  when it loses it [J4].
- **How an application is recognized**: by "only the first portion of the executable file
  name", with aliases in `ConfigNames.ini` [J11] (excerpt); third-party installers add entries
  there [J12].
- **Lookup order for functions**: application script file, then the default script file, then
  built-in functions; a same-named function in the application file wins [J8] (excerpt). An
  override usually does its own work and calls the same name, which reaches the next layer
  down; an override of a chained event that forgets to do so stops the other layers' versions
  from running. This last point comes from community posts seen only in search summaries; it
  was not found in the official manual (unverified).
- **Linking files**: `Use` links one compiled script file to another so that its functions
  can be called; `Include` pulls headers and message files in at compile time [J13]
  (excerpt).
- **One file of each type per application.** Doug Lee described in 2003 that combining script
  sets meant chaining `Use` statements and renaming files, and that separate key maps per
  script set were not possible [J14]. His installers now link script sets through a chain
  manager file with priority numbers, so that his scripts can load alongside Freedom
  Scientific's for the same application [J12]. Collisions still happen: his VS Code scripts
  were withdrawn after clashing with another author's files [J60].
- **Domain scripts (JAWS 17, 2015)**: scripts "that only load when a particular web site is
  active", named after the domain, loaded "in addition to the host application scripts"; a
  whole set (scripts, key map, configuration, dictionary) can belong to a domain [J15], [J16],
  [J17] (excerpts), [J19].
- **Shared and user folders**: shared files under
  `ProgramData\Freedom Scientific\JAWS\<version>\Settings\<language>`, the user's under
  `AppData\Roaming\…\JAWS\<version>\Settings\<language>` (for example `…\2025\Settings\enu`);
  [J25] (excerpt); a user copy overrides the shared file [J30]. A script set is installed per
  JAWS version and per language.
- **Updates between releases**: since 2024, Freedom Scientific can push script fixes to the
  shared folder between releases; user scripts override them; "Significant features will not be
  introduced using this method" [J30]. Since June 2026, Cloud Settings Sync (early adopters)
  syncs settings "including JAWS scripts" to the Vispero account [J31].

### 2.3 Keys

- **Key maps decide, scripts run.** JAWS looks a key up in the application's key map, then the
  default key map; the script name found is looked up in the application's script file, then
  the default. A key in neither key map is "passed through to the application just as if JAWS
  were not running" [J10] (excerpts). The application key map "always takes precedence"
  [J20] (excerpt).
- **Pass-through scripts**: a key bound to a script is intercepted before the application
  sees it; a script that should also act in the application sends the key on itself (the
  manual's `TypeKey`), or "the keystroke never activates the intended action" [J72] (excerpt).
- This is the vision's principle 2 as JAWS has always done it: **whether a key is consumed is
  decided by the key maps, a table; the script runs afterwards and may send the key on.**
- **Keyboard Manager** views, adds, changes and deletes assignments; adding a key adds an
  assignment rather than replacing one; Freedom Scientific advises against changing its own
  key maps [J20] (excerpts).
- **Key help**: Insert+H lists JAWS keys for the application, Insert+W the application's own
  keys; application scripts add their entries (WhatsApp's, [J54]). Keyboard help mode
  (Insert+1) says what keys do without doing it [J49] (excerpt).

### 2.4 Messages and languages

- **Message files** (`.jsm`, since JAWS 4.0) hold named messages apart from code so that
  "it is much easier to translate those messages"; translators need only the message file
  [J34] (excerpts).
- **Placeholders** `%1` to `%9`, filled by a formatting function; since JAWS 12 update 1 the
  speaking function fills them directly [J34] (excerpts).
- **Long and short messages.** Messages come in long and short forms; the long one is spoken
  at the Beginner verbosity level, the short one at Intermediate and Advanced; the plain speak
  function ignores length, the message functions honor it [J34], [J73] (excerpts).
- **Output types.** Every spoken message carries an output type, a category constant that
  verbosity settings act on [J73] (excerpt). The list of output types lives in a JAWS header
  file, which AGENTS.md §1.4 forbids reading: Okur must learn the categories from the
  documented verbosity options and from behavior, not from the header.
- **Languages.** JAWS "is available in many languages", some from local distributors [J35]
  (excerpt); listed languages include Arabic, Danish, Dutch, Finnish, German, Hebrew,
  Norwegian, Russian, Swedish, Portuguese, French, Hungarian, Italian, Spanish [J35]
  (excerpt), Czech [J28] and Slovak [J35]. A second interface language is added at install
  and chosen from a JAWS Language menu (excerpts, [J74]).
- **No Turkish JAWS interface was found**; Turkish appears as voices only, and Vispero's
  distributor in Turkey shows none [J76]. Unverified either way; the installer's language list
  would settle it. This matters for Okur: the owner's language has no JAWS to compare with.
- **Settings folders are per language** (`enu` for English) [J25]; third-party scripts are
  often English only ("All JAWS scripts are for the English release of JAWS for Windows only",
  Hartgen) [J36]; Doug Lee built his own runtime translation system for his scripts in 2020
  [J37].

### 2.5 Verbosity and settings

- **User verbosity levels**: Beginner (the default, "the highest level of speech verbosity"),
  Intermediate and Advanced. Quick Settings' User Verbosity item cycles them; Settings
  Center's "Customise Verbosity Level" opens, for each level, a list of check boxes, one per
  type of speech message, and a Long or Short message length [J38]. The check boxes' names are
  not in that page; they can be read in JAWS's own dialog on old-pc or the dev box (allowed:
  it is JAWS's help and UI, AGENTS.md §1.4).
- **Speech and Sounds Manager**: schemes map control types, control states and text
  attributes to voices, sounds or replacement text; one scheme is active; schemes are stored
  in `.smf` files; JAWS 14 had 21 built-in schemes with Classic the default [J22], [J40]
  (excerpts). Okur's role marks
  (`\u{1d}`…`\u{1e}`, `announcement.rs:3-6`) model the separate voice JAWS uses for roles and
  states.
- **Punctuation**: None, Some, Most, All; changed for now in Quick Settings or saved in Settings
  Center [J41] (excerpt). The reference machine runs at Most
  ([`verification-loop.md`](verification-loop.md) §3).
- **Quick Settings** (Insert+V, JAWS 13): the focused application's settings, each change
  saved, kept until JAWS exits, or until the focus moves [J42] (excerpt). **Settings Center**
  (Insert+6): all applications, Ctrl+Shift+D for the default file [J43] (excerpt). Okur's
  [`jaws-dialogs-and-visual-features.md`](jaws-dialogs-and-visual-features.md) records both as
  observed.
- **Scripts add options**: Freedom Scientific's application scripts put their own options in
  Quick Settings (WhatsApp's incoming message announcements and typing sound, February and May
  2026; PowerPoint's "use previous scripts", August 2026) [J27]. Third-party packages ship
  Quick Settings files of their own (`.qs` with a companion `.qsm`) [J25] (excerpt). The format
  and any API for it were not found in prose.
- **Per application and per site**: application settings live in the application's
  configuration file (`.jcf`); Personalized Web Settings are saved per domain (`.jsi`) [J23]
  (excerpt).
- **What the reference machine's own settings files show.** Okur's capture tooling records
  JAWS's user settings files into each new evidence file's provenance
  (`skills/jaws-reference/scripts/run-scenario.ps1:225-245` at `9ccf0ba`; reading user
  settings is allowed by AGENTS.md §1.4). A JCF holds only options changed from JAWS's
  defaults. Recorded on old-pc (provenance of
  `corpus/evidence/209-web-title-and-font-evidence.json` for the JCFs and
  `corpus/evidence/207-web-character-review-evidence.json` for the voice profiles):
  - `DEFAULT.JCF`: `[Options] TypingEcho=0`, `KeyboardType=Laptop`, `LanguageDetection=0`;
    `[FormsMode] AutoFormsMode=0`; `[OutputModes] ACCESS_KEY=0|0|0|Access Key`.
  - The `OutputModes` entry names one output type with three values and a display name. The
    three values are presumably one per verbosity level (Beginner, Intermediate, Advanced);
    that is an inference from the format, to be confirmed by changing the item in JAWS's
    verbosity dialog on old-pc and comparing the file.
  - `Chrome.JCF`: `[VirtualCursorVerbosity] VirtualCursorVerbosityLevel=2` (a per-application
    web verbosity level; what level 2 names is not confirmed) and
    `[options] AllowWebAppReservedKeystrokes=0`.
  - Voice profiles (`.VPF`; the active one is "SAPI 5X 64"): per language, one voice per
    context, `Message`, `PCCursor`, `JAWSCursor`, `Keyboard`, `MenuAndDialog`, with `Global`
    as their parent; each with voice name, rate, pitch, punctuation, spelling rate and capital
    pitch where changed; and voice aliases for spelling and capitals. One recorded profile
    (`Persian.VPF`, not the active one) gives the message voice pitch 20, the PC cursor 10 and
    the JAWS cursor 1. So JAWS speaks
    its own messages, the cursor's text and the keyboard echo with separate voice settings:
    Okur's speech output needs a context per utterance (proposal §6.4).

### 2.6 Customization without scripts

What users change in JAWS without writing a script, per site or application:
- **Custom labels** per domain, in the same per-domain file as Personalized Web Settings
  [J23] (excerpt); a newer Label Manager handles them (excerpt).
- **PlaceMarkers** per page, shareable files [J24] (excerpt).
- **Flexible Web** (JAWS 14): hide elements or start reading at one, as rules for a page, a
  site or all sites, with a sound when a rule applies [J44] (excerpt), [J18].
- **Dictionary Manager**: pronunciation and sound substitutions, per synthesizer, importable
  [J45] (excerpt).
- **Graphics labels**, now led by the AI Labeler (Insert+G), which since December 2025 keeps
  its descriptions for later visits [J46].
- **Packaging**: Settings Packager (JAWS 5.10) zips schemes, PlaceMarkers and web settings for
  sharing [J67] (excerpt); Import/Export (JAWS 18) writes one backup with custom scripts,
  dictionaries, PlaceMarkers, Flexible Web rules and voice profiles [J68] (excerpt).

This is the declarative layer §9.6 recommends for Okur: most of it is data, per site or
application.

### 2.7 Tools and automation

- **Script Manager** edits, inserts functions, shows reference information, checks rules and
  compiles; Ctrl+S saves and compiles; a file edited elsewhere is not compiled and JAWS "will
  be unaware of the changes"; a JAWS restart is needed only after changing the default script
  file [J21], [J29] (excerpts), [J4].
- **Debugging**: the manual's advice is temporary speak calls in each branch; event functions
  can repeat speech in ways "analyzing the script will not reveal" [J29] (excerpt). No step
  debugger or breakpoints were found in any documentation (absence unverified).
- **Script Utility Mode** shows window hierarchy, classes, control ids and MSAA objects [J48]
  (excerpt). **Speech History** (Insert+Space, H) keeps the last 500 strings [J50] (excerpt).
  JAWS has no speech viewer like NVDA's; Vispero sells JAWS Inspect, which shows "true speech
  and Braille output" for testing [J51], [J52]. Insert+Q says which settings are loaded
  ("WhatsApp settings are loaded") [J53].
- **FSAPI**: a COM object (`FreedomSci.JAWSAPI`) with members to speak a string, stop speech,
  run a script, run a function, and enable or disable JAWS [J55]; it has shipped since JAWS 8.0
  update 2, and a successful run call only means the call was scheduled [J56] (excerpt). No
  current official page was found. Okur's harness uses it (`okur jaws-history`,
  `skills/jaws-reference/`).
- **Third-party tooling**: Doug Lee's BX explores window, MSAA and UIA trees and logs events;
  binary-only license [J7].

### 2.8 Braille

- Structured mode is the default and adds field type, attributes and label to what Line mode
  shows [J47] (excerpt). JAWS 5.0 added a way to define structured mode [J78] (excerpt), and
  Settings Center has Braille, Structured Mode options, HTML attributes among them [J79]
  (excerpt). Whether today's JAWS has a per-control-type editor was not confirmed. Braille gets
  its own rendering of the same information that speech gets: the reason §10.3 keeps parts
  until rendering.

### 2.9 How scripts ship

- **Freedom Scientific's own application scripts** ship with JAWS: Office, Teams (2021
  release), Zoom, Outlook [J28]; new PowerPoint scripts became the default for PowerPoint
  16.0.20026 and later in August 2026, with a switch back to the old ones [J27].
- **WhatsApp (the owner's case).** Freedom Scientific had WhatsApp scripts for the Microsoft
  Store app by February 2023: Alt+1 to 9 read recent messages, Alt+E, Alt+C and Alt+H move to
  the edit field, chat list and history, announcements can be switched off in Quick Settings
  [J54]. The official Spanish "What's New" lists "Se actualizan los scripts para WhatsApp"
  (WhatsApp scripts updated) under JAWS 2026.2601.197, posted 11 February 2026 [J27], [J57]:
  automatic reading of incoming messages and a typing sound, both switchable in Quick Settings,
  Alt+1 to 9, Alt+E, Alt+H, Alt+C, Alt+Shift+S for the contact's name and status, image
  description (Insert+Space, I, Enter), and key help (Insert+H, Insert+W); May 2026 made
  automatic reading a Quick Settings option and the typing sound work in locked chats; June
  2026 fixed the previous message in braille after Up Arrow from the last one [J27]. So
  February 2026 was an update for the redesigned Windows app, not the first WhatsApp script.
  The English page could not be read (403).
- **Third parties**: Doug Lee's free script sets (WhatsApp revision 41, December 2025, for the
  WebView2 app; Teams, VS Code, and others) [J58], [J59]; BSD-licensed until 2018, binary-only
  since 2019, free to redistribute, no reverse engineering [J37]. His Teams scripts load
  Freedom Scientific's and "enhance, but do not replace" them, and note that "Teams HTML code
  can change without a corresponding change to the version number" [J59]. Hartgen Consultancy
  sells Leasey (a productivity suite of JAWS scripts) and application scripts for Zoom, Teams,
  WhatsApp Classic, WhatsApp Web and X, and a twelve-lesson scripting course for £200 [J36],
  [J61], [J62], [J63].
  Vispero (TPGi) offers scripting services for business applications [J64]; employers'
  guidance lists several scripting vendors and notes "Writing scripts can take some time"
  [J65]; software vendors ship scripts for their own products (Intuit for QuickBooks) [J25].
- **Installing**: vendor installers (Doug Lee's write only to the user folder, any JAWS version
  and language, and edit `ConfigNames.ini`) or copying files and recompiling [J12], [J69]
  (excerpt). Third-party packages declare minimum JAWS versions and drop old ones [J59].
- **How many people write scripts**: no estimate was found. The JAWSScripts mailing list exists
  [J77]; JAWS was the primary screen reader of 40.5% of WebAIM's 2023-24 survey respondents
  [J71] (excerpt).

### 2.10 Strengths and weaknesses

Strengths (from the above):
- Application support is layered and automatic: the right scripts load with the
  application or site, and fall back to the default.
- Keys are decided by tables (key maps); scripts run after.
- Messages are separate from code, in long and short forms, with categories that verbosity
  acts on.
- A large ecosystem: Freedom Scientific's own scripts, free and commercial third parties,
  vendors scripting their own software, scripting services.
- Users customize much without code: labels, PlaceMarkers, Flexible Web, dictionaries, schemes,
  per-site settings.

Weaknesses reported or documented:
- A small proprietary language: no floating point, collections added late, no error handling,
  syntax-only compiler [J29], [J32].
- No debugger beyond speaking from the script; event functions make behavior hard to follow
  [J29].
- One file of each type per application: coexistence needs chaining and naming conventions,
  and collisions happen [J12], [J14], [J60].
- Windows only, and per JAWS version and language folder; third-party scripts are mostly
  English only [J36].
- Application churn: Teams and WhatsApp redesigns forced rewrites by Freedom Scientific and by
  third parties [J27], [J58], [J59], [J61].
- No isolation: a script runs inside JAWS (not stated in a source; JAWS's architecture
  documentation was not available).
- Distribution has moved to binary-only packages [J37], so scripts are hard to learn from or
  port, and Okur may not read them anyway (AGENTS.md §1.4).
- Forum posts contrasted paid JAWS scripting training and a custom syntax with free,
  Python-based NVDA add-ons [J70] (excerpt).

## 3. NVDA add-ons

**About the sources.** NVDA's Developer Guide and User Guide (2026.2), its What's New, its
`master` branch on GitHub (read 2026-10-18; `master` is the 2027.1 development line), the
add-on store repositories and NV Access's posts. Source line numbers are approximate; files are
named so they can be found. NVDA is GPL-2.0; only identifiers are named here.

### 3.1 Package, manifest, install, disable

- **Package**: a zip with the extension `.nvda-addon`, installed through the Add-on Store
  [N1]. Inside: `manifest.ini` and folders per plugin type: `appModules/`, `globalPlugins/`,
  `synthDrivers/`, `brailleDisplayDrivers/`, `visionEnhancementProviders/`, plus
  `speechDicts/`, `brailleTables/`, `locale/<lang>/` and `doc/<lang>/` [N1], [N6]. NVDA puts
  each running add-on's folders at the front of the matching Python package's search path
  [N5], [N6].
- **Manifest keys**: `name` (the id), `summary`, `description`, `author`, `version`,
  `changelog` (new in 2026.1), `url`, `docFileName`, `minimumNVDAVersion`,
  `lastTestedNVDAVersion`; optional sections declare braille tables, symbol dictionaries and
  speech dictionaries [N1], [N5], [N3]. An add-on without a recent enough
  `lastTestedNVDAVersion` is refused or warned about (§3.9).
- **Install tasks**: `installTasks.py` may define `onInstall` (an exception fails the install)
  and `onUninstall` (runs at the next restart, cannot ask the user anything) [N1], [N5].
- **Lifecycle**: install, update, enable, disable and remove all take effect after a restart;
  state is kept in `addonsState.json` with categories such as pending install, disabled,
  blocked and compatibility overridden [N5], [N7], [N8].
- **Safe start**: the command line `--disable-addons` ("Add-ons will have no effect") and the
  exit dialog's "Restart with add-ons disabled" (also with debug logging) [N2], [N9]. The User
  Guide presents this as the way to find a problem an add-on causes [N2].
- **Secure screens**: NVDA runs in secure mode on secure screens; the store, the Python
  console, viewers and configuration saving are off [N2]. Since 2026.1 users choose which
  add-ons are copied to sign-in and secure screens, none by default; the User Guide warns that
  copying add-ons there "is a serious security risk" [N3], [N2].

### 3.2 Plugin types

- **App modules**: a module named after the executable (`notepad.py`) in `appModules/`
  defines an `AppModule` subclass [N1]. The name is the executable without extension, lower
  case; add-ons can register aliases, which win; hosted applications (Java, WebView2) are named
  after the hosted app [N1], [N30]. One instance per process; if none exists or it fails to
  load, the base class is used and the error logged [N30]. An application switch is
  announced to listeners (`post_appSwitch`) [N30]. **Sleep mode** (NVDA+Shift+S) drops events
  and scripts for an application, scripts can opt out [N30], [N31].
- **Global plugins**: `GlobalPlugin` subclasses loaded at startup; import and constructor
  errors are logged and the plugin skipped; NVDA+Ctrl+F3 reloads plugins [N33], [N2].
- **Synthesizer drivers**: a `SynthDriver` with a name, a description, `check()` (is it
  available), `speak(sequence)`, `cancel()`, and declared settings (voice, variant, rate,
  volume, pitch, inflection, language and others) from which the voice settings panel is built
  [N40], [N45]. "Auto" tries OneCore, eSpeak, then silence [N40]. Speech arrives as a sequence
  of text and commands: index marks, character mode, language changes, pitch, volume, rate,
  breaks, phonemes [N41]. Drivers load in process; since 2026.1 the only out-of-process
  drivers are the built-in 32-bit SAPI ones, run by a 32-bit helper that "is not a
  generalized 32 bit shim for add-ons" [N18]. Add-on authors built their own bridges (the
  IBMTTS driver uses a 32-bit host written in Rust with named pipes and shared memory) [N69].
- **Braille display drivers**: cell count, `display(cells)`, a gesture map, automatic detection
  by USB and Bluetooth ids on a background worker, standard HID braille [N42], [N43];
  translation through liblouis; add-ons can ship liblouis tables since 2024.3 [N1].
- **Vision enhancement providers**: settings, `canStart`, extension points for focus, caret,
  browse mode and mouse moves [N44].

### 3.3 Objects and overlay classes

- An NVDA object's class is assembled at run time: the best API class (IAccessible, UIA…),
  overlay classes it adds itself, then the app module's and each global plugin's
  `chooseNVDAObjectOverlayClasses`, which insert their own classes [N32]. An overlay class
  changes properties (name, role, states) and adds scripts and event handlers for objects of
  one kind; app modules can also adjust a new object in `event_NVDAObject_init` [N32], [N1].
- **Tree interceptors** (browse mode) intercept events and scripts for a document; their
  pass-through flag is focus mode [N35].
- This is the design behind R3's "activation by kind of object" and behind much of what NVDA
  add-ons do for one control in one application.

### 3.4 Events and extension points

- **Events**: handlers named `event_<name>`; at plugin level they receive the object and a
  `nextHandler`; calling it continues the chain, not calling it stops the event [N1]. The chain
  is global plugins, the app module, the tree interceptor, then the object [N31]. Focus events
  jump the queue, and stale focus speech is dropped [N31]. Raw events are filtered before
  objects are created because that is expensive, and plugins can ask for events they need
  [N31]. Objects have handlers for live regions, typed characters, selection, state, focus,
  value, name, description and caret changes [N32].
- **`extensionPoints`**: small typed hooks NVDA calls, with new points added in 2023.1,
  2024.1 and 2025.1 [N37], [N3]:
  - *Action*: notify listeners;
  - *Filter*: each handler may change a value passed along;
  - *Decider*: any handler can say no;
  - *AccumulatingDecider* and *Chain* (2023.2);
  - handler exceptions are logged and skipped; handlers are held by weak reference.
  The Developer Guide lists 44 points [N1], among them `filter_speechSequence` (change speech
  before the synthesizer), `pre_speech`, synthesizer index and done notifications, braille
  `pre_writeCells`, input deciders that can swallow a key or a gesture, configuration profile
  switches and saves, application switches, browse mode changes, sound and beep deciders,
  session lock and secure desktop changes [N1], [N38], [N39].
- **In practice, monkeypatching.** Popular add-ons replace core functions: Speech History
  wraps NVDA's speak function instead of using an extension point [N67]; Windows App
  Essentials patched event paths when the core lacked a feature [N65].

### 3.5 Scripts, gestures, key remapping

- Scripts are methods `script_<name>(self, gesture)` with a `@script` decorator giving the
  description, category and gestures, and flags (may propagate to ancestors, allowed in sleep
  mode, speaks in on-demand mode) [N36], [N1]. Gesture ids name the source: `kb:`, `br(…)`,
  `ts:` [N1].
- **Lookup order**: the gesture's own object, global plugins, the app module, the braille
  driver, vision providers, the tree interceptor (unless in focus mode), the focus object and
  its ancestors, then global commands; at each, the user's gesture map and the locale's map can
  remap [N36]. The first script found runs and the key is not passed on; a script that wants the
  application to get the key sends it itself (`gesture.send()`), as Clipspeak does for
  Ctrl+C [N1], [N68]. This is JAWS's pass-through script (§2.3), with the difference that
  NVDA's lookup walks Python objects for every key.
- **Users remap** in the Input Gestures dialog; scripts without a description do not appear
  there [N1], [N2]. Double and triple presses are counted by the script [N36].

### 3.6 Settings, dialogs, profiles

- **Settings panels**: a `SettingsPanel` subclass with `makeSettings` and `onSave`, appended to
  the NVDA settings dialog's categories and removed when the add-on ends [N45], [N46].
  Other dialogs are plain wxPython on the main thread [N46].
- **Configuration**: `config.conf`, a configobj tree with a validation spec; an add-on adds a
  section with typed defaults [N47], [N67].
- **Profiles**: named sets of settings stacked over the base configuration, activated by
  triggers (the current application, Say All) or by hand; the newest active profile wins;
  some sections are base-only [N47], [N2]. Add-ons listen for profile switches [N1]. Profiles
  are NVDA's per-application settings.

### 3.7 Speech, verbosity, languages

- **Speech modes**: off, beeps, talk, on demand (speaks only for scripts marked so) [N48].
- **Order of processing** in `speak()`: add-on filters see the sequence first, then NVDA
  applies mode, language, speech dictionaries, symbol processing and normalization [N48]. So
  add-on filters see raw text.
- **Symbol levels**: none, some, most, all, plus character [N49]. Symbol dictionaries per
  locale (`symbols.dic`, and CLDR data including emoji), English merged underneath others, a
  user dictionary on top; add-ons ship symbol and speech dictionaries [N1], [N49].
- **Speech dictionaries**: default, per voice, temporary [N2], [N50].
- **Verbosity**: NVDA has no levels like JAWS's; it has many separate switches (object
  presentation, document formatting) in the settings dialog (not researched in detail here).
- **Language switching**: options to switch language or dialect when a page or program
  declares one [N79].
- **Translation**: gettext functions `_()`, `pgettext`, `ngettext`, `npgettext`; every
  translatable string needs a translators' comment, and a check enforces it [N51]. NVDA's
  interface messages and, since September 2024, its User Guide are translated on Crowdin;
  symbols, character descriptions and gestures still on GitHub; a string freeze precedes each
  release [N52], [N53].
- **Add-on translation**: `addonHandler.initTranslation()` binds the gettext functions for the
  add-on's own catalog in `locale/<lang>/LC_MESSAGES/nvda.mo`; a translated `manifest.ini` may
  override the summary and description [N5], [N1]. The add-on template ships a Crowdin
  workflow for a shared community project or the author's own [N55].

### 3.8 The Add-on Store

- **History**: 2023.2 replaced the Add-ons Manager with the Add-on Store [N13]; 2024.3 added
  update notifications, 2025.1 automatic updates, 2026.1 VirusTotal results and changelogs in
  the store [N3].
- **Channels**: stable, beta, dev, and external (installed from a file); updates are checked
  every 24 hours [N2], [N21].
- **Architecture**: `addon-datastore` on GitHub is "the source of truth"; a GitHub issue form
  creates a pull request adding a JSON file per version (id, version, display name, publisher,
  description, NVDA versions, channel, download URL, SHA-256, source URL, license, translations,
  optional VirusTotal results); NV Access's server serves views per language, channel and API
  version [N21], [N23], [N24], [N29].
- **Review**: "NV Access doesn't require a manual review of the add-on (code or user
  experience)"; add-ons "do not undergo human security audits"; "Add-ons are run at user's own
  risk" [N20]. A submitter's first add-on id needs NV Access's approval (up to two weeks);
  automated checks verify the schema, the URL, the hash and the manifest; flagged VirusTotal
  results are held for a human [N25], [N26], [N28].
- **No code signing**: integrity is HTTPS and SHA-256 [N21], [N22]. Publisher identity is a
  GitHub account [N21]. Community reviews are GitHub Discussions [N2].
- **Scale**: 397 registered add-ons on 6 May 2026 [N72]; about 410 stable entries for API
  2026.2 in the store's web listing, counted on 2026-10-18 (approximate) [N29].

### 3.9 Versions and compatibility

- An add-on is compatible when its `minimumNVDAVersion` is at most the running API version and
  its `lastTestedNVDAVersion` is at least the oldest API version still accepted
  (`BACK_COMPAT_TO`) [N10], [N11]. Incompatible add-ons are blocked [N5].
- **Breaks once a year**: the year's first release is "the only release window where
  signature breaking changes… are permitted"; public API is every Python symbol without a
  leading underscore; pip packages may change at any time [N1]. Deprecations are announced on
  a read-only API mailing list [N15], [N16].
- **Breaking releases**: 2019.3 (Python 3, speech rewrite), 2021.1, 2022.1, 2023.1, 2024.1
  (Python 3.11), 2025.1, 2026.1 (Python 3.13, 64-bit, released 6 May 2026), 2027.1 in
  development [N10], [N3], [N17], [N19].
- **Overriding**: since 2023.2 users can enable an add-on that is too old "at your own risk";
  an add-on that needs a newer NVDA cannot be overridden; overrides reset at the next API
  upgrade [N13], [N12], [N5].

### 3.10 Security

- Add-ons are Python in NVDA's process, with no permission model; the store says add-ons "may
  have unrestricted functionality and access to information" [N29]; NV Access says add-ons
  can "run any code, even with administrator privileges" and that it "does not review the code"
  [R86]. NVDA runs with UIAccess [N58].
- A security advisory describes a malicious add-on hijacking NVDA's UAC elevation; it stays
  open as "a consequence of an intended feature" [N59]. Several advisories concerned escapes on
  secure screens [N60].
- **Plans**: NV Access's roadmap (6 May 2026) lists a medium-term "Secure add-on runtime": "A
  runtime execution environment, separate from NVDA, that allows untrusted add-ons to run with
  limited access", first for speech synthesizers and braille displays [N61]. A closed pull
  request prototyped low integrity, restricted tokens and AppContainer for it [N62]; the
  discussion found that out-of-process add-ons can only use what can be proxied [N63].

### 3.11 Tools and testing

- **Add-on template**: SCons build, `buildVars.py`, Markdown documentation, GitHub Actions for
  builds, releases and Crowdin [N54].
- **Python console** (NVDA+Ctrl+Z) with the focus, navigator and caret objects at hand; log
  viewer, Speech Viewer, Braille Viewer; NVDA+F1 logs the navigator object's details [N1],
  [N2].
- **Developer scratchpad**: an off-by-default setting loads app modules, global plugins and
  drivers from a scratchpad folder, never in secure mode [N1], [N6].
- **NVDA's own tests**: unit tests and Robot Framework system tests, which start NVDA with a
  spy global plugin and a speech-spy synthesizer driver [N56], [N57].
- **No add-on test framework** is documented [N56]; community advice is manual testing with
  each beta [N46], [N74].

### 3.12 Pain points

- **The yearly cliff.** On 6 May 2026, 113 of 397 stable add-ons (28.4%) were compatible with
  2026.1, against 309 with 2025.3.3 [N72]; on 11 May, 181 stable add-ons, against 309 for
  2025.x [N73]. Causes in the 64-bit move: 32-bit DLLs and `.pyd` files, removed APIs,
  pointer-size bugs, old dependencies; overriding does not help 32-bit binaries [N74].
- **Python and toolkit churn**: Python 2 to 3, 3.11, 3.13, wxPython updates; NV Access's reason
  is keeping NVDA "secure, current, and supported" [N10], [N75].
- **Monkeypatching** makes add-ons collide with each other and with core changes [N65], [N67].
  In March 2026 NV Access warned about "vibe coding" add-ons whose authors don't understand
  them, listing data deletion, data leaks, conflicts and inefficiency [N27].
- **Performance**: add-on code runs on NVDA's core thread (§3.13); add-on changelogs mention
  fixing input lag [N70].
- **Translation burden**: each add-on wires up its own translation workflow [N55].
- **Features move into the core** (Remote Access in 2025.1, sound split, browse mode
  navigation, add-on updates), leaving add-ons to retire [N64], [N80], [N65], [N66].

### 3.13 Core architecture and performance

- One main thread runs the wx loop; a core pump processes queued events and function calls
  with no time budget; event, script and extension-point handlers run synchronously on it,
  with no per-handler timeout [N76], [N77].
- A watchdog notices a frozen core (timeouts of 0.5 s to 15 s) and cancels pending COM and
  window-message calls, but cannot interrupt a Python loop in an add-on [N78].
- Lesson for Okur: a screen reader whose extensions share its main thread has no answer to a
  slow extension but a watchdog and a restart with add-ons disabled. R22 and R23 exist because
  of this.

### 3.14 Add-ons as examples

| Add-on | What it uses | Source |
|---|---|---|
| NVDA Remote | became core as Remote Access in 2025.1 | [N64] |
| Windows App Essentials | app modules, overlay classes, UIA event handlers, patches to core functions; many features later moved into NVDA | [N65] |
| Speech History | one global plugin wrapping the speak function; a settings panel; a config section | [N67] |
| Clipspeak | a global plugin with Ctrl+C, X, V scripts that send the key on, then speak | [N68] |
| IBMTTS driver | a synth driver with its own 32-bit host process | [N69] |
| WhatsApp Desktop enhancements (whatsAppNG) | one app module; turns browse mode off; "optimized object filtering to reduce input lag" | [N70] |
| WhatsApp Companion | app modules and a global plugin that drives WhatsApp's page through the Chrome DevTools Protocol, with its own updater | [N71] |
| Add-on Updater | discontinued April 2025, replaced by store updates | [N66] |

## 4. Other screen readers

Short, for contrast; Okur targets Linux too, so Orca matters most here.

### 4.1 Orca (GNOME, Linux)

- **Scripts per application, chosen by the core.** Orca's script manager builds a script for
  the application in front from a chain: sleep mode, role scripts (the terminal), toolkit
  scripts, then the application's own, falling back to the default script; an alias table maps
  application names to script modules [R59]. App scripts in the tree include Thunderbird,
  Evolution, gnome-shell and LibreOffice [R59]. This is JAWS's layering, in Python, inside
  Orca's own source tree. Orca is LGPL-2.1-or-later [R91].
- **Announcements from parts.** Orca's speech generator builds each utterance from parts (name
  and label, role, states, position, context) and takes its words from message modules
  [R63]: the "announcements as data" Okur is moving to.
- **User extensions since Orca 51 (2026).** NEWS for 51.alpha (20 July 2026): "Orca now
  supports user extensions/plugins" [R60]. The documentation [R61] describes them as "early
  and experimental":
  - Python files or packages in `~/.local/share/orca/extensions/`;
  - hooks: enabled and disabled, ready, shutdown, `on_speech_output` (observe, replace or
    consume speech), `on_braille_output`, `on_region_changed` (51.2);
  - commands with stable names and default desktop and laptop bindings, double and triple
    presses included; **extensions cannot take keys Orca already uses, cannot monitor all
    input and cannot grab the keyboard**; a conflicting command is registered but unbound,
    and the dialog reports it;
  - declared settings produce a generated preferences dialog;
  - per-extension gettext catalogs, which count toward the approval hash;
  - **every extension must be approved by its SHA-256 hash**; any change needs approval
    again;
  - no store, no audit, no sandbox: "Only approve extensions you trust";
  - time: hooks run on the main thread and must return quickly; shutdown hooks get half a
    second in total; speech hooks are latency-sensitive.
- **Remote controller.** Since Orca 49 (July 2025, then "very basic, very work-in-progress and
  unstable"), a D-Bus service exposes commands and properties to external tools [R60], [R62]:
  the counterpart of the developer interface in the vision.
- **Settings.** Orca 50 made "all settings and commands … global", keeping per-application
  settings possible, and moved settings to GSettings [R60].
- **Translation:** GNOME's Damned Lies platform; the UI template has 1,878 messages, in 84
  languages [R58].

### 4.2 VoiceOver (macOS)

- No third-party plugin API was found [R69]. Automation is AppleScript: VoiceOver Utility
  has an option to use AppleScript scripts to automate VoiceOver tasks [R64], and
  commands can be assigned to keys, the numeric keypad, Option-key combinations, Quick Nav and
  trackpad gestures, including scripts in older guides [R66].
- **Activities** are named sets of settings (voices, verbosity, punctuation) that switch on
  automatically per application or web site, or by context such as word processing or source
  code [R65]: per-application and per-site profiles, without code.
- Punctuation levels All, Some, None, plus custom groups [R67]; settings export and import as
  a file, labels merged [R68].

### 4.3 Windows Narrator

- Verbosity levels 1 to 5, from text only to all text details, plus context levels [R70].
- **Narrator extensions** exist for Outlook and Excel, delivered through the Microsoft Store
  ("NarratorExtension – Excel") and listed under experimental features [R71], [R72]. No
  third-party SDK was found.
- Developer mode masks the screen except what Narrator exposes [R73]; its key differs between
  sources (unverified).

### 4.4 Others

- **TalkBack** is open source (Apache-2.0) with no plugin API [R74].
- **Dolphin SuperNova** scripts are written in Lua with Dolphin's IDE and "Mapping system"
  [R75]; whether customers write their own was unclear.
- **Narrator, VoiceOver and TalkBack** show that a screen reader can be useful without
  third-party code; **JAWS, NVDA, Orca and SuperNova** show that serious application support
  ends up as code per application, whoever writes it.

## 5. Side by side

Each cell summarizes the section named in its row; sources are there.

| | JAWS (§2) | NVDA (§3) | Orca 51 (§4.1) | Okur today (§7) | Okur proposed (§8-§10) |
|---|---|---|---|---|---|
| **Unit of extension** | script set per application or domain, plus the default set | add-on: app modules, global plugins, drivers, providers | app scripts in Orca's tree; user extensions | compiled-in `AppModule` | extension package: data plus optional code |
| **Language** | JAWS script, proprietary, compiled | Python, in process | Python, in process | Rust | data; Rust and JavaScript as Wasm components; others out of process |
| **Activation** | executable name, `ConfigNames.ini` aliases, domain | executable name, aliases, hosted apps; overlay classes per object | application name, toolkit, role | `owns(focus)`, `owns_page(url)` | app identity, URL pattern, object kind (R3) |
| **Layering** | app, then default, then built-in; same-name override and call-through | event chain with `nextHandler`; overlay class order | script chain; extensions hook output | first module to answer | app/site, global, default, with fall-through (R2) |
| **Events** | event functions written by FS, overridable | `event_*` handlers, extension points | output and region hooks | focus only | full set; observers and interceptors (R4) |
| **Changing speech** | override say functions, speak messages | filter the speech sequence, overlay properties, patches | replace or consume output | `FocusResponse::Speak` | filters on typed parts (R5) |
| **Keys** | key maps decide, then the script; app map wins | script lookup per key; user and locale gesture maps | named commands; cannot take Orca's keys | static table, closed command enum | declared bindings, decided by table on the hook (R8-R10) |
| **User remapping** | Keyboard Manager | Input Gestures | preferences | none | key map file and dialog (R10) |
| **Settings** | `.jcf` per app, `.jsi` per domain; scripts add Quick Settings items | `confspec` sections, profiles with triggers, settings panels | declared, generated dialog | five global settings | registry with scopes and lifetimes (R17) |
| **Verbosity** | levels, message categories, long and short messages, Speech and Sounds schemes | many switches; speech modes; no categories for add-ons | not researched | none | profiles over typed parts, JAWS defaults (R13) |
| **Messages and languages** | message files per language folder; many JAWS languages | gettext, Crowdin; add-on catalogs | gettext; per-extension catalogs | English literals | catalogs with ids, Fluent (R14, §10) |
| **Speech drivers** | synthesizer drivers (not researched) | `SynthDriver`, in process; speech commands | Speech Dispatcher, Spiel | closed enum | driver registry, speech commands, out of process allowed (R15) |
| **Braille** | structured mode, braille customization | display drivers, liblouis tables | liblouis, BRLTTY | none | drivers and tables later (R15) |
| **Dialogs** | Windows dialogs from scripts (not researched) | wxPython | generated preferences; custom dialogs unmanaged | `okur-ui-model`, closed set | data-described dialogs (R16) |
| **Debugging** | speak calls, Script Utility Mode, Speech History, third-party BX | Python console, log, Speech Viewer, scratchpad, reload | debug output | debug log, `okur latency`, traces, parity | developer interface, replay (R26, R27) |
| **Testing** | none documented | none for add-ons; NVDA's own system tests | not documented | parity replay, 198 core tests | recorded interactions with expected speech (R26) |
| **Versions** | per JAWS version folder; recompile | API version, yearly breaks, overrides | none yet | n/a | SDK range in the manifest (R19) |
| **Distribution** | installers, vendors, binary-only | Add-on Store, no code review, no signing | approval by hash, no store | n/a | reviewed registry, hashes (R20) |
| **Isolation** | none found | none; secure add-on runtime planned | none | n/a | Wasm sandbox; process boundary (§9) |
| **Time limits** | none found | none per handler; watchdog | shutdown 0.5 s total | 150 ms key fallback | per-handler deadlines (R22) |

## 6. Patterns to adopt, and to avoid

### 6.1 Adopt

1. **Application code activates itself.** JAWS loads script sets per executable and per
   domain; NVDA loads app modules per executable; Orca builds a script per application. Users
   never turn application support on by hand. (§2.2, §3.2, §4.1)
2. **Layers with fall-through.** JAWS's call-the-same-name and NVDA's `nextHandler` let an
   application layer add to the default instead of copying it. (§2.2, §3.4)
3. **Keys decided by table, code after.** JAWS's key maps decide; a script that wants the
   application to have the key sends it. Orca 51 forbids extensions to watch all input. It
   fits the hook timeout, which nothing else does safely. (§2.3, §4.1, §9.1)
4. **Messages apart from code, with long and short forms and categories.** JAWS's message
   files serve translation and verbosity at once. (§2.4)
5. **Settings that extensions declare and the core displays.** Scripts' Quick Settings items,
   NVDA's settings panels, Orca's generated preferences. A generated dialog is made of the
   standard controls Okur already reads, with no per-extension window code. (§2.5, §3.6,
   §4.1)
6. **Per-application and per-site profiles without code.** JAWS's `.jcf` and `.jsi`, NVDA's
   profiles with triggers, VoiceOver's Activities. (§2.5, §3.6, §4.2)
7. **Customization as data.** Custom labels, PlaceMarkers, Flexible Web rules, dictionaries:
   most user fixes need no code. (§2.6)
8. **Object-kind overrides.** NVDA's overlay classes fix one control type in one application
   cleanly. (§3.3)
9. **A safe start.** NVDA's restart with add-ons disabled and `--disable-addons`. (§3.1)
10. **Small typed hooks.** NVDA's Action, Filter and Decider are easy to reason about; a
    handler's exception is logged and skipped. (§3.4)
11. **Translator comments and catalog checks.** NVDA's rule that every translatable string has
    a translators' comment, enforced by a check. (§3.7)
12. **Approval by hash.** Orca re-asks when an extension's files change. (§4.1)
13. **A registry with hashes, channels and automated checks.** NVDA's datastore. (§3.8)

### 6.2 Avoid

1. **Extension code on the main thread with no time limit** (NVDA's core thread, Orca's main
   thread): a slow handler silences the screen reader, and a watchdog can only notice.
   (§3.13, §4.1)
2. **Monkeypatching as the real extension API.** When the hooks are too few, authors patch
   core functions, and every core change breaks them. (§3.4, §3.12)
3. **Full-privilege extensions with no review**, run "at user's own risk" (NVDA's store) or
   "Only approve extensions you trust" (Orca); NVDA's UAC hijack advisory stays open. (§3.8,
   §3.10, §4.1)
4. **One file of each type per application**: JAWS's coexistence problems between Freedom
   Scientific's and third parties' scripts. Okur needs several extensions per application by
   design. (§2.2)
5. **Yearly breaking releases without migration help**: 28% of NVDA's store add-ons
   compatible on release day of 2026.1. If Okur breaks its SDK, the replay tests of each
   extension should tell authors exactly what broke, and agents can port them. (§3.12)
6. **A language of one's own.** Thirty years on, JAWS's script language has no floating point
   and no error handling, and no debugger was found. (§2.1, §2.10)
7. **Binary-only distribution of extensions**: hard to review, learn from or port. (§2.9)
8. **Hand-made settings dialogs per extension** (wxPython in NVDA): each add-on builds and
   tests its own; Orca says custom dialogs are the extension's responsibility for
   accessibility and behavior. (§3.6, §4.1)
9. **Matching display text** (titles, localized names) to recognize applications. (§7.1,
   §10.1)
10. **Speech filters on finished text.** NVDA's filters see a sequence of strings; JAWS's
    scripts call say functions. Once announcements are parts, changing a part is cheaper and
    survives translation. (§3.7, §7.4)

## 7. Okur today: what exists, what is missing

Each subsection says what the code does now, with lines, then the gap against an extension
system like those in §2 and §3.

### 7.1 Application modules

What exists:
- `AppModule` (`crates/okur-core/src/apps/mod.rs:57-131`) is the closest thing to JAWS's
  application scripts and NVDA's app modules. It has 13 hooks, each with a default that leaves
  the engine alone: `owns`, `focus`, `names_window`, `window_name`, `reads_key`, `types_into`,
  `edit_key`, `virtualizes_text`, `viewer_lines`, `line_after_viewer`, `owns_page`,
  `font_elements`, `font_from_elements`.
- A focus hook answers `Pass`, `Silent` or `Speak(Announcement)` (`apps/mod.rs:25-32`). It
  gets an `AppContext` with mutable access to the engine's mode, the active window's title and
  kind, and whether the last key selected (`apps/mod.rs:35-43`).
- Nine modules, compiled in: eight for Windows (Widgets, task switcher, desktop, File
  Explorer, taskbar, terminal, Windows search, Windows Settings;
  `apps/windows/mod.rs:18-29`) and one web application (Google Docs; `apps/web/mod.rs:9-11`).
  The order is fixed in code (`apps/mod.rs:134-138`).
- Dispatch: every focus event is offered to every module in order before the engine's own
  handling (`engine/focus.rs:93-122`), "as JAWS runs an application's script before the
  default one" (comment at line 93). Other hooks go to the module that owns the focus
  (`engine.rs:207-212`) or, for a web application, the page's address (`engine.rs:215-221`).
- Application identity: `FocusEvent::app` is the executable on Windows and is meant to be the
  bundle identifier on macOS (`event.rs:187-189`). Raw window and element classes live in
  `FocusEvent::platform` (`event.rs:237-252`) for modules only. Structure step 2 removed every
  Win32 class and process name from general code: a search of `okur-core` outside `apps/`
  finds them only in two doc comments (`event.rs:188`, `event.rs:244`).

Gaps:
- **No command hook.** The structure plan listed "command handling" among the hooks
  (plan step 3), but the trait has none. A module cannot add a key or a command, and cannot
  change what an existing command does in its application.
- **Only focus is an event.** Value, name and state changes (`AccessibilityEvent::Change`),
  selection, live regions, page changes, documents loading and menus reach only the engine
  (`engine.rs:566-605`; `engine/page.rs:287`, `342`, `491`). JAWS and NVDA both give
  application code these events (§2.2, §3.4).
- **No application lifecycle.** Nothing tells a module that its application gained or lost the
  foreground, which JAWS and NVDA both expose (§2.2, §3.2).
- **Modes are the engine's enum.** Three of the five `AppMode` values are application modes:
  `NativeDesktop`, `NativeShell`, `NativeExplorer` (`engine/state.rs:182-189`), set by the
  desktop, taskbar and Explorer modules and read by the engine
  (`engine/commands.rs:110`). An extension could not add a mode.
- **No settings or messages of its own.** Modules hold state in their own structs (File
  Explorer's last item, `apps/windows/explorer.rs:26-33`), but there is no per-module
  setting, and their words are English literals (`explorer.rs:138-151`).
- **Matching on localized names.** `is_explorer_window` falls back to UI names and titles:
  "Items View", "Shell Folder View", a title containing "File Explorer"
  (`explorer.rs:12-23`). These are words of the Windows display language. Whether they hold on
  another UI language was not checked; JAWS's application scripts face the same problem, and
  the vision's "identified by what they are" applies here.
- **Registration is code.** A new module is a Rust change and a rebuild.

### 7.2 Commands and keys

What exists:
- `Command`, 69 variants (`crates/okur-core/src/command.rs:39-139`), 20 quick-navigation
  element kinds (`command.rs:12-36`). Named after JAWS's script names where JAWS has one.
- `Command::from_name` (`command.rs:143-294`) is the one place command names are strings: it
  accepts JAWS script names and JAWS's key notation ("{DOWN}", "^%{RIGHT}"). This is a ready
  base for a key map file in JAWS's own vocabulary.
- One key map as data: 106 desktop bindings (`command.rs:404-561`) and 7 laptop bindings
  (`command.rs:566-578`), first match wins. Double presses are a table too
  (`command.rs:298-305`).
- Key routing is one ordered state machine (`engine.rs:232-480`), documented in
  [`forms-and-interaction-modes.md`](forms-and-interaction-modes.md) ("Who takes a key"):
  Pass Key Through, the JAWS key, the Virtual Viewer, menu mode, forms mode, Escape, the PC
  cursor, the virtual PC cursor.

Gaps:
- **The command set is closed.** An extension command needs a variant of a Rust enum. An
  open set needs commands with ids (like JAWS script names or NVDA's script names) next to
  the built-in enum.
- **The key map is a `static` array.** There is no user key map, no per-application or
  per-site map, no unbinding, and no Keyboard Manager equivalent. JAWS layers key maps per
  application and NVDA has user gesture maps (§2.3, §3.5).
- **Whether a key is consumed depends on running the command.** In the virtual cursor the
  engine runs the command and consumes the key if it produced speech, or if the key is a
  navigation key (`engine.rs:455-477`). The vision's principle 2 needs the opposite order:
  decide from tables and state, then run. This is the most important refactor for extension
  keys (§11, phase 0.4).
- **Other key tables** live outside the key map: the edit keys (`editing.rs:64-90`), the
  Virtual Viewer's keys (`viewer.rs`), Tab handling (`engine.rs:444-452`). Layers
  (Insert+Space) are planned, not built
  ([`docs/plans/2026-10-17-helper-dialogs.md`](../plans/2026-10-17-helper-dialogs.md) step 10).
- **Keyboard layouts.** Bindings are by key identity (`Key::Alpha('H')`,
  `command.rs:580-592`). On the Turkish Q layout the key at the US `;` position types "ş", and
  JAWS answered with the onMouseOver message there
  ([`jaws-virtual-buffer-spec.md`](jaws-virtual-buffer-spec.md) item 18). Extension key maps
  inherit whatever rule Okur settles on for physical versus typed keys.

### 7.3 Threads and time budgets

What exists:
- The whole engine runs inside the low-level keyboard hook's callback, under one mutex
  (`crates/okur/src/main.rs:853-902`; the hook procedure calls the callback directly,
  `crates/okur-platform/src/hook/windows.rs:230`). A key that waits 150 ms for the engine
  passes to the application unread (`main.rs:858-867`). Engine work over 20 ms is logged as
  slow (`main.rs:604`).
- Accessibility events are handled on a tokio task under the same mutex
  (`main.rs:914-977`). Page buffers are built before the lock is taken, because the hook waits
  on it (`main.rs:923-936`).
- `okur latency` reads per-key `waited_us` and `held_us` from a debug log
  (`crates/okur-corpus/src/latency.rs:1-13`). This is the tool the vision names for handler
  budgets.
- Speech is stopped on every key (`engine.rs:272`) and spoken synchronously from the engine
  (61 `speak` call sites in `engine.rs` and `engine/*.rs`).

Gaps:
- Any extension code called from `handle_key_event` today would run on the hook thread.
  Principle 2 needs a split: the hook thread decides consume or pass from tables and a small
  state snapshot; command execution, speech and extension handlers run on an engine thread.
- There is no per-handler timing, no skipping of slow handlers, and no kill switch.

### 7.4 Announcements and Okur's own words

What exists:
- `Announcement` (`crates/okur-core/src/announcement.rs:13-89`): lines of parts; a part is
  `Text`, `Role` or `State`; a line may be said only on arrival (`on_arrival`, the hints that
  Say Current Focus leaves out). It renders to the text parity compares, with JAWS's role
  marks `\u{1d}` … `\u{1e}` (`linearize.rs:23-29`).
- Native focus speech is built with it (64 lines of `native_focus.rs` use it), and so are
  application modules and the window heading (structure step 4, commits `8cf7ffd`,
  `a7ddbf8`).

Gaps:
- **Parts carry English words.** `Part::Role(String)` holds "check box", "Button", "List
  view": rendered text, not a role. A verbosity setting or a translation cannot act on it
  without parsing words back.
- **Part kinds are too few** for verbosity: there is no name, value, position, description,
  hint, shortcut, container or message kind. The structure plan listed position and hint
  (plan step 4); `on_arrival` stands in for hints today.
- **Most speech is still strings.** `SpeechFormatter` (`speech.rs`) writes role marks by hand;
  the virtual buffer's lines store their speech as a finished string (`BufferLine::speech`,
  `buffer.rs:63-72`). Across non-test `okur-core` code, 73 lines write role marks by hand.
  Structure step 4 says what is left: `SpeechFormatter` module by module, then a structured
  page line.
- **Okur's own words are literals.** In non-test `okur-core` code at `9ccf0ba`, there are 553
  distinct string literals containing a word, leaving out `command.rs` (command names) and
  attribute lookups. 419 have spaces or line breaks (phrases and templates); 134 are single
  words. By a manual pass, about 40 of the single words are identifiers (class and process
  names, ARIA attribute and setting keys); the rest are what Okur says or shows: role and
  state words, hints, messages, symbol names, month and day names, dialog labels. Method:
  `git show 9ccf0ba:<file>` for every `okur-core/src` file except test files, cut at
  `mod tests {`, comment and `tracing` lines dropped, literals extracted with a regular
  expression. The count is approximate; the files with the most are `speech.rs` (135 lines with
  literals), `cursor.rs` (108), `linearize.rs` (98), `native_focus.rs` (94),
  `pronounce.rs` (52, symbol names), `ui/settings.rs` (48).
- Examples: 38 lines of "There are no … on this page." messages; 42 lines of hints ("To
  activate press Enter."); "Not in a table." written out 35 times in `cursor.rs` (first at
  `cursor.rs:1785`); "Enter key to pass through" (`engine/commands.rs:69`); "virtual PC
  Cursor" (`engine.rs:365`, `engine/commands.rs:135`); "Caps lock on/off"
  (`engine/commands.rs:571-575`).
- **JAWS's wording depends on the context, and Okur copies it.** A button is "Button" in
  focus speech and the Virtual Viewer (`native_focus.rs:556-569`) but "button" in Say Window
  (`native_focus.rs:486-528`); "Combo box" and "combo box" likewise. The frames message has no
  final period where the others have one (`cursor.rs:970`). A message catalog must key words
  by context and keep each message exact, never compose them.
- **Some formats are already the system's.** Say System Time takes month and day names,
  patterns and AM/PM from the Windows locale (`crates/okur-platform/src/clock.rs:44-68`); the
  English table in `clock.rs:107` is the fallback for tests. "Week {}" is still English.
- **Case folding is ASCII.** Finding a heading, link or field by text compares with
  `eq_ignore_ascii_case` (`engine/commands.rs:155`, `164`, `173`), which does not fold
  Turkish "İ" and "ı" or any other non-ASCII letter.
- **Tests pin English.** `okur-core` has 198 tests; about 340 string literals in its sources,
  tests included, contain words such as "Button", "heading level", "Link", "dialog" or "To
  activate" (same extraction, rough).

### 7.5 Settings

What exists:
- `Settings` (`crates/okur-core/src/settings.rs:175-183`): voice, keyboard (layout and JAWS
  key), live region append mode, typing echo, viewer display. One global set, saved as JSON at
  `%APPDATA%\Okur\settings.json` and the macOS and XDG equivalents
  (`crates/okur-platform/src/settings_file.rs:13-28`), through the `SettingsStore` trait
  (`settings.rs:186-188`).
- The reference machine's JAWS settings are a constructor (`settings.rs:218-226`), and parity
  replay reads each evidence file's recorded `DEFAULT.JCF` options
  (`crates/okur-corpus/src/parity.rs:568-588`). This is the precedent for verbosity: replay
  must run with the settings JAWS ran with.
- The settings dialog (Insert+6, Insert+V) is built by hand, control by control, with English
  label tables (`crates/okur-core/src/ui/settings.rs:14-36`).

Gaps:
- No scopes (application, site, document) or lifetimes (saved, until exit, until focus moves);
  no registry an extension could add to. Both are decided
  ([`helper-ui-architecture.md`](helper-ui-architecture.md) §4 "Settings") and planned (helper
  dialogs plan step 9), not built.
- No verbosity, punctuation level, speech language or Okur language setting.
- In progress (working tree, 2026-10-18, not at `9ccf0ba`): another session is adding
  `auto_forms_mode` and `access_keys` to `Settings`, defaulting to JAWS's defaults (on) and
  off in `Settings::reference_machine()`, mirroring old-pc's `AutoFormsMode=0` and
  `ACCESS_KEY=0|0|0`. `access_keys` is one JAWS output type as a flat switch; the verbosity
  design (proposal §6.5) absorbs it.

### 7.6 Okur's own dialogs

What exists:
- Dialogs are data: `DialogSpec { id, title, controls }`
  (`crates/okur-ui-model/src/lib.rs:23-28`), controls of kinds `Group`, `CheckBox`, `Choice`,
  `Slider`, `Button`, `List`, `Radios`, `Text` (`lib.rs:48-85`), messages `ToUi` and `FromUi`
  (`lib.rs:100-150`). `okur-ui` draws them natively and Okur reads them through the
  accessibility API like any application's
  ([`helper-ui-architecture.md`](helper-ui-architecture.md) §3). The architecture page names
  extensions as a reason for this design (requirement 13).
- The transport is the child process's standard input and output, one JSON message per line
  (`lib.rs:1-6`; `crates/okur-platform/src/ui_host.rs:1-5`). The architecture page says named
  pipe or Unix socket; the code uses stdio, which is simpler and serves the same purpose.

Gaps:
- **No editable text, tree or tabs** among the controls, though the architecture page lists
  them (§3). An extension's settings or find dialog needs an edit field.
- **Dialogs are engine fields.** The engine holds `settings_dialog` and `links_dialog`
  (`engine.rs:117-120`) and routes `FromUi` messages to them by id (`engine.rs:670-715`,
  `825-833`). An extension's dialog needs a controller trait and a table of open dialogs.
- The renderer exists on Windows only (`crates/okur-ui/src/main.rs:8-24`).

### 7.7 Speech engines and braille

What exists:
- `TtsEngine` (`crates/okur-platform/src/tts/mod.rs:32-80`): speak text, stop, pause,
  resume, rate, volume, voices, a pitch change for capitals (`mod.rs:41-46`). Drivers:
  OneCore, SAPI 5, no speech, chosen by a closed enum (`mod.rs:83-89`; `settings.rs:136-147`)
  in a factory `match` (`mod.rs:141-180`).
- The engine sees `SpeechOutput` (`crates/okur-core/src/speech.rs:769-806`): a string with role
  marks, an interrupt flag, a character channel and an echo channel.

Gaps:
- Adding a synthesizer touches three closed enums and a factory: no driver registry.
- Text in, nothing structured: no speech commands (voice or pitch per part, as JAWS speaks
  roles and states in their own voice), no language per utterance, no index marks or
  completion callbacks (needed for Say All to move the cursor with the speech), no
  priorities beyond interrupt or queue.
- No macOS or Linux synthesizer.
- No sounds: nothing plays a sound file or a tone. OneCore's own PCM playback
  (`tts/onecore.rs:390`) is internal to that driver. JAWS's Speech and Sounds Manager can
  replace words with sounds (§2.5), so verbosity needs a sound output.
- **No braille at all.** A search of the crates finds the word only in comments.

### 7.8 Objects an extension would read

What exists:
- The page is an `AomNode` tree (`crates/okur-core/src/aom.rs:196-215`) with a platform-neutral
  `Role` of 35 values (`aom.rs:6-53`) and string attributes.
- A focus is a `FocusEvent`, a flat snapshot of 39 fields (`event.rs:103-192`), read once by
  the platform layer.
- Reads beyond the snapshot are asynchronous: `ApplicationActions` asks, and the answer comes
  back later as an event (`crates/okur-core/src/actions.rs:11-90`; for example
  `read_page_elements`, used by the Google Docs module for Say Font). Nothing blocks the
  engine on a cross-process call.
- `okur-core` depends on neither the platform crate nor xa11y (`crates/okur-core/Cargo.toml`).
  The vendored xa11y has platform-neutral `ElementData` and selectors, and its planned
  extensions include the live event pipeline (`crates/xa11y/VENDORED.md`, "Planned
  extensions").

Gaps:
- No live object handle: an extension could not ask for a parent, a sibling or a property
  the snapshot left out. NVDA app modules walk live objects synchronously (§3.4); Okur's
  asynchronous request model is safer for latency and should be kept, with a richer query.
- macOS input and events are stubs of about 45 lines each
  (`crates/okur-platform/src/hook/macos.rs`, `events/macos.rs`).

### 7.9 Verification harness

What exists:
- `okur parity` drives the engine's real key and event entry points with recorded keys and
  traced events and compares normalized speech with JAWS's transcripts
  (`crates/okur-corpus/src/parity.rs:1-8`, `592-604`); results are ratcheted in
  `corpus/parity-baseline.json`. The last recorded figures: IA2 819/844 steps, UIA 720/844
  ([`docs/handoff/2026-10-17-whatsapp-web.md`](../handoff/2026-10-17-whatsapp-web.md),
  "Verification").
- Traces are Windows event streams (MSAA and UIA, format version 7,
  `crates/okur-platform/src/event_trace.rs:38`) converted to `FocusEvent`s in okur-platform.
  `AccessibilityEvent` itself is serializable (`event.rs:393-394`).
- Evidence records the JAWS version, keyboard layout, culture and UTC offset
  (`corpus/evidence/308-desktop-time-and-date-evidence.json`, lines 11-16). The newer
  recordings, 8 of the 65 evidence files at `9ccf0ba`, also record JAWS's user settings files:
  `DEFAULT.JCF`, `Chrome.JCF` for Chrome scenarios, and every voice profile
  (`skills/jaws-reference/scripts/run-scenario.ps1:225-245`; §2.5).

Gaps:
- No replay format that is not Windows'. An extension test written on one platform should
  replay on all: recording the engine's own input (serialized `KeyEvent`s and
  `AccessibilityEvent`s) next to the platform trace would give that.
- No replay with extensions on, and no expected-speech file for an extension.
- Evidence does not record the Windows display language, and JAWS's language only implicitly
  (the settings are read from the `enu` folder); both decide the words in a transcript (§10).
  Application JCFs are recorded for Chrome only, and replay reads only `KeyboardType` and
  `TypingEcho` from them (`okur-corpus/src/parity.rs:568-588`).
- No developer interface into a running Okur (the read-only MCP server of the vision).

### 7.10 Summary table

| Capability | JAWS / NVDA have | Okur has | Where |
|---|---|---|---|
| Behavior per application | app scripts / app modules | yes, compiled in, focus-only | `apps/mod.rs:57-131` |
| Behavior per web site | domain scripts and settings / no site scope was found in what was read | address match, one module | `apps/mod.rs:113-117`, `apps/web/` |
| Global extensions | changes to the default script set / global plugins | no | |
| Events beyond focus | yes / yes | no (engine only) | `engine.rs:566-605` |
| New commands and keys | scripts and key maps / scripts and gestures | no (closed enum, static map) | `command.rs:39-139`, `404-561` |
| User key remapping | Keyboard Manager / Input Gestures | no | |
| Changed announcements | scripts / overlay classes, speech filters | parts exist, English words | `announcement.rs` |
| Settings per app and site | yes / config profiles | no | `settings.rs:175-183` |
| Verbosity levels, output categories | yes / partial | no | |
| Punctuation and symbols | yes / yes | names for single characters only | `pronounce.rs:73` |
| Translated messages | per-language folders / gettext | no | §7.4 |
| Dialogs for extensions | Windows dialogs from scripts / wxPython | data-described dialogs, closed set | `okur-ui-model` |
| More speech engines | synthesizer drivers / synth drivers | closed enum | `tts/mod.rs:83-89` |
| Braille drivers | yes / yes | none | |
| Packaging, versioning, store | installers, per-version folders / add-on store | none | |
| Isolation and budgets | none / none (main thread, watchdog) | 150 ms key fallback only | `main.rs:858-867` |
| Proof by replay | none / none | parity replay, extensions not included | `okur-corpus` |

The JAWS and NVDA columns summarize §2 and §3, where each entry is sourced.

## 8. Requirements for Okur's extension system

Numbered so that plans and tests can cite them. "Must" items come from the vision, from
AGENTS.md, or from a failure JAWS or NVDA users report (§2.10, §3.12). Each says how it would
be verified.

### 8.1 The contract

- **R1. One contract in Rust, used by Okur first.** The nine built-in modules move onto it
  before any outside extension exists (vision principle 1). *Verified by:* `apps/` compiles
  only against the contract; parity output unchanged byte for byte
  (`skills/jaws-reference/scripts/parity-snapshot.sh` before and after).
- **R2. Explicit precedence and a way to fall through.** Order: an application or site
  extension, then global extensions, then Okur's default behavior. A handler can do the
  default and then add to it, as a JAWS application script calls the default script's version
  and an NVDA handler calls `nextHandler` (§2.2, §3.4). Two extensions for one application are
  ordered deterministically (load order recorded, shown to the user). *Verified by:* unit
  tests of the dispatcher with mock extensions.
- **R3. Activation by identity, never by a machine's names.** An extension declares what it is
  for:
  - applications by platform-neutral identity (`FocusEvent::app`: executable on Windows,
    bundle identifier on macOS, desktop file or process name on Linux);
  - web applications by address patterns (scheme, host, path prefix), as `owns_page` does
    now, and JAWS's domain settings do (§2.6);
  - kinds of objects (role, states, properties), as NVDA's overlay classes choose by the
    object (§3.3).
  Raw platform identifiers stay reachable in a `platform` namespace, marked non-portable
  (AGENTS.md §3, integrity rule 7: no fixture-shaped special cases).
- **R4. Events.** Focus; name, value and state changes; selection; text and caret changes;
  live regions and application notifications; page loaded, changed and part reloaded; windows
  opened and closed; menus; application gained and lost; idle. Two kinds of handler:
  - *observers*, asynchronous, never delaying speech;
  - *interceptors*, synchronous, with a hard deadline (R22), able to replace or suppress what
    Okur would say.
- **R5. Announcement filters on parts.** An extension changes parts of an `Announcement`
  (drop the role, replace the name, add a line) before it is rendered to words, for focus
  speech, virtual buffer lines, live regions and messages alike (vision principle 3). Needs
  §11 phase 0.2 (typed parts) and the structured page line (structure step 4).
- **R6. Objects, not handles.** Extensions read Okur's objects (`AomNode`, focus snapshots,
  the virtual buffer), and ask for more through asynchronous queries answered as events, the
  `ApplicationActions` model (§7.8). No raw UIA, IA2, AX or AT-SPI object crosses the
  contract (vision principle 4).
- **R7. Actions.** Focus an object, activate it, press keys in the application, move the
  virtual cursor, speak (with priority and interrupt choice), show a dialog, play a sound,
  copy text. Each is a permission (R18).

### 8.2 Keys, commands and navigation

- **R8. Commands with ids, bindings as data.** An extension declares commands (id, label,
  description, category for key help) and default bindings per layout (desktop, laptop),
  scope (application, site, mode) and cursor (virtual, PC, forms mode). Built-in commands get
  ids too: JAWS's script names, which `Command::from_name` already accepts.
- **R9. The hook decides from a table.** The keyboard hook decides consume or pass from the
  merged tables and a small state snapshot (mode, scope, open layers), in constant time, with
  no extension code and no cross-process call (vision principle 2). The command then runs off
  the hook thread. *Verified by:* `okur latency` showing `held_us` on the hook independent of
  extension handlers; a test that a sleeping handler never delays the key decision.
- **R10. User key maps.** Users remap, unbind and add keys per scope, in a file and in a
  Keyboard Manager style dialog; conflicts are reported. JAWS's key notation is the file's
  vocabulary, so a JAWS user's habits carry over. Key help (Insert+H, Insert+W in JAWS) lists
  extension commands for the application in front.
- **R11. New quick-navigation kinds.** An extension can define an element kind by a
  predicate over the buffer (a chat's messages, a mail list's rows) and bind next and prior
  keys to it; Okur supplies the wrapping and "There are no …" behavior and the lists dialog.
- **R12. Layers.** Extensions can add layered keys (JAWS's Insert+Space layers) once layers
  exist as a key-routing step (helper dialogs plan step 10).

### 8.3 Output: speech, verbosity, braille, language

- **R13. Verbosity as data, JAWS's defaults.** Okur models JAWS's verbosity (§2.5): levels,
  per-category announcement switches, the Speech and Sounds Manager's choices of speech,
  sound or voice change per role and state, punctuation levels. Every `Part` has a kind the
  verbosity profile acts on. Okur's default profile is JAWS's default; replay runs with the
  profile recorded in the evidence. Extensions declare which categories their messages
  belong to and get them filtered like Okur's own.
- **R14. Translated messages.** Every word Okur says comes from a message catalog with stable
  ids, per language, with plural and context support (§10). The English catalog is JAWS's
  English, verified by parity. Extensions ship catalogs in the same format. Okur's language is
  a setting, independent of the operating system's display language, as JAWS's language is
  (the reference machine runs English JAWS on a Turkish Windows display language,
  [`verification-loop.md`](verification-loop.md) §3).
- **R15. Speech and braille drivers.** A synthesizer is a driver behind a registry, not an
  enum: voices with languages, rate, pitch, volume, speech commands (voice change, pitch,
  pause, language), index marks and completion callbacks. Braille display drivers and
  translation tables follow the same pattern. Drivers may live out of process (licenses,
  crashes; §9.4). *Verified by:* the no-speech driver and a recording driver in
  `cargo test`; live latency with `okur speak --measure-latency`.

### 8.4 Dialogs and settings

- **R16. Extension dialogs through okur-ui.** An extension describes a dialog with
  `okur-ui-model` and handles its events through a controller interface; Okur routes events by
  dialog id. Needed first: edit, tree and tabs controls (§7.6).
- **R17. Settings registry with scopes.** An extension declares settings (id, label,
  description, type, default, allowed scopes: all applications, one application, one site,
  one document; lifetimes: saved, until exit, until the focus moves). Values are layered;
  Quick Settings and Settings Center list them automatically, under the extension's name, as
  JAWS lists script-provided options (§2.5) and NVDA lists add-on settings panels (§3.6).
  Per-application and per-site settings are profiles in NVDA's sense (§3.6).

### 8.5 Packaging, versions, distribution, security

- **R18. Permissions.** An extension declares what it needs: which applications or sites,
  reading text fields (and never protected fields), the clipboard, sending keys, network,
  files, starting programs, showing dialogs. The runtime enforces them (§9). The vision's
  privacy principle applies: a screen reader sees passwords and private messages.
- **R19. Manifest and versions.** A manifest names the extension, its version, its author,
  its license, the SDK version range it supports, the platforms it supports and its
  permissions. Okur refuses an extension outside its SDK range, and says why. NVDA's
  `minimumNVDAVersion` and `lastTestedNVDAVersion` and the yearly breaks that come with them
  are the model and the warning (§3.9).
- **R20. Distribution.** A registry of manifests (a repository, as NVDA's add-on datastore,
  §3.8) with reviewed versions, hashes, source links and licenses. Local installs for
  development. Agent-written extensions arrive as reviewed changes with their recorded
  interactions and replay results (vision, steps 4 and 5).
- **R21. Clean room for ports.** Freedom Scientific's scripts are never read or translated.
  Third-party scripts are ported only where their license allows, and AGENTS.md §1.4
  currently forbids reading any script source, so porting needs the owner's decision first
  (§12, question 7).

### 8.6 Reliability, performance, platforms

- **R22. Budgets.** Every handler call has a deadline, measured and logged as `okur latency`
  measures keys today. An interceptor that misses its deadline is treated as having passed;
  repeated misses disable the extension for the session and say so once. Proposed starting
  budgets, to be settled by the spike in §9.6: key decision 0 µs of extension time (tables
  only); focus interceptor 2 ms; observer 50 ms, never on the speech path.
- **R23. Kill switch and safe mode.** One chord turns every extension off at once (vision
  principle 5). Okur can start with extensions off (NVDA's "restart with add-ons disabled",
  §3.1). An extension that crashes or loops is disabled and named, never taking speech with
  it.
- **R24. Zero cost when idle.** An application with no extension pays nothing: dispatch looks
  up a table by application identity.
- **R25. Cross-platform by default.** The same extension runs on Windows, macOS and Linux,
  unless it declares otherwise. Its tests replay on every platform (R26).
- **R26. Proof by replay.** An extension's test is a recorded interaction (keys and
  accessibility events, in a platform-neutral form) plus the speech it should produce. A
  runner replays it hermetically, with the extension on. Parity with JAWS stays measured with
  extensions off (vision principle 6). *Verified by:* the runner in CI on all three
  platforms.
- **R27. Agent development loop.** A developer interface into a running Okur, off by default
  and local only: state, speech history, events, the Virtual Viewer, traces, and reloading an
  extension (vision, pieces 4 and 5). Read-only first.

## 9. Candidate designs for the extension runtime

### 9.1 What the runtime must respect

- **The hook.** Microsoft's documentation of the low-level keyboard hook: the procedure must
  return within `LowLevelHooksTimeout`; "on Windows 7 and later, the hook is silently removed
  without being called", and "There is no way for the application to know"; since Windows 10
  1709 the timeout is capped at 1000 ms; and Microsoft advises running the hook "on a
  dedicated thread that passes the work off to a worker thread and then immediately returns"
  [R1]. Microsoft does not document the default value; a third-party library says Windows
  uses 300 ms when it is missing [R95], a forum answer says 5000 ms (unverified either way).
  macOS disables an event tap whose callback is too slow and sends the tap a "disabled by
  timeout" event, which the tap must answer by enabling itself again [R2], [R3]; the threshold
  is not documented in what was found. So: **no extension code on the hook path, on any
  platform** (R9). Orca 51 likewise gives its extensions no access to raw input [R61].
- **The focus path.** A focus interceptor sits between the event and speech. Its budget has
  to be a few milliseconds at most (R22), every time, on a loaded machine.
- **Failure.** A crash, a hang, an out-of-memory or an infinite loop in an extension must not
  stop speech (vision principle 5).
- **Platforms.** One extension artifact should run on Windows, macOS and Linux (R25).
- **Authors.** Agents and people who have never written a screen reader script (vision).
- **Permissions** enforced, not just declared (R18).

### 9.2 The options

| | Option | Isolation | CPU and memory limits | Cross-platform artifact | Languages | Latency | Maturity |
|---|---|---|---|---|---|---|---|
| A | **Rust compiled into Okur** (today's `apps/`) | none | none | rebuild per platform | Rust | lowest | ready |
| B | **Native dynamic libraries** (C ABI, `abi_stable`) | none: a segfault or abort ends Okur [R39] | none | one build per platform | Rust, C | lowest | `abi_stable` has no unloading [R39] |
| C | **WebAssembly components in Wasmtime** | memory-safe sandbox; imports are capabilities | epochs or fuel [R9], [R10]; `ResourceLimiter` [R11] | one `.wasm` | Rust (stable), JS (experimental), Python, C# (pre-release), C/C++, Go (TinyGo or go-modules) [R7], [R8] | instance in µs with pooling [R12], [R13]; call overhead tens of ns (core Wasm, 2021) [R87] | Wasmtime monthly releases, LTS every 12th [R5]; WASI 0.3 since June 2026 [R6] |
| D | **Embedded JavaScript** (QuickJS via rquickjs; Boa) | interpreter in Okur's address space | QuickJS: interrupt handler, memory limit [R30]; Boa: loop and recursion limits, no time budget [R28] | one script | JavaScript | QuickJS: a full runtime lifecycle under 300 µs [R29] | QuickJS mature; Boa about 94% of Test262 at 0.21 [R27] |
| E | **Embedded Lua (Luau) or Rhai** | interpreter in-process; Luau has a sandbox mode [R36]; Rhai never panics the host [R37] | Luau interrupts, Rhai operation counts [R36], [R37] | one script | Luau, Rhai | low | mlua warns of unsafe code and bugs [R35] |
| F | **Embedded CPython** (PyO3) | none; "each past attempt" at a Python sandbox "has failed" [R33] | none reliable | needs a Python per platform | Python | GIL or free-threaded 3.14 [R34] | NVDA's model (§3.10) |
| G | **Out-of-process extension host** (any language, JSON or MessagePack over a pipe) | process boundary; OS sandboxes possible (§9.4) | the OS's (job objects, kill) [R44] | depends on the host's language | any | an IPC round trip, about 4-8 µs for local sockets and FIFOs on Linux in an old benchmark [R42]; unmeasured on Windows and macOS | VS Code's extension host model [R40] |

Notes on the table:
- **C, Wasm components.** Epoch interruption costs "around a 10% slowdown"; fuel is
  deterministic but slower [R9]; a deadline can trap or call back into the host [R10]. Memory
  growth is decided by a host callback; Wasmtime's own bookkeeping is not counted [R11].
  Instantiation is fast with the pooling allocator, copy-on-write images and `InstancePre`
  [R12]; the Bytecode Alliance reported SpiderMonkey's instantiation going from about 2 ms to
  5 µs [R13]. The C API library is 19 MB in a release build with default features (which
  include the compiler) and 2.1 MB without the compiler, when extensions are compiled ahead of
  time; Linux x86-64, figures of December 2024 [R14]. JavaScript guests embed
  an engine of about 8 MB and ComponentizeJS calls itself experimental [R15]; Python guests
  are larger [R16]. Rust components are small. Zed runs its extensions as Wasm components in
  Wasmtime. Its team first went "in a JavaScript direction", then found it wanted to write
  extensions in Rust [R17]; for language grammars it wanted to "make it impossible for an
  extension to crash Zed" and could not have prevented crashes by loading shared libraries
  [R18]. Zed keeps its interface in versioned folders (`wit/since_v0.2.0`), records in each
  extension the API version it was compiled against, and instantiates it against that version
  [R17]: a way to keep older extensions working after the interface changes. A Zed discussion
  notes it never enabled epoch interruption, so a looping extension can burn CPU [R19]. Shopify Functions caps guests by
  instruction count and binary size, and recommends Rust over JavaScript for cost [R21].
- **D, JavaScript.** QuickJS is about 367 KiB of code for hello world [R29]; rquickjs exposes
  memory and stack limits and an interrupt handler that raises an uncatchable exception, and
  calls its MSVC targets experimental [R30]. V8 is excluded: an isolate that exhausts its
  heap limit crashes the whole process [R31]. Figma moved its plugins to QuickJS compiled to
  Wasm after its JavaScript realm sandbox proved insecure [R22], [R23]: an interpreter inside
  Wasm combines D's language with C's isolation.
- **F, Python.** RustPython says it is "not totally production-ready" [R32]. CPython can't be
  sandboxed [R33]. NV Access states that add-ons can "run any code, even with administrator
  privileges" and that it "does not review the code" (§3.10).
- **G, out of process.** VS Code runs extensions in an extension host so they cannot slow
  its UI or startup [R40]; Neovim's remote plugins are RPC coprocesses with synchronous and
  fire-and-forget calls [R41]. A sandboxed extension process cannot read the accessibility
  tree itself on macOS (App Sandbox conflicts with the accessibility API [R46]) and probably not
  from an AppContainer on Windows (unverified), so Okur would serve it the objects, which R6
  asks for anyway.

### 9.3 Trade-offs against the vision's principles

| Principle | A | B | C | D | E | F | G |
|---|---|---|---|---|---|---|---|
| 1. One Rust contract, adapters around it | is the contract | C ABI layer | WIT generated from or checked against it | bindings | bindings | bindings | a schema (JSON) |
| 2. Keys by table | yes | yes | yes | yes | yes | yes | yes (declared at load) |
| 3. Announcements as data | yes | yes | records in WIT | JS objects | tables | objects | JSON |
| 4. Platform-neutral objects | yes | yes | yes | yes | yes | yes | yes |
| 5. Budgets, skip slow handlers | no | no | **yes** (epochs) | yes (interrupt) | yes | no | yes (timeout, then kill) |
| 5. Crash cannot silence Okur | no | no | **yes** (trap) | mostly (bugs in C code remain) | mostly | no | **yes** |
| 6. Replay testing | yes | yes | yes | yes | yes | yes | yes |
| 7. Privacy, declared permissions | no | no | **yes** (imports) | partly (host functions) | partly | no | yes (OS sandbox, served data) |
| Agents and newcomers can write it | no (rebuild Okur) | no | Rust yes; JS experimental | **yes** | Lua yes | **yes** | any |
| Size | 0 | 0 | ~2-19 MB runtime | ~0.4 MB | small | 30-60 MB [R34] | per host |

### 9.4 Platform sandboxes, for option G and for drivers

- Windows: AppContainer isolates files, registry, network, processes and windows, though
  "keyboard and mouse … are always available" [R43]; job objects cap CPU and memory and kill
  the process tree when the handle closes [R44].
- macOS: XPC services each have their own sandbox and entitlements and are started on demand
  and restarted after a crash by launchd [R94]; App Sandbox and the accessibility API do not
  go together [R46], so a sandboxed extension process would get its objects from Okur.
- Linux: Landlock lets an unprivileged process restrict its own file and network access
  [R45].
- Drivers under GPL (eSpeak NG is GPL-3.0-or-later [R78]) can stay separate programs talking
  over a pipe; the FSF treats pipes and sockets as communication between separate programs
  [R84]; Apache-2.0 code is incompatible with GPLv2 [R83]. liblouis, BRLTTY's BrlAPI and
  Speech Dispatcher's client library are LGPL [R76], [R77], [R79]. This is not legal advice.

### 9.5 Message catalogs (for §10)

| | Fluent (`fluent-rs`) | MessageFormat 2 | gettext | Rust-native tables (`rust-i18n`) |
|---|---|---|---|---|
| Status | Mozilla's; Rust crates maintained [R47] | Unicode standard, stable in CLDR 47 (2025-03-13) [R53]; Rust crates young [R54] | GNU; libintl LGPL | YAML/JSON/TOML in the binary |
| Plurals, selectors | CLDR categories, ordinals, string selectors [R49] | yes | `ngettext` (plural forms per catalog) | not documented |
| Context per message | ids are free-form; terms for shared words [R49] | ids free-form | `pgettext` | keys free-form |
| Speech caveat | inserts Unicode isolation marks by default; turn off for speech and braille [R48] | n/a | n/a | n/a |
| Translator tools | Pontoon (self-hostable) [R50]; Weblate reads Fluent without plural support [R85] | few yet | every tool | generic |
| Used by | Firefox | ICU (preview) | NVDA (Crowdin) [R57], Orca [R58] | |

Locale data (plural rules, lists, numbers, collation, segmentation) comes from ICU4X, which
reached 2.0 in May 2025 and follows CLDR 48 since 2.1 [R51], [R52]. CLDR's plural rules: English
cardinals `one`/`other`, ordinals `one`/`two`/`few`/`other`; Turkish cardinals `one`/`other`,
ordinals `other` only [R55].

### 9.6 Recommendation

1. **Keep A as the contract's first and permanent user.** The nine modules move onto the
   contract (R1); Okur's own application support stays Rust, compiled in, as JAWS ships its
   application scripts with JAWS.
2. **Make a declarative layer first** (no code): manifests with application and site
   matchers, key bindings in JAWS's notation, settings schemas, message catalogs, custom labels,
   verbosity overrides, and simple announcement rules (match a role and name pattern, drop or
   replace a part). Many user customizations in JAWS need no script (§2.6), and Orca's
   generated preferences show how far declarations go [R61]. Data is safe, portable and easy
   for agents to write and review.
3. **Choose C, WebAssembly components on Wasmtime, as the code runtime,** with these
   conditions:
   - WIT is the public form of the Rust contract; a test fails if they drift (R1, phase 3.2);
   - guests: Rust first; JavaScript through a JS engine compiled to Wasm (ComponentizeJS, or
     QuickJS in Wasm as Figma does [R23]) once its size and startup are measured;
     Python through componentize-py only if demand appears;
   - every call under an epoch deadline; observers on a worker, interceptors with a
     2 ms budget; traps disable the extension, never Okur;
   - imports are the permissions: an extension without the "read text fields" import cannot
     read them;
   - extensions compiled ahead of time at install, so Okur ships the small runtime [R14];
   - track a Wasmtime LTS line [R5].
4. **Keep G as the escape hatch** for what must run natively or in another ecosystem: a CPython
   host for NVDA-style add-on authors, GPL synthesizers, braille drivers. Same schema, over a
   pipe.
5. **Do not do B or F in process.** No isolation, no budgets, and NVDA's experience (§3.10,
   §3.12) shows the cost.
6. **Settle it with a spike before phase 3** (§11 phase 3.1). Measure, with `okur latency`,
   the same focus filter (read five properties, replace one part) as:
   - a Rust trait call (baseline);
   - a Wasm component with a Rust guest;
   - a Wasm component with a JavaScript guest;
   - rquickjs in process.
   Report p50 and p99 per call, instantiation time, memory per extension, and binary size on
   the dev box and old-pc. Pass marks to agree with the owner first; proposed: p99 under 1 ms
   for a filter, under 50 ms to load an extension.

This keeps the vision's order (Rust SDK, one runtime chosen by measuring, packaging, the
developer interface) and its candidates (QuickJS, Boa, Wasm components): Boa is dropped from
the shortlist because it has no time budget [R28], and QuickJS is kept both as option D and as a
way to run JavaScript inside option C.

## 10. Okur's own words: languages and verbosity

These two belong together: both act on the same parts of an announcement, and both must leave
JAWS parity measurable.

### 10.1 What the reference machine shows about languages

- old-pc runs English JAWS on a Windows whose display language is Turkish, with the en-US
  culture and a Turkish Q keyboard ([`verification-loop.md`](verification-loop.md) §3;
  evidence provenance, `corpus/evidence/308-desktop-time-and-date-evidence.json` lines 11-16).
- So a JAWS transcript mixes two languages: JAWS's own words in English ("Button",
  "collapsed", "To activate press Enter.") and the application's words as Windows or the page
  gives them (WhatsApp's "Mesaj seçeneklerini aç Button collapsed",
  [`web-applications.md`](web-applications.md)).
- Okur's parity strings are therefore English for the screen reader's words and whatever the
  application says for the rest. A translation of Okur's words cannot be verified against this
  lab; it needs JAWS in that language (§2.4 on JAWS's languages) or a native reviewer.
- Some application words are also screen reader decisions: Okur matches UIA names such as
  "Items View" to recognize File Explorer (§7.1). Recognition must use language-independent
  properties (automation ids, class names inside modules, roles), never display text.

### 10.2 Design for translated messages

1. **Message ids, one catalog per language** (§11 phase 0.3). Ids name the situation, not the
   words: `quicknav.none.links`, `table.not-in-table`, `hint.activate.enter`,
   `role.button.focus`, `role.button.say-window`. Context goes into the id because JAWS's own
   English varies by context (§7.4).
2. **Each message stays whole.** Never assemble "There are no " + element + " on this page."
   Word order, cases and agreement differ by language (Turkish puts the verb last and
   inflects the noun). JAWS's English has per-message quirks Okur reproduces (the frames
   message without a period, `cursor.rs:970`).
3. **Placeholders with types, plural selection by CLDR rules** (number, count, ordinal), so
   "{count} items" can follow each language's rules; and so English can keep JAWS's exact
   wording even where it is not grammatical, because the English catalog is the parity
   reference and may hard-code what JAWS says.
4. **Format: Fluent or MessageFormat 2.** Fluent has a mature Rust implementation and
   selectors for plurals; Unicode MessageFormat 2.0 became stable in CLDR 47 (2025-03-13)
   [R53], but its Rust implementations are young [R54]. Fluent is the safer choice
   today; §9.5 compares them.
5. **Locale data from CLDR through ICU4X** (plural rules, list and number formatting), and from
   the operating system for dates and times, as Say System Time already does
   (`okur-platform/src/clock.rs:44-68`).
6. **Okur's language is a setting**, defaulting to the operating system's display language
   only when a catalog for it exists, otherwise English. Parity tests pin the English catalog.
7. **Speech language follows content.** Parts carry a language tag when the content has one
   (an HTML `lang`, a UIA culture), so the synthesizer can switch voices, as JAWS (since 8.0,
   by primary language code [R92]) and NVDA (§3.7) can. Okur's own words carry Okur's
   language. The reference machine has JAWS's language detection off (`LanguageDetection=0`,
   §2.5), so replay must keep switching off to compare like with like.
8. **Case and text rules are Unicode's**, with the language's tailoring where it matters
   (Turkish dotted and dotless i), §11 phase 0.7.
9. **Keys are physical.** A binding names the key's position (scan code), not the character
   it types, so a Turkish Q keyboard and a US keyboard press the same JAWS commands; key names
   in key help are localized. This needs evidence first: JAWS's behaviour on Turkish Q is an
   open question ([`jaws-virtual-buffer-spec.md`](jaws-virtual-buffer-spec.md) item 18;
   [`verification-loop.md`](verification-loop.md) §9).
10. **Extensions ship catalogs** in the same format, with the same id rules, and translators
    work on them in the same tool (Pontoon for Fluent, or a hosted service; NVDA's translation
    workflow is the reference, §3.7).
11. **Tests**: a catalog completeness check per language (every id present or explicitly
    falling back to English), a pseudo-locale run of the hermetic tests (every message
    replaced by a marked form) to find words still outside the catalog, and the English
    parity as today.

### 10.3 Design for verbosity

1. **Kinds on parts** (§11 phase 0.2) are what verbosity acts on: an announcement says "this is
   the role", "this is the position", "this is a hint", and the profile decides whether and
   how each is said.
2. **A profile** maps each kind and each role or state to one of: speak, speak in another
   voice or pitch, play a sound, say nothing. That is the shape of JAWS's Speech and Sounds
   Manager schemes and of its per-category settings (§2.5). JAWS's levels (§2.5) become named
   profiles.
3. **JAWS's defaults are the default profile;** the reference machine's recorded profile is
   what replay uses, as the keyboard layout and typing echo are today
   (`okur-corpus/src/parity.rs:568-588`). Newer evidence already records the user settings
   JAWS ran with (output modes, forms mode, application verbosity levels, voice profiles with
   punctuation; §2.5); replay must read them all, not only `KeyboardType` and `TypingEcho`.
4. **Long and short messages.** Where JAWS has a long and a short form of a message chosen by
   verbosity (§2.4), the catalog holds both under one id.
5. **Punctuation and symbols** are per-language data with levels, generalizing
   `pronounce.rs`, which today only names single characters read alone
   (`pronounce.rs:1-17`, `73`).
6. **Extensions declare categories** for their messages and get the same filtering, as JAWS's
   output types do for scripts (§2.4). NVDA has no such categories: add-ons filter or patch the
   speech sequence (§3.4), which verbosity settings then cannot reach.
7. **Braille** renders the same parts with its own profile (JAWS's structured mode, §2.8),
   when braille exists.

## 11. Roadmap: the refactoring before and for an SDK

The structure plan's rule holds for every refactor below: nothing the user hears changes, and
the commit shows it with `skills/jaws-reference/scripts/parity-snapshot.sh` before and after
(`diff -r` empty), every unit test passing unchanged in what it asserts, and the pre-commit
gate. Behavior changes are separate commits with their own tests. Phases 0 and 1 can start
now, alongside feature work; phase 3 waits for the "stable on Windows and macOS" condition.

### Phase 0: groundwork with no new behavior (now)

| # | Work | Where | Done when |
|---|---|---|---|
| 0.1 | **Finish announcements as data** (structure step 4): `SpeechFormatter` module by module, then the structured page line. | `speech.rs`, `linearize.rs`, `cursor.rs`, `buffer.rs:63-72` | No role mark is written outside the renderer (the step's own criterion); the 73 hand-written mark lines are gone. |
| 0.2 | **Typed parts.** `Part` gains kinds: name, value, role (an enum with a context, not a word), state (enum), position (index, total), description, hint (replacing `on_arrival`), shortcut, container, message (by id). The renderer chooses JAWS's word per role and context. | `announcement.rs:13-31`, `native_focus.rs:486-569` | `Part::Role(String)` and `Part::State(String)` no longer exist. |
| 0.3 | **Message ids.** Every user-facing literal becomes a message id with English text in one catalog file; duplicates collapse ("Not in a table." ×35). No new language yet; the English catalog is JAWS's English. | all of §7.4 | A test fails on any user-facing literal outside the catalog (an allow-list for identifiers); a test fails on catalog entries nothing uses. |
| 0.4 | **Decide keys before running them.** Split `handle_key` into a decision (consume or pass, from tables and a state snapshot) and an execution. Keep the order of "Who takes a key". | `engine.rs:232-480`, `455-477` | A unit test proves the decision never calls `execute`; replay unchanged. |
| 0.5 | **Commands with ids.** Built-in commands keep the enum and gain stable string ids (JAWS script names); bindings gain scope fields (application, site, mode) though only the global scope is used. | `command.rs` | `Command::from_name` and the ids agree (a test over all variants). |
| 0.6 | **Record languages and every application's settings in evidence.** Capture tooling already records `DEFAULT.JCF`, `Chrome.JCF` and voice profiles (`run-scenario.ps1:225-245`); add the Windows display language, JAWS's language folder, and the target application's JCF for every application, not only Chrome. | `skills/jaws-reference/scripts/run-scenario.ps1` | New evidence carries them; `okur corpus-manifest` shows them. |
| 0.7 | **Unicode case folding** where text is matched (finding headings, links, fields). | `engine/commands.rs:155-173` | Tests with "İ"/"ı" and other non-ASCII letters. (A behavior change: its own commit.) |

### Phase 1: the internal SDK (before macOS is finished, so macOS is built on it)

| # | Work | Where | Done when |
|---|---|---|---|
| 1.1 | **The contract, version 0.** `AppModule` becomes the extension trait: events beyond focus (R4), commands (R8), announcement filters (R5), lifecycle (application gained and lost), per-extension settings (R17), dialogs (R16). Precedence and fall-through (R2). | `apps/mod.rs`, `engine/focus.rs:93-122`, `engine.rs:207-221` | The nine modules use it; the engine has no `if app == …`. |
| 1.2 | **Modes owned by modules.** The desktop, taskbar and Explorer modes leave `AppMode` and become module state with a "keys go to the application" answer for the key decision. | `engine/state.rs:182-189`, `engine/commands.rs:110` | `AppMode` holds only cursor modes. |
| 1.3 | **Settings registry** with scopes and lifetimes (already decided, helper dialogs step 9), then verbosity and language settings in it. | `settings.rs`, `ui/settings.rs` | Quick Settings and Settings Center are generated from the registry. |
| 1.4 | **Generic dialogs.** A controller trait, a table of open dialogs by id, edit, tree and tabs controls. | `engine.rs:117-120`, `670-715`; `okur-ui-model` | The settings and Links List dialogs use the generic path. |
| 1.5 | **Key map as a file**, in JAWS's notation, with user overrides per scope; key help lists scoped commands. | `command.rs:404-578` | The built-in map loads from data; a user file overrides one binding in a test. |
| 1.6 | **Driver registry** for synthesizers; speech commands and index marks in `SpeechOutput`. | `tts/mod.rs:83-180`, `speech.rs:769-806` | Adding a driver touches one registration. |
| 1.7 | **A platform-neutral replay format**: the engine's own input (keys with times, `AccessibilityEvent`s) recorded beside the platform trace; `okur replay <file>` prints speech. | `okur-corpus`, `event.rs:393` | A test replays a recorded focus sequence on Linux CI. |
| 1.8 | **Read-only developer interface**: local IPC to a running Okur (state, speech history, events, the buffer), off by default; an MCP front end. | new crate | An agent reads the speech history of a live run. |

### Phase 2: verbosity and languages (after phase 1.3)

| # | Work | Done when |
|---|---|---|
| 2.1 | **Verbosity profiles** modeled on JAWS's (levels, categories, speech and sounds per role and state, punctuation), JAWS's defaults, the reference machine's recorded profile in replay. | Parity unchanged with the reference profile; a test per category switch. |
| 2.2 | **Second language.** A Fluent (or MessageFormat 2) catalog for Turkish, the owner's language, reviewed by the owner; language setting; number and list formatting from CLDR data. | Turkish speech in hermetic tests; English parity unchanged. |
| 2.3 | **Symbols and punctuation per language**, generalizing `pronounce.rs`. | Character names per language; JAWS's English names unchanged. |
| 2.4 | **Language switching** in speech when content declares a language (page `lang`, UIA culture). | A test with mixed-language content and a two-voice recording driver. |

### Phase 3: the runtime and packaging (after Okur is stable on Windows and macOS)

| # | Work | Done when |
|---|---|---|
| 3.1 | **Runtime spike** (§9.6): the same focus filter in Rust, a Wasm component (Rust and JavaScript guests), and QuickJS; latency and memory measured with `okur latency`. | A decision recorded in this page, with the measurements. |
| 3.2 | **The public contract** in the chosen interface language (WIT if Wasm), generated from or checked against the Rust contract. | One extension written outside the repository passes its replay tests. |
| 3.3 | **Packaging**: manifest, permissions, install, enable, disable, update, safe mode, kill switch (R18-R23). | A broken test extension is disabled without silencing Okur, in a live check. |
| 3.4 | **First external extension**: WhatsApp Web commands (the owner's request), proven by replay. | Its recorded interactions replay with the expected speech. |
| 3.5 | **`okur-developer` skill**: SDK docs, examples, the replay workflow, OpenDesk and old-pc workflows; the developer interface gains reload. | An agent writes, tests and proposes an extension from a plain-language report. |

### Phase 4: sharing

Registry, review and signing (R20), translation workflow for extension catalogs, and porting of
third-party scripts whose license allows it, after the owner's decision on R21.

## 12. Open questions for the owner

1. **Runtime.** Is WebAssembly components on Wasmtime acceptable as the code runtime, subject to
   the spike in §9.6? Or should agent-friendliness win, with JavaScript in QuickJS first?
   How much does Python matter (NVDA add-on authors), given it can only run out of process
   safely?
2. **Okur's own application modules.** Keep them in Rust in the repository, as JAWS ships its
   application scripts with JAWS (the recommendation), or move some to first-party extensions?
3. **Okur's language.** Should Okur speak English by default, like the reference machine's
   JAWS, or follow the Windows display language once a catalog exists? Is Turkish the first
   translation, reviewed by you?
4. **Evidence for other languages.** No Turkish JAWS interface was found (§2.4). Would a second
   JAWS language on old-pc (installed at setup, §2.4) be worth it, to test that catalogs and
   parity work for a language other than English?
5. **Verbosity defaults.** JAWS's defaults (Beginner verbosity) or the reference machine's
   settings, for a new Okur install? The names of JAWS's verbosity check boxes need reading in
   JAWS's own dialog on old-pc or the dev box, and the reference machine's `.jcf` files read
   for its verbosity (allowed by AGENTS.md §1.4). Does a Speech and Sounds scheme file count as
   user settings that may be read? AGENTS.md lists `.jcf` and `.vpf` only.
6. **Physical or typed keys.** On a Turkish Q keyboard, should Okur's quick navigation keys
   follow the key's position or its character? This needs JAWS evidence (§10.2 item 9).
7. **Porting third-party scripts.** The vision allows porting third-party scripts whose
   license permits it; AGENTS.md §1.4 forbids reading any script source, third parties'
   included, and Doug Lee's scripts have been binary-only since 2019 [J37]. Should porting be
   dropped, or allowed for scripts whose authors give source and permission? Legal advice may
   be needed.
8. **WhatsApp as the first extension.** Freedom Scientific's WhatsApp scripts target the
   Windows application, not WhatsApp Web in Chrome, which you use (§2.9). Hartgen sells WhatsApp
   Web scripts [J36]. Should the extension follow JAWS's WhatsApp keys (Alt+1 to 9, Alt+E,
   Alt+C, Alt+H) in the web version, observed on old-pc in the Windows app?
9. **Trust.** A reviewed registry with hashes (NVDA), approval by hash on each machine (Orca),
   signing, or all three? Who reviews: you, agents, both?
10. **Permissions.** Should any extension ever read protected (password) fields? Have network
    access? Run programs?
11. **Global extensions.** Allow extensions that act everywhere (NVDA's global plugins), or
    only application and site extensions plus global commands?
12. **Timing.** Phase 0 can start now; should phase 1 (the internal contract) land before the
    macOS port, so that macOS is built on it?
13. **Sounds and braille.** Verbosity schemes need a sound output; braille needs drivers and
    liblouis (LGPL). When do these enter the plan?

## 13. What could not be verified

- Freedom Scientific's documentation pages refused automated access (HTTP 403); every
  "(excerpt)" in §2 should be checked in a browser.
- JAWS's list of output types and its verbosity check box names (header files may not be
  read; the dialog can be read on old-pc).
- The format of script-provided Quick Settings files.
- Whether a Turkish JAWS interface exists.
- The default `LowLevelHooksTimeout` and macOS's event tap timeout.
- Call overhead of Wasm component calls with strings and records, and IPC latency on Windows
  and macOS: no published measurements were found; the spike in §9.6 measures them.
- Whether UI Automation works from inside an AppContainer.
- NVDA 2027.1's Python version (sources disagree).

## Sources

Read 2026-10-18 by three research sub-agents working under the clean-room rules above, one for
JAWS, one for NVDA, one for runtimes, languages and other screen readers. "(excerpt)": known
only through search engines' excerpts of the page, because the site refused automated access;
check in a browser. Pages marked "(checked)" were opened again by the writer of this page to
confirm the facts and quoted words used here.

### JAWS and its ecosystem

Freedom Scientific Script Manual pages share the prefix
`https://support.freedomscientific.com/Content/Documents/Other/ScriptManual/`.

- [J1] Script Manual 3.4, Scripts vs. Functions: `…/03-4_ScriptsVsFunctions.htm` (excerpt)
- [J2] Script Manual 5.2, The New Script Dialog: `…/05-2_TheNewScriptDialog.htm` (excerpt)
- [J3] Script Manual 1.2, Definition of Terms:
  https://www.freedomscientific.com/Content/Documents/Other/ScriptManual/01-2_DefinitionOfTerms.htm (excerpt)
- [J4] AccessWorld, J. Lazzaro, introduction to JAWS scripting, 2003: https://www.afb.org/aw/4/6/14806
- [J5] FSDN: https://support.freedomscientific.com/support/jawsdocumentation/FSDN (excerpt)
- [J6] Italian JAWS scripting tutorial, event functions: https://www.norobot.it/gli-script-di-jaws-08/
- [J7] Doug Lee, BX user's guide: https://www.dlee.org/bx/
- [J8] Script Manual 3.3, Performing Functions: `…/03-3_PerformingFunctions.htm` (excerpt)
- [J9] UIA Script API: https://support.freedomscientific.com/support/jawsdocumentation/UIAScriptAPI (excerpt)
- [J10] Script Manual 3.2, Processing Keystrokes: `…/03-2_ProcessingKeystrokes.htm` (excerpt; checked by a second search)
- [J11] Script Manual Appendix D: `…/Appendix_D.htm` (excerpt)
- [J12] Doug Lee, common script elements (chaining, installers): https://dlee.org/code_common.htm
- [J13] Script Manual 12.5, The Use Statement: `…/12-5_TheUseStatement.htm` (excerpt)
- [J14] Doug Lee, 2003 proposal on JAWS file layering: https://www.dlee.org/jflprop.htm
- [J15] Scripting information (domain scripts, collections, UIA):
  https://support.freedomscientific.com/support/jawsdocumentation/ScriptingInfo (excerpt)
- [J16] Domain Scripting Quick Start:
  https://support.freedomscientific.com/Content/Documents/Other/Domain-Scripting-Quick-Start.pdf (excerpt)
- [J17] Doc Center, Domain Scripts:
  https://doccenter.freedomscientific.com/doccenter2/doccenter/rs25c51746a0cc/DomainScripts/02_DomainScripts.htm (excerpt)
- [J18] Vispero, JAWS timeline: https://vispero.com/jaws-timeline/
- [J19] JAWS 17 release: https://vispero.com/news/freedom-scientific-releases-jaws-17/
- [J20] Script Manual 2.0, 2.2, 2.7, 2.8, Keyboard Manager: `…/02-0_TheKeyboardManager.htm`,
  `…/02-2_KeyboardManagerFiles.htm`, `…/02-7_AddingChangingAndRemovingKeystrokes.htm`,
  `…/02-8_KeyboardManagerOptions.htm` (excerpts)
- [J21] Script Manual 4.0 and 4.1, Script Manager: `…/04-0_TheScriptManager.htm`,
  `…/04-1_ScriptManagerFiles.htm` (excerpts)
- [J22] Doc Center, Speech and Sounds Manager:
  https://doccenter.freedomscientific.com/doccenter2/doccenter/rs25c51746a0cc/speechandsoundsmanager/02_Speech&SoundsManager.htm (excerpt)
- [J23] Surf's Up, Personalized Settings and Custom Labels:
  https://www.freedomscientific.com/SurfsUp/Personalized_Settings.htm,
  https://support.freedomscientific.com/SurfsUp/19-CustomLabels.htm (excerpts)
- [J24] Surf's Up, PlaceMarkers: https://support.freedomscientific.com/SurfsUp/15-PlaceMarkers.htm (excerpt)
- [J25] Intuit, installing JAWS scripts for QuickBooks Desktop:
  https://http-download.intuit.com/http.intuit/QBDTjaws/scripts/Installing_JAWS_Scripts_for_QuickBooks_Desktop.pdf (excerpt)
- [J26] Modern Script Compiler, December 2024 update (Hebrew): https://ltalk.net/?p=44704
- [J27] Freedom Scientific GmbH, JAWS 2026 What's New (Spanish): https://www.freedomsci.de/news/new_2026_esn.htm (checked)
- [J28] JAWS 2026 What's New (Czech): https://www.galop.cz/doc/jaws/jaws2026novinky.html
- [J29] Script Manual 13.5, Debugging Scripts: `…/13-5_DebuggingScripts.htm` (excerpt)
- [J30] Seamless script updates, 2024: https://nelowvision.com/software-updates-for-jaws-zoomtext-and-fusion-2024/
- [J31] Vispero, June 2026 update (Cloud Settings Sync):
  https://training.vispero.com/download-the-june-update-of-jaws-zoomtext-and-fusion-2026/
- [J32] Script Manual 7.1, Variables: `…/07-1_Variables.htm` (excerpt)
- [J33] Wikipedia, JAWS Scripting Language: https://en.wikipedia.org/wiki/JAWS_Scripting_Language
- [J34] Script Manual 8.0, 8.2, 8.3, 8.5, messages: `…/08-0_CreatingAndSpeakingMessages.htm`,
  `…/08-2_TheJAWSMessageFormat.htm`, `…/08-3_FormattingYourMessages.htm`,
  `…/08-5_ChapterExercises.htm` (excerpts)
- [J35] JAWS languages: https://www.freedomscientific.com/training/jaws/frequently-asked-questions/,
  https://support.freedomscientific.com/Downloads/OfflineInstallers (excerpts);
  https://nelowvision.com/low-vision-software-updates-for-2025/ (Slovak)
- [J36] Hartgen Consultancy: https://hartgenconsultancy.com/, https://hartgenconsultancy.com/jawsscripts
- [J37] Doug Lee, code policy (licenses, translation system): https://www.dlee.org/code_policy.htm
- [J38] JAWS, set speech verbosity (support article): https://sastltd.zohodesk.eu/portal/en/kb/articles/jaws-reducing-speech-verbosity (checked)
- [J40] Surf's Up, Speech and Sounds schemes; TAVIP introduction:
  https://support.freedomscientific.com/SurfsUp/18-SpeechSoundsSchemes.htm,
  https://www.tavip.org.uk/an-introduction-to-the-jaws-speech-and-sounds-manager/ (excerpts)
- [J41] Changing JAWS punctuation: https://sastltd.zohodesk.eu/portal/en/kb/articles/changing-jaws-punctuation-announcement (excerpt)
- [J42] Doc Center, Quick Settings:
  https://doccenter.freedomscientific.com/doccenter2/doccenter/rs25c51746a0cc/QuickSettings/02_QuickSettings.htm (excerpt)
- [J43] JAWS keystrokes: https://support.freedomscientific.com/content/html/jawshq/JAWS-Keystrokes.html (excerpt)
- [J44] Surf's Up, Flexible Web: https://support.freedomscientific.com/SurfsUp/16-FlexibleWeb.htm (excerpt)
- [J45] Dictionary Manager lesson: https://support.freedomscientific.com/teachers/lessons/9.0.2_Objectives.htm (excerpt)
- [J46] Vispero, December 2025 update (AI Labeler):
  https://vispero.com/resources/download-the-december-update-of-jaws-zoomtext-and-fusion-2026/
- [J47] Braille structured mode:
  https://support.freedomscientific.com/teachers/lessons/8.2_Transcript-StartupWizardBrailleOptions.htm,
  https://webaim.org/discussion/mail_message?id=22902 (excerpts)
- [J48] Script Manual 13.3, Script Utility Mode: `…/13-3_ScriptUtilityMode.htm` (excerpt)
- [J49] JAWS Quick Start Guide:
  https://support.freedomscientific.com/Content/Documents/Manuals/JAWS/JAWS-Quick-Start-Guide.pdf (excerpt)
- [J50] Speech History lesson: https://support.freedomscientific.com/teachers/lessons/5.5.1_SpeechHistory.htm (excerpt)
- [J51] Accessibility Developer Guide, JAWS: https://www.accessibility-developer-guide.com/setup/screen-readers/jaws/
- [J52] JAWS Inspect: https://vispero.com/jaws-inspect-accessibility-testing-tool/
- [J53] JAWS support for WhatsApp on Windows: https://sastltd.zohodesk.eu/portal/en/kb/articles/jaws-support-for-whatsapp-on-windows
- [J54] Vispero, February 2023 updates (WhatsApp scripts):
  https://training.vispero.com/whats-new-in-the-february-2023-software-updates/
- [J55] FSAPI members, Doug Lee on NFBCS, 2018:
  http://nfbnet.org/pipermail/nfbcs_nfbnet.org/2018-September/012321.html
- [J56] FSAPI, 2012 support reply relayed on JAWSScripts: https://www.freelists.org/post/jawsscripts/FW-FSAPIdll (excerpt)
- [J57] Freedom Scientific GmbH, release dates: https://www.freedomsci.de/news_esn.htm
- [J58] Doug Lee, WhatsApp scripts: https://dlee.org/whatsapp/
- [J59] Doug Lee, Teams scripts: https://www.dlee.org/teams/
- [J60] Doug Lee, VS Code install issues: https://www.dlee.org/vscode/inst_issues.htm
- [J61] Hartgen, WhatsApp scripts: https://hartgenconsultancy.com/whatsapp
- [J62] AccessWorld, Leasey review, 2020: https://www.afb.org/aw/21/7/17111
- [J63] Hartgen, scripting course: https://hartgenconsultancy.com/jawsscripting
- [J64] Vispero, JAWS scripting services, 2025:
  https://vispero.com/resources/jaws-scripting-better-support-for-jaws-screen-reader-users/
- [J65] Job Accommodation Network, screen reader scripting: https://askjan.org/solutions/Screen-Reader-Scripting.cfm
- [J67] Surf's Up, Settings Packager: https://www2.freedomscientific.com/training/Surfs-Up/Settings_Packager.htm (excerpt)
- [J68] Import/Export lesson: https://support.freedomscientific.com/teachers/lessons/9.8_Transcript-ImportExport.htm (excerpt)
- [J69] Installing JAWS scripts (TechTalk): https://techtalk.groups.io/g/main/topic/installing_jaws_scripts/5121410 (excerpt)
- [J70] AudioGames forum: https://forum.audiogames.net/viewtopic.php?id=15035,
  https://forum.audiogames.net/viewtopic.php?id=15924 (excerpts)
- [J71] WebAIM screen reader survey 10: https://webaim.org/projects/screenreadersurvey10/ (excerpt)
- [J72] Script Manual 10.1, Pass-Through Scripts: `…/10-1_PassThroughScripts.htm` (excerpt)
- [J73] Script Manual 8.4, Speaking Your Messages: `…/08-4_SpeakingYourMessages.htm` (excerpt)
- [J74] JAWS interface languages at install: https://x.com/FreedomSci/status/2021616596981092637,
  https://support.freedomscientific.com/Downloads/JAWS (excerpts)
- [J76] Turkish: https://www.brailleteknik.com/makaleler;
  https://doccenter.freedomscientific.com/doccenter2/doccenter/rs25c51746a0cc/voiceprofiles/02_voiceprofiles.htm (excerpt)
- [J77] JAWSScripts mailing list: https://www.freelists.org/list/jawsscripts (excerpt)
- [J78] JAWS 5.0 features: https://www2.freedomscientific.com/fs_products/software_jaws5udfea.asp (excerpt)
- [J79] Settings Center braille structured mode options:
  https://connection.nwea.org/articles/Knowledge/JAWS-repeats-clickable-How-to-modify-onclick-settings (excerpt)

### NVDA

- [N1] NVDA Developer Guide (2026.2): https://download.nvaccess.org/documentation/developerGuide.html (checked)
- [N2] NVDA User Guide (2026.2): https://download.nvaccess.org/documentation/userGuide.html
- [N3] NVDA What's New (2026.1.1): https://download.nvaccess.org/releases/2026.1.1/documentation/changes.html (checked)
- NVDA source, `master`, prefix `https://github.com/nvaccess/nvda/blob/master/`:
  [N5] `source/addonHandler/__init__.py`; [N6] `source/addonHandler/packaging.py`;
  [N7] `source/addonStore/models/status.py`; [N8] `source/addonStore/install.py`;
  [N9] `source/gui/exit.py`; [N10] `source/addonAPIVersion.py`;
  [N11] `source/addonHandler/addonVersionCheck.py`; [N12] `source/addonStore/models/version.py`;
  [N15] `projectDocs/dev/deprecations.md`; [N22] `source/addonStore/network.py`;
  [N30] `source/appModuleHandler.py`; [N31] `source/eventHandler.py`;
  [N32] `source/NVDAObjects/__init__.py`; [N33] `source/globalPluginHandler.py`;
  [N35] `source/treeInterceptorHandler.py`; [N36] `source/scriptHandler.py`;
  [N37] `source/extensionPoints/__init__.py`; [N38] `source/speech/extensions.py`;
  [N39] `source/braille/extensions.py`; [N40] `source/synthDriverHandler.py`;
  [N41] `source/speech/commands.py`; [N42] `source/braille/display/driver.py`;
  [N43] `source/bdDetect.py`; [N44] `source/vision/providerBase.py`;
  [N45] `source/gui/settingsDialogs.py`; [N47] `source/config/__init__.py`;
  [N48] `source/speech/speech.py`; [N49] `source/characterProcessing.py`;
  [N50] `source/speechDictHandler/__init__.py`; [N51] `projectDocs/dev/codingStandards.md`;
  [N53] `projectDocs/translating/readme.md`; [N56] `projectDocs/testing/automated.md`;
  [N57] `tests/system/readme.md`; [N76] `source/core.py`; [N77] `source/queueHandler.py`;
  [N78] `source/watchdog.py`
- [N13] NVDA 2023.2 (Add-on Store): https://www.nvaccess.org/post/nvda-2023-2/
- [N16] NVDA Add-on API announcements group: https://groups.google.com/a/nvaccess.org/g/nvda-api
- [N17] NVDA 2026.1: https://www.nvaccess.org/post/nvda-2026-1/ (checked)
- [N18] 32-bit synthesizer host, PR #19432: https://github.com/nvaccess/nvda/pull/19432
- [N19] 2027.1 API announcement: https://groups.google.com/a/nvaccess.org/g/nvda-api/c/b3GzxEjATsg
- [N20] Add-on datastore README: https://github.com/nvaccess/addon-datastore (checked)
- [N21] Datastore design overview: https://github.com/nvaccess/addon-datastore/blob/master/docs/design/designOverview.md
- [N23] Add-on version schema:
  https://raw.githubusercontent.com/nvaccess/addon-datastore-validation/main/_validate/addonVersion_schema.json
- [N24] Submission form: https://raw.githubusercontent.com/nvaccess/addon-datastore/master/.github/ISSUE_TEMPLATE/registerAddon.yml
- [N25] Submission guide: https://github.com/nvaccess/addon-datastore/blob/master/docs/submitters/submissionGuide.md
- [N26] Datastore validation: https://github.com/nvaccess/addon-datastore-validation
- [N27] NV Access In-Process, 10 March 2026: https://www.nvaccess.org/?p=32581
- [N28] Submission review: https://github.com/nvaccess/addon-datastore/blob/master/docs/dev/submissionReview.md
- [N29] Add-on Store web listing: https://addonstore.nvaccess.org/
- [N46] Community Add-on Development Guide: https://github.com/nvdaaddons/DevGuide/wiki/NVDA-Add-on-Development-Guide
- [N52] User Guide translation on Crowdin, PR #17050: https://github.com/nvaccess/nvda/pull/17050
- [N54] Add-on template: https://github.com/nvaccess/AddonTemplate
- [N55] Add-on translation: https://github.com/nvaccess/AddonTemplate/blob/master/docs/l10n/addonAuthors.md
- [N58] Why NVDA can't run on Windows 10 S (UIAccess): https://www.nvaccess.org/post/why-cant-nvda-run-on-windows-10-mobile-and-windows-10-s/
- [N59] Advisory GHSA-727q-h8j2-6p45: https://github.com/nvaccess/nvda/security/advisories/GHSA-727q-h8j2-6p45
- [N60] NVDA security advisories: https://github.com/nvaccess/nvda/security/advisories
- [N61] NVDA roadmap, 6 May 2026: https://www.nvaccess.org/post/nvda-roadmap/ (checked)
- [N62] Sandboxing prototype, PR #19412: https://github.com/nvaccess/nvda/pull/19412
- [N63] Out-of-process add-ons discussion #16304: https://github.com/nvaccess/nvda/discussions/16304
- [N64] Remote Access in NVDA 2025.1: https://www.nvaccess.org/?p=31673
- [N65] Windows App Essentials: https://github.com/josephsl/wintenApps,
  https://github.com/josephsl/wintenApps/blob/master/addoninternals.md
- [N66] Add-on Updater: https://github.com/josephsl/addonUpdater
- [N67] Speech History: https://github.com/jscholes/nvda-speech-history
- [N68] Clipspeak: https://github.com/ragb/clipspeak
- [N69] IBMTTS driver: https://github.com/davidacm/NVDA-IBMTTS-Driver
- [N70] WhatsAppNG: https://github.com/nunotfc/WhatsAppNG
- [N71] WhatsApp Companion: https://github.com/muhammadGagah/whatsapp-web-plus-companion
- [N72] Add-on compatibility with 2026.1, 6 May 2026: https://groups.google.com/a/nvaccess.org/g/nvda-users/c/F3Rav4HvBgE
- [N73] NV Access In-Process, 11 May 2026: https://www.nvaccess.org/post/in-process-11th-may-2026/ (checked)
- [N74] 64-bit add-on compatibility guide: https://groups.google.com/a/nvaccess.org/g/nvda-users/c/z_TOySDuatE
- [N75] NV Access In-Process, 30 April 2025: https://www.nvaccess.org/post/in-process-30th-april-2025/
- [N79] NV Access In-Process, 11 August 2022 (language switching): https://www.nvaccess.org/?p=27129 (checked)
- [N80] Sound split, PR #16071: https://github.com/nvaccess/nvda/pull/16071

### Runtimes, languages, platforms, other screen readers

- [R1] Microsoft, LowLevelKeyboardProc: https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc (checked)
- [R2] Apple, CGEventType.tapDisabledByTimeout: https://developer.apple.com/documentation/coregraphics/cgeventtype/tapdisabledbytimeout
- [R3] Cocoa-dev, event taps disabled: https://lists.apple.com/archives/Cocoa-dev/2010/Sep/msg00892.html
- [R5] Wasmtime release policy: https://docs.wasmtime.dev/stability-release.html
- [R6] WASI 0.3 release: https://wasi.dev/releases/wasi-p3 (checked)
- [R7] WASI languages: https://wasi.dev/languages
- [R8] wit-bindgen: https://github.com/bytecodealliance/wit-bindgen
- [R9] Wasmtime, interrupting execution: https://docs.wasmtime.dev/examples-interrupting-wasm.html (checked)
- [R10] Wasmtime `Config`: https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html
- [R11] Wasmtime `ResourceLimiter`: https://docs.rs/wasmtime/latest/wasmtime/trait.ResourceLimiter.html
- [R12] Wasmtime, fast instantiation: https://docs.wasmtime.dev/examples-fast-instantiation.html
- [R13] Wasmtime 1.0 performance: https://bytecodealliance.org/articles/wasmtime-10-performance
- [R14] Wasmtime, minimal builds: https://docs.wasmtime.dev/examples-minimal.html (checked)
- [R15] ComponentizeJS: https://github.com/bytecodealliance/ComponentizeJS
- [R16] componentize-py: https://pypi.org/project/componentize-py/;
  https://learn.golem.cloud/python-language-guide/building-components
- [R17] Zed, extensions: https://zed.dev/blog/zed-decoded-extensions (checked)
- [R18] Zed, language extensions: https://zed.dev/blog/language-extensions-part-1 (checked)
- [R19] Zed discussion #24515: https://github.com/zed-industries/zed/discussions/24515
- [R21] Shopify Functions, instruction counts: https://shopify.dev/docs/apps/build/functions/optimize-instruction-counts
- [R22] Figma, plugin system: https://www.figma.com/blog/how-we-built-the-figma-plugin-system/
- [R23] Figma plugins in QuickJS: https://macwright.com/2024/03/29/figma-plugins
- [R27] Boa 0.21: https://opencollective.com/boa/updates/boa-release-v0-21; https://boajs.dev/
- [R28] Boa `RuntimeLimits`: https://docs.rs/boa_engine/latest/boa_engine/vm/struct.RuntimeLimits.html (checked)
- [R29] QuickJS: https://bellard.org/quickjs/ (checked)
- [R30] rquickjs: https://docs.rs/rquickjs/latest/rquickjs/struct.Runtime.html; https://github.com/DelSkayn/rquickjs (checked)
- [R31] V8 heap limits: https://docs.rs/v8/latest/v8/struct.CreateParams.html
- [R32] RustPython: https://github.com/RustPython/RustPython
- [R33] PEP 578: https://peps.python.org/pep-0578/
- [R34] Python 3.14: https://docs.python.org/3/whatsnew/3.14.html; Python distribution sizes:
  https://pydevtools.com/handbook/reference/python-build-standalone/
- [R35] mlua: https://github.com/mlua-rs/mlua
- [R36] Luau sandbox: https://luau.org/sandbox
- [R37] Rhai: https://rhai.rs/book/safety/max-operations.html; https://rhai.rs/book/about/features.html
- [R39] abi_stable: https://docs.rs/abi_stable/latest/abi_stable/
- [R40] VS Code extension host: https://code.visualstudio.com/api/advanced-topics/extension-host
- [R41] Neovim remote plugins: https://neovim.io/doc/user/remote_plugin/
- [R42] ipc-bench: https://github.com/goldsborough/ipc-bench
- [R43] AppContainer isolation: https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation
- [R44] Job objects: https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects
- [R45] Landlock: https://docs.kernel.org/userspace-api/landlock.html
- [R46] App Sandbox and accessibility (Apple forums): https://developer.apple.com/forums/thread/810677;
  https://developer.apple.com/forums/thread/756130
- [R47] fluent-rs: https://github.com/projectfluent/fluent-rs
- [R48] `FluentBundle` isolation marks: https://docs.rs/fluent-bundle/latest/fluent_bundle/bundle/struct.FluentBundle.html
- [R49] Fluent selectors and terms: https://projectfluent.org/fluent/guide/selectors.html;
  https://projectfluent.org/fluent/guide/terms.html
- [R50] Pontoon: https://github.com/mozilla/pontoon
- [R51] ICU4X 2.0 and 2.1: https://blog.unicode.org/2025/05/icu4x-20-released.html;
  https://blog.unicode.org/2025/10/icu4x-21-released.html
- [R52] `icu` crate: https://docs.rs/icu/latest/icu/
- [R53] CLDR 47 (MessageFormat 2.0 stable): https://cldr.unicode.org/downloads/cldr-47 (checked)
- [R54] `mf2` crate: https://docs.rs/crate/mf2/latest (checked by search)
- [R55] CLDR plural rules: https://www.unicode.org/cldr/charts/latest/supplemental/language_plural_rules.html
- [R57] NVDA translation workflow: https://github.com/nvaccess/nvda/blob/master/projectDocs/translating/readme.md
- [R58] Orca translations (GNOME Damned Lies): https://l10n.gnome.org/module/orca/
- [R59] Orca script manager: https://gitlab.gnome.org/GNOME/orca/-/raw/main/src/orca/script_manager.py
- [R60] Orca NEWS: https://gitlab.gnome.org/GNOME/orca/-/raw/main/NEWS (checked)
- [R61] Orca user extensions: https://gitlab.gnome.org/GNOME/orca/-/raw/main/docs/user-extensions.md (checked)
- [R62] Orca remote controller: https://gitlab.gnome.org/GNOME/orca/-/raw/main/docs/remote-controller.md
- [R63] Orca speech generator: https://gitlab.gnome.org/GNOME/orca/-/raw/main/src/orca/speech_generator.py
- [R64] VoiceOver Utility, General: https://support.apple.com/guide/voiceover/cpvougen/mac
- [R65] VoiceOver Activities: https://support.apple.com/guide/voiceover/cpvouactivities/mac
- [R66] VoiceOver commands: https://support.apple.com/guide/voiceover/commands-category-voe7adb19917/mac;
  https://support.apple.com/en-gu/guide/voiceover/vo14096/8/mac
- [R67] VoiceOver verbosity, text: https://support.apple.com/guide/voiceover/cpvouverbtext/mac
- [R68] VoiceOver preferences export: https://support.apple.com/guide/voiceover/vo27926/mac
- [R69] AppleVis forum, VoiceOver and AppleScript:
  https://applevis.com/forum/macos-mac-apps/what-are-real-possibilities-voiceover-apple-script
- [R70] Narrator, customizing: https://support.microsoft.com/en-us/windows/chapter-7-customizing-narrator-ce950246-c915-0d44-9be6-fb474387a285
- [R71] Narrator and Excel: https://support.microsoft.com/en-us/excel/enhanced-narrator-support-with-excel
- [R72] Narrator extensions setting: https://www.elevenforum.com/t/enable-or-disable-narrator-extensions-in-windows-11.44254/
- [R73] Narrator developer mode: https://learn.microsoft.com/en-us/shows/one-dev-minute/using-narrator-dev-mode;
  https://tink.uk/using-narrator-dev-mode
- [R74] TalkBack: https://github.com/google/talkback
- [R75] Dolphin scripting: https://yourdolphin.com/Scripting
- [R76] liblouis: https://github.com/liblouis/liblouis; https://liblouis.io/
- [R77] BRLTTY: https://brltty.app/doc/README.txt
- [R78] eSpeak NG: https://github.com/espeak-ng/espeak-ng
- [R79] Speech Dispatcher: https://github.com/brailcom/speechd
- [R83] Apache License and GPL compatibility: https://www.apache.org/licenses/GPL-compatibility
- [R84] GPL FAQ: https://www.gnu.org/licenses/gpl-faq.html (wording via
  https://lists.opensource.org/pipermail/license-discuss_lists.opensource.org/2019-January/020246.html;
  gnu.org refused automated access)
- [R85] Weblate, Fluent format: https://docs.weblate.org/en/latest/formats/fluent.html
- [R86] NV Access, Corporate & Government page (add-ons can run any code): https://www.nvaccess.org/?p=23742 (checked)
- [R87] Wasmtime call overhead, PR #2759: https://github.com/bytecodealliance/wasmtime/pull/2759
- [R91] Orca license: https://gitlab.gnome.org/GNOME/orca/-/raw/main/COPYING
- [R92] W3C WCAG 2.0 technique H58 (JAWS language switching):
  https://www.w3.org/TR/2010/NOTE-WCAG20-TECHS-20101014/H58
- [R94] Apple, XPC services:
  https://developer.apple.com/library/mac/documentation/MacOSX/Conceptual/BPSystemStartup/Chapters/CreatingXPCServices.html
- [R95] LibreAutomate, `LowLevelHooksTimeout`: https://libreautomate.com/api/Au.More.WindowsHook.LowLevelHooksTimeout.html

### Repository files read

- `AGENTS.md`, `CLAUDE.md`
- `docs/references/extension-sdk-vision.md`, `helper-ui-architecture.md`, `web-applications.md`,
  `forms-and-interaction-modes.md`, `jaws-dialogs-and-visual-features.md`,
  `jaws-virtual-buffer-spec.md`, `tts-synthesizers.md`, `verification-loop.md`
- `docs/plans/2026-10-15-structure-for-macos-and-extensions.md`, `2026-10-17-helper-dialogs.md`,
  `2026-10-17-web-apps-round-3.md`
- `docs/handoff/2026-10-15-structure-round-1.md`, `2026-10-17-whatsapp-web.md`
- `crates/okur-core/src/`: `apps/mod.rs`, `apps/windows/mod.rs`, `apps/windows/explorer.rs`,
  `apps/web/mod.rs`, `apps/web/google_docs.rs`, `engine.rs`, `engine/commands.rs`,
  `engine/focus.rs`, `engine/page.rs`, `engine/state.rs`, `command.rs`, `announcement.rs`,
  `speech.rs`, `linearize.rs`, `native_focus.rs`, `buffer.rs`, `cursor.rs`, `event.rs`,
  `aom.rs`, `actions.rs`, `settings.rs`, `clock.rs`, `pronounce.rs`, `editing.rs`, `ui.rs`,
  `ui/settings.rs`, `lib.rs`, `Cargo.toml`
- `crates/okur-ui-model/src/lib.rs`, `crates/okur-ui/src/main.rs`
- `crates/okur-platform/src/`: `lib.rs`, `hook/mod.rs`, `hook/windows.rs`, `hook/macos.rs`,
  `events/mod.rs`, `tts/mod.rs`, `settings_file.rs`, `ui_host.rs`, `clock.rs`,
  `event_trace.rs` (header)
- `crates/okur/src/main.rs`; `crates/okur-corpus/src/parity.rs`, `latency.rs`
- `crates/xa11y/VENDORED.md`
- `corpus/evidence/308-desktop-time-and-date-evidence.json` (provenance)
- The session transcript `79caf556-….jsonl`, searched for "extension", "SDK", "WhatsApp",
  "script": the ideation exchange of 2026-10-15 and the owner's WhatsApp remarks of
  2026-10-17.
