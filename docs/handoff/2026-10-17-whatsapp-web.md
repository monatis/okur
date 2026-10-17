# Handoff: WhatsApp Web as a web application (2026-10-17, afternoon)

After [`2026-10-17-okur-ui-viewer-and-settings.md`](2026-10-17-okur-ui-viewer-and-settings.md).
The user put WhatsApp Web first, before the remaining okur-ui dialogs: "it's one of the few
blockers for me to use okur daily". They allowed reading their messages while working, and
sending messages only to their own chat ("message to self"); its context menus, dialogs and
file previews could be tried there too. No WhatsApp-specific commands for now: it is a web
application with complex controls, and an extension can come after the SDK.

Their workflow is the specification:
- `E` to the search box (the only edit field with no chat open), `Enter`, type a few
  letters, `Down` through the results, `Enter` on the chat;
- `Ctrl+End`, `Shift+E` to the message field, `Enter`, type, `Enter` to send;
- each message has an options button; JAWS leaves the focus outside the menu it opens until
  `Tab` or `Shift+Tab`; then the item and `Enter`;
- the chat list and a chat's messages behave like list boxes: focus in them, then the arrows.

Everything below was observed on the dev box with the user's JAWS settings: research, not
evidence. Okur ran beside JAWS (`--engine no-speech --jaws-key scrolllock --settings-file`
from the scratchpad); JAWS's speech came from `okur jaws-history`.

## Commits (on `main`, not pushed)

- `077a879` feat(web): WhatsApp Web as a web application: changed pages, focusable items,
  popup menus (and Pass Key Through).
- `285e6b0` fix(web): the virtual cursor goes to a focus whose lines are its children's.
- This handoff, the reference page [`web-applications.md`](../references/web-applications.md),
  pending scenario 219 and its fixture, and the next plan: a `docs:` commit.

## What was wrong, and what Okur does now

1. **The buffer never saw content added later.** Okur hooked no "children changed" event, so
   a chat's messages, rendered after it opened, were not in the buffer. Okur now hooks
   Chrome's show, hide and reorder WinEvents on the page in front (no cross-process read in the
   hook, at most one every 200 ms) and sends `PageChanged`. Browse mode re-reads once the
   changes settle; forms mode waits until it ends, then re-reads at once. Idle WhatsApp sent
   none in 15 s; typing in the message field sends some with each key, held until forms mode
   ends.
2. **A new title reloaded the page as a new one.** WhatsApp's title counts unread chats
   ("(7) WhatsApp", "(8) WhatsApp"): the cursor jumped to the top and the page was announced.
   A page is now known by its address (engine and platform).
3. **"group start Siz:", "group end Siz test 14:13 Okundu".** Chromium reports every `div` and
   `span` as an MSAA grouping; only IA2's role (`IA2_ROLE_SECTION`) tells a generic one from a
   fieldset. Named generics are no longer groups, and an empty labelled `span` reads its label,
   as JAWS reads "Siz:".
4. **Tab from the virtual cursor went to buttons nothing focuses** (a message's time) and
   skipped the messages, which are plain focusable divs. The AOM now has `focusable` (IA2,
   UIA, xa11y), the buffer has focus stops for focusable non-controls, and Tab from the
   virtual cursor uses both, skipping controls that cannot take the focus.
5. **A renamed focus was lost.** WhatsApp names a message differently once it has the focus;
   matching by name failed and the cursor read the message field instead. Focus events now
   carry the IA2 uniqueID from the WinEvent's child id, page re-reads keep the cursor on the
   same object, and an element whose lines are its children's is entered at its first line.
6. **Lists.** Tab onto a list, grid, tree or table item turns forms mode on (the page's
   arrows); forms mode also persists from a field into such items (the search results), and
   Enter on an item that sends the focus elsewhere (a chat opening) returns to browse mode.
7. **Menus.** A popup page menu (no menu bar) says each item's position, counted among its
   menu items when the page gives none, and no "menu"; a focus on a non-item inside the menu
   (the reactions) keeps menu mode, with no usage hint, and the item the focus returns to is
   said again. Toggle buttons say "Toggle button" and "Pressed"/"Not pressed".
8. **Smaller.** A field holding only a line break is blank (WhatsApp's empty message field
   said "Contains text"); a control the buffer lacks is said from its event as the buffer
   would; Pass Key Through (`Insert+3`, "Enter key to pass through"), which Okur lacked.

## Live comparisons (dev box, own chat)

Okur's speech log against JAWS's history, same keys, same state. "=" means identical text.

| Step | Okur | JAWS |
|---|---|---|
| Down into the search results | `+90 … (Siz) 14:46 okur 1` | the same after `Arama sonuçları. table / Column 0, Row 0 / … grid with 1 columns and 21 rows` |
| Enter on the own chat | `ContentInfoRegion / … sohbetine bir mesaj yazın edit / blank` (then browse mode) | `ContentInfoRegion / … edit` (JAWS stayed in forms mode) |
| Up through a chat from `Ctrl+End` | `14:46 Okundu Button / okur 1 / Siz: / 14:13 Okundu Button / test / 13:57 Okundu Button / E / …` | = (message lines); JAWS shows the message field as `Editable Section End / / Editable Section Start / …` |
| `Shift+Tab` from the `E` line | `Siz hello 13:14 Okundu` | = |
| `Down`, `Up` on messages (both in forms mode) | `Siz okur 1 14:46 Okundu Daha fazla seçenek için sol veya sağ ok tuşuna basarak bağlam menüsüne erişin`, then `Siz test …` | = |
| `Right` on a message | `Mesaj seçeneklerini aç Button collapsed` | `… Button collapsed / To activate press Enter.` (the hint is now added) |
| Menu items | `Kopyala / 3 of 9`, `Cevapla / 2 of 9` | = |
| `Shift+Tab` to a reaction | `👍 Toggle button Not pressed` | `(7) WhatsApp - Google Chrome dialog / 👍 Toggle button Not pressed` |
| `Tab` back to the items | `Cevapla / 2 of 9` | = |
| `Enter` on Cevapla (reply) | `Leaving menus / ContentInfoRegion / … edit / blank` | `Leaving menus / ContentInfoRegion / … edit / / Type in text.` |
| The PDF preview (Enter on the document) | the viewer's content, then its text line by line in browse mode | JAWS read the viewer's header |

## Running both readers at once

JAWS sees each key after Okur. When Okur passes a key on in forms mode while JAWS is in
browse mode, JAWS takes it for its virtual cursor and the page never gets it: Okur's arrows
on WhatsApp's messages did nothing until JAWS was in forms mode too. This also explains the
open question in the previous handoff about `web-menus.html`. To test, put JAWS in forms mode
first: Okur's `E` and `Enter` on the message field, then a second `Enter` that Okur passes and
JAWS takes; JAWS's forms mode survives the focus moving to a message. Only the user's real
setup (Okur alone) or old-pc can show the keys as they are.

## Left in the user's WhatsApp

- Messages sent to the own chat: `E` (13:57, by accident during a JAWS reference run: JAWS
  stayed in forms mode after the chat opened, so `Shift+E` typed and `Enter` sent), `test`
  (14:13) and `okur 1` (14:46), both on purpose. Nothing was sent anywhere else.
- JAWS's `Up` in the empty message field opened WhatsApp's "edit last message" dialog once;
  it was closed with Escape, unchanged. A reply was opened from the menu and cancelled.
- The PDF preview was opened and closed; Downloads did not change. The
  `WhatsApp Image 2026-10-17 at 13.33.47.jpeg` in Downloads (13:40) is not from these tests:
  no keys were sent between 13:30 and 13:56.
- The chat list's search box was left with the search typed into it.
- Snapshots and speech logs with personal content stay in the session scratchpad, never the
  repository; test strings use invented numbers.

## Verification

- `skills/jaws-reference/scripts/parity-snapshot.sh` before (at `a047048`) and after each
  change, compared with `diff -r`: identical. Last run, at `285e6b0`:
  ```
  JAWS parity (Ia2): 819/844 steps match exactly (0 scenarios could not run)
  JAWS parity (Uia): 720/844 steps match exactly (0 scenarios could not run)
  JAWS parity (Ia2): 819/844 steps match exactly (0 scenarios could not run)
  ```
- `cargo test --workspace` at `285e6b0`, the crates with tests (new: 12 web-application tests
  in `engine/web_app_tests.rs`, 3 IA2 mapping tests, the linearizer's focus stops):
  ```
  okur_core              197 passed, 0 failed, 0 ignored
  okur_corpus             14 passed, 0 failed, 0 ignored
  okur_platform           51 passed, 0 failed, 3 ignored
  okur_speech_recorder     3 passed, 0 failed, 0 ignored
  okur_ui                  2 passed, 0 failed, 0 ignored
  okur_ui_model            3 passed, 0 failed, 0 ignored
  xa11y                  299 passed, 0 failed, 0 ignored
  xa11y_core             294 passed, 0 failed, 0 ignored
  xa11y_windows           69 passed, 0 failed, 16 ignored
  ```
  `cargo clippy --workspace --all-targets --all-features -- -D warnings -D
  clippy::indexing_slicing -D clippy::string_slice` and `cargo fmt --check`: no output.

## Differences left (known)

- JAWS names the menu's dialog when the focus moves to the reactions ("(7) WhatsApp - Google
  Chrome dialog"); Okur says nothing for entering a dialog it cannot see in its buffer yet.
- The message field in browse mode: JAWS's `Editable Section Start`, its lines and
  `Editable Section End`; Okur's single edit line.
- JAWS's table introduction on entering the search results (`Arama sonuçları. table / Column
  0, Row 0 / … grid with 1 columns and 21 rows`); Okur says the result alone.
- "Type in text." when the page itself moves the focus into a field after a menu (JAWS); Okur
  says "blank" and is silently in forms mode.
- An element with role `application` that itself takes the focus (the PDF viewer's content)
  does not turn the application keys on; only a focus inside one does. JAWS's behaviour there
  is unknown.
- After a page re-read the speech of the first focus can repeat the page title when Okur had
  lost track of the focus (`(7) WhatsApp / PDF content`).
- Okur reads a whole page through IA2 for each settled change (about 2 s on WhatsApp). A
  subtree re-read (the changed object is known from the WinEvent) would be cheaper.

## Next

See [`docs/plans/2026-10-17-web-apps-round-3.md`](../plans/2026-10-17-web-apps-round-3.md):
record scenario 219 on old-pc, the differences above, then back to the okur-ui dialogs
(headings and form fields lists).
