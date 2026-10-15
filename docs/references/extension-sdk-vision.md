# Extensions: a future goal

**Status:** not started. Okur gets an extension SDK after it is stable on Windows and macOS.
This page records the goal and the principles. The refactors that prepare Okur for it, and
for macOS, are in
[`docs/plans/2026-10-15-structure-for-macos-and-extensions.md`](../plans/2026-10-15-structure-for-macos-and-extensions.md).

## The goal

A user describes an accessibility problem in plain words: "Okur does not read new messages in
this chat app." An agent then works through it:
1. It reproduces the problem with OpenDesk (what the operating system shows) and Okur's own
   debugging view (what Okur understood and spoke).
2. It compares with JAWS when that helps (old-pc).
3. It writes a fix:
   - a core fix when the defect is Okur's;
   - an extension when the application needs behavior of its own.
4. It proves the fix by replaying the recorded interaction.
5. It proposes the fix for review and sharing.

Application support, which JAWS keeps in application scripts, becomes extensions written in a
common language. Agents can write them, and so can people who have never written a screen
reader script.

## Why not JAWS's scripting language

- **The value is in the capabilities.** JAWS scripts are useful for what they can do:
  - behavior per application;
  - commands bound to keys;
  - changed announcements;
  - reactions to focus and value changes.
  
  The language adds nothing a modern one lacks. NVDA reaches the same capabilities with
  Python add-ons.
- **Running existing scripts would mean rebuilding much of the JAWS runtime.** The language
  alone would not make them run.
- **Clean room** (AGENTS.md §1.4):
  - Freedom Scientific's own scripts, the source of most of JAWS's application support, are
    never read or translated.
  - Supporting an application JAWS supports means observing JAWS in that application on
    old-pc and reproducing the behavior, as Okur does today.
  - Third-party scripts may be ported only where their license allows it.

## What an extension will be able to do

- Become active for an application, by a platform-neutral application identity.
- Add commands and bind them to keys.
- Change what an announcement says: drop, replace or add parts of it.
- React to events: focus, value, selection, live regions, windows opening.
- Keep its own state and settings per application.
- Read Okur's accessibility objects, never raw UI Automation or AX objects, so that one
  extension works on Windows and macOS.

## Principles, from how Okur works

1. **One contract, defined in Rust.** Language runtimes are adapters around it. Okur's own
   application modules (`okur-core/src/apps/`) use the same contract first, so it is tested
   before anyone outside depends on it.
2. **Keys are decided by table.** The keyboard hook must keep or pass a key at once. Okur
   passes a key through when the engine has been busy for 150 ms, because Windows removes slow
   hooks. So an extension declares its keys when it loads, the hook decides from that table, and
   the extension's code runs afterwards, off the hook thread.
3. **Announcements are data.** An announcement is built from parts: name, role, state,
   position, description. It becomes text, with JAWS's wording, only at the end. Extensions
   change parts, never edit strings.
4. **Objects are platform-neutral.**
   - Applications and windows are identified by what they are (a dialog, a property page,
     a console), not by Windows class names.
   - Class names stay inside the platform layer and inside application modules that need
     them.
5. **Budgets.**
   - Every handler has a time limit, measured with `okur latency`.
   - A slow handler is skipped, never waited for.
   - One key chord turns every extension off.
   
   A screen reader that stops talking leaves its user with nothing.
6. **Verification stays clean.**
   - Parity with JAWS is measured with extensions off.
   - An extension is tested by replaying recorded interactions (`okur trace` and the
     replay `okur parity` already uses), with the speech it should produce.
7. **Privacy.** A screen reader, its extensions and its debugging tools see everything on
   screen, including passwords and private messages.
   - The developer interface is off by default and local only.
   - Extensions declare what they need.
8. **Core defects get core fixes.** Extensions are for behavior specific to one application,
   never a place to hide workarounds for Okur's own bugs.

## The pieces, in order, when the time comes

1. **The Rust SDK:** the internal contract, made public once the "no backward compatibility"
   notice in AGENTS.md is lifted.
2. **One script runtime,** chosen by measuring handler latency on focus events. Candidates:
   - JavaScript through QuickJS or Boa;
   - WebAssembly components, which bring a language-neutral interface, sandboxing and CPU
     limits.
   
   Python comes later, through RustPython or a separate CPython host, if its compatibility and
   speed are enough.
3. **Packaging:** a manifest, declared permissions, enable and disable, and recovery from a
   broken extension.
4. **A developer MCP server,** where an agent reads Okur's state, speech, events, the Virtual
   Viewer and traces, and reloads extensions.
   - It goes through local IPC to the running Okur and is off by default.
   - A read-only first version can come earlier, because it helps develop Okur itself.
5. **An `okur-developer` agent skill,** like `skills/jaws-reference/`:
   - SDK documentation and examples;
   - replay testing;
   - the OpenDesk and old-pc workflows.
6. **Agent-assisted porting** of third-party scripts whose license allows it.
