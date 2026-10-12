# Reference: JAWS Quick Navigation Keys & Structural Navigation

This reference document outlines the keyboard conventions, speech announcements, and virtual buffer navigation behaviors for Freedom Scientific JAWS 2025 structural navigation keys supported by the screen reader engine.

---

## 1. Quick Navigation Keys Matrix

| Key / Command | JAWS Script Function | Semantic Role | Direction | Speech Output Format |
|---|---|---|---|---|
| `H` | `MoveToNextHeading` | `Heading { level: 1..6 }` | Forward | `{HeadingText}\n \u001d heading level {level} \u001e` |
| `Shift + H` | `MoveToPriorHeading` | `Heading { level: 1..6 }` | Prior | `{HeadingText}\n \u001d heading level {level} \u001e` |
| `1` – `6` | `MoveToNextHeadingLevel{N}` | `Heading { level: N }` | Forward | `{HeadingText}\n \u001d heading level {N} \u001e` |
| `Shift + 1` – `6` | `MoveToPriorHeadingLevel{N}`| `Heading { level: N }` | Prior | `{HeadingText}\n \u001d heading level {N} \u001e` |
| `U` / `K` | `MoveToNextUnvisitedLink` | `Link` | Forward | `•  \u001d same page link \u001e {LinkText}` (same-page) or ` \u001d link \u001e {LinkText}` |
| `Shift + U` / `K` | `MoveToPriorUnvisitedLink` | `Link` | Prior | `•  \u001d same page link \u001e {LinkText}` (same-page) or ` \u001d link \u001e {LinkText}` |
| `L` | `MoveToNextList` | `List` | Forward | `List of {count} items` |
| `Shift + L` | `MoveToPriorList` | `List` | Prior | `List of {count} items` |
| `I` | `MoveToNextListItem` | `ListItem` | Forward | `{ItemText}` (or with child link formatting) |
| `Shift + I` | `MoveToPriorListItem` | `ListItem` | Prior | `{ItemText}` (or with child link formatting) |
| `R` / `;` | `MoveToNextRegion` | `Landmark` | Forward | `{Name} {LandmarkType} region` or `{LandmarkType}` |
| `Shift + R` / `;` | `MoveToPriorRegion` | `Landmark` | Prior | `{Name} {LandmarkType} region` or `{LandmarkType}` |

---

## 2. AOM Linearization & Hierarchy Resolution

1. **Containers**:
   - `Landmark`: Linearized at offset boundary preceding child content.
   - `List`: Linearized at offset boundary preceding first list item.
   - `ListItem`: Linearized as distinct block lines and paragraphs within `VirtualBuffer`.
2. **Context-Sensitive Navigation**:
   - When focused on a `List` container via `L`, pressing `I` (`MoveToNextListItem`) immediately selects the first list item within that container.
3. **JAWS Speech Token Formatting**:
   - JAWS incorporates control codes `\u001d` (Group Separator) and `\u001e` (Record Separator) around structural tokens such as `heading level {N}` and `link` to instruct the speech synthesizer on role prosody and history buffering.
