# JAWS Automation & Capture Troubleshooting

## 1. COM Instantiation Failures
- **Symptom**: `New-Object -ComObject "FreedomSci.JawsApi"` throws `Retrieval of the COM class factory failed with error 80040154 (Class not registered)`.
- **Causes & Fixes**:
  - `FSAPI.dll` is not registered. Run `regsvr32.exe "C:\Program Files\Freedom Scientific\Shared\FSAPI\1.0\FSAPI.dll"` from an elevated shell.
  - Calling from a 32-bit process when only 64-bit registration exists, or vice versa. Both DLLs should be registered.

---

## 2. Session 0 / Non-Interactive Desktop Errors
- **Symptom**: Script dispatch returns false or speech history is consistently empty.
- **Cause**: JAWS and its virtual PC cursor cannot run in Windows Session 0 (background services / SSH without desktop GUI).
- **Fix**: Run in an interactive desktop session (Session 1+). Verify via `(Get-Process -Id $PID).SessionId -ne 0`.

---

## 3. Window Focus & Activation Slips
- **Symptom**: Script returns `true` but no speech or unexpected desktop announcements occur.
- **Cause**: JAWS scripts execute against the active foreground window. If another window steals focus, scripts target the wrong context.
- **Fix**: Use `ShowWindow(hWnd, 9)` (SW_RESTORE) and `SetForegroundWindow(hWnd)`, followed by a 200ms settle delay before dispatching scripts.

---

## 4. Stale Clipboard & Clipboard Lock Contention
- **Symptom**: `Get-Clipboard` returns text from a previous capture or throws clipboard access exceptions.
- **Causes & Fixes**:
  - Another process (e.g., clipboard monitor) temporarily locked the clipboard. `capture.ps1` implements 3 exponential-backoff retries.
  - JAWS did not copy new speech because speech history was empty. `capture.ps1` sets a unique sentinel string (`__SENTINEL_<guid>__`) before dispatch and confirms the clipboard changes away from the sentinel.

---

## 5. Virtual Cursor Out of Sync
- **Symptom**: `MoveToNextHeading` wraps immediately or skips freshly loaded DOM elements.
- **Cause**: The browser DOM changed after JAWS generated its virtual buffer cache.
- **Fix**: Dispatch `RefreshVirtualBuffer` (`Insert+Escape`), or navigate with `Ctrl+Home` after allowing the page to fully load.

---

## 6. Wrap-to-Top and Boundary Behavior
- **Symptom**: Transcript contains `"wrapping to top"` unexpectedly.
- **Behavior**: JAWS loops to the beginning of the document when navigation reaches the final matching element. This is expected JAWS behavior, not a harness bug.
