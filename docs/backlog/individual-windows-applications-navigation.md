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
