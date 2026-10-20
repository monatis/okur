# Plan: everyday-use gaps, round 9 (2026-10-21)

Follows [`../handoff/2026-10-21-everyday-gaps-round-9.md`](../handoff/2026-10-21-everyday-gaps-round-9.md).
The list stays [`docs/backlog/everyday-use-gaps-2026-10-19.md`](../backlog/everyday-use-gaps-2026-10-19.md):
the gaps are closed before macOS. Each item is found in JAWS's documentation, recorded on
old-pc, then built; JAWS's accidents become accepted divergences. old-pc always has a round
queued behind the running one. Okur's own words never use JAWS's name (AGENTS.md §1.5).

The workflow, every stretch: this plan, then implementation, then proof with `okur parity`,
then a dated handoff and the next plan, with AGENTS.md pointing at them. A stretch is a few
hours or about thirty commits, not more.

## 0. What old-pc records meanwhile

| Round | What | Then |
|---|---|---|
| 120 (queued) | 670, 671: 111's Say All at Intermediate and Advanced | import |
| 121 | Say Window in the Find, Page Setup and Save As dialogs (540, 544, 524 have it once): each again with a long wait | §1: Say Window's shapes beyond combo boxes |

For each: fetch with `lab-fetch.sh`, check every trial passed and the capture script's hash
against a committed version, check each scenario's hash against the committed file, grep the
evidence and traces for links, the lab account's folders and the owner's name, compare parity
before and after, and commit the corpus alone, with the baseline from the committed code.
Before queueing more, check `Get-MpThreatDetection` on old-pc. A waiter that pulls a build is
checked by its content before it starts, and replaced only before it pulls.

## 1. Say Window

665/s3 is JAWS's Say Window in Notepad's Font dialog. What Okur reads differently:

- **Scroll bars** and their buttons ("Vertical", "Page down", "Line down", "Position"): JAWS
  reads none.
- **A drop-down's "Open" button**: JAWS reads none.
- **The sample text** ("AaBbYyZz" in the Sample group): JAWS reads the group's name only.
- **A static text beside a link of the same name** ("Show more fonts"): read once, as the link.
- **A combo box**, in JAWS's order:
  - "Font:", "Combo box", then the value and the list's shown items on one line, then "38 of
    187";
  - its list box: "List box" (named by its label when it has one, "Size: List box"), then its
    selected item;
  - its edit: "Font: edit combo", the value, and the place again;
  - a drop-down list (Script): "Script:", "Combo box", "Western", "1 of 7".

Build these in `native_focus::say_window` and the window items' reading
(`event_trace::window_items`), from the window's tree as the tracer now keeps it (list items
on screen, the dialog read again when shown). Check 665/s3, and 540, 544 and 524's Say Window
steps.

## 2. Carried over

- **A page field's colors** (662/s5, s7): IA2's `color` and `background-color` at the caret,
  without changing Say Font in page fields.
- **The clipboard keys** (627): record again with `okur-lab-keys` once the owner has installed
  it.
- **The layer's help and Command Search** (628, inconclusive on the Turkish layout).
- **Notifications**: one recording with Focus Assist off, if the owner agrees.
- **The spoken "blank"** for an empty first line in a large selection's last 20 characters.
- **Performance**: P5 steps 4 and 5, P7, P11-P15, P2 steps 2-3, P4 steps 2 and 4-6, P9 step 3,
  and the live checks never made.

## 3. Owner

- **The development certificate on old-pc:** trust the `.cer` (`dev-signing.ps1 -TrustOnLab`,
  elevated), then `okur-setup.exe install --from C:\Users\ailabs\sr-harness\okur-signed
  --no-start --no-start-after-sign-in`; the agent then checks M3. Then copy `okur-lab-keys.exe`
  to `C:\Program Files\Okur Lab`.
- **Focus Assist off on old-pc**, for one notification recording, if the owner agrees.
- **The Defender exclusion on old-pc**, as before.
- **Office on old-pc**, last; 518, 519 and 521 wait for it.
