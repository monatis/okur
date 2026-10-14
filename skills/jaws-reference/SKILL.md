---
name: jaws-reference
description: Programmatically control JAWS 2025 screen reader via FreedomSci.JawsApi COM, inspect scripts, capture speech history, and execute reproducible navigation scenarios.
---

# JAWS Reference Control & Speech Capture Skill

This skill provides deterministic, programmatic control over the Freedom Scientific JAWS screen reader (JAWS 2025 on Windows 10 reference machine `old-pc`) using the `FreedomSci.JawsApi` COM dispatch interface and the native speech history clipboard capture mechanism (`ClearSpeechHistory` + `CopySpeechHistoryToClipboard`).

It is designed as foundational reference infrastructure for developing, benchmarking, and parity-testing screen readers (such as cross-platform Rust screen readers) against canonical JAWS behavior.

---

## 1. Safety Rules & Operational Constraints

1. **Shared Single-User Desktop State**: JAWS, the active Windows desktop session, the virtual cursor, and the system clipboard are shared global state. **Never run scenarios or capture passes in parallel**. Run all tests serially.
2. **Interactive Session Required**: JAWS virtual buffer navigation and script execution require an active interactive Windows desktop session (Session 1+). It does not function in non-interactive Session 0 service contexts.
3. **Target Window Focus**: JAWS script dispatch (e.g. `MoveToNextHeading`, `FocusToNextField`) executes against the **currently active foreground window**. Always ensure the target fixture window (e.g. Chrome, Edge) is foregrounded and settled before dispatching commands.
4. **Verbatim Canonical Transcripts**: Never alter, normalize, or silently trim raw speech history transcripts. The raw output is canonical evidence for differential testing.
5. **No System Configuration Tampering**: Do not modify installed JAWS script files (`.jss`, `.jsb`, `.jkm`, `.jcf`), license configurations, or system registry settings unless specifically authorized.

---

## 2. Directory Structure & Reference Navigation

- **`scripts/probe.ps1`**: Probes the host environment (JAWS running, COM instantiation, Session 1, speech history enabled, synthetic speech round-trip).
- **`scripts/invoke.ps1`**: Dispatches a named JAWS script or function directly via COM and reports dispatch metadata.
- **`scripts/capture.ps1`**: Executes a navigation action (COM script, function, or keyboard stroke), clears speech history, waits for settle, captures the raw transcript, times execution, and outputs structured JSON.
- **`scripts/run-scenario.ps1`**: Executes multi-step declarative scenarios, enforces explicit reset preconditions, supports multiple repetitions, and saves comprehensive evidence files.
- **`references/fsapi.md`**: COM ProgID, CLSID, IID, TypeLib, in-proc DLL paths, method signatures, COM apartment/bitness, and dispatch return semantics.
- **`references/script-inventory.md`**: Catalog of verified JAWS scripts, quick navigation keys, desktop keystrokes, source file locations, and caveats.
- **`references/speech-capture.md`**: Speech History buffer mechanics, clipboard transfer, settle timing, retry policies, and audio vs transcript distinctions.
- **`references/experimentation.md`**: Methodology for agents: discovering commands, building fixtures, running repeated trials, and investigating parity discrepancies.
- **`references/troubleshooting.md`**: COM bitness, focus slips, stale clipboard, wrap-to-top, and non-interactive session remediations.
- **`examples/`**: Sample HTML fixtures, declarative scenario JSON files, and generated evidence records.
- **`tests/`**: Automated harness tests and live smoke tests for the scripts.

---

## 3. Core Workflows

### Workflow A: Probe the Reference Environment
Verify that JAWS is running, COM responds, and speech history capture works:

`powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\skills\jaws-reference\scripts\probe.ps1
`

### Workflow B: Invoke a JAWS Script
Dispatch a verified JAWS script name to the active application:

`powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\skills\jaws-reference\scripts\invoke.ps1 -Script "MoveToNextHeading" -TargetProcess "chrome"
`

### Workflow C: Capture Action Speech Output
Execute an action (COM script or keystroke) and retrieve its raw speech transcript:

`powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\skills\jaws-reference\scripts\capture.ps1 -Script "MoveToNextHeading" -TargetProcess "chrome" -SettleTimeoutMs 400
`

### Workflow D: Execute a Declarative Scenario
Run a multi-step scenario across multiple repetitions and save full evidence:

`powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\skills\jaws-reference\scripts\run-scenario.ps1 -ScenarioFile .\skills\jaws-reference\examples\fixture-navigation.json -Repetitions 5 -OutputPath .\skills\jaws-reference\examples\example-evidence.json
`

### Workflow E: Investigate a Parity Discrepancy
1. Construct a minimal HTML fixture in `examples/` or temp.
2. Execute the command using FSAPI (`capture.ps1 -Script <ScriptName>`).
3. Execute the equivalent keyboard input (`capture.ps1 -Keystroke <Key>`).
4. Compare the two raw speech transcripts for parity differences.
5. Record findings in `references/script-inventory.md` and reference corpus.

---
### Workflow F: Capture the Chrome UIA Snapshot of a Fixture
The differential parity harness (`sr-cli parity`) drives our engine over the exact UIA tree Chrome
exposed for the fixture JAWS was recorded on. Capture it on the reference machine with the same
page-loading mechanism as `run-scenario.ps1`:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\skills\jaws-reference\scripts\capture-snapshot.ps1 `
    -Fixture C:\Users\ailabs\corpus\fixtures\doc-reading.html `
    -OutputPath C:\Users\ailabs\Dropbox\sr-harness\snapshots\doc-reading.chrome.uia.json `
    -SrCli C:\Users\ailabs\sr-harness\bin\sr-cli.exe
```

The snapshot's `provenance` records the fixture SHA-256 and Chrome version; the parity harness
refuses snapshots whose fixture hash differs from `corpus/fixtures/`. Copy snapshots into
`corpus/snapshots/` unmodified and commit them separately from `crates/` changes.

### Workflow G: Record What JAWS Actually Speaks (Speech Recorder Voice)
JAWS's speech history holds text from *before* its symbol and punctuation processing: a Right
Arrow onto `$` records `$`, and onto a space records an empty entry, while the user hears
"dollar" and "space". To capture what is spoken, JAWS on old-pc speaks through **SR Speech
Recorder** (`crates/sr-sapi-recorder`), a SAPI 5 voice that appends every text fragment it is
asked to speak to `C:\Users\ailabs\sr-harness\recorder\spoken.jsonl` and hands the call to
Microsoft David, so speech stays audible and timed for silence detection.

1. Build and deploy: `cargo build --release -p sr-sapi-recorder` (x64) and with
   `--target i686-pc-windows-msvc` (x86), then `lab-push.ps1 -Recorder <x64 dll> -Recorder32
   <x86 dll>` and `lab-pull.ps1` on old-pc. Stop JAWS first if it has the voice loaded.
2. Register once, elevated (SAPI lists voices only from HKLM; JAWS runs elevated):
   `install-speech-recorder.ps1` (`-Uninstall` removes it).
3. Select it as JAWS's voice (SAPI 5 synthesizer, voice "SR Speech Recorder").

`run-scenario.ps1` then stores, per step, `observation.spoken` (the text of each synthesizer
call) and `observation.spoken_records` (fragments with SAPI action, pitch, rate and volume)
next to `speech_raw`. The log is read before the history copy, whose confirmation JAWS speaks.

## 4. Quick Command Reference Table

| Operation | JAWS COM Script | Quick Nav Key | Keyboard Equivalent | Context |
| :--- | :--- | :--- | :--- | :--- |
| Next Heading | `MoveToNextHeading` | `H` | `H` | Virtual Cursor (Browser) |
| Prior Heading | `moveToPriorHeading` | `Shift+H` | `Shift+H` | Virtual Cursor (Browser) |
| Heading Level 1-6 | `MoveToNextHeadingLevel1`..`6` | `1` .. `6` | `1` .. `6` | Virtual Cursor (Browser) |
| Prior Heading Level 1-6 | `MoveToPriorHeadingLevel1`..`6` | `Shift+1` .. `Shift+6` | `Shift+1` .. `Shift+6` | Virtual Cursor (Browser) |
| Any Next Link | `MoveToAnyNextLink` | *(none)* | `Tab` (in DOM) | Virtual Cursor (Browser) |
| Next Unvisited Link | `MoveToNextUnvisitedLink` | `U` | `U` | Virtual Cursor (Browser) |
| Next Visited Link | `MoveToNextVisitedLink` | `V` | `V` | Virtual Cursor (Browser) |
| Next Form Control | `FocusToNextField` | `F` | `F` | Virtual Cursor (Browser) |
| Prior Form Control | `FocusToPriorField` | `Shift+F` | `Shift+F` | Virtual Cursor (Browser) |
| Next Button | `MoveToNextButton` | `B` | `B` | Virtual Cursor (Browser) |
| Next Edit Box | `MoveToNextEdit` | `E` | `E` | Virtual Cursor (Browser) |
| Next Checkbox | `MoveToNextCheckbox` | `X` | `X` | Virtual Cursor (Browser) |
| Next Combo Box | `MoveToNextCombo` | `C` | `C` | Virtual Cursor (Browser) |
| Next Radio Button | `MoveToNextRadioButton` | `A` | `A` | Virtual Cursor (Browser) |
| Read Current Line | `SayLine` | *(none)* | `Insert+UpArrow` | Global / Virtual Cursor |
| Read Current Character | `SayCharacter` | *(none)* | `Insert+Numpad5` | Global / Virtual Cursor |
| Read Current Word | `SayWord` | *(none)* | `Insert+Numpad5 (2x)` | Global / Virtual Cursor |
| Clear Speech History | `ClearSpeechHistory` | *(none)* | *(none)* | Global |
| Copy Speech History | `CopySpeechHistoryToClipboard` | *(none)* | `Insert+Space, Ctrl+H` | Global |
---

## 5. Structured Evidence Format

Every scenario execution output adheres to the canonical JSON schema:

```json
{
  "scenario_id": "fixture-navigation",
  "scenario_version": "1.0.0",
  "timestamp": "2026-10-12T15:04:20.1545224+03:00",
  "environment": {
    "os_version": "Microsoft Windows NT 10.0.26200.0",
    "jaws_version": "2025.2505.43.400 (File: 26.4.10.0)",
    "target_process": "chrome",
    "interactive_session": true
  },
  "fixture": "examples/fixture.html",
  "repetitions_requested": 5,
  "trials": [ ... ],
  "summary": {
    "total_trials": 5,
    "successful_trials": 5,
    "deterministic_match": true,
    "total_elapsed_ms": 23500
  }
}
```

---

## 6. Agent Instruction Checklist

When using this skill to test a screen reader parity question or add a reference case:

1. **Check Environment**: Run `probe.ps1` to ensure JAWS is active and speech history is enabled.
2. **Find JAWS Command**: Consult `references/script-inventory.md` or inspect `Default.JKM`.
3. **Open Target Fixture**: Load HTML in Chrome/Edge, ensure window is restored and foregrounded.
4. **Execute Capture or Scenario**: Run `capture.ps1` for single-step testing or `run-scenario.ps1` for multi-step sequences.
5. **Verify Multi-Trial Consistency**: Always test across 5 trials (`-Repetitions 5`).
6. **Save Canonical Evidence**: Commit JSON evidence output for differential testing and benchmarking.
