# Pending scenarios (written, not yet recorded)

Scenarios and fixtures wait here until they are recorded, because a scenario in
`corpus/scenarios/` without evidence fails the parity test, and the corpus manifest and
`okur parity` read only `corpus/scenarios/` and `corpus/fixtures/`.

To record one:
1. Move it into `corpus/scenarios/`, and its fixture (if it is here) into `corpus/fixtures/`.
2. Follow `docs/references/verification-loop.md` §4.3–4.5: deploy to `old-pc`, record, bring
   the evidence home, regenerate the manifest and baselines.
3. Commit the scenario, fixture and evidence together in a `corpus:` commit.

| Scenario | What it records | Fixture |
|---|---|---|
| 289 | N and Shift+N (non-link text), Insert+Enter and Insert+Shift+Enter | `virtual-keys.html` (here) |
| 290 | S and D (same and different element), Ctrl+Insert+Right, Left and Up | `virtual-keys.html` (here) |
| 291 | P (paragraphs), the sentence keys, Ctrl+Numpad5, Say from and to Cursor | `virtual-keys.html` (here) |
| 292 | Dash (separators), Z (divisions), Y (Smart Glance highlights) | `virtual-keys.html` (here) |
| 293 | Backslash (mail links), Slash (clickable elements), Equals (flow to and from) | `virtual-keys.html` (here) |
| 294 | Shift+Period and Shift+Comma, Alt+Win+End and Home (step past and before an element) | `virtual-keys.html` (here) |
| 295 | First and last heading, and at a level (Alt+Insert+Home and End, Ctrl+Alt+Insert+2) | `news-article.html` |
| 296 | First and last form field (Ctrl+Insert+Home and End) | `signup-form.html` |
| 297 | Table rows and columns (Win+Alt+arrows, Alt+Win+Numpad5, Alt+Win+Period) | `news-article.html` |
| 298 | Insert+N, Insert+Shift+Z, Insert+Z, Insert+Escape | `news-article.html` |
| 299 | J (Jump to Line), Shift+J, W and Shift+W (Word from List) | `news-article.html` |
| 285 | Auto Forms Mode on: Up into an 18-line text area, through its lines, out at its top | `reading-settings.html` |
| 500 | Say All stopped by Ctrl, moved by Right and Left, rate keys and H during it | `news-article.html` |
| 501 | Selecting and copying with the virtual cursor, Say and Spell Selected Text | `doc-reading.html` |
| 502 | Home, End, PageDown and PageUp with the virtual cursor | `news-article.html` |
| 503 | Reading keys on a native check box and list box, top and bottom line, Say All in a dialog | `desktop-forms.ps1` |
| 504 | A native edit: selected text, clipboard keys, Say To and From Cursor, sentence, paragraph | `desktop-editor.ps1` |
| 505 | Say All in a native edit stopped by Ctrl and moved by Right and Left | `desktop-reading.ps1` |
| 506 | JAWS Find (Ctrl+F, F3, Shift+F3, Insert+Ctrl+F) | `news-article.html` |
| 507 | A page load stopped by Ctrl, then Numpad Plus in forms mode | `signup-form.html` |

Scenarios from 400 record JAWS's reading settings (plan
`docs/plans/2026-10-18-reading-settings.md`). Their fixtures, `reading-settings.html`
and its desktop twin `desktop-reading.ps1` (recorded with 400), hold the 18 lines of
`docs/references/jaws-reading-settings-research.md` §12, the same text in both. A scenario
that needs a setting other than old-pc's names it in `jaws_settings`
(`skills/jaws-reference/scripts/jaws-settings.ps1`), and `run-batch.ps1` sets it.

Scenarios from 500 record the everyday-use gaps of
`docs/backlog/everyday-use-gaps-2026-10-19.md`. A step's `during` presses keys while JAWS
still speaks (Ctrl or Right during Say All).

Every scenario written here before (208, 210–219, 315–323) has been recorded; git history keeps
the notes written with them.
