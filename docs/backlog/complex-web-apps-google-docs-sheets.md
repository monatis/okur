# Backlog: Complex Web Apps (Google Docs, Sheets, Workspace)

**Status**: Deferred in Backlog  
**Target Milestone**: Post-Desktop Navigation Parity  

## Summary
Advanced web productivity suites (Google Docs, Google Sheets) require dedicated screen reader accommodations:
1. **Braille / Screen Reader Mode in Google Docs**:
   - Google Docs uses canvas rendering with hidden DOM active elements and ARIA live region announcements.
   - Requires detecting Google Docs application mode, disabling default virtual PC buffer navigation where canvas intercepts keystrokes, and processing braille display/speech events.
2. **Google Sheets Grid & Formula Bar**:
   - Complex custom grid navigation with aria-rowindex, aria-colindex, cell formatting, formula bar announcements.
3. **Reference Corpus Collection**:
   - `old-pc` has authenticated Google session.
   - Ensure screen reader support is turned on in Docs (`Ctrl+Alt+Z`) and JAWS accessibility tree is fully verified.

## Status (2026-10-19)

- **1. Docs with braille support: done for documents** (`2a66579` and the 2026-10-17 work).
  The application module is `crates/okur-core/src/apps/web/google_docs.rs`. Keys, Insert+F,
  selection, Say All and the menus say what JAWS says on the dev box
  ([`2026-10-16-google-docs.md`](../handoff/2026-10-16-google-docs.md),
  [`2026-10-17-docs-sheets-settings.md`](../handoff/2026-10-17-docs-sheets-settings.md)).
- **2. Sheets: done for the grid and the cell editor** (`e2d2b50`): keys across cells, Ctrl+arrow
  jumps, selections, Tab, Enter, typing, F2, Escape and Delete.
- **3. Reference evidence: open.** Docs and Sheets were compared on the dev box with the owner's
  JAWS. That is research, not evidence; no old-pc recordings exist.
- **Left:** JAWS's "Type in text." after "Leaving menus" in Docs, and a Sheets module for its font.
  See [`open-items.md`](open-items.md).
