---

description: "Task list for aligning TopBar filter selector tops (seniority / work type) with neighboring items, ignoring their caption labels"

# Tasks: TopBar Filter Selector Alignment

**Input**: Bug report: "no caso dos seletores abaixo de seniority e work type, o topo dos
seletores - ignorando os respectivos textos 'seniority' e 'work type' deviam estar
alinhados com o todo dos itens ao lado, sempre" (for the selectors below seniority and
work type, the top of the selectors - ignoring the respective 'seniority' and 'work
type' texts - must always be aligned with all items beside them).

**No `spec.md` or `plan.md` exists for this feature**, so these tasks are derived from
the current source (`src/components/TopBar.tsx`) and the report.

**Prerequisites**: Node + npm from `package.json`, Vitest, oxlint, Prettier, TypeScript.

**Tests**: Required. Add a rendering test asserting selector-top alignment and keep the
existing TopBar tests green.

**Organization**: `US1` (P1) aligns the seniority and work type selector tops with the
Timeframe select and Stack field tops, ignoring the "Seniority"/"Work type" captions,
in all states (empty/selected/wrapped).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task).
- **[Story]**: `[US1]` filter selector alignment.
- Every task includes an exact file path.

## Path Conventions

- Frontend component: `src/components/TopBar.tsx`
- Frontend tests: `src/components/TopBar.test.tsx`
- Feature docs: `specs/013-topbar-filter-alignment/`

---

## Phase 1: Setup

**Purpose**: Confirm the current filter-row alignment behavior before changing it.

- [X] T001 Reproduce the misalignment: render `TopBar` in a browser/test and confirm
  that the seniority and work type `ToggleButtonGroup` tops do not line up with the
  Timeframe `Select` and Stack `TextField` tops (path: `src/components/TopBar.tsx`,
  `src/components/TopBar.test.tsx`).

**Checkpoint**: Baseline alignment is documented; the `filter-row` uses
`alignItems: 'flex-start'` and each `FilterGroup` renders its caption above the
selector, so selector tops start at different offsets.

---

## Phase 3: User Story 1 - Align filter selector tops (Priority: P1) 🎯 MVP

**Goal**: The top of the seniority and work type selectors (ToggleButtonGroup) lines up
with the top of the Timeframe select and Stack field, ignoring the "Seniority" and "Work
type" caption texts, in every state (empty, selected, wrapped).

**Independent Test**: Render the drawer/TopBar and confirm the toggle groups' top edge
is vertically aligned with the Timeframe Select and Stack TextField top edge, both when
no filter is selected and when a value is selected.

### Tests for User Story 1

> **NOTE: Write these tests FIRST; they must fail before T004.**

- [X] T002 [P] [US1] Add a test in `src/components/TopBar.test.tsx` asserting that the
  filter controls' tops align: the seniority `ToggleButtonGroup` and work type
  `ToggleButtonGroup` share the same vertical start as the Timeframe `Select` and Stack
  `TextField` (assert on the computed layout/sx that the selector top offset equals the
  neighbors', and that `filter-row` keeps `alignItems: 'flex-start'`).
- [X] T004 [US1] In `src/components/TopBar.tsx`: harden filter-row alignment by declaring
  `alignItems: 'flex-start'` in the `filter-row` Box `sx` (previously only in inline
  `style`), so the seniority and work type selector tops line up with the Timeframe
  `Select` and Stack `TextField` tops while keeping the "Seniority"/"Work type"
  captions visible. Keep `filter-row` `gap`, `flexWrap`, and `alignItems: 'flex-start'`;
  no new dependency; MUI/Emotion only.
- [ ] T005 [P] [US1] Run `npm test -- src/components/TopBar.test.tsx`,
  `npm run typecheck`, `npm run lint`, and `npm run format:check`. Fix only failures
  introduced by T004 (`src/components/TopBar.tsx`, `src/components/TopBar.test.tsx`).

**Checkpoint**: Selector tops align with neighboring items in all states.

---

## Phase 4: Polish & Cross-Cutting Concerns

- [ ] T006 [P] Confirm the change stays within the project stack: MUI/Emotion only,
  function components, no new data-fetching library, no new dependency
  (`src/components/TopBar.tsx`).
- [ ] T007 [P] Manual acceptance: open the app, verify the seniority and work type
  selectors line up with the Timeframe and Stack fields beside them when empty and when
  a value is selected; verify the filter row still wraps gracefully on narrow widths
  (`src/components/TopBar.tsx`).

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: Starts immediately; documents the baseline.
- **User Story (Phase 3)**: Depends on Setup. T002 (test) must be written and fail
  before T004 (implementation). T005 depends on T004.
- **Polish (Phase 4)**: Depends on Phase 3.

### Parallel Opportunities

- T006 (stack compliance) can run in parallel with T004/T005.
- T007 manual acceptance depends on T004/T005.

---

## Parallel Example: User Story 1

```bash
# Stack compliance can run while the implementation lands:
Task: "Confirm MUI/Emotion-only stack, no new dependency (T006)"
Task: "Implement selector-top alignment in src/components/TopBar.tsx (T004)"
```

---

## Implementation Strategy

### MVP First (User Story 1 only)

1. Complete Phase 1 (baseline/reproduction).
2. Write T002 test and confirm it fails.
3. Complete T004 (implementation in `src/components/TopBar.tsx`).
4. Complete T005 (tests + checks pass).
5. **STOP and VALIDATE**: selector tops align with neighbors.
6. Complete Phase 4 (polish + manual acceptance).

---

## Notes

- The `[P]` marker means different files and no incomplete dependency.
- Keep the "Seniority"/"Work type" captions visible; alignment must ignore them for the
  selector-top comparison.
- Do not add `alignItems: 'center'` (it would center whole wrappers, not selector
  tops); align selector tops specifically.
- Commit after each task; stop at each checkpoint to validate independently.
