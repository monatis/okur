# JAWS Virtual Buffer Behavior: Evidence-Derived Specification

Observed black-box behavior of JAWS 2025.2505.43.400 with Chrome 154 on Windows 10 (`old-pc`),
derived only from recorded evidence in `corpus/evidence/`. Each rule cites
`scenario/step`; view a transcript with `cargo run -p sr-cli -- evidence <scenario-id>`.

In transcripts below, `‹ ›` stand for JAWS's `\u{1d}`/`\u{1e}` role markers and `|` for a
line break within one step's speech history.

**Status markers:** **[obs]** is directly observed and deterministic across 5 trials.
**[hyp]** is a hypothesis that still needs a dedicated experiment.

---

## 1. Line model (Down / Up arrow)

1. **Line 0 is the page title.** Up Arrow at the top repeats it [obs, 102/s72–s74], and Say All starts with it [obs, 111/s1].
2. **Ends repeat rather than announce.**
   - Down Arrow on the last line re-speaks that line [obs, 101/s72–s74, 201/s26–s27].
   - Up Arrow on the title re-speaks the title.
   - There is no "top" or "bottom" message.
3. **Containers produce their own start and end lines.** Exact phrases [obs, 101, 201]:

   | Element | Start line | End line |
   |---|---|---|
   | `banner` | `banner` | `banner end` |
   | `navigation` labelled "Primary" | `Primary navigation region` | `Primary navigation region end` |
   | `main` | `main region` | `main region end` |
   | `article` | `article` | `article end` |
   | `complementary` labelled "Related stories" | `Related stories complementary information` | `Related stories complementary information end` |
   | `contentinfo` | `content information` | `content information end` |
   | list | `list of N items` | `list end` |
   | nested list | `list of N items nesting level 1` | `list end nesting level 1` |
   | table | `table with C columns and R rows` | `table end` |
   | blockquote | `block quote` | `block quote end` |
   | fieldset with legend L | `group start L` | `group end L` |

4. **Each link is its own line**, splitting the surrounding text [obs, 101/s15–s17: `By` / `‹ same page link › Maria Lopez,` / `published March 14, 2026`].
   - Punctuation immediately after a link stays on the link's line ("Maria Lopez,").
   - Inline `strong`, `em` and `time` do not split lines [obs, 101/s18: `…until 9 p.m. on weekdays…`].
5. **Long paragraph text wraps after the word that crosses 150 characters** [obs, 101/s18–s19, s22–s23, s52–s54].
   - The limit is JAWS's `[HTML] MaxLineLength=150` in `Default.JCF`, described there as the maximum number of characters on a line of a paragraph. There is no Chrome or user override on `old-pc`.
   - Rule: while more than 150 characters remain, the line ends at the first space at or after character 150, and the next line starts at the following word. This reproduces all four recorded breaks: 154, 152, 155 and 152 characters.
   - It is not Chrome's layout. On `old-pc`, Chrome's IA2 and UIA line boundaries put each of these paragraphs on one rendered line (a 1920 px window at 125% scaling).
   - A wrapped paragraph is still one paragraph for Ctrl+Up/Down.
   - [hyp] Headings, list items and table cells longer than 150 characters wrap the same way. This is unrecorded.
6. **Headings while reading lines:** `‹ heading level N › Text` [obs, 101/s14].
7. **Graphics:** the alt text and `‹ Graphic ›` form one step [obs, 101/s20, 113/s1].
8. **List items:** the item's marker is spoken as text. Examples: `•` (disc), `◦` (circle, nested) and `1.` (ordered) [obs, 101/s25–s29, 108/s9]. A link inside an item: `•  ‹ same page link › News`.
9. **Table cells while reading lines:** each cell is a line, in row order. The caption is a line right after the table start [obs, 101/s36–s47].
10. **Form controls while reading lines** [obs, 201]:
    - A label element is its own text line, and the control follows on the next line: `Full name` / `‹Required› ‹ Edit ›`.
    - Edit with a value: `‹ Edit › sam@example.com`. Password: `‹Required› ‹ password edit ›`.
    - Radio: `·Adult ‹ radio button › ‹checked› | 1 of 3`. The `·` is a no-break space from the source.
    - Check box: `·Send me the monthly newsletter ‹ check box › ‹checked›` (state is `‹not checked›` otherwise). The label is spoken with the control, on one line.
    - A check box label containing a link is also read as separate lines: `I agree to the` / `‹ same page link › terms of use` [obs, 201/s21–s23].
    - Combo box: `‹ Combo box › ‹collapsed› Northside`.
    - Button: `Create account ‹ Button ›`; when disabled: `Save draft ‹ Button › ‹Unavailable›`.
    - `aria-describedby` text is a separate line after the control [obs, 201/s8].

## 2. Quick navigation

11. **H / Shift+H:** `Text | ‹ heading level N ›` (text first, unlike line reading) [obs, 103].
    - Wraps: `wrapping to top | …` and `wrapping to bottom | …` [obs, 103/s6; probe 2026-10-14].
12. **Heading level keys 1–6 do not wrap:**
    - `No more headings at level2 in this section.`, the same with `level3` [obs, 104/s4–s6].
    - Level 1: `No more headings at level1.` [obs, 104/s7].
    - Pressing `2` from the top also speaks the level 1 heading before the first level 2 heading: `City Library Expands Evening Hours ‹ heading level 1 › | What changes | ‹ heading level 2 ›` [obs, 104/s1; also legacy 12/s1].
13. **U / Shift+U (links):** the link line as read by Down Arrow, including a list bullet [obs, 105]. Wraps silently at the end in this capture (s12 re-speaks the last link). *Recheck.*
14. **L / Shift+L:** the list start line; **I:** `• New` and `1. Board vote in March` [obs, 108].
15. **R:** region start lines, wrapping with `wrapping to top | banner` [obs, 110/s1–s7].
16. **G:** `alt ‹ Graphic ›`, wrapping with `wrapping to top` [obs, 113].
17. **F / E / X / A / C / B:** speak the control as in Tab focus, without hints, and wrap with `wrapping to top` [obs, 202, 203].
    - Disabled buttons are skipped by F and B [obs, 202/s11, 203/s15].
    - The first radio of a group is prefixed with `Membership type ‹ group › |` [obs, 202/s4].
18. **Semicolon (landmarks) is layout dependent.** On the Turkish Q layout, VK_OEM_1 is `ş`, and JAWS answered `There are no onMouseOver elements on this page.` [obs, 110/s8–s14]. The landmark key for Turkish layouts is an open question for the project owner.

## 3. Tables (T, Ctrl+Alt+Arrows)

19. **T:** `table with 3 columns and 3 rows | Branch hours after the change | Column 1, Row 1 | Branch` [obs, 109/s1]. Shift+T from inside the table gives the same.
20. **Ctrl+Alt+Right:** `Weekdays | Column 2 of 3`. Past the edge: `end of row` [obs, 109/s2–s4].
21. **Ctrl+Alt+Down onto a new row:** the row header first, then the cell, then the position: `Central | ‹ same page link › Directions | Row 2 of 3`. Past the edge: `bottom of column` [obs, 109/s5–s7].
22. **Ctrl+Alt+Left / Up announce the header of the new column or row:** `Weekdays | 10 a.m. to 8 p.m. | Column 2 of 3` and `Central | 9 a.m. to 9 p.m. | Row 2 of 3` [obs, 109/s8–s9].

## 4. Words and characters

23. **Ctrl+Right / Ctrl+Left word segmentation** [obs, 106]:
    - Kept whole: `$5.99`, `555-0142`, `3.5%`, `month,`, `adults;`.
    - Split apart: `e` `.` `g` `.`; `free` `.`; `help@riverside` `-` `library` `.` `org`; `—`.
    - [hyp] A period is a separate word unless it sits between digits. Commas and semicolons attach to the preceding word. A hyphen between letters is its own word, but between digits it is not.
24. **Right / Left:** one character per step. Spaces produced audio but an empty history entry in the first capture [106/107 v1]; this is being re-verified with the race-free capture script.

## 5. Paragraphs, Say All, Tab

25. **Ctrl+Down / Ctrl+Up group lines into paragraphs** [obs, 112]:
    - A container start line merges with the next link line: `‹ same page link › Riverside Gazette Primary navigation region`.
    - A graphic merges with the following heading.
    - A list item merges with the nested list start.
26. **Insert+Down (Say All)** reads the line sequence from the current line to the end [obs, 111].
27. **Tab** follows browser focus and speaks the focused control with hints (`To activate press Enter.`). On the first Tab into a form, JAWS varies between two phrasings, one of which includes `‹ MainRegion › | ‹ FormRegion ›` [obs, 204/s1]. Tab past the page moves into Chrome's toolbar.

## Capture caveats

- Scenario 103 steps s7–s8 (Shift+H) recorded empty speech because of a capture race, since fixed in `run-scenario.ps1`: a silent baseline before each step, and audio heard with no captured text counts as a failed step. 103 and 107 are being re-recorded.
