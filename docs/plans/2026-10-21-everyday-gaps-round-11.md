# Plan: everyday-use gaps, round 11 (2026-10-21)

Follows [`../handoff/2026-10-21-everyday-gaps-round-11.md`](../handoff/2026-10-21-everyday-gaps-round-11.md).
The owner's order stands: the README's "Not yet" list, in its order. Done or mostly done:
Say All by sentence or paragraph, visited links, the temporary PlaceMarker, settings for one
application or site. Next: custom labels, then saved PlaceMarkers and their list, Flexible
Web, then the rest of the list. Each item is found in JAWS's documentation, recorded on
old-pc, then built; JAWS's accidents become accepted divergences. old-pc always has a round
queued behind the running one. Okur's own words never use JAWS's name (AGENTS.md §1.5).

The workflow, every stretch: this plan, then implementation, then proof with `okur parity`,
then a dated handoff and the next plan, with AGENTS.md pointing at them. A stretch is a few
hours or about thirty commits, not more. The README is updated as items are proven.

## 0. What old-pc records meanwhile

| Round | What | Then |
|---|---|---|
| 134 (running) | 695-698: a followed link and the JAWS cursor at Intermediate and Advanced | import |
| 135 (waiter135) | 691, 692 again; the custom-label snapshots | import 688 too |

For each: fetch with `lab-fetch.sh`; check every trial passed, the capture script's hash, and
that no first trial opened on Chrome's restore bubble or JAWS's window; grep for links, the
lab account's folders and the owner's name; compare parity before and after; commit the
corpus alone with the baseline from the committed code. A scenario's keys in JAWS's own
dialogs go through `jaws_function` `TypeKey`.

## 1. Custom labels

From 688 and JAWS's documentation (Surf's Up, Using Custom Labels):
- **Insert+Ctrl+Tab** on a page's element opens Okur's Custom Label dialog: the text "Enter a
  custom label for this item" and an edit of that name, OK and Cancel. OK keeps the label;
  the dialog closes silently.
- **The label** takes the element's name wherever it is read: its line ("lamp ‹ Edit ›"), its
  focus, the element lists ("lamp Edit"). Labels are kept per site (per page for a page
  without one, as a `file:` page), in Okur's settings folder, and an element is known by its
  id, else its role and name.
- **Deleting**: JAWS's Custom Label Delete and Delete All are in Run JAWS Manager (Insert+F2),
  Delete All asking first. Okur: a label emptied in the dialog is deleted; Run Okur Manager
  (Insert+F2) comes with its list later.
- After labelling, JAWS's cursor is at the page's top (688/s15): an accident of its refresh;
  Okur keeps the cursor on the element, as an accepted divergence.

## 2. PlaceMarkers saved, and their list

Explore by hand on old-pc first, when no round runs (`okur keys`, `okur jaws-history`,
research only): the PlaceMarker List's Add dialog, MoveTo, Remove All and its confirmation.
Then a scenario that removes what it adds, and the build: the list in `okur-ui`, a per-URL
store, K and Shift+K over saved markers, 7 to 0.

## 3. Flexible Web

As plan 10 §5: explore the wizard by hand, then record with an undo at the end.

## 4. Smaller items

- **Line without Pauses**: several lines in one utterance with a mark per line (SAPI
  bookmarks, OneCore SSML marks); JAWS's default.
- **The Okur cursor** moving the mouse by line; reading outside text fields.
- **689/s1**: JAWS's first Tab into the page also says the link's focus, visited included.
- **676/s3**: Enter into a field with a value: JAWS leaves the page's title out.

## 5. The rest of the list

The Word List and skim reading; Speech On Demand; the rest of the verbosity levels; the
clipboard keys (627, once okur-lab-keys is installed); notifications (Focus Assist); starting
at sign-in (the development certificate); Office. The last four need the owner.

## 6. Owner

As before:
- the development certificate on old-pc, then `okur-lab-keys.exe` to `C:\Program Files\Okur Lab`;
- Focus Assist off for one notification recording, if the owner agrees;
- the Defender exclusion;
- Office, last.
