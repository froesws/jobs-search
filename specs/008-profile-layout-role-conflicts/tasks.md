---

description: "Task list for profile card overflow and multi-role conflicts"
---

# Tasks: Profile Layout Wrapping and Multi-Role Conflicts

**Input**: Two bug reports: profile text fields overflow into neighboring
cards instead of wrapping, and having more than one role must not be treated as
a profile conflict. No `spec.md` or `plan.md` exists for this feature, so the
tasks are derived from the reports and the current frontend/Rust source.

**Prerequisites**: The current workspace may contain unrelated, uncommitted
job-source integration work; do not include it in this feature's commits.
The relevant files are `src/views/ProfileView.tsx`,
`src/views/ProfileView.test.tsx`, `src-tauri/src/profile/merge.rs`, and
`src-tauri/src/db/profile.rs`.

**Tests**: Required. Both bugs are regressions; each fix needs a test that
fails before the change and passes after it.

**Organization**: `US1` covers the Profile tab layout/overflow; `US2` covers
the false `current_title` conflict and multi-role work-history merging.

## Root Cause Notes (verified 2026-11-11)

- **Grid children can force overflow**: `ProfileView` uses a CSS grid of
  `Paper` cards, but `EditorCard`/`ReadOnlyCard` do not set `minWidth: 0`.
  Grid items default to `min-width: auto`, so a long Autocomplete tag or Chip
  can widen its track and invade the neighboring card.
- **MUI tags and chips do not wrap by default**: MUI `Chip` labels use
  `white-space: nowrap`, and Autocomplete tags are laid out in a flex row.
  Long roles, cities, technologies, or dealbreakers therefore extend past the
  card instead of wrapping.
- **One text field is single-line**: the "Work authorization / visa notes"
  `TextField` has no `multiline`, so long notes scroll horizontally instead of
  wrapping.
- **False current-title conflict**: `merge_extracted` calls
  `record_current_title_conflict`, which compares the first
  `work_history` entry of the existing profile with the first entry of the new
  resume. Multiple roles, reordered resumes, or a promotion make the first
  title differ even though there is no real conflict.
- **Work history is replaced**: the merge currently assigns
  `profile.extras.work_history = extracted.work_history.clone()`, so roles
  that exist only in an earlier resume can be lost. Merge the lists by a
  stable key instead.
- **Existing data can keep the warning**: a profile previously saved with a
  `current_title` conflict in `extras.conflicts` will keep showing the warning
  until those legacy conflicts are filtered or cleared.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete
  task).
- **[Story]**: `[US1]` for layout wrapping, `[US2]` for multi-role conflicts.
- Every task includes an exact file path.

## Path Conventions

- Frontend: `src/views/ProfileView.tsx` and `src/views/ProfileView.test.tsx`
- Rust profile domain: `src-tauri/src/profile/merge.rs`
- Rust persistence: `src-tauri/src/db/profile.rs`
- Integration tests: `src-tauri/tests/profile_pipeline.rs`

---

## Phase 1: Setup (Baseline)

**Purpose**: Establish the current profile-focused baseline without running
the unrelated source-integration changes as part of this feature.

- [X] T001 Run the focused baseline (`npm test -- src/views/ProfileView.test.tsx` and `cargo test --manifest-path src-tauri/Cargo.toml --lib profile::merge`) and record any pre-existing failures; leave unrelated job-source work untouched (paths: `src/views/ProfileView.test.tsx`, `src-tauri/src/profile/merge.rs`).

**Checkpoint**: The profile layout and merge tests are known before editing.

---

## Phase 2: Foundational (Blocking Prerequisites)

No shared implementation prerequisites are required. US1 and US2 touch
different layers and can run independently. Do not add a database schema
migration: profile conflicts live in the existing `profile_json` extras.

**Checkpoint**: T001 is complete; both stories can start.

---

## Phase 3: User Story 1 - Profile cards wrap long text (Priority: P1) 🎯 MVP

**Goal**: Long values in the My Profile tab stay inside their cards at any
supported window width. Autocomplete tags, read-only chips, notes, and the
conflicts alert wrap instead of widening the grid or clipping text.

**Independent Test**: Render a profile with long unbroken values in target
roles, accepted cities, acceptable technologies, dealbreakers, work
authorization, and read-only competencies. Confirm every value remains visible,
the cards stay in their tracks, and the page has no horizontal overflow at
desktop and narrow widths.

### Tests for User Story 1 ⚠️

> Write this test first and confirm it fails against the current grid/chip
> styles.

- [X] T002 [US1] Add a ProfileView regression test with long unbroken values in target roles, accepted cities, acceptable technologies, dealbreakers, work authorization, and read-only competencies; assert the values remain rendered and the wrapping containers expose `minWidth: 0` plus `overflowWrap: anywhere`/`whiteSpace: normal` (path: `src/views/ProfileView.test.tsx`).

### Implementation for User Story 1

- [X] T003 [US1] Add a shared wrapping style (`minWidth: 0`, `overflowWrap: 'anywhere'`, `wordBreak: 'break-word'`) and apply it to the profile grid, `EditorCard`, `ReadOnlyCard`, and the conflicts `Alert` (path: `src/views/ProfileView.tsx`).
- [X] T004 [US1] Make the profile grid responsive with `repeat(auto-fit, minmax(min(100%, 330px), 1fr))` and set `minWidth: 0` on every grid child so a long value cannot widen its track (path: `src/views/ProfileView.tsx`).
- [X] T005 [US1] Fix Autocomplete tag overflow for target roles, seniorities, accepted cities, acceptable technologies, and dealbreakers: use `sx`/`renderTags` so each tag is capped at the field width and its label wraps onto multiple lines (path: `src/views/ProfileView.tsx`).
- [X] T006 [US1] Fix read-only Chip overflow in `ReadOnlyCard`: allow chip labels to wrap, set `maxWidth: '100%'`, and keep the chip height auto so long competencies and certifications stay inside the card (path: `src/views/ProfileView.tsx`).
- [X] T007 [US1] Make the "Work authorization / visa notes" field multiline (`minRows={2}`) and ensure the conflicts alert text wraps instead of overflowing the page (path: `src/views/ProfileView.tsx`).
- [X] T008 [US1] Run `npm test -- src/views/ProfileView.test.tsx` and inspect the page at 1024/768/480 px widths with the long-value fixture (path: `src/views/ProfileView.test.tsx`).

**Checkpoint**: Profile cards wrap long text without invading adjacent cards.

---

## Phase 4: User Story 2 - Multiple roles are not a conflict (Priority: P2)

**Goal**: Multiple work-history roles, reordered resumes, and promotions do not
produce a `current_title` conflict. Work history from multiple resumes is
merged without losing roles, and legacy `current_title` conflicts stop showing.

**Independent Test**: Merge two resumes that contain multiple overlapping
roles. Confirm `extras.conflicts` has no `current_title` entry, both unique
roles remain in `extras.work_history`, and a stored profile with a legacy
`current_title` conflict no longer shows the Profile tab warning.

### Tests for User Story 2 ⚠️

> Write these tests first and confirm they fail against the current
> `record_current_title_conflict` and replacement behavior.

- [X] T009 [US2] Replace `flags_conflicting_current_titles_without_discarding_values` with a regression test that a profile with multiple work-history roles produces no `current_title` conflict, and add a merge test that two resumes with different/overlapping roles union the work history without duplicates (path: `src-tauri/src/profile/merge.rs`).
- [X] T010 [P] [US2] Add an end-to-end upload test in which two PDFs contribute different work-history roles; assert the saved profile has no `current_title` conflict and contains both unique roles in `extras.work_history` (path: `src-tauri/tests/profile_pipeline.rs`).
- [X] T011 [P] [US2] Add a ProfileView test with multiple `extras.work_history` entries and no conflicts; assert the conflict warning is absent, and add a case with a legacy `current_title` conflict to confirm the UI is driven only by the filtered backend result (path: `src/views/ProfileView.test.tsx`).

### Implementation for User Story 2

- [X] T012 [US2] Remove `record_current_title_conflict` and its call from `merge_extracted`; update the merge doc comment so it no longer claims current-title values are recorded as conflicts, while keeping `ProfileConflict` available for future non-role conflicts (path: `src-tauri/src/profile/merge.rs`).
- [X] T013 [US2] Replace the `profile.extras.work_history = extracted.work_history.clone()` assignment with a deduplicated union: latest-resume roles first, then older roles whose normalized title/company/start/end key is not already present (path: `src-tauri/src/profile/merge.rs`).
- [X] T014 [US2] Filter legacy `current_title` conflicts when loading a profile and when merging, so existing profiles saved with the false conflict stop showing the warning; persist the cleaned profile on the next save (paths: `src-tauri/src/db/profile.rs`, `src-tauri/src/profile/merge.rs`).
- [X] T015 [US2] Run the focused suites (`cargo test --manifest-path src-tauri/Cargo.toml --lib profile::merge`, `cargo test --manifest-path src-tauri/Cargo.toml --test profile_pipeline`, and `npm test -- src/views/ProfileView.test.tsx`) and fix failures (paths: `src-tauri/tests/profile_pipeline.rs`, `src/views/ProfileView.test.tsx`).

**Checkpoint**: Multiple roles are preserved and never reported as a conflict;
legacy current-title warnings disappear.

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Final regression coverage and validation across both stories.

- [X] T016 Run the full local gate `./scripts/verify.sh` (format, lint, typecheck, frontend tests, Rust fmt/check/clippy/tests, and sidecar pytest) and report exactly which checks ran (path: `scripts/verify.sh`).
- [ ] T017 Perform manual acceptance at 1024/768/480 px: fill every profile card with long unbroken values, confirm no horizontal overflow or card invasion, and confirm a profile with multiple work-history roles shows no conflict warning (paths: `src/views/ProfileView.tsx`, `src-tauri/src/profile/merge.rs`).
- [ ] T018 [P] Review `merge.rs` comments and the ProfileView conflict copy for stale language that still implies a single role or a current-title conflict; update the wording to describe multi-role work history (path: `src-tauri/src/profile/merge.rs`).

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: T001 only.
- **Foundational (Phase 2)**: No tasks; both stories can start after T001.
- **US1 (Phase 3)**: T002 first; T003-T007 are one component and run
  sequentially; T008 validates.
- **US2 (Phase 4)**: T009-T011 first (T010/T011 touch different files and can
  run in parallel with T009); T012-T014 share `merge.rs`/`profile.rs` and run
  sequentially; T015 validates.
- **Polish (Phase 5)**: Depends on US1 and US2.

### User Story Dependencies

- **US1 (P1)**: Independent of US2; no backend changes.
- **US2 (P2)**: Independent of US1; no CSS changes.
- US1 and US2 can be implemented in parallel by different developers, but the
  final full gate must run after both.

### Within Each User Story

- Tests must fail before implementation.
- US1: T003-T007 all edit `ProfileView.tsx`, so do not parallelize them.
- US2: T012-T014 all edit `merge.rs` or `profile.rs`, so do not parallelize
  them. T010 and T011 touch different files and can run beside T009.

### Parallel Opportunities

- T009, T010, and T011 can be written in parallel (different files).
- T018 can run in parallel with the final gate once US2 is complete.
- US1 and US2 can run in parallel across layers.

---

## Parallel Example: User Story 2

```bash
# Tests first:
# T009 -> src-tauri/src/profile/merge.rs
# T010 -> src-tauri/tests/profile_pipeline.rs
# T011 -> src/views/ProfileView.test.tsx

# Then implementation, sequentially because of shared files:
# T012-T014 -> src-tauri/src/profile/merge.rs and src-tauri/src/db/profile.rs
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Complete T001.
2. Complete T002-T008 so long profile values wrap correctly.
3. **STOP and VALIDATE** at narrow and desktop widths.

### Incremental Delivery

1. Deliver US1 (layout).
2. Deliver US2 (multi-role merge and legacy conflict cleanup).
3. Run the full gate and manual acceptance.

---

## Notes

- Do not add a database migration: conflicts are stored in the existing
  `profile_json` extras. Filtering legacy `current_title` entries on load is
  enough; the next save persists the cleaned profile.
- Do not treat different roles, different resume ordering, or a promotion as a
  conflict. Work history is a list, not a scalar field.
- Keep the `ProfileConflict` type for future non-role conflicts, but do not
  generate `current_title` conflicts.
- The ProfileView tests run in JSDOM and cannot prove pixel layout; assert the
  wrapping styles on the actual containers and use the manual acceptance task
  at narrow widths for the visual check.
