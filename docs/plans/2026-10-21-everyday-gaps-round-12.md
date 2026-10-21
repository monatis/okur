# Plan: everyday-use gaps, round 12 (2026-10-21)

Follows [`../handoff/2026-10-21-everyday-gaps-round-12.md`](../handoff/2026-10-21-everyday-gaps-round-12.md).
The owner's order stands: the README's "Not yet" list, in its order. Done: Say All by sentence
or paragraph, visited links, PlaceMarkers (temporary and saved, the list, 7 to 0), custom
labels, settings for one application or site. Next: Speech On Demand (recorded), Flexible
Web, and, at the owner's new request, Okur's settings made as customizable as JAWS's. Each
item is found in JAWS's documentation, recorded on old-pc, then built; JAWS's accidents become
accepted divergences. old-pc always has a round queued behind the running one. Okur's own
words never use JAWS's name (AGENTS.md §1.5).

The workflow, every stretch: this plan, then implementation, then proof with `okur parity`,
then a dated handoff and the next plan, with AGENTS.md pointing at them. A stretch is a few
hours or about thirty commits, not more. The README is updated as items are proven.

## 0. What old-pc records meanwhile

| Round | What | Then |
|---|---|---|
| 143 (waiter143) | the lab cleanup of the reading room page's PlaceMarkers; 704 version 2 | import 704 |
| 144 (waiter144) | 700 (Speech On Demand) and 705 (Flexible Web's wizard), with the in-page check | import both |

For each: fetch with `lab-fetch.sh`; check every trial passed, `page_focus.in_document` and
`jaws_in_page` where the harness records them, and the capture script's hash; grep for links,
the lab account's folders and the owner's name; compare parity before and after; commit the
corpus alone with the baseline from the committed code. Keys in JAWS's own dialogs go
through `jaws_function` `TypeKey`; a scenario that changes what JAWS keeps (PlaceMarkers,
labels, Flexible Web rules) undoes it before its end.

## 1. PlaceMarkers' leftovers

From 704 (round 141, trial 1, and version 2):
- **The duplicate checks**: adding where one is gives "PlaceMarker Add Error" ("The placemarker
  has already been defined at this location.", Ok); a name in use asks to replace it ("A
  PlaceMarker with the same name already exists on this page. Press enter now to replace …").
- **Display PlaceMarkers For** URL, Domain, All filtering the list.
- **MoveTo's speech and the cursor after it**, Change's renaming, Remove's silence, from 704
  version 2.
- A live check on old-pc (`run-scenario.ps1 -Reader okur`, `okur live-parity`) of Okur's own
  list window: the Table control, the default moving to MoveTo, the owned Add dialog.

## 2. Speech On Demand

From 700 (rounds 142, 144) and the key map's [Speech On Demand] section (Default.JKM: the
scripts that still speak in it; a key map may be read, AGENTS.md §1.4):
- **Insert+Space, S** cycles Full Speech and Speech On Demand, as DEFAULT.JCF's `SpeechMode`
  and `LessSpeechMode` choose (0 on demand, 1 muted); Okur's settings get the same two values
  (§3's registry, if it is in by then).
- **A filter in front of the synthesizer**: in Speech On Demand only the listed commands
  speak (Say Line, Say Word, Say Character, Say All, Say Window Title, Say Color, the rate
  keys, …); everything else is still written to the speech history, as JAWS writes it
  ("banner" for a silent Down [obs, 700/s1]).
- Replay reads `SpeechMode` and `LessSpeechMode` into the settings, so 700 compares both
  layers.

## 3. Settings as customizable as JAWS's (the owner's request)

From [`jaws-customization-map.md`](../references/jaws-customization-map.md):
- **Step 0, the registry (§7 of the map)**: one static table declaring every setting once:
  its TOML path, group, catalog label and description, kind and values, scopes and lifetimes,
  its JAWS `.jcf` key and decoder. The settings dialog (with a search box, Settings Center's
  tree and descriptions), Quick Settings, `okur settings list|get|set`, the files' reference and
  replay's JAWS import are generated from it; a test keeps the table and the `Settings` struct
  in step. The existing 16 settings move onto it first, and the drift the map found goes away
  (typing echo's four values in the dialog and on the command line, Auto Forms Mode in the
  dialog, the verbosity output types and message lengths).
- **Then the map's top ten (§8)**, each found in JAWS's documentation, recorded on old-pc
  where it changes speech, then built: typing echo and key labels; forms mode (Manual,
  Semi-Auto, Auto); page load reading and summary; web verbosity levels; the verbosity output
  types and tutor messages; punctuation per voice context; capitalization; numbers and word
  splitting; voice pitch and contexts; Say All's line pauses, blank lines, indentation, live
  regions.
- The scenarios set the JAWS key in `jaws_settings`, one setting per scenario, so a batch
  restarts JAWS once per setting; old-pc records them while the dev box builds the registry.

## 4. Flexible Web

From 705 and JAWS's guide (chapter 16):
- **The wizard** (Insert+Space, X): its pages, Back, Next, Cancel; "You must be in the web page
  area to activate Flexible Web." outside a page.
- **A customization recorded and undone**: hide an element (the fixture's advertisement) or
  start reading at one, the page read with it, then Undo in the wizard, so no rule stays on
  old-pc. Rules for a page, a site or all sites come after.

## 5. The rest of plan 11

Line without Pauses (several lines in one utterance, a mark per line); the Okur cursor moving
the mouse by line and reading outside text fields; 689/s1 (the first Tab's visited link);
676/s3 (Enter into a field with a value); the Word List and skim reading; the rest of the
verbosity levels; Run Okur Manager (Insert+F2) with its list, where Custom Label Delete and
Delete All live in JAWS; the clipboard keys (627, once okur-lab-keys is installed);
notifications (Focus Assist); starting at sign-in (the development certificate); Office. The
last four need the owner.

## 6. Owner

Nothing is waiting on the owner for §1 to §4. For §5: the items marked as needing the owner.
