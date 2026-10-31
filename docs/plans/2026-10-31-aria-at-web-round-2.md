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
