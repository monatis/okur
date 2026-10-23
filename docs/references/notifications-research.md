# Notifications: research (2026-10-21)

The README's next "Not yet" item after the Word List and skim reading: Windows' notifications
(toasts) said as they arrive, and JAWS's notification history. This page gathers what is known
before anything is built: what JAWS does, how Windows exposes a toast, how NVDA reads one, and
what the lab has recorded. Okur's behavior follows JAWS; the API facts come from Microsoft's
documentation and NVDA's source, never from JAWS's scripts.

## What JAWS does

From JAWS's public "What's New" pages (2021, 2022, 2023) and FSCast 216 (June 2022):

- **Toasts are spoken as they arrive.** Settings Center's "Enable Accessible Notification
  Events" is on by default (2021).
- **Insert+Space, N** (key map: `ShowNotificationHistory`) opens the Notification History
  (2022): the notifications of the past 24 hours while JAWS ran, at most 500 (FSCast). It is
  also in the Utilities menu and in Run JAWS Manager.
  - Each row has the text, the application and the time: "New notification from Windows
    PowerShell, SR Fixture, Build finished in 12 seconds. 1 of 1 Windows PowerShell 10:11 PM"
    [obs, 533/s5].
  - Enter on a row shows its details: the text, the application, the time received (2023).
  - Buttons: Enable Rules (a check box), Create Rule, Manage Rules, Clear history (2023) and
    Close. The Applications key menu offers Create Rule and leaving the application out of the
    history.
  - Rules match a notification's text (Contains, Begins with, Ends with, Matches pattern), may
    be limited to one application, and say the whole text, a shorter one, nothing, or a sound,
    with a braille action. The first rule that matches applies; rules are named, renamed,
    ordered (Ctrl+Shift+Up and Down) and turned off (2023 updates).
- **Insert+Space, Shift+N** (`RepeatLastNotification`) says the latest notification again (2022;
  in 2021 the layer's N did). With none caught JAWS says "No notification" [obs, 533/s3].
- **Insert+Space, Ctrl+N** (`ToggleToastNotificationAnnouncement`) stops and starts speaking
  toasts until JAWS restarts; they still go into the history; applications' own UIA
  notifications are not affected (2023).
- JAWS keeps the history and the rules in its user settings folder (`Notifications.json`,
  `Notification.rules.json`, per a support article); Okur keeps its own in its settings folder.

Sources: https://support.freedomscientific.com/downloads/jaws/PreviousFeatures?version=2021,
`?version=2022`, `?version=2023`; FSCast 216 transcript,
https://blog.freedomscientific.com/wp-content/uploads/2022/06/FSCast216Transcript-June-2022.html.
JAWS's 2025 help on the Notification History (JFW.chm, customizing_jaws_for_windows /
Notification_History) is still to be read on old-pc.

## What the lab recorded

Scenario 533 (`corpus/fixtures/desktop-notifications.ps1`, round 2026-10-19) raises a toast
sent as Windows PowerShell, with the title "SR Fixture" and the text "Build finished in 12
seconds":

- old-pc has Focus Assist on: no banner showed in two trials of three. In the third JAWS said the
  toast, "New notification from Windows Power Shell, SR Fixture, Build finished in 12 seconds. 1
  of 1", and Repeat Last Notification said it again; in the other two it said "No notification".
- The Notification History listed a fixture toast in every trial, but not each trial's own
  (re-read 2026-10-23): trials 1 and 2 (22:34 and 22:35) show 26 items, the newest at 10:11 PM,
  from an earlier run; trial 3 (22:35:54), whose toast was said, shows 27, the newest at 10:36 PM.
  **A toast Focus Assist hides is neither said nor kept by JAWS**: it reads the banner, as Okur
  does. (An earlier reading of this evidence said every trial listed its own toast; it was
  wrong.)
- "New notification from" and "1 of 1" are not in the toast's XML: JAWS reads text Windows
  makes, not only the notification's own.
- `okur trace` saw no event from the toast at all: it takes UIA notifications only from the
  application in front, and no WinEvent came from the shell's toast window.

## How Windows exposes a toast

- **Windows 10 (from build 14366):** the banner is a UIA Window, class
  `Windows.UI.Core.CoreWindow`, of ShellExperienceHost.exe, its AutomationId holding "ToastView".
  Only UIA's WindowOpened event fires as it appears; from build 15063 it may fire twice for one
  toast. Narrator reads toasts the same way. When ShellExperienceHost is suspended, toasts can
  go unsaid by every screen reader. (NVDA `source/NVDAObjects/UIA/__init__.py`, `Toast_win10`;
  nvaccess/nvda issues 5136, 6096, 7128, PR 7376, issue 5688.)
- **Windows 11 (24H2):** still a CoreWindow of ShellExperienceHost.exe, always there, resized when
  a banner shows and cloaked at rest; not listed by EnumWindows (found with `FindWindowExW` over
  the desktop's children); a `DismissButton` inside. Notification Center and Quick Settings moved
  to ShellHost.exe. (windhawk-mods PR 6135; NVDA `source/appModules/shellexperiencehost.py`.)
- The banner's exact UIA Name is not confirmed from any source; its window title reads "New
  notification". Whether the shell also raises a UIA notification event for toasts is not
  confirmed either.
- **UserNotificationListener** (`Windows.UI.Notifications.Management`, Windows 10 1607 on): lists
  the notifications Windows keeps (application name, creation time, the toast's text elements,
  the first its title), after the user allows it once (`RequestAccessAsync`, from a UI thread;
  Settings can take the permission back, and then the list is silently empty).
  - An unpackaged program can list (`GetNotificationsAsync`) but subscribing to
    `NotificationChanged` fails with 0x80070490 on Windows 11 24H2 (WindowsAppSDK issue 6172);
    such programs poll, once a second (miklol/Muna PR 36), or get a sparse package identity.
  - Notifications hidden by Focus Assist or Do Not Disturb still reach Notification Center, so
    the listener should see them.
  - Tried (2026-10-23, `okur-lab list-notifications`, which prints only the toasts holding a
    given text and counts the rest): an unpackaged program may list without asking, `access:
    allowed` on the dev box (Windows 11, 13 toasts) and on old-pc (Windows 10, Focus Assist on,
    8 toasts). JAWS does not read hidden toasts (533, above), so Okur does not use the list.
  - The `windows` crate has its bindings.

## How NVDA reads toasts

At nvaccess/nvda master (2026-10-21):

- `Toast_win10.event_UIA_window_windowOpen` speaks the toast as an alert
  (`NVDAObjects/behaviors.py` `Notification.event_alert`: `speakObject` with reason FOCUS, and a
  braille message), when "Report notifications" is on (`presentation.reportHelpBalloons`, on by
  default).
- A toast with the same runtime ID within a second of the last is dropped (Windows 10 1703 on).
- NVDA has no notification history, does not use UserNotificationListener, and does nothing
  about Focus Assist.
- Applications' own UIA notification events from the background are not NVDA's toast path.

## Built so far (2026-10-22)

- **Toasts said as they show** (`1b6a460`): the tracer records the shell's WindowOpened, and a
  window holding a `NormalToastView` becomes `AccessibilityEvent::Toast`, said once by its
  name, with its sender (770).
- **Insert+Space, Shift+N and Ctrl+N** (`4662125`, `cb92608`): the last notification again,
  "No notification" before any; toasts off and on, kept while off (773).
- **The Notification History** (`ed3c449`): Insert+Space, N, as 774 shows JAWS's, with the
  details on Enter and Clear History; the last 500 kept while Okur runs.
- **Rules** (`5aee2fb`): Create Rule... with JAWS's controls (775); the first rule a toast
  matches mutes it or keeps it out of the history while Enable Rules is checked.
- **Not yet:** the rule's name prompt, Manage Rules, rules saved to the settings folder,
  Shorten and Play Sound, braille; applications' own UIA notifications in the history;
  the past 24 hours across restarts. Toasts Focus Assist hides are not a gap: JAWS neither says
  nor keeps them (533, re-read 2026-10-23).

## What it means for Okur

1. **Speaking toasts:** subscribe to UIA WindowOpened desktop-wide and take the
   ShellExperienceHost CoreWindows whose AutomationId says a toast view, with NVDA's one-second
   rule against doubles; say them as JAWS does ("New notification from …"). Through xa11y's
   Windows backend, as an event kind every platform maps.
2. **The history:** Okur's own list of what it said, kept by Okur (not Windows'), the past 24
   hours, at most 500, as JAWS's: Insert+Space, N in an okur-ui dialog; Shift+N the last; Ctrl+N
   on and off. Rules later, with the settings.
3. **Toasts Focus Assist hides:** not read, as JAWS does not read them (533, re-read
   2026-10-23).
4. **macOS later:** an `AXObserver` on the Notification Center process
   (`com.apple.notificationcenterui`), its windows' created elements with the subroles
   `AXNotificationCenterBanner` and `AXNotificationCenterAlert` (fthux/NotiShift); VoiceOver says
   a banner's application and summary without moving the cursor.

## Next in the lab

- `okur trace`: record UIA WindowOpened desktop-wide (and UIA notification events from any
  process, marked as such), so a toast's banner shows in the trace with its name and
  AutomationId.
- Record 533 again with the trace, and a toast with the banner shown. Focus Assist is the
  owner's setting on old-pc: ask before turning it off, or find a toast that shows through it
  (an alarm or a reminder).
- Read JAWS's 2025 help on the Notification History on old-pc.

## Applications' own UI Automation notifications (2026-10-23)

Scenario 820: a WinForms window (`desktop-uia-notifications.ps1`, .NET Framework 4.8 in
`powershell.exe`) raises a notification through `AccessibleObject.RaiseAutomationNotification`
as its button is pressed. JAWS says it ("Upload finished: report.pdf") and lists it in its
Notification History under "powershell" (820/s1, s3). Okur's tracer heard nothing (rounds
247-249), and neither did `okur-lab probe-notifications`, which registers for UI Automation
notifications seven ways (the root's and the window's subtree, control and raw views; the
focused element, with and without its ancestors; and as a generic automation event): every
registration answered Ok, and no notification came. The same probe hears Calculator's.

Two hypotheses were ruled out on old-pc, with the fixture's copy raising its notification by
itself 8 s after its window shows (`sr-harness\experiments\exp2`, experiments 8 and 9):
- **UI Access:** the probe signed with the development certificate, `uiAccess="true"`, installed
  by the owner in `C:\Program Files\Okur Lab` and started through the shell (as JAWS runs, with
  UI Access) heard nothing either.
- **Registering before the window existed:** started 3 s after the window showed and its button
  took the focus, both probes heard nothing.

Neither probe saw any WinEvent of the window's process at the notification's time (only its
window's creation, state and name changes as it showed). How JAWS hears it is open: JAWS's own
code runs inside applications' processes, which an Okur client does not. Next: whether
`RaiseAutomationNotification` returns true in that process (the fixture discards the result),
and whether a notification raised by a UIA provider of the window itself (`notify-host.ps1`,
experiment 4) reaches JAWS. Until then 820's speech is a known gap, not an accepted divergence.

Experiments 10 and 11 (old-pc, 2026-10-23):
- **WinForms raises it:** `RaiseAutomationNotification` returned True in the fixture's process
  ("raised: True, runtime: v4.0.30319", experiment 10).
- **No registration hears it, the control's own included** (experiment 11, `okur-lab-exp11.exe`,
  started 3 s after the window showed): the focused button's element, and the element made from
  its own window's IAccessible (`AccessibleObjectFromWindow`, then `ElementFromIAccessible`),
  were each registered alone and with their ancestors, besides the five registrations above;
  every registration answered Ok and none heard the notification. Both elements' providers are
  UI Automation's MSAA proxy ("Main: MSAA Proxy (unmanaged:UIAutomationCore.DLL)", the focused
  one nested in the HWND and non-client proxies): .NET Framework 4.8's button gives no provider
  of its own across processes, so its notification goes nowhere a client in another process
  listens. Only the window's creation, state and name WinEvents came.
- **So:** an application raising notifications from a .NET Framework WinForms control is heard
  in its own process only (JAWS's code runs inside applications). Okur, an out-of-process client
  as NVDA is, hears notifications from providers of their own (Calculator's, which the probe
  hears; others not tried), and 820 stays a known gap.
