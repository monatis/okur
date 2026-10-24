# Plan: everyday-use gaps, round 29 (2026-10-24)

Follows [`../handoff/2026-10-24-word.md`](../handoff/2026-10-24-word.md). The owner's order stands:
- Okur's settings made as customizable as JAWS's, now first (the owner, 2026-10-24: "the
  biggest gap seems to be settings / customization now", scheduled after Word);
- the README's "Not yet" list, in its order.

Each item is found in JAWS's documentation, recorded on old-pc, then built. JAWS's accidents
become accepted divergences. old-pc always has a round queued behind the running one. Okur's own
words never use JAWS's name (AGENTS.md §1.5).

The workflow, every stretch:
1. this plan;
2. implementation;
3. proof with `okur-lab parity`;
4. a dated handoff and the next plan, with AGENTS.md pointing at them.

A stretch is a few hours or about thirty commits, not more.

## 0. old-pc

- okur-lab parity at `7d381cf0`: "8675/9239 steps match exactly; 251 accepted divergences".
- The JAWS window walk (`skills/jaws-reference/scripts/jaws-window-walk.ps1`) ran after round
  304; its files are in `C:\Users\ailabs\sr-harness\probe\jaws-window`. The Options menu came out
  in 2025's order (below); the walk's later parts reopen JAWS's window with Insert+J from Notepad,
  which `okur-lab keys` refused while JAWS's window was still in front: start each part from
  Notepad, or allow `jfw` too.
- **Queue next:** the settings walks of §1.1, then the scenarios their answers call for.

## 1. Okur's settings, redesigned as JAWS's

The design: [`../references/okur-settings-redesign.md`](../references/okur-settings-redesign.md).
Research: [`../references/jaws-settings-ui-research.md`](../references/jaws-settings-ui-research.md),
[`../references/jaws-customization-map.md`](../references/jaws-customization-map.md).

### 1.1 Record first (the design's §5)

- **JAWS's window, observed** (the walk, 2026-10-24): Alt says "Menu bar | Options | To navigate
  press Left or Right Arrow."; Down opens "Options menu | Basics... | To move through items press
  up or down arrow."; the 2025 Options menu is Basics..., Voices sub menu (Voice Adjustment...,
  Select a Voice Profile..., Add/Remove Voices...), Braille..., Early Adopter Program..., Manage
  Application Settings..., Restore to Factory Settings..., Remote Desktop..., Exit Alt+F4; Escape
  twice says "leaving menu bar". The other menus, Basics and Voice Adjustment: the walk's other
  files.
- Still to walk: Settings Center's tree in Default and in Notepad (its groups and settings),
  Shift+Space, Enter on a setting, F6, the Application box, Quick Settings' Tab order, Select a
  Voice Profile.
- Then scenarios for the spoken forms Okur will be compared with: Quick Settings' tree items
  ("level 1 | Typing Echo None | 1 of 4 | Editing Options"), Settings Center's search, the
  main window's menus.

### 1.2 Build, in the design's milestones

1. **The tree dialog**: `TreeItem` gains a value and a check state; `okur-ui` draws it as a
   native tree view; Settings Center and Quick Settings generated from the settings table with
   JAWS's groups, search, Space cycling, descriptions, Apply/OK/Cancel and the Application box,
   replacing today's dialog. Tests on the dialog model.
2. **Lifetimes and Last Changed Settings** in Quick Settings; the until-the-focus-moves layer.
3. **The Okur window and its menus** (Insert+J), Basics with its new settings, the quit
   confirmation from Basics' check box; the desktop icon from setup opens it (backlog,
   Installer).
4. **Voices**: voice contexts, profiles, Voice Adjustment, Select a Voice Profile.
5. **Startup Wizard**, Manage Application Settings, Restore to Factory Settings.

## 2. Word, what is left

- **Say All with the structure** (518/s20): JAWS announces headings, lists and tables while
  reading. The platform gives the caret line's structure only; Say All needs the document's
  (each paragraph's heading level, list and table cell), read once at its start.
- **Say Font's line spacing** (1034/s1, s3, known-gap test): Word's UI Automation text has no
  spacing. Try a fixture with explicit line and paragraph spacing to learn what JAWS's "Line
  spacing: N lines." measures, then find it (Word's object model is out: no automation).
- **Word's other views** (Web Layout, Read Mode, Draft, Outline): record Insert+T in each; only
  "Print View" is known.
- **The table's title reading in other tables** (a table without a header row, merged cells).
- **Bold, italic and underline in Say Font**: record a formatted word ("Hello **world**").
- **Excel** (521, its fixture written without automation, untried) and **Outlook**.

## 3. Convenient OCR, what is left

Plan 28 §1, unchanged: the Results Viewer's "Open in Word..." link, D in a PDF reader, a scanned
page's paragraphs, 1032/s22, the OCR voice, the replay's clock for Background OCR, research §9.

## 4. Smart Navigation and Screen Layout

On the same line builder (`screen_model::lines`), from plan 26 §2: Screen Layout (986, 987),
Smart Navigation's lines (991) and its units by part (992, 993).

## 5. Harness

- A speech clock for the replay: keys pressed during a Say All arrive where JAWS's speech was
  when they were pressed (505, 916, 988, 989).
- `preview-round.sh` defaults to the release build; default to the newest of the two.

## 6. Still open from before

- The hardening review's open findings
  ([`../backlog/performance-review-2026-10-24.md`](../backlog/performance-review-2026-10-24.md)):
  1, 3, 4, 6, 9a-c.
- The backlog ([`../backlog/open-items.md`](../backlog/open-items.md)), now open items only; new
  there: setup's desktop icon, JAWS's feedback sounds, Virtual HTML Features' PlaceMarkers item.
- Typing Interrupt: not observable in the speech history (988, 989).
- M3's remainder: `okur-ui`'s lowered token; the owner's sign-out and uninstall checks.
- Windows Terminal's output.

## 7. Owner

- Old-pc has a windowless Word (pid 9688, `/Automation -Embedding`) and a PowerShell (4200) left
  from the first Word fixture's automation; ending them was refused as interfering with old-pc's
  workloads. They hold nothing of the owner's; end them when convenient.
- Old-pc's Adobe Acrobat Reader is broken (no `AcroRd32.exe`): D in Acrobat can be recorded once
  it is reinstalled, which is the owner's decision.
