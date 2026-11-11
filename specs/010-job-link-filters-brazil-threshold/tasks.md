---

description: "Task list for copyable job link, filter alignment, Brazil-only jobs, and threshold 50"
---

# Tasks: Job Link, Filter Alignment, Brazil-Only Search, Threshold 50

**Input**: Four user reports: the original job link must be copyable text instead
of a button; selecting one seniority/work-type value and removing the "Profile
default" footer makes the filter cards misalign; only jobs in Brazil/Brasil are
allowed; the approval threshold should be 50 instead of 70.

No `spec.md` or `plan.md` exists for this feature, so the tasks are derived from
the reports and the current frontend/Rust source. The active feature pointer
still targets `specs/008-profile-layout-role-conflicts`, which is unrelated;
this list is self-contained and must not modify that feature.

**Prerequisites**: The full local gate (`./scripts/verify.sh`) must pass before
the changes. `.env` is gitignored and local; never commit it. Job URLs and job
data are untrusted and must keep going through `safeExternalUrl`.

**Tests**: Required by the project rules for new behavior and failure paths.
Frontend uses Vitest + Testing Library; Rust uses the existing contract tests.

**Organization**: `US1` (P1) renders the link as copyable text. `US2` (P1)
enforces Brazil-only locations. `US3` (P2) fixes the filter bar alignment.
`US4` (P2) changes the default approval threshold to 50. Each story is
independently testable.

## Root Cause Notes (verified 2026-11-11)

- **Link is not text**: `src/components/JobDrawer.tsx` renders the URL as a
  MUI `Button component="a"` labelled "Open original posting". The URL itself
  is never displayed, so it cannot be selected or copied.
- **Filter height changes**: `src/components/TopBar.tsx` lays the filter row out
  with `alignItems: 'center'` and renders the "Profile default" caption only
  when a panel is empty. Selecting a value removes that caption, shrinks the
  group's box, and re-centers it, so the toggle groups no longer line up with the
  neighbouring fields.
- **Brazil is not enforced**: `mentions_brazil` in
  `src-tauri/src/match/filters.rs` treats `latam`, `latin america`,
  `south america`, `worldwide`, `anywhere`, and `global` as Brazil. Remote
  postings are also accepted when they merely avoid a short hardcoded blocklist
  (`mentions_restricted_region`), which misses most countries.
- **Threshold lives in three places**: `src-tauri/src/config/env.rs`
  (`parse_i64(..., 70, ...)`), `src-tauri/src/match/mod.rs`
  (`EvaluationSettings::default`), and `.env.example` / the local `.env`.
  `README.md`, `specs/001-career-job-sourcing/spec.md`, and
  `specs/002-job-sourcing-dashboard/spec.md` also state 70.
- **Unknown metadata rule must be preserved**: a missing location/modality must
  not exclude a posting by itself. Brazil-only means a *known* non-Brazil
  location fails; unknown stays allowed unless the text proves a foreign
  location.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete
  task).
- **[Story]**: `[US1]` copyable link, `[US2]` Brazil-only, `[US3]` alignment,
  `[US4]` threshold 50.
- Every task includes an exact file path.

## Path Conventions

- Frontend: `src/components/`, `src/views/`, and `src/lib/`
- Rust backend: `src-tauri/src/` and `src-tauri/tests/`
- Configuration: `.env` (local, gitignored), `.env.example`, `README.md`
- Feature docs: `specs/010-job-link-filters-brazil-threshold/`

---

## Phase 1: Setup (Baseline)

**Purpose**: Confirm the current gate before touching four unrelated areas.

- [x] T001 Run `./scripts/verify.sh` and record the exact passing/failing checks;
  do not start implementation while an unrelated failure is present (path:
  `scripts/verify.sh`).

**Checkpoint**: Baseline is known and no unrelated failures are mixed in.

---

## Phase 2: Foundational (Blocking Prerequisites)

No shared implementation prerequisites are required. US1 and US3 are frontend
only, US2 is Rust only, and US4 touches configuration/docs; they can proceed in
parallel after T001. Do not add a new dependency for any story.

**Checkpoint**: T001 is complete; all four stories can start.

---

## Phase 3: User Story 1 - Copyable original job link (Priority: P1)

**Goal**: The job drawer shows the original posting URL as selectable text that
can be copied with the keyboard or mouse, without a "Open original posting"
button.

**Independent Test**: Render the drawer and confirm the normalized URL appears
as an anchor's visible text, no button with that label exists, a long URL wraps
instead of overflowing, and an invalid URL still shows the warning.

### Tests for User Story 1

- [x] T002 [P] [US1] Add tests in `src/components/JobDrawer.test.tsx`: the
  normalized `job.url` is visible as link text with `href`, the
  `button` role "Open original posting" is absent, the element uses
  `user-select: text` and `word-break: break-all`, and the invalid-URL warning
  still renders.

### Implementation for User Story 1

- [x] T003 [US1] Replace the `Button` in `src/components/JobDrawer.tsx` with a
  `Typography component="a"` (or equivalent anchor) whose children are the
  normalized URL; keep `target="_blank"`, `rel="noreferrer"`,
  `safeExternalUrl`, and add `wordBreak: 'break-all'`, `userSelect: 'text'`, and
  a visible "Original posting" caption.
- [x] T004 [US1] Run `npm test -- src/components/JobDrawer.test.tsx` and
  `npm run typecheck`; verify in the running drawer that a long URL wraps and is
  selectable/copyable (path: `src/components/JobDrawer.tsx`).

**Checkpoint**: The posting URL is visible, selectable, and copyable; no action
button is used for the link.

---

## Phase 4: User Story 2 - Only jobs in Brazil/Brasil (Priority: P1)

**Goal**: Every evaluated posting is provably in Brazil. Known locations outside
Brazil, and generic global/LatAm postings, are rejected before the LLM call.

**Independent Test**: With the fixture profile, "Remote - Brazil" and
"Brasil" pass; "Remote - Worldwide", "Remote - USA", "Remote - Europe",
"Buenos Aires, Argentina", "Lisbon, Portugal", and "Mexico" are rejected;
Hybrid/Onsite still require Porto Alegre or Curitiba; an unknown location still
passes when no foreign region is named.

### Tests for User Story 2

- [x] T005 [P] [US2] Rewrite the location tests in
  `src-tauri/tests/match_filters.rs`: Brazil/Brasil/Brazilian/BR and Brazilian
  cities pass; Worldwide/global/anywhere/LatAm/Latin America/South America,
  USA/Europe/Argentina/Portugal/Mexico fail; Hybrid/Onsite outside accepted
  cities still fail; unknown location passes.
- [x] T006 [P] [US2] Add unit tests in `src-tauri/src/match/filters.rs` for
  `mentions_brazil` (only explicit Brazil markers count) and for the unknown
  location branch.

### Implementation for User Story 2

- [x] T007 [US2] Implement the allowlist in `src-tauri/src/match/filters.rs`:
  remove `latam`, `latin america`, `south america`, `worldwide`, `anywhere`, and
  `global` from `mentions_brazil`; for Remote/Unknown require a Brazil marker
  when the location is known; replace the `mentions_restricted_region`
  blocklist fallback; keep unknown/empty location allowed unless the text names
  a foreign region; keep the accepted-cities rule for Hybrid/Onsite.
- [ ] T008 [US2] Update the "Search filters" section in `README.md` to state
  that Remote now accepts Brazil only and that unknown locations are kept only
  when nothing proves a foreign location.
- [ ] T009 [US2] Run
  `cargo test --manifest-path src-tauri/Cargo.toml --test match_filters` and the
  full `cargo test`; confirm no existing accepted-city behavior regressed
  (paths: `src-tauri/src/match/filters.rs`,
  `src-tauri/tests/match_filters.rs`).

**Checkpoint**: The hard filter rejects known non-Brazil and global postings
while preserving the unknown-metadata rule.

---

## Phase 5: User Story 3 - Stable filter bar alignment (Priority: P2)

**Goal**: Selecting a seniority or work-type value removes the "Profile default"
caption without changing the card height or shifting the row.

**Independent Test**: Render the top bar with empty and non-empty panels; the
footer slot exists in both states, the caption is hidden when selected, and the
row uses top alignment so all filter groups start at the same height.

### Tests for User Story 3

- [ ] T010 [P] [US3] Add tests in `src/components/TopBar.test.tsx`: both footer
  slots are present with no selection; after selecting one seniority (and
  separately one modality) the caption is hidden but its slot remains
  (`aria-hidden`/`visibility: hidden`), and the filter row uses
  `align-items: flex-start`.

### Implementation for User Story 3

- [ ] T011 [US3] In `src/components/TopBar.tsx`, extract a `FilterGroup` that
  always reserves the caption line (render an `aria-hidden` placeholder with
  `visibility: 'hidden'` when a value is selected) and change the filter row
  from `alignItems: 'center'` to `alignItems: 'flex-start'`; apply it to both
  Seniority and Work type.
- [ ] T012 [US3] Run `npm test -- src/components/TopBar.test.tsx` and verify in
  the dev app that selecting one seniority and one work type keeps every group
  and the cards below aligned (path: `src/components/TopBar.tsx`).

**Checkpoint**: The filter row height is identical with and without a selection.

---

## Phase 6: User Story 4 - Default approval threshold 50 (Priority: P2)

**Goal**: A score of 50 or more is approved by default; 49 remains pending.
Explicit `MATCH_THRESHOLD` values still override the default within 0-100.

**Independent Test**: With no override, a 50 score is `APPROVED` and a 49 score
is `PENDING`; setting `MATCH_THRESHOLD=70` restores the old boundary.

### Tests for User Story 4

- [ ] T013 [P] [US4] Update the default-threshold assertions in
  `src-tauri/src/config/env.rs` (70 -> 50) and
  `src-tauri/tests/fixture_match.rs` (`EvaluationSettings::default`), and change
  the boundary test in `src-tauri/tests/score_contract.rs` to assert 50 is
  inclusive while 49 is pending.
- [ ] T014 [P] [US4] Add a config test in `src-tauri/src/config/env.rs` proving
  `MATCH_THRESHOLD=70` still overrides the 50 default.

### Implementation for User Story 4

- [ ] T015 [US4] Change the default from 70 to 50 in
  `src-tauri/src/config/env.rs` (`parse_i64` default),
  `src-tauri/src/match/mod.rs` (`EvaluationSettings::default`), and
  `.env.example`; keep the 0-100 clamp.
- [ ] T016 [US4] Set `MATCH_THRESHOLD=50` in the local gitignored `.env` (do not
  commit it) and confirm the running app reports the new default.
- [ ] T017 [P] [US4] Update every document that states 70 as the default:
  `README.md`, `specs/001-career-job-sourcing/spec.md`, and
  `specs/002-job-sourcing-dashboard/spec.md`.
- [ ] T018 [US4] Run `cargo test --manifest-path src-tauri/Cargo.toml` and
  confirm the 50/49 boundary and the explicit-override test both pass (path:
  `src-tauri/Cargo.toml`).

**Checkpoint**: Default approval happens at 50 and is still configurable.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T019 [P] Update `README.md` with a short UI note: the original posting is
  selectable text, the filter footer reserves its line, and only Brazil jobs are
  accepted by default.
- [ ] T020 Run `./scripts/verify.sh` and both sidecar pytest suites; report
  exactly which checks ran and which could not run (path: `scripts/verify.sh`).
- [ ] T021 Manual acceptance: open a job, select and copy the URL; select one
  seniority and one work type and compare alignment; run a real search and
  confirm no foreign posting reaches the pending/approved lists and the
  threshold boundary is 50 (paths: `src/components/JobDrawer.tsx`,
  `src/components/TopBar.tsx`, `src-tauri/src/match/filters.rs`,
  `src-tauri/src/config/env.rs`).

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: T001 must pass before any change.
- **US1 (Phase 3)**: Independent; frontend only.
- **US2 (Phase 4)**: Independent; Rust only.
- **US3 (Phase 5)**: Independent; frontend only, different files from US1.
- **US4 (Phase 6)**: Independent; configuration and docs.
- **Polish (Phase 7)**: Depends on US1-US4.

### User Story Dependencies

- **US1 (P1)**: No dependency on the other stories.
- **US2 (P1)**: No dependency; the location filter does not touch the drawer or
  top bar.
- **US3 (P2)**: No dependency; only `TopBar.tsx` and its test.
- **US4 (P2)**: No dependency; only defaults, `.env`, tests, and docs.

### Within Each User Story

- Tests fail before implementation.
- US1: T002 -> T003 -> T004.
- US2: T005/T006 -> T007 -> T008 -> T009.
- US3: T010 -> T011 -> T012.
- US4: T013/T014 -> T015 -> T016/T017 -> T018.

### Parallel Opportunities

- US1, US2, US3, and US4 touch disjoint files and can run in parallel after T001.
- T002, T005, T010, and T013 are all test-only and can be written in parallel.
- T008, T017, and T019 are documentation-only and can run while implementation
  continues.

---

## Parallel Example: All Stories

```bash
# Tests first (disjoint files):
# US1 -> src/components/JobDrawer.test.tsx
# US2 -> src-tauri/tests/match_filters.rs and src-tauri/src/match/filters.rs
# US3 -> src/components/TopBar.test.tsx
# US4 -> src-tauri/src/config/env.rs and src-tauri/tests/score_contract.rs

# Implementation:
# US1 -> src/components/JobDrawer.tsx
# US2 -> src-tauri/src/match/filters.rs
# US3 -> src/components/TopBar.tsx
# US4 -> src-tauri/src/config/env.rs and src-tauri/src/match/mod.rs
```

---

## Implementation Strategy

### MVP First (P1 Stories)

1. Complete T001.
2. Ship US1 (copyable link) and US2 (Brazil-only) as the two P1 fixes; they are
   independent and cover the user-visible rules.
3. Ship US3 (alignment) and US4 (threshold 50) next.
4. Run the Polish phase and the full gate.

### Incremental Delivery

1. US1: drawer-only change with a component test.
2. US2: hard-filter change with contract tests; re-run a real search to confirm
   foreign postings disappear.
3. US3: layout-only change with a rendering test.
4. US4: default/config/docs change; verify the 50/49 boundary.

---

## Notes

- Keep using `safeExternalUrl`; never render an unvalidated URL as an anchor.
- Do not turn the copyable URL back into a button; text selection is the
  requirement.
- For Brazil-only, do not reject a posting solely because the location is
  unknown; reject only known non-Brazil or explicitly global locations.
- Hybrid/Onsite still require an accepted city; do not weaken that rule.
- Threshold changes must stay within 0-100 and remain overridable through the
  environment.
- Never commit `.env`; only `.env.example` is tracked.

---

## Execution Notes (2026-11-11)

- **T001 baseline**: `./scripts/verify.sh` passed before the changes.
- **T004**: `npm test -- src/components/JobDrawer.test.tsx` passed (6).
