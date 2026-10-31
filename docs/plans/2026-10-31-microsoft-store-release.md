# Plan: the first release, through the Microsoft Store (scheduled after ARIA-AT)

Asked for by the owner on 2026-10-31: after the ARIA-AT work
([`2026-10-31-aria-at-web-round-1.md`](2026-10-31-aria-at-web-round-1.md)), make Okur's first
release in the Microsoft Store. Backlog row 45 of
[`../backlog/everyday-use-gaps-2026-10-19.md`](../backlog/everyday-use-gaps-2026-10-19.md).

## What the owner has done

- Signed up in Partner Center as an individual developer and reserved the name **okur**.
- The reason for the Store: an MSIX package submitted there is signed by Microsoft for free.
  That ends the signing problem of
  [`../references/startup-and-secure-screens-proposal.md`](../references/startup-and-secure-screens-proposal.md)
  §5.12 (no affordable certificate for an individual in Turkey), and a Store-signed package
  should end SmartScreen's and Defender's distrust of an unknown, unsigned `okur.exe` (old-pc's
  Defender has quarantined it as `Bearfoos.A!ml`).

## Questions to answer first (research, with sources)

1. **UI Access in a package.** Okur needs `uiAccess="true"` to read and drive elevated windows
   (proposal §2.6). Whether an MSIX can carry it: the restricted capabilities' list (`rescap`),
   what Partner Center asks to approve one, and what a full-trust packaged process
   (`runFullTrust`) may and may not do (low-level keyboard hook, `SetWinEventHook`, injecting
   nothing).
2. **NVDA's Store version** as the precedent: its `appx` build in the NVDA repository and its
   documented limits against the installed NVDA. What NVDA gives up there, Okur probably does
   too.
3. **Starting at sign-in** with the package's `windows.startupTask`; **the sign-in screen and
   secure desktops**, which need HKLM's assistive-technology registration (proposal §2.2), which
   a package cannot write: so probably not in the Store build, and said so in its description.
4. **The package's processes**: `okur-sr.exe`, `okur-ui.exe` and the settings; where settings
   and logs live (the package's own folders); speech through SAPI 5 and OneCore voices.
5. **Identity and updates**: the package identity and publisher from Partner Center in
   `AppxManifest.xml`; the version scheme; Store updates; what `okur-setup.exe` keeps doing (a
   development and lab route, and later the secure-screen route).
6. **The listing**: Okur's own words (AGENTS.md §1.5: JAWS named only to compare, and whether a
   Store listing may compare at all is the owner's to decide), the privacy policy URL, the age
   rating, screenshots, the accessibility declaration.

## Steps

1. Research the questions above into `docs/references/microsoft-store-release-research.md`.
2. Build the package from CI or a script (`MakeAppx.exe` from the Windows SDK, or a maintained
   Rust packaging tool), with a test certificate for local installs.
3. Run the Windows App Certification Kit on it.
4. Install it on old-pc with the test certificate, which trusting is the owner's to approve
   (a system change), and check there: Okur starts, speaks, reads Chrome, Word and Excel,
   starts at sign-in, quits; what it cannot do without UI Access, written down.
5. The release's text: README, the help, the "Not yet" list as the release's known limits.
6. The owner submits in Partner Center (their account, their decision); then the Store's
   certification report, and fixes.
