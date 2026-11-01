# Plan: ARIA-AT's test plans, round 2 (2026-10-31)

Follows [`../handoff/2026-10-31-excel-menus-and-aria-at-start.md`](../handoff/2026-10-31-excel-menus-and-aria-at-start.md)
and continues [`2026-10-31-aria-at-web-round-1.md`](2026-10-31-aria-at-web-round-1.md). The
owner's order: ARIA-AT now (Excel paused, continued later from
[`2026-10-31-everyday-gaps-round-41.md`](2026-10-31-everyday-gaps-round-41.md)), then the first
release through the Microsoft Store ([`2026-10-31-microsoft-store-release.md`](2026-10-31-microsoft-store-release.md)).

Rules as before: each recorded on old-pc, previewed (`preview-round.sh <round> target/debug/okur-lab.exe`),
fixed by which mode owns the key, imported in a `Corpus:` commit of its own. A figure is
copied from the tool's output after the tool ran. The dev box is shared: cargo on 4 cores, one
heavy job at a time. old-pc always has a round queued.

## Status (2026-10-31, evening)

Imported: checkbox-tri-state, disclosure-faq, radiogroup-roving-tabindex, accordion, checkbox,
toggle-button, switch, switch-button, switch-checkbox, command-button, the three links
([`../handoff/2026-10-31-aria-at-first-plans.md`](../handoff/2026-10-31-aria-at-first-plans.md)).
Alert recorded again in round-aria12b and imported, 6 of 6 steps matching once replay and the
live reader took Chromium's alert event (`db2d52aa`). §2's plans are queued as round-aria16 to
aria29, §3's eleven as round-aria30 to aria40 (ids 2859-3153; menubar-editor has no JAWS
commands).

2026-11-01: radiogroup-aria-activedescendant (aria16) and rating-radio-group (aria17) imported.
A probe of seven radio groups (1126) showed a checked ARIA radio button says its place, so the
roving-tabindex page's silence about it became an accepted divergence. Open, with a second probe
queued after aria40 (round-probe-radio2: 1127, 1128): rating's "Rating group" has no empty state
after it where the heading-labelled groups have one, and its checked radio leaves out its place
after setups that leave the focus on a radio button of the group.

Later on 2026-11-01: tabs-manual-activation (aria18, 60/64), tabs-automatic-activation (aria19),
menu-button-actions (aria20), menu-button-actions-active-descendant (aria21) and
menu-button-navigation (aria22) imported, the last four with every step matching; `okur-lab
aria-at-report` measures the assertions (§4).

Later still: modal-dialog (recorded again as aria23b with the tracer reading IA2's modal state),
minimal-data-grid, quantity-spin-button, horizontal-slider and seek-slider imported
([`../handoff/2026-11-01-aria-at-widgets.md`](../handoff/2026-11-01-aria-at-widgets.md)).

Then: vertical-temperature-slider (aria28), disclosure-navigation (aria29), banner (aria30)
and breadcrumb (aria31) imported, every step of the last two matching; a newer JAWS's words
from ARIA-AT's reports followed where it changed on purpose (§4b)
([`../handoff/2026-11-01-aria-at-newer-jaws.md`](../handoff/2026-11-01-aria-at-newer-jaws.md)).

## 1. The plans recording now

As each round finishes: fetch, preview, fix what differs, import. In order: disclosure-faq
(aria3), radiogroup-roving-tabindex (aria4), accordion (aria5), checkbox (aria6), toggle-button
(aria7), switch, switch-button, switch-checkbox (aria8-10), command-button (aria11), alert
(aria12), the three links (aria13-15).

## 2. The rest of the second format

Import and queue, after them: tabs-manual-activation, tabs-automatic-activation, the three menu
buttons, modal-dialog, minimal-data-grid, quantity-spin-button, radiogroup-aria-activedescendant,
rating-radio-group, the four sliders (horizontal, seek, vertical-temperature), disclosure-navigation.
Watch for menus and dialogs that close when their window loses the foreground (the recorder's
history copy; Excel's NetUI menu did, 1122/s2).

## 3. The first format

The 11 plans with JAWS commands (the landmarks, breadcrumb, the two comboboxes, meter,
rating-slider, slider-multithumb): import with `aria-at-import`, which now reads them.

## 4. The measure of assertions

Each scenario carries its test's assertions (`aria_at.assertions`). A report, per plan, of how
many of them Okur's speech conveys, beside ARIA-AT's JAWS figures: a role, name or state
assertion judged by JAWS's words for it, reviewed by hand before it is trusted.

## 4b. Beyond JAWS (the owner, 2026-11-01)

Where JAWS does not convey what a plan's assertion requires, Okur conveys it (AGENTS.md §1.2): the
most accessible screen reader for the web. `okur-lab aria-at-report --unmet <plan>` lists the
assertions neither conveys (334 on 2026-11-01, MUST first). Each is checked by hand against the
judge (a value text holding its number is no failure of "not convey numeric value"), then added
in JAWS's vocabulary with the rest of the utterance JAWS's, and its steps become accepted
divergences (`OkurAlsoSays`) naming the assertion. In order:

1. MUST: an accordion header's heading and level on the PC cursor's Tab; a spin button's name,
   role and invalid state on the PC cursor's Insert+Up; a named region a quick key enters (where
   it does not change other pages' speech).
2. SHOULD: a grid cell's row and column, a grid's size on Tab; a menu's item count and an item's
   role and place; a slider's and a spin button's limits; a menu button's state on opening;
   checkbox-tri-state's group.
3. MAY: the tab list as a tab list, a tab's "not selected".

Status, 2026-11-01: 1 is done. Of 2, a menu item's place, a slider's and a spin button's limits
follow what a newer JAWS (2025.2508.120) says in ARIA-AT's reports
(`okur-lab aria-at-report --newer-jaws`, `docs/references/aria-at-research.md`); the limits show
in parity once the slider and spin button pages' snapshots are captured again with their limits
(round-probe-radio2). menu-button-navigation's "number of items, '4'" contradicts its own page,
which has 6 items ("1 of 6"): not Okur's gap. Next: the menu button's state on opening and
checkbox-tri-state's group, then 3.

Later: a menu button's state on a focus (done). **Waiting for the owner**: the containers a quick
key enters. ARIA-AT's landmark and breadcrumb plans (drafts) require a quick key into a landmark
or list to say it (MUST: "BannerRegion" before B's button, "Breadcrumb NavigationRegion | list
with 4 items" before U's link). JAWS says them on Tab, not on quick keys. Saying them on quick
keys too (`SpeechFormatter::container_entered`, as a focus does) changed 215 steps on
2026-11-01, 180 of them outside ARIA-AT: H, U, L and the rest on every page ("MainRegion | City
Library Expands Evening Hours heading level 1"). It is the everyday sound of the reader, so it
waits for the owner's choice: always, never (JAWS), or a setting. Also not taken: arrows into a
landmark saying its first line with it (JAWS's "banner" line of its own, ARIA-AT MUST in the
banner plan), which would change JAWS's line model.

Then (2026-11-01, later): an ARIA grid's cell says where it is (Insert+Tab, and the arrows with
the virtual cursor off); an invalid field says its error message; a tab its place on the PC
cursor's Insert+Up. Next from the open steps: JAWS's descriptive text, Insert+Alt+R, and its
hint "Use JawsKey+Alt+R to read descriptive text" on a line whose element has a description
(modal-dialog 2692; Word's open item too): find it in JAWS's documentation, then build it, the
hint in Okur's name ("Okur Key"), which parity's name rule must learn to compare.
round-probe-desc (1129, queued after round-probe-radio2) records what Insert+Alt+R does: no key
map of JAWS 2025 binds it, though its hint names it.

combobox-autocomplete-both-updated (round-aria32) and combobox-select-only (round-aria33), imported:
the combo box's "expanded" as the arrow moves into its list and its state on a focus are done.
MUST assertions still unmet: a field's text said "selected" where the page selected it (the
completion an inline autocomplete adds as a letter is typed, "Alabama" after "a", and a focus
onto a field whose text is selected), and the list box's name and role on each arrow within it.
The first needs the field's selection at the focus and after a typed key (IA2 text selection), a
feature of its own: plan it next to Say All's.

## 5. Then

Status, 2026-11-01, afternoon: every ARIA-AT plan with JAWS commands is imported, and five probes
(1127-1131) answered what they left
([`../handoff/2026-11-01-aria-at-complete-and-probes.md`](../handoff/2026-11-01-aria-at-complete-and-probes.md)).
Open: a dialog's start line (1129/s11, its own slice), the owner's two choices (containers on
quick keys; a field's selected text on a focus).

Excel's round-1122b (typing in cells, its dialogs, Word's context menu), when Excel resumes.
