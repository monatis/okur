# Boundaries, Tab Navigation, and Dynamic Content Reference

This document outlines the JAWS 2025 parity specifications for boundary wrapping, negative quick-key search feedback, ARIA tablist traversal, and hidden subtree filtering.

---

## 1. Boundary Wrapping & Negative Search Announcements

When quick navigation keys (`H`, `T`, `L`, etc.) are used:
- **Wrap-around**: When the cursor reaches the end of the document while moving forward, navigation wraps to the first element and prefixes speech output with:
  ```text
  wrapping to top
  {ElementAnnouncement}
  ```
  When moving backward past the first element, navigation wraps to the last element and prefixes speech with:
  ```text
  wrapping to bottom
  {ElementAnnouncement}
  ```
- **Negative Search Announcements**: When no matching elements exist on the page:
  - Tables: `"There are no tables on this page."`
  - Headings: `"There are no headings on this page."`
  - Lists: `"There are no lists on this page."`
  - Links: `"There are no links on this page."`

---

## 2. Interactive Control Tab Navigation (`{TAB}` / `Shift+{TAB}`)

Pressing `Tab` or `Shift+Tab` navigates through focusable controls in document order:
- Buttons (`Role::Button`)
- Edit boxes (`Role::Edit`)
- Checkboxes (`Role::CheckBox`)
- Links (`Role::Link`)
- ARIA Tabs (`Role::Tab`)

### Tab Announcements
When tabbing into an ARIA tab:
1. If entering the tablist container from an outside control, the container group name is spoken:
   ```text
   {GroupName}  group 
   {TabName}   Tab 
   {Index} of {Total}
   [selected]
   ```
2. If moving between tabs within the same tablist, the group announcement is omitted:
   ```text
   {TabName}   Tab 
   {Index} of {Total}
   [selected]
   ```

### Tab Selection via Spacebar (`{SPACE}`)
Pressing `Space` while focused on a `Role::Tab` activates that tab:
- Deselects sibling tabs within the active tablist.
- Sets the focused tab state to `selected: true`.
- Emits the speech token: `" selected"`.

---

## 3. ARIA Hidden Filtering

Elements marked with:
- `aria-hidden="true"`
- HTML `hidden` attribute
- `states.hidden = true`

are completely filtered out by `BufferBuilder` during linearized virtual buffer generation. Their subtrees are excluded from line navigation, word navigation, and quick-nav indices.
