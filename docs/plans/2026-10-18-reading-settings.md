# Plan: JAWS's reading settings and contextual overrides, the core before the SDK (2026-10-18)

Follows [`docs/handoff/2026-10-18-helper-dialogs-round-2.md`](../handoff/2026-10-18-helper-dialogs-round-2.md).

## Why this round

The owner's decisions (2026-10-18):
- After the list dialogs, the next goal is punctuation and the other document reading and review
  settings. These are the settings users change most to fit JAWS to their tasks and habits.
- Beyond scripting, JAWS is customized through settings:
  - punctuation, typing echo, key repeat and verbosity, which decide what is heard;
  - per-application and per-site settings, which override them by context and fall back to the
    defaults;
  - the virtual HTML, document reading and review options.

  These are core Okur features. They must work before the extension SDK starts, or they risk
  being built as extensions.
- Every feature follows the same order: "find them in user manual / tutorials for jaws, test it
  in old-pc, and only then can you implement it". The owner's own description is a lead, never
  the specification.

## Inputs

- [`jaws-reading-settings-research.md`](../references/jaws-reading-settings-research.md): what
  JAWS's documentation says about each setting, Okur's state, and a recording plan (§13). Its
  claims are marked documented, evidenced, inferred or open.
- [`parity-gap-audit.md`](../references/parity-gap-audit.md) §3: the spoken-layer evidence
  already recorded. Punctuation accounts for 59 steps, word splitting for 13, and the harness's
  500 ms cut for 46.
- [`accepted-divergences.md`](../references/accepted-divergences.md): JAWS's accidents are
  written there, not copied.

## Owner decisions needed first

- May the shipped `.sbl` symbol files be read? JAWS's help tells users to edit them, but
  AGENTS.md §1.4 lists only `.jcf` and `.vpf` among readable settings files.
- May the `PersonalizedSettings\*.jsi` files be read, as user settings files?
- Review the two accepted divergences proposed by the audit (312's emoji, 313's Say Font).

## Steps

### 1. Harness

- Fix the spoken capture: it stops at the first 500 ms pause (audit §3.3). Read the recorder's
  records up to the next step's key, then re-record the affected scenarios.
- Add the numpad keys (5, 4, 6), comma and period to `run-scenario.ps1`. Old-pc's Turkish Q
  layout needs US scan codes (research §11.1).
- Choose a setting's value per scenario: write `DEFAULT.JCF` or the voice profile, restart JAWS,
  and record the values in provenance. Restore the files afterwards (research §11.2).
- Capture JAWS's own tables on old-pc: Customize Punctuation, Voice Adjustment's punctuation
  per context, Manage Key Labels, and the verbosity lists.

### 2. Settings with scopes

- Model Okur's settings as layers: default, then application, then site or page. A value is
  looked up from the most specific layer down to the defaults.
- Record JAWS's per-site override on two host names. A `file:///` page has no host, so serve the
  fixture over HTTP. Also record the site, application, default fallback and the lifetime of
  Quick Settings changes (research §9.5).
- Parity replay must apply the recorded application and site settings, not only
  `DEFAULT.JCF`.

### 3. Punctuation

- Levels None, Some, Most (the default) and All, and JAWS's symbol names in running text.
- Punctuation per context (reading, typing, selection: the audit's "Friday comma").
- Insert+Shift+2, which cycles the level.

### 4. Character reading

- Say Character (numpad 5) pressed once, twice (phonetic) and three times (the character's
  value).
- How 4 and 6 read afterwards, and what ends phonetic mode (the documents disagree).
- Phonetic reading on a pause; Spell Word and Spell Line.

### 5. Typing echo and key echo

- Off, Characters, Words, and Characters and Words; Insert+2.
- Key labels and key repeat.

### 6. Words, numbers, capitals

- Mixed Case Processing ("Menu Bar"), Number Processing and single digits.
- Caps Indicate values, and "cap" in Say All.

### 7. Say All, blank lines, indentation, verbosity

- The Say All unit, blank line and indentation announcements.
- The three verbosity levels and their message lists.

## Carried over from the dialog round

- The focus-and-forms-mode wording when a list's OK focuses an edit field: 225/s10, 228/s9,
  249/s3. JAWS repeats the field where Okur says "Type in text."
- 274/s2: JAWS adds the parent's name to a tree child ("level 1 Primary Navigation 1 of 1
  Banner").
- Lists not built yet: Smart Glance Highlights, Anchors, Frames, List Items, PlaceMarkers.
  Each needs its recording first.
- 205/s6: JAWS varies on repeating a radio button's position after Space.
- The audit's daily-use items: a multi-line native field read whole on focus (316/s3), and web
  application focus that leaves the virtual cursor behind (219).
