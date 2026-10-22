# Plan: everyday-use gaps, round 21 (2026-10-22)

Follows [`../handoff/2026-10-22-everyday-gaps-round-21.md`](../handoff/2026-10-22-everyday-gaps-round-21.md).
The owner's order stands: the README's "Not yet" list, in its order, and Okur's settings made
as customizable as JAWS's
([`../references/jaws-customization-map.md`](../references/jaws-customization-map.md)). Each item is
found in JAWS's documentation, recorded on old-pc, then built. JAWS's accidents become accepted
divergences. old-pc always has a round queued behind the running one. Okur's own words never
use JAWS's name (AGENTS.md §1.5).

The workflow, every stretch: this plan, then implementation, then proof with
`okur-lab parity`, then a dated handoff and the next plan, with AGENTS.md pointing at them. A
stretch is a few hours or about thirty commits, not more. A feature that draws a window or
reads the screen live is also checked live on old-pc (`run-scenario.ps1 -Reader okur`, then
`okur-lab live-parity`).

## 0. old-pc

- **820's traces**: the tracer recorded no UIA notification in rounds 247 to 249, though JAWS
  spoke the fixture's every time. `okur-lab probe-notifications` (`b1dea4e`) registered five
  ways at once (the desktop root's subtree and the window's, in the control and the raw view,
  and the generic automation event): none heard a WinForms notification, whether raised
  through `RaiseAutomationNotification` or `UiaRaiseNotificationEvent` on the window's host
  provider, while JAWS spoke each (its speech history). Calculator's notifications ("Ekran
  değeri 5") are heard by every registration, so Okur's client works; registrations on the
  focused element (element scope, and with its ancestors) heard WinForms' no better, and the
  window's process fired no WinEvent in UI Automation's range (0x4E00-0x4EFF), only a state
  and a name change. How JAWS hears WinForms' notification is open: test whether UI Access
  matters (JAWS has it; `okur-lab-keys` has it once the development certificate is on
  old-pc), then a Chromium page's. A WPF window's (.NET 4.8's `RaiseNotificationEvent`)
  reached neither the probe nor JAWS. 820 is imported (`5994b30`, round 249); its replay
  differs at s1, s5 and s8 until the tracer hears these notifications.
- Round 250 (821, the edits page by character) is imported and built (`d4beaf5`).
- Live run 251, Okur itself reading 819's keys on old-pc: `okur-lab live-parity` "Okur live:
  24/24 steps match JAWS exactly; 0 accepted divergences not counted" and "Okur live, spoken:
  24/24 steps send the synthesizer what JAWS sent its own".

## 1. The README's next items

- **Toasts Focus Assist hides**: how JAWS hears a toast that shows no banner (533). Research
  first, without changing old-pc's notification settings (they are the owner's).
- The clipboard keys (627, needs `okur-lab-keys` signed), the rest of verbosity.
- Reading with the Okur cursor where the screen has no text to give, after the above.

## 2. What this stretch's recordings left

- Word reading said the last mark before a word (819); character reading over a mark is 821.
- `<ins>` and `<mark>` in skim reading's attribute rules: none of JAWS's check boxes is for
  them; leave as is unless a recording shows otherwise.
- Superscript marks: JAWS's rule (a number of two or more characters right after a digit) is
  from 817 and 818; a decimal or signed exponent other than "-6" is unrecorded.

## 3. Okur's own dialogs, as JAWS's are read

- Done: the renamed row after Disable (`522d67c`, 787/s16), an empty list's "List box" and hint
  (`629d819`, 787/s18), and JAWS's "Rules _Rules" as an accepted phrase.
- JAWS's "edit | Contains text" for the rule's unnamed text field (775/s7, 787/s12, 798/s5,
  801/s5, 802/s5): JAWS's field is a multi-line WPF text box, whose text JAWS does not read on
  arrival; decide whether Okur's own field should be one too, or the difference accepted.

## 4. Unchanged

- 748's opening and "dialog" plain on return; Start reading at on load.
- Settings: the dialog's labels, capitals and line pauses in Say All, live regions and
  indentation, Settings Center.
- Say All progress in replay; 769/s3's byline.

## 5. Owner

- Import the development certificate on old-pc (`sr-harness\signing\`), so that
  `okur-lab-keys` can be installed (517, 627).
- Office on old-pc is not activated (518, 519, 521 wait for it).
- Code signing for releases, when the owner chooses.
