# The Store submissions: what goes on each Partner Center page

Product: Okur, Store ID 9NV0FM5H54X8, `Monatis.okur` published by Monatis.

## Submission 2: Okur 1.1.0.0, public (2026-11-03)

The owner, 2026-11-03: "after this round of fixes, we can make a new release on microsoft
store, this time public." Submission 1 (below) answered what it was for: certified, and the
Store-installed program runs with UI Access (the research's §8). Submission 2 is a new
submission of the same product; Partner Center starts it as a copy of submission 1, so only
the pages below change. Everything else stays as submission 1 has it.

### Packages

Remove 1.0.0.0's package and upload
`target/submission-1.1.0.0-9e9af731/okur-1.1.0.0-x64-uiaccess.msix`, unsigned (the Store signs
it), SHA-256 `d46537433ebb8bcc877335345e221b677faea8e717c48f19b3c673eeb997cf76`. Built from
`9e9af731` by `build-msix.ps1 -Version 1.1.0.0 -UiAccess`, with the same capabilities as
1.0.0.0 (`runFullTrust`, `uiAccess`) and the same minimum Windows (10.0.18990.0). Defender on
the dev box: "found no threats" (`MpCmdRun -Scan -ScanType 3`, the package's folder).

### Pricing and availability

- **Visibility: Public audience**, and discoverable in the Store ("Make this product available
  and discoverable in the Microsoft Store"). **This cannot be undone:** a product made public
  can never be made private again (research R-store-32). The private customer group then no
  longer applies.
- Markets: all. Pricing: free. As in submission 1.

### Store listing (English, United States)

"What's new in this version" (new; at most 1,500 characters):

```
Okur 1.1, the first public release.

- Windows Settings and the apps from the Microsoft Store are heard again: in 1.0 they were silent. In Settings, Insert+Tab names the group of the focused control and Insert+T the page.
- Windows' lists of programs: Apps in Settings, and Programs and Features in Control Panel, with their column titles, each program said once, and its state and place.
- File Explorer: its ribbon, from Alt through its tabs and commands; Insert+Tab on a file says it as moving to it does.
- A combo box you can type in, such as the Run dialog's, is said once, not twice; Say Window (Insert+B) reads it as its box, field and button.
- A window that opens with its focus already set, such as Programs and Features, is read when it comes to the front.
- Web pages: the virtual cursor's place is framed on screen and scrolled into view as it reads; a page still loading is read once it has loaded; "clickable" is said only in a table that answers clicks itself.
- Okur's own windows report their errors in Okur's log.
```

In the description, the line on Windows names the Store's apps and the lists of programs:

```
- Windows itself: menus, dialog boxes, the desktop, the taskbar, Alt+Tab, File Explorer and its ribbon, Windows Settings and its list of apps, Programs and Features, the Run dialog, the apps from the Microsoft Store, Windows Terminal, Notepad and notifications.
```

The rest of the description, the short description, keywords, screenshot and logos stay.

### Submission options

As in submission 1: published as soon as it passes certification, now to everyone; the
restricted capabilities' reasons and the certification notes unchanged.

### After certification

- The listing at `https://apps.microsoft.com/detail/9NV0FM5H54X8`: the README's install
  instructions point there.
- On old-pc, where 1.0.0.0 is installed from the Store: the Store updates it to 1.1.0.0.
  That is experiment E6: Okur, running, closes for the update and restarts by itself
  (`RegisterApplicationRestart`), and speech returns without the user doing anything; its log
  (`%LOCALAPPDATA%\Okur\okur.log`, the package's redirected copy) says the update's close.

## Submission 1: Okur 1.0.0.0, private audience (2026-11-02)

For the owner's Private-audience submission of Okur 1.0.0.0 with UI Access (decided
2026-11-02), whose purpose is to learn how Microsoft treats it: the malware scan, the review of
the `uiAccess` restricted capability, and whether the Store-signed program gets UI Access on
old-pc (the research's §8, "E1's results"). Each section below is one page of the submission,
in Partner Center's order; texts in blocks are ready to paste.

## Where the draft stands (2026-11-02)

Submission 1 is filled in as below and saved as a draft; nothing has been submitted. Partner
Center's own status per page: Properties, Packages (validated, with the restricted-capability
warning for `runFullTrust` and `uiAccess`), Store listings and Submission options complete.
Left for the owner:
- **Age ratings**: the questionnaire is answered (below) and its preview shows "All ages",
  ESRB Everyone, IARC 3+, PEGI 3, USK 0; saving it needs the box that agrees to IARC's Terms
  of Use and says the owner is of age, which is the owner's to tick.
- **Pricing and availability**: Private audience, free, all markets are set; the customer
  group (the Microsoft accounts that may install) is not, and the page stays incomplete until
  one is chosen. The install link for the group:
  `https://apps.microsoft.com/detail/restricted/9NV0FM5H54X8`.
- Push `PRIVACY.md` (the privacy policy URL points at `main` on GitHub), then submit.

## Packages

Upload `target/submission-1.0.0.0-5f22ae21/okur-1.0.0.0-x64-uiaccess.msix`, unsigned (the Store
signs it), SHA-256
`8c03bae1c19049ad11e1a2c48dc9f74c68b3f52c2ab9a2fa5b7ac33e6a39151b`. It declares `runFullTrust`
and `uiAccess`, and needs Windows 10 build 18990 or later. Built from `5f22ae21`.

Partner Center then lists the restricted capabilities on the Submission options page (below).

## Pricing and availability

- **Visibility: Private audience.** A Public submission can never be made private again
  (research R-store-32).
- The audience: a known user group with the Microsoft accounts that may install it. It needs
  the account the Microsoft Store on old-pc is signed in with, for the test after
  certification (the Store there has to be signed in with it: your step, as it takes the
  account's password).
- Markets: all. Pricing: free.

## Properties

- Category: **Productivity** (as NVDA's Store listing was).
- Privacy policy URL:

  ```
  https://github.com/monatis/okur/blob/main/PRIVACY.md
  ```

  Push first: `PRIVACY.md` was updated on 2026-11-02 for the reader's own log.
- Website:

  ```
  https://github.com/monatis/okur
  ```

- Support contact:

  ```
  https://github.com/monatis/okur/issues
  ```

- System requirements: keyboard required.
- Product declarations: leave "This product has been tested to meet accessibility guidelines"
  unticked for now. Its criteria are about Okur's own windows (Narrator, high contrast, high
  DPI and an inspection tool's errors fixed), not checked yet (research §5.6).

## Age ratings

The IARC questionnaire (version 10.3), as answered:
- App type: All Other App Types (not a game, not social or communication);
- ratings-relevant content downloaded in the package: No;
- users interact or exchange content: No;
- online content featured or promoted: No;
- promotion or sale of age-restricted products: No;
- shares precise location, digital purchases, cash rewards or transferable assets, a web
  browser or search engine, primarily news or education: No to each;
- ratings from a ratings board or physical media: No.

## Store listing (English, United States)

Product name: okur (the reserved name, as Partner Center spells it).

Description:

```
Okur is a free, open-source screen reader for Windows. It reads what is on the screen aloud and lets you work with the keyboard alone, with the keyboard commands long-time screen reader users already know.

What it reads:
- Web pages in Google Chrome, in browse mode: by character, word, line, sentence and paragraph; Say All; quick navigation keys for headings, links, form fields, tables, lists, regions and more; the lists of links, headings and form fields; forms mode for filling in fields.
- Documents in Microsoft Word, with their headings, lists, tables, comments and notes.
- Spreadsheets in Microsoft Excel: cells with their coordinates, row and column titles, totals and selections.
- Windows itself: menus, dialog boxes, the desktop, the taskbar, Alt+Tab, File Explorer, Windows Settings, Windows Terminal, Notepad and notifications.
- Any window's screen, pictures included, with Windows' own text recognition.
- Windows that run as administrator, such as an administrator Command Prompt or an installer.

It speaks with the voices installed in Windows, and its Settings Center keeps settings for all applications or for one application or web site: typing echo, punctuation, verbosity, voices and more.

This is an early release. Important limitations:
- Okur does not run on the Windows sign-in screen, the lock screen or the screens of User Account Control prompts: apps from the Microsoft Store cannot.
- Installing it asks for administrator permission once, so that Okur can read windows that run as administrator.
- English only for now. No braille yet. Microsoft Outlook is not supported yet.

Privacy: Okur collects nothing and makes no network connections. What it reads stays on your computer.
```

Short description (optional, shown at the top of the listing):

```
Okur is a free, open-source screen reader for Windows. It reads what is on the screen aloud and lets you work with the keyboard alone.
```

Keywords (at most seven, none another publisher's product name, policy 10.1.3):

```
screen reader
blind
accessibility
visually impaired
text to speech
keyboard navigation
assistive technology
```

Copyright and trademark info:

```
Copyright 2026 M. Yusuf Sarıgöz. Licensed under the Apache License 2.0.
```

Screenshots: at least one is required.
`target/submission-1.0.0.0-5f22ae21/okur-settings-center-1366x768.png`: Okur's Settings
Center as the release `okur-ui.exe` draws it, fed the dialog Okur's own code builds (the reader
itself not running), captured as a window and centered unscaled on a plain 1366x768 background.
Its caption:

```
Okur's Settings Center: a search box, the groups of settings, and what the selected one does. Settings apply to all applications or to one application or web site.
```

Logos: the package carries its own; the 1:1 box art is `target/msix/listing/okur-logo-1080.png`.
"What's new", product features, "Developed by" and additional license terms are left empty.

## Submission options

Publishing hold: the default, published as soon as it passes certification (to the private
audience only).

Restricted capabilities, why Okur needs each (each box takes at most 500 characters):

`runFullTrust`:

```
Okur is a screen reader for blind and visually impaired people. It reads other applications' windows through Windows' accessibility interfaces (UI Automation, Microsoft Active Accessibility and IAccessible2), takes its keyboard commands through a low-level keyboard hook, and speaks with the voices installed in Windows. That needs a full-trust process running as the signed-in user. Okur loads nothing into other processes, makes no network connections, and needs no administrator rights to run.
```

`uiAccess`:

```
Okur is an assistive technology: a screen reader. UI Access lets it read and operate windows that run as administrator, such as an administrator Command Prompt, installers and Registry Editor, which a blind user cannot use at all without it. This is the use Microsoft's "Security Considerations for Assistive Technologies" describes for UIAccess. Okur's program requests it in its manifest (level asInvoker, uiAccess true); it never elevates, and runs as the signed-in user.
```

Notes for certification (now on the product's "Additional Testing Information" page, under
Supplemental info, not on Submission options):

```
Okur is a screen reader: it works by speech and the keyboard, and has no main window. Please test with speakers or headphones on.

1. Start Okur from the Start menu. It starts speaking at once; the first time, its Startup Wizard opens (a window with speech settings, read aloud as you move in it).
2. With Okur running: Insert+J opens Okur's window, Insert+6 its Settings Center, Insert+H lists the keys for the window in front. Okur reads each control as the focus moves (Tab, arrow keys).
3. Open a web page in Google Chrome: Okur reads it. The Down Arrow reads the next line, H moves to the next heading, Insert+Down reads from there to the end.
4. To quit Okur, press Insert+F4 and then Enter.

Okur starts after sign-in through its startup task, which the user can turn off in Settings, Apps, Startup. It changes no Windows settings on its own: while it runs it sets Windows' screen reader flag (SPI_SETSCREENREADER), as screen readers do, and clears it when it quits. Installing asks for administrator permission because of the uiAccess capability.
```

## After certification

When the submission passes, the Store gives the private audience an install link. On old-pc,
with the Store signed in with an account in the group: install Okur from the Store, start it,
and run its doctor report through the activation API
(`C:\Users\ailabs\sr-harness\store-e1\activate.ps1 -Arguments '--doctor-exit <file>'`). The
report's `TokenUIAccess` and integrity answer the question: 1 and High, the Store's signature
counts; otherwise Windows refused the program as unsigned (`0x8007202B`), and UI Access needs a
code-signing certificate of Okur's own. Then E3 to E6 with the Store-installed package (the
startup task at sign-in, files, an update with a second version).

If certification fails, its report says at which step: the malware scan, the Windows App
Certification Kit, the restricted capabilities, or the content review.
