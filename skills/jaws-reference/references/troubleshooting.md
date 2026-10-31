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

---

## 7. A JAWS Yes/No Question Left Open by a Walk
- **Symptom**: after a walk of JAWS's window, keys go nowhere and JAWS's speech history reads
  empty; the foreground window is one of JAWS's questions ("Restore to Factory Settings").
- **Cause**: a Yes/No question box ignores Escape, so a walk that tours it with Tab and leaves
  with Escape leaves it open, and the walk's next Alt opens its system menu. JAWS's dialogs are
  elevated: plain `okur-lab keys` cannot reach them, only `okur-lab-keys.exe` (UI Access, in
  `C:\Program Files\Okur Lab`).
- **Fix**: read the focus with UI Automation (it reads elevated windows); send Tab through
  `okur-lab-keys.exe --allow jfw`; read the focus again, and press Space only when it is on
  **No**. Then Alt+Tab the same way if JAWS's window keeps the foreground. Never walk a
  confirmation whose Yes is irreversible: record its words once, by Enter and Tab only, and
  answer it No by the procedure above (2026-10-24, Restore to Factory Settings).

## 8. A Typed Character Taken as a JAWS Key
- **Symptom**: a `text` step that types a digit, a capital or punctuation into an application
  does something else: "7" in an Excel cell said "right mouse button" and opened the cell's
  context menu at the mouse (round-1122, every trial of 1123).
- **Cause**: a character sent as Unicode input (`KEYEVENTF_UNICODE`) carries the character in
  the scan code of the key event, and JAWS's keyboard hook reads that scan code as a key: 0x37
  ("7") is the numpad's star, 0x52 ("R") the numpad's Insert, 0x4A ("J") and 0x4E ("N") the
  numpad's minus and plus, 0x43 ("C") F9. Lowercase letters (0x61 and up) are past every key.
- **Fix**: `run-scenario.ps1` types each character by its own key on the layout of the window
  in front (`VkKeyScanEx`, Shift included), as a hand types it, and keeps Unicode input for the
  characters that need AltGr or are not on the layout (2026-10-31).
