# Microsoft Store release: research notes

Date: 2026-11-01. Research for [`../plans/2026-10-31-microsoft-store-release.md`](../plans/2026-10-31-microsoft-store-release.md).
Research only: nothing was installed, signed or run on the dev box or on old-pc.

Every finding carries source IDs from §4 and a tag:

- **fact**: a source states it;
- **observed**: seen in data fetched today (Store catalog data, source files, file listings);
- **report**: a user, contributor or third party says it happened;
- **inference**: reasoning from the facts, not stated anywhere;
- **open question**: not settled; §8 lists the experiment that settles it.

## 1. The decision and its scope

The decision: can Okur's first release ship as an MSIX package through the Microsoft Store?
If so, what does that build lose against the installed Okur of
[`startup-and-secure-screens-proposal.md`](startup-and-secure-screens-proposal.md) §5, and what
must be built and checked before the owner submits?

Scope: the plan's six questions. Out of scope: macOS, and the secure-desktop design itself
(proposal §5.7 and §5.8).

## 2. The answer in short

- **The Store route works for a screen reader without UI Access.** A full-trust packaged
  process runs at medium integrity as the user. Okur's way of working is allowed there: a
  low-level keyboard hook, out-of-process WinEvents, UI Automation and IAccessible2 clients,
  speech, and `SendInput`. The Store signs the package for free.
- **UI Access in a Store package is documented, but unproven.**
  - Microsoft lists a `uiAccess` restricted capability (R-store-01).
  - NV Access shipped NVDA in the Store with `runFullTrust` and `uiAccess` from 2019
    (R-store-06, R-store-07, R-store-10).
  - Against that, a Microsoft repository has had an open bug since 2021 in which packaged UI
    Access fails ("UI Access is not supported for Desktop AppX processes"), still unresolved
    in 2026 (R-store-04).
  - Microsoft's own page on assistive technology security still says UWP apps cannot have UI
    Access (R-store-03).
  - Installing a UI Access package takes an administrator: the Store shows a UAC prompt, and a
    sideload needs an elevated `Add-AppxPackage` (R-store-10).
  - Approval is case by case, and one user reports a refusal (R-store-04).
  - It has to be tried on old-pc (experiment E1) and asked for in the submission.
- **What the Store build cannot do at all:**
  - the sign-in screen and the other secure desktops;
  - the Ease of Access registration;
  - anything that needs administrator rights.

  A package cannot write HKLM, the Store refuses apps that need elevation, and a package's HKCU
  writes are private to it (R-store-02, R-store-19). Starting after sign-in goes through the
  package's startup task instead (R-store-17, R-store-18).
- **What NVDA's Store build gave up** (§5.2): add-ons and all user code; in-process injection,
  and with it Firefox, Chrome and most of Excel; the sign-in and UAC screens; Ease of Access;
  its elevated helper; portable copies; its own updates. NV Access stopped building it in 2025
  and removed the code in September 2026 (R-store-11, R-store-12, R-store-13).
- **Okur keeps more than NVDA's Store build did** (inference): it injects nothing into other
  processes and reads browsers out of process.

## 3. Contradictions with the project's current assumptions (read first)

1. **"uiAccess probably cannot be in an MSIX"** (plan, question 1): wrong as stated.
   - A `uiAccess` restricted capability exists, with a documented rule: it must be declared
     when the executable's manifest has `uiAccess="true"` (R-store-01, **fact**).
   - NVDA's Store package declares it (R-store-06, **observed**).
   - But it is unreliable, and reported broken for others (R-store-04, **report**).
   - So the answer is "maybe, after an experiment and an approval", not "no".
2. **"A Store-signed package ends Defender's distrust"** (plan, "What the owner has done"):
   not documented.
   - The first certification step is a malware scan (R-store-24, **fact**).
   - A Defender ML detection such as `Bearfoos.A!ml` is therefore more likely to fail
     certification than to be cured by it (**inference**).
   - The false positive needs solving on its own: a submission to Microsoft's malware
     analysis, or a binary that is no longer flagged.
   - What the Store signature does give: one catalog signature covers every file in the
     package (R-store-38, **fact**), and Store-signed packages get content-tamper protection
     (R-store-37, **fact**).
3. **No administrator needed for the Store build** (implicit in the plan): true only without
   `uiAccess`. With it, the Store install shows a UAC prompt and a sideload needs an elevated
   PowerShell (R-store-10, **report** from NV Access's lead developer, 2019).
4. **Settings and start-up keys in HKCU** (proposal §5.4): they do not reach Windows from a
   package.
   - Every package write under HKCU goes to a private per-user, per-app copy (R-store-02,
     R-store-19, **fact**).
   - So `HKCU\...\Accessibility\Configuration` (start after sign-in through Ease of Access)
     would be invisible to Windows, and `HKCU\SOFTWARE\Okur\StartAfterSignIn` would be
     private (**inference**).
   - Use the package's `windows.startupTask` (R-store-17).
5. **Version `0.1.0`** (workspace `Cargo.toml`): not a valid Store package version.
   - The first of the four parts cannot be 0, and the fourth must be 0 (R-store-23,
     **fact**).
   - Map `0.1.0` to, for example, `1.0.0.0` or a date-based `2026.11.0.0`. NVDA used
     `year.major.build.0` (R-store-08, **observed**).
6. **NVDA's Store version as a living precedent** (plan, question 2): it is stale.
   - The published package targets Windows 10 build 18990 and was tested only up to 18995,
     which are late-2019 Insider builds (R-store-06, **observed**).
   - NV Access stopped building AppX in 2025 and removed the code in September 2026, for
     NVDA 2027.1 (R-store-11, R-store-12, **fact**).
   - The precedent shows what Microsoft certified in 2019, not today's practice.
7. **Starting `okur-sr.exe` through `ShellExecuteExW`** (proposal §5.2): it does not carry over
   to a package as is.
   - A packaged app is started through its package activation: the Start menu entry, its
     startup task, or an app execution alias (R-store-17, R-store-22, **fact**).
   - How a packaged UI Access executable restarts itself, or is restarted from `okur.exe`, is
     not documented (**open question**, E4).
8. **The Store's EXE/MSI route as a way around signing**: not available.
   - The Store accepts `.exe`/`.msi` installers only if they and every PE file in them are
     signed with a certificate from a CA in Microsoft's Trusted Root Program (R-store-16
     policy 10.2.9, R-store-23, **fact**).
   - Only MSIX gets free Store signing. `okur-setup.exe` stays unsigned unless the owner finds
     another certificate (proposal §5.12).
9. **A public first submission closes off private testing.** If a product is submitted with
   "Public audience", it can never be switched to "Private audience" later (R-store-32,
   **fact**). The first submission should be Private audience, so that a Store-signed build
   with `uiAccess` can be tested on old-pc before anyone else sees it.

## 4. Sources

Accessed 2026-11-01 unless noted. Strength: **strong** means current official documentation or
primary source code; **medium** means older official documentation, or staff and contributor
comments in issue threads; **weak** means user reports, or catalog data that needs
interpretation.

| ID | Source | Kind | Strength | Supports |
| --- | --- | --- | --- | --- |
| R-store-01 | [App capability declarations](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/app-capability-declarations) (ms.date 2026-10-02) | official | strong | the `uiAccess`, `runFullTrust`, `allowElevation`, `unvirtualizedResources`, `packagedServices` and `inputObservation` rows; the approval process; sideloading needs no approval; full trust means medium IL |
| R-store-02 | [Prepare to package a desktop application](https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-prepare) (ms.date 2023-10-04) | official | strong | the Store refuses apps that need elevation; HKLM writes fail with access denied; HKCU and AppData writes are private; no writing to the install folder; no in-process modules in other processes |
| R-store-03 | [Security Considerations for Assistive Technologies](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-securityoverview) (ms.date 2026-02-18) | official | strong | what UIAccess gives and needs; "UWP Applications do not have UIAccess as an available option" |
| R-store-04 | [WindowsAppSDK issue #1669](https://github.com/microsoft/WindowsAppSDK/issues/1669) (2021-10-28, open; comments read through 2026-06-13) | issue thread: Microsoft members, contributors, users | medium (staff), weak (users) | packaged UIAccess fails with event 0x80070032; deployment fails to register the `windows.uiAccess` extension; staff say the combination "is supposed to work"; a user reports a Store refusal |
| R-store-05 | [msix-packaging issue #486](https://github.com/microsoft/msix-packaging/issues/486) (closed as a duplicate of R-store-04, 2021-11-04) | issue thread | medium | the same bug |
| R-store-06 | NVDA Store listing [9NVL6Z0TM57D](https://apps.microsoft.com/detail/9nvl6z0tm57d), data from [the Store catalog API](https://storeedgefd.dsx.mp.microsoft.com/v9.0/products/9NVL6Z0TM57D?market=US&locale=en-us&deviceFamily=Windows.Desktop) | vendor listing and catalog data | strong (its own text), medium (interpreting fields) | NV Access's list of limitations; capabilities `runFullTrust` and `uiAccess`; min build 18990, max tested 18995; x86 only; ESRB Everyone; privacy URL; Productivity category; listing updated 2025-03-03 |
| R-store-07 | NVDA [`appx/manifest.xml.subst`](https://github.com/nvaccess/nvda/blob/release-2026.2/appx/manifest.xml.subst) at tag `release-2026.2` | source code | strong | the manifest: `rescap` `runFullTrust` and `uiAccess`, `Windows.FullTrustApplication`, `nvda_UIAccess.exe`, a startup task enabled, MinVersion 10.0.18990.0 |
| R-store-08 | NVDA [`appx/sconscript`](https://github.com/nvaccess/nvda/blob/release-2026.2/appx/sconscript) at `release-2026.2` | source code | strong | files left out "due to policy or security restrictions"; the Store publisher is a `CN=<GUID>`; the version scheme; an unsigned Store upload against a signed sideload |
| R-store-09 | [NVDA PR #7851](https://github.com/nvaccess/nvda/pull/7851), "Package NVDA for the Windows Store" (merged 2018-01-04) | PR description and review | strong for NVDA's choices | what was disabled and why; no UIAccess for Store apps then |
| R-store-10 | [NVDA PR #10464](https://github.com/nvaccess/nvda/pull/10464), "Provide NVDA store app with UIAccess capabilities" (merged 2019-11-06) | PR description | strong for what NV Access saw; medium as Windows behavior | UIAccess for packaged apps from build 18990; reading elevated apps such as Regedit; a UAC prompt on Store install; sideload only with an elevated `Add-AppxPackage`; the startup task |
| R-store-11 | [NVDA issue #18799](https://github.com/nvaccess/nvda/issues/18799), "Cleanup AppX code" (2025-08-25) | issue | strong | "no longer building AppX for the Windows Store" |
| R-store-12 | [NVDA PR #20681](https://github.com/nvaccess/nvda/pull/20681) (merged 2026-09-29) and [its diff](https://patch-diff.githubusercontent.com/raw/nvaccess/nvda/pull/20681.diff) | source diff | strong | every `config.isAppX` check NVDA had, so what the Store build disabled |
| R-store-13 | [NVDA `changes.md`](https://github.com/nvaccess/nvda/blob/master/user_docs/en/changes.md) on master (2027.1 section) | changelog | strong | the AppX build and `config.isAppX` were removed |
| R-store-14 | [NV Access, "Why can't NVDA run on Windows 10 Mobile and Windows 10 S?"](https://www.nvaccess.org/post/why-cant-nvda-run-on-windows-10-mobile-and-windows-10-s/) (2017-05-12) | vendor blog | medium (old) | a screen reader needs UIAccess, in-process modules and HKLM, which the Store did not allow in 2017 |
| R-store-15 | [NVDA PR #13032](https://github.com/nvaccess/nvda/pull/13032), comment by Michael Curran, 2021-11-22 | developer comment | medium | Windows 10 has shipped the IAccessible2 proxy registered system-wide "since roughly 2018" |
| R-store-16 | [Microsoft Store Policies 7.20](https://learn.microsoft.com/en-us/windows/apps/publish/store-policies) (ms.date 2026-09-14) | official policy | strong | 10.1.1, 10.1.3 (search terms), 10.1.5, 10.2.2, 10.2.4, 10.2.7, 10.2.8, 10.2.9, 10.4, 10.5.1 (privacy policy), 10.6, 10.14, 11.2, 11.11 (IARC) |
| R-store-17 | [StartupTask class](https://learn.microsoft.com/en-us/uwp/api/windows.applicationmodel.startuptask) (ms.date 2017-02-08) | official | strong | startup-task behavior for packaged desktop apps |
| R-store-18 | [`desktop:StartupTask`](https://learn.microsoft.com/en-us/uwp/schemas/appxpackage/uapmanifestschema/element-desktop-startuptask) (ms.date 2026-06-05) | official | strong | `Enabled`; `rescap5:ImmediateRegistration` needs a custom capability |
| R-store-19 | [Understanding how packaged desktop apps run on Windows](https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-behind-the-scenes) (ms.date 2026-09-30) | official | strong | the install folder; AppData, VFS and registry virtualization; uninstall |
| R-store-20 | [Store and retrieve settings and other app data](https://learn.microsoft.com/en-us/windows/apps/develop/data/store-and-retrieve-app-data) (ms.date 2026-04-08) | official | strong | local, roaming and temporary stores; roaming is gone in Windows 11; data is deleted on uninstall |
| R-store-21 | [UpdateProcThreadAttribute](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute) (ms.date 2021-02-02) | official | medium (ambiguous wording) | `PROC_THREAD_ATTRIBUTE_DESKTOP_APP_POLICY`: child processes inside or outside the package |
| R-store-22 | [`uap3:AppExecutionAlias`](https://learn.microsoft.com/en-us/uwp/schemas/appxpackage/uapmanifestschema/element-uap3-appexecutionalias) (ms.date 2026-06-05) | official | medium (thin page) | the execution alias exists, from build 14393 |
| R-store-23 | [App package requirements for MSIX app](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/app-package-requirements) (updated 2026-08-24) | official | strong | Store re-signing; identity values; version rules; formats; EXE/MSI submissions need your own signature |
| R-store-24 | [The app certification process for MSIX app](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/app-certification-process) (updated 2026-08-24) | official | strong | the certification steps; up to three business days; the malware scan; WACK; signing at publishing |
| R-store-25 | [View app identity details](https://learn.microsoft.com/en-us/windows/apps/publish/view-app-identity-details) | official | strong | Name, Publisher and PublisherDisplayName come from the Product identity page |
| R-store-26 | [Manage submission options for MSIX app](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/manage-submission-options) | official | strong | restricted-capability justification; notes for certification; publishing holds |
| R-store-27 | [Product declarations for MSIX app](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/product-declarations) | official | strong | the accessibility declaration and its conditions; the generative-AI declaration |
| R-store-28 | [Accessibility in the Store](https://learn.microsoft.com/en-us/windows/apps/design/accessibility/accessibility-in-the-store) (ms.date 2026-03-17) | official | strong | what declaring "accessible" does |
| R-store-29 | [Create a certificate for package signing](https://learn.microsoft.com/en-us/windows/msix/package/create-certificate-package-signing) | official | strong | the Subject must equal the Publisher; `New-SelfSignedCertificate` parameters; the LocalMachine Trusted People store |
| R-store-30 | [Sign an app package using SignTool](https://learn.microsoft.com/en-us/windows/msix/package/sign-app-package-using-signtool) | official | strong | `/fd` must match the block map hash (SHA256 by default) |
| R-store-31 | [Windows App Certification Kit](https://learn.microsoft.com/en-us/windows/uwp/debug-test-perf/windows-app-certification-kit) (ms.date 2025-07-14) | official | strong | `appcert.exe reset` / `test -appxpackagepath`; admin rights and an active user session |
| R-store-32 | [Choose visibility options for MSIX app](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/visibility-options) (ms.date 2026-08-28) | official | strong | private audience; no going back from public; package flights |
| R-store-33 | [MSIX features and supported platforms](https://learn.microsoft.com/en-us/windows/msix/supported-platforms) (ms.date 2023-04-11) | official | strong | Store MSIX installs need Windows 10 1809 or later |
| R-store-34 | [Update non-Store published apps from your code](https://learn.microsoft.com/en-us/windows/msix/non-store-developer-updates) (ms.date 2020-06-12) | official | medium (about non-Store updates) | an update applies when the app shuts down; desktop apps call `RegisterApplicationRestart` to come back |
| R-store-35 | [LowLevelKeyboardProc](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc) (ms.date 2025-07-14) | official | strong | the hook runs in the installing thread and is never injected |
| R-store-36 | [SendInput](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput) | official | strong | UIPI: input only into equal or lower integrity |
| R-store-37 | [Inside MSIX: Package Integrity and Enforcement](https://devblogs.microsoft.com/insidemsix/msix-chain-of-trust/) (Howard Kapustein, 2026-07-28) | Microsoft engineering blog | strong | the signature protects the block map, which protects the files; Store-signed packages get content enforcement |
| R-store-38 | [Manage packaged apps with App Control](https://learn.microsoft.com/en-us/windows/security/application-security/application-control/app-control-for-business/design/manage-packaged-apps-with-appcontrol) (ms.date 2024-09-11) | official | strong | every file of an MSIX app is covered by one catalog signature |
| R-store-39 | [Smart App Control](https://learn.microsoft.com/en-us/windows/apps/develop/smart-app-control/overview) (ms.date 2026-09-30) | official | strong | unknown, unsigned code is blocked; code signed by a Trusted Root Program CA is allowed |
| R-store-40 | [microsoft/winappCli](https://github.com/microsoft/winappCli) (last push 2026-10-30) | official tool, public preview | medium | `winapp pack`, `cert`, `sign`, `manifest`; Rust is named as supported |
| R-store-41 | [crabnebula-dev/cargo-packager](https://github.com/crabnebula-dev/cargo-packager) (last push 2026-10-31) | open-source tool | strong for its own feature list | no MSIX: NSIS and WiX MSI only on Windows |
| R-store-42 | [davidanthoff/cargo-msix](https://github.com/davidanthoff/cargo-msix) (last push 2024-06-09; crates.io 0.5.1, 2024-01-13) | open-source tool | medium | a cargo subcommand for MSIX; not maintained recently |
| R-store-43 | [Choochmeque/msixbundle-rs](https://github.com/Choochmeque/msixbundle-rs) (last push 2026-10-28; crates.io 1.1.15, 2026-09-01) | open-source tool | medium | packs, signs and bundles MSIX by driving the Windows SDK tools |
| R-store-44 | [crates.io search "msix"](https://crates.io/api/v1/crates?q=msix) | registry data | medium | the list of Rust MSIX crates with versions and dates |
| R-store-45 | Store catalog search API, queries "screen reader", "NVDA", "JAWS screen reader", "SuperNova", "ZoomText", "Dolphin screen reader", "screen reader for blind" (`https://storeedgefd.dsx.mp.microsoft.com/v9.0/search?...`) | catalog data | weak (ranking unknown) | no third-party full screen reader among the top results |

Already cited in the proposal and not repeated: the AT registration values ([MS1] there), the UIAccess
policy ([MS4]), and NVDA's installer mechanics ([N1]–[N13]).

## 5. Findings

### 5.1 Question 1: UI Access and what a full-trust packaged process may do

**Full trust.**

- A packaged desktop app that runs at medium integrity is a "full trust" app. It must declare
  the `runFullTrust` restricted capability (R-store-01, **fact**).
- Medium-IL apps "already running as the user" need no capability for things the user can
  already do (R-store-01, **fact**).
- NVDA's package uses the older form, `EntryPoint="Windows.FullTrustApplication"` on the
  `Application` element (R-store-07, **observed**).
- The newer form is `uap10:TrustLevel="mediumIL"` with
  `uap10:RuntimeBehavior="packagedClassicApp"`, from Windows 10 2004, build 19041
  (R-store-19, **fact**).

**The `uiAccess` restricted capability** (R-store-01, **fact**):

- it is named "UI Access", for apps that interact with other applications' UI "even when they
  are running with higher privileges";
- the row says it "must be specified when the uiAccess attribute of the requestedExecutionLevel
  element is set to true in the app manifest file";
- it links to the assistive technology security page.

**Other restricted capabilities that touch this** (R-store-01, **fact**):

- `allowElevation`: for partners or enterprises, "subject to approval under strict criteria",
  with an email to `reportapp@microsoft.com` in advance.
- `inputObservation` and `inputSuppression`: "only available for use by select Microsoft
  partners". These cover the WinRT input APIs, not `WH_KEYBOARD_LL` (**inference**: the rows
  name raw input observation and suppression through APIs, and the hook needs no capability).
- `inputInjectionBrokered`: WinRT input injection, received only by the same app container.
  Not needed for `SendInput` (**inference**).
- `unvirtualizedResources`: turns off registry and AppData write virtualization. It is meant
  for games and for packages with an external location, and is "not intended to be used for
  other scenarios".
- `packagedServices` and `localSystemServices`: "In most cases, the use of this capability
  won't be approved."
- `uiAutomation` (also in the restricted list): it lets a UI Automation client "such as
  Narrator" connect to providers. NVDA's full-trust package did not declare it (R-store-07,
  **observed**). Okur does not need it as a full-trust app (**inference**).

**The approval process** (R-store-01, R-store-26, **fact**):

- Partner Center detects restricted capabilities on upload. On the Submission options page
  you explain, per capability, why the app needs it and how it uses it.
- Testers review that during certification, which adds time. A refusal fails certification
  and the report says why.
- Approval generally carries over to updates.
- You may also ask in advance through the Microsoft account team (that path is written for
  sandbox and managed partners), which "typically takes 5 business days or longer".
- Some restricted capabilities are "almost never approved"; the table marks them. `uiAccess`
  is **not** among the ones marked that way (**observed** in R-store-01's rows).
- Sideloading a package with restricted capabilities needs no approval.

**Has Microsoft approved a screen reader with it?**

- Yes, once that I can show. NV Access's published package carries the capabilities
  `runFullTrust` and `uiAccess` (R-store-06, **observed**).
- NV Access tested it "installed via a private Store submission URL" in November 2019
  (R-store-10, **fact** for what they report).
- When that approval was given, restricted capabilities were approved through support, not
  the current Partner Center form (R-store-01 says the form replaced contacting support;
  **inference** on the timing).
- A user wrote on 2024-12-03 that the Store refused the `uiAccess` capability, after a long
  review, for "a mouse replacement utility" (R-store-04, **report**, weak).
- Microsoft's AT page says UIAccess must not be used "by applications that are not assistive
  technologies" (R-store-03, **fact**). A screen reader is the case it is written for
  (**inference**).

**Does Windows honor it?**

- NVDA's November 2019 report: "in recent Windows insider builds" a Desktop Bridge app can
  request UIAccess (R-store-10).
  - The UIAccess privilege is "only available in Windows Insider build 18990 and higher".
  - The packaged NVDA then read elevated apps such as Regedit, and audio ducking and touch
    worked.
  - A Store install shows a UAC prompt. A sideload "will fail" unless installed with
    `Add-AppxPackage` from an administrator PowerShell.
  - (**fact** that NV Access says so; **inference** that it still holds.)
- WindowsAppSDK #1669, opened 2021-10-28 on Windows 11 build 22489 (R-store-04, **report**):
  - a package with `runFullTrust` and `uiAccess`, and an EXE with
    `asInvoker uiAccess="true"`, fails to start;
  - event: `0x80070032 ... UI Access is not supported for Desktop AppX processes`;
  - a Microsoft member wrote on 2021-11-05 that `runFullTrust` plus `uiAccess` "is supposed to
    work" and that the documentation was wrong;
  - on 2023-08-05 a contributor found that deploying an unsigned test package failed to
    "register the windows.uiAccess extension" with "Access is denied", and named an internal
    feature `CentennialUiAccess` added about three years earlier;
  - on 2025-07-01 the same contributor wrote it is "Not going to get fixed";
  - the issue is still open, with the latest comment on 2026-06-13.
- Reading these together (**inference**): the 2019 feature exists, and installing a UI Access
  package needs an elevated deployment. That matches both NVDA's note and the "Access is
  denied" event. The 2021 failure may be a deployment that was not elevated, or a different
  defect. Only E1 settles it for Okur.
- Microsoft's AT security page still says "UWP Applications do not have UIAccess as an
  available option" (R-store-03, **fact**). It does not mention packaged desktop apps; it is
  about UWP apps.

**What UIAccess requires and gives** (R-store-03, **fact**; details in proposal §2.6):

- An Authenticode signature, a secure install location (Program Files is the example), and
  the manifest flag.
- With an administrator who signed in: High integrity and access to elevated UI.
- With a standard user: "medium+", with no access to elevated UI.
- Never access to UI at system integrity, such as the UAC prompt itself.
- `C:\Program Files\WindowsApps` lies under Program Files (R-store-19, **fact**). Whether the
  AppInfo service treats it as a secure location is not documented. A contributor believed it
  is not (R-store-04, **report**).

**What a full-trust packaged process may do** (Okur's needs):

| Need | Allowed? | Evidence |
| --- | --- | --- |
| `WH_KEYBOARD_LL` hook | yes | The hook "is not injected into another process"; it is called in the installing thread's context (R-store-35, **fact**). The packaging restriction is on modules loaded into other processes (R-store-02, **fact**). NVDA's Store build kept its keyboard handling (R-store-12 shows no `isAppX` check around it, **observed**). |
| `SetWinEventHook` out of context | yes (**inference**) | No DLL enters other processes. In-context hooks would load a DLL into other processes, which packaging forbids (R-store-02). |
| UI Automation client | yes (**inference**) | Medium IL as the user (R-store-01). NVDA's Store build read UIA apps (R-store-06 lists Firefox, Chrome and Excel as the exceptions). |
| IAccessible2, out of process | yes (**inference**) | Okur calls IA2 from its own process (`crates/xa11y/xa11y-windows/src/ia2.rs`). Windows 10 has shipped the IA2 proxy registered system-wide since about 2018 (R-store-15, **report**). NVDA's Store build lost browsers because NVDA builds browse mode inside the browser process (R-store-09, R-store-12). |
| SAPI 5 voices | yes (**inference**) | Reads of `HKLM\Software` see the real registry merged with the package's hive (R-store-19, **fact**), so installed voices are listed. A voice engine's DLL loads into Okur's own process, which is not forbidden. |
| OneCore voices (WinRT) | yes (**inference**) | Package identity only adds to what an unpackaged app can do. |
| `SendInput` | yes, within UIPI | Only into equal or lower integrity (R-store-36, **fact**). Without UI Access, not into elevated windows. |
| Reading or driving elevated windows | only with UI Access | R-store-03 (**fact**). |
| Writing HKLM | no | "access-denied failure" (R-store-02, **fact**). |
| Elevating any part | no, for the Store | "apps that require elevation for any part of their functionality won't be accepted into the Store" (R-store-02, **fact**). `allowElevation` exists but is approved only "under strict criteria" (R-store-01). |
| Writing next to its own EXE | no | The package folder is read-only (R-store-02, R-store-19, **fact**). |
| Launching cmd or PowerShell | discouraged | It may block submission for Windows 10 S compatibility (R-store-02, **fact**). |

**What virtualization changes** (R-store-19, R-store-02, **fact**):

- New files under `AppData` (Local, Roaming and some Microsoft subfolders) go to a private
  per-user, per-package location from Windows 10 1903. Existing files in the real `AppData`
  are read and changed in place.
- Every HKCU write goes to a private per-user, per-app location.
- HKLM reads merge the package's `registry.dat` with the real hive.
- On uninstall, the package folder and the redirected `AppData` and registry writes are
  removed.

### 5.2 Question 2: NVDA's Store version

**The listing** (R-store-06, **observed**, 2026-11-01):

- product 9NVL6Z0TM57D, publisher NV Access Limited;
- package family `NVAccessLimited.NVDANonVisualDesktopAccess_pw2g7jqch2xgm`;
- category Productivity; ESRB "Everyone"; privacy policy `https://www.nvaccess.org/privacy/`;
- x86 only; minimum OS 10.0.18990.0, maximum tested 10.0.18995.0 (decoded from the catalog's
  packed version numbers);
- first available 2017-09-21; listing last updated 2025-03-03;
- the release note in the listing is about NVDA 2019.3;
- seven ratings averaging 3.4;
- the description's limitations, in NV Access's words: "No support for NVDA add-ons", "No
  support for running on the Windows logon screen", "No support for reading UAC (User Account
  Control) screens", "No support for Mozilla Firefox or Google Chrome", "Very limited support
  for Microsoft Excel".

A search of the Store catalog for "NVDA" did not return this listing among its top results
(R-store-45, **observed**).

**The build** (R-store-07, R-store-08, **observed**, tag `release-2026.2`):

- The manifest declares `rescap` `runFullTrust` and `uiAccess`.
- The one application is `nvda_UIAccess.exe` with `EntryPoint="Windows.FullTrustApplication"`.
- A `desktop:StartupTask` with `Enabled="true"`.
- The build drops these files "due to policy or security restrictions":
  - `nvda_slave.exe` (the elevated helper);
  - `nvda_noUIAccess.exe`;
  - `lib/IAccessible2Proxy.dll` and `lib/ISimpleDOM.dll`;
  - `lib/NVDAHelperRemote.dll` (the in-process injection library);
  - the `lib64/` and `libArm64/` folders;
  - `uninstall.exe`.
- The Store upload is unsigned, with publisher `CN=83B1DA31-9B66-442C-88AB-77B4B815E1DE`. The
  sideload build is signed, with the certificate's own subject.
- The package version is `year.major.build.0`.
- The product name was "NVDA Screen Reader (Windows Store Edition)".

**What the Store build gave up**, from every `isAppX` check removed in R-store-12 (**observed**),
with the reasons from R-store-09:

| Area | In the Store build | Why (R-store-09 unless noted) |
| --- | --- | --- |
| Process injection (`NVDAHelperRemote`) | off; "Remote injection disabled" | injection was not allowed; so no Firefox, Chrome or Internet Explorer browse mode, and little Excel |
| IA2 support installed in other processes | not done | same |
| Add-ons, add-on store, add-on GUI, add-on braille tables | off | "not allowed by Store policy" (2017) |
| Custom code from the user's config (appModules, globalPlugins, drivers), scratchpad | off | "we cannot currently allow any custom code" (2017) |
| Python console, Reload plugins | off | Store policy (2017) |
| Create portable copy | off | — |
| Update check, usage-statistics question | off; "updates managed by Windows Store" | — |
| Restart after a Store update | `RegisterApplicationRestart` on `WM_QUERYENDSESSION` | Windows closes the app to update it |
| Ease of Access notifications (`easeOfAccess.notify(3/2)`) | off | Ease of Access "is not notified about this copy" |
| User config folder | a separate `..._appx` folder | the shared config may name add-ons the Store copy cannot load |
| Sign-in screen, UAC screens | not supported | no HKLM registration; secure desktops (R-store-06) |
| Touch, audio ducking, elevated apps | off until 2019.3; on from build 18990 with `uiAccess` | "store apps not being allowed to run with the UIAccess privilidge" (2017); R-store-10 (2019) |
| Actions blocked at run time | announced "Action unavailable in NVDA Windows Store version" | `gui.blockAction.Context.WINDOWS_STORE_VERSION` |

**Status.** NV Access stopped building AppX ("We are no longer building AppX for the Windows
Store", R-store-11, 2025-08-25) and removed the code for 2027.1 on 2026-09-29 (R-store-12,
R-store-13, **fact**). The PR says future Store distribution, if any, would use MSIX
(R-store-12, **fact**).

**Other screen readers in the Store** (R-store-45, **observed**, weak): searches for "screen
reader", "JAWS screen reader", "SuperNova", "ZoomText", "Dolphin screen reader" and "screen
reader for blind" returned text-to-speech readers, ebook readers and Be My Eyes, but no
third-party full screen reader. The Store's ranking is unknown, so this shows absence among the
top results only. Narrator is part of Windows, not a Store app (**inference**, not looked up).

### 5.3 Question 3: starting at sign-in; the sign-in screen and secure desktops

**The startup task** (R-store-17, R-store-18, **fact**):

- Declared as a `windows.startupTask` extension (`desktop:` or `uap5:` namespace) with
  `Executable`, `EntryPoint="Windows.FullTrustApplication"`, a `TaskId` and an optional
  `Enabled`.
- A packaged desktop app may set `Enabled="true"`. It then needs no `RequestEnableAsync`, and
  no consent dialog is shown.
- In every case, start-up begins only after the user has launched the app once, or turned it
  on in Settings' Startup page.
- From then on the user controls it in Settings' Startup page or Task Manager's Startup tab.
- If the user turned it off there, `RequestEnableAsync` will not override that.
- `StartupTaskState` includes `DisabledByUser` and `DisabledByPolicy`.
- `rescap5:ImmediateRegistration` enables the task at install without a first launch. It
  requires the custom capability `Microsoft.nonUserConfigurableStartupTasks_8wekyb3d8bbwe`,
  which also takes the switch away from the user (R-store-01). Not for Okur (**inference**).
- It starts the app when the user signs in. Not before sign-in, and not on the lock screen
  (**fact**: "runs at system startup or when the user logs in").
- Whether Windows delays startup apps after sign-in, and by how much, is not stated in these
  pages (**open question**, E3).

**The sign-in screen and secure desktops: not from a package.**

- Windows starts an AT there from `HKLM\...\Accessibility\ATs\<name>` and the HKLM
  `Configuration` value (proposal §2.2, from Microsoft's AT registration page). A package
  cannot create or modify HKLM keys (R-store-02, **fact**) and cannot elevate (R-store-02,
  **fact**).
- NVDA's Store build skipped Ease of Access entirely and lists "No support for running on the
  Windows logon screen" (R-store-06, R-store-12, **fact**).
- I found no package manifest extension that registers an assistive technology with Windows
  (searched 2026-11-01; **observed** absence, not proof).
- Even with a registration written by some other means, `StartExe` would have to point into
  `C:\Program Files\WindowsApps\<package_full_name>`. That folder changes with every version
  (R-store-19, **fact** that the name contains the full package name, which includes the
  version). Packages are also registered per user, so the SYSTEM copy on the secure desktop
  has no registration (**inference**).
- The UAC prompt runs at system integrity, which no UIAccess process reaches (R-store-03,
  **fact**). Reading it always needs the SYSTEM copy on the secure desktop (proposal §2.6).

**Consequence** (**inference**): the Store build says plainly, in its description and its
settings, that it does not run on the sign-in screen, the lock screen or UAC prompts. The
secure-screen route stays with `okur-setup.exe`, which needs its own signature.

### 5.4 Question 4: the package's folders and processes

**Folders** (R-store-19, R-store-20, R-store-02, **fact**):

- **Install:** `C:\Program Files\WindowsApps\<package_full_name>`, read-only and locked
  against tampering. Users may move packages to other volumes.
- **App data:** `ApplicationData.LocalFolder`, `LocalCacheFolder` and `TemporaryFolder`.
  LocalSettings "reliably persists through app updates".
- **Roaming:** "no longer supported as of Windows 11", and RoamingSettings may not survive
  Store updates.
- **Uninstall:** everything in the app-data stores is deleted.
- **Plain Win32 file APIs:** writes to `%APPDATA%\Okur\...` for a new file land in the
  per-package private location and are deleted on uninstall. A file that already existed in
  the real `%APPDATA%` (from an earlier, unpackaged Okur) is opened and changed in place
  (Windows 10 1903 and later).
- **Logs:** never next to the EXE; use the local or local-cache store.
- **Where those stores are on disk:** the pages fetched do not give the path. It is commonly
  `%LOCALAPPDATA%\Packages\<PackageFamilyName>\...` (**inference** from common knowledge, not
  verified here; E5 confirms it).

**Several executables in one package.**

- A package may hold several executables and several `Application` elements, each with a
  unique `Id` (search summary of the `Application` element page; **fact**, medium).
- NVDA's package had one application (R-store-07).
- Okur needs one application entry (`okur-sr.exe`, the screen reader) for the Start menu and
  the startup task. `okur-ui.exe` can stay an ordinary file in the package that `okur-sr.exe`
  starts by full path (**inference**).
- `okur.exe` (the developer console program) and `okur-setup.exe` do not belong in the Store
  package: one is a developer tool, and the other needs elevation, which the Store refuses
  (**inference** from R-store-02).

**Starting the helper `okur-ui.exe`** (**open question**, E4):

- `UpdateProcThreadAttribute` documents `PROC_THREAD_ATTRIBUTE_DESKTOP_APP_POLICY` for Desktop
  Bridge apps (R-store-21, **fact**). Its flags choose whether children run "inside" or
  "outside of the desktop app runtime environment". The page calls the "outside" behavior "the
  default for processes for which no policy has been set". It is unclear whether that means a
  plain `CreateProcess` child leaves the package.
- Whether `okur-ui.exe` sees the same virtualized `AppData` as `okur-sr.exe` follows from that.
- Proposal §5.2 lowers the child's token (no UIAccess, Medium). That still applies if
  `okur-sr.exe` gets UI Access.

**Starting Okur by name.** An app execution alias (`uap3:AppExecutionAlias`, from build 14393,
R-store-22, **fact**) is the packaged replacement for proposal §5.4's `App Paths\okur.exe`
(**inference**). The exact manifest syntax still needs looking up. Whether Win+R finds the
alias is to be tested (E4).

**Speech.** SAPI 5 and OneCore voices: see §5.1 (**inference**, E2 confirms).

### 5.5 Question 5: identity, versions, updates, signing, tools

**Identity** (R-store-25, R-store-23, **fact**):

- `Package/Identity/Name`, `Package/Identity/Publisher` and
  `Package/Properties/PublisherDisplayName` must be copied from Partner Center's Product
  identity page, exactly and case-sensitively. They define the package family.
- Partner Center also shows the Package Family Name, the Package SID, the Store ID and the
  listing link `https://apps.microsoft.com/detail/<Store ID>`.
- NV Access's Store publisher is a `CN=<GUID>` value (R-store-08, **observed**). Okur's
  publisher will be of the same form (**inference**).

**Versions** (R-store-23, **fact**):

- Four parts. The fourth "is reserved for Store use and must be left as 0". The others are
  0–65535, except that the first "cannot be 0".
- The Store gives each device the highest-versioned package that applies to it.
- Packages may be submitted in any order. Same version is allowed only with different
  architectures.
- Rolling back means resubmitting the old package. Devices that already have the bad version
  keep it until a higher version arrives.

**Supported Windows** (R-store-33, R-store-23, **fact**):

- Store MSIX installs need Windows 10 1809 or later. `TargetDeviceFamily` `MinVersion` and
  `MaxVersionTested` declare the range.
- A package with `uiAccess` needs at least 10.0.18990 (R-store-10, **report**).
- old-pc runs 19045 and the dev box 26200 (proposal §2.7).

**Store signing** (R-store-23, R-store-24, **fact**):

- Packages need not be signed with a trusted certificate for submission. The Store "will
  automatically re-sign" MSIX/AppX packages "with a Microsoft certificate" after certification.
- No purchased certificate, `.pfx`, USB token or HSM is needed.
- The Store does **not** re-sign EXE/MSI submissions.
- An MSIX distributed outside the Store must be signed with your own certificate.

What the Store signature covers:

- Every file in the package shares one catalog signature (R-store-38, **fact**).
- The block map protects the files and the signature protects the block map (R-store-37,
  **fact**).
- Smart App Control allows code that Microsoft's service recognizes, or that is signed by a
  Trusted Root Program CA; unknown unsigned code is blocked (R-store-39, **fact**). The page
  does not say how it treats Store packages. A Microsoft-signed package should pass
  (**inference**).

**Updates.**

- The Store updates installed apps (R-store-33 lists "Update Notification" and "Delta Updates"
  as Store features, **fact**).
- An update to a running packaged app is applied when the app shuts down. Desktop apps that
  want to come back register with `RegisterApplicationRestart` before the shutdown
  (R-store-34, **fact**, written for non-Store updates).
- NVDA registered for restart on `WM_QUERYENDSESSION` because "Windows closes the Store app
  during an update" (R-store-09, R-store-12, **fact** for NVDA).
- For a screen reader, an update must never leave the user without speech. Okur must restart
  itself after a Store update (**inference**; E6 tests it).

**Outside the Store, for local tests** (R-store-29, R-store-30, **fact**):

- The test certificate's Subject must equal the manifest's `Publisher`. For a test build of the
  Store identity, that is the Partner Center `CN=<GUID>` (**inference**).
- Create it with `New-SelfSignedCertificate -Type Custom -KeyUsage DigitalSignature
  -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")`, export it to
  `.pfx`, and import it into `Cert:\LocalMachine\TrustedPeople` on the test machine. That is a
  machine-wide trust change: the owner's to approve on old-pc. Remove it afterwards.
- `SignTool sign /fd SHA256 /a /f <pfx> /p <password> <package>.msix`. The `/fd` algorithm
  must match the block map's, which MakeAppx sets to SHA256 by default.
- A sideload with `uiAccess` must be installed from an elevated PowerShell (R-store-10,
  **report**).

**Distributing the same app outside the Store.** Allowed, signed with your own certificate
(R-store-23, **fact**). A public CA will not issue a certificate whose subject is the Store's
`CN=<GUID>`, so an outside package would carry a different Publisher and so a different package
family: a separate app to Windows (**inference** from R-store-29's subject rule). That route
needs the certificate Okur does not have (proposal §5.12).

**`okur-setup.exe` after the Store release** (**inference**):

- It stays the development and lab route on old-pc (Program Files, UIAccess with the lab's
  self-signed root).
- Later it is the only route to the sign-in screen and secure desktops.
- It cannot use the Store for signing (policy 10.2.9 needs a CA signature, R-store-16).

**Windows App Certification Kit** (R-store-31, **fact**):

- Part of the Windows SDK.
- From the command line: `appcert.exe reset`, then
  `appcert.exe test -appxpackagepath <package> -reportoutputpath <report>`, from an
  administrator prompt in an active user session.
- Desktop Bridge tests are included.
- On the dev box, `C:\Program Files (x86)\Windows Kits\10\App Certification Kit\appcert.exe`,
  and `makeappx.exe` and `signtool.exe` under `...\bin\10.0.26100.0\x64\`, are present
  (**observed** by file listing; nothing was run).

**Rust tooling** (checked 2026-11-01):

| Tool | MSIX? | State | Source |
| --- | --- | --- | --- |
| Windows SDK `MakeAppx.exe` + `SignTool.exe` | yes | Microsoft's own tools; present on the dev box | R-store-29, R-store-30, observed |
| `winapp` CLI (Microsoft) | yes: `pack`, `cert`, `sign`, `manifest` | public preview, "experimental and in active development"; MIT; names Rust as supported; `winget install Microsoft.winappcli` | R-store-40 |
| `msixbundle-rs` / `msixbundle-cli` | yes, by driving the SDK tools | 1.1.15 (2026-09-01); 25 stars | R-store-43, R-store-44 |
| `cargo-msix` | yes | 0.5.1 (2024-01-13); last push 2024-06-09: unmaintained in practice | R-store-42, R-store-44 |
| `cargo-packager` | **no** (NSIS and WiX MSI on Windows) | active | R-store-41 |
| `msix` crate (cloudpeers) | library | 0.4.0 (2022-12-21) | R-store-44 |
| `pineapple-msix` | yes | 0.2.0 (2026-09-03), 70 downloads | R-store-44 |

Recommendation (**inference**): keep a hand-written `AppxManifest.xml` template in the repository
and a small script that stages the files and calls `MakeAppx pack` and `SignTool`. Those are the
documented tools, already installed, and NVDA's build did the same (R-store-08). `winapp` is
worth a look later, once it leaves preview. No Rust packaging crate is both maintained and
widely used.

### 5.6 Question 6: the listing, policies and certification

**Privacy policy: mandatory.** "Product types that inherently have access to Personal
Information must always have privacy policies. These include ... Desktop Bridge and Win32
products." The URL goes into Partner Center (R-store-16, 10.5.1, **fact**).

- The policy must say what is accessed, how it is used, stored and secured, and to whom it is
  disclosed.
- Okur reads everything on screen, so the policy must say that this stays on the device, and
  what changes when the planned AI and remote features arrive (**inference**).
- NVDA's listing links `https://www.nvaccess.org/privacy/` (R-store-06).

**Age rating.** Obtained in Partner Center through the IARC questionnaire, and kept current
(R-store-16, 11.11, **fact**). NVDA's rating is ESRB "Everyone" (R-store-06).

**Naming other products** (R-store-16, **fact**):

- **Search terms:** at most seven, and they must "Not use other product titles unless those
  products are also published by you" (10.1.3). So "JAWS" cannot be a search term.
- **Metadata:** must not "attempt to mislead customers as to its ... relationship to other
  products", and must not use "a name, images, or any other metadata that is the same as that
  of other products" (10.1.1).
- **Content:** must be original, licensed, permitted by the rights holder "or used as otherwise
  permitted by law". Owners can file infringement complaints (11.2).
- **Inference:** the policies do not forbid a factual comparison in the description, such as
  keyboard commands familiar to JAWS users, but a rights holder may complain. Whether to
  compare at all is the owner's decision (plan, question 6; AGENTS.md §1.5). NVDA's listing
  names no other screen reader (R-store-06).

**Describing limits.** Metadata must state "any important limitations" (10.1.1, **fact**). NVDA's
listing lists its limits (R-store-06). Okur's description should list: no sign-in or lock
screen; no UAC prompts; no elevated windows if `uiAccess` is not approved; and the README's
"Not yet" items (**inference**).

**Accessibility declaration** (R-store-27, R-store-28, **fact**):

- The Product declarations checkbox "This product has been tested to meet accessibility
  guidelines" makes the app findable through the Store's "Accessible" filter and adds an
  "Accessible" tag.
- Tick it only after: accessible names set; full keyboard operation; 4.5:1 text contrast; Inspect
  or AccChecker or UIAVerify run with high-priority errors fixed; and key scenarios verified with
  Narrator, Magnifier, On-Screen Keyboard, high contrast and high DPI.
- For Okur this concerns its own windows: settings, Virtual Viewer, dialogs (**inference**).
- Another declaration, "incorporates generative AI features", becomes required when the planned
  AI image descriptions ship (R-store-27, **fact**). Policy 11.16 then applies (not read here).

**Other policies that touch Okur** (R-store-16, **fact**, with **inference** on how they apply):

- **10.2.8**: changes to Windows settings must use supported methods with user consent;
  "use of accessibility APIs ... in unsupported ways" is named as unsupported. Okur changes
  settings only when the user operates them; say so in the certification notes.
- **10.2.2**: no "dynamic inclusion of code" that changes described functionality. This matters
  for the future extension SDK.
- **10.1.5**: allows "Add-ons or extensions, excluding non-Microsoft drivers or NT services"
  acquired after download with consent. Whether an extension SDK fits that is an **open
  question** for later.
- **10.2.4**: dependencies on non-Microsoft drivers or NT services are generally not allowed.
  Okur has none.
- **10.2.7**: a clean uninstall must be possible. MSIX provides it (R-store-19).
- **10.4.1**: on an incompatible device, detect it at launch and tell the user.
- **10.4.2**: start promptly, stay responsive, shut down gracefully.
- **10.6**: declared capabilities must "legitimately relate to the functions of your product".
- **10.14**: an individual account suits "a single developer working on their own". A company
  account is needed if users would read the publisher name as a business.

**Certification** (R-store-24, R-store-26, R-store-32, **fact**):

1. Preprocessing.
2. Security tests: a virus and malware scan.
3. Technical compliance by the Windows App Certification Kit.
4. Content compliance by people, using the "Notes for certification".
5. Release and publishing, when Microsoft signs the packages.

- Up to three business days; listings appear about 15 minutes after publishing.
- Restricted-capability review adds time.
- Spot checks continue after publishing.
- Publishing can be held until "Publish now" or a date.
- A **Private audience** limits the listing and download to named personal Microsoft accounts.
  Updates reach them as normal updates, and package flights can target subsets.
- Moving from Private to Public can trigger another certification. Public can never go back to
  Private.
- Questions about a review go to `reportapp@microsoft.com` (R-store-16).

## 6. What an Okur Store build would give up, against proposal §5

| Proposal item | Store build | Basis |
| --- | --- | --- |
| `okur-sr.exe` with UIAccess in Program Files | only if E1 works and Microsoft approves `uiAccess`; else medium IL, no elevated windows | §5.1 |
| Sign-in screen, lock screen, UAC (§5.7, §5.8) | no | §5.3 |
| HKLM `ATs\okur_okur_v1`, `Configuration`, `App Paths`, `Run`, `Uninstall` (§5.4) | no; Windows' own uninstall; an execution alias instead of `App Paths` | R-store-02, R-store-22 |
| Start after sign-in through Ease of Access (§5.5) | startup task instead; on after the first launch; user-controlled in Settings and Task Manager | R-store-17 |
| "All users" start | no: packages and startup tasks are per user | R-store-19 (per-user install), **inference** |
| `okur-setup.exe` (§5.10) | not in the package | R-store-02 |
| Settings in `%APPDATA%\Okur` | work, but new files are private to the package and deleted on uninstall | R-store-19 |
| Single-instance mutex and quit command (§5.6, §5.9) | unchanged (**inference**) | — |
| Reading Chrome, Word, Excel | expected to work: no injection (**inference**; E2) | §5.1 |
| Own update check | none; Store updates plus restart after update | R-store-34, R-store-12 |

## 7. Glossary

- **Package identity**: the Name, Publisher, version and architecture in `AppxManifest.xml`. Name
  plus a hash of Publisher make the **package family name (PFN)**.
- **Full trust / mediumIL**: a packaged desktop process that runs at medium integrity as the
  user, outside an app container. It needs `runFullTrust`.
- **Restricted capability (`rescap`)**: a manifest capability that needs Microsoft's approval
  for the Store, and none for sideloading.
- **Desktop Bridge / Centennial / packagedClassicApp**: names for a Win32 app packaged as
  MSIX.
- **Startup task**: the `windows.startupTask` extension that starts a packaged app at user
  sign-in.
- **App execution alias**: a name under which a packaged app can be started from a command
  line.
- **Store signing**: Microsoft re-signs certified MSIX packages with its own certificate.
- **Private audience / package flight**: Partner Center's ways to give a Store-signed build only
  to named Microsoft accounts.
- **WACK**: Windows App Certification Kit (`appcert.exe`), the technical test the Store also runs.
- **IARC**: International Age Rating Coalition questionnaire, which produces the age rating.

## 8. Implications: requirements, experiments and tests

**Requirements for the package (step 2 of the plan):**

1. **Identity.** `AppxManifest.xml` uses Name, Publisher and PublisherDisplayName copied from
   Partner Center's Product identity page. The repository keeps them in one place, and the
   build fails if they are empty.
2. **Version.** The package version is derived from the workspace version with a non-zero first
   part and a zero fourth part. A hermetic test covers the mapping, including `0.1.0`.
3. **Architecture.** x64 first (old-pc and the dev box), as one `.msix`; an `.msixbundle` when
   Arm64 is added.
4. **Contents.** `okur-sr.exe` (as the one `Application`, full trust), `okur-ui.exe`, locales,
   help, logos. Not `okur.exe`, not `okur-setup.exe`, not `okur-lab`.
5. **Capabilities.** `runFullTrust`. `uiAccess` only in a variant that E1 has shown to work, with
   the matching EXE manifest (`okur-sr.manifest`, `uiAccess="true"`). The variant without it
   uses `okur-sr-no-uiaccess.manifest`'s settings.
6. **Minimum Windows.** `MinVersion` 10.0.18990.0 or higher if `uiAccess` is declared
   (R-store-10); else 10.0.17763.0 (R-store-33).
7. **Start-up.** A `windows.startupTask` with `Enabled="true"`.
8. **Runtime switch.** Okur knows it is packaged (`GetCurrentPackageFullName`, as NVDA did,
   R-store-12). It then hides the sign-in-screen and all-users options, writes nothing to HKLM,
   and restarts itself after a Store update with `RegisterApplicationRestart`.
9. **Logs and settings.** Never next to the EXE. `okur doctor` prints where they are in the
   packaged build.

**Experiments on old-pc** (each needs the owner's approval of the test certificate, a
machine-wide trust change; remove it afterwards):

- **E1, UI Access in a package.**
  1. Build a test-signed package with `rescap` `uiAccess` and `okur-sr.exe` carrying
     `uiAccess="true"`.
  2. Install it once with `Add-AppxPackage` from a normal PowerShell. Expect a deployment
     failure; record the AppXDeploymentServer event.
  3. Install it once from an elevated PowerShell.
  4. Start it from the Start menu. Pass if `okur doctor` shows `TokenUIAccess` = 1 and High
     integrity for the administrator account, and Okur reads and drives an elevated Regedit.
  5. Record the result either way, with the event log text.
- **E2, the package without UI Access.** Okur starts, speaks with SAPI 5 and OneCore voices,
  hooks keys, and reads Chrome (IA2), Word and Excel, as the unpackaged build does on the same
  scenarios. Write down what fails in elevated windows.
- **E3, the startup task.** Launch once, sign out, sign in. Okur starts. Measure seconds from
  sign-in to first speech, and compare with the Ease of Access start of the unpackaged build.
- **E4, processes.**
  - Is `okur-ui.exe`, started by `okur-sr.exe` with `CreateProcess`, inside the package
    (`GetCurrentPackageFullName` in the child)?
  - Does it see the same settings?
  - Does Win+R `okur` find the execution alias?
  - Can a running Okur restart itself (alias or `shell:AppsFolder\<PFN>!<AppId>`)?
- **E5, files.** Where settings and logs land, and that uninstall removes them.
- **E6, Store update.** With a Private-audience submission and a second version: Okur restarts
  by itself after the update, and speech returns without the user doing anything.
- **E7, WACK.** `appcert.exe test -appxpackagepath` on the package passes; keep the report.

**E1's results** (old-pc, Windows 10 19045.6456, 2026-11-02, with the owner's leave to trust a
test certificate; scripts and logs in `C:\Users\ailabs\sr-harness\store-e1`; everything undone
after: packages removed, the test certificate taken out of the Trusted Root store and deleted
on the dev box). The release binaries of `5f22ae21`.

1. **Partner Center's identity cannot run sideloaded.** A package with `Monatis.okur` and its
   publisher, signed with a test certificate of that subject, installs, but Windows never
   starts it, with or without UI Access. Each launch re-registers the package ("Getting
   registration status ... statusFound: false"), and the activation API answers `0x80270254`
   (`E_APPLICATION_ACTIVATION_EXEC_FAILURE`) with no process created. The same package under a
   test identity (`Okur.LabTest`, the development certificate's subject) starts. So the real
   identity is tested only as the Store installs it; the experiments use a test identity.
2. **Installing a UI Access package needs an administrator** (R-store-10 holds): unelevated,
   `0x80073CF6`, "the system failed to register the windows.uiAccess extension due to the
   following error: Access is denied"; elevated, it installs.
3. **Windows checks the program's own signature, not the package's.** With only the package
   signed, the launch fails with `0x8007202B`, "A referral was returned from the server", as an
   unsigned UI Access program does unpackaged (E1 of 2026-10-20).
4. **With `okur-sr.exe` signed by a trusted certificate, packaged UI Access works.** Launched as
   the Start menu does, `okur-sr.exe --doctor-exit` reported:

   ```
   executable                           C:\Program Files\WindowsApps\Okur.LabTest_1.0.2.0_x64__85p8zkww4b41e\okur-sr.exe
   integrity                            High (0x3000)
   TokenUIAccess                        1
   reads elevated windows               yes
   signature                            valid
   manifest                             level=asInvoker uiAccess=true
   ```

   Started normally, with an elevated Notepad in front, its log said
   `spoken for event msg=e1-check.txt - Notepad | edit | Type in text.`
5. The package without UI Access, test identity, installs unelevated and starts at Medium
   integrity with `TokenUIAccess 0` (E2's start, nothing more checked).

What this leaves for the Store: whether the Store's own signature makes the program count as
signed (the Store signs the package; NVDA's Store programs carried NV Access's signature), or
whether `okur-sr.exe` needs a publicly trusted code-signing certificate of Okur's own
(§5.12: none affordable found for an individual in Turkey). Only a Store install can tell the
first: a Private-audience submission of the UI Access package, installed on old-pc from the
Store, then `okur-sr.exe --doctor-exit`. `build-msix.ps1 -ProgramCertificateThumbprint` signs
the programs before packing when a certificate exists, and warns when a UI Access package is
built without it.

**The Store install answers it (old-pc, 2026-11-03).** The owner installed the Private-audience
submission from the Store on old-pc (`Monatis.okur` 1.0.0.0,
`C:\Program Files\WindowsApps\Monatis.okur_1.0.0.0_x64__rfy6yt361pz56`), signed in to the
Store. Activated as the Start menu does
(`store-e1\activate.ps1 -App 'Monatis.okur_rfy6yt361pz56!Okur' -Arguments '--doctor-exit
<file>'`: `hr 0x00000000`), `okur-sr.exe` reported:

```
executable                           C:\Program Files\WindowsApps\Monatis.okur_1.0.0.0_x64__rfy6yt361pz56\okur-sr.exe
integrity                            High (0x3000)
TokenUIAccess                        1
elevated                             no (limited token)
reads elevated windows               yes
signature                            not signed
manifest                             level=asInvoker uiAccess=true
```

So the Store's signature of the package is enough: the program itself carries no signature,
and Windows started it with UI Access, which a sideloaded package signed the same way did not
get (result 3 above). Okur needs no code-signing certificate of its own for UI Access through
the Store. Not yet checked: Okur started normally from the Store reading an elevated window
(it needs a UAC consent on old-pc), and a standard user's install.

The `AT key` the report shows (`StartExe C:\Program Files\Okur\okur-sr.exe`) is left from an
earlier `okur-setup` install on old-pc, not the package's: a Store install writes no HKLM key,
so registering the Store's Okur for Ease of Access is still to design (§5, "The sign-in screen
and secure desktops: not from a package").

**Before the owner submits:**

1. Defender's `Bearfoos.A!ml` detection is resolved, or the built binaries are confirmed not
   flagged, because certification starts with a malware scan (R-store-24).
2. A privacy policy URL exists.
3. The first submission is **Private audience** (R-store-32).
4. The restricted-capability justification for `uiAccess` is written: Okur is a screen reader;
   UI Access lets a blind user read and operate elevated windows and UAC-elevated apps;
   R-store-03 names exactly this use.
5. The certification notes explain how to test with the keyboard and speech.

## 9. Open questions

1. Does packaged UI Access work today on Windows 10 19045 and Windows 11 26200 for a package that
   Microsoft did not make, and does the Store approve it for Okur? (E1; the submission.)
2. Does a Store install of a `uiAccess` package still show a UAC prompt, and can a standard user
   install it at all? (E1 variant on a standard account; R-store-10 is from 2019.)
3. Child-process package context (R-store-21 is ambiguous). (E4.)
4. The startup task's delay after sign-in. (E3.)
5. Whether Smart App Control and SmartScreen ever object to a Store-signed package. Not
   documented (R-store-39).
6. Whether an extension SDK can live in a Store build (policies 10.2.2 and 10.1.5).
7. The exact app execution alias manifest syntax (R-store-22's page is thin; look up
   `uap5:ExecutionAlias` or `desktop:ExecutionAlias` before writing the manifest).

## 10. What ran

All of it on the dev box, read-only, into the session scratchpad:

- `curl` of the Store catalog API for NVDA's product and for seven search queries;
- `curl` of NVDA's `appx/` files at `release-2026.2`, master's `sconstruct`, `config/__init__.py`
  and `changes.md`;
- `curl` of PR #20681's diff, PR #10464's description, the appx manifest's commit list, and
  WindowsAppSDK #1669's comments, through GitHub's public API;
- `curl` of crates.io's search for "msix" and of four repositories' metadata;
- a file listing of `C:\Program Files (x86)\Windows Kits\10`.

Nothing was installed, signed or executed. No probe script was written: the packaging questions
need old-pc (E1–E7), not a library probe.
