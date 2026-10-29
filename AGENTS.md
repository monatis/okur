# AGENTS.md - Developer & Autonomous Agent Guide

This repository hosts **Okur** ("reader" in Turkish), an open-source, modern, cross-platform screen reader implemented in Rust. The screen reader's user-facing behavior, announcements, keyboard shortcuts, and document interaction patterns are designed to match **Freedom Scientific JAWS 2025**, validated through differential testing.

This is an **autonomous, agent-driven project with a "verification harness in the loop"**.

> [!IMPORTANT]
> **Development Notice – No Backward Compatibility Tricks**:
> This project is in early and fast development. Our ultimate goal is correctness, parity with JAWS, high performance, and real-world usefulness (it should be able to be actually used on Windows and macOS for serious tasks). Until this notice is removed from `AGENTS.md`, **we do not need any backward compatibility tricks or legacy artifacts** in this project. Everything can be changed, updated, or removed cleanly without preserving legacy scaffolding or shims just for the sake of backward compatibility. Implementations must be clean, high-performance, and strictly match JAWS in functionality and behavior.

---

## 1. Mission & Engineering Principles

1. **Parity Through Autonomous Verification ("Verification Harness in the Loop")**:
   - Talk is cheap; demonstrable behavior verified against evidence is ground truth.
   - Every reading command, virtual cursor movement, mode transition, and speech announcement must be verified automatically against canonical transcripts in [`corpus/evidence/`](file:///c:/Users/Gigabyte/screen-reader/corpus/evidence/).
   - The agent autonomously discovers features and behavior of JAWS, chooses a task, implements it, conducts research (online, JAWS documentation, help manuals and keystroke lists, NVDA repository, OS accessibility API documentation), records reference trials from live JAWS on `old-pc`, and implements code until full functional parity is verified.

2. **User-Facing Parity Target: JAWS**:
   - While internal architecture and algorithms may borrow best practices from other screen readers (e.g., NVDA), **all user-facing behavior must match JAWS**.
   - Keyboard navigation defaults, quick-nav keys (e.g., `H` for headings, `1`–`6` for levels, `K` for links, `C` for checkboxes, `E` for edits), announcement phrasing, and browse/forms mode switching must follow JAWS conventions.

3. **Cross-Platform Target Tiers & Zero-Cost Trait Architecture**:
   - **First-Citizen Targets (Tier 1)**:
     - **Windows**: Reference verification environment. Windows UI Automation (UIA), IAccessible2, Win32 hooks (`WH_KEYBOARD_LL`), and SAPI 5.
     - **macOS**: Equal Tier 1 target. Native `AXUIElement` / NSAccessibility, `CGEventTap`, and native macOS speech synthesizers.
   - **Best-Effort / Nice-to-Have (Tier 2)**:
     - **Linux**: AT-SPI2 / D-Bus.
   - **Portability Without Compromise (Performance & Correctness First)**:
     - Cross-platform capability must **never** mean sacrificing native responsiveness, sub-millisecond event handling, or correctness.
     - Subsystems with OS dependencies (keyboard hooks, TTS synthesizers, accessibility bridges, event loops) must be defined as clean, zero-overhead Rust traits in core crates (e.g., `KeyboardHook`, `SpeechOutput`, `PlatformBridge`).
     - Concrete platform drivers reside behind conditional compilation (`#[cfg(target_os = "windows")]`, `#[cfg(target_os = "macos")]`, `#[cfg(target_os = "linux")]`).
     - Every subsystem must provide a lightweight headless/mock implementation (`MockKeyboardHook`, `MockSpeechOutput`) ensuring complete testability in CI and headless environments.
     - Even when testing on a Windows host, all traits and abstractions must be strictly structured and future-proofed so that native macOS implementations drop into place without refactoring core screen reader logic.

4. **Agent-Driven Clean-Room Software Engineering & Legal Interoperability**:
   - This project strictly follows **[Clean-room design](https://en.wikipedia.org/wiki/Clean-room_design)** and **[Cleanroom software engineering](https://en.wikipedia.org/wiki/Cleanroom_software_engineering)** methodologies.
   - **Zero Proprietary Ingestion**: No source code, scripts, binaries, or decompiled/disassembled bytecodes from Freedom Scientific or JAWS are ever read, copied, bundled, or disassembled. All code is written from scratch in modern, memory-safe Rust.
     - **May be read**:
       - JAWS's public documentation and help, including its keystroke lists;
       - its key map (`.jkm`): key assignments and command names, as Keyboard Manager shows them;
       - user settings files (`.jcf`, `.vpf`), recorded as the reference machine's configuration;
       - what JAWS does: speech, braille, and FSAPI calls of scripts by name.
     - **Never read**: script source (`.jss`, `.jsh`), compiled scripts (`.jsb`), script documentation (`.jsd`), or any other JAWS program file. A JAWS script is known only by its name and by what it does.
   - **Black-Box Functional Observation**: The verification harness observes JAWS strictly as a black box (keystrokes in -> spoken announcements / braille output out) across synthetic fixtures on an external reference machine (`old-pc`).
   - **Interoperability, Fair Use & Decades of Muscle Memory**: Observing software behavior to create an interoperable, compatible implementation is protected fair use. Decades of user muscle memory—keystrokes, shortcuts, announcement messages, speech tokens, and document navigation models—are functional methods of operation and interoperability interfaces that cannot be copyrighted. Our mission is to preserve this familiarity so screen reader users have an accessible, modern, cross-platform alternative.

5. **Okur Speaks in Its Own Name (Trademarks)**:
   - JAWS, Freedom Scientific and Vispero are trademarks of Vispero. **Okur's own user-facing text never uses them as Okur's name**: its message catalog (`crates/okur-core/locales/`), its dialogs and settings, its command-line options and help, its installer, and anything else Okur says or shows about itself.
   - Where JAWS names itself, Okur names itself: "JAWS Cursor" is "Okur Cursor", "Route JAWS to PC" is "Route Okur to PC", "JAWS Find" is "Okur Find", the "JAWS key" is the "Okur key" (`--okur-key`). The words around the name stay JAWS's, for the muscle memory.
   - JAWS is named only to refer to JAWS itself: in comparisons ("as JAWS does"), in documentation and code comments about parity, and in internal identifiers that stand for JAWS's own commands (`ShutDownJAWS`, `JAWSKey+F4` from its key map).
   - Parity compares Okur's name with JAWS's own as equal, so following this rule costs no parity; a test fails on "JAWS" in a catalog message.

---

## 2. Active Session Plan & Session Lifecycle

To ensure seamless context passing across autonomous sessions, all engineering work is orchestrated through the `docs/` tree:

### Active Plan for Current / Next Session
- **Active Plan**: [`docs/plans/2026-10-29-everyday-gaps-round-38.md`](docs/plans/2026-10-29-everyday-gaps-round-38.md): the recorder after JAWS's own list dialogs and Word's lists of comments and footnotes, the settings' long tail (Voices, Run from System Tray), Word's rest (Insert Hyperlink, page keys), Excel (521, once the owner ends old-pc's Excel processes) and Outlook, the review of 2026-10-25's open findings, then punctuation by word's rest, then the README's "Not yet" list in its order (after [`2026-10-29-everyday-gaps-round-37.md`](docs/plans/2026-10-29-everyday-gaps-round-37.md), [`2026-10-28-everyday-gaps-round-36.md`](docs/plans/2026-10-28-everyday-gaps-round-36.md), [`2026-10-25-everyday-gaps-round-35.md`](docs/plans/2026-10-25-everyday-gaps-round-35.md), [`2026-10-25-everyday-gaps-round-34.md`](docs/plans/2026-10-25-everyday-gaps-round-34.md), [`2026-10-25-everyday-gaps-round-33.md`](docs/plans/2026-10-25-everyday-gaps-round-33.md), [`2026-10-24-everyday-gaps-round-32.md`](docs/plans/2026-10-24-everyday-gaps-round-32.md), [`2026-10-24-everyday-gaps-round-31.md`](docs/plans/2026-10-24-everyday-gaps-round-31.md), [`2026-10-24-everyday-gaps-round-30.md`](docs/plans/2026-10-24-everyday-gaps-round-30.md), [`2026-10-24-everyday-gaps-round-29.md`](docs/plans/2026-10-24-everyday-gaps-round-29.md), [`2026-10-24-everyday-gaps-round-28.md`](docs/plans/2026-10-24-everyday-gaps-round-28.md), [`2026-10-24-everyday-gaps-round-27.md`](docs/plans/2026-10-24-everyday-gaps-round-27.md), [`2026-10-23-everyday-gaps-round-26.md`](docs/plans/2026-10-23-everyday-gaps-round-26.md), [`2026-10-23-everyday-gaps-round-25.md`](docs/plans/2026-10-23-everyday-gaps-round-25.md), [`2026-10-23-everyday-gaps-round-24.md`](docs/plans/2026-10-23-everyday-gaps-round-24.md), [`2026-10-23-everyday-gaps-round-23.md`](docs/plans/2026-10-23-everyday-gaps-round-23.md), [`2026-10-22-everyday-gaps-round-22.md`](docs/plans/2026-10-22-everyday-gaps-round-22.md), [`2026-10-22-everyday-gaps-round-21.md`](docs/plans/2026-10-22-everyday-gaps-round-21.md), [`2026-10-22-everyday-gaps-round-20.md`](docs/plans/2026-10-22-everyday-gaps-round-20.md), [`2026-10-22-everyday-gaps-round-19.md`](docs/plans/2026-10-22-everyday-gaps-round-19.md), [`2026-10-22-everyday-gaps-round-18.md`](docs/plans/2026-10-22-everyday-gaps-round-18.md), [`2026-10-21-everyday-gaps-round-17.md`](docs/plans/2026-10-21-everyday-gaps-round-17.md), [`2026-10-21-everyday-gaps-round-16.md`](docs/plans/2026-10-21-everyday-gaps-round-16.md), [`2026-10-21-everyday-gaps-round-15.md`](docs/plans/2026-10-21-everyday-gaps-round-15.md), [`2026-10-21-everyday-gaps-round-14.md`](docs/plans/2026-10-21-everyday-gaps-round-14.md), [`2026-10-21-everyday-gaps-round-13.md`](docs/plans/2026-10-21-everyday-gaps-round-13.md), [`2026-10-21-everyday-gaps-round-12.md`](docs/plans/2026-10-21-everyday-gaps-round-12.md), [`2026-10-21-everyday-gaps-round-11.md`](docs/plans/2026-10-21-everyday-gaps-round-11.md), [`2026-10-21-everyday-gaps-round-10.md`](docs/plans/2026-10-21-everyday-gaps-round-10.md), [`2026-10-21-everyday-gaps-round-9.md`](docs/plans/2026-10-21-everyday-gaps-round-9.md), [`2026-10-21-everyday-gaps-round-8.md`](docs/plans/2026-10-21-everyday-gaps-round-8.md), [`2026-10-21-everyday-gaps-round-7.md`](docs/plans/2026-10-21-everyday-gaps-round-7.md), [`2026-10-21-everyday-gaps-round-6.md`](docs/plans/2026-10-21-everyday-gaps-round-6.md), [`2026-10-21-everyday-gaps-round-5.md`](docs/plans/2026-10-21-everyday-gaps-round-5.md), [`2026-10-21-everyday-gaps-round-4.md`](docs/plans/2026-10-21-everyday-gaps-round-4.md), [`2026-10-21-everyday-gaps-round-3.md`](docs/plans/2026-10-21-everyday-gaps-round-3.md) and [`2026-10-20-everyday-gaps-round-2.md`](docs/plans/2026-10-20-everyday-gaps-round-2.md)): the everyday-use gaps of [`docs/backlog/everyday-use-gaps-2026-10-19.md`](docs/backlog/everyday-use-gaps-2026-10-19.md), closed before macOS or any public announcement (the owner's decision), each found in JAWS's documentation, recorded on old-pc, then built. Still open from before: [`docs/plans/2026-10-18-reading-settings.md`](docs/plans/2026-10-18-reading-settings.md) (round 1 done: the harness and punctuation; open items in [`docs/backlog/open-items.md`](docs/backlog/open-items.md)): JAWS's reading settings and their per-application and per-site overrides, the core before the extension SDK (the owner's decision), each found in JAWS's documentation ([`docs/references/jaws-reading-settings-research.md`](docs/references/jaws-reading-settings-research.md)), recorded on old-pc, then built; JAWS's accidents are written in [`docs/references/accepted-divergences.md`](docs/references/accepted-divergences.md), not copied. The rest of [`docs/plans/2026-10-17-helper-dialogs.md`](docs/plans/2026-10-17-helper-dialogs.md) (JAWS Find, the reading highlight, PlaceMarkers) and the open items of [`docs/plans/2026-10-16-web-apps-round-2.md`](docs/plans/2026-10-16-web-apps-round-2.md), [`docs/plans/2026-10-15-jaws-feature-loop-round-2.md`](docs/plans/2026-10-15-jaws-feature-loop-round-2.md) and [`docs/plans/2026-10-15-structure-for-macos-and-extensions.md`](docs/plans/2026-10-15-structure-for-macos-and-extensions.md) remain.
- **Future goal**: an extension SDK, after Okur is stable on Windows and macOS ([`docs/references/extension-sdk-vision.md`](docs/references/extension-sdk-vision.md), proposal [`docs/references/extension-sdk-proposal.md`](docs/references/extension-sdk-proposal.md); its groundwork M0 is done, see [`docs/references/messages-and-verbosity.md`](docs/references/messages-and-verbosity.md)); then JAWS's assistive utilities (OCR, AI image descriptions with local GGUF models and Sign in with ChatGPT, remote help modeled on OpenDesk): [`docs/references/assistive-ai-and-remote-proposal.md`](docs/references/assistive-ai-and-remote-proposal.md).
- **Latest Handoff**: [`docs/handoff/2026-10-29-word-keys-comments-access-keys.md`](docs/handoff/2026-10-29-word-keys-comments-access-keys.md), after [`docs/handoff/2026-10-29-settings-results-and-word-keys.md`](docs/handoff/2026-10-29-settings-results-and-word-keys.md), [`docs/handoff/2026-10-28-word-forms-dialogs-read-mode.md`](docs/handoff/2026-10-28-word-forms-dialogs-read-mode.md), [`docs/handoff/2026-10-25-word-links-and-ribbon-round-2.md`](docs/handoff/2026-10-25-word-links-and-ribbon-round-2.md), [`docs/handoff/2026-10-25-word-ribbon-commands-and-review.md`](docs/handoff/2026-10-25-word-ribbon-commands-and-review.md), [`docs/handoff/2026-10-25-word-ribbon-and-tracer.md`](docs/handoff/2026-10-25-word-ribbon-and-tracer.md), [`docs/handoff/2026-10-25-settings-long-tail.md`](docs/handoff/2026-10-25-settings-long-tail.md), [`docs/handoff/2026-10-24-word-round-2.md`](docs/handoff/2026-10-24-word-round-2.md), [`docs/handoff/2026-10-24-settings-redesign-round-2.md`](docs/handoff/2026-10-24-settings-redesign-round-2.md), [`docs/handoff/2026-10-24-settings-redesign.md`](docs/handoff/2026-10-24-settings-redesign.md), [`docs/handoff/2026-10-24-word.md`](docs/handoff/2026-10-24-word.md), [`docs/handoff/2026-10-24-scan-cursor-and-file-ocr.md`](docs/handoff/2026-10-24-scan-cursor-and-file-ocr.md), [`docs/handoff/2026-10-24-okur-cursor-screen-text.md`](docs/handoff/2026-10-24-okur-cursor-screen-text.md), [`docs/handoff/2026-10-23-everyday-gaps-round-26.md`](docs/handoff/2026-10-23-everyday-gaps-round-26.md), [`docs/handoff/2026-10-23-everyday-gaps-round-25.md`](docs/handoff/2026-10-23-everyday-gaps-round-25.md), [`docs/handoff/2026-10-23-everyday-gaps-round-24.md`](docs/handoff/2026-10-23-everyday-gaps-round-24.md), [`docs/handoff/2026-10-23-everyday-gaps-round-23.md`](docs/handoff/2026-10-23-everyday-gaps-round-23.md), [`docs/handoff/2026-10-22-everyday-gaps-round-22.md`](docs/handoff/2026-10-22-everyday-gaps-round-22.md), [`docs/handoff/2026-10-22-everyday-gaps-round-21.md`](docs/handoff/2026-10-22-everyday-gaps-round-21.md), [`docs/handoff/2026-10-22-everyday-gaps-round-20.md`](docs/handoff/2026-10-22-everyday-gaps-round-20.md), [`docs/handoff/2026-10-22-everyday-gaps-round-19.md`](docs/handoff/2026-10-22-everyday-gaps-round-19.md), [`docs/handoff/2026-10-22-everyday-gaps-round-18.md`](docs/handoff/2026-10-22-everyday-gaps-round-18.md), [`docs/handoff/2026-10-21-everyday-gaps-round-17.md`](docs/handoff/2026-10-21-everyday-gaps-round-17.md), [`docs/handoff/2026-10-21-everyday-gaps-round-16.md`](docs/handoff/2026-10-21-everyday-gaps-round-16.md), [`docs/handoff/2026-10-21-everyday-gaps-round-15.md`](docs/handoff/2026-10-21-everyday-gaps-round-15.md), [`docs/handoff/2026-10-21-everyday-gaps-round-14.md`](docs/handoff/2026-10-21-everyday-gaps-round-14.md), [`docs/handoff/2026-10-21-everyday-gaps-round-13.md`](docs/handoff/2026-10-21-everyday-gaps-round-13.md), [`docs/handoff/2026-10-21-everyday-gaps-round-12.md`](docs/handoff/2026-10-21-everyday-gaps-round-12.md), [`docs/handoff/2026-10-21-everyday-gaps-round-11.md`](docs/handoff/2026-10-21-everyday-gaps-round-11.md), [`docs/handoff/2026-10-21-everyday-gaps-round-10.md`](docs/handoff/2026-10-21-everyday-gaps-round-10.md), [`docs/handoff/2026-10-21-everyday-gaps-round-9.md`](docs/handoff/2026-10-21-everyday-gaps-round-9.md), [`docs/handoff/2026-10-21-everyday-gaps-round-8.md`](docs/handoff/2026-10-21-everyday-gaps-round-8.md), [`docs/handoff/2026-10-20-everyday-gaps-round-7.md`](docs/handoff/2026-10-20-everyday-gaps-round-7.md), [`docs/handoff/2026-10-20-everyday-gaps-round-6.md`](docs/handoff/2026-10-20-everyday-gaps-round-6.md), [`docs/handoff/2026-10-20-everyday-gaps-round-5.md`](docs/handoff/2026-10-20-everyday-gaps-round-5.md), [`docs/handoff/2026-10-20-everyday-gaps-round-4.md`](docs/handoff/2026-10-20-everyday-gaps-round-4.md), [`docs/handoff/2026-10-20-everyday-gaps-round-3.md`](docs/handoff/2026-10-20-everyday-gaps-round-3.md), [`docs/handoff/2026-10-20-everyday-gaps-round-2.md`](docs/handoff/2026-10-20-everyday-gaps-round-2.md), [`docs/handoff/2026-10-19-everyday-gaps-round-1.md`](docs/handoff/2026-10-19-everyday-gaps-round-1.md), [`docs/handoff/2026-10-19-reading-settings-round-1.md`](docs/handoff/2026-10-19-reading-settings-round-1.md), [`docs/handoff/2026-10-18-helper-dialogs-round-2.md`](docs/handoff/2026-10-18-helper-dialogs-round-2.md), [`docs/handoff/2026-10-18-old-pc-round-3.md`](docs/handoff/2026-10-18-old-pc-round-3.md), [`docs/handoff/2026-10-17-whatsapp-web.md`](docs/handoff/2026-10-17-whatsapp-web.md), [`docs/handoff/2026-10-17-okur-ui-viewer-and-settings.md`](docs/handoff/2026-10-17-okur-ui-viewer-and-settings.md), [`docs/handoff/2026-10-17-helper-dialogs-research.md`](docs/handoff/2026-10-17-helper-dialogs-research.md), [`docs/handoff/2026-10-17-docs-sheets-settings.md`](docs/handoff/2026-10-17-docs-sheets-settings.md), [`docs/handoff/2026-10-16-google-docs.md`](docs/handoff/2026-10-16-google-docs.md), [`docs/handoff/2026-10-16-responsiveness-and-web-apps.md`](docs/handoff/2026-10-16-responsiveness-and-web-apps.md), [`docs/handoff/2026-10-15-structure-round-1.md`](docs/handoff/2026-10-15-structure-round-1.md), [`docs/handoff/2026-10-15-virtual-viewer.md`](docs/handoff/2026-10-15-virtual-viewer.md) and [`docs/handoff/2026-10-15-jaws-feature-loop-round-1.md`](docs/handoff/2026-10-15-jaws-feature-loop-round-1.md); read the audit [`docs/handoff/2026-10-14-full-audit-of-prior-development.md`](docs/handoff/2026-10-14-full-audit-of-prior-development.md) before trusting any earlier handoff
- **Loop lifecycle**: [`docs/references/verification-loop.md`](docs/references/verification-loop.md): the machines, OpenDesk and `old-pc`, recording JAWS evidence, importing it, dev-box live checks, and pitfalls. Read it before touching `old-pc` or the corpus.


### Standard Session Execution Protocol
Every agent session must follow this exact loop:
1. **Inception & Context Ingestion**:
   - Read this `AGENTS.md` file and inspect the **Active Plan** linked above.
   - Review the prerequisite handoff document in [`docs/handoff/`](file:///c:/Users/Gigabyte/screen-reader/docs/handoff/).
2. **Autonomous Research & Discovery**:
   - Investigate JAWS behavior through its documentation, help and key assignments, and by observing it as a black box (§1.4 lists what may and may not be read). Learn platform APIs from the NVDA codebase and OS accessibility specifications.
   - When necessary, connect to `old-pc` via OpenDesk to test commands or capture new reference evidence.
3. **Implementation & In-Loop Verification**:
   - Implement the planned features in the appropriate crates.
   - Execute differential tests against the reference evidence in `corpus/evidence/`.
4. **Code Quality Gate**:
   - Ensure `cargo fmt --check` passes.
   - Run `cargo clippy --all-targets --all-features -- -D warnings` and verify **0 errors and 0 warnings**.
   - Run `cargo test` and verify all tests pass.
   - *(Note: Automated hooks enforce formatting and clippy clean passes on loop completion, and gate `git commit`. Gate logic lives in `.agents/scripts/`; `.agents/hooks.json` (Antigravity) and `.claude/settings.json` (Claude Code) both invoke it).*
5. **Documentation & Handoff**:
   - Write a dated handoff document in [`docs/handoff/`](file:///c:/Users/Gigabyte/screen-reader/docs/handoff/) (referencing previous steps and test results).
   - Author user-facing documentation in [`docs/references/`](file:///c:/Users/Gigabyte/screen-reader/docs/references/) when applicable.
   - Create the next dated plan in [`docs/plans/`](file:///c:/Users/Gigabyte/screen-reader/docs/plans/).
   - Update `AGENTS.md` to point to the newly created plan for the next session.

---

## 3. Autonomous Verification Harness: OpenDesk & `old-pc`

When developing or verifying screen reader behavior against canonical JAWS execution:
- **`old-pc` Remote Peer**:
  - Connect to the paired remote peer `old-pc` using OpenDesk (`opendesk-use` / MCP peer routing).
  - `old-pc` hosts a live Freedom Scientific JAWS environment (`2025.2505.43.400`) with Google Chrome.
- **Collecting Reference Evidence**:
  - Use the `skills/jaws-reference/` automation skill to send JAWS script calls, keyboard input, query scripts, and retrieve speech history buffers.
  - Multi-trial evidence is recorded in `corpus/evidence/` and cataloged in `corpus/CORPUS_MANIFEST.json`.
- **Differential Verification**:
  - Compare the output of the Rust engine with JAWS observations. Parity is defined as semantic equivalence in announcement tokens, state changes, and navigation boundaries.

### Harness Engineering Is the Core Work

Building evidence collection and verification tooling is the real engineering in this project. Improve `skills/jaws-reference/` as you learn, and add utility scripts, binaries, and `okur` commands whenever they make evidence more trustworthy or debugging faster (capture, snapshots, focus/OS event tracing, debug logs). Harness tools today:
- `okur-lab snapshot` / `capture-snapshot.ps1`: raw Chrome UIA tree of a fixture on `old-pc`.
- `run-scenario.ps1` (format 2): scan-coded keys, audio-silence settling (`okur-lab wait-silence`), provenance.
- `lab-push.ps1` / `lab-pull.ps1`: hash-verified deployment of binaries, scripts, fixtures and scenarios to `old-pc` via Dropbox.
- `okur-lab parity`, `okur-lab invariants`, `okur-lab corpus-manifest`: generated, ratcheted verification results. `okur-lab parity --only <id prefix>` replays chosen scenarios; with `RUST_LOG=okur_core=debug`, the engine's debug log for them.
- `okur-lab snapshot --api ia2`: IAccessible2 view of a Chromium document (attributes, states, hypertext, rendered line ranges).
- `okur-lab trace` / `run-scenario.ps1 -TraceDir`: per-trial accessibility event traces, replayed into the engine by `okur-lab parity`.
- `okur-lab evidence <scenario-id> [--trace]`: readable view of recorded JAWS transcripts and trial variants; `--trace` lists the accessibility events traced during each step.
- `okur-lab keys --allow <process>` and `okur-lab jaws-history`: quick live checks (skill Workflow H). Chords are sent only while an allowed application is in front, and JAWS's speech history is read through FSAPI with the clipboard put back. Research, never evidence. `okur-lab front <title>` brings a test window forward first.
- `okur-lab latency <debug log>`: per-step timings of a live run, including key to speech audio per chord; `skills/jaws-reference/scripts/measure-edit-keys.sh` plays the everyday edit keys for it.
- `skills/jaws-reference/scripts/parity-snapshot.sh <dir>`: the three parity outputs, to compare before and after a change with `diff -r`.
- `skills/jaws-reference/scripts/preview-round.sh <fetched round dir>`: replays the scenarios a round has recorded so far, from a scratch corpus, before the round is imported (fetch what is done with `lab-fetch.sh`).

### Verification Integrity Rules (non-negotiable)

These exist because an audit (2026-10-14) found parity claims that no tool had ever computed.
1. **Evidence is written only by capture tooling** run on `old-pc`. Never hand-write, edit, or "repair" an evidence file. Wrong evidence is deleted and re-recorded.
2. **Evidence and engine code never land in the same commit** (enforced by the pre-commit gate).
3. **Generated artifacts are never hand-edited**: `corpus/CORPUS_MANIFEST.json` (`okur-lab corpus-manifest --write`) and, once it exists, the parity baseline. Tests fail when they are stale.
4. **Claims quote tools**: any parity number or "verified" statement in a handoff must be pasted output of a repo command, named next to it. No unbacked adjectives ("100% parity", "fully verified").
5. **Hermetic `cargo test`**: no live UI, speech, audio, or network. Live tests are `#[ignore = "live: ..."]` and run with `cargo test -- --ignored`.
6. **Known gaps are tests**: when the correct JAWS behavior is known but unimplemented, add a test asserting the JAWS behavior marked `#[ignore = "known gap: ..."]`. Never write assertions that cannot fail.
7. **No fixture-shaped special cases**: code must not branch on fixture text, specific window titles, or names from one machine. App-specific behavior belongs in clearly named per-application modules, like JAWS app scripts.

---

## 4. Accessibility APIs, `xa11y`, & NVDA Cross-Reference

- **Strict Production Standard**:
  - This is a production screen reader project: **NO toy, mocked, or handcrafted custom toy parsers (e.g. ad-hoc HTML parsers) are acceptable**.
  - A screen reader is not simply about parsing and announcing strings; it is an active medium for interacting with the operating system and running applications. All UI consumption must occur through genuine OS accessibility APIs.
- **Core Accessibility Engine (`xa11y`)**:
  - Repository: [`https://github.com/xa11y/xa11y`](https://github.com/xa11y/xa11y)
  - Use `xa11y` as the primary cross-platform accessibility abstraction layer.
  - **Delegation Principle**: If an accessibility feature, query, pattern, or action is covered in `xa11y`, **delegate that work directly to `xa11y`**.
  - **OS-Level Self-Sufficiency**: If `xa11y` lacks an accessibility API, pattern, event hook, or complex text-range support required by the screen reader, **implement and extend it directly at the OS level** (e.g., Windows UI Automation `IUIAutomation`, IAccessible2, Win32 events, macOS `AXUIElement`, Linux AT-SPI2).
  - **Vendored (2026-10-14)**: xa11y now lives in `crates/xa11y/` as our own accessibility-API layer. Extend it there (new `ElementData` properties go through the `ElementParts` completeness guard so every platform backend must map them) rather than growing parallel hand-rolled API code elsewhere.
- **Learning from NVDA**:
  - Repository: [`https://github.com/nvaccess/nvda`](https://github.com/nvaccess/nvda)
  - Consult the open-source **NVDA** and **`xa11y`** codebases for accessibility API patterns, complex text-range normalization, line/word boundaries, formatting runs, and virtual buffer linearization.
  - **Crucial Distinction**: Learn architecture and OS API interaction from NVDA and `xa11y`, but keep the **user-facing speech strings, announcement structures, and keyboard UX strictly matched to JAWS**.

---

## 5. Documentation Structure (`docs/`)

- `docs/plans/`: Future implementation plans (dated, milestone-oriented).
- `docs/handoff/`: Session handoff documents created after verifying implemented features against JAWS.
- `docs/references/`: User-facing canonical documentation, architecture guides, and API specs.

---

## 6. Repository Structure

- `crates/okur-core/`: AOM, virtual buffer, virtual cursor, reading commands, mode transitions.
- `crates/okur-platform/`: Platform accessibility bindings (xa11y, UIA, AXUIElement).
- `crates/okur-corpus/`: Test harness, normalizer, differential comparison engine.
- `crates/okur/`: the screen reader's programs: `okur` (`okur run`, `quit`, `doctor`, `voices`), and `okur-sr`, the installed reader.
- `crates/okur-lab/`: the verification harness's programs, never installed for users: `okur-lab` (`parity`, `trace`, `snapshot`, `wait-silence`, `keys`, `jaws-history`, ...) and `okur-lab-keys`. What sends keys, reads the clipboard and traces every window stays out of `okur.exe`, which antivirus classifiers flagged while it held all of this.
- `crates/okur-ui/`: draws Okur's own windows (the Virtual Viewer on screen, later its dialogs), natively per platform, in a process of its own; `crates/okur-ui-model/` is what Okur and it say to each other (see `docs/references/helper-ui-architecture.md`).
- `crates/xa11y/`: our accessibility-API layer, vendored from xa11y 0.15.2 (MIT) and extended in-tree. See `crates/xa11y/VENDORED.md` for import changes, the hardening ratchet, and planned extensions.
- `corpus/`: Reference speech corpus (manifest, fixtures, scenarios, evidence, quality report). `corpus/pending/` holds scenarios written but not yet recorded; the manifest and parity ignore it.
- `skills/jaws-reference/`: Reusable automation skill for JAWS COM and speech extraction.
- `.agents/`: Antigravity agent configuration (`mcp_config.json`, `hooks.json`) and the shared quality-gate scripts (`scripts/`) used by every harness and by CI.
- `.github/workflows/`: CI.
- `.claude/`, `.mcp.json`, `CLAUDE.md`: Claude Code adapters for the same hooks, MCP servers, and the `jaws-reference` skill.

---

## 7. Build, Check, and Quality Workflow

```bash
# Code formatting
cargo fmt --check

# Strict lint check (must have 0 errors, 0 warnings; the gate also denies unchecked indexing)
cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::indexing_slicing -D clippy::string_slice

# Test execution
cargo test

# Run CLI
cargo run --bin okur-lab -- --help
```

CI ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)) runs the same checks on every push to
`main` and every pull request: formatting, `.agents/scripts/check-sources.ps1` (the gate's panic scan),
clippy and the tests on Windows, `.agents/scripts/check-reference-separation.sh` over the pushed
commits, and the platform-neutral crates' tests on macOS and Linux (informational until those ports
are built there). `rust-toolchain.toml` pins the Rust version for this machine, the gate and CI alike;
move to a new one deliberately, fixing the lints its clippy adds. The project is licensed under Apache-2.0 (`LICENSE`, `NOTICE`); the vendored xa11y
crates keep their MIT license.
