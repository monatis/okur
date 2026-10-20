# Plan: everyday-use gaps, round 6 (2026-10-21)

Follows [`../handoff/2026-10-20-everyday-gaps-round-6.md`](../handoff/2026-10-20-everyday-gaps-round-6.md).
The list stays [`docs/backlog/everyday-use-gaps-2026-10-19.md`](../backlog/everyday-use-gaps-2026-10-19.md):
the gaps are closed before macOS. Each item is found in JAWS's documentation, recorded on
old-pc, then built; JAWS's accidents become accepted divergences. old-pc always has a round
queued behind the running one. Okur's own words never use JAWS's name (AGENTS.md §1.5).

The workflow, every stretch: this plan, then implementation, then proof with `okur parity`,
then a dated handoff and the next plan, with AGENTS.md pointing at them. A stretch is a few
hours or about thirty commits, not more.

## 0. First: what old-pc records meanwhile

| Round | What | Then |
|---|---|---|
| 85 | 513 and 516 again, with the tracer of `87314da` | 516/s8 and 513/s8 (the PC cursor at 125%) should match; if not, the previous plans' DPI probe |
| 86 | 598 (Ctrl+A at Intermediate) and 613 (the Turkish page) | import; build what differs |
| snap87 | snapshots of `selection-sizes.html` and `selection-threshold.html` | import 617 (in the scratchpad's `r82/`) with its snapshot |
| 88 | 620: selections of 989 to 1029 characters | build the large-selection rule (§1) |
| 89 | 621 and 622: what brings the hints on Tab | §2 |
| 90 | 623 (F3 after Okur Find), 624 and 625 (Insert+T in a page), 626 (a field arrowed back onto) | §1 |
| 91 | 509 (Keyboard Help, again with the speech recorder voice), 627 (the clipboard keys in their key map's order), 628 (the layer's help and Command Search) | §3 |
| perf92 | 540, 541, 323 and 524 once each with `-TraceTimings` (the release tracer `b6f072e9`) | P6's measurement (§4) |

For each: fetch with `lab-fetch.sh`, check every trial passed and the capture script's hash
against the committed one (perf92 installs `run-scenario.ps1` `87215244`; the rounds before it
run `1a29afa8`), check each scenario's hash against the committed file, grep the evidence and
traces for links, the lab account's folders and the owner's name, compare parity before and
after, and commit the corpus alone, with the baseline from the committed code. Before queueing
more, check `Get-MpThreatDetection` on old-pc.

## 1. Recorded, to build

- **A large selection (562, 617, 620).** As the previous plan's §1.
- **Say Color** is done (`af4fb10`, `3cf36df`): every step of 565, 618 and 619 matches. Left:
  colors outside pages (JAWS reads the screen there), and the Turkish table (`colors.txt` per
  locale).
- **F3 and Shift+F3 after Okur Find (623).** In 506 JAWS said nothing and stayed on the
  match; its key map binds them to JAWSFindNext and JAWSFindPrior. The replay's dialog mirror
  does not yet carry the text typed into Find what (the patch in the scratchpad,
  `mirror-typed-text.patch`, does: 506/s7 then matches, and s8-s10 depend on 623).
- **Insert+T in a page (624, 625)**: what JAWS adds after the title ("article | level 3 |
  heading is …", 566/s7) and where.
- **A field arrowed back onto (626)**: 567/s12 against 283/s13.
- **598 and 613** after round 86.

## 2. Verbosity and forms mode

- **The hints on Tab, and forms mode on Tab without Auto Forms Mode.** 552, 553 and 555 (Down
  twice, then Tab; Verbosity written) have JAWS's hints, and their Escape left forms mode
  ("virtual PC Cursor"), though old-pc's AutoFormsMode is 0; 204, 212 and 213 (no Verbosity
  written) had Tab keep the virtual cursor and no hints. 621 (555's keys, nothing written) and
  622 (204's keys, Verbosity written) tell whether writing a setting is what changes JAWS. If
  it is, the harness must restore JAWS's settings exactly, or every recording with
  `jaws_settings` describes another configuration than it names. `jaws-settings.ps1 -Apply`
  edits keys in old-pc's own files, so AutoFormsMode stays 0; but applying restarts JAWS, so
  555 ran on a JAWS just started and 204, 212 and 213 on one long running. If 621 (on a JAWS
  restarted by round 86's settings) has the hints too, JAWS's fresh state is Auto Forms Mode
  on whatever the file says, and the old recordings had a state of their own: record 204 and
  212 again with `-FreshJaws`, and give the engine Auto Forms Mode on by default.
- 610/s6, a toolbar at Advanced: as before.

## 3. Help, clipboard and notifications

- **Keyboard Help (509), the layer's help and Command Search (628).** All three need a catalog
  of Okur's command names and descriptions in Okur's own words (JAWS's help texts are Freedom
  Scientific's). Build the catalog once, from the recordings' shape: what a key says once and
  twice, the on and off messages.
- **The clipboard keys (627):** Say Clipboard, View Clipboard Text, Append to Clipboard.
- **Notifications.** In 533 Focus Assist was on: no banner showed, yet JAWS said the toast and
  listed it in its Notification History. Two ways for Okur: the banner's UI Automation window
  (as NVDA reads toasts), which needs a recording with Focus Assist off; or Windows' notification
  listener (`UserNotificationListener`), which reportedly needs package identity for a desktop
  program. Ask the owner to turn Focus Assist off on old-pc for one recording; research the
  listener meanwhile.

## 4. Performance and live checks

- **P6 (dialogs read whole on every focus):** measure from perf92's logs with `okur latency`
  (the `window` stage per Tab in 540, 541, 323, 524), then decide the cache. In the Find dialog
  a Tab among check boxes raises only the focus; onto a push button, two state changes follow
  (the default button moving), so a cache must not be emptied by those.
- **P3** steps 1 and 3 are built (`ba281e3`, `6754422`); live check: Space on a check box on a
  big page, then Down at once.
- The rest as the previous plan's §4: P5 steps 2, 4, 5, P7, P11-P15, P2 steps 2-3, P4 steps 2
  and 4-6, P9 step 3; the live checks never made (P2, P4, P5, P8, the Window List's switch,
  the secure-screens M2 with Insert+F4).

## 5. Harness

- **Keys into JAWS's modal dialogs.** TypeKey closed JAWS's Jump To Line dialog (559/s2), and
  Command Search may do the same: an FSAPI call while JAWS waits in its own modal dialog
  cancels it. Keys sent with UI Access reach those dialogs. `okur-sr` takes only its own few
  options, and the shipped screen reader should not grow harness commands, so this is a small
  harness program of its own (`okur-lab-keys`, a manifest with `uiAccess="true"`), signed with
  the development certificate. Windows grants UI Access only from a secure folder (Program
  Files), so the owner copies it there once, elevated, after the certificate is trusted (§6);
  then `run-scenario.ps1` sends a step's keys through it when the step asks, and 559 and 299
  are recorded again.

## 6. Owner

- **The development certificate on old-pc:** trust the `.cer` (`dev-signing.ps1 -TrustOnLab`,
  elevated), then `okur-setup.exe install --from C:\Users\ailabs\sr-harness\okur-signed
  --no-start --no-start-after-sign-in`; the agent then checks M3 (signature, `--doctor-exit`,
  `okur doctor`, the registry). Later: sign out and in, uninstall, `-UndoLab`.
- **Focus Assist off on old-pc**, for one notification recording (§3), if the owner agrees.
- **The Defender exclusion on old-pc**, as before.
- **Office on old-pc**, last; 518, 519 and 521 wait for it.
