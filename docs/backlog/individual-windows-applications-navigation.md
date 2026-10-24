# Backlog: Individual Windows Applications Navigation Parity

**Target Milestone**: Future Milestone (Deferred from Milestone 13)  
**Parent Initiative**: Native Desktop & OS Navigation Parity  

---

## 1. Overview
While Milestone 13 focuses on the Windows desktop surface and shell components (desktop list view, item initials, arrows, Tab/Shift+Tab shell cycling across Start, Search, Taskbar, System Tray), future milestones will tackle deeper interaction with individual native Windows applications:

1. **File Explorer (`CabinetWClass`)**:
   - Tree view navigation in left sidebar (Quick Access, OneDrive, This PC).
   - Address bar (`Breadcrumb Parent`), Search edit box.
   - File details list view with multi-column sorting (Name, Date Modified, Type, Size).
2. **Standard Win32 Dialogs (`#32770`)**:
   - Common File Open / Save As dialogs.
   - Property sheets with Tab controls (`SysTabControl32`).
3. **Core OS Productivity Utilities**:
   - Task Manager (`TaskManagerWindow`).

## Status (2026-10-19)

What is done (among it Notepad, the Run dialog, dialogs and message boxes, group boxes, radio
buttons and check boxes) was removed on 2026-10-24; it is in git history.

- **File Explorer:** largely done on the dev box (handoff
  [`2026-10-17-docs-sheets-settings.md`](../handoff/2026-10-17-docs-sheets-settings.md)). The
  differences left are in [`open-items.md`](open-items.md). No old-pc recordings exist.
- **Standard dialogs:**
  - Open: Save As and Open dialogs, and the differences in [`open-items.md`](open-items.md).
- **Task Manager:** open, no recordings.
