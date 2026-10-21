# Plan: everyday-use gaps, round 14 (2026-10-21)

Follows [`../handoff/2026-10-21-everyday-gaps-round-14.md`](../handoff/2026-10-21-everyday-gaps-round-14.md).
The owner's order stands: the README's "Not yet" list, in its order, and Okur's settings made
as customizable as JAWS's
([`../references/jaws-customization-map.md`](../references/jaws-customization-map.md)). Each item is
found in JAWS's documentation, recorded on old-pc, then built. JAWS's accidents become accepted
divergences. old-pc always has a round queued behind the running one. Okur's own words never
use JAWS's name (AGENTS.md §1.5).

The workflow, every stretch: this plan, then implementation, then proof with
`okur-lab parity`, then a dated handoff and the next plan, with AGENTS.md pointing at them. A
stretch is a few hours or about thirty commits, not more. A feature that draws a window is also
checked live on old-pc (`run-scenario.ps1 -Reader okur`, then `okur-lab live-parity`).

## 0. What old-pc records meanwhile

| Round | What | Then |
|---|---|---|
| 165 (`waiter165`, 1 trial) | 741: a Flexible Web rule saved, applied on reload, deleted | import; check that no rule is left on old-pc |

Each round is fetched with `lab-fetch.sh` and checked:
- every trial passed, and the capture script's and the scenario's hashes match;
- no links, lab folders or owner names in the files.

The corpus is committed alone, with the baseline from the committed code; then the code.

## 1. Flexible Web's rules (finishing the README's item)

- **From 741:**
  - Save Temporary Customizations: a rule's name, its customizations, where it applies;
  - what a page with a rule says and hides when it loads;
  - the rules page (View or change where rules apply), with F2 to rename and Delete to
    remove.
- **Rules in a store of their own**, as PlaceMarkers are: per page, per site or for all sites.
  They are applied when a page loads, and "Start reading at" moves the cursor there.
- A live check of Okur on old-pc after it.

## 2. The README's next items

These are the README's "Not yet" list in order:
- the Okur cursor outside text fields, and the mouse keys;
- the clipboard keys (627, which needs `okur-lab-keys` signed);
- the Word List and skim reading;
- notifications;
- the rest of verbosity.

Each is found in JAWS's documentation first, then recorded.

## 3. Settings

- **The settings dialog's controls** for this round's settings, once JAWS's labels are found
  in its documentation: Text Processing, same page links, graphics, Heading Indication.
- **Capitals in Say All and Say All's line pauses** (718, 719): per-word pitch and line
  grouping in the live synthesizers' Say All path.
- **Live regions on or off, and indentation**, recorded again on fixtures that show them.
- **The remaining output types** of the shared `Default.JCF`: Control Name, Dialog Name and
  Text, Screen Message, Status, Tool Tip, Toasts. Their modes 0, 1 and 2 (2 likely the short
  form) are to be learned.
- **Settings Center and Quick Settings** (the map's §7.6, step 3).

## 4. Harness

- **Say All progress in replay**, from JAWS's own transcript (507, 538, 539, 712/s2).
- **Deploy the harness scripts** to old-pc at the next idle moment (`lab-deploy.sh`), so
  `queue-batch.ps1` installs `okur-ui.exe`.

## 5. Owner

- Import the development certificate on old-pc (`sr-harness\signing\` in the shared Dropbox),
  so that `okur-lab-keys` can be installed (517, 627).
- Code signing for releases, when the owner chooses.
