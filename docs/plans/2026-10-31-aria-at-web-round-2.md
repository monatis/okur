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

## 5. Then

Excel's round-1122b (typing in cells, its dialogs, Word's context menu), when Excel resumes.
