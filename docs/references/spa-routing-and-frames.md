# Single Page Applications (SPA), In-Page Anchors, and Inline Frames

This reference describes the screen reader's navigation and announcement behavior for inline frames (`<iframe>`), same-page jump anchors (e.g. "Skip to content"), and client-side Single Page Application (SPA) route transitions, engineered to match **Freedom Scientific JAWS 2025**.

---

## 1. Inline Frame (`<iframe>`) Navigation

In Freedom Scientific JAWS, nested frames and inline frames (`<iframe>`) are first-class structural document boundaries.

### Keystroke Commands
| Key | Command | Description | Speech Announcement |
| :--- | :--- | :--- | :--- |
| `M` | `MoveToNextFrame` | Jump forward to the start of the next inline frame | `"{FrameTitle} frame"` |
| `Shift + M` | `MoveToPriorFrame` | Jump backward to the start of the prior inline frame | `"{FrameTitle} frame"` |

### Sequential Reading Through Frames
When reading down through a document with the `DownArrow` (`{DOWN}`) reading command:
1. **Entering Frame Boundary**: Announces the frame entry token:
   ```text
   {FrameTitle} frame
   ```
2. **Inner Document Content**: Reads through the nested headings, text nodes, links, and form controls contained within the frame's document tree.
3. **Exiting Frame Boundary**: Upon navigating past the final node inside the frame, announces the exit token:
   ```text
   {FrameTitle} frame end
   ```
4. **Continuing Document**: The subsequent `DownArrow` advances into the next sibling node in the outer parent document.

### Boundary Wrapping
When navigation reaches the last frame in the document and `M` is pressed again (or `Shift+M` from the first frame), the virtual cursor wraps around and announces the standard boundary token:
```text
wrapping to top
{FrameTitle} frame
```
or
```text
wrapping to bottom
{FrameTitle} frame
```

---

## 2. In-Page Skip Links and Fragment Anchors

Modern web applications use in-page fragment links (`<a href="#main-content">`) to allow keyboard and screen reader users to skip navigation bars and header boilerplate.

### Announcement Format
In JAWS 2025, links with fragment hrefs starting with `#` are announced with the specialized `same page link` token:
```text
 \x1d same page link \x1e {LinkText}
```

### Activation (`Enter`)
When the virtual cursor is placed on a same page link and the user presses `Enter` (`ActivateLink`):
1. The link target (`id` or `name` attribute matching the fragment) is resolved against the buffer's indexed target anchor map.
2. The virtual cursor jumps directly to the target element's start offset.
3. The target line is spoken immediately to provide confirmation of destination arrival:
   ```text
    \x1d heading level 2 \x1e Main Content Area
   ```

---

## 3. Client-Side Single Page Application (SPA) Routing

In client-side single page applications (React, Vue, Angular, Next.js, etc.), document navigation occurs without full browser page reloads.

### Page Title Changes
When a route change modifies `document.title`, JAWS announces page load completion:
```text
Page has loaded: {NewDocumentTitle}
```

### Virtual Focus Retargeting
If the SPA updates the URL or route and explicitly retargets focus to the primary view container (e.g., `<main id="main-content">` or `app-root`):
1. The screen reader announces the page load completion token.
2. The virtual cursor updates its offset to the retargeted container or heading.
3. The new line content is announced sequentially:
   ```text
   Page has loaded: {NewDocumentTitle}
    \x1d heading level 2 \x1e Main Content Area
   ```
