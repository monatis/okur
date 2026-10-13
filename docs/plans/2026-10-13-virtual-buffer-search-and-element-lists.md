# Milestone 11: Virtual Buffer In-Page Search & Element Lists (Links, Headings, Form Fields)

**Milestone Target**: Milestone 11  
**Prerequisites**: Milestone 10 ([`docs/handoff/2026-10-13-multi-page-spa-routing-and-iframes.md`](file:///c:/Users/Gigabyte/screen-reader/docs/handoff/2026-10-13-multi-page-spa-routing-and-iframes.md))  
**Target Parity**: Freedom Scientific JAWS 2025  

---

## 1. Objective & Scope

In real-world web browsing with Freedom Scientific JAWS, power users heavily rely on:
1. **In-Buffer Text Search (`Ctrl+F`, `F3`, `Shift+F3`)**:
   - Searching the linearized virtual buffer for arbitrary case-insensitive text strings.
   - Advancing forward to the next match via `F3` (`FindNext`) and backward via `Shift+F3` (`FindPrior`).
   - Boundary wrapping and negative search announcements (`"Text not found: {query}"`).
2. **Element List Generation & Target Navigation**:
   - **Links List (`JAWSKey + F7` / `Insert + F7`)**: Generates an ordered list of all links on the page, with anchor names, URLs, and same-page indicators. Selecting an item and pressing `Enter` moves the virtual cursor directly to that link in the virtual buffer.
   - **Headings List (`JAWSKey + F6` / `Insert + F6`)**: Generates an ordered hierarchical list of all headings on the page with level depth prefix. Selecting an item jumps virtual cursor to that heading.
   - **Form Fields List (`JAWSKey + Ctrl + F` / `Insert + Ctrl + F`)**: Generates an ordered list of all interactive inputs, buttons, checkboxes, comboboxes, and textareas.

---

## 2. Engineering Architecture

### A. Core Engine (`sr-core`)
1. **Search State (`VirtualCursorSearch`)**:
   - Maintain active query string, search case sensitivity flag, and list of matching character ranges in `VirtualCursor`.
   - `find_text(query: &str, case_sensitive: bool) -> Option<String>`: Finds first occurrence starting from current cursor offset.
   - `find_next() -> Option<String>`: Advances to next match offset; wraps around buffer or reports `"Search wrapped to top"`.
   - `find_prior() -> Option<String>`: Retreats to prior match offset.
   - Emits speech announcement of the destination line containing the matched text.

2. **Element Extraction Queries (`VirtualBuffer`)**:
   - `get_links_list(&self) -> Vec<ElementListItem>`: Returns all links with text, href, and buffer offset.
   - `get_headings_list(&self) -> Vec<HeadingListItem>`: Returns all headings with text, level, and buffer offset.
   - `get_form_fields_list(&self) -> Vec<FormFieldListItem>`: Returns all form fields with name, role, value, and buffer offset.
   - `jump_to_element_offset(&mut self, offset: usize) -> String`: Repositions virtual cursor to specified element and speaks the landing line.

3. **Key Mappings & Command Dispatches (`input.rs`, `engine.rs`)**:
   - Map `^f` / `Ctrl+F` to search query initiation.
   - Map `{F3}` to `FindNext`.
   - Map `+{F3}` / `Shift+F3` to `FindPrior`.
   - Map `LinkList` (`JAWSKey+F7`), `HeadingList` (`JAWSKey+F6`), `FormFieldList` (`JAWSKey+Ctrl+F`).

---

## 3. Scenarios to Author (51 through 55)

1. **`51-find-text-forward`**:
   - Search for a specific word `"destination"` using `Ctrl+F`. Verify cursor jumps forward to matching line and speaks line content.
2. **`52-find-next-and-wrap`**:
   - Execute `F3` (`FindNext`) to advance to second match, then trigger wrap around with speech confirmation.
3. **`53-find-text-not-found`**:
   - Search for non-existent string `"nonexistent-term"`. Verify speech output `"Text not found: nonexistent-term"` and unchanged cursor position.
4. **`54-headings-list-jump`**:
   - Query headings list (`JAWSKey+F6`), select target heading at level 2, jump virtual cursor, and verify landing speech.
5. **`55-links-list-jump`**:
   - Query links list (`JAWSKey+F7`), select target destination link, jump virtual cursor, and verify arrival speech.

---

## 4. Verification Harness & `old-pc` Recording

1. Validate against live Freedom Scientific JAWS 2025 on `old-pc`.
2. Extract reference announcements from JAWS speech history for `FindNext`, `FindPrior`, and list activations.
3. Record canonical evidence in `corpus/evidence/`.
4. Ensure 100% differential parity across all 55 scenarios.
