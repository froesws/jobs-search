---

description: "Task list for multi-select seniority and modality search filters"
---

# Tasks: Flexible Multi-Select Search (Seniority and Modality)

**Input**: User request: "we should be capable to search trainee, mid/pleno,
jr/junior, pleno-senior at the same search, and remote/hibrid and on site based
on listed cities." No `spec.md` or `plan.md` exists for this feature yet, so
the tasks are derived from the request, the current Rust/frontend source, and
the existing profile/search tests.

**Prerequisites**: Rust toolchain, Node.js `24+`, the existing frontend/Rust
test suites, and a candidate profile with at least one accepted city.

**Tests**: Required. The project rules require tests for new behavior and
failure paths; the search filters are user-facing, so each rule needs a
regression test.

**Organization**: `US1` covers multi-select seniority filters; `US2` covers
multi-select modality filters with city-based hard filtering. The stories are
independently testable, but they share `match/filters.rs`, `filters.ts`,
`TopBar.tsx`, and their test files, so implement them sequentially unless the
work is split by file.

## Current Behavior Notes (verified 2026-11-11)

- **The wire type already supports arrays**: `SearchFilters.seniorities` and
  `SearchFilters.modalities` are arrays in both Rust and TypeScript, and
  `normalizeFilters` deduplicates them. The limitation is in the UI and in the
  hard-filter path, not in the IPC shape.
- **The UI is single-select**: `TopBar` renders `filters.seniorities[0]` in a
  single `Select` and `filters.modalities[0]` in an exclusive
  `ToggleButtonGroup`. Selecting more than one value is impossible today.
- **Search filters do not reach hard filters**: `matching::filters::check`
  uses `profile.accepted_seniorities()` for seniority and only uses the job
  modality for location eligibility. `EvaluationSettings`/`evaluate_run` do
  not carry the search criteria, so the selected levels/modalities affect the
  source query but not the post-ingestion acceptance rules.
- **Modality defaults are inconsistent**: `SearchFilters::resolve` defaults
  modalities to `[Remote, Hybrid, Onsite]`, but `ingestion_request` sets
  `is_remote = modalities.contains(Remote)`. A mixed or all-modality search
  therefore asks JobSpy for remote-only results, which can hide hybrid/onsite
  postings. Set `is_remote = true` only when Remote is the sole selected
  modality; otherwise search broadly and let the hard filters decide. Note that
  JobSpy/Indeed may ignore `is_remote` when `hours_old` is set, so the hard
  filters are always the authoritative modality check.
- **Seniority aliases**: `detect_seniority` recognizes "pleno",
  "mid-level"/"mid level", "jr"/"junior", "trainee", and "senior", but not a
  plain "mid". Add "mid" as Pleno without matching "middle". A
  "Pleno/Sênior" title is currently detected as Pleno; a `Pleno-Senior` filter
  option must therefore expand to both `Pleno` and `Senior`.
- **Cities are profile-owned**: `profile.accepted_cities` remains the source of
  truth for Hybrid/Onsite eligibility. Remote jobs are eligible when they are
  Brazil/global and not explicitly restricted to another region. Do not add a
  second city selector to the search bar.
- **Unknown modality policy**: when all three modalities are selected (the
  default), preserve the current behavior for jobs with an unknown modality;
  when the selection is a strict subset, reject unknown modality because it
  cannot be proven to match the selected set.
- **No LLM change**: the hard filters run before any provider call; the LLM
  prompt and evaluation schema do not change.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete
  task).
- **[Story]**: `[US1]` for seniority, `[US2]` for modality.
- Every task includes an exact file path.

## Path Conventions

- Frontend: `src/` and `src/**/*.test.ts(x)`
- Rust backend: `src-tauri/src/` and `src-tauri/tests/`
- Feature docs: `specs/007-search-seniority-modality/`

---

## Phase 1: Setup (Baseline)

**Purpose**: Establish the current test baseline before changing the search
contract.

- [ ] T001 Run `./scripts/verify.sh` from the repo root and record any pre-existing failures in the first commit message; do not fix unrelated failures in this feature (path: `scripts/verify.sh`).

**Checkpoint**: Baseline is known; no unrelated failures are mixed into the
feature work.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Add the shared search-criteria plumbing both stories need. Do not
add a database migration or change the IPC shape; the arrays already exist.

- [ ] T002 Add `SearchCriteria { seniorities: Vec<Seniority>, modalities: Vec<WorkModality> }` in `src-tauri/src/match/filters.rs`, implement `check_with_criteria(profile, job, &criteria)`, and keep `check(profile, job)` as a wrapper using an empty criteria (empty seniorities fall back to the profile, empty modalities mean all three) so existing tests and callers keep their behavior (path: `src-tauri/src/match/filters.rs`).
- [ ] T003 Add `criteria: SearchCriteria` to `EvaluationSettings`, pass it into every `evaluate_one` call, and use `check_with_criteria` there; update the existing `EvaluationSettings` literals in the fixture test with `..Default::default()` so they compile unchanged (paths: `src-tauri/src/match/mod.rs`, `src-tauri/tests/fixture_match.rs`).
- [ ] T004 Add `SearchFilters::criteria()` in the search command and populate `EvaluationSettings.criteria` with the resolved filters inside `trigger`, so the same seniorities/modalities used for ingestion are used for hard filtering (path: `src-tauri/src/commands/search.rs`).

**Checkpoint**: Existing behavior is preserved through the empty-criteria
wrapper, and the search command now carries the resolved criteria into
evaluation.

---

## Phase 3: User Story 1 - Multi-select seniority (Priority: P1) 🎯 MVP

**Goal**: A single search can select Trainee, Mid/Pleno, Jr/Junior, and the
Pleno-Senior range together. The selected seniorities, not only the profile
defaults, decide which jobs pass the hard filter.

**Independent Test**: Select Trainee + Jr/Junior + Mid/Pleno + Pleno-Senior in
the search bar and run a search. Confirm the wire filters contain
`Trainee`, `Junior`, `Pleno`, and `Senior` (deduplicated), a Pleno/Sênior job
passes when Pleno or Senior is selected, a Senior-only job fails when Senior is
not selected, and clearing the selection falls back to the profile's accepted
seniorities. Confirm a job titled "Mid Rust Engineer" is detected as Pleno.

### Tests for User Story 1 ⚠️

> Write these tests first and confirm they fail against the single-select UI
> and the profile-only hard filter.

- [ ] T005 [P] [US1] Add backend tests for `check_with_criteria`: selecting `[Trainee, Junior, Pleno]` accepts a "Pleno/Sênior" job (detected as Pleno) and rejects a Senior-only job; selecting `[Pleno, Senior]` accepts both; an empty criteria falls back to the profile's accepted seniorities (path: `src-tauri/tests/match_filters.rs`).
- [ ] T006 [P] [US1] Add normalization tests proving `detect_seniority` maps "Mid Rust Engineer", "Mid-level Rust Engineer", and "Pleno/Sênior Rust Engineer" to `Pleno`/the existing precedence, while "middle" does not match "mid" and "Jr/Junior" still maps to `Junior` (path: `src-tauri/src/ingestion/normalize.rs`).
- [ ] T007 [P] [US1] Add frontend helper tests for seniority filter options: the option list includes Trainee, Intern, Jr/Junior, Mid/Pleno, Pleno-Senior (`[Pleno, Senior]`), and Senior; selecting Pleno-Senior adds both values, toggling it off removes both, overlapping options deduplicate, and the selected option ids are derived correctly from a `Seniority[]` (path: `src/lib/filters.test.ts`).
- [ ] T008 [P] [US1] Add TopBar tests for multi-select seniority: selecting Trainee, Jr/Junior, Mid/Pleno, and Pleno-Senior in one interaction calls `onChange` with the expected deduplicated `seniorities` array; clearing the control falls back to the "Profile default" state (path: `src/components/TopBar.test.tsx`).

### Implementation for User Story 1

- [ ] T009 [P] [US1] Add `SENIORITY_FILTER_OPTIONS`, `selectedSeniorityOptionIds`, and `senioritiesForOptions` (or equivalent pure helpers) to `src/lib/filters.ts`; the Pleno-Senior option expands to `[Pleno, Senior]`, values are deduplicated in the canonical `Trainee, Intern, Junior, Pleno, Senior` order, and the helper tolerates overlapping options (path: `src/lib/filters.ts`).
- [ ] T010 [US1] Replace the single seniority `Select` in `TopBar` with a multi-select control (MUI `Select multiple` with checkboxes, or `ToggleButtonGroup multiple`) driven by the helpers from T009; show selected labels/chips, show "Profile default" when `filters.seniorities` is empty, and never reduce the value to `filters.seniorities[0]` (path: `src/components/TopBar.tsx`).
- [ ] T011 [P] [US1] Extend `detect_seniority` so a standalone "mid" (for example "Mid Rust Engineer") is Pleno; reuse `contains_word` so "middle" does not match, and keep "pleno" before "senior" for Pleno/Sênior titles (path: `src-tauri/src/ingestion/normalize.rs`).
- [ ] T012 [P] [US1] Enforce seniority in `check_with_criteria`: resolve the accepted set from `criteria.seniorities` (empty means the profile), reject a job whose detected seniority is not in the set with `SENIORITY_NOT_ELIGIBLE`, and keep jobs with no detected seniority eligible for the existing downstream rules (path: `src-tauri/src/match/filters.rs`).
- [ ] T013 [US1] Run the focused suites (`cargo test --manifest-path src-tauri/Cargo.toml --test match_filters`, the `ingestion::normalize` unit tests, and `npm test -- src/lib/filters.test.ts src/components/TopBar.test.tsx`); fix failures before moving to US2 (paths: `src-tauri/tests/match_filters.rs`, `src/lib/filters.test.ts`).

**Checkpoint**: Multi-select seniority works in the UI and the selected levels
are enforced before the LLM call.

---

## Phase 4: User Story 2 - Multi-select modality and city rules (Priority: P2)

**Goal**: A single search can select Remote, Hybrid, and Onsite together. The
selected modality set is enforced by the hard filters: Remote accepts
Brazil/global, Hybrid/Onsite require a city from `profile.accepted_cities`, and
the source query stops forcing remote-only results for mixed selections.

**Independent Test**: Select Remote only and confirm Hybrid/Onsite jobs are
rejected even in Porto Alegre/Curitiba. Select Hybrid only and confirm a Porto
Alegre hybrid job passes while Remote and Onsite jobs fail. Select Onsite only
and confirm a Curitiba onsite job passes while a São Paulo onsite job fails with
`LOCATION_NOT_ELIGIBLE`. Select all three and confirm known modalities pass and
the unknown-modality behavior is preserved. Confirm `is_remote` is true only for
a Remote-only request.

### Tests for User Story 2 ⚠️

> Write these tests first and confirm they fail against the exclusive modality
> control and the profile/location-only hard filter.

- [ ] T014 [P] [US2] Add backend tests for modality criteria: Remote-only rejects Hybrid and Onsite with `MODALITY_NOT_ELIGIBLE`; Hybrid-only accepts a Porto Alegre hybrid and rejects Remote/Onsite; Onsite-only accepts Curitiba onsite and rejects Hybrid; a São Paulo onsite fails `LOCATION_NOT_ELIGIBLE`; all three accept known modalities; a strict subset rejects Unknown while all three preserve the current Unknown behavior (path: `src-tauri/tests/match_filters.rs`).
- [ ] T015 [P] [US2] Add frontend helper tests for modality toggling: empty means all three; toggling Remote from empty yields `[Hybrid, Onsite]`; toggling Onsite back yields all three normalized to empty; the last selected modality cannot be cleared; the "All" action normalizes to an empty array (path: `src/lib/filters.test.ts`).
- [ ] T016 [P] [US2] Add TopBar tests for multi-select modality: Remote + Hybrid can be selected together, On-site can be added, "All" clears to the all-modalities state, and `onChange` receives a modalities array rather than a single value (path: `src/components/TopBar.test.tsx`).

### Implementation for User Story 2

- [ ] T017 [P] [US2] Add `ALL_MODALITIES`, `effectiveModalities`, and `toggleModality` helpers to `src/lib/filters.ts`; keep at least one modality selected, normalize the all-three state to an empty array, and export the constant for the UI (path: `src/lib/filters.ts`).
- [ ] T018 [US2] Replace the exclusive modality `ToggleButtonGroup` in `TopBar` with a multi-select control driven by T017; show Remote, Hybrid, and On-site as independently toggleable, provide an "All" action that clears to the default, and keep the control usable when `filters.modalities` is empty (path: `src/components/TopBar.tsx`).
- [ ] T019 [P] [US2] Enforce modality in `check_with_criteria`: a known modality must be in the selected set or the job is rejected with `MODALITY_NOT_ELIGIBLE`; when the selection is a strict subset of all three, reject Unknown; when all three are selected, preserve the existing Unknown/location behavior. Keep the Hybrid/Onsite accepted-city rule and Remote Brazil/global rule in `location_eligible` (path: `src-tauri/src/match/filters.rs`).
- [ ] T020 [P] [US2] Fix `SearchFilters::ingestion_request` so `is_remote` is true only when Remote is the sole selected modality (empty/all, Hybrid, Onsite, and mixed selections use false to search broadly); add tests for empty, Remote-only, Hybrid-only, Onsite-only, and mixed selections, and document that JobSpy/Indeed may ignore `is_remote` when `hours_old` is set so the hard filters remain authoritative (path: `src-tauri/src/commands/search.rs`).
- [ ] T021 [US2] Run the focused suites (`cargo test --manifest-path src-tauri/Cargo.toml --test match_filters`, the `commands::search` unit tests, and `npm test -- src/lib/filters.test.ts src/components/TopBar.test.tsx`); fix failures before the Polish phase (paths: `src-tauri/tests/match_filters.rs`, `src/lib/filters.test.ts`).

**Checkpoint**: Multi-select modality works in the UI, city rules are enforced,
and mixed searches no longer force remote-only source results.

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Prove the search filters reach evaluation end to end, document the
semantics, and run the full gate.

- [ ] T022 [P] Add an end-to-end backend test that builds `SearchFilters`, maps them through `criteria()`, runs `evaluate_run` with a fake provider, and proves selected seniorities/modalities reject a job before the LLM while an accepted combination reaches the provider (path: `src-tauri/tests/fixture_match.rs`).
- [ ] T023 [P] Add an end-to-end frontend test that selects multiple seniorities and modalities in the dashboard, triggers a search, and asserts `triggerSearch` receives the deduplicated arrays with Pleno-Senior expanded to Pleno+Senior (path: `src/views/DashboardView.test.tsx`).
- [ ] T024 [P] Document the new search semantics in `README.md`: multi-select seniority and modality, the Pleno-Senior range, profile-default seniorities, all-modalities default, Remote Brazil/global, Hybrid/Onsite accepted cities, the unknown-modality policy, and the `is_remote` source-query rule (path: `README.md`).
- [ ] T025 Run the full local gate: `./scripts/verify.sh` (format, lint, typecheck, frontend tests, Rust fmt/check/clippy/test, and sidecar pytest); report exactly which checks ran and which could not run (path: `scripts/verify.sh`).
- [ ] T026 Perform manual acceptance with a profile whose accepted cities are Porto Alegre and Curitiba: select Trainee + Jr/Junior + Mid/Pleno + Pleno-Senior and Remote + Hybrid + On-site, run a search, and confirm the accepted feed contains the selected levels plus Remote Brazil/global and Hybrid/Onsite only in the listed cities; then select Senior only and confirm a Pleno/Sênior job is rejected unless the Pleno-Senior range is selected (paths: `src/components/TopBar.tsx`, `src-tauri/src/match/filters.rs`).

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies; T001 must pass before code changes.
- **Foundational (Phase 2)**: T002 → T003 → T004; blocks both user stories.
- **US1 (Phase 3)**: Depends on Phase 2. Tests T005-T008 first; T009-T012
  implement; T013 validates.
- **US2 (Phase 4)**: Depends on Phase 2. Tests T014-T016 first; T017-T020
  implement; T021 validates.
- **Polish (Phase 5)**: Depends on US1 and US2 completion.

### User Story Dependencies

- **US1 (P1)**: Independently testable after Phase 2; no dependency on US2
  behavior.
- **US2 (P2)**: Independently testable after Phase 2; no dependency on US1
  behavior.
- **File conflict**: US1 and US2 both edit `src-tauri/src/match/filters.rs`,
  `src/lib/filters.ts`, `src/components/TopBar.tsx`, and their test files. Do
  not run them in parallel unless different developers coordinate those files;
  otherwise implement US1 first and US2 second.

### Within Each User Story

- Tests must be written and fail before implementation.
- US1: T009 before T010; T011 is independent of the frontend work; T012
  depends on T002 and is most useful after T011; T013 is last.
- US2: T017 before T018; T019 depends on T002 and is most useful after T014;
  T020 is independent of the UI; T021 is last.

### Parallel Opportunities

- US1 tests T005, T006, T007, and T008 touch different files and can be written
  in parallel.
- US2 tests T014, T015, and T016 touch different files and can be written in
  parallel.
- Within US1, T009 (frontend helpers) and T011 (backend normalization) can run
  in parallel; T010 follows T009.
- Within US2, T017 (frontend helpers) and T019/T020 (backend) can run in
  parallel; T018 follows T017.
- Polish T022, T023, and T024 touch different files and can run in parallel.
- Do not mark US1 and US2 implementation tasks `[P]` across stories because of
  the shared files listed above.

---

## Parallel Example: User Story 1

```bash
# Tests first:
# T005 -> src-tauri/tests/match_filters.rs
# T006 -> src-tauri/src/ingestion/normalize.rs
# T007 -> src/lib/filters.test.ts
# T008 -> src/components/TopBar.test.tsx

# Implementation:
# T009 -> src/lib/filters.ts
# T011 -> src-tauri/src/ingestion/normalize.rs
# T010 -> src/components/TopBar.tsx (after T009)
# T012 -> src-tauri/src/match/filters.rs
```

## Parallel Example: User Story 2

```bash
# Tests first:
# T014 -> src-tauri/tests/match_filters.rs
# T015 -> src/lib/filters.test.ts
# T016 -> src/components/TopBar.test.tsx

# Implementation:
# T017 -> src/lib/filters.ts
# T019 -> src-tauri/src/match/filters.rs
# T020 -> src-tauri/src/commands/search.rs
# T018 -> src/components/TopBar.tsx (after T017)
```

---

## Implementation Strategy

### MVP First (Foundational + User Story 1)

1. Complete T001 baseline.
2. Complete T002-T004 shared criteria plumbing.
3. Complete US1 (T005-T013) so seniority is multi-select and enforced.
4. **STOP and VALIDATE**: select all seniority categories in one search and
   confirm the accepted feed respects the selected levels.

### Incremental Delivery

1. Deliver US1 for multi-select seniority.
2. Deliver US2 for multi-select modality and city-based hard filtering.
3. Run the Polish phase and the full local gate.
4. Each story is independently testable; the shared criteria plumbing keeps
   both paths consistent.

---

## Notes

- `[P]` tasks touch different files and can be assigned in parallel.
- `Pleno-Senior` is a UI/filter range, not a new enum value: it expands to
  `Pleno` and `Senior` in the existing `seniorities: Seniority[]` wire field.
- Empty `seniorities` means "use the profile's accepted seniorities"; empty
  `modalities` means "all three modalities." The UI must label these defaults
  clearly ("Profile default" and "All").
- Hybrid/Onsite eligibility comes from `profile.accepted_cities`; Remote
  eligibility keeps the Brazil/global rule and rejects explicit foreign
  restrictions. Do not add a second city picker to the search bar.
- `is_remote` is a source-query hint, not the final modality decision. Set it
  true only for Remote-only searches; always enforce the exact selected set in
  `check_with_criteria`. JobSpy/Indeed can ignore `is_remote` when `hours_old`
  is set, so never rely on the source filter for correctness.
- The unknown-modality policy is intentionally conservative for restricted
  selections and backward-compatible when all three modalities are selected.
- No database migration, new dependency, or LLM prompt change is required.
