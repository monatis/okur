# Implementation Plan: Complex Web Applications (Google Docs & Sheets Parity)

- **Date**: 2026-10-13
- **Author**: Autonomous Pair Programming Agent
- **Target**: Freedom Scientific JAWS 2025 Parity in Modern Complex Web Productivity Applications
- **Reference Machine**: `old-pc` (hosts live Google Chrome authenticated session with Google Docs & Sheets and JAWS 2025.2505.43.400)

---

## 1. Background & Scope

Modern cloud productivity suites like Google Docs and Google Sheets employ advanced DOM virtualization, custom canvas rendering, braille-display mode bridges, and custom keyboard hooks rather than conventional static HTML.

In this milestone, we will achieve full behavioral parity with JAWS 2025 across:
1. **Google Docs Accessibility Mode (Braille/Screen Reader Support)**:
   - Detection of Google Docs editor (`.kix-appview`, hidden braille textarea).
   - Paragraph and line navigation commands (`SayCurrentLine`, `NextLine`, `PriorLine`).
   - Character formatting changes announced on-the-fly (bold, font size, headings).
   - Real-time cursor synchronization with the hidden accessible edit surface.
2. **Google Sheets Grid & Formula Navigation**:
   - Detection of Google Sheets grid canvas and active cell formula bar.
   - Arrow-based cell navigation reading coordinates (`A1`, `B2`), formula contents, and cell values.
   - Column/Row header announcements matching JAWS conventions.
   - Range selection announcements (e.g. `A1:C5`).

---

## 2. Planned Workflow

1. **Remote Investigation on `old-pc`**:
   - Connect via OpenDesk to `old-pc` where Chrome is already authenticated with Google Workspace.
   - Enable screen reader mode in Google Docs (`Ctrl+Alt+Z`) and Google Sheets.
   - Drive keyboard interactions and extract exact JAWS speech tokens into reference trials.
2. **AOM Fixtures & Differential Scenarios**:
   - Export sanitized AOM snapshots (`google-docs.aom.json`, `google-sheets.aom.json`).
   - Create Scenarios 66 through 70 and canonical evidence files.
3. **Engine Implementation**:
   - Extend `VirtualCursor` and `ScreenReaderEngine` with Google Docs / Sheets editor mode support.
   - Implement cell coordinate and formatting announcements.
4. **Validation & Quality Gate**:
   - `cargo test`, `cargo clippy`, and `cargo fmt`.
   - Update `CORPUS_MANIFEST.json` and author handoff.
