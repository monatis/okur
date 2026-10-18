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
   - Group boxes, Radio buttons, Checkboxes, and OK/Cancel buttons.
3. **Core OS Productivity Utilities**:
   - Notepad (`Edit` / RichEdit control navigation, line/word boundaries, cursor tracking).
   - Run Dialog (`Win+R`).
   - Task Manager (`TaskManagerWindow`).

## Status (2026-10-19)

- **File Explorer:** largely done on the dev box (handoff
  [`2026-10-17-docs-sheets-settings.md`](../handoff/2026-10-17-docs-sheets-settings.md)):
  - Tab through the window;
  - the list with its type and size columns;
  - the context menu and the navigation pane (303/s6's rule).

  The differences left are in [`open-items.md`](open-items.md). No old-pc recordings exist.
- **Standard dialogs:**
  - dialogs and message boxes (306) and a dialog's window view (311), property sheets with their
    tabs (323) and WinForms forms (322) are recorded and replayed;
  - group boxes, radio buttons by `WS_GROUP` and check boxes are done.
  - Open: Save As and Open dialogs, which have no recordings, and the differences in
    [`open-items.md`](open-items.md).
- **Notepad:** recorded and replayed (314, 315, 320), including the font, the window view and the
  laptop layout's reading keys.
- **Run dialog:** recorded (319).
- **Task Manager:** open, no recordings.
