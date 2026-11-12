---
description: 'Task list for repairing provider-dialog ghosting and pointer drag reordering'
---

# Tasks: Provider dialog drag and drop

**Input**: Design documents from `/specs/016-provider-drag-ghost/`

**Prerequisites**: `plan.md`, `spec.md`, `quickstart.md`

**Tests**: Required. `AGENTS.md` §10 mandates a test for every behavior change,
including the failure path. Test tasks precede their implementation tasks.

**Organization**: Tasks are grouped by user story so each story is independently
implementable and testable.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: `[US1]`–`[US3]`, mapping to `spec.md`
- Every task names an exact file path

## Path Conventions

- Dialog and helpers: `src/components/`
- Feature docs: `specs/016-provider-drag-ghost/`

---

## Phase 1: Setup

**Purpose**: Capture the reproduction and keep the change dependency-free.

- [X] T001 Add the manual WebKitGTK reproduction and result log to `specs/016-provider-drag-ghost/quickstart.md`; confirm the fix needs no new dependency and may use `createPortal` from `react-dom`.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Pure geometry used by both the ghost and the drop logic.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T002 [P] Add failing unit tests in `src/components/providerDrag.test.ts` for `ghostTransform({ x, y })` producing `translate(x + 14px, y - 18px)`, `rowIndexAt` selecting the containing rect, selecting the nearest row by center distance for a gap coordinate, and clamping coordinates above the first and below the last rect.
- [X] T003 Implement `src/components/providerDrag.ts` with `ghostTransform(point)` and `rowIndexAt(rects, clientY)` so T002 passes; keep both functions pure and exported.

**Checkpoint**: Geometry helpers exist and are covered before any dialog change.

---

## Phase 3: User Story 1 - The ghost follows the pointer from the first frame (Priority: P1) 🎯 MVP

**Goal**: The ghost mounts at the press point, tracks the pointer, and stays above the dialog.

**Independent Test**: Press a handle without moving; the ghost appears at the press point. Move; it tracks at the same offset. Release; it disappears.

### Tests for User Story 1

> Write these tests first; they must fail before T005.

- [X] T004 [US1] In `src/components/ProviderConfigDialog.test.tsx`, assert that after `pointerDown` on `handle-ollama` with `clientX`/`clientY` the ghost has the offset transform immediately (not `translate(0px, 0px)`), is inside `document.body` through a portal, and sits above the dialog's modal z-index.

### Implementation for User Story 1

- [X] T005 [US1] In `src/components/ProviderConfigDialog.tsx`, store the latest pointer position in a ref during `startDrag`, and position the ghost in a `useLayoutEffect` that runs when the drag/ghost mounts; delete the dead `dragGhost.current` write in `startDrag` that runs before the ghost exists.
- [X] T006 [US1] In `src/components/ProviderConfigDialog.tsx`, render the ghost through `createPortal(..., document.body)` with `pointerEvents: 'none'`, `aria-hidden`, and `zIndex: theme.zIndex.modal + 2`; remove the ghost when the drag ends, Escape cancels, or the dialog closes.

**Checkpoint**: US1 works on its own; the ghost never renders at the viewport origin.

---

## Phase 4: User Story 2 - A drop lands on the intended position (Priority: P2)

**Goal**: Multi-row, gap, boundary, and multi-pointer drags resolve to the row under the pointer.

**Independent Test**: Drag the last provider to the first row in one motion, drop in a gap, and drop above/below the list; the order matches the pointer each time.

### Tests for User Story 2

- [X] T007 [US2] In `src/components/ProviderConfigDialog.test.tsx`, add cases for a single pointer move that crosses several rows, a `clientY` that falls in the gap between two stubbed rects, a `clientY` above the first and below the last rect, a second `pointerId` that must not reorder or end the drag, and a re-render during the drag that must not break hit-testing.

### Implementation for User Story 2

- [X] T008 [US2] In `src/components/ProviderConfigDialog.tsx`, add a container ref plus `data-drag-row` on each row, and replace the `orderRows` index-keyed ref array and `targetIndex` scan with live-rect hit-testing through `rowIndexAt` on every move; do not cache rects across moves.
- [X] T009 [US2] In `src/components/ProviderConfigDialog.tsx`, remove the `setPointerCapture` call, store the initiating `pointerId`, and ignore `pointermove`/`pointerup`/`pointercancel` from other pointers; keep window listeners and remove them on drag end.
- [X] T010 [US2] In `src/components/ProviderConfigDialog.tsx`, derive the ghost label and dragged-row highlight from the dragged provider ID (not the moving index) so they stay correct after each reorder.

**Checkpoint**: US1 and US2 work independently.

---

## Phase 5: User Story 3 - A drag can be cancelled without side effects (Priority: P3)

**Goal**: Escape and `pointercancel` restore the pre-drag order; assistive technology hears the new position.

**Independent Test**: Drag two positions, press Escape; the original order returns. Repeat with `pointercancel`. Reorder with the keyboard and hear the announcement.

### Tests for User Story 3

- [X] T011 [US3] In `src/components/ProviderConfigDialog.test.tsx`, assert that Escape restores the exact pre-drag order and hides the ghost, that `pointercancel` does the same, that `pointerup` keeps the new order, and that an `aria-live` region announces the provider and its new position after a reorder.

### Implementation for User Story 3

- [X] T012 [US3] In `src/components/ProviderConfigDialog.tsx`, snapshot the order on drag start, restore it on Escape and `pointercancel`, commit on `pointerup`, and clear the drag when `open` becomes false or the component unmounts.
- [X] T013 [US3] In `src/components/ProviderConfigDialog.tsx`, add a visually hidden `aria-live="polite"` status that announces `"<Provider> moved to position <n> of <total>"` after each pointer or keyboard reorder.

**Checkpoint**: All three stories work independently.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T014 Run `npx vitest run src/components/providerDrag.test.ts src/components/ProviderConfigDialog.test.tsx` and fix every failure.
- [ ] T015 Run `./scripts/verify.sh` and fix every failure; report any skipped check explicitly.
- [ ] T016 [P] Verify the scenarios in `specs/016-provider-drag-ghost/quickstart.md` with `npm run tauri dev` on Linux and record the result in its result log.
- [ ] T017 [P] Update the WebKitGTK pitfall bullet in `AGENTS.md` §13 to record the portal ghost, the no-pointer-capture rule, and live-rect hit-testing.
- [ ] T018 Commit one completed task per commit with a Conventional Commit subject carrying the task ID, for example `fix(T005): position the drag ghost on mount`.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies.
- **Foundational (Phase 2)**: depends on Setup; blocks all stories.
- **US1 (Phase 3)**: depends on T003; MVP.
- **US2 (Phase 4)**: depends on T003 and T006 (the ghost portal and drag lifecycle).
- **US3 (Phase 5)**: depends on T008/T009 (live rects and pointer tracking).
- **Polish (Phase 6)**: depends on the chosen stories.

### Within Each Story

- Tests before implementation.
- `ProviderConfigDialog.tsx` tasks run sequentially; they share one file.
- `ProviderConfigDialog.test.tsx` tasks run sequentially; they share one file.

### Parallel Opportunities

- T002 and T003 are sequential (test, then implementation).
- T016 and T017 run in parallel: different files, no shared dependency.
- Most dialog tasks are sequential by design; the file is the shared resource.

---

## Parallel Example: Polish

```bash
Task: "Verify quickstart scenarios in tauri dev and record the result (T016)"
Task: "Update the WebKitGTK pitfall bullet in AGENTS.md (T017)"
```

---

## Implementation Strategy

### MVP First (User Story 1 only)

1. Phase 1 and Phase 2.
2. Phase 3 (US1).
3. **STOP and VALIDATE** with T004's suite and a manual press-without-move check.

### Incremental Delivery

1. Helpers (Phase 2) → geometry is testable in isolation.
2. US1 → the ghost is visible and correct (MVP).
3. US2 → drops land where the pointer is.
4. US3 → accidents are recoverable and accessible.
5. Polish → focused suites, full gate, desktop verification.

---

## Notes

- `[P]` means different files and no incomplete dependency.
- Keep the dialog's save contract and the local draft behavior unchanged.
- Never rely on HTML5 drag/drop; WebKitGTK does not deliver `drop` reliably.
- Do not hold the drag in a global store; state stays inside the dialog.
- Verify keyboard reordering after every pointer change.
