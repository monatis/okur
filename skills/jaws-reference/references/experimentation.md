# Agent-Driven Experimentation Methodology

This guide outlines how coding agents design, execute, verify, and document behavioral experiments comparing JAWS behavior against alternative screen reader implementations (e.g., cross-platform Rust screen readers).

---

## Experimentation Lifecycle

### Step 1: Hypothesis & Command Identification
1. Identify the accessibility behavior or key navigation command under investigation (e.g., table cell traversal, unvisited link filtering).
2. Locate candidate JAWS scripts in `C:\ProgramData\Freedom Scientific\JAWS\2025\Scripts\enu\Default.JKM` or `Chrome.jkm`.
3. Check `references/script-inventory.md` to see if the command is already cataloged.

### Step 2: Minimal Controlled Fixture Design
1. Author a minimal HTML fixture isolating the target element(s) under `examples/` or in a temporary workspace directory.
2. Ensure the fixture has distinct landmarks, headings, links, or form controls to prevent ambiguous cursor landing.
3. Include boundaries (start/end of document) to test boundary and wrap-around semantics.

### Step 3: Establish Strict Preconditions
1. Verify host environment via `scripts/probe.ps1`.
2. Launch or navigate the target browser (Chrome/Edge) to the fixture.
3. Ensure the browser window is restored and foregrounded (`ShowWindow` / `SetForegroundWindow`).
4. Reset virtual cursor explicitly to top-of-document (`Ctrl+Home`).

### Step 4: Dispatch via FSAPI COM
1. Run `scripts/capture.ps1 -Script <ScriptName> -TargetProcess "chrome"`.
2. Inspect the returned JSON: confirm `DispatchResult: true`, capture success, and examine `RawTranscript`.

### Step 5: Dispatch via Keyboard Input
1. Reset virtual cursor (`Ctrl+Home`).
2. Run `scripts/capture.ps1 -Keystroke <Key> -TargetProcess "chrome"`.
3. Compare the keyboard raw transcript with the FSAPI transcript.

### Step 6: Multi-Trial Repeatability Verification
1. Screen reader navigation can experience transient timing jitter or buffer synchronization delays.
2. Always execute a minimum of **5 independent repetitions** using `scripts/run-scenario.ps1 -Repetitions 5`.
3. Confirm that all trials yield identical transcripts (`deterministic_match: true`). If differences occur, analyze whether the root cause is timing, focus, or document state.

### Step 7: Parity Analysis & Reference Corpus
1. Compare the canonical JAWS raw transcript with your implementation's output.
2. Check announcement structure: role presentation, state announcements (e.g. checked, visited), level announcements.
3. Save structured evidence JSON into the reference corpus.
4. Update `references/script-inventory.md` with new verified commands, keys, and caveats.
