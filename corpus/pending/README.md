# Pending scenarios (written, not yet recorded)

These scenarios and fixtures were written while `old-pc` was offline (2026-10-15). They wait
here because a scenario in `corpus/scenarios/` without evidence fails the parity test, and the
corpus manifest and `okur parity` read only `corpus/scenarios/` and `corpus/fixtures/`.

To record one:
1. Move it into `corpus/scenarios/`, and its fixture (if it is here) into `corpus/fixtures/`.
2. Follow `docs/references/verification-loop.md` §4.3–4.5: lab-push, lab-pull, record on
   `old-pc`, bring the evidence home, regenerate the manifest and baselines.
3. Commit the scenario, fixture and evidence together in a `corpus:` commit.

| Scenario | What it records | Fixture |
|---|---|---|
| 208 | A web textarea, character by character, with the fixed tracer | `characters.html` |
| 315 | A RichEdit field: lines, and Say Font with paragraph formatting | `desktop-richedit.ps1` (here) |
| 316 | Win+M (show the desktop) | `desktop-editor.ps1` |
| 317 | Alt held while Tab is pressed three times (task switcher) | `desktop-dialogs.ps1` |
| 318 | Virtualize Window (Insert+Alt+W) in a page | `news-article.html` |
| 319 | The Run dialog: its announcement and Virtualize Window | `desktop-editor.ps1` |
| 320 | Notepad, laptop layout: CapsLock+I, K, O, L, J, M, U, Insert+Up, Ctrl+Right | `desktop-notepad.ps1` |
| 321 | Virtualize Window in a console: opening line, Insert+T ("Virtual Viewer"), first and last lines, Escape | `desktop-console.ps1` (here) |
| 322 | Virtualize Window on a WinForms form from a one-line field, and Virtualize Current Control on its list view | `desktop-forms.ps1` |
| 323 | A file's Properties (a property sheet): its page, Say Window, Virtualize Window, switching pages | `desktop-property-sheet.ps1` (here) |
| 210 | Read down: aria-current links (`page`, `step`, a link to `#`), role descriptions, popup buttons | `web-app-patterns.html` (here) |
| 211 | An autocomplete: Tab, Enter, type A, Down and Up through its options (aria-activedescendant), Enter | `web-app-patterns.html` (here) |
| 212 | Tab and Shift+Tab move on from the virtual cursor, not from the focus | `web-app-patterns.html` (here) |
| 213 | Ctrl+Right and Ctrl+Left in a textarea through "dog," and "std::io" | `web-app-patterns.html` (here) |
| 214 | Tab from a heading into a link in a list in a navigation region | `web-app-patterns.html` (here) |
| 215 | A role="application" region: its name on the way in, H, Down and Escape go to the page | `web-app-patterns.html` (here) |

210–215 were written on 2026-10-16 from dev-box JAWS observations (research); the handoff
`docs/handoff/2026-10-16-responsiveness-and-web-apps.md` quotes what JAWS said there. The dev
box's JAWS settings differ from old-pc's (it says no "banner"), so compare the recordings with
those notes before trusting either. The H counts assume JAWS's first H after Ctrl+Home lands
on the page's h1, as it did on the dev box.

322 and 323 need `run-scenario.ps1` from 2026-10-15 or later: a property sheet is an owned
window, which its launch finds among the desktop's windows rather than as a process's main
window.

The plan `docs/plans/2026-10-15-jaws-feature-loop-round-2.md` (work item 1) lists what to
check in each recording.
