# Web applications in Okur

Web applications such as WhatsApp Web build their pages as they are used: a chat's messages
appear after the chat opens, a search box's results as it is typed in, a message's options
button only once the message has the focus. This page says how Okur keeps up with such pages
and which keys do what in them. WhatsApp Web (Turkish interface, Chrome, dev box,
2026-10-17) is the worked example; nothing here is specific to it in code.

## The page stays current

- **Changes anywhere on the page.** Chrome reports objects shown, hidden and reordered in the
  page in front. Okur notes them in its keyboard-free event hook, without reading anything
  across processes, at most every 200 ms, and tells the engine the page changed.
  - In browse mode the page is read again once the changes settle (0.7 s of quiet, at most
    3 s), in full through IA2: about 2 s on a WhatsApp chat.
  - In forms mode (typing in the message field changes the page with each key) the page is
    read again only when forms mode ends, at once: Escape after sending a message, and the
    new message is there.
- **A focus the buffer does not hold** (a search result, an options button, a viewer the
  page opened) makes Okur read the page quickly through UIA (about 0.15 s), put the virtual
  cursor on the focus, and then read it in full through IA2.
- **The same page** is known by its address. WhatsApp's title counts unread chats
  ("(8) WhatsApp"); a new title is not a new page, and nothing is announced.
- **The cursor keeps its place** across a re-read on the same object (Chrome's IA2 uniqueID):
  WhatsApp repeats lines ("Siz:") and loads older messages above as a chat is read up.

## Keys

| Where | Key | What happens |
|---|---|---|
| Browse mode | `E`, `Shift+E` | Next or previous edit field: the search box, the message field. |
| On a field | `Enter` | Forms mode: "Type in text." Typing goes to the field. |
| Search box, forms mode | `Down`, `Up` | Into and through the results, each read whole ("+90 … (Siz) 14:46 okur 1"); forms mode stays on. |
| On a result | `Enter` | Opens the chat. The page puts the focus in its message field; Okur reads it and returns to browse mode, so `Ctrl+End`, `Shift+E`, `Enter` reach the message field as usual. |
| Browse mode, on a message's text | `Shift+Tab`, `Tab` | The focus moves to the nearest element that can take it: the message before or after, past the time buttons nothing focuses. |
| Tab onto a list, grid, tree or table item (a message, a chat) | | Forms mode turns on: the page's arrows move through its items, each said by the name the page gives it ("Siz test 14:13 Okundu Daha fazla seçenek …"). JAWS 2025 stays in browse mode here; Okur behaves like a list box. `Escape` leaves forms mode. |
| On a message, forms mode | `Right` | WhatsApp's "Mesaj seçeneklerini aç Button collapsed". `Enter` opens its menu. |
| In a popup menu | `Up`, `Down` | Items with their positions: "Mesaj bilgisi, 1 of 9, To move through items press up or down arrow." first, then "Cevapla, 2 of 9". |
| In a popup menu | `Tab`, `Shift+Tab` | Between the reactions ("👍 Toggle button Not pressed") and the items. Still in the menu: no "Leaving menus". |
| Anywhere | `Insert+3` | "Enter key to pass through": the next key goes to the application as it is. |

## What JAWS says differently, and why

- JAWS stays in browse mode when Tab or Shift+Tab lands on a message, and the user has to
  pass keys to the page. Okur gives the page its arrows there (the user's request, 2026-10-17).
- JAWS names an unnamed dialog by its window ("(7) WhatsApp - Google Chrome dialog") when a
  popup menu's focus leaves its items for the reactions; Okur does not yet.
- JAWS shows a rich-text field in browse mode as "Editable Section Start", its lines,
  "Editable Section End"; Okur shows one edit line.

The handoff `docs/handoff/2026-10-17-whatsapp-web.md` has the session's live comparisons, and
the pending scenario 219 (`corpus/pending/`) records these patterns from JAWS on `old-pc`.
