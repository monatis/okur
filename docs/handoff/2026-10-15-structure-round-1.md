# Handoff: structure for macOS and extensions, round 1 (2026-10-15)

**Branch**: `structure`, from `main` at `5cc13e1`.
**Plan**: [`docs/plans/2026-10-15-structure-for-macos-and-extensions.md`](../plans/2026-10-15-structure-for-macos-and-extensions.md); its "Status" section lists what is left.
**Goal**: [`docs/references/extension-sdk-vision.md`](../references/extension-sdk-vision.md).

## Why

old-pc is offline, so no JAWS evidence can be recorded. The user asked to use the time to put
Okur into the structure a macOS port and a future extension SDK need, before more features
make the refactor bigger. Every step changes no speech, and the tools below prove it.

## What changed

| Commit | Change |
|---|---|
| `cfeedb5` | Docs: the extension vision, this plan, AGENTS.md §1.4 on what may be read; script-source references removed from the skill (see "Clean room") |
| `5a87202` | The simulated desktop (unreachable since the audit) and five unused speech formatters removed |
| `0a94da5` | `Command` values; the key map as a table of bindings; `Command::from_name` the one place command names are strings |
| `6185a8e` | `FocusEvent`: `in_dialog_box`, `container_is_list_view` and `app` for general code; window and element classes in `platform` for application modules |
| `2fc61b5` | `AppModule`: Widgets, the task switcher, the desktop, File Explorer, the taskbar, consoles and Windows search in `apps/windows/`, offered every focus first |
| `8cf7ffd` | `Announcement` (lines of text, role and state parts); native focus speech built with it |
| `88eff63` | Fix: Q and Shift+Q move to the main region (no key was bound) |
| `e696275` | Fix: `okur run`'s key help no longer calls K a link key |

## Verification

**Speech unchanged.** Before the first refactor, `okur parity --all`, `okur parity --all --api
uia` and `okur parity --spoken` were saved at `5cc13e1`. After every refactor commit the
same commands were rerun and compared byte for byte. Last run, at `e696275`:

```
same: parity-all-ia2.txt (1718 lines)
same: parity-all-uia.txt (1817 lines)
same: parity-spoken.txt (85 lines)
```

`okur parity`:

```
JAWS spoken parity: 171/172 steps match exactly (evidence recorded through the speech recorder voice)
JAWS parity (Ia2): 818/844 steps match exactly (0 scenarios could not run)
```

**Keys unchanged.** Before `to_command_string` and `layout_command` were removed, a temporary
test compared them with the new key map. It covered every key under every combination of
Ctrl, Alt, Shift, Windows and the JAWS key, pressed and released:

```
key map equals the old key strings for 7104 key events
layout table equals the old one for 3328 key events
```

**Tests.** `cargo test --workspace`: every result line "ok". okur-core went from 106 to 111
tests:
- three command tests;
- one announcement test;
- one Q test, which fails without the binding.

The pre-commit gate passed every commit.

**Calls by name that changed** (none made by a key, a scenario or a test; listed in
`0a94da5`):
- canonical names such as `SayNextLine` now run;
- `SayFont` and `WhatFont` run Say Font as Insert+F does;
- `EnterFormsMode`, `ActivateLink` and `ExitFormsMode` act in element lists;
- Insert+F3 is unbound.

**Not checked live.** No live run of `okur run` was made on this branch: there was no
dev-box permission for it. Replay drives the same key and focus entry points that live use
does.

## Clean room

`skills/jaws-reference/references/script-inventory.md`, from the initial commit
(2026-10-12), had three problems:
- it cited `Default.JSS` with line numbers;
- it quoted a message constant from the script source;
- its discovery steps said to "Inspect Script Sources".

So an early session read JAWS's own script source. The citations and that step are removed.
AGENTS.md §1.4 now lists what may be read:
- documentation and help;
- the key map (key assignments and script names);
- user settings;
- black-box behavior.

It also lists what never is: script source, compiled scripts and script documentation. Git
history still has the old text. Whether that early reading matters legally is the user's
call.

## Next

The plan's "Status" section:
- the rest of step 4, announcements in the window heading, application modules,
  `SpeechFormatter` and the buffer;
- the spacing decision;
- the step 5 items still open.

The old-pc work items are unchanged in the round-2 plan.
