# FreedomSci.JawsApi COM Interface Specification

## 1. Overview & Architecture
Freedom Scientific JAWS (Job Access With Speech) provides an automation interface via the `FreedomSci.JawsApi` ProgID. This interface allows external Windows applications and test harnesses to control speech synthesis, toggle screen reader states, dispatch script invocations, and trigger internal functions.

On Windows 11 with JAWS 2025, this is our primary deterministic reference-control interface.

---

## 2. COM Registration Details

| Property | Value | Notes |
| :--- | :--- | :--- |
| **ProgID** | `FreedomSci.JawsApi` | Registered in HKCR |
| **CLSID** | `{CCE5B1E5-B2ED-45D5-B09F-8ECE54B75ABF4}` | Primary COM Class ID |
| **Interface (IID)** | `{123DEDB4-2CF6-429C-A2AB-CC089E5516CE}` | `IJawsApi` Dispatch Interface |
| **TypeLib GUID** | `{F152C4EF-B92F-4139-A901-E8F1E28CC8E0}` | JAWS API Type Library v1.0 |
| **InprocServer32 (x64)** | `C:\Program Files\Freedom Scientific\Shared\FSAPI\1.0\FSAPI.dll` | 64-bit native bridge |
| **InprocServer32 (x86)** | `C:\Program Files (x86)\Freedom Scientific\Shared\FSAPI\1.0\FSAPI.dll` | 32-bit WOW64 bridge |
| **Threading Model** | `Apartment` | Single-Threaded Apartment (STA) |

Both 64-bit and 32-bit callers (PowerShell, .NET, Rust, C++) can instantiate this object directly. The underlying DLL bridges to the active `jfw.exe` process.

---

## 3. Method Catalog & Signatures

The `IJawsApi` interface exposes six methods:

### 3.1 `bool RunScript(string scriptName)`
- **Parameters**: `scriptName` (string) - Name of a JAWS script defined in `.jss` / `.jsb` files (e.g. `"MoveToNextHeading"`, `"ClearSpeechHistory"`). Passed without parentheses.
- **Return Value**: `bool` (`VARIANT_BOOL`: `-1` / `True`, `0` / `False`).
- **Semantics**: Indicates **only** whether the script dispatch was successfully accepted and queued by JAWS. It does *not* indicate whether the script succeeded internally or whether target elements existed.

### 3.2 `bool RunFunction(string functionName)`
- **Parameters**: `functionName` (string) - Name of a built-in or script function.
- **Return Value**: `bool` (`VARIANT_BOOL`, False or True).
- **Semantics**: Dispatches a function call. Like `RunScript`, the return value is strictly the COM dispatch status, *not* the return value of the underlying JAWS function.

### 3.3 `bool SayString(string text, bool bInterrupt)`
- **Parameters**:
  - `text` (string): Text string to synthesize and speak.
  - `bInterrupt` (bool): If `true`, purges currently buffered speech before speaking; if `false`, queues after current speech.
- **Return Value**: `bool` (`VARIANT_BOOL` true on dispatch).
- **Semantics**: Injects a speech event into JAWS speech queue and Speech History buffer.

### 3.4 `void StopSpeech()`
- **Parameters**: None.
- **Return Value**: `void`.
- **Semantics**: Immediately silences active speech synthesizer output.

### 3.5 `bool Enable(bool bEnable)`
- **Parameters**: `bEnable` (bool) - `true` to enable JAWS, `false` to disable.
- **Return Value**: `bool` (`VARIANT_BOOL`).
- **Semantics**: Toggles speech and screen reader processing.

### 3.6 `bool Disable()`
- **Parameters**: None.
- **Return Value**: `bool` (`VARIANT_BOOL`, false on success).
- **Semantics**: Convenience helper equivalent to `Enable(false)`.

---

## 4. Behavioral Mechanics & Operational Constraints

### 4.1 Dispatch Status vs Function Return Data
- `RunScript` and `RunFunction` return `bool` (`VARIANT_BOOL`, True = -1, False = 0).
- **True (-1)** indicates only that JAWS received and queued the dispatch message.
- **False (0)** indicates the COM bridge failed or JAWS process was unreachable.
- FSAPI does not return the internal return value of the script or function. Observable evidence must be extracted from the Speech History buffer or accessibility state.

### 4.2 Active Application Context Dependency
- **Context-Sensitive**: Scripts execute against the currently active foreground window.
- If focus is lost or shifts to another application, scripts like `MoveToNextHeading` will no-op or fail.
- **Precondition**: Always ensure the target application window is restored, foregrounded, and settled before invocing scripts.

### 4.3 Interactive Session Requirement
- JAWS requires an active interactive desktop session (Session 1+).
- Service sessions (Session 0) cannot host JAWS runtime virtual buffers or speech.

---

## 5. PowerShell Invocation Examples

```powershell
# 1. Instantiate JAWS API
$jaws = New-Object -ComObject "FreedomSci.JawsApi"

# 2. Synthetic speech test
$res = $jaws.SayString("Reference test", $false)

# 3. Invoke Script
$res = $jaws.RunScript("MoveToNextHeading")
```

---

## 6. Verified Facts vs Assumptions

- **Verified Fact**: CLSID `{CCE5B1E5-B2ED-45D5-B09F-8ECE54B75ABF4}` registered in 64-bit and 32-bit registries.
- **Verified Fact**: `RunScript` accepts valid script names and returns `True` (`-1`).
- **Verified Fact**: Vispero AT Driver (`RemoteCommandServer.dll` on port 9002) is not installed by default. FSAPI is our canonical interface.
- **Assumption**: Script names appear case-insensitive in JAWS runtime; use exact case from `ScriptInventory` to guarantee compatibility.
