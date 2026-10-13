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
