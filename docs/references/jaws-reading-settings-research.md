# JAWS reading and review settings: research before recording

Date: 2026-10-18. Step 1 of the owner's three steps for this round
([`2026-10-18-helper-dialogs-round-2.md`](../plans/2026-10-18-helper-dialogs-round-2.md) §6):
find each feature in JAWS's documentation, then record it on old-pc, then implement it. This
document does step 1 and proposes step 2.

Nothing here was recorded on old-pc, and no code was changed.

## How to read this document

Every statement carries one of these marks:
- **Doc**: a JAWS document says it. The source id is in brackets, for example [S1]. The sources
  are listed at the end.
- **Obs**: recorded evidence on old-pc shows it (`corpus/evidence/<scenario>`, step id).
- **Inf**: my inference. Confirm it before building on it.
- **Open**: neither documented nor observed. It must be recorded.

Documents are summarized, not copied. At most one short phrase is quoted from any one source.

### Sources and how they were read

- **JAWS's own help.** The dev box has JAWS 2025.2505.43.400, the same build as old-pc. Its help
  files are in `C:\ProgramData\Freedom Scientific\JAWS\2025\Locale\enu\Help\`. AGENTS.md §1.4
  allows reading them. I extracted the following into the session scratchpad with
  `hh.exe -decompile`; they are not in the repository:
  - `JFW.chm` (the JAWS help);
  - `SettingsCenter.chm`;
  - `jdiction.chm` (Dictionary Manager);
  - `tutorial.chm`;
  - `popapps.chm` (the help for popular applications, including the browsers and Personalized
    Web Settings).

  These are the main sources. They describe the installed build and are more current than the
  web pages.
- **JAWS's key map.** `C:\ProgramData\Freedom Scientific\JAWS\2025\Scripts\enu\Default.JKM`
  [K1]: key assignments and script names, which §1.4 allows. I read only this file in that
  folder.
- **Freedom Scientific web pages:**
  - the keystroke list;
  - the What's New pages for JAWS 15 to 2026;
  - the teacher training lessons;
  - a 2015 settings lesson;
  - the Surf's Up lesson on personalized settings;
  - the Script Manual's appendix on how settings files are named and loaded;
  - a support bulletin on application settings.

  Plus third-party support articles (Sight and Sound Technology, Deque).
- **Pages I could not read:**
  - `support.freedomscientific.com/support/technicalsupport/bulletin/1636` (typing echo
    bulletin): HTTP 403 to the fetch tool. In a browser it says the bulletin is no longer
    available.
  - `doccenter.freedomscientific.com/doccenter2/.../QuickSettings/02_QuickSettings.htm`: HTTP 403
    to the fetch tool, and 401 Unauthorized in a browser.
  - `www.freedomscientific.com/training/jaws/hotkeys/`: HTTP 403 to the fetch tool. I read it in
    the browser pane; it matches the keystroke list [W1].
  - `sastltd.zohodesk.eu/.../jaws-change-or-increase-uppercase-pitch-announcement`: not fetched.
    Only the search engine's excerpt was seen.
  - `www.freedomscientific.com/SurfsUp/Personalized_Settings.htm` (cited by the extension SDK
    research as [J23]): it now gives "Page Has Moved". The same lesson is at
    `support.freedomscientific.com/SurfsUp/17-PersonalizedSettings.htm` [W16], which I read.
  - The What's New pages (`support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=…`)
    and the teacher lessons were read in the browser pane. `Keystrokes.txt` was read through the
    fetch tool, whose summarizer returned the matching rows.
- **Old-pc's own settings files.** These are the evidence provenance in
  `corpus/evidence/207-web-character-review-evidence.json` and
  `209-web-title-and-font-evidence.json`. Every JCF and VPF key name this document gives comes
  from those files, never from a guess.

### Two layers, and why the speech recorder matters here

JAWS keeps the text it decided to speak in its speech history. It applies symbol processing
afterwards, when it hands the text to the synthesizer.
- This is the model in `crates/okur-core/src/pronounce.rs:1-8`.
- Parity checks it in both layers (`parity-gap-audit.md` §3).

Old-pc's voice is the SR Speech Recorder. It logs every fragment JAWS sends the synthesizer,
with its rate and pitch. For example, `"pitch": 4` for a capital in 307's evidence
([`verification-loop.md`](verification-loop.md) §3).

What this means for these settings:
- **Punctuation, symbol names, word splitting, number processing, spelling rate and capital
  pitch** show only in the spoken layer (Inf: the history keeps the text as written).
- **Typing echo** is not kept in the speech history at all. Okur already models this from
  305/s10-s12 (`crates/okur-core/src/speech.rs:926-931`). Only the spoken layer shows it.
- **Messages** (the level announced by a toggle, "misspelled", "cap") presumably show in both
  layers (Inf).

So every scenario in this plan must be recorded with the recorder voice active, and compared
with `okur-lab parity --spoken`.

## 1. Where JAWS keeps these settings

| Place | Opened with | What it holds | Source |
|---|---|---|---|
| Settings Center | Insert+6 (laptop CapsLock+6), or Insert+F2 then Settings Center | Every option, for the application in front or, after Ctrl+Shift+D, for all applications (`Default.jcf`) | Doc [S12], [W7] |
| Quick Settings | Insert+V (CapsLock+V) | The options of the application in front. Each change has a persistence: until JAWS exits, until the focus changes, or saved (the default). Saved changes go to the application's JCF or the document's JSI. | Doc [H6] |
| Voice Adjustment | JAWS window (Insert+J), Options, Voices, Voice Adjustment | The voice profile (`.VPF`): synthesizer, language, and per voice context the person, rate, punctuation, pitch and volume; spelling rate and capital pitch apply to all contexts | Doc [H1] |
| Select a Voice Profile | Ctrl+Insert+S (CapsLock+Ctrl+S) | The profile for all applications and for the one in front | Doc [H9], [W1] |
| Startup Wizard | JAWS window, Help, Startup Wizard | Rate, punctuation, typing echo, verbosity and more | Doc [H3] |
| Speech and Sounds Manager | A group of Settings Center; Alt+Insert+S switches schemes temporarily | Schemes (`.smf`): per control type, state, attribute, font, color, indentation and misc item: speak, play a sound, change voice, change language, or ignore | Doc [S8] |
| Dictionary Manager | Insert+D | Word pronunciation rules (`.jdf`) | Doc [D1] |
| Customize Punctuation | A dialog in Settings Center | Per symbol: spoken description and the level from which it is spoken | Doc [S2] |

The setting files and their keys. These were observed in old-pc's user files; I did not guess
any of them:
- **`DEFAULT.JCF`:**
  - `[Options] TypingEcho=0`, `KeyboardType=Laptop`, `LanguageDetection=0`;
  - `[FormsMode] AutoFormsMode=0`;
  - `[OutputModes] ACCESS_KEY=0|0|0|Access Key`.
- **`Chrome.JCF`:**
  - `[VirtualCursorVerbosity] VirtualCursorVerbosityLevel=2`;
  - `[options] AllowWebAppReservedKeystrokes=0`.
- **Voice profiles** (`voiceprofiles/*.VPF`):
  - one section per language and context: `[ENU-Global]`, `[ENU-PCCursor]`, `[ENU-JAWSCursor]`,
    `[ENU-Keyboard]`, `[ENU-Message]`, `[ENU-MenuAndDialog]`;
  - keys `Parent`, `VoiceName`, `Rate`, `Pitch`, `Volume`, `Punctuation`, `SpellRateDelta`,
    `UppercasePitchDelta`, `SynthLangString`;
  - a section `[ENU-Voice Aliases]` with `SpellingVoice`, `SingleCapVoice`, `AllCapsVoice`,
    `SmallCapsVoice`.

The active profile "SAPI 5X 64" sets only the voice, two pitches and the aliases (Obs, 207
provenance). Its punctuation therefore comes from the factory profile. The help gives Most as
the default (Doc [H1], [H3]).

The help names contexts differently from the file:
- **In the help:** All Context, PC Cursor Voice, JAWS Cursor Voice, Keyboard Voice, and Tutor
  and Message Voice (Doc [H1]).
- **In the file:** also a `MenuAndDialog` context that the help does not name (Obs).

### Temporary toggles and how long they last

| Key (desktop / laptop) | Script [K1] | Cycles | Lasts | Source |
|---|---|---|---|---|
| Insert+2 / CapsLock+2 | `SetTypingEcho` | characters, words, characters and words, none | Two documents disagree. The 2015 lesson [W7] says Alt+Tab to another window restores the default. The 2021 What's New [W2] says the setting holds until JAWS restarts. **Open** | Doc [H4], [W2], [W7] |
| Insert+Shift+2 / CapsLock+Shift+2 | `SetPunctuationLevel` | None, Some, Most, All | Until JAWS restarts | Doc [W2], [K1] |
| Insert+S / CapsLock+S | `SetScreenEcho` | highlighted, all, none | Not stated | Doc [H4], [K1] |

The JAWS 2025 help's keystroke tables and `Keystrokes.txt` [W1] list Insert+2 and Insert+S but
not Insert+Shift+2. Only the 2021 What's New [W2] and the key map [K1] give it.

None of the documents says what JAWS speaks when these keys change a setting. One support
article says the new value is spoken [W11]. The 2015 lesson says that pressing Insert+S until
JAWS says "highlighted" restores screen echo [W7]. **Open** for all three.

---

## Item 1. Punctuation

### The setting

- **Name and place:**
  - Punctuation, a combo box in Voice Adjustment, one per voice context and per language (Doc
    [H1]);
  - the Startup Wizard's speech page (Doc [H3]);
  - Quick Settings, Editing Options group, per application (Doc [H2], [W7]).
- **Values:** None, Some, Most, All.
- **Default:** Most (Doc [H1], [H2], [H3]). Old-pc runs at Most (Obs: `verification-loop.md`
  §3).
- **What a level means** (Doc [S2]):
  - Every symbol has a "Level When Spoken". A symbol is spoken at that level and every level
    above it.
  - The help's example uses the comma. Set to None, a comma is read at every level. Set to Most,
    it is read at Most and All. Set to All, it is read only at All.
  - At All, every symbol in the list is spoken.
- **Symbols whose level cannot be changed** (Doc [S2]): blank, colon, comma, dot, point and
  space. Only their descriptions can be changed. JAWS uses them in special places, such as a
  decimal point or the dots of an IP address or web address.
- **None still pauses:**
  - A third-party guide says that at None JAWS still marks punctuation with pauses and
    inflection [W12 search excerpt].
  - JAWS 18 fixed speech having no pauses at None (Doc [W6]).

  So at None, the voice should still get the punctuation characters, or a pause, rather than
  have them removed (Inf).
- **Which symbol belongs to which level:**
  - Not in any document. It is shown in the Customize Punctuation dialog's list, which has three
    columns: Symbols, Description, Level When Spoken (Doc [S2]).
  - Reading that list on old-pc is reading JAWS's own interface, which §1.4 allows.
  - Read on 2026-10-19: [`jaws-punctuation-levels.md`](jaws-punctuation-levels.md), with what
    each level does to running text (scenario 400).

### Customizing symbols

- **Customize Punctuation** (Settings Center; search "Customize Punctuation") (Doc [S2]):
  - F2 or Edit Description changes a symbol's spoken text;
  - Space, or the level radio buttons (Alt+N, Alt+S, Alt+M, Alt+L per [W12]), changes its level;
  - Restore Defaults resets the whole list;
  - Advanced opens descriptions for special symbols that are not punctuation.

  Which file stores these changes is not documented. **Open:** change one symbol in the Default
  file and see which file changes.
- **Character definitions per synthesizer:**
  - The help topic "Adding Speech for Special Symbols" says these live in an `.sbl` file in the
    shared settings folder, for example `Eloq.sbl`, with one section per language. Entries have
    the form `U+XXXX=` followed by the spoken text, and JAWS must be restarted after a change
    (Doc [H7]).
  - I did not read any `.sbl` file for this research. The owner has since decided (2026-10-19)
    that the shipped `.sbl` may be looked at, since the help tells users to edit it. Symbol
    names still come from recordings, as `pronounce.rs` does today.
- **Filter Repeated Characters** (Settings Center, Text Processing, General) (Doc [S1]):
  - Values: Three (the default), Four, Five, Six, All, Count.
  - It sets how many characters of an unbroken run are spoken.
  - Spaces are not reported except at Count, which is meant for programming.
  - Evidence: "..." gives "dot dot dot" (Obs, 306/s3, 311/s6), three of three.
- **Announce Special Symbols When** (Settings Center, Graphics and Symbols) (Doc [S6]):
  - Values: Never; Type Echo, Say Char, Spell Word; Also Say Word; Also Say All.
  - It controls when JAWS speaks the numeric value of special characters such as copyright,
    registered and trademark.
  - The default is not stated.
  - Old-pc says "copyright" for © both alone and in running text (Obs, 307 and 313/s4), so its
    setting does not say the value there (Inf: Never).

### Per-context settings (the question asked)

- **Per voice context** (Doc [H1], [W4]): Voice Adjustment sets punctuation separately for the
  PC Cursor, JAWS Cursor, Keyboard (typing echo) and Tutor and Message voices. All Context sets
  them all at once.
  - JAWS 2020 fixed per-context punctuation. The fix note's example: None for the PC Cursor
    while the JAWS Cursor, Keyboard and Tutor and Message voices stay at Most.
  - So typing echo has its own level, through the Keyboard voice.
- **Per language within a profile** (Doc [H1], [H9]; JAWS 16 fixed it [W6]).
- **Per application:**
  - Quick Settings' Punctuation is saved in the application's JCF (Doc [H6], [W7]). The 2015
    lesson suggests All in programming tools [W7].
  - A different voice profile can be chosen per application (Doc [H9]). The help suggests a
    profile at All for a coding application.
- **Edit fields against documents:** no separate setting is documented. Any difference comes
  from the voice context in use (Inf).
- **Old-pc's evidence already shows context dependence at Most:**
  - Reading a line leaves commas and a sentence's last period unspoken:
    - 319/s4: "Internet resource, and Windows will open it for you.";
    - 228/s10: "the lazy dog, then reads std colon colon io again".
  - Reading a selection names them:
    - 323/s7, after Ctrl+A in the property sheet: "Friday comma, January 2 comma, 2026
      comma,";
    - 314/s6: "Press ESCAPE to close this message period.".
  - "!" is left alone on a RichEdit line (315/s3, "println!") but named in the viewer's
    selection (314/s11, "println exclaim!").
  - "?" ends a dialog's text unnamed but is named "question?" in the viewer (audit §3.1;
    311/s21).
  - Inf: the selection text and the viewer's text are spoken in a different voice context, or at
    a different level, from line reading. **Open:**
    - read the punctuation of each context of "SAPI 5X 64" in Voice Adjustment on old-pc;
    - measure which context each kind of speech uses, with the pitch-per-context test proposed in
      [`extension-sdk-proposal.md`](extension-sdk-proposal.md) §6.4.
- **The level is stored as `Punctuation`** in a VPF context section:
  - Obs: `Eloquence.VPF` has `Punctuation=0` in every context. That profile is the one
    `verification-loop.md` §3 describes as punctuation off.
  - Obs: `Persian.VPF` has `Punctuation=2`.
  - Inf: 0 to 3 stand for None to All. Confirm by changing the level in Voice Adjustment on
    old-pc and comparing the file.

### Symbol names: what is known

Read alone (Left/Right, Delete), at Most:
- **The full list:** recorded in 307, 207 and 208, and kept in `pronounce.rs:68-125` (Obs).
- **The symbols asked about:**
  - "(" is "left paren", "/" is "slash", "@" is "at", "#" is "number";
  - "…" (U+2026) is "ellipsis …";
  - "." is "period.", "!" is "exclaim!", "," is "comma,", "?" is "question?".
- **Names that keep the symbol after them:** "period.", "comma,", "exclaim!", "question?". The
  synthesizer is given both the name and the symbol (Obs).
- **The open claim:** `pronounce.rs` says JAWS names a character read alone whatever the
  punctuation level. Only Most has been recorded, so this is **Open** for None, Some and All.

In running text, at Most (Obs). Old-pc's synthesizer was given:
- **Code** (314/s8, 315/s1, 315/s3):
  - "fn main() {" as "fn main left paren right paren left brace";
  - "let total = price * 1.5;" as "let total equals price star 1.5 semi colon".
- **Code read in a WinForms edit field** (313/s3): "x  = 1.5; a.b end." as "x equals 1.5 semi
  colon a dot b end.". The decimal point and the sentence's last period stay as they are, and
  the period between letters is "dot".
- **Ellipses:** "Save..." as "Save dot dot dot" (306/s3). The single character "…" is passed
  through in running text (313/s4).
- **A file path in the address bar** (220/s7): "file:///C:/Users/…/news-article.html" as "file
  colon slash slash slash C colon slash Users slash … news dash article dot html".
- **Shortcuts and hyphens:**
  - "Ctrl+N" as "Ctrl plus N" (220/s2);
  - "Read-only" as "Read dash only" (323/s1);
  - "std::" as "std colon colon" (213/s14).
- **A line of every ASCII symbol, read as a selection** (313/s7): every one is named:
  - "exclaim!", "quote", "number", "dollar", "percent", "and", "apostrophe";
  - "left paren", "right paren", "star", "plus", "comma,", "dash", "period.", "slash";
  - "colon", "semi colon", "less", "equals", "greater", "question?", "at";
  - "left bracket", "back slash", "right bracket", "carat", "underline", "grawv";
  - "left brace", "vertical bar", "right brace", "til de".
- **The Unicode line** (313/s4):
  - named: "euro sign", "left quote", "right quote", "apostrophe", "bullet", "rightwards arrow",
    "copyright", "one half", "not equal to", "less-than or equal to", "lamda";
  - passed through: the em dash, "…", "°", "×", "÷", "±".

No other level has evidence.

### Keys

- Insert+Shift+2 (laptop CapsLock+Shift+2) cycles the level (Doc [W2], [K1]). What it says is
  **Open**, and so is the order starting from Most.
- On some Dell keyboards in the laptop layout, Right Shift may be needed for this key (Doc
  [W2]).

### Okur today

- **Not there:**
  - a punctuation setting (`settings.rs` has none);
  - per-context voices;
  - symbol processing of running text. The synthesizer gets the history text with the role marks
    removed (`pronounce.rs:62-66`).
- **There:** only a character read alone is named (`pronounce.rs:26-37`).
- **Already planned:** the extension SDK proposal plans levels and per-context voices
  (`extension-sdk-proposal.md` §6.3, §6.4).

### Recording proposal

1. **Read JAWS's tables** (not a scenario; capture the screens with `okur-lab snapshot` and save the
   output with the research):
   - The Customize Punctuation list in Settings Center, in the Default file: every symbol, its
     description and its level. Also the Advanced dialog's list.
   - Voice Adjustment for "SAPI 5X 64": the Punctuation value for each Adjust context.
   - Then change the PC Cursor punctuation to Some, compare the VPF, and change it back. This
     confirms the `Punctuation` encoding.
2. **The level-cycle scenario** (desktop, `desktop-editor.ps1`, PC cursor). Insert+Shift+2
   pressed four times, with CapsLock+I (Say Line) on the punctuation line after each press.
   - Four presses bring the level back to where it started, so every trial starts at Most.
   - This records the announcements, the cycle order, and each level's effect in one scenario.
3. **One scenario per level** (None, Some, Most, All) on a new reading fixture (§12), in two
   versions:
   - **web:** Chrome, virtual cursor: Down through each line, then Insert+Down (Say All) over the
     paragraph;
   - **desktop:** a WinForms twin: CapsLock+I on each line; Ctrl+A in the field, which gives the
     selection context; Right onto "(", "@" and "…" to test the claim that characters read
     alone do not depend on the level.

   Set the level by writing the VPF and restarting JAWS (harness note 2 in §11), so each
   evidence file's provenance shows it. Put Most back afterwards.
4. **One per-context scenario:** PC Cursor at None, the other contexts at Most. Read a line, a
   selection (Ctrl+A) and the Virtual Viewer (Insert+F twice). Combine it with the
   pitch-per-context test from the SDK proposal §6.4.
5. **Repeated characters:** the fixture's repeated-symbols line, at Filter Repeated Characters
   Three (the default) and at Count.

---

## Item 2. Typing echo

### The setting

- **Name and place:**
  - Typing Echo, in Settings Center's User Options group (Doc [S3]);
  - the Startup Wizard's speech page (Doc [H3]);
  - Quick Settings, Editing Options group (Doc [H2], [H6]).
- **Values:**
  - in Settings Center and the Startup Wizard: Off, Characters, Words, Characters and Words;
  - in Quick Settings the same four are called None, Characters, Words, Both Characters and
    Words (Doc [S3], [H2]).
- **What each value does:**
  - Characters says each character as it is typed;
  - Words says each word after Space or Enter;
  - Characters and Words does both (Doc [S3]).
- **Default:** Characters (Doc [H2], [H3], [H6]). Old-pc has `TypingEcho=0`, so echo is off
  there (Obs, provenance). The other values: 1 characters, 2 words, 3 both (Obs, 709-711: each
  value set in `DEFAULT.JCF`, "ab cd" typed). With 3, a word corrected with Backspace was not
  said at its end, as it is with 2 (711/s7, an accepted divergence).
- **Other typing settings:**
  - **The Keyboard voice:** Voice Adjustment has its own Keyboard voice for typing echo, with
    its own punctuation (Doc [H1], [W4]).
  - **Backspace and Delete:** they still say the deleted letter when echo of letters is off
    [W11, Freedom Scientific guidance quoted in a search excerpt]. Okur's notes say the same:
    on the dev box Backspace spoke with `TypingEcho=0` (`settings.rs:106-108`).
  - **Speech On Demand** silences typing echo unless its Allow Typing Echo box is checked; that
    box is cleared by default (Doc [S11]).
  - **The touch keyboard** has its own Touch Typing Echo, Both by default (Doc [S4]).
- **Release notes about echo:**
  - JAWS 16: a password in a protected Word form is echoed as "*" (Doc [W6]).
  - JAWS 2023: extra letters with echo set to Words (Doc [W5-2023]).
  - JAWS 2025: in VS Code, deleting no longer reads characters from other lines (Doc [H10]).

### Keys

Insert+2 (laptop CapsLock+2), script `SetTypingEcho`, cycles characters, words, characters and
words, none (Doc [H4], [K1]). How long the change lasts is in §1. What JAWS says on each press
is **Open**.

### Okur today

- **There:**
  - `TypingEcho` has two values, `Characters` and `None` (`settings.rs:99-122`);
  - a JCF value of "0" becomes `None`, and any other value `Characters`;
  - echo speaks through `speak_echo` (`speech.rs:926-931`), from
    `engine/text_feedback.rs:56`;
  - Okur's settings dialog offers Off and Characters (`dialogs.ftl`).
- **Not there:** word echo; the Insert+2 command; a Keyboard voice context.

### Recording proposal

1. **Learn the encoding:** set each value in Settings Center (Default) on old-pc, compare
   `DEFAULT.JCF` each time, and put `TypingEcho=0` back.
2. **The toggle scenario** (desktop, `desktop-editor.ps1`): Insert+2 pressed four times.
   - Before each press, type "ab cd" and Enter, which shows each mode at work.
   - The fixture is relaunched before each trial, so its text starts the same.
   - Four presses bring the echo back to old-pc's None.
3. **One scenario per value** (Characters, Words, Characters and Words; None is already the
   reference). Type:
   - lowercase letters, a capital (Shift+A), a digit, Space, comma and period;
   - Enter, Backspace, Delete;
   - a word followed by Space, and a word followed by Enter.

   Record in two places:
   - the desktop editor (PC cursor);
   - a page's text area in forms mode (Chrome; old-pc has manual forms mode, so press Enter
     first).

---

## Item 3. Key echo, key labels and key repeat

### What the documents give

- **Manage Key Labels** (Settings Center, Keyboard group) (Doc [S4]): a list with three columns.
  - Key Name;
  - Key Label: what JAWS says when it echoes the key;
  - Mute Status: Speak, Mute, or Always Speak. Always Speak keys cannot be muted.
  - Toggle Mute and Change Label edit an entry.
- **Defaults:**
  - Since JAWS 2019 the Tab key is not echoed by default, so Alt+Tab and Tab are no longer
    spoken; Space, Enter and Esc can be muted in the same list (Doc [W3]).
  - Freedom Scientific's guidance is to mute the "Letters" entry to stop letter echo while
    keeping numbers and symbols [W11 excerpt].
  - Space is spoken by default when typing echo includes characters [W13].
- **Chords are echoed by their labels:**
  - Freedom Scientific's Startup Wizard lesson transcript shows JAWS, with typing echo on,
    saying "ALT+H", "Z", "Space", "ALT+N" and "Enter" as the narrator pressed them (Doc [W9]).
  - So modifier chords are announced as label plus label (Inf: when typing echo is on).
- **Key Repeat** (User Options): when it is checked, a held key is repeated; it is cleared by
  default (Doc [S3]). What this changes in speech is not explained. **Open.**
- **Typing Interrupt** (User Options): typing keys interrupt speech; it is checked by default
  (Doc [S3]).
- **Insert Key Mode** (User Options): makes Insert a sticky key; it is cleared by default (Doc
  [S3]).
- **Keyboard Help:**
  - Insert+1 (laptop CapsLock+1), script `KeyboardHelp`, is a toggle;
  - while it is on, keys are described rather than run (Doc [W1], [W8], [K1]);
  - the key map has a `[Key Name Translations]` section for the key names it speaks [K1].

### Okur today

Okur has no key labels, no echo of chords or modifiers, no Key Repeat or Typing Interrupt
setting, and no Keyboard Help.

### Recording proposal

1. **Read Manage Key Labels on old-pc** (UI capture). This gives every default label and mute
   status, which is the data Okur needs.
2. **A chord-echo scenario,** with typing echo set to Characters (`desktop-editor.ps1`):
   - single keys: Home, End, Escape, Space, Enter;
   - chords: Ctrl+Home, Shift+End, Alt pressed alone.
3. **A Key Repeat scenario:** in the same editor, hold Right Arrow for 1.5 s (`key_down`, then a
   wait step, then `key_up`). Record it with Key Repeat cleared (the default) and checked.

---

## Item 4. Character reading, spelling and phonetics

### Commands (Doc [H4], [H5], [W1], [W8], [K1])

| Command | Desktop | Laptop | Script [K1] |
|---|---|---|---|
| Say Character | Numpad 5 | CapsLock+Comma | `SayCharacter` |
| Say Character Phonetically | Numpad 5 twice quickly | CapsLock+Comma twice quickly | (`SayCharacter`) |
| Say ASCII or Hexadecimal Value | Numpad 5 three times quickly | CapsLock+Comma three times quickly | (`SayCharacter`) |
| Say Prior / Next Character | Left / Right Arrow | CapsLock+M / CapsLock+Period | `SayPriorCharacter`, `SayNextCharacter` |
| Say Word | Insert+Numpad 5 | CapsLock+K | `SayWord` |
| Spell Word | Insert+Numpad 5 twice quickly | CapsLock+K twice quickly | (`SayWord`) |
| Say Prior / Next Word | Insert+Left / Right | CapsLock+J / L | `SayPriorWord`, `SayNextWord` |
| Say Line | Insert+Up | CapsLock+I | `SayLine` |
| Spell Line | Insert+Up twice quickly | CapsLock+I twice quickly | (`SayLine`) |
| Say Sentence | Alt+Numpad 5 | CapsLock+H | `SaySentence` |
| Say Paragraph | Ctrl+Numpad 5 | CapsLock+Ctrl+I | `SayParagraph` |
| Say To / From Cursor | Insert+Home / Insert+PageUp | CapsLock+Shift+J / L (or CapsLock+Home / PageUp) | `SayToCursor`, `SayFromCursor` |
| Spell To / From Cursor | the same, twice quickly | the same, twice quickly | |

These commands do not move the cursor, and they work with every cursor (Doc [H5]).

### What the documents say they do

- **Say Character twice:**
  - It says the current character phonetically. After it, Left and Right Arrow (laptop
    CapsLock+M and CapsLock+Period) also speak phonetically (Doc [H5], [H5L]).
  - **Three documents disagree about when this mode ends:**
    - the 2025 help: until any other key is pressed (Doc [H5]);
    - Freedom Scientific's teacher lesson: by pressing Numpad 5 once more (Doc [W8]);
    - the Settings Center help and the 2019 What's New: a double press "toggles this feature on
      and off" (Doc [S1], [W3]).
  - The July 2025 update fixed this mode in Notepad (Doc [W5]).
  - **Open:** what ends it, and whether Numpad 4 and Numpad 6 (the owner's habit) act like the
    arrows. They should, because JAWS does not tell the numpad from the extended keys by default
    (Doc [S4]).
- **Say Character three times:**
  - It says the character's value: ASCII by default, hexadecimal when Speak Character Value in
    Hex is checked (Graphics and Symbols; cleared by default) (Doc [H5], [S6]).
  - In hex mode, the help's check is that "a" gives "Character U+61HEX" (Doc [H7]).
  - Speak Character Value as Sequence of Multibyte Values (cleared by default) changes how
    Unicode values are given. The help's example is "u+627 hex" (Doc [S6]).
  - The default, decimal message is **Open**.
- **Phonetic on a pause:**
  - Setting: "When Pausing After Character Navigation, Speak Character Phonetically" (Settings
    Center, Text Processing, General), added in JAWS 2019 (Doc [S1], [W3]).
  - After moving by character, pausing on a letter makes JAWS spell it phonetically. The
    default pause is one and a half seconds.
  - The setting offers other delays and off; the list of values is not given.
  - The pause announcement does not happen while the manual phonetic mode is on (Doc [S1]).
  - **Unexplained evidence:** 207 has letters held for about 3.4 s between steps (s2-s9), and
    no phonetic word was ever recorded. **Open:** is the feature off on old-pc, or not used in
    the virtual cursor, or cancelled by the harness? The harness copies the speech history
    through FSAPI after each step (see §11).
- **The phonetic alphabet:**
  - The documents give NATO words in examples: "Juliet Alpha Whiskey Sierra" for JAWS, and
    "Romeo Oscar November" for RON (Doc [S1], [H2]).
  - **Open:** the full list, and what digits, symbols, capitals and non-ASCII letters give.
- **Spell Word:**
  - After it, Insert+Left and Insert+Right (laptop CapsLock+J and CapsLock+L) spell the prior
    and next word, "until some other keystroke is pressed" (Doc [H5]). The teacher lesson
    instead says pressing Say Word once ends spelling (Doc [W8]). **Open.**
- **Spelling options:**
  - Spell Phonetically Always (Text Processing, General; cleared by default): Spell Word and
    Spell Line spell phonetically (Doc [S1]).
  - Quick Settings calls the same choice Text Spell, Alphabetically (the default) or
    Phonetically (Doc [H2]; Inf: the same option).
- **Spell Alphanumeric Data** (Text Processing; Quick Settings calls it Alphanumeric Text):
  - Off (Read as Word) by default; also Spell, and Spell Phonetically;
  - the documents' example: "123RON" reads as "one hundred twenty-three Ron", "1 2 3 R O N", or
    "1 2 3 Romeo Oscar November" (Doc [S1], [H2]).
- **Smart Word Reading** (on by default; Doc [H2]): Say Word reads a whole unit (Doc [S1]).
  - Examples: "sister-in-law", "2:00 P.M." read as a time, a phone number.
  - It depends on the application.
- **Spelling rate:**
  - Voice Adjustment's Spell Rate Change Percent slows speech while spelling (All Context only;
    some synthesizers) (Doc [H1]).
  - Old-pc's profile has `SpellingVoice=*|0|-2%`, and `Persian.VPF` has `SpellRateDelta=-2`
    (Obs).
  - Inf: spelling is sent at a slightly lower rate. The recorder logs rate per fragment, so
    this can be measured.
- **Smart Navigation** (off by default) treats controls as single units. Inside a control,
  pressing Say Character, Say Word or Say Line twice quickly returns to the usual movement by
  character or word (Doc [H3]). The meaning of a double press therefore changes when Smart
  Navigation is on.

### What JAWS says when moving by character (Left / Right)

- **The names:** observed at Most in 207, 208 and 307 and kept in `pronounce.rs`.
- **Space:** a space gives "space".
- **Capitals:** a capital letter is spoken at a higher pitch with no word added. In 307/s35,
  "B" was sent at SAPI pitch +4 (Obs).
- **Lines:** a blank line gives "blank" (Obs; `speech.ftl` `blank`).

### Okur today

- **Commands:** `SayCharacter` and `SayWord` exist (`command.rs:44-45`, `cursor.rs:489-556`).
- **Keys:** only the laptop layout binds Say Word (CapsLock+K, `command.rs:583`).
  - Numpad 5, Insert+Numpad 5, CapsLock+Comma and CapsLock+Period cannot be bound: `Key` has no
    numpad keys and no comma or period (`input.rs:3-36`).
  - `pressed_twice` covers only Say System Time and Say Font (`command.rs:302-311`).
- **Not there:**
  - phonetic reading, phonetic mode, or phonetic on a pause;
  - Spell Word, Spell Line, or spelling rate;
  - character value;
  - Say Sentence, Say Paragraph, Say To / From Cursor;
  - Smart Word Reading, alphanumeric spelling.

### Recording proposal

**Harness first:**
- Add `numpad5`, `numpad4`, `numpad6` (the numpad keys with Num Lock off), and `comma` and
  `period` as US-position scan codes (§11, harness note 1).
- Check on old-pc with Keyboard Help (Insert+1) that JAWS reads them as Say Character and Say
  Next Character.

**Fixture:** the "spelling" line of the reading fixture (§12). It contains JAWS, 123RON, aB9,
é, € and sister-in-law. Use both the web version and the desktop version.

**Scenarios:**
1. **Say Character once, twice, three times.**
   - Run it on "a", "B", "9", "é", "€" and "😀". Use CapsLock+Comma (laptop) and Numpad 5
     (desktop): this also checks whether numpad commands work in the laptop layout.
   - For "three times", a second variant with Speak Character Value in Hex checked.
2. **Phonetic mode and how it ends.**
   - Numpad 5 twice.
   - Then Right, Right, Numpad 6, Numpad 4, Left.
   - Then Down, which is another key.
   - Then Right.
   - Then Numpad 5 once, then Right.
   - Each step is recorded, so the evidence shows where phonetic speech stops.
3. **Phonetic on a pause.**
   - One Right onto a letter, with `quiet_ms` 3000 and `settle_max_ms` 8000.
   - Run it in Chrome (virtual cursor) and in the desktop editor.
   - If nothing comes, record again with the setting set to off, to check whether the harness
     hides it.
4. **Spell Word and Spell Line.**
   - CapsLock+K twice on "JAWS", then CapsLock+L, CapsLock+J, then CapsLock+K once.
   - CapsLock+I twice on a short line ("Hi, Ada!").
   - Insert+Numpad 5 twice and Insert+Up twice for the desktop keys.
   - Then the same with Spell Phonetically Always checked (or Text Spell set to Phonetically).
5. **Smart Word Reading and alphanumeric data.**
   - CapsLock+K on "sister-in-law", "2:00 P.M." and the phone number, at the default (on) and
     off.
   - Down over "123RON" with Spell Alphanumeric Data at Off, Spell and Spell Phonetically.

---

## Item 5. Capitalization

### What the documents give

- **Indicate Capitalization** (Settings Center, Text Processing, General):
  - By default JAWS indicates capitals only when reading by character and when spelling words
    or lines (Doc [S1]).
  - Quick Settings' Caps Indicate offers On Character (the default); On Character and Word; On
    Character, Word, and Line; and Never (Doc [H2]; Inf: the same option).
- **How a capital is indicated:**
  - The Speech and Sounds scheme's Misc page has items Cap and AllCaps. Each can be spoken, with
    a word or a voice alias, or play a sound, change voice, or be ignored. They work together
    with the Text Processing option (Doc [S8]).
  - The voice side:
    - Voice Adjustment's capital pitch setting (the help spells it "Upper Case Pitch Case
      Percent"; All Context only; Eloquence and Vocalizer Expressive among the synthesizers that
      support it) (Doc [H1]);
    - a support article gives its default as 20 [W14 excerpt].
  - Old-pc's files:
    - `UppercasePitchDelta=20` in `Persian.VPF`;
    - `SingleCapVoice`, `AllCapsVoice`, `SmallCapsVoice` = `*|20%|0` in the active profile
      (Obs).
  - Inf: the voice alias format is person|pitch|rate, and a capital is spoken by the same person
    20% higher. The pitch was observed: SAPI +4 for "B" (Obs, 307/s35).
- **Say All:**
  - Indicate Caps During Say All (Say All group; cleared by default) says "cap" before a single
    capital letter and "all cap" before a word in capitals (Doc [S5]).
  - Quick Settings calls it Capitalization, Indicate or Ignore, Ignore by default (Doc [H2]).
- **Other ways:** a support article describes speaking a word such as "cap", or playing a
  sound, through the scheme's Misc page [W14 excerpt].

### Okur today

- **There:** `pronounce::CAPITAL_PITCH` = 4 for a capital read alone
  (`pronounce.rs:39-51`).
- **Not there:** capital indication when reading by word or line; "cap" or "all cap"; a pitch
  setting.

### Recording proposal

Use the capitals line ("Ada READ me B x").
1. **At the default** (On Character):
   - Right onto "A" and "B";
   - CapsLock+K on "Ada" and on "READ";
   - CapsLock+I on the line;
   - CapsLock+K twice on "READ" (spelling);
   - Say All over the line;
   - Shift+A typed with echo set to Characters.
2. **The same at each Caps Indicate value:** On Character and Word; On Character, Word, and
   Line; Never.
3. **Say All with Indicate Caps During Say All checked:** this should give "cap" and "all cap"
   in both layers.

---

## Item 6. Word splitting and numbers

### What the documents give (all Settings Center, Text Processing, unless marked)

- **Mixed Case Processing** (checked by default): a capital inside a word starts a new word. The
  help's example is "SayAll" read as "Say All" (Doc [S1]).
- **Use Dictionary** (checked by default): words pass through the default and application
  dictionaries (Doc [S1]).
  - Dictionary Manager (Insert+D) edits them: per application, or `Default.jdf` after
    Ctrl+Shift+D (Doc [D1]).
  - A rule can be case sensitive, and can match a root word followed by `*`.
  - A rule can be limited to one language, synthesizer or voice.
  - A rule can play a sound or speak in another language (Doc [D1]).
- **Number Processing:**
  - Values: Controlled by Synthesizer (the default); Single Digits ("123" as "one two three");
    Pairs ("1050" as "ten fifty"); Full Numbers (Doc [S1]).
  - JAWS 2025 added Controlled by JAWS, which processes ordinals and currency itself (Doc [W5]).
- **Speak Single Digits if Number Contains:**
  - Values: Five or more digits (the default), Six, Seven, Eight, Controlled by Synthesizer.
  - A long number is read digit by digit, for example 54321 as 5 4 3 2 1. This does not apply
    when the number touches letters, or contains a dash, colon, decimal point or slash.
  - The If Number Contains Dashes box (checked by default) makes dashed numbers single digits
    too (Doc [S1], [H2]).
- **Speak Dollars** (cleared by default): "$9.95" as "nine dollars and ninety five cents", and
  "$0.02" as "two cents" (Doc [S1]).
- **Numeric Date Processing:**
  - Values: No Translation (the default); Some Translation (03-16-00 as "March sixteenth two
    thousand"); Extended Translation (03-16 as "March sixteen", and 1/3 as "January three").
  - It follows Windows' regional date separator (Doc [S1]). Old-pc's format is en-US
    (`verification-loop.md` §3).

### Evidence at old-pc's defaults

The synthesizer was given:
- **Words split by case:** "MenuBar" as "Menu Bar", "PowerShell" as "Power Shell", "AppData" as
  "App Data" (323/s7), "ÜstKrktr" as "Üst Krktr" (audit §3.2).
- **Words split from digits:** "System32" as "System 32" (321/s5), "UTF-8" as "UTF dash 8"
  (314/s11).
- **Numbers and times left as they are:** "1.2 MB", "3:04:05 AM", "2026".
- **A respelling:** "spacebar" as "space Bar". Inf: a dictionary entry.

### Okur today

- **Not there:** word splitting, number processing, dictionaries. The audit counts 13 spoken
  steps that differ for this reason (§3.2).
- **A trap for the implementation:** Okur's catalog keeps JAWS's history spelling ("MenuBar"),
  which is right for the history layer. The spoken step must split it the way JAWS does.

### Recording proposal

1. **Defaults:** read every line of the mixed-case and numbers part of the fixture.
   - Web: Down in the virtual cursor.
   - Desktop: CapsLock+I in the editor.
2. **Mixed Case Processing off,** on the same lines.
3. **Number Processing:** Single Digits, Pairs, Full Numbers, Controlled by JAWS, on the numbers
   lines only.
4. **Speak Dollars on, and Numeric Date Processing at Some and at Extended,** on the dates and
   money line.

---

## Item 7. Verbosity levels

### What the documents give

- **Verbosity Level:**
  - Values: Beginner (the default), Intermediate, Advanced (Speech Verbosity group). Quick
    Settings calls it User Verbosity (Doc [S7], [H2], [W7]).
  - Beginner says the most. Intermediate and Advanced say less, and each level can be
    customized.
- **Per level, a Preferences dialog with two parts** (Doc [S7]):
  - **Items to be Spoken:** a list of output types, different for each level.
  - **Message Length:** Long or Short. It applies to JAWS messages, status messages, JAWS error
    messages, application start messages, smart help messages and help messages.
  - The help's example of message length is Copied against Copied selected text to clipboard.
    Old-pc's JAWS 2025 says "Copied selection to clipboard" (Obs, 311, 313, 314, 322, 323),
    which shows that some help examples are older than the build. Evidence wins.
- **The output types the help lists** (Doc [S7]):
  - Announce Position and Count When Navigating in Lists;
  - Application Start Messages;
  - Control Description, Control Group Name, Control Name, Control Type;
  - Dialog Name, Dialog Text, Document Name;
  - Error Message, Help Balloon;
  - Help Messages (they cannot be turned off);
  - Item Number, Item State;
  - JAWS Messages, Mouse Echo, Position Information, Screen Messages;
  - Select, Selected Item, Smart Help, Status Information;
  - Toasts, Tool Tips.

  Old-pc's JCF also has `ACCESS_KEY` ("Access Key"), which the list does not name. So the list
  in the help is not complete (Inf).
- **Enable Speech History** (checked by default): JAWS keeps the last 50 spoken items (Doc
  [S7]).
- **Related but separate settings:**
  - **Tutor messages:** Turn off Menu and Control Help; Announce Menu and Control Help; Announce
    Custom Messages Only (Doc [W7], [W9]).
  - **Web Verbosity Level,** per application: High, Medium (the default), Low (Doc [S10]). What
    old-pc's `VirtualCursorVerbosityLevel=2` stands for is **Open**.
- **Per-level defaults:** not documented. **Open.** They are read from the three Preferences
  dialogs.

### Okur today

- **There:**
  - `VerbosityLevel` (Beginner by default);
  - per-level modes for two output types, `AccessKey` and `Hint`;
  - message length, Long at Beginner and Short above (`settings.rs:173-320`;
    [`messages-and-verbosity.md`](messages-and-verbosity.md)).
- **Not there:** no dialog control and no key changes the level.

### Recording proposal

1. **Read JAWS's lists** (UI capture):
   - the Beginner, Intermediate and Advanced Preferences lists with their checked states, and
     their message lengths;
   - then switch one item and compare the JCF. This confirms that `[OutputModes]` holds one
     value per level, as `extension-sdk-proposal.md` §6.5 presumes.
2. **One scenario per level** on `desktop-dialogs.ps1`:
   - Tab through the controls, which gives tutor messages and positions;
   - a list (which gives position and count);
   - Ctrl+C on a selection (which gives message length).
   - Old-pc runs at Beginner, so record Intermediate and Advanced.

---

## Item 8. Other reading options

### 8.1 Say All by line, sentence or paragraph

- **Say All Reads** (Say All group):
  - Values: Line with Pauses; Line without Pauses (the default, per the Quick Settings topic);
    Sentence; Paragraph (Doc [S5], [H2]).
  - During Say All, Left and Right Arrow (and the Shift keys) rewind and fast-forward by this
    unit, and Page Up and Page Down change the rate (Doc [H5]).
  - Right Shift skips paragraphs in Paragraph mode (Doc [S5]).
- **The Say All scheme:** Say All Text With Sounds or Say All Text Only. The Quick Settings
  topic adds No Change, its default (Doc [S5], [H2]).
- **Okur (2026-10-21):** Say All Reads is a setting (`say_all_unit`: line, sentence,
  paragraph; the settings dialog's "Say All Reads"), and replay takes `SayAllMode`. Say All
  reads a page by the unit, and Right and Left move by it. A sentence is what Alt+Down reads
  [obs, 291]; a paragraph what Ctrl+Down reads. In a text field, by sentence it reads the
  sentences from the caret's; a field's paragraphs are its lines. Line with Pauses
  (`LinePauses`) is not built. How JAWS splits a page and a field by each unit waits
  for 677-681 (round 124) and 684-686 (round 125).
- **Record:**
  - Say All (CapsLock+A, and Insert+Down) over the Say All paragraph, at each unit. The
    recorder's fragments show where JAWS breaks the speech.
  - Right Arrow during Say All, once per unit.

### 8.2 Blank lines

- **Say Blank Lines During Say All:** cleared by default (Doc [S5]).
- **Skip Blank Lines While Navigating** (Web/HTML/PDFs, Reading): checked by default since JAWS
  2020; the virtual cursor skips blank lines (Doc [S10], [W4]).
- **Word's Include Blank Paragraphs For Paragraph Navigation:** off by default (Doc [W6]).
- **Indicate New Lines When** (Text Processing; four check boxes) works together with the
  scheme's NewLine item (Doc [S1], [S8]).
- **Okur:** says "blank" (`speech.ftl`). It has no skip setting.
- **Record:** Down over the fixture's blank lines in Chrome, with Skip Blank Lines on and off;
  Down over a blank line in the desktop editor; and Say All with Say Blank Lines checked.

### 8.3 Indentation

- **Say Indented Characters** (Text Processing; cleared by default): indentation is indicated
  by the scheme's Indentation rules (Doc [S1]).
- **Quick Settings' Indentation:** Ignore by default. With the Classic scheme, it is indicated
  by saying the number of spaces (Doc [H2], [S8]).
- **Okur:** none. 315's lines begin with four spaces, and the spoken text has none (Obs).
- **Record:** CapsLock+I and Down over the indented lines (4 spaces, a tab) in
  `desktop-notepad.ps1`, with indentation indicated and ignored.

### 8.4 Text attributes and formatting

- **The scheme's pages:** Attributes, Font Name, Font Size and Color can speak, play a sound or
  change voice for bold, italic and so on (Doc [S8]).
- **Commands:** Say Font (Insert+F; twice shows it in the viewer) and Say Color (Insert+5) (Doc
  [H5]).
- **Web:** Indicate Element Attribute follows the scheme's Attributes page (Doc [S10]).
- **Okur:** has Say Font.
- **Priority:** lower than the items above.

### 8.5 Spelling and grammar errors

Proofing group (Doc [S1]):
- **Detect Spelling While Navigating:**
  - Values: Off; Play Sound; Say Misspelled (the default, which says "misspelled").
- **Detect Spelling While Typing:** a sound; checked by default.
- **Speak Spelling Error Count:** cleared by default.
- **Grammar and Advanced Proofing:** checked by default, except in Word and Outlook.
- **Apply Options During Say All:** cleared by default.
- **Announcements in web editors such as Google Docs:** comments, inserted, deleted, marked and
  suggested text.

**Okur:** none. **Record later:** a text area with "teh" and "fxo" in Chrome, which marks
spelling errors; Down in forms mode at the default.

### 8.6 Reading what is on the screen

- **Screen Echo** (Insert+S): Echo No Text, Echo Highlighted Text (the default), Echo All Text
  (Doc [S3], [H4]).
- **Document Presentation Mode:** Simple Layout (the default) or Screen Layout. Smart Navigation
  (Insert+X) switches to Screen Layout (Doc [S10]).
- **Maximum Line Length** applies to Screen Layout (Doc [S10]).
- **Priority:** lower. Screen Echo matters for desktop applications.

### 8.7 Language detection

- **Detect Language** (Text Processing, Language Processing):
  - Values: Based On Markup of document and web page; Based On Text (Vocalizer Expressive
    only); Based on AI predictions (new in JAWS 2025); Off (Doc [S1], [H10]).
  - Generalize Dialect (checked by default) keeps one voice for dialects of one language. A
    separate choice sets which voice reads numbers.
- **Detect Keyboard Language** (checked by default): JAWS follows the input language.
- **Quick Settings' Language Detect Change:** on by default in Word, off elsewhere (Doc [H2]).
- **Select a Language:** Ctrl+Win+L.
- **Old-pc:** `LanguageDetection=0` (Obs).
- **Okur:** none. It is planned for M5.
- **Priority:** low. The recorder voice is English only.

### 8.8 Voice contexts, and a different voice for links or headings

- **Voice contexts:** PC Cursor, JAWS Cursor, Keyboard, Tutor and Message, each with its own
  person, rate, pitch and punctuation (Doc [H1]).
- **Voice Aliases:** named combinations of person, pitch and rate. The help suggests a
  "HeadingVoice" (Doc [S9]).
- **Schemes:** a scheme's Control Type page can give links or headings a voice alias, a sound,
  or other words (Doc [S8]).
- **Choosing a scheme:**
  - Alt+Insert+S switches schemes for the session, and also turns on training mode, which names
    each sound (Doc [S8], [W1]);
  - Classic is the default and cannot be deleted.
- **Okur:**
  - the role marks (`\u{1d}…\u{1e}`) model JAWS's separate role voice;
  - `speak_echo` is a separate channel;
  - there are no contexts, aliases or schemes.
- **Record:**
  - the per-context pitch test (`extension-sdk-proposal.md` §6.4), which tells which context
    each kind of speech uses;
  - a scheme that changes voice for links (made with Alt+Insert+S), over `news-article.html`
    links. The recorder logs voice changes as pitch. It cannot record sounds.

---

## Item 9. Contextual settings: per application and per site

The owner has decided that these contextual settings are core Okur features. They come before
the extension SDK. They also decide where every setting in items 1 to 8 is stored and looked
up, so this item is recorded early (§13, step 2).

JAWS has four layers of settings (Doc [H12], [W17], [P1]):
- **the factory defaults,** in the shared settings folder;
- **the user's defaults for all applications** (`Default.jcf`);
- **one application's settings;**
- **for web pages, one site's personalized settings.**

Quick Settings can also save settings for one document (Doc [H6]). Some settings have a
lifetime shorter than "saved".

### 9.1 Per application

How a user changes them:
- **Settings Center** (Doc [S12], [W7]):
  - It opens on the settings of the application in front. Its title names that application,
    for example "JAWS Settings Center - Notepad".
  - Ctrl+Shift+D loads the default for all applications. The title then reads "Default (All
    Applications)".
  - An Application combo box chooses any other application. Observed in Chrome: "Chrome, 17 of
    180", with Default (All Applications) at the top
    ([`jaws-dialogs-and-visual-features.md`](jaws-dialogs-and-visual-features.md)).
  - The 2015 lesson repeats this for typing echo, verbosity, screen echo and Speak Dollars. Its
    advice is to change Speak Dollars in Excel only.
- **Quick Settings** (Insert+V) (Doc [H6]):
  - It changes only the application in front, or the open document. It cannot switch to the
    default file; Settings Center is for changes to all applications.
  - The application's own groups come first. In Chrome these are Virtual Cursor Options, Forms
    Options, General Options and Personalized Web Settings (Obs,
    [`jaws-dialogs-and-visual-features.md`](jaws-dialogs-and-visual-features.md)).

Lifetimes, in Quick Settings (Doc [H6]):
- **The three choices:** the Applications key on an item chooses how long a change lasts:
  - saved, the default;
  - restored when JAWS exits;
  - restored when the focus moves to another application or document.

  The lifetime must be chosen before the value is changed.
- **The help's example:** typing echo set to None, restored when the focus changes. While the
  focus stays in the Word document, nothing is echoed. Back in that document after another
  application, the echo is Characters again.
- **Document settings:** they have no lifetime choice. They are always written to disk.

Where the settings are stored (Doc [H6], [H12], [W17]):
- **Folders:**
  - application settings go in the user settings folder, which "Explore My Settings" opens;
  - the factory defaults are in the shared settings folder, which "Explore Shared Settings"
    opens.
- **File names:**
  - An application's file is named after the executable without its extension, for example
    `winword`.
  - Or it uses an alias from `Confignames.ini` in the shared settings folder. The Script
    Manual's example: `ieframe` maps to Internet Explorer, so the file is `Internet
    Explorer.jcf` [W17].
  - This is why old-pc's file is `Chrome.JCF` (Inf).
- **Insert+Q** (Say Active Configuration Name) says the settings file in use and the
  application's executable name. Pressed twice quickly, it shows them in the Virtual Viewer
  (Doc [H4], [W17]).
- **Manage Application Settings** (JAWS window, Options):
  - It lists DEFAULTS and every application with customized settings.
  - Clearing an application's box stops JAWS using its settings without deleting them, and
    clearing DEFAULTS makes JAWS use the factory defaults.
  - Remove Selected deletes an application's settings (Doc [H11], [W18]).

How a setting falls back:
- **Documented:** JAWS loads an application's files "on top of the set of default files
  already in memory" [W17].
- **Not documented:** how JAWS resolves one setting's value between the files.
- **Inf:** a setting the application's file does not hold takes the user's `DEFAULT.JCF` value,
  and failing that the shared factory default. A JCF holds only options that differ from the
  defaults (Inf, from old-pc's files).
- **Obs, supporting this:** old-pc's `Chrome.JCF` holds only `VirtualCursorVerbosityLevel` and
  `AllowWebAppReservedKeystrokes`. Yet in Chrome old-pc's JAWS uses manual forms mode, which
  only `DEFAULT.JCF` sets (`AutoFormsMode=0`; `settings.rs:332-335`). The default file's value
  reaches Chrome.
- **Dictionaries:** they layer the same way. There is an application `.jdf` and a
  `Default.jdf`. A user's dictionary file overrides the shared file of the same name (Doc [D1]).

Settings that exist only in the default file (Doc):
- JAWS Key for Current Layout, Initial State of Num Lock and Allow On Screen Keyboards [S4];
- Allow Wave Files with Software Synthesizers [S13];
- Enable Flexible Web [S10].

Focus Loss Announcement is also saved to `Default.JCF` [H2].

Applications can have their own factory defaults:
- Language Detect Change is on in Word and off elsewhere [H2].
- Grammar and Advanced Proofing is cleared only in Word and Outlook [S1].
- Inf: JAWS ships application JCFs in its shared settings folder. Their contents are a program
  file's data and are not read here. The help states the defaults that matter.

Other per-application choices:
- **Voice profile:** Select a Voice Profile (Ctrl+Insert+S) sets one for all applications and
  one for the application in front. Use Default is the first entry (Doc [H9]).
- **Dictionary:** Insert+D opens the application's dictionary (Doc [D1]).

### 9.2 Per site: Personalized Web Settings

What the documents say (Doc [P1], [W16]):
- **What they are:** Personalized Web Settings change JAWS for one web site only. JAWS loads
  them again on every later visit.
- **Scope:**
  - They apply to every page under the domain or subdomain where the change was made.
  - A different subdomain does not share them. The help's example: settings made on
    shopping.yahoo.com do not apply to mail.yahoo.com.
  - Surf's Up says "all pages of a given domain".
- **A notice:** JAWS says so when you visit a domain that has personalized settings. **Open:**
  the words.
- **How to make one:**
  - Open Quick Settings in the browser.
  - Press P in the tree to reach the Personalize Web Settings group.
  - Change items with Space, then choose OK.
  - The change is saved at once. These settings have no lifetime choice.
- **Where they are stored:**
  - in a JSI file named after the domain, in the `PersonalizedSettings` folder of the user
    settings;
  - the documents' examples are `FreedomScientific.com.jsi`, `Vispero.com.jsi` and
    `Google.com.jsi`.
  - The examples drop "www." although the scope example uses www.freedomscientific.com.
    **Open:** how JAWS turns a host name into the file name.
- **Sharing:** copy the JSI file into another user's `PersonalizedSettings` folder [W16].
  Settings exports include it [H13].
- **What can be personalized.** The documents give examples:
  - page refresh;
  - graphic links;
  - heading announcement;
  - Navigation Quick Keys;
  - forms, frames, lists and long quotations;
  - inline frames;
  - image map links.
- **The HTML Options topic** [P2] says the settings it describes can be applied to specific
  sites. It describes:
  - Virtual Cursor Verbosity Level;
  - the forms options, such as Auto Forms Mode;
  - the general options: Smart Navigation, Document Presentation, Page Refresh, Navigation
    Quick Keys, live regions, automatic reading on load;
  - graphics, links, headings and frames, and tables.

  The exact list is what Quick Settings' Personalize Web Settings group holds. **Open:** read
  it on old-pc.
- **Other things kept per domain or page:**
  - Custom labels are kept per domain in the same kind of JSI file [P3].
  - PlaceMarkers belong to one URL, or to the whole domain with "Define for All Pages on
    Current Domain" [P4].
  - Custom PlaceMarker Page Summary appears in both the Virtual Cursor Options and the
    Personalized Web Settings groups [P4].
  - Flexible Web rules apply to a page, a site or all sites
    ([`jaws-dialogs-and-visual-features.md`](jaws-dialogs-and-visual-features.md) §6).

How a site setting falls back:
- **Not documented.**
- **Inf:** the site's JSI first, then the browser's JCF (`Chrome.JCF`), then `DEFAULT.JCF`,
  then the factory default.
- For settings there is no page level: a site is the smallest unit. Only PlaceMarkers and
  Flexible Web rules work per page.
- Test both points (§9.5).

**Local files:** the corpus fixtures are `file:///` URLs. A local file has no host, and what
JAWS does with Personalize Web Settings on such a page is **Open**. A per-site test therefore
needs pages served over HTTP from two host names (§9.5).

### 9.3 Which reading settings can be set where

| Setting | All applications | Per application | Per site | Other layers | Source |
|---|---|---|---|---|---|
| Punctuation | yes (Settings Center, voice profile) | yes: Quick Settings; an application's voice profile | not documented (Inf: no) | per voice context, per language | [H1], [H9], [W7] |
| Typing echo | yes | yes | not documented (Inf: no) | Insert+2 for the session | [H6], [W7] |
| Verbosity level, output types, message length | yes | yes (Restore Factory Settings works per JCF) | not documented | | [S7], [W7] |
| Web verbosity level | yes | yes (Obs: `Chrome.JCF`) | yes (Inf, from [P2]) | Configure Web Verbosity Levels | [S10], [P2] |
| Auto Forms Mode | yes (Obs: `DEFAULT.JCF`) | yes (Chrome's Forms Options) | "forms" are listed | | [P1], [P2] |
| Headings, graphics, graphic links, image map links, inline frames | yes | yes | yes | Speech and Sounds scheme | [P1], [P2], [W16] |
| Page Refresh | yes | yes | yes | | [P1], [P2] |
| Navigation Quick Keys | yes | yes | yes | | [S4], [P1] |
| Document Presentation, Smart Navigation | yes | yes | Inf yes | Insert+X for the session | [P2], [S10] |
| Skip Blank Lines, read the page when it loads | yes | yes (Inf) | Open | | [S10] |
| Say All unit, Caps Indicate, indentation, numbers, Text Spell, Smart Word Reading | yes | yes (Quick Settings lists them) | Open (Inf: no) | | [H2] |
| Voice profile | yes | yes | no | per language, per voice context | [H9] |
| Dictionary | `Default.jdf` | application `.jdf` | no | per language, synthesizer, voice | [D1] |
| Speech and Sounds scheme | yes | Open | Open | Alt+Insert+S for the session; Say All scheme | [S8], [S5] |
| Custom labels, PlaceMarkers | | | per domain; PlaceMarkers also per URL | | [P3], [P4] |
| JAWS key, Num Lock, Flexible Web on or off | yes, only here | no | no | | [S4], [S10] |

### 9.4 Okur today

- **One global settings struct.** `Settings` (`settings.rs:322-354`) is saved through
  `SettingsStore` (`settings.rs:356-383`). It has no application or site layer and no
  lifetimes. Okur's settings dialog opens on both Insert+6 and Insert+V (`dialogs.ftl`).
- **Parity replay reads only `DEFAULT.JCF`** (`crates/okur-corpus/src/parity.rs:613-656`).
  `Chrome.JCF`'s `VirtualCursorVerbosityLevel` is recorded in the provenance but not applied.
- **The design already exists.** The extension SDK proposal plans scopes Global, Application
  and Site, with lifetimes Saved, UntilExit and UntilFocusMoves
  ([`extension-sdk-proposal.md`](extension-sdk-proposal.md) §8.1-8.2).
  - Its lookup order: until the focus moves, until Okur exits, the site, the application,
    global, then the default.
  - Its storage: `applications/<id>.json` and `sites/<host>.json`.

  The documentation agrees with this order as far as it goes.
- **Open questions the documentation leaves for that design:**
  - **The application's id:** JAWS uses the executable's base name or a `Confignames.ini`
    alias. Okur needs an equivalent that is stable on macOS too.
  - **The site's key:** JAWS uses the host, including subdomains. Whether "www." is dropped is
    open.
  - **Default-only settings:** settings JAWS keeps only in the default file must refuse an
    application scope.
  - **Per-application factory defaults:** JAWS has them, as for Word.
  - **Voice settings:** per voice context and per language they are part of the profile's
    structure, not scopes.
  - **The session toggles:** how long Insert+2 lasts (§1) decides whether it is UntilExit or
    UntilFocusMoves.

### 9.5 Recording proposal

Harness first:
1. **Serve fixtures over HTTP under two host names.**
   - Use a small static server on old-pc. It could be an `okur serve <dir> --port N`
     subcommand, in the spirit of AGENTS.md §3's harness tools, or a PowerShell `HttpListener`.
     A listener for any host name needs a URL reservation, which an administrator makes, and
     old-pc elevates silently.
   - Reach the same folder as `http://site-a.localhost:N/` and `http://site-b.localhost:N/`.
     Chrome sends `*.localhost` to the loopback address (Inf: Chrome's behavior; check on
     old-pc).
   - If that fails, use `localhost` and `127.0.0.1`.
   - The pages are byte-identical; only the host differs. A difference in speech can then come
     only from per-site settings.
   - `run-scenario.ps1` hashes only `file:///` fixtures (lines 354-395). Map a served URL to its
     file for `fixture_sha256`.
2. **Record the settings in the provenance, in Okur's own terms.**
   - The scenario names the settings it asks for: the site or application, each setting and its
     value. The provenance records them, and the target application's JCF (the name Insert+Q
     gives, for example the Notepad file) beside `DEFAULT.JCF`, `Chrome.JCF` and the VPFs
     (`run-scenario.ps1:359-404`).
   - JAWS's per-site files are not copied into the corpus (the owner's decision, 2026-10-19).
3. **Make the changes in JAWS's own dialogs, once per run.**
   - The changes are made in Quick Settings or Settings Center: elevated JAWS windows, so keys
     go through `elev.ps1` or `TypeKey`.
   - Make them before the first trial, never inside a trial.
   - Afterwards delete the JSI JAWS created, and put back the JCFs from a copy. Record the JCFs
     before and after.

Fixture: `corpus/fixtures/site/index.html` and `site/second.html`. Each page has:
- a title naming the page;
- an `h2`;
- a text link and a graphic link with alt text;
- a graphic link without alt text;
- an image map with two areas;
- an edit field;
- an inline frame holding a heading.

Each scenario reloads one URL, so the scenarios below each name the page they load.

Scenarios:
1. **Read the Personalize Web Settings group** (UI capture) on `site-a`. Also try Quick
   Settings on `characters.html` (`file:///`): is the group offered, and which JSI file
   appears?
2. **A per-site override.**
   - Setup: on `site-a/index.html`, Personalize Web Settings, Headings Announce Off, and
     Graphical Links Show None. Save.
   - Record three scenarios, each with the steps: page load, Down through the page, H,
     Insert+Q.
     - `site-a/index.html`: what JAWS says on arrival, then the page without heading and
       graphic-link announcements;
     - `site-a/second.html`: the same host, so the documents say the override applies;
     - `site-b/index.html`: defaults.
   - Then remove the JSI, put back the defaults, and confirm with one scenario on
     `site-a/index.html`.
3. **Fallback: site, then application, then default.** On `site-a/index.html` and
   `site-b/index.html`:
   - **Step one:** keep site A's override, and set Headings Announce to On (without the level)
     for Chrome in Settings Center. Expected:
     - A: no heading announcement;
     - B: "heading" without the level.

     This shows that the site beats the application.
   - **Step two:** remove Chrome's value, and set On for all applications (Ctrl+Shift+D).
     - Expected: B follows the default, because `Chrome.JCF` lacks the key; A keeps its
       override.
     - This shows that the application beats the default only where it has a value.
4. **Per application** (desktop).
   - Setup: in Notepad, Quick Settings, Punctuation set to All (saved, so it is written to
     Notepad's JCF).
   - Record CapsLock+I on `desktop-notepad.ps1`'s code line, and on `desktop-editor.ps1`'s code
     line (which stays at Most). Add Insert+Q in each.
   - Put the settings back afterwards.
5. **Lifetimes.**
   - In Notepad, Quick Settings: Typing Echo set to Characters with "Restore settings when focus
     changes". Type a letter, Alt+Tab away and back, then type again: the help's own example.
   - Insert+2 in Notepad, then Alt+Tab to the WinForms editor and type. This settles §1's
     conflict about how long Insert+2 lasts.

---

## 10. What Okur has, in one table

| Setting or command | JAWS | Okur | Where |
|---|---|---|---|
| Keyboard layout, JAWS key | Settings Center | yes | `settings.rs:12-83` |
| Typing echo | 4 values, Insert+2 | Characters, None; no key | `settings.rs:99-122`, `text_feedback.rs:56` |
| Punctuation level | 4 levels, per context, Insert+Shift+2 | none | (planned, SDK proposal §6.3) |
| Symbol names, read alone | per level and context | Most only | `pronounce.rs:10-125` |
| Symbol names in running text | per level and context | none | audit §3.1 |
| Repeated characters filter | Three by default | none | |
| Capital pitch, read alone | Voice Adjustment, alias | fixed +4 | `pronounce.rs:39-51` |
| Caps Indicate, "cap" in Say All | 4 values; check box | none | |
| Say Character, Say Word | Numpad 5, Insert+Numpad 5, laptop keys | commands; only CapsLock+K bound | `command.rs:44-45, 583` |
| Phonetic, spelling, value (double and triple press) | yes | none | `command.rs:302-311` |
| Phonetic on pause | 1.5 s default | none | |
| Mixed case, numbers, dates, dollars, dictionary | Text Processing, Dictionary Manager | none | |
| Smart Word Reading, alphanumeric spelling | on; off | none | |
| Verbosity level, output types, message length | Beginner by default, about 25 types | level; 2 types; length | `settings.rs:173-320` |
| Say All unit, blank lines in Say All | Line without Pauses; off | by line | `cursor.rs:601-617` |
| Skip blank lines (web) | on | says "blank" | `speech.ftl` |
| Indentation | Ignore | none | |
| Key labels, key repeat, typing interrupt | lists and boxes | none | |
| Voice contexts, aliases, schemes | yes | role marks only | `announcement.rs` |
| Language detection | markup by default; old-pc off | none | (M5) |
| Per-application settings | an application JCF over `DEFAULT.JCF`; Insert+Q | one global `Settings` | `settings.rs:322-383`; SDK proposal §8.2 |
| Per-site settings | a JSI per domain (Personalize Web Settings) | none | SDK proposal §8.2 |
| Lifetimes (until exit, until the focus moves) | Quick Settings, session toggles | none | SDK proposal §8.1 |
| Replay of the recorded contextual files | | `DEFAULT.JCF` only | `parity.rs:613-656` |

The message catalog (`crates/okur-core/locales/en-US/`) has no message for any of these
toggles. It has no level names, no "cap" or "all cap", no "misspelled", no phonetic words and
no character-value message. Each needs a catalog entry, with JAWS's words as recorded.

## 11. Harness notes before recording

1. **Key names** (`skills/jaws-reference/scripts/run-scenario.ps1:130-146`).
   - The key table has no numpad keys, and no comma or period. Add:
     - `numpad5`: VK_CLEAR, not extended, with Num Lock off;
     - `numpad4` and `numpad6`: the arrow keys, not extended;
     - `comma` and `period`.
   - Old-pc's keyboard layout is Turkish Q (provenance), and JAWS processes keys by scan code
     unless Use Virtual Key Codes is checked; it is cleared by default (Doc [S4]).
   - So JAWS's "Comma" is most likely the key at the US comma position (Inf).
     `Resolve-Key` maps virtual keys through the active layout (`MapVirtualKey`), which on
     Turkish Q may give another physical key for VK_OEM_COMMA. Send US scan codes for these
     keys (comma 0x33, period 0x34).
   - Check with Keyboard Help (Insert+1) on old-pc.
2. **Choosing a setting's value for a scenario.**
   - The quickest way is a toggle key inside the trial, but those settings last until JAWS
     restarts (or until the focus changes), so the next trial starts from a different value.
   - The toggle scenarios proposed here press the key until it returns to the start.
   - For one scenario per value, write the user's `DEFAULT.JCF` or the active `.VPF` and
     restart JAWS before the run, as was done for the rate on 2026-10-18
     ([`verification-loop.md`](verification-loop.md) §3). Restore the files afterwards.
     `run-scenario.ps1:359-404` already records these files in each evidence file's provenance,
     so the evidence states the value.
   - The values must first be learned by changing them in JAWS's dialogs and comparing the
     files (items 1, 2 and 7).
   - Quick Settings changes can be set to last only until the focus changes (Doc [H6]). Avoid
     them for recordings.
3. **Timing.**
   - Settling ends after 500 ms of silence. A step can set `quiet_ms`
     (`run-scenario.ps1:573-574`).
   - Phonetic on a pause needs a quiet period longer than 1.5 s.
   - After a step settles, the harness copies the speech history through FSAPI. That may cancel
     a pending phonetic announcement (Inf, from the absence of any phonetic word in 207).
     Compare a run where the step's `quiet_ms` covers the pause.
   - The recordings cut short by the harness (audit §3.3) matter more here: spelling and Say All
     send many fragments with pauses between them. Fix the cut first, or give these steps a long
     `quiet_ms`.
4. **Typing fixtures reset their text per trial.** Desktop fixtures are relaunched before each
   trial. A page's text area may be restored by Chrome after a reload (Inf). Use
   `autocomplete="off"` and check the first step of each trial.
5. **JAWS's dialogs** (Settings Center, Voice Adjustment, the Customize Punctuation and Manage
   Key Labels lists) run elevated. Keys go through `elev.ps1`
   ([`verification-loop.md`](verification-loop.md) §2). Check that `okur-lab snapshot` can read
   their lists, or use JAWS's own reading of them (Insert+Up on each row) recorded as a
   scenario.
6. **Contextual settings** (item 9, §9.5):
   - Serve fixtures over HTTP under two host names, because a `file:///` page has no host.
   - Record the settings the scenario asks for, and the target application's JCF, in each
     evidence file's provenance (item 2 above).
   - Parity replay must then apply those application and site settings, not only
     `DEFAULT.JCF` (`parity.rs:613-656`). Otherwise per-application and per-site evidence would
     replay with the wrong settings.

## 12. A fixture for these recordings

Proposed: `corpus/fixtures/reading-settings.html`, plus a desktop twin built like
`desktop-editor.ps1`, with the same lines in a multi-line edit field.
- **Content:** each line in its own `<p>`; the blank and indented lines in a `<pre>`; and a
  `<textarea autocomplete="off" spellcheck="true">` with the same text, for typing and
  forms-mode reading.
- **Rules:** ASCII except line 6, no personal data, and `example.org` addresses only.

```
1  sentence:  Hello, world. Is it done? Yes! Wait; maybe: no.
2  address:   Mail ada@example.org or see https://example.org/a-b_c?x=1&y=2#top
3  path:      C:\Users\Ada\notes.txt and /usr/local/bin
4  math:      Price $9.95, 50% off; total = 3 * 4 + 1 - 2 / 5 ^ 2 ~ 7
5  code:      fn main() { let v = [1, 2]; v.len() } // done <ok> | a && b
6  quotes:    "double" 'single' “curly” ‘curly’ — em – en … ellipsis • bullet
7  repeats:   ===== ----- ..... !!!!! ##### ***
8  mixed:     MenuBar getElementById XMLHttpRequest iPhone McDonald PowerShell
9  digits:    System32 x86_64 COVID19 123RON 1st 2nd 3rd
10 numbers:   7 42 1234 54321 123456 1-800-444-4443 2.5 1,000,000 10:30
11 dates:     03-16-00 3/2/2002 1/3 $0.02 $1,250.50
12 capitals:  Ada READ me B x
13 spelling:  JAWS sister-in-law 2:00 P.M. aB9 é €
14 (blank)
15 indented:      four spaces
16 tabbed:    	one tab
17 paragraph: Say All reads this. It has three sentences. The last one ends here.
18 errors:    teh quick brown fxo
```

`characters.html` and `desktop-editor.ps1` stay as they are. Their evidence (207, 208, 307,
313) pins them by SHA-256.

For item 9, a second fixture, `corpus/fixtures/site/index.html` and `site/second.html`, is
described in §9.5. It is served under two host names.

## 13. Recording plan, in order

1. **Harness and reading JAWS's own tables (no evidence yet).**
   - Add the key names (§11.1) and the setting switch through files (§11.2).
   - On old-pc, capture these lists:
     - Customize Punctuation, with its Advanced dialog;
     - Voice Adjustment's punctuation per context;
     - Manage Key Labels;
     - the three verbosity Preferences lists.
   - Learn the `Punctuation`, `TypingEcho` and `[OutputModes]` encodings by comparing the files.
     Put every value back.
2. **Contextual settings** (item 9). The owner made them core features before the extension
   SDK, and they decide where every later setting is stored and looked up.
   - Harness: serve the site fixture under two host names, and record the application JCF and
     the settings each scenario asks for (§11.6).
   - Read the Personalize Web Settings group.
   - Record the per-site override on site A's two pages and on site B.
   - Record the site, then application, then default fallback.
   - Record the per-application punctuation in Notepad against the WinForms editor.
   - Record the two lifetime checks. The Insert+2 check also settles §1's conflict.
   - Try Personalize Web Settings on a `file:///` page.
3. **Punctuation:**
   - the level-cycle scenario (Insert+Shift+2 four times);
   - four per-level scenarios, web and desktop;
   - the per-context scenario.

   This is the largest daily-use gap: it covers 59 spoken steps (audit §3.1).
4. **Character reading:**
   - Say Character once, twice, three times;
   - how phonetic mode ends, with Numpad 4 and 6;
   - phonetic on a pause;
   - Spell Word and Spell Line, then Text Spell set to phonetic.

   The owner uses these every day.
5. **Typing echo:** the Insert+2 cycle, then Characters, Words, and Characters and Words; chord
   echo and key labels; Key Repeat.
6. **Word splitting and numbers:** defaults first (13 spoken steps, audit §3.2), then Mixed Case
   off and the Number Processing values.
7. **Capitalization:** each Caps Indicate value; Say All with "cap".
8. **Say All unit, blank lines, indentation.**
9. **Verbosity levels:** Intermediate and Advanced on the dialog fixture.
10. **Later:** spelling errors, Screen Echo, language detection, schemes and voice aliases.

Each recording leaves old-pc's settings as they were. Each is followed by `okur-lab corpus-manifest
--write` and `okur-lab parity --spoken`, in commits without engine code
([`verification-loop.md`](verification-loop.md); AGENTS.md §3).

## Sources

JAWS 2025 help (dev box, build 2025.2505.43.400, `C:\ProgramData\Freedom Scientific\JAWS\2025\Locale\enu\Help\`):
- [H1] `JFW.chm`, Voice Profile Adjustment Settings (`using_jaws/the_jaws_application_window/Voice_Adjustment.htm`).
- [H2] `JFW.chm`, Adjusting JAWS Options: the Quick Settings items and their defaults
  (`using_jaws/the_jaws_application_window/jaws_dialogs/adjust_jaws_verbosity_dialog.htm`).
- [H3] `JFW.chm`, JAWS Startup Wizard (`using_jaws/installing_jaws/Startup_Wizard.htm`).
- [H4] `JFW.chm`, Miscellaneous JAWS Keystrokes and its notes
  (`keystrokes/jaws_keystrokes/miscellaneous_jaws_keystrokes.htm`, `…-an.htm`).
- [H5] `JFW.chm`, JAWS Keystrokes for Reading Text and its notes
  (`keystrokes/jaws_keystrokes/jaws_keystrokes_for_reading_text.htm`, `…-an.htm`).
  [H5L] the laptop versions (`keystrokes/laptop_jaws_keystrokes/laptop_keystrokes_for_reading_text_and_information.htm`,
  `llaptop_keystrokes_for_reading_text_an.htm`).
- [H6] `JFW.chm`, Quick Settings Overview (`customizing_jaws_for_windows/Quick_Settings/Quick_Settings.htm`).
- [H7] `JFW.chm`, Adding Speech for Special Symbols (`using_jaws/controlling_speech/Adding_Speech_for_Special_Symbols.htm`).
- [H9] `JFW.chm`, Selecting a Voice Profile (`using_jaws/controlling_speech/application_specific_voice_settings.htm`).
- [H10] `JFW.chm`, What's New in JAWS 2025 (`whats_new/Whats_New.htm`).
- [H11] `JFW.chm`, Manage Application Settings (`using_jaws/the_jaws_application_window/Application_Settings.htm`).
- [H12] `JFW.chm`, The JAWS Program Group: Explore My Settings and Explore Shared Settings
  (`using_jaws/The_JAWS_Program_Group.htm`).
- [H13] `JFW.chm`, Importing and Exporting User Settings
  (`customizing_jaws_for_windows/Settings_Import_and_Export/Import_and_Export_Settings.htm`).
- [P1] `popapps.chm`, Personalized Web Settings (`internet/internet_explorer_5/personalize_ie.htm`).
- [P2] `popapps.chm`, HTML Options (`internet/internet_explorer_5/verbosity_ie.htm`).
- [P3] `popapps.chm`, Custom Labels (`internet/internet_explorer_5/Custom_Labels.htm`).
- [P4] `popapps.chm`, PlaceMarkers (`internet/internet_explorer_5/placemarkers.htm`).
- [S1] `SettingsCenter.chm`, Text Processing Group (`TextProcessing/text_processing_dialog.htm`).
- [S2] `SettingsCenter.chm`, Customize Punctuation Dialog (`Punctuation/customize_punctuation.htm`).
- [S3] `SettingsCenter.chm`, User Options Group (`User/user_options_dialog.htm`).
- [S4] `SettingsCenter.chm`, Keyboard Group (`Keyboard/keyboard_options_dialog.htm`).
- [S5] `SettingsCenter.chm`, Say All Group (`SayAll/say_all_options.htm`).
- [S6] `SettingsCenter.chm`, Graphics and Symbols (`GraphicsAndSymbols/graphics_and_symbols_dialog.htm`).
- [S7] `SettingsCenter.chm`, Speech Verbosity Group and Speech Output Types
  (`SpeechVerbosity/general_verbosity_dialog.htm`, `speech_output_types.htm`).
- [S8] `SettingsCenter.chm`, Speech and Sounds Schemes and Edit Scheme Dialog
  (`SpeechAndSoundsSchemes/speech_manager_dialog.htm`, `edit_scheme.htm`).
- [S9] `SettingsCenter.chm`, Voice Aliases (`VoiceAliases/voice_aliases.htm`).
- [S10] `SettingsCenter.chm`, Web/HTML/PDF Group (`Web-HTML-PDF/html_options_dialog.htm`).
- [S11] `SettingsCenter.chm`, Speech On Demand (`Speech_On_Demand/Speech_On_Demand.htm`).
- [S12] `SettingsCenter.chm`, Settings Center Overview (`Overview.htm`).
- [S13] `SettingsCenter.chm`, Synthesizer Group (`Synthesizer/synthesizer_options_dialog.htm`).
- [D1] `jdiction.chm`, Dictionary Manager Basics (`jdiction/dictionary_manager_basics.htm`).
- [K1] JAWS 2025 key map, `C:\ProgramData\Freedom Scientific\JAWS\2025\Scripts\enu\Default.JKM`:
  `[Common Keys]`, `[Laptop Keys]`, `[DESKTOP Keys]`, `[Key Name Translations]`.

Web:
- [W1] Freedom Scientific, JAWS keystrokes: https://support.freedomscientific.com/Content/Documents/Manuals/JAWS/Keystrokes.txt
  (through the fetch tool); the same table at https://www.freedomscientific.com/training/jaws/hotkeys/
  (browser pane).
- [W2] What's New in JAWS 2021 (Insert+Shift+2): https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2021
- [W3] What's New in JAWS 2019 (phonetic on a pause; Tab no longer echoed): https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2019
- [W4] What's New in JAWS 2020 (per-context punctuation fix; Skip Blank Lines): https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2020
- [W5] What's New in JAWS 2025 (phonetic mode in Notepad; Number Processing Controlled by JAWS):
  https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2025.
  [W5-2023] the 2023 page (typing echo Words fix): `…?version=2023`.
- [W6] What's New in JAWS 16 and 18 (punctuation per language; pauses at None; blank paragraphs
  in Word; password echo): `…?version=16`, `…?version=18`.
- [W7] Freedom Scientific teacher training, Changing Settings in JAWS (2015):
  https://doccenter.freedomscientific.com/doccenter/doccenter/rs11f929e9c511/2015-02-10_teacherstrainers-l3/02_ChangingSettingsInJAWS.htm
- [W8] Freedom Scientific teacher lessons, Read by Character, Word, Etc.:
  https://support.freedomscientific.com/teachers/lessons/4.1.4_ReadByCharacterWordEtc.htm, and Specialized Reading Keys,
  https://support.freedomscientific.com/teachers/lessons/4.1.5_SpecializedReadingKeys.htm
- [W9] Freedom Scientific teacher lessons, Transcript: Navigating the JAWS Startup Wizard:
  https://support.freedomscientific.com/teachers/lessons/2.3_StartupWizard-transcript-Navigating.htm
- [W11] Sight and Sound Technology, JAWS: adjust typing echo: https://sastltd.zohodesk.eu/portal/en/kb/articles/jaws-adjust-typing-echo
- [W12] Sight and Sound Technology, Changing JAWS punctuation announcement:
  https://sastltd.zohodesk.eu/portal/en/kb/articles/changing-jaws-punctuation-announcement
- [W13] Sight and Sound Technology, Spacebar not announced:
  https://sastltd.zohodesk.eu/portal/en/kb/articles/jaws-spacebar-is-not-announced-when-keyed
- [W14] Sight and Sound Technology, uppercase pitch and uppercase indication (search excerpts only):
  https://sastltd.zohodesk.eu/portal/en/kb/articles/jaws-change-or-increase-uppercase-pitch-announcement,
  https://sastltd.zohodesk.eu/portal/en/kb/articles/configure-jaws-to-play-sound-or-speak-a-word-to-indicate-uppercase-characters
- [W15] Deque, JAWS keyboard shortcuts: https://dequeuniversity.com/screenreaders/jaws-keyboard-shortcuts
  (it agrees with [W1] and adds nothing on these settings).
- [W16] Freedom Scientific, Surf's Up, Personalized Web Page Settings:
  https://support.freedomscientific.com/SurfsUp/17-PersonalizedSettings.htm (browser pane).
- [W17] Freedom Scientific, JAWS Script Manual, Appendix D: Naming JAWS Settings Files (public
  documentation, not script source):
  https://support.freedomscientific.com/Content/Documents/Other/ScriptManual/Appendix_D.htm
- [W18] Freedom Scientific technical support bulletin 1656, restoring factory defaults and turning
  off application settings (2017): https://support.freedomscientific.com/support/technicalsupport/bulletin/1656
