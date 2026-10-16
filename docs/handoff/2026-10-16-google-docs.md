# Handoff: Google Docs (2026-10-16, evening)

Follows [`2026-10-16-responsiveness-and-web-apps.md`](2026-10-16-responsiveness-and-web-apps.md).
The user asked for work on Google Docs, in a new document in their signed-in Chrome:
JAWS is the reference, except where JAWS falls short in Docs. Two such places were named:
- JAWS's Insert+F gives no font information (no "bold");
- Backspace says the character to the right of the caret along with the deleted one.

Everything here was observed on the dev box with the user's own JAWS configuration:
research, not evidence. old-pc was offline (`opendesk exec --peer old-pc`:
`rendezvous error: target_offline`).

## How Docs works for a screen reader (observed)

- Docs draws the document on a canvas. Its text is not in the accessibility tree (the IA2
  capture of the page has the toolbar, menus and live regions only).
- Keys go to an empty element Docs focuses for typing: DOM id `docs-texteventtarget-descendant`,
  a group inside an ARIA `application` region. Okur passes every key there.
- What the caret passes over reaches screen readers as Docs' own announcements, in the live
  region `docs-aria-speakable` (`aria-live=assertive`, `aria-atomic=true`):
  - Right says the character passed over, and Left back over it says it again (the same text,
    the region filled again);
  - Backspace adds the deleted character to what the region said last ("e", then "e r"), and
    JAWS reads the region whole. That is the "character to the right" the user hears: it is
    Docs' previous announcement.
- The toolbar shows the formatting at the caret:
  - the Bold, Italic and Underline buttons are pressed (UIA toggle state);
  - the size field holds the size;
  - the font list's selected option is named "Font list. Arial selected.";
  - DOM ids: `boldButton`, `italicButton`, `underlineButton`, `fontSizeSelect`,
    `docs-font-family`.
- Docs' own "Verbalize selection formatting" (Accessibility menu, Ctrl+Alt+A then Ctrl+Alt+F)
  says "Normal text | font Arial | 11 point | bold | color black | vertical align normal". On
  the dev box's Turkish keyboard, Ctrl+Alt+A is AltGr+A: it typed "æ", and Ctrl+Alt+F then
  inserted a footnote (both undone). Docs' Ctrl+Alt shortcuts do not work on such layouts.

## What changed (commits on `main`, not pushed)

- `2a66579` feat(web): Google Docs says its font, and its announcements as Docs means them.
  - `crates/okur-core/src/apps/web/google_docs.rs`: the first web application module, known by
    the page's address (`AppModule::owns_page`). Say Font reads the toolbar as it is when the
    key is pressed:
    - in "with": "Font is Arial 11 point";
    - in "bold": "Font is bold  Arial 11 point";
    - pressed twice: the Virtual Viewer, as elsewhere.
    The font family comes from the sentence Docs names the selection with, using the words the
    size list puts around its size, so it does not depend on the document's language.
  - `ApplicationActions::read_page_elements` and `AccessibilityEvent::PageElements`: page
    elements by DOM id with their subtrees, one cached UIA search. Live: "page elements read
    asked=5 found=5 ms=17" to "ms=26" in the debug log.
  - Live regions keep `aria-live` and `aria-atomic` (the platform said "polite" for every
    region before):
    - an assertive region interrupts speech;
    - an atomic region filled again with the same text says it again (Right, Left, Right over
      "w": "w" three times; before, the repeat was silent);
    - what a change adds is read alone, so Backspace says only the deleted character ("r",
      then "e"), as Notepad's does. This is better than JAWS, which says "e r".
  - A page refresh finds the focus again by its DOM id: Docs' menu bar keeps an IA2 focused
    state after a menu closes, and the first focused node was taken for the focus, which
    stopped Insert+F after the first refresh.
- `cd327cc` fix(web): a page's menus are left with "Leaving menus", each item said once.
- `10c4e16` fix(web): a page's menu takes its keys, and opens on its first item.
  - Chrome reports a page's menu item twice; both reports are now one menu item.
  - A popup that opens before any item has focus (Docs' Alt+Shift+A) introduces its first
    item.
  - The focus back in the page says "Leaving menus", as JAWS did.
  - A frame's document the buffer does not hold keeps the page focus.
  - A focus with nothing of it in the buffer is said by its name, not by the cursor's line,
    which was the page title.
  - While a page's menu is open every key is the page's: Escape closes the menu instead of
    leaving forms mode, and Down is not read as the field the menu was opened from.
  - Docs now (Okur): Alt+Shift+A "menu | Accessibility | To move through items press up or
    down arrow.", Down "Verbalize to screen reader s ►", Escape "Leaving menus", "Document
    region.".
  - JAWS (dev box): "menu | Accessibility | 10 of 10 | To move through items press left or
    right arrow.", then only the item, then "Leaving menus".
- `af565f1` corpus: pending scenario 218 (`corpus/pending/fixtures/web-menus.html`), an ARIA menu
  bar, to settle JAWS's wording for page menus on old-pc.

## Verification

- `cargo test --workspace`: 926 passed, 0 failed, 198 ignored (summed from the `test result`
  lines).
- Clippy with the gate's flags: clean. The commit gate passed for each commit.
- `skills/jaws-reference/scripts/parity-snapshot.sh`, then `diff -r` against the saved `5cc13e1`
  reference: identical after each commit.
  ```
  JAWS parity (Ia2): 818/844 steps match exactly (0 scenarios could not run)
  JAWS parity (Uia): 719/844 steps match exactly (0 scenarios could not run)
  JAWS spoken parity: 171/172 steps match exactly (evidence recorded through the speech recorder voice)
  ```
- New tests fail without their fix (checked by disabling each fix): the refresh's focus, the
  empty focus, and the menu keys.

## Incidents

- **JAWS stopped responding at about 21:06** (Windows: `Responding=False`, no CPU use). It
  happened during the first live Insert+F with Okur running next to it, and JAWS was still
  hung at 21:50. The cause is unknown; it was not restarted (the user's JAWS is theirs to
  restart). Live checks after that ran with Okur alone.
- With JAWS hung, a separate Chrome instance on the fixture stopped receiving arrow keys at
  times while Escape still arrived. The menu fixture's live check is therefore partial; Docs'
  menus were checked live.

## Gaps found (in the plan)

- **Tab skips focusable custom widgets.** Okur moves Tab from the virtual cursor to the next
  tab stop it knows: links, buttons, fields. An ARIA menu bar item with `tabindex=0` is not
  one, so Tab went past it to the Notes field. Accessibility APIs do not tell the Tab order
  (`tabindex=-1` elements are focusable too). The fix is JAWS's way: start the browser's own
  Tab from the virtual cursor.
- **Insert+Up (Say Line) in Docs says nothing.** The line is not in the tree. Docs' braille
  support (a Docs setting) puts the text in the tree; changing it needs the user's say.
- **Insert+Tab on a page focus:** Okur has no page Say Focus ("not read this way yet" in
  `engine/commands.rs`). On Docs' unnamed text target it now says nothing, where it said the
  title by accident.
- JAWS's position ("10 of 10") and left/right tutor for a menu bar item: Okur's menu items carry
  no position or orientation yet. Scenario 218 decides the wording.

## Left in the user's account

- The test document "Untitled document" in the user's Google Docs, with four test lines (one
  bold word). It can be deleted, or kept for the next Docs session.
