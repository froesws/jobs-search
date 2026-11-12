# Feature Specification: Provider dialog drag and drop

**Feature Branch**: `[016-provider-drag-ghost]`

**Created**: 2026-11-12

**Status**: Draft

**Input**: User description: "The ghosting and the drag n drop still doesnt work properly"

## Context

`ProviderConfigDialog` reorders the provider chain with pointer events instead
of HTML5 drag and drop, because the Tauri Linux WebKitGTK webview does not
deliver `drop` reliably. The current pointer implementation starts the drag on
the handle, renders a floating ghost, and hit-tests rows by `clientY`.

Three defects remain:

1. The ghost is rendered only after the drag state changes, but the initial
   transform is written before the ghost exists. It first appears at the
   top-left corner of the window and jumps to the pointer on the first move.
2. Row hit-testing reads an index-keyed ref array and requires the pointer to be
   inside a row rect. Gaps between rows, fast multi-row moves, and dragging past
   the ends do not resolve reliably.
3. The drag relies on `setPointerCapture` on the handle that is re-ordered by
   React. When the webview loses capture (or the call throws), the drag stops
   after one position. There is also no way to cancel a drag.

## User Scenarios & Testing

### User Story 1 - The ghost follows the pointer from the first frame (Priority: P1)

As an operator, when I press a reorder handle, a ghost appears immediately under
the pointer and tracks it smoothly above the dialog.

**Why this priority**: Without a visible ghost the drag feels broken, and a ghost
at the wrong position hides the interaction entirely.

**Independent Test**: Open the provider dialog, press and hold a handle, and
confirm the ghost appears at the press point without moving the pointer; move
the pointer and confirm the ghost tracks it at a constant offset; release and
confirm the ghost disappears.

**Acceptance Scenarios**:

1. **Given** the provider dialog is open, **When** the operator presses a reorder
   handle, **Then** the ghost is visible at the press coordinates plus the
   documented offset and never at the viewport origin.
2. **Given** a drag is active, **When** the pointer moves, **Then** the ghost
   follows at the same offset and stays above the dialog surface.
3. **Given** a drag is active, **When** the operator releases the pointer, the
   dialog closes, or the pointer is cancelled, **Then** the ghost is removed.

---

### User Story 2 - A drop lands on the intended position (Priority: P2)

As an operator, I can drag a provider across several rows, through the gaps
between rows, or beyond the list ends, and it lands where I point.

**Why this priority**: A ghost that follows but drops on the wrong row is still
unusable.

**Independent Test**: Drag the last provider to the first position in one motion,
drop in the gap between two rows, and drop above and below the list; confirm the
order matches the pointer position every time.

**Acceptance Scenarios**:

1. **Given** a drag is active, **When** the pointer moves across multiple rows,
   **Then** the dragged provider is placed at the row under the pointer in a
   single move.
2. **Given** the pointer is in the gap between two rows, **When** the pointer
   moves, **Then** the nearest row is used and no move is lost.
3. **Given** the pointer is above the first or below the last row, **When** the
   pointer moves, **Then** the provider moves to the first or last position.
4. **Given** two pointers are active, **When** the second pointer moves or
   releases, **Then** it does not reorder or end the first pointer's drag.

---

### User Story 3 - A drag can be cancelled without side effects (Priority: P3)

As an operator, I can press Escape or lose the pointer, and the provider order
returns to what it was before the drag.

**Why this priority**: An accidental drag must not silently persist a wrong
provider order.

**Independent Test**: Start a drag, move two rows, press Escape; confirm the
original order returns and the ghost disappears. Repeat with `pointercancel`.

**Acceptance Scenarios**:

1. **Given** a drag is active, **When** the operator presses Escape, **Then** the
   pre-drag order is restored, the ghost is removed, and the drag ends.
2. **Given** a drag is active, **When** the webview emits `pointercancel`,
   **Then** the same restore happens.
3. **Given** a drag is active, **When** a reorder happens, **Then** an
   `aria-live` status announces the new position for assistive technology.

### Edge Cases

- The pointer leaves the dialog while dragging; the drag continues and clamps to
  the first or last row.
- The dialog unmounts or closes mid-drag; the ghost and listeners are cleaned up.
- `setPointerCapture` is unavailable or throws; the drag still works.
- The provider list is re-rendered during the drag (catalog refresh, credential
  save); hit-testing reads the live DOM, not a cached index.
- Keyboard reorder buttons remain available and unchanged.

## Requirements

### Functional Requirements

- **FR-001**: Dragging MUST start from `pointerdown` on the reorder handle and
  MUST NOT depend on HTML5 drag/drop events.
- **FR-002**: The ghost MUST receive its position (pointer coordinates plus the
  fixed offset) on the frame it mounts, not on the first pointer move.
- **FR-003**: The ghost MUST be rendered through a portal with a z-index above
  the modal so it is never clipped or hidden behind the dialog or backdrop.
- **FR-004**: Row hit-testing MUST use live row rectangles at the time of the
  move, resolve gaps to the nearest row, and clamp positions beyond the ends.
- **FR-005**: The drag MUST work when pointer capture is unavailable, throws, or
  is lost, and MUST ignore input from pointers other than the one that started
  the drag.
- **FR-006**: Escape and `pointercancel` MUST restore the order captured at drag
  start; `pointerup` MUST commit the current order.
- **FR-007**: The ghost, dragged row, and `aria-live` status MUST stay in sync
  after each reorder, and keyboard reorder controls and `aria-label` values MUST
  be preserved.
- **FR-008**: No new dependency may be added; reuse `react-dom`'s
  `createPortal` and the existing MUI theme.

### Key Entities

- **Drag session**: the provider ID being dragged, its original index, the
  captured order snapshot, the active pointer ID, and the last pointer position.
- **Ghost**: floating label positioned in viewport coordinates above the modal.
- **Row geometry**: the ordered live `DOMRect` list used to resolve a `clientY`
  to an index.

## Success Criteria

### Measurable Outcomes

- SC-001: In `npm run tauri dev` on Linux, the ghost appears at the press point
  on the first frame with no visible jump.
- SC-002: A single press-move-release reorders the provider list from the last to
  the first position, including when the pointer crosses rows in one move.
- SC-003: Escape during a drag restores the exact pre-drag order in 100% of
  attempts.
- SC-004: The dialog test suite covers ghost position, portal stacking, gap and
  boundary drops, Escape/pointercancel restore, and multi-pointer isolation.

## Assumptions

- The provider order is local draft state until Save, so cancelling a drag only
  needs to restore the draft order; no backend call is involved.
- Rows remain vertically stacked; horizontal drag distance is irrelevant.
