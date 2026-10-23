# JAWS's customization surface, mapped to Okur's settings

Date: 2026-10-21. The owner asked which of JAWS's many settings Okur already has, and how
Okur's settings should be designed so that adding more is easy. This document is research
only: no code was changed, nothing was recorded on old-pc, and JAWS was not touched on any
machine. Every JAWS fact names its source (listed at the end); every Okur fact names a file and
line. Freedom Scientific's help texts are summarized, never copied.

Following the owner's rule, each setting proposed here is still to be found in JAWS's
documentation (done here), then recorded on old-pc, then built. Okur's own dialogs and messages
keep Okur's name for what JAWS names after itself (AGENTS.md §1.5): the Okur key, Okur
messages, Prompt to Confirm When Exiting Okur.

## 1. Summary

- **JAWS** spreads its customization over Settings Center (every option, for all applications
  or one), Quick Settings (the application in front, with lifetimes: saved, until exit, until
  the focus changes), Personalize Web Settings (one site), voice profiles with voice contexts,
  Speech and Sounds schemes, Dictionary Manager, key managers, toggle keys and a Startup Wizard.
  Both big dialogs have a search box, a tree of groups and a description of each setting.
- **Okur** already has the hard part of the model: settings layered as all applications, one
  application, one site and the session, saved as TOML files that hold only what differs
  (`settings_layers.rs`, `settings_file.rs`). What it lacks is breadth (16 settings), a
  generated dialog with search and description, lifetimes shorter than the session, and a
  single declaration per setting.
- **The map (§4)**: 134 JAWS settings and facilities, braille and the assistive utilities left
  out. **9 built, 41 partial, 84 missing.** Most partial rows are behaviors Okur already has at
  JAWS's default, with no way to change them (§5): the cheapest wins.
- **The design (§7)**: keep the typed `Settings` struct for the engine's hot paths; add one
  static table of `SettingSpec` (id and TOML path, group, catalog label, description and search
  words, kind, allowed scopes and lifetimes, Quick Settings flag, JAWS `.jcf`/`.vpf` key, toggle
  key, source). Settings Center, Quick Settings, search, the command line, the TOML reference,
  the JAWS import used by parity replay, and the toggle keys are generated from it; a guard
  test keeps the table and the struct in step.
- **First additions (§8)**: typing echo and key labels; forms mode (Manual, Semi-Auto, Auto);
  page load reading and summary; web verbosity levels; verbosity output types and tutor
  messages; punctuation per context and symbol customizing; capitalization; numbers and word
  splitting; voice pitch and contexts; Say All line pauses, blank lines, indentation, live
  regions.

## 2. How JAWS organizes its settings

### 2.1 The places a user changes them

| Place | How it opens | What it is | Source |
|---|---|---|---|
| Settings Center | Insert+6, or Insert+F2 then Settings Center | Every option. A search box (focused on opening, Ctrl+E) filters a tree of groups; Tab reaches a read-only description of the selected item, then Apply, OK, Cancel and an Application combo box. It opens on the application in front; Ctrl+Shift+D loads "Default (All Applications)". A "Last Changed Settings" group at the end of the tree lists recent changes. Space cycles an item's values. | [SU6], [SU7], [F1] |
| Quick Settings | Insert+V | The application in front only (its own groups first, the general ones after), with the same search, tree and description. Each item's context menu sets how long a change lasts: saved (the default), until JAWS exits, or until the focus changes; a group's context menu sets it for every item in it. Document settings are always saved. Global changes belong in Settings Center. | [SU6] |
| Personalize Web Settings | A group of Quick Settings in a browser (press P in the tree) | Settings for one site: all pages of a domain. Saved at once to a `.jsi` file named after the domain in the `PersonalizedSettings` folder, which can be copied to another user. | [SU17] |
| Voice Adjustment and voice profiles | JAWS window, Options, Voices; Ctrl+Insert+S selects a profile | Synthesizer, language, and per voice context (all, PC cursor, JAWS cursor, keyboard, tutor and message) the person, rate, pitch, volume and punctuation; spelling rate and capital pitch. A profile can be chosen for all applications and for the one in front. | [KS]; [R] §1, §8.8 |
| Speech and Sounds Manager | A Settings Center group; Alt+Insert+S switches schemes for the session | Schemes: per control type, state, attribute, font, color and indentation, speak, play a sound, change voice or ignore. Classic is the default. Training mode names each sound. | [SU18]; [R] §8.8 |
| Dictionary Manager | Insert+D | Pronunciation rules, per application or for all, per language, synthesizer or voice; a rule can play a sound. | [R] §6 |
| Keyboard Manager, Navigation Quick Key Manager | Insert+F2 list | Reassign keystrokes (per application), and reassign the web quick navigation letters (since 2024; Slider has no key by default). | [F1], [F2] |
| Startup Wizard | JAWS window, Help; also on first start | Usage data; rate, punctuation, typing echo; starting JAWS at sign-in; Smart Navigation, Forms Mode, keyboard layout, Num Lock; tutor messages, access keys, verbosity level; braille. Older versions also offered audio ducking here. | [S-wizard], [WFH-wizard], [F3], [F7] |
| Toggle keys | Insert+2, Insert+Shift+2, Insert+S, Insert+X, Insert+Space then S, D, Ctrl+N, Grave, E O, F11; Alt+Ctrl+PageUp and PageDown | Change one setting until JAWS restarts (or, for rate, add Windows to save it). | [KS], [F3], [F5], [F7]; [R] §1 |
| Restore to factory settings | JAWS window, Options | Clears every custom setting; JAWS restarts as newly installed. | [F4] |
| Manage Application Settings | JAWS window, Options | Turns an application's settings off without deleting them, or removes them; clearing DEFAULTS uses the factory defaults. | [B1656]; [R] §9.1 |
| Settings migration, import and export | Installer; JAWS window, Utilities | Since JAWS 2022 an upgrade imports the previous version's settings automatically (a check box at install skips it); settings can be exported and imported, personalized settings included. | [F4]; [R] §9.2 |
| Explore My Settings, Explore Shared Settings | JAWS window, Utilities | Open the user's and the shared settings folders. | [SU17]; [R] §9.1 |
| Insert+Q | | Says which settings file is in use and the application's executable name. | [R] §9.1 |

### 2.2 Scopes and how a value is found

- **Factory defaults** (shared folder), then the **user's defaults for all applications**
  (`DEFAULT.JCF`), then **one application's** file (named after the executable or an alias),
  then for web pages **one site's** `.jsi`. Application files are loaded "on top of" the
  defaults [AD]; a file holds only what differs.
- **Shorter lifetimes** on top: until the focus changes, until JAWS exits (Quick Settings and
  the toggle keys) [SU6].
- **Per document** for some Quick Settings (Word) [SU6]; **per web application** in Settings
  Center (Ctrl+Shift+W loads Google Docs' own settings) [F1].
- **Default-only settings:** the JAWS key, Num Lock's initial state and Flexible Web on or off
  live only in the default file ([R] §9.1).
- **Per-application factory defaults:** some applications ship their own defaults: language
  detection is off in some applications [F1]; grammar detection is on except in Word and
  Outlook [F5].
- **Marks kept per site or page:** custom labels per domain (`.jsi`) [SU19]; PlaceMarkers per
  page address (`.pmi`) or domain [SU15]; Flexible Web rules per page, site or all sites
  [SU16]; Find history per site when Context Specific History is checked [SU13].

## 3. Okur's settings today

> **Since this map was written (2026-10-21),** §7.6's steps 1 and 2 are done: the table is
> `crates/okur-core/src/settings/registry.rs` (`SETTINGS`, 18 settings). The settings dialog's
> controls, the JAWS import that parity replay uses, `okur run --typing-echo` and the new
> `okur run --set <id>=<value>` are made from it, so the dialog and the command line offer
> typing echo's four values. The Okur key and the layout are kept for all applications only. A
> test keeps the table and `Settings` in step. The rows below describe the code before the
> table; the drift that §3.3 lists is gone except `auto_forms_mode` and the verbosity details,
> which are declared but have no control yet.

### 3.1 The pieces

| Piece | What it does | Where |
|---|---|---|
| `Settings` | One typed struct: `voice`, `keyboard`, `live_region_append`, `typing_echo`, `say_all_unit`, `viewer_display`, `auto_forms_mode`, `verbosity`, `confirm_exit`. Defaults are JAWS's. | `crates/okur-core/src/settings.rs:486-524` |
| `Settings::reference_machine` | old-pc's JAWS configuration, for parity replay | `settings.rs:563-581` |
| Layers | Defaults, all applications, one application, one site, the session. Looked up from the most specific down; each layer keeps only what it changes, as serde values by path | `crates/okur-core/src/settings_layers.rs:21-149` |
| Application and site keys | `chrome.exe` is `chrome`; a site is the page's whole host; a `file:` page has none | `settings_layers.rs:206-238` |
| Files | `settings.toml`, `applications/<app>.toml`, `sites/<host>.toml` in `%APPDATA%\Okur` (macOS: Application Support; Linux: XDG). Only changed values are rewritten, comments stay | `crates/okur-platform/src/settings_file.rs:1-148` |
| Own stores | `labels.toml` (custom labels, per site), `placemarkers.toml` (per page address) | `settings_file.rs:150-213` |
| Okur Settings dialog | One dialog for Insert+V and Insert+6: Settings for (all applications, the application in front, its site), then Speech, Keyboard and Reading groups; OK, Cancel, Apply | `crates/okur-core/src/ui/settings.rs:184-318`; keys `crates/okur-core/src/command.rs:939-940` |
| Places | The dialog opens on the application in front; the site is offered when the focus is on a page | `crates/okur-core/src/engine.rs:1205-1232` |
| Session toggles | Insert+Shift+2 (punctuation) and Insert+2 (typing echo) write the session layer, never a file | `engine.rs:1173-1199` |
| Other session state, outside the layers | Insert+N quick keys mode, Insert+Shift+Z, Insert+Z virtual cursor, Insert+Space S speech off, Ctrl+Alt+PageUp/PageDown rate | `crates/okur-core/src/engine/toggles.rs:10-75`; `engine/layer.rs:44-53` |
| Command line | `okur run --engine --voice --rate --volume --keyboard-layout --okur-key --live-region-whole-text --typing-echo` | `crates/okur/src/run.rs:23-80` |
| Replay of JAWS's settings | Reads `DEFAULT.JCF` `[Options] KeyboardType, TypingEcho, SayAllMode, Verbosity`, `[FormsMode] AutoFormsMode`, `[OutputModes]` (5 keys) | `crates/okur-corpus/src/parity.rs:924-981` |
| Insert+Q | "Default settings are loaded" when the application has no file of its own | `crates/okur-core/src/engine/information.rs:100-120` |

### 3.2 The settings, one by one

| Okur setting (TOML path) | Values | In the dialog | Command line | Toggle key | Replayed from JAWS | Where |
|---|---|---|---|---|---|---|
| `voice.synthesizer` | auto, onecore, sapi5, no-speech | yes | `--engine` | | | `settings.rs:200-212` |
| `voice.voice` | a voice name | yes | `--voice` | | | `settings.rs:259-261` |
| `voice.rate` | -10 to 10 | yes (slider) | `--rate` | Ctrl+Alt+PageUp/PageDown, session only, outside the layers | | `settings.rs:262-263`; `toggles.rs` |
| `voice.volume` | 0 to 100 | yes (slider) | `--volume` | | | `settings.rs:264-265` |
| `voice.punctuation` | none, some, most, all | yes | | Insert+Shift+2 (session) | no (the VPF's `Punctuation` is not read) | `settings.rs:217-253`; applied to running text in `crates/okur-platform/src/tts/thread.rs:296` |
| `keyboard.layout` | desktop, laptop | yes | `--keyboard-layout` | | `KeyboardType` | `settings.rs:12-22` |
| `keyboard.okur_key` | insert, caps_lock, scroll_lock | yes | `--okur-key` | | (follows the layout) | `settings.rs:24-41` |
| `typing_echo` | characters, words, both, none | **2 of 4** (Off, Characters) | **2 of 4** | Insert+2 (session, all 4) | `TypingEcho` (only "0" understood) | `settings.rs:126-187`; dialog `ui/settings.rs:40-44`; CLI `run.rs:68-77` |
| `say_all_unit` | line, sentence, paragraph | yes | | | `SayAllMode` | `settings.rs:103-124` |
| `verbosity.level` | beginner, intermediate, advanced | yes | | | `Verbosity` | `settings.rs:283-303` |
| `verbosity.output_modes` | per output type, on or off per level (7 types) | no | | | `[OutputModes]` 5 keys | `settings.rs:322-408` |
| `verbosity.message_length` | long or short per level | no | | | no | `settings.rs:413-436` |
| `auto_forms_mode` | true, false | **no** | | | `AutoFormsMode` | `settings.rs:496-499` |
| `live_region_append` | appended, whole_text | yes (check box) | `--live-region-whole-text` | | | `settings.rs:89-101` |
| `viewer_display` | on_screen, speech_only | yes (check box) | | | | `settings.rs:189-198` |
| `confirm_exit` | true, false | yes (check box) | | | | `settings.rs:504-507` |

Thirteen settings reach the user through the dialog (`ui/settings.rs:197-300`);
`auto_forms_mode` and the verbosity details are only in the files.

### 3.3 What adding one setting costs today

Adding Say All Reads touched six places: the type and field in `settings.rs`, its label table,
control and change branch in `ui/settings.rs`, its words in `dialogs.ftl`, its JCF decoding in
`parity.rs`'s `replay_settings`, and tests in each. A command line option and a toggle key are
two more. Nothing checks that the places agree, and they already drift:
- the dialog and `--typing-echo` offer two of typing echo's four values, while Insert+2 cycles
  all four;
- `auto_forms_mode` has no control at all;
- the verbosity output types and message lengths have no dialog;
- `--live-region-whole-text` can turn the whole text on, not off.

Section 7 proposes one declaration per setting from which all of these are generated.

## 4. The map

Each row is one JAWS setting or settings facility. **Values** gives the default in bold where
a source states it. **Okur**: **Built** (Okur has the setting, with its values and the scope
JAWS gives it, reachable by the user), **Partial** (the behavior or the setting exists, but
values, scope, a dialog control or persistence are missing; or Okur behaves as JAWS's default
with no way to change it), **Missing** (neither). §4.13 and §4.14 list settings that are not counted (§6).

### 4.1 Settings facilities

| # | JAWS | Values, default | Scope | Source | Okur | Okur today |
|---|---|---|---|---|---|---|
| 1 | Settings Center: tree of groups, description, Apply/OK/Cancel | | all, per application | [SU6] | Partial | One flat dialog with three group boxes (`ui/settings.rs:184-318`), opened by Insert+6 and Insert+V; no tree, no description |
| 2 | Settings Center search box (Ctrl+E) | | | [SU6], [SU7] | Missing | |
| 3 | "Settings for": Application combo box, Ctrl+Shift+D for Default (All Applications) | opens on the application in front | | [SU6] | Partial | "Settings for" with all applications, the application in front and its site (`engine.rs:1205-1232`); no Ctrl+Shift+D, no other applications |
| 4 | Last Changed Settings group | | | [SU7] | Missing | |
| 5 | Quick Settings (Insert+V): the application in front's own groups first | | per application, per document | [SU6] | Partial | Insert+V opens the same dialog as Insert+6 (`command.rs`, `QuickSettings`); no application-specific groups |
| 6 | Lifetimes: saved, until JAWS exits, until focus changes; per item and per group | **saved** | | [SU6] | Partial | A session layer (until Okur exits) used only by Insert+2 and Insert+Shift+2 (`settings_layers.rs:31-32`, `engine.rs:1173-1199`); no until-focus-changes; no choice in the dialog |
| 7 | Per-application settings files | only what differs | per application | [AD], [SU6] | Built | `applications/<app>.toml`, layered (`settings_layers.rs:54-69`, `settings_file.rs:1-148`) |
| 8 | Personalize Web Settings (per domain, `.jsi`, shareable) | | per site | [SU17] | Partial | `sites/<host>.toml` through "Settings for" (`settings_layers.rs:225-238`); any setting can be set per site, where JAWS documents only some (web settings); no Personalize Web Settings group; no notice on arrival |
| 9 | Per-document settings (Quick Settings in Word) | | per document | [SU6] | Missing | |
| 10 | Per-web-application settings (Settings Center, Ctrl+Shift+W in Google Docs) | | per web application | [F1] | Missing | Okur has application modules (`crates/okur-core/src/apps/`), no settings layer for them |
| 11 | Restore to factory settings | | everything | [F4] | Missing | Deleting `%APPDATA%\Okur` by hand |
| 12 | Manage Application Settings: turn an application's settings off, or remove them | | per application | [B1656]; [R] §9.1 | Missing | |
| 13 | Settings migration on upgrade; Import/Export (Utilities menu: all settings or chosen categories to one backup file; import merges or replaces) | migration **on** | | [F4], [S-transfer]; [R] §9.2 | Missing | TOML files can be copied; no import or export command |
| 14 | Startup Wizard: usage data; speech (rate, punctuation, typing echo); starting JAWS; common options (Smart Navigation, Forms Mode, keyboard layout, Num Lock); verbosity (tutor messages, access keys, level) | | all | [S-wizard], [WFH-wizard], [F3], [F7] | Missing | |
| 14a | Starting JAWS: at the sign-in screen; after sign-in for all users (**off**) or for this user (**always**); run from the system tray (**off**) | | all | [S-wizard], [WFH-wizard] | Missing | An installer that starts Okur at sign-in is being tested (README "Not yet") |
| 15 | Explore My Settings / Explore Shared Settings | | | [SU17] | Missing | |
| 16 | Insert+Q: which settings are loaded | | | [R] §9.1 | Built | "Default settings are loaded" and the application, spelled (`engine/information.rs:100-120`) |
| 17 | Run JAWS Manager (Insert+F2): the list of managers | | | [SU6]; [`jaws-dialogs-and-visual-features.md`](jaws-dialogs-and-visual-features.md) | Missing | |

### 4.2 Speech and voice

| # | JAWS | Values, default | Scope | Source | Okur | Okur today |
|---|---|---|---|---|---|---|
| 18 | Synthesizer | | all, per application (profile) | [R] §1 | Built | `voice.synthesizer` (`settings.rs:200-212`) |
| 19 | Voice (person) | | per voice context | [R] §1 | Partial | `voice.voice`, one voice, no contexts |
| 20 | Rate | | per voice context | [R] §1 | Partial | `voice.rate`, -10 to 10, no contexts |
| 21 | Pitch | | per voice context | [R] §1 | Missing | `VoiceSettings` has none (`settings.rs:256-269`) |
| 22 | Volume | | per voice context | [R] §1 | Partial | `voice.volume`, no contexts |
| 23 | Voice contexts: all, PC cursor, JAWS cursor, keyboard, tutor and message (the files also have menus and dialogs) | | | [R] §1, §8.8; old-pc's `.VPF` in evidence provenance | Missing | |
| 24 | Voice profiles, chosen for all applications and for one (Ctrl+Insert+S) | | all, per application | [KS]; [R] §9.1 | Partial | Voice settings can differ per application through the layers; no named profiles, no Ctrl+Insert+S |
| 25 | Voice aliases (capital, spelling, heading voices) | | | [R] §8.8 | Missing | |
| 26 | Spell rate change percent (Voice Adjustment) | negative values | profile | [S-voice]; [R] §4 | Missing | |
| 27 | Rate for the moment (Alt+Ctrl+PageUp/PageDown) | | until Insert+Escape (JAWS: lost on Alt+Tab) | [KS], [S-rate] | Built | `toggles.rs` (`change_rate`) |
| 28 | Rate saved (Alt+Windows+Ctrl+PageUp/PageDown) | | | [KS] | Missing | |
| 29 | Speech Mode: **Full Speech** or Less Speech; Less Speech: Mute or **Speech On Demand**; Insert+Space then S | | all, per application | [F8], [KS] | Partial | Insert+Space, S mutes until pressed again (`engine/layer.rs:48-53`); no Speech On Demand, no saved mode |
| 30 | Speech On Demand options: **Allow Reading Commands**, Allow Mouse Echo, Allow Typing Echo (cleared), JAWS sounds | | | [F8], [F6]; [R] §2 | Missing | |
| 31 | Audio ducking: "Lower the volume of other programs while JAWS is speaking"; Insert+Space then D for the session | off in 2019 [F7]; on in a later support article [S-ducking] | default file | [F7], [KS], [S-ducking] | Missing | |
| 31a | Volume layer (Insert+Space, V): JAWS's own volume, the sound splitter or balance (JAWS in one ear), the sound card | | session | [S-splitter] | Missing | |
| 32 | Keep audio devices awake by playing silence | **off** | | [F6] | Missing | |
| 33 | Detect Language (based on markup, on text, on AI predictions, off); Generalize Dialect; Detect Keyboard Language; Ctrl+Windows+L | **based on markup** | all, per application (off by default in some) | [F1], [F6], [KS], [TAG]; [R] §8.7 | Missing | |
| 34 | Enable Speech History | **on**; 50 items in one help topic, 500 in another and in the 2021 notes | | [R] §7; [`everyday-use-gaps-2026-10-19.md`](../backlog/everyday-use-gaps-2026-10-19.md); [SE-2021] | Partial | Always on, 50 items (`crates/okur-platform/src/tts/thread.rs:32-33`) |

### 4.3 Punctuation, symbols and pronunciation

| # | JAWS | Values, default | Scope | Source | Okur | Okur today |
|---|---|---|---|---|---|---|
| 35 | Punctuation level; Insert+Shift+2 cycles it | None, Some, **Most**, All | all, per application, per voice context, per language; the key until restart | [F5]; [R] §1 | Partial | `voice.punctuation` at every layer, applied to running text (`settings.rs:214-253`, `tts/thread.rs:296`); Insert+Shift+2 (`engine.rs:1173-1186`); no voice contexts; replay does not read the profile's `Punctuation` |
| 36 | Customize Punctuation: each symbol's description and level; Restore Defaults; Advanced | | all, per application | [R] §1 | Missing | Symbol names and levels are tables in code (`crates/okur-core/src/pronounce.rs`) |
| 37 | Filter Repeated Characters | **Three**, Four, Five, Six, All, Count | | [R] §1 | Partial | Fixed at three (`pronounce.rs:231-250`) |
| 38 | Announce special symbols when; character value in hex; as multibyte sequence | | | [R] §1, §4 | Missing | |
| 39 | Dictionary Manager (Insert+D) | | all, per application; per language, synthesizer, voice | [R] §6 | Missing | One fixed respelling list (`pronounce.rs`, `RESPELLINGS`) |
| 40 | Use Dictionary | **on** | | [R] §6 | Missing | |

### 4.4 Typing and keyboard

| # | JAWS | Values, default | Scope | Source | Okur | Okur today |
|---|---|---|---|---|---|---|
| 41 | Typing Echo; Insert+2 cycles it | Off, **Characters**, Words, Characters and Words | all, per application; the key for the session | [R] §2 | Partial | All four in the engine and on Insert+2 (`settings.rs:126-187`, `engine.rs:1188-1199`); the dialog and `--typing-echo` offer two (`ui/settings.rs:40-44`, `run.rs:68-77`); replay understands only "0" (`settings.rs:144-153`) |
| 42 | Manage Key Labels: each key's spoken label and Speak, Mute or Always Speak (Alt+T); Tab muted since 2019; muting Letters silences letters in Characters echo only | | | [F7], [S-space]; [R] §3 | Missing | |
| 43 | Typing Interrupt (`DEFAULT.JCF` `[Options]` `TypingInterrupt`, shipped 1: "pressing a key that is not bound to macro will interrupt speech") | **on** | | [R] §3; the shipped `DEFAULT.JCF` (old-pc, 2026-10-23) | Missing; round 264 (914 on, 915 off: X typed during a Say All in Chrome's text area) inconclusive, both settings reading on in some trials and stopping in others; no trial settled, nothing imported | |
| 43a | Reading Interrupt (`[Options]` `ReadingInterrupt`, shipped 1: "pressing a key that is bound to a macro will interrupt speech") | **on** | | the shipped `DEFAULT.JCF` (old-pc, 2026-10-23) | Missing. Recorded (round 264): on, Insert+T during a page's Say All stops it (916, imported); off, Insert+T's answer is said in the middle of the reading ("Title is ... heading is City Library Expands Evening Hours") and the Say All reads on to the page's end (917, every trial; not imported, its Say All never settled) | |
| 44 | Key Repeat (`[Options]` `KeyRepeat`, shipped 1, where the help gives it cleared) | **off** in the help, 1 in the shipped file | | [R] §3; the shipped `DEFAULT.JCF` | Missing | |
| 45 | Insert Key Mode (Insert as a sticky key) | **off** | | [F3]; [R] §3 | Missing | |
| 46 | Keyboard Layout | **Desktop**, Laptop (the Startup Wizard also lists Kinesis) | all | [S-wizard]; [R] §1 | Built | `keyboard.layout`: Desktop, Laptop (`settings.rs:12-22`) |
| 47 | JAWS Key for current layout (Okur: the Okur key) | Insert; CapsLock; ScrollLock; Muhenkan on Japanese keyboards | default file only | [F2]; [R] §9.1 | Built | `keyboard.okur_key`: Insert, Caps Lock, Scroll Lock (`settings.rs:24-41`); not restricted to the global scope |
| 48 | Initial State of Num Lock Key | **turn off at startup**, turn on at startup, don't modify | default file only | [S-numlock], [S-wizard]; [R] §9.1 | Missing | |
| 49 | Use Virtual Key Codes | **off** (scan codes) | | [R] §11 | Missing | |
| 50 | Keyboard Manager (Insert+F2 list): reassign keystrokes | | all, per application | [F1]; [`jaws-dialogs-and-visual-features.md`](jaws-dialogs-and-visual-features.md) | Missing | One key table in code (`command.rs`) |
| 51 | Navigation Quick Key Manager: reassign quick navigation letters | Slider unassigned | | [F2] | Missing | |
| 52 | Allow web application reserved keystrokes | | per application, per site | [F7]; `Chrome.JCF` in evidence provenance | Missing | |
| 53 | Prompt to Confirm When Exiting | **on** (JAWS 2025 help, `settings.rs:504-506`; one Startup Wizard walkthrough shows it cleared [WFH-wizard]) | all | [F7] | Built | `confirm_exit` (`settings.rs:504-507`); in the dialog |

### 4.5 Text processing and proofing

| # | JAWS | Values, default | Scope | Source | Okur | Okur today |
|---|---|---|---|---|---|---|
| 54 | Indicate capitalization (Caps Indicate) | **On character**; character and word; character, word and line; never | all, per application | [R] §5 | Missing | Capitals are raised when read alone, always |
| 55 | Uppercase pitch change percent (Voice Adjustment; the capital voice aliases) | 0 to 100, **20** | profile | [S-pitch]; [R] §5 | Partial | Fixed at SAPI +4 (`pronounce.rs:100-103`) |
| 56 | Mixed Case Processing | **on** | | [R] §6 | Partial | Always on (`pronounce.rs:125-139`, `split_words`) |
| 57 | Number Processing | **Controlled by synthesizer**, single digits, pairs, full numbers, controlled by JAWS (2025) | | [F1]; [R] §6 | Missing | |
| 58 | Speak Single Digits if Number Contains; If Number Contains Dashes | **five or more**, six, seven, eight, by synthesizer; dashes **on** | | [R] §6 | Partial | Fixed at five or more, and dashes (`pronounce.rs:129-130`, `single_digits`) |
| 59 | Speak Dollars | **off** | per application (the lesson advises Excel) | [R] §6 | Missing | |
| 60 | Numeric Date Processing | **No translation**, some, extended | | [R] §6 | Missing | |
| 61 | Smart Word Reading | **on** | | [R] §4 | Missing | |
| 62 | Spell Alphanumeric Data | **Off**, spell, spell phonetically | | [R] §4 | Missing | |
| 63 | When pausing after character navigation, speak character phonetically | **after about 1.5 s**; other delays; Never | | [F7], [S-phonetic]; [R] §4 | Missing | |
| 64 | Spell Phonetically Always (Quick Settings: Text Spell) | **alphabetically** | | [R] §4 | Partial | Spelling and phonetic spelling exist (`pronounce.rs:44`, `speech.rs:953-962`); no setting |
| 65 | Indentation: Say Indented Characters (`[Options] Indentation`, 0 by default); Quick Settings' Indentation | **Ignore** | | [R] §8.3 | Partial (2026-10-23, `62b3ddb`): the setting `indentation`, imported from JAWS's key; "Left Margin" before a page line at the margin on Say Line and as a Say All starts (729); indented lines not recorded yet | `settings.rs`, `engine/commands.rs` (`with_margin_mark`) |
| 66 | Indicate New Lines When | | | [R] §8.2 | Missing | |
| 67 | Detect Spelling While Navigating | Off, play sound, **say misspelled** | all, per application | [F7], [DS-spell]; [R] §8.5 | Partial | "Misspelled" said in text fields (`editing.rs:170-188`); no setting |
| 68 | Detect spelling while typing; grammar detection; error count; apply during Say All | sound **on**; grammar **on** except Word and Outlook; count **off**; Say All **off** | | [F5]; [R] §8.5 | Missing | |
| 69 | Comment, inserted, deleted, marked and suggested text announcements (Proofing) | | | [F4], [F5] | Missing | |
| 70 | Text Analyzer: **off**, indicate with a sound, speak the count, describe the inconsistencies; what it checks (unmatched brackets and quotes, stray punctuation, extra spaces, missing capitals, inverted case, font changes); Insert+Space, A for the moment | off | all, per application | [WFH-ta] | Partial | Insert+Tab after a text field names its inconsistencies (`crates/okur-core/src/text_analysis.rs`); no setting, no automatic indication |

### 4.6 Verbosity

| # | JAWS | Values, default | Scope | Source | Okur | Okur today |
|---|---|---|---|---|---|---|
| 71 | User Verbosity level | **Beginner**, Intermediate, Advanced | all, per application | [T1]; [R] §7 | Built | `verbosity.level`, in the dialog (`settings.rs:283-303`); replayed from `Verbosity` |
| 72 | Configure Verbosity Levels: per level, the output types spoken; the 7 Okur models (access key, hint, list position and count, description, control group name, select, JAWS messages) | per type and level, JAWS's shipped modes | all, per application | [F1]; [R] §7; [`messages-and-verbosity.md`](messages-and-verbosity.md) | Partial | `verbosity.output_modes` with JAWS's shipped modes (`settings.rs:322-408`); 5 replayed from `[OutputModes]`; no dialog, files only |
| 73 | The other output types (about 18): control name, control type, item state, dialog name and text, document name, position information, item number, selected item, status, screen messages, tool tips, toasts, help balloons, smart help, error messages, application start messages, mouse echo | per type and level | all, per application | [R] §7 | Missing | Always spoken where Okur speaks them |
| 74 | Message length per level | **Long** at Beginner, short above | all, per application | [R] §7 | Partial | `verbosity.message_length`, short forms in the catalog; no dialog |
| 75 | Tutor Messages: **announce menu and control help**, turn it off, announce custom messages only | announce | all, per application | [S-wizard]; [R] §7 | Partial | The `hint` output type, on at every level; no tutor setting of its own |
| 75a | Access Keys: off, **speak all**, menus only, dialogs only | speak all | all, per application | [S-wizard] | Partial | The `access_key` output type, on or off per level (`settings.rs:329-331`); no menus-only or dialogs-only |
| 75b | Say item state first (a control's state before its name) | | all, per application | [S-state] | Missing | |
| 75c | Progress bar announcements: off, at an interval, every change | reportedly every 5 s (forum, unconfirmed) | all, per application | [G-progress]; [SU6] (progress bars as a lifetime example) | Missing | |
| 76 | Smart help messages | | | [R] §7 | Missing | |
| 77 | Focus loss announcement | | default file | [R] §9.1 | Missing | |
| 78 | Announce multi-line edit | **off** | | [F7] | Missing | |
| 79 | Enable accessible notification events (Office business bars and the like) | **on** | all, per application | [F5] | Missing | |

### 4.7 Say All

| # | JAWS | Values, default | Scope | Source | Okur | Okur today |
|---|---|---|---|---|---|---|
| 80 | Say All Reads | line with pauses, **line without pauses**, sentence, paragraph | all, per application | [R] §8.1 | Partial | `say_all_unit`: line, sentence, paragraph, in the dialog, replayed from `SayAllMode` (`settings.rs:103-124`); the two line variants are one; `LinePauses` is in evidence provenance and not replayed |
| 81 | Indicate caps during Say All | **off** | | [R] §5 | Missing | |
| 82 | Say blank lines during Say All | **off** | | [R] §8.2 | Missing | |
| 83 | Say All scheme: with sounds, text only, no change | | | [R] §8.1 | Missing | |
| 84 | Navigation Quick Keys: on, off, Say All only (Insert+N cycles them) | **on** | all, per application, per site | [R] §9.2; [`jaws-quick-navigation-keys.md`](jaws-quick-navigation-keys.md) | Partial | Insert+N cycles the three for the session (`engine/toggles.rs:10-29`); no saved setting |

### 4.8 Echo and the screen

| # | JAWS | Values, default | Scope | Source | Okur | Okur today |
|---|---|---|---|---|---|---|
| 85 | Screen Echo (Insert+S cycles it for the moment; `[Options]` `ScreenEcho`: 0 off, 1 highlighted, 2 all, shipped 1) | echo no text, **echo highlighted text**, echo all text | all, per application | [R] §8.6; the shipped `DEFAULT.JCF` | Partial: the setting `screen_echo` (read from `ScreenEcho`, file only) and Insert+S, which says "All", "None", "Highlighted" from the default (913/s1-s3). Recorded (round 268): a classic console's new output after Enter is read at highlighted and all, not at none (962-964); Chrome's live regions are read alike at none, all and the default (960, 961). Built: the console's output (`screen_echo::new_rows`, `ec8d05f7`), to compare once round 273 records 962-964 again with its tracer. Highlighted text alone (what "highlighted" adds over none outside consoles) is not recorded | |
| 86 | Mouse Echo: enable (**off**); interrupt; instant; delay; unit (character, word, **line**, paragraph); speak the item's description; speak its control type and state. Insert+Space, E, then O toggles it and C, W, L, P choose the unit | off | all, per application | [KS], [F1], [S-mouse], [LN-mouse], [SAOMAI] (JAWS 18 defaults) | Missing | |
| 87 | Show Virtual Viewer on Screen | **on** | | [`jaws-dialogs-and-visual-features.md`](jaws-dialogs-and-visual-features.md) | Built | `viewer_display`, in the dialog (`settings.rs:189-198`) |
| 88 | Screen Shade (Insert+Space, F11; Quick Settings) | | | [KS], [F8] | Missing | |
| 89 | Reading highlight (visual tracking) | | | [`jaws-dialogs-and-visual-features.md`](jaws-dialogs-and-visual-features.md) §3 | Missing | |

### 4.9 Web, HTML and PDF

| # | JAWS | Values, default | Scope | Source | Okur | Okur today |
|---|---|---|---|---|---|---|
| 90 | Web Verbosity Level | High, **Medium**, Low | all, per browser, per site | [SU7] | Missing | Structure is announced one way; old-pc's `Chrome.JCF` `VirtualCursorVerbosityLevel=2` is recorded, not replayed |
| 91 | Configure Web Verbosity Levels: per level, which structures are announced (each region type, frames, lists, tables, block quotes, articles, groups; descriptions since 2025) | Medium: navigation and main regions only; frames at High only; tables at every level | all, per browser | [SU7], [F1], [F8] | Missing | Always said: every region type, lists with nesting, block quotes, articles, groups, frames (`crates/okur-core/locales/en-US/page.ftl`, `speech.ftl`) |
| 92 | Text Links Show Using | **Screen Text**, Title, Alt attribute, Longest, Custom Search | all, per application, per site | [SU12] | Partial | Screen text, or the aria-label (`linearize.rs:753-821`); no setting |
| 92a | Filter Consecutive Duplicate Links (a graphic link and a text link to one address) | on when added (JAWS 11); reported off for Chrome and Firefox today | all, per application | [DS-web], [WebAIM-dup] | Missing | Both links are lines |
| 93 | Untagged Graphical Links Show | image source, URL (more choices) | | [SU12] | Missing | |
| 94 | Graphics Show (Graphics Verbosity; "Include Images") | **labeled** (since 2021), all, none | all, per application, per site | [SU12], [F5], [DS-web] | Partial | A graphic is a line of its own with its name (`linearize.rs:678-690`); no setting |
| 95 | Identify Same Page Links, Say Link Type (named in older JAWS's HTML options; not confirmed for JAWS 2025) | | | [UAAG-jfw] | Partial | Same page links and mail links always identified (`linearize.rs:753-821`) |
| 96 | Heading announcement; Heading Navigation Using 1-6 | | all, per application, per site | [SU5], [SU17] | Missing | "heading level N" always (`roles.ftl`, `role-heading-level`) |
| 97 | Inline Frames Show | **On** | per site | [SU10] | Missing | |
| 98 | Page Refresh (filter) | refresh as needed, every 60 seconds, suppress all refreshes (Quick Settings also offers Off) | per application, per site | [SU12], [S-refresh] | Missing | Updates are applied as they come (`engine/page.rs`, `refresh_stale_page`) |
| 99 | Document and web pages automatically read when loaded | **on** in JAWS 2025; off by default from the December 2025 update (JAWS 2026) | all, per application | [F8], [SE-2026] | Partial | Always (`engine/page.rs:126-144`, `engine/say_all.rs:66-77`) |
| 100 | Smart Glance: Indicate highlights (count on page load, sound while navigating, both, off), Sensitivity (normal, high), Include links, headings, form fields | count on page load; fields only when invalid or required | all, per application | [F3], [F1], [SE-2023] | Missing | Okur says the page summary, not the Smart Glance sentence (`engine/page.rs:127-129`) |
| 101 | Skip Blank Lines While Navigating | **on** | | [F6] | Partial | The virtual buffer has no blank lines; no setting |
| 102 | Lines per page (PageUp and PageDown) | **24** | | [W3C-lines]; [R] §8.6 | Partial | Constant 24 (`cursor.rs:6-8`) |
| 103 | Maximum line length (applies to Screen Layout) | 150, per Okur's code comment (`[HTML] MaxLineLength`) | | [R] §8.6 | Partial | Constant 150 (`linearize.rs:72-74`) |
| 104 | Document Presentation Mode (Select Layout) | **Simple Layout**, Screen Layout | all, per browser | [SU8] | Partial | Simple layout only |
| 105 | Smart Navigation (Insert+X for the moment) | **off**, controls, controls and tables, tables (2023) | all, per application | [F3], [KS], [S-wizard]; [R] §4 | Partial: the setting `smart_navigation` (read from `[HTML] SmartNavigation`, a bit pattern: 1 a control, 2 a table cell, as units of the virtual cursor's character and word keys; file only) and Insert+X, which alternates "Smart Navigation: Controls and Tables" and "Smart Navigation: Off" (913/s5-s8). Recorded (round 268, `SmartNavigation=3`): Left and Right step over a control as one unit, said as its line says it ("‹Edit› Izmir", "·Subscribe ‹check box› ‹not checked›", the link whole), and over a label ("City"), the page's title, a region's start and a list's start; plain text and a combo box's value go by character (955, 954); Ctrl+Left and Ctrl+Right take the same units; in a table the arrows move by cell, "Revenue | $18,000 | Column 2 of 4", "Q1 | Row 2 of 4", and past its edge to "table end" (956); Down said nothing once (955/s12), as if the lines were the screen's (Screen Layout). Not built: the units, the table's arrows, the layout | |
| 106 | Navigation Quick Key Delay (forms mode after pausing on a field) | **Never**, 1 to 5 s | per site | [SU9] | Missing | |
| 106a | Virtual cursor on or off remembered per browser tab | **on** (since 2020) | all, per browser | [SE-2020], [S-vpc] | Missing | Insert+Z holds for the session, across tabs (`engine/toggles.rs:45-56`) |
| 107 | Announce live region updates (in every application since 2022) | **on** | all, per application, per site | [F4], [SE-2022] | Missing | Always spoken, by politeness (`engine/page.rs:845-892`) |
| 108 | ARIA live region text filter (Windows+Ctrl+Dash) | | | [KS] | Missing | |
| 109 | Custom Page Summary (PlaceMarkers) | **Do Nothing** | per application, per site | [SU15] | Missing | |
| 110 | Indicate text inside INS and DEL elements | **on** | | [F4] | Missing | |
| 111 | Flexible Web: Enable Flexible Web; rules for a page, a site or all sites | **on** | default file only; rules per page, site, all | [SU16]; [R] §9.1 | Missing | In the active plan |
| 112 | Custom labels per domain; Label Manager (2025) | | per domain | [SU19], [F1] | Partial | Labels per site, per page without one (`crates/okur-core/src/labels.rs`, `labels.toml`); no Label Manager |
| 113 | PlaceMarkers per page or domain; temporary PlaceMarker | | per page, per domain | [SU15] | Partial | Saved per page address, the temporary one, K and Shift+K (`crates/okur-core/src/placemarkers.rs`, `placemarkers.toml`); the PlaceMarker List is being built |
| 114 | JAWS Find: Context Specific History, Maintain History, direction, Ignore Case | history **on**, per site **on**, forward, ignore case **on** | | [SU13] | Partial | The dialog has them (`crates/okur-core/src/ui/find.rs`); history kept in memory only, 20 items |
| 115 | Graphics Labeler (Insert+G; an AI labeler in the 2025 Early Adopter Program, Ctrl+Insert+G auto labeler) | | per application, per site | [F1], [KS] | Missing | |

### 4.10 Forms mode

| # | JAWS | Values, default | Scope | Source | Okur | Okur today |
|---|---|---|---|---|---|---|
| 116 | Forms Mode: Manual (Enter turns it on), Semi-Auto (only Tab and Shift+Tab onto a field turn it on; since JAWS 16), **Auto** | Auto | all, per browser, per site | [SU9], [F1], [F8], [ADG], [S-wizard] | Partial | `auto_forms_mode` true or false, replayed from `AutoFormsMode` (`settings.rs:496-499`); not in the dialog; no Semi-Auto |
| 117 | Forms mode sounds: Use Sound (**checked**); Select Sound for Entering and for Leaving Forms Mode | | all, per browser | [SU9] | Missing | Okur has no sound output |

### 4.11 Tables

| # | JAWS | Values, default | Scope | Source | Okur | Okur today |
|---|---|---|---|---|---|---|
| 118 | Table Titles Announce | **Only Marked Headers**, Off, Row, Column, Both Row and Column | per application, per document | [SU8] | Partial | Marked headers, said when the column or row changes (`cursor.rs:1299-1353`); no setting |
| 119 | Cell Header and Content: headers before or after the cell | **before** | per application | [F3] | Partial | Before, always |
| 120 | Table cell coordinates announcement | on | per application | [F4], [SE-2021] | Partial | Always said |
| 120a | Layout tables: treat data tables only, or all tables, as tables (name unconfirmed) | data tables only | | a search-engine excerpt of a Freedom Scientific lesson, not opened; [SU8] explains layout against data tables | Missing | Unconfirmed: read the Settings Center tree on old-pc first |

### 4.12 Notifications, sounds and the rest

| # | JAWS | Values, default | Scope | Source | Okur | Okur today |
|---|---|---|---|---|---|---|
| 121 | Notification History (Insert+Space, N; the latest one with Insert+Space, Shift+N) and notification rules: contains, begins with, ends with or matches a pattern; change the text, play a sound or mute; named, ordered, each can be disabled | | all | [F4], [F3], [TTT177] | Missing | |
| 122 | Toast announcements on or off (Insert+Space, Ctrl+N), until JAWS restarts | on | session | [F3] | Missing | |
| 123 | Volume notification announcements (Insert+Space, Grave) | **off** | | [F5] | Missing | |
| 124 | Speech and Sounds schemes: pages for control type, control state, attributes, font name, font size, color, indentation and misc (cap, all caps, new line); each item speaks (with other words), plays a sound, changes voice or is ignored; "say window type first" and "say window state first"; training mode | **Classic** | all, per application; session (Alt+Insert+S) | [SU18], [TAVIP], [S-caps]; [R] §8.8 | Missing | Okur has no sound output; role and state words are fixed in the catalog |
| 125 | Automatic notification of updates; seamless script updates | | all | `DEFAULT.JCF` in evidence provenance; [F2] | Missing | |
| 126 | Show the Startup Wizard when JAWS starts | | all | `DEFAULT.JCF` `DisplayStartupWizard` in evidence provenance | Missing | |

### 4.13 Braille (listed only; not counted)

Okur has no braille output yet (README). JAWS's braille settings, for the registry's future
groups: Braille Mode (Line, **Structured**, Speech Output or Speech History); flash messages
(**on**); reverse panning buttons; word wrap; Active cursor follows the braille display and the
reverse; Pan Text by Paragraph (2021, **off**); status cells (placement, the time, 24-hour
format); translation (language, output and input grade, language profiles, the Grade 2
translator); font attribute marks (2025); Structured Mode's attribute representation; Split
Braille views and buffer unit; wrapped or cropped reading (2025); math code (Nemeth or UEB);
automatic detection of displays over Bluetooth. Sources: [WFH-wizard], [F1], [F4], [F5],
[SE-2021], [ETS-braille].

### 4.14 Assistive utilities (later; not counted)

Picture Smart's language, Face in View's start options, FSCompanion, the Voice Assistant (wake
word, microphone), the Message Center's categories, Convenient OCR: each has settings in
Settings Center or its own dialog [F1], [F2], [F5]. They come with the assistive utilities
([`assistive-ai-and-remote-proposal.md`](assistive-ai-and-remote-proposal.md)).

JAWS 2026, beyond the parity target, adds a cloud settings sync (Early Adopter Program), turns
automatic reading on page load off by default, and adds a Unicode special character
translation option [SE-2026].

## 5. What Okur decides by itself today that JAWS lets the user change

Each of these is right at JAWS's default, or at old-pc's configuration, so parity holds; but a
JAWS user can change it and an Okur user cannot. The row numbers point into §4.

| Behavior fixed in Okur | Where | JAWS's setting (§4 row) |
|---|---|---|
| A page that loads says its title, the "Page has ..." summary, then reads from the top | `crates/okur-core/src/engine/page.rs:126-144`, `page_summary` `:148-177`; `engine/say_all.rs:66-77` | automatic reading (99); Smart Glance indication (100); Custom Page Summary (109) |
| Every region type, list (with nesting level), block quote, article, group, dialog and frame is announced at its start and end | `linearize.rs`; words in `locales/en-US/page.ftl:1-40`, `speech.ftl` `frame` | Web Verbosity Level and its per-level elements (90, 91); Inline Frames Show (97) |
| Headings say "heading level N" | `locales/en-US/roles.ftl` `role-heading-level`; `linearize.rs` | heading announcement (96) |
| A link is read by its text (or aria-label); same page links, mail links and visited links are always marked | `linearize.rs:753-821` | Text Links Show Using (92); link identification (95) |
| A graphic is a line with its name and "Graphic" | `linearize.rs:678-690` | Graphics Show (94); Untagged Graphical Links Show (93) |
| A table cell move says the marked header when the column or row changes, before the cell, and the coordinates | `cursor.rs:1299-1353`; `locales/en-US/tables.ftl` | Table Titles Announce (118); Cell Header and Content (119); coordinates (120) |
| Live regions are always spoken, assertive ones interrupting | `engine/page.rs:845-892` | Announce live region updates (107); text filter (108) |
| Page updates are applied as they arrive | `engine/page.rs` (`refresh_stale_page`, `handle_page_refreshed`) | Page Refresh (98) |
| PageUp and PageDown move 24 lines; page text wraps at 150 characters | `cursor.rs:6-8`; `linearize.rs:72-74` | Lines per page (102); Maximum line length (103) |
| Simple layout only: a table cell is a line of its own | `linearize.rs` | Document Presentation Mode (104) |
| Forms mode turns on by Enter, and with Auto Forms Mode by arrows and Tab onto a field; no sound marks it | `engine/commands.rs:636-660, 736-760`; `engine/focus.rs:178-195, 452-470` | Forms Mode Manual / Semi-Auto / Auto (116); forms mode sounds (117); quick key delay (106) |
| Words split at capitals and digits; numbers of five or more digits, or with dashes, read digit by digit; runs of a symbol cut to three | `pronounce.rs:125-139, 231-250, 309, 346` | Mixed Case Processing (56); Speak Single Digits (58); Filter Repeated Characters (37) |
| A capital read alone is raised by SAPI pitch +4; nothing else marks capitals | `pronounce.rs:100-112` | Caps Indicate (54); capital pitch (55); caps in Say All (81) |
| Symbol names and their punctuation levels | `pronounce.rs` (`SYMBOL_NAMES`, `PUNCTUATION`) | Customize Punctuation (36) |
| "spacebar" is respelled "space Bar" | `pronounce.rs:282` (`RESPELLINGS`) | Dictionary Manager (39) |
| "Misspelled" is said in a text field on a marked word | `editing.rs:170-188` | Detect Spelling While Navigating (67) |
| Hints ("To activate press Enter.") at every level unless the settings file turns the `hint` type off | `settings.rs:381-393` | Tutor messages (75) |
| The Speech History keeps 50 utterances | `crates/okur-platform/src/tts/thread.rs:32-33` | Enable Speech History (34) |
| Quick keys mode, the virtual cursor and speech on or off last until Okur exits and start "on" | `engine/toggles.rs:10-56`; `engine/layer.rs:48-53` | Navigation Quick Keys (84); Speech Mode (29) |
| The rate set with Alt+Ctrl+PageUp/PageDown is never saved | `engine/toggles.rs` (`change_rate`) | saved rate change (28) |
| JAWS Find's history is forgotten when Okur exits | `ui/find.rs:13`; `engine/find.rs` | Maintain History (114) |

Not JAWS settings, so not in the map, but tunable values that a registry would expose for
testing: the double-press interval, 350 ms (`engine.rs:21`); a selection read whole up to 1000
characters (`cursor.rs:10-16`); an element list item cut at 254 characters
(`element_lists.rs:23`). Okur's own setting with no JAWS twin: `live_region_append`
(Appended, Okur's default, or Whole Text, JAWS's behavior; `settings.rs:89-101`).

## 6. Counts

Counted from §4's tables (rows 1 to 126, with the lettered rows; §4.13 and §4.14 left out):

| Group | Built | Partial | Missing | Rows |
|---|---|---|---|---|
| 4.1 Settings facilities | 2 | 5 | 11 | 18 |
| 4.2 Speech and voice | 2 | 6 | 10 | 18 |
| 4.3 Punctuation, symbols, pronunciation | 0 | 2 | 4 | 6 |
| 4.4 Typing and keyboard | 3 | 1 | 9 | 13 |
| 4.5 Text processing and proofing | 0 | 6 | 11 | 17 |
| 4.6 Verbosity | 1 | 4 | 7 | 12 |
| 4.7 Say All | 0 | 2 | 3 | 5 |
| 4.8 Echo and the screen | 1 | 0 | 4 | 5 |
| 4.9 Web, HTML and PDF | 0 | 11 | 17 | 28 |
| 4.10 Forms mode | 0 | 1 | 1 | 2 |
| 4.11 Tables | 0 | 3 | 1 | 4 |
| 4.12 Notifications, sounds and the rest | 0 | 0 | 6 | 6 |
| **All** | **9** | **41** | **84** | **134** |

Of the 41 partial rows, 19 are a behavior Okur has at JAWS's default with no setting at all
(rows 34, 37, 55, 56, 58, 64, 67, 70, 92, 94, 95, 99, 101-104, 118-120; most are in §5). The
rest have the setting, with values, a scope, a dialog control or persistence missing.

## 7. A settings design that is easy to extend

The extension SDK proposal already sketches one settings registry
([`extension-sdk-proposal.md`](extension-sdk-proposal.md) §8.1-8.4), and the helper UI decision
asks for Quick Settings and Settings Center to be generated from it
([`helper-ui-architecture.md`](helper-ui-architecture.md) §4). This section makes that concrete
for today's code, without waiting for the SDK.

### 7.1 Keep the typed struct; add one declaration table beside it

The engine reads settings on hot paths (every key, every focus): `self.settings.typing_echo`
must stay a field read, not a lookup by name. The layers, the files and the dialogs, on the
other hand, already work on values by path (`settings_layers.rs:18-19`). So:

- **`Settings` stays a typed, nested, serde struct**, grouped as Settings Center groups it
  (`speech`, `voice`, `typing`, `text`, `verbosity`, `say_all`, `web`, `forms`, `tables`, ...).
  The flat fields of today move into groups once (no compatibility shims are needed while
  AGENTS.md's notice stands).
- **A static table, `SETTINGS: &[SettingSpec]`, in `crates/okur-core/src/settings/registry.rs`,
  declares every leaf once:**

```rust
pub struct SettingSpec {
    /// The TOML path, which is also the id: "typing.echo", "web.headings".
    pub id: &'static str,
    /// Its place in the Settings Center tree.
    pub group: Group,
    /// Catalog messages (dialogs.ftl): the label, a one-paragraph description, and search
    /// words (synonyms a user may type: "echo", "keyboard", "letters").
    pub label: fn() -> StaticMessage,
    pub description: fn() -> StaticMessage,
    pub keywords: fn() -> StaticMessage,
    /// Bool, Choice (an enum's values and their catalog labels), Range { min, max, step },
    /// Text, or PerLevel (one value per verbosity level).
    pub kind: Kind,
    /// Where it may be set: any of Global, Application, Site, Document.
    pub scopes: Scopes,
    /// How long a change may last: Saved, UntilExit, UntilFocusMoves.
    pub lifetimes: Lifetimes,
    /// Also offered in Quick Settings for the application or site in front.
    pub quick: bool,
    /// JAWS's own key for it, for replay and for importing a user's JAWS settings:
    /// Jcf { file: Default | Application, section, key, decode } or Vpf { context, key, decode }.
    pub jaws: &'static [JawsKey],
    /// A key that cycles it for the session, with the cycle's order (Insert+2, Insert+Shift+2).
    pub toggle: Option<Toggle>,
    /// Where JAWS documents the setting and its default (a URL or a research section).
    pub source: &'static str,
}
```

- **Choices come from the enum itself**: each settings enum implements a small `Choices` trait
  (`ALL`, `label`). The dialog, the command line, Quick Settings' Space cycling, the toggle key
  and the TOML documentation then list the same values. Typing echo's 2-of-4 drift (§3.3)
  becomes impossible.
- **The default stays in `Settings::default()`**, so the engine's defaults and the registry's
  cannot disagree; `source` says which JAWS document gives it.

### 7.2 What is generated from the table

| Consumer | Generated how |
|---|---|
| Settings Center (Insert+6) | A search edit (`ControlKind::Edit`), the group tree (`ControlKind::Tree`), the selected group's controls by `kind`, the selected setting's description, "Settings for" (all applications, the application in front, its site), Apply, OK, Cancel, and "Restore defaults" for the chosen place. The control kinds exist (`crates/okur-ui-model/src/lib.rs:48-124`) but one: JAWS's description is a read-only field that Tab reaches, while `ControlKind::Text` is outside the tab order, so a read-only `Edit` (or a new kind) is needed. |
| Search | Matches the typed words against each setting's label, description, keywords and option labels in the user's language; the tree shows only the groups with matches, as JAWS's does. |
| Quick Settings (Insert+V) | The `quick` settings, for the application in front and its site, with a lifetime choice per item (Saved, Until Okur exits, Until the focus moves) and a "Last changed" group. |
| Files | Nothing new: the layers already store values by path. On load, a value whose id is unknown, whose type is wrong, or whose scope is not allowed (`keyboard.okur_key` in a site file) is ignored with a warning naming the file and the nearest id. |
| Command line | `okur settings list`, `okur settings get <id>`, `okur settings set <id> <value>` (with `--app X` or `--site Y`), `okur settings reset <id>`, and `okur run --set <id>=<value>` in place of one flag per setting. |
| A settings reference | `okur settings --write-reference` writes `docs/references/okur-settings.md` (every id, values, default, scopes, JAWS source); a test fails when it is stale, as `CORPUS_MANIFEST.json`'s does. |
| JAWS import | One loop over the specs' `jaws` keys turns a `DEFAULT.JCF`, an application JCF and a voice profile into layer values. `replay_settings` (`parity.rs:924-981`) becomes that loop over each evidence file's provenance, so `Chrome.JCF`, `LinePauses`, `LanguageDetection` and the VPF's punctuation and rates are replayed too. Reading a user's own `.jcf` and `.vpf` is allowed (AGENTS.md §1.4). |
| Toggle keys | One command, "cycle setting `id`", writes the session layer and says the new value's label. Insert+2, Insert+Shift+2, and later Insert+S (screen echo) and Insert+N (quick keys) become rows in the table. |

### 7.3 Layers and lifetimes

- Add the missing **until-the-focus-moves layer** between the session and the site (the SDK
  proposal's lookup order, §8.2): cleared when the application or document in front changes.
- Move the session state that lives outside the layers today (quick keys mode, virtual cursor
  on or off, speech on or off, the rate change; `toggles.rs`, `layer.rs`) into settings with a
  session lifetime, so the dialog shows what is in force and Insert+Q can report it.
- **Scopes per setting.** JAWS keeps some settings in the default file only (the JAWS key,
  Num Lock's initial state, Flexible Web on or off) [R §9.1]. `scopes` makes the dialog grey
  out the application and site places for them, and `SettingsLayers::change` refuse them.
- **A document scope** (JAWS's Quick Settings can save for one document) can come later; the
  `Scopes` flag reserves it.

### 7.4 Per-level and per-context settings

- Verbosity output types become a `PerLevel(Bool)` kind: the "Configure Verbosity Levels" dialog
  is generated from `OutputType::ALL`, a column per level, and message length beside it.
- Voice contexts (PC cursor, Okur cursor, keyboard, messages, menus and dialogs) become a
  `voice.contexts.<context>` table inheriting from `voice.global`, with rate, pitch, volume,
  punctuation and the voice; the dialog shows a context combo box above the voice controls, as
  Voice Adjustment does.
- Web settings that JAWS sets per site (§4.9) are declared with `Scopes::SITE`, so the
  Personalize Web Settings group of Quick Settings is simply "the quick web settings, site
  scope".

### 7.5 Tests the table makes cheap

- Every leaf of `Settings` has exactly one spec, and every spec's id resolves in `Settings`
  (set a non-default value through the path, read it back typed): a completeness guard like
  xa11y's `ElementParts`.
- Every spec has a label and a description in the catalog; no label says "JAWS" (the existing
  catalog test already enforces AGENTS.md §1.5).
- Every `jaws` key decodes the values JAWS's files were seen to hold, and the import of old-pc's
  recorded files gives `Settings::reference_machine()`.
- Access keys are unique within each generated dialog page.
- Search finds each setting by its label and by each keyword.

### 7.6 Order of work

1. Introduce `SettingSpec` and the table for today's 16 leaves; generate the current dialog
   from it (same controls, same words: parity of Okur's own dialog stays as it is). **Done
   2026-10-21**, with typing echo's four values and `--set`.
2. Generate the JAWS import and replace `replay_settings`. **Done 2026-10-21** (`DEFAULT.JCF`
   only; application JCFs and voice profiles next): parity unchanged, 5192/5395.
3. Add the tree, search and description: Settings Center. Quick Settings with lifetimes.
4. Then each new setting is one enum (or bool), one table row, two catalog messages and the
   behavior itself.

## 8. Settings to add, most everyday impact first

Each still follows the owner's rule: found in JAWS's documentation (the sources here), recorded
on old-pc, then built. JAWS's defaults stay Okur's defaults, so none of these changes what
parity measures until a scenario sets a value. Effort: S (an enum and a branch where the
behavior already exists), M (new behavior in one place), L (new subsystem).

### The top ten

| Rank | Setting (§4 row) | Why first | Effort |
|---|---|---|---|
| 0 | The registry, generated Settings Center with search, Quick Settings with lifetimes (§7) | Makes every row below a small change, and fixes today's drift | M |
| 1 | Typing Echo, all four values everywhere; Manage Key Labels (mute letters, Tab, Space) (41, 42) | Typing is constant; the dialog offers half the values today | S, then M |
| 2 | Forms Mode: Manual, Semi-Auto, Auto, in the dialog and per site; forms mode on/off said or sounded (116, 117) | The setting exists but cannot be reached from a dialog; old-pc itself runs Manual, so users do change it; Semi-Auto turns forms mode on by Tab only | S, then M |
| 3 | Page load: read automatically when loaded, page summary, Smart Glance indication; per site (99, 100, 109) | Every page load speaks, and today nothing can stop it; Freedom Scientific suggests turning the reading off when a magnifier is used [F8] | S |
| 4 | Web Verbosity Level and Configure Web Verbosity Levels, per browser and per site (90, 91), with Inline Frames Show (97) and headings (96) | The biggest lever on how chatty pages are; the evidence's `VirtualCursorVerbosityLevel=2` can then be replayed | M |
| 5 | Configure Verbosity Levels: every output type per level, message length, tutor messages and access keys (72-75a) | The model exists; a dialog and the remaining types let a user tune each level, as JAWS does | M |
| 6 | Punctuation per voice context, Customize Punctuation, Filter Repeated Characters (35-37) | Coders and anyone reading addresses; per-context levels may also explain why recorded selections name symbols that line reading leaves out ([R] §1) | M |
| 7 | Capitalization: Caps Indicate, capital pitch, caps during Say All (54, 55, 81) | Proofreading names and code | S to M |
| 8 | Numbers and words: Number Processing, Speak Single Digits, Mixed Case, dollars, dates, alphanumeric, Smart Word Reading (56-62) | Daily reading of prices, phone numbers, identifiers; Mixed Case and Speak Single Digits already work at JAWS's default in `pronounce.rs` and only need the setting | M |
| 9 | Voice: pitch, voice contexts (keyboard, messages, cursors), voice profiles per application, saved rate change (19-28) | Pitch is the one basic voice control missing; contexts are how JAWS gives typing echo and messages their own voice and punctuation, and old-pc's profiles use them | M |
| 10 | Say All and lines: Line with and without pauses, blank lines, indentation, live regions on or off and the live region filter (65, 80, 82, 107, 108) | Long reading, code, chat pages | M |

### Next

- Text Links Show Using, Graphics Show, Untagged Graphical Links Show (92-94); Table Titles
  Announce, Cell Header and Content, coordinates (118-120); Document Presentation Mode (104);
  Lines per page and Maximum line length (102, 103).
- Navigation Quick Keys saved per site, Navigation Quick Key Delay, Smart Navigation (84, 105,
  106).
- Speech Mode with Speech On Demand and its options (29, 30); Screen Echo (85); Mouse Echo (86).
- Proofing: spelling and grammar indication, and the comment, insertion and deletion
  announcements (67-69).
- Restore defaults per scope, Manage Application Settings, export and import, Insert+Q twice
  (11-13, 16).
- Language detection (33); Dictionary Manager (39, 40).
- Notifications: toasts on or off, the history and its rules (121-123).
- Needs a sound output first: forms mode sounds (117), Speech and Sounds schemes (124), Say All
  scheme (83), audio ducking (31).
- Keyboard Manager and Navigation Quick Key Manager (50, 51): after the key table becomes data
  (extension SDK proposal §7.2).

## Sources

Freedom Scientific's pages under `support.freedomscientific.com` answer HTTP 403 to the fetch
tool; they were read on 2026-10-21 in the browser pane, as public pages. Nothing here was
recorded on old-pc, and no JAWS program file, script or settings file of this machine was read.

Freedom Scientific:
- [F1] What's New in JAWS 2025: https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2025
- [F2] What's New in JAWS 2024: https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2024
- [F3] What's New in JAWS 2023: https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2023
- [F4] What's New in JAWS 2022: https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2022
- [F5] What's New in JAWS 2021: https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2021
- [F6] What's New in JAWS 2020: https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2020
- [F7] What's New in JAWS 2019: https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2019
- [F8] What's New in JAWS 2018: https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2018
- [SU5] Surf's Up, Navigating Web Pages: https://support.freedomscientific.com/SurfsUp/5-Navigating.htm
- [SU6] Surf's Up, Temporary Versus Permanent Changes: https://support.freedomscientific.com/SurfsUp/6-TemporaryVersusPermanent.htm
- [SU7] Surf's Up, JAWS Web Verbosity: https://support.freedomscientific.com/SurfsUp/7-WebVerbosity.htm
- [SU8] Surf's Up, Tables: https://support.freedomscientific.com/SurfsUp/8-Tables.htm
- [SU9] Surf's Up, Forms: https://support.freedomscientific.com/SurfsUp/9-Forms.htm
- [SU10] Surf's Up, Frames: https://support.freedomscientific.com/SurfsUp/10-Frames.htm
- [SU12] Surf's Up, Dealing with Difficult Pages: https://support.freedomscientific.com/SurfsUp/12-difficultPages.htm
- [SU13] Surf's Up, Using the JAWS Find Command: https://support.freedomscientific.com/SurfsUp/13-JAWSFind.htm
- [SU15] Surf's Up, PlaceMarkers and the Custom Page Summary: https://support.freedomscientific.com/SurfsUp/15-PlaceMarkers.htm
- [SU16] Surf's Up, Flexible Web: https://support.freedomscientific.com/SurfsUp/16-FlexibleWeb.htm
- [SU17] Surf's Up, Personalized Web Settings: https://support.freedomscientific.com/SurfsUp/17-PersonalizedSettings.htm
- [SU18] Surf's Up, Speech and Sounds Schemes: https://support.freedomscientific.com/SurfsUp/18-SpeechSoundsSchemes.htm
- [SU19] Surf's Up, Using Custom Labels: https://support.freedomscientific.com/SurfsUp/19-CustomLabels.htm
- [KS] JAWS keystrokes: https://support.freedomscientific.com/Content/Documents/Manuals/JAWS/Keystrokes.txt
- [AD] JAWS Script Manual, Appendix D, naming settings files (public documentation, not script
  source; as cited by [R], not opened here):
  https://support.freedomscientific.com/Content/Documents/Other/ScriptManual/Appendix_D.htm
- [B1656] Technical support bulletin 1656, factory defaults and application settings:
  https://support.freedomscientific.com/support/technicalsupport/bulletin/1656 (as cited by [R])

Others (Sight and Sound Technology is a JAWS distributor; its articles describe JAWS's dialogs):
- [T1] Sight and Sound, set speech verbosity:
  https://sastltd.zohodesk.eu/portal/en/kb/articles/jaws-reducing-speech-verbosity
- [S-wizard] Sight and Sound, Startup Wizard screens:
  https://sastltd.zohodesk.eu/portal/en/kb/articles/jaws-startup-wizard-screens
- [S-voice] Sight and Sound, voice adjustments:
  https://sastltd.zohodesk.eu/portal/en/kb/articles/jaws-set-voice-adjustments
- [S-pitch] Sight and Sound, uppercase pitch:
  https://sastltd.zohodesk.eu/portal/en/kb/articles/jaws-change-or-increase-uppercase-pitch-announcement
- [S-caps] Sight and Sound, a sound or a word for capitals:
  https://sastltd.zohodesk.eu/portal/en/kb/articles/configure-jaws-to-play-sound-or-speak-a-word-to-indicate-uppercase-characters
- [S-rate] Sight and Sound, rate changes on the fly:
  https://sastltd.zohodesk.eu/portal/en/kb/articles/unable-to-permanently-adjust-speech-rate-on-the-fly
- [S-ducking] Sight and Sound, audio ducking:
  https://sastltd.zohodesk.eu/portal/en/kb/articles/disable-jaws-audio-ducking-feature
- [S-splitter] Sight and Sound, the sound splitter:
  https://sastltd.zohodesk.eu/portal/en/kb/articles/jaws-using-the-sound-splitter-feature
- [S-space] Sight and Sound, Manage Key Labels:
  https://sastltd.zohodesk.eu/portal/en/kb/articles/jaws-spacebar-is-not-announced-when-keyed
- [S-numlock] Sight and Sound, initial state of Num Lock:
  https://sastltd.zohodesk.eu/portal/en/kb/articles/jaws-set-initial-state-of-number-lock
- [S-phonetic] Sight and Sound, phonetics after pausing:
  https://sastltd.zohodesk.eu/portal/en/kb/articles/jaws-says-phonetic-of-every-letter-after-pausing-during-character-navigation
- [S-state] Sight and Sound, control state first:
  https://sastltd.zohodesk.eu/portal/en/kb/articles/make-jaws-speak-the-control-state-first
- [S-mouse] Sight and Sound, mouse echo:
  https://sastltd.zohodesk.eu/portal/en/kb/articles/jaws-enable-mouse-echo
- [S-transfer] Sight and Sound, moving JAWS settings to another computer:
  https://sastltd.zohodesk.eu/portal/en/kb/articles/transfer-your-jaws-settings-to-another-computer
- [WFH-wizard] Web Friendly Help, the Startup Wizard:
  https://webfriendlyhelp.com/put-the-jaws-startup-wizard-to-work-for-you/
- [WFH-ta] Web Friendly Help, Text Analyzer:
  https://webfriendlyhelp.com/taking-a-look-at-text-analyzer-inside-jaws/
- [DS-spell] dsurf.net, spelling indication: https://dsurf.net/disable-buzzer-and-misspelled
- [TAG] The Accessibility Guy, language detection:
  https://theaccessibilityguy.com/how-to-setup-jaws-to-autodetect-language/
- [TAVIP] TAVIP, the Speech and Sounds Manager:
  https://www.tavip.org.uk/an-introduction-to-the-jaws-speech-and-sounds-manager/
- [TTT177] Top Tech Tidbits 177 (Notification History):
  https://www.toptechtidbits.com/177.html
- [LN-mouse] LexisNexis support, mouse echo options:
  https://supportcenter.lexisnexis.com/app/answers/answer_view/a_id/1124308/~/enabling-the-mouse-echo-feature-in-jaws
- [SAOMAI] Sao Mai Center's copy of the JAWS 18 release notes:
  https://www.saomaicenter.org/en/blog/access-tech/whats-new-jaws-180
- [G-progress] JAWS users' group thread on progress bars (unconfirmed):
  https://jfw.groups.io/g/main/topic/jaws_and_progress_bars/2909954
- [SE-yyyy] Freedom Scientific Sweden (the Swedish distributor), What's New pages in Swedish:
  https://freedomscientific.se/news/view/vad-ar-nytt-i-jaws-2020/ (likewise `-2021/`,
  `-2022/`, `-2023/`, `-2024/`, `-2026/`; 2025 is `-20251/`)
- [S-refresh] Sight and Sound, position lost on a refreshing page (Page Refresh filter):
  https://sastltd.zohodesk.eu/portal/en/kb/articles/jaws-position-on-web-page-gets-lost-or-changes
- [S-vpc] Sight and Sound, the virtual cursor's state per browser tab:
  https://sastltd.zohodesk.eu/portal/en/kb/articles/jaws-virtual-pc-state-not-remembered-when-returning-to-a-browser-hosted-web-app
- [DS-web] dsurf.net, HTML and web settings for testing with JAWS:
  https://dsurf.net/html-and-web-settings-for-testing-with-jaws
- [WebAIM-dup] WebAIM list, duplicate links filter: https://webaim.org/discussion/mail_message?id=28405
- [W3C-lines] W3C WAI-IG list, lines per page quoted from JAWS's help:
  https://lists.w3.org/Archives/Public/w3c-wai-ig/2021JanMar/0078.html
- [ADG] Accessibility Developer Guide, JAWS forms modes:
  https://www.accessibility-developer-guide.com/setup/screen-readers/jaws/
- [UAAG-jfw] W3C UAAG 1.0 techniques, a picture description of an older JAWS's HTML options:
  https://www.w3.org/TR/UAAG10-TECHS/techimages/jfw-html-options-desc.html
- [ETS-braille] ETS, configuring JAWS braille settings:
  https://ca-toms-help-new.ets.org/accessibility-guide/requirements-for-visual-impairment/config-and-administer-with-jaws/

In this repository:
- [R] [`jaws-reading-settings-research.md`](jaws-reading-settings-research.md): JAWS 2025's
  installed help (`SettingsCenter.chm`, `JFW.chm`, `popapps.chm`, `jdiction.chm`) summarized by
  section, with its own source list; cited here by section.
- Key names of old-pc's `DEFAULT.JCF`, `Chrome.JCF` and voice profiles, as recorded in
  `corpus/evidence/*.json` provenance (`jaws_settings`), which AGENTS.md §1.4 allows as the
  reference machine's configuration.
