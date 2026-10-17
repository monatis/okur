# Plan: Web applications, round 3 (after WhatsApp Web) (2026-10-17)

Follows [`docs/handoff/2026-10-17-whatsapp-web.md`](../handoff/2026-10-17-whatsapp-web.md).
WhatsApp Web now works for the user's daily flow (search, open a chat, read it, write and
send, a message's menu, a PDF preview). This plan finishes what that session left, then goes
back to the okur-ui dialogs of [`2026-10-17-helper-dialogs.md`](2026-10-17-helper-dialogs.md).

## Rules for this round

- WhatsApp: the user's own chat only ("+90 … (Siz)", message to self) for anything that sends
  or changes; never another conversation. No WhatsApp content in the repository: test strings
  are invented, and snapshots and speech logs stay in the session scratchpad. Never quote a
  document's or message's content in a report beyond what the user's own test messages say.
- Okur beside JAWS on the dev box: keys Okur passes on in forms mode are taken by JAWS when
  JAWS is in browse mode (handoff, "Running both readers at once"). Put JAWS in forms mode
  first, or trust only old-pc.
- Every behaviour change: parity snapshot before and after (`diff -r` identical, or each
  difference explained), tests in `engine/web_app_tests.rs`.

## Steps

### 1. Record scenario 219 on old-pc

`corpus/pending/scenarios/219-web-chat-patterns.json` with `web-chat.html` (both in
`corpus/pending/`): toggle buttons, labelled divs, Shift+Tab onto a message, Insert+3, the
options button, a popup menu with reactions. Move them into the corpus, record, import, and
compare with the dev-box notes in `corpus/pending/README.md`. Then:
- if JAWS gives the toggle hint inside a menu, or names popup items without positions on
  old-pc, follow old-pc;
- keep the one deliberate difference (Tab onto an item turns forms mode on) as a known gap
  test, not an assertion of JAWS's behaviour.

### 2. The differences left

- **Entering a dialog the buffer does not hold yet**: JAWS names it, by its window when it has
  no name ("(7) WhatsApp - Google Chrome dialog"). The focus chain has the dialog: give the
  focus event its nearest dialog (role, name), and say it when the focus enters it.
- **Rich text fields in browse mode**: JAWS's `Editable Section Start`, the field's lines,
  `Editable Section End` for a `contenteditable` (WhatsApp's message field). Check JAWS on a
  plain page first (old-pc), then build it in the linearizer.
- **A grid's introduction**: entering WhatsApp's search results JAWS said the table's name,
  "table", the position, and "grid with 1 columns and 21 rows". Find JAWS's rule on old-pc
  (a fixture grid), then add it.
- **"Type in text."** when the page itself moves the focus into a field after a menu (JAWS,
  replying from a message's menu).
- **An application region that takes the focus itself** (the PDF viewer's content): what JAWS
  does there decides whether its keys go to the page.

### 3. Cheaper re-reads

Each settled page change re-reads the whole page through IA2 (about 2 s on a WhatsApp chat).
The WinEvent names the changed object (its uniqueID): capture that subtree
(`capture_chromium_object`) and splice it into the page by `UNIQUE_ID_ATTR`, falling back to
the whole page when the object is not found. Measure with `okur latency` before and after, on
WhatsApp and on GitHub.

### 4. With the user, WhatsApp beyond the daily flow

Images and the image viewer, voice messages, the attach menu ("Ekle"), the emoji picker,
starring and searching in a chat, and what arrives while reading (JAWS announces nothing by
default). Only in the own chat; ask before anything that downloads.

### 5. Then the dialogs

Back to [`2026-10-17-helper-dialogs.md`](2026-10-17-helper-dialogs.md): the headings and form
fields lists as okur-ui dialogs, then the rest of that plan.
