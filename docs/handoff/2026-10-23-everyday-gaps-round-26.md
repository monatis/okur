# Handoff: everyday-use gaps, round 26 (2026-10-23)

Follows [`2026-10-23-everyday-gaps-round-25.md`](2026-10-23-everyday-gaps-round-25.md) and covers
the commits after it (`f8f526a9` to this handoff's), under the plan
[`docs/plans/2026-10-23-everyday-gaps-round-25.md`](../plans/2026-10-23-everyday-gaps-round-25.md).
The next plan is
[`docs/plans/2026-10-23-everyday-gaps-round-26.md`](../plans/2026-10-23-everyday-gaps-round-26.md).
The owner asked for research on one README "Not yet" item, reading with the Okur cursor where
the screen has no text. It is in
[`docs/references/okur-cursor-screen-text-research.md`](../references/okur-cursor-screen-text-research.md),
and its plan is
[`docs/plans/2026-10-23-okur-cursor-screen-text.md`](../plans/2026-10-23-okur-cursor-screen-text.md).

## Decided without the owner (reversible)

- **965-968 stay pending** (`513d660e`). In round 269, 965 and 966's Say All outlasted the
  harness's wait in every trial, and 967 and 968 reached a one-word line. 980-983 record the
  same questions differently.
- **Typing Interrupt is not built.** In a field's Say All, JAWS takes a letter as a navigation
  quick key, with Typing Interrupt on and off alike (988, 989). Typing Interrupt changes only
  what is heard, which the speech history does not show.
- **Smart Navigation is still not built.** Round 280 shows what it changes, but not yet enough
  to build it (below).

## What was done

### Ribbons (WordPad's, and Office's NetUI)

UI Automation stops at the NetUI window, so Okur reads the ribbon's MSAA objects, which the
tracer now records (round 270: 970-973, and 523 again).
- **Alt and F10** (`70b796b3`) reach the selected tab and say "Upper Ribbon", with the tab's
  place among the visible tabs. Leaving the tabs says "Leaving menus".
- **Leaving the ribbon** (`f3140db3`) says "Leaving ribbons". WordPad's document is named as
  JAWS names it.
- **The lower ribbon** (`56c6162f`): a command is said with its groups, its place and its
  state, and a split button with its value.
- **Two accepted divergences** (`20fe0246`): JAWS's places are one key behind on a ribbon, and
  JAWS leaves WordPad's text unread after its ribbon.

### Fields and the virtual buffer

- **A selection's caret** (`5d5852d4`, from 974-976): the caret is the end of the selection
  that moved. Say Word follows the field's own words. Insert+R has its short messages.
- **A control's line in the buffer** (`82a66661`, from 977-978) holds its label and value, not
  its role, states or place. Ctrl+Right no longer lands on "Button".
- **A quick key in a field's Say All** (`110aeddb`, from 988-989): JAWS says "This feature is
  only available from within a virtual document, such as a page on the Internet." and reads on
  from the start of the line it was reading. Okur used to end the reading and type the letter.

### Settings

- **Reading Interrupt** (`33a71641`), from `[Options] ReadingInterrupt` (969, 982, 983). With
  it off, a bound key during a Say All changes nothing; one that only reports is answered after
  what is being said.
- **Verbosity levels customized** (`f2f22c91`): Okur Settings' Verbosity Levels group chooses,
  per level, the output types spoken and the message length, as JAWS's Settings Center
  customizes a level. The README's Not yet line for it went.

### The console

- **Screen Echo's rows** (`8eff3f78`): the visible ranges are read as rows. The replay passes a
  console's screen after it wrote (`b4169831`). 962-964 match in full.

### Startup (M3)

- **E1** (`5fe9c116`): the installed okur-sr, with UI Access, reads an elevated Notepad: its
  title, role, hint and typed text. The lab's okur.exe, without UI Access, reads only the
  window title.

## Found, not built

- **Smart Navigation** (round 280; JAWS's defaults are 990):
  - On, with Simple Layout (991): blank lines come between blocks, and the two radio buttons
    share one line, which says the first. That is what Screen Layout gives (986).
  - Controls alone (992): a table's arrows go by character.
  - Tables alone (993): a control is no longer one unit of the arrows.

  Preview: 25 of 66 steps match.
- **Screen Layout** (986-987, round 278): lines come from the page's geometry. A link stays
  inside its paragraph's line.
- **The replay has no speech clock.** A key pressed during a Say All arrives after the whole
  reading has been handed over (505, 916, 988, 989). The plan for round 3 already named this.

## Figures

Each is the named command's output at `3c07297f`:

- `okur-lab parity --write-baseline`: "JAWS parity (Ia2): 8358/8816 steps match exactly; 214
  accepted divergences not counted (0 scenarios could not run)". It was 8145/8473 at handoff
  25, before rounds 269-281 added 343 steps.
- `okur-lab invariants`: "Buffer invariants: 0 violation(s)".
- `cargo test --workspace`: 1395 passed, 0 failed, 199 ignored.

## The owner's research item

Reading with the Okur cursor where the screen has no text.

- **Sources:**
  - JAWS 2025's installed help;
  - its `DEFAULT.JCF` (`[OSM]`, `[OCR]`) and key map;
  - a research agent's survey of Freedom Scientific's What's New pages, FSCast, mailing lists,
    NVDA's source and Windows' OCR interface;
  - round 281 (994-998, imported in `3c07297f`).
- **Findings:**
  - JAWS's cursor reads a model of what applications draw. The old driver gives GDI's painted
    text only (994); the Accessibility Monitor adds a WinForms label and GDI+ text (995).
    Neither gives pictures.
  - Convenient OCR's W puts the recognized words in place among them, read with the JAWS
    cursor ("OCR started using OmniPage.", "finished.", "JAWS Cursor"; 996, and 997 with
    Microsoft's engine).
- **Documents:**
  - the research,
    [`okur-cursor-screen-text-research.md`](../references/okur-cursor-screen-text-research.md);
  - the plan,
    [`2026-10-23-okur-cursor-screen-text.md`](../plans/2026-10-23-okur-cursor-screen-text.md).
    It corrects the assistive AI proposal's §5.2: JAWS 2025's W, S and C results are in place,
    not in the Results Viewer.

## old-pc

- **Round 282 is recording** (999-1003): OCR of a control, routed and clicked; clicking
  recognized text; Background OCR over a changing picture; the JAWS cursor in a WPF window with
  each capture method.
- **old-pc's Windows OCR** has English and Turkish, with images up to 10,000 pixels a side.
