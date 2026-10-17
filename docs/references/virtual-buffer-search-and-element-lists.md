# In-Buffer Search and Element List Dialogs

This reference describes the screen reader's in-buffer text search mechanics and element list dialog interactions (Headings List, Links List, Form Fields List), engineered to strictly match **Freedom Scientific JAWS 2025**.

---

## 1. In-Buffer Text Search

JAWS provides fast, non-destructive text searching directly within the virtual buffer.

### Keystroke Commands
| Key | JAWS Command | Description | Speech Announcement |
| :--- | :--- | :--- | :--- |
| `Ctrl + F` | `VirtualFind` | Initiates search prompt for target string | Prompts for query string |
| `F3` | `FindNext` | Repeats search forward for active query | Spoken match line or `"wrapping to top\n{Match}"` |
| `Shift + F3` | `FindPrior` | Repeats search backward for active query | Spoken match line or `"wrapping to bottom\n{Match}"` |

### Search Behavior & Wrapping
1. **Target Positioning**: When a match is identified, the virtual cursor is repositioned directly to the beginning of the matching line or element.
2. **Element Semantics**: If the match falls on a semantic element (such as a heading, link, or form field), the screen reader speaks the element with its JAWS announcement formatting (e.g. `• \x1d same page link \x1e Architecture Link` or `Architecture Guide\n \x1d heading level 2 \x1e`).
3. **Boundary Wrapping**:
   - Reaching the end of the buffer wraps to the first match with:
     ```text
     wrapping to top
     {MatchAnnouncement}
     ```
   - Reaching the top of the buffer wraps to the last match with:
     ```text
     wrapping to bottom
     {MatchAnnouncement}
     ```
4. **Negative Search Feedback**: If no matches are found anywhere in the buffer, the virtual cursor position remains unchanged and JAWS speaks:
   ```text
   Text not found: {query}
   ```

---

## 2. Element List Dialogs

JAWS provides element list dialogs allowing users to inspect and jump directly to structural document elements.

### Dialog Shortcuts
| Key | JAWS Command | Dialog Name | Speech Announcement on Open |
| :--- | :--- | :--- | :--- |
| `Insert + F6` | `SelectAHeading` | Headings List | `Heading List  \x1d dialog \x1e \nheadings  \x1d List view \x1e \n{Text} : {Level}\n1 of {Total}\nTo move to items use the Arrow keys.` |
| `Insert + F7` | `SelectALink` | Links List | `Links List  \x1d dialog \x1e \nlinks  \x1d List view \x1e \n{Text}\n1 of {Total}\nTo move to items use the Arrow keys.` |
| `Insert + F5` / `Insert + Ctrl + F` | `SelectAFormField` | Select a Form Field | `Select a Form Field  \x1d dialog \x1e \nList1  \x1d List view \x1e \n{Name} {Role}\n1 of {Total}\nTo move to items use the Arrow keys.` |

### The Links List as a real dialog (2026-10-17)

With Okur's window process running (`okur-ui`, see [helper-ui-architecture.md](helper-ui-architecture.md)),
Insert+F7 opens a real dialog built as JAWS's: the list view "links", Move To Link,
Activate Link (the default), Cancel, the radio groups Sort Links (In Tab Order,
Alphabetically) and Display (All Links, Visited Links Only, Unvisited Links), and the
selected link's address in a read-only field. Okur reads it as JAWS reads its own (dev box):
`Links List dialog | links List view | Reputation for Quality Service | 1 of 13 | To move to
items use the Arrow keys.`, and each Tab stop alike (`Sort Links In Tab Order radio button
checked | 1 of 2 | ... | alt+T`).
- Move To Link closes it and says the link and "To activate press Enter.", as JAWS does.
- Activate Link (Enter) closes it, moves to the link and activates it, silently.
- Escape closes it; the cursor stays. The page taking the focus back is not announced.
- Okur does not know which links were visited yet: Visited Links Only shows none.

Without `okur-ui` (parity replay, headless), the speech-only list below remains, and so do
the headings and form fields lists until they get their dialogs.

### Interaction Model
1. **Navigation (`DownArrow` / `UpArrow`)**: Moves through the items in the list view. Spoken format includes item details and position:
   ```text
   {ItemText} : {Level}
   {Index} of {Total}
   To move to items use the Arrow keys.
   ```
2. **Selection & Jump (`Enter`)**: Closes the dialog, positions the virtual cursor at the target element's buffer offset, and announces the landing element.
3. **Dismissal (`Escape`)**: Closes the dialog without changing cursor position, returning to the document:
   ```text
   virtual PC Cursor
   ```
