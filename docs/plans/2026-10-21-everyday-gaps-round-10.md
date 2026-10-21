# Plan: everyday-use gaps, round 10 (2026-10-21)

Follows [`../handoff/2026-10-21-everyday-gaps-round-10.md`](../handoff/2026-10-21-everyday-gaps-round-10.md).
The owner's order for this stretch is the README's "Not yet" list, in its order: Say All by
sentence or paragraph, visited links, PlaceMarkers, custom labels, Flexible Web, then the rest
of the list. Each item is found in JAWS's documentation, recorded on old-pc, then built;
JAWS's accidents become accepted divergences. old-pc always has a round queued behind the
running one. Okur's own words never use JAWS's name (AGENTS.md §1.5).

The workflow, every stretch: this plan, then implementation, then proof with `okur parity`,
then a dated handoff and the next plan, with AGENTS.md pointing at them. A stretch is a few
hours or about thirty commits, not more. The README's "Not yet" list is updated as items
are proven.

## 0. What old-pc records meanwhile

| Round | What | Then |
|---|---|---|
| 125 (running) | 684-686 Say All in a text box by unit; 687 a temporary PlaceMarker | §1, §3 |
| probe126 | the Font dialog's window styles | §6 |
| 127 (waiter127) | 662, 676-682 again, with the browser brought to front | §1, §2, §6 |
| 128 | 688 a custom label | §4 |

For each: fetch with `lab-fetch.sh`; check every trial passed and the capture script's hash;
grep for links, the lab account's folders and the owner's name (a failed trial can hold the
lab desktop's file names: never import one); compare parity before and after; commit the
corpus alone with the baseline from the committed code.

## 1. Say All by sentence or paragraph

Built (`9451c37`, `e3b10a0`). Against JAWS:
- **Pieces**: compare Okur's sentences and paragraphs with 677/678's (trials 2-3 of round 124
  show JAWS breaks after "p.m." and "e.g.", keeps lines without a period together, and makes a
  link a piece of its own). The text layer joins pieces, so the cursor's place after Right and
  Left (680/681's Insert+Up) is what tells.
- **A text field** (684-686): sentences across a WinForms text box's line.
- **Line with Pauses** (679): what `LinePauses=1` changes in the speech.
- Then the README's line moves from "Not yet" to what works.

## 2. Visited links

Built for V, U and the Links List (`95c4968`); JAWS's message for a page without one is
Okur's (682). Chrome marks no link visited on a page opened by its address, so:
- record a scenario that follows a link from a fixture and comes back (Enter, Alt+Left), then
  reads the link (its line, Tab, V, the Links List's Visited Links Only). A fixture of its
  own, whose snapshot is taken after the scenario;
- what JAWS says for a visited link ("visited link"?) is built from that recording.

## 3. PlaceMarkers

From JAWS's documentation (see `jaws-dialogs-and-visual-features.md`, PlaceMarker List):
- 687 (round 125): the temporary PlaceMarker, Ctrl+Windows+K, K, Shift+K, 7.
- Then the PlaceMarker List (Ctrl+Shift+K): Add (the text at the cursor proposed), MoveTo,
  Remove All at the end so no trial leaves one; K wrapping; the store per URL.
- Build: the list dialog in `okur-ui`, a per-URL store in Okur's settings folder, K and
  Shift+K over it, Ctrl+Windows+K, 7 to 0.

## 4. Custom labels

From JAWS's documentation (Surf's Up, Using Custom Labels): Insert+Ctrl+Tab labels the
element at the cursor; the label is read in its place, in the element lists too; kept per
domain; Insert+F2's Custom Label Delete and Delete All. 688 (round 128) records the dialog
and the reading. Build: the dialog, a per-domain store, the label applied wherever a name is
read (the buffer's linearization, the lists), the two delete commands.

## 5. Flexible Web

Research first: JAWS's documentation of Flexible Web (Insert+Space, X), its wizard, its rules
per page, site or all sites. Recording changes old-pc's rules, so a scenario must remove what
it adds.

## 6. Say Window leftovers

probe126 gives the Font dialog's window styles: why JAWS skips the sample text, and why only
the Size list is named. Then 665/s3, 672/s3 and 673/s3.

## 7. The rest of the list, after these

- **The Okur cursor's reading keys**: a recording of JAWS's cursor in Notepad and a dialog
  (Up, Down, Insert+Up at the mouse); then reading the screen's text at the mouse.
- **The Word List and skim reading**, **Speech On Demand**, **settings for one application or
  site from the dialog**, **the rest of the verbosity levels**.
- **The clipboard keys** (627, once okur-lab-keys is installed), **notifications** (Focus
  Assist), **starting at sign-in** (the development certificate), **Office**: owner steps.

## 8. Owner

As before:
- the development certificate on old-pc, then `okur-lab-keys.exe` to `C:\Program Files\Okur Lab`;
- Focus Assist off for one notification recording, if the owner agrees;
- the Defender exclusion;
- Office, last.
