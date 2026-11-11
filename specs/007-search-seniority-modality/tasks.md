---

description: "Task list for search panels and Portuguese query terms"
---

# Tasks: Search Panels with Portuguese Query Terms (Seniority and Work Type)

**Input**: User request: "No lugar de any na senioridade, é melhor um painel
com trainee|junior|pleno|senior onde possamos clicar um por um; assim se faz
uma pesquisa procurando vagas de trainee, outra de junior, outra de pleno se os
3 estiverem clicados. O mesmo vale pro tipo de emprego On-site|híbrido|remoto.
Lembre que, embora a UI esteja em inglês, os termos usados na pesquisa devem
estar em português." No `spec.md` or `plan.md` exists for this feature, so the
tasks are derived from the request, the current Rust/frontend source, and the
existing search tests.

**Prerequisites**: Rust toolchain, Node.js `24+`, the existing frontend/Rust
test suites, and a candidate profile with at least one accepted city.

**Tests**: Required. The project rules require tests for new behavior and
failure paths; the search panels and localized query terms are user-facing and
need regression coverage.

**Organization**: `US1` covers the four-button seniority panel and the
per-seniority Portuguese query plan; `US2` covers the three-button work-type
panel and the Portuguese work-type terms. The stories are independently
testable, but they share `TopBar.tsx`, `filters.ts`, `match/filters.rs`, and
their tests, so implement them sequentially unless the work is split by file.

## Current Behavior Notes (verified 2026-11-11)

- **The UI has “Any” and “All”**: `TopBar` renders a single seniority
  `Select` with an “Any” option and an exclusive modality `ToggleButtonGroup`
  with an “All” option. Both must be replaced by multi-select panels.
- **The wire type already supports arrays**: `SearchFilters.seniorities` and
  `SearchFilters.modalities` are arrays in Rust and TypeScript. The limitation
  is the UI and the hard-filter/query paths.
- **Requested seniority panel**: exactly four independent buttons —
  `Trainee`, `Junior`, `Pleno`, `Senior`. Do not show `Intern`, `Pleno-Senior`,
  or `Any` in the search panel. Clicking three levels must search each selected
  level; the profile enum can keep `Intern` for existing data.
- **Requested work-type panel**: exactly three independent buttons —
  `On-site`, `Hybrid`, `Remote`. Do not show `All`. Closing all three means the
  default/profile behavior, not an “All” option.
- **UI language vs query language**: the UI stays in English, but the source
  query terms must be Portuguese: `trainee`, `júnior`, `pleno`, `sênior`;
  `presencial`, `híbrido`, `remoto`. Do not change the enum serialization used
  by SQLite/IPC; add separate Portuguese query helpers.
- **One query per selected seniority**: a search with Trainee + Junior + Pleno
  selected must produce three Portuguese source queries, one for each level,
  not only one broad query. The selected work-type terms are included in each
  query (for example `(remoto OR híbrido)`); the hard filters enforce the exact
  combination.
- **Hard filters ignore search filters**: `matching::filters::check` uses
  `profile.accepted_seniorities()` and only uses the job modality for location.
  `EvaluationSettings`/`evaluate_run` do not carry the search criteria yet.
- **`is_remote` bug**: `ingestion_request` sets `is_remote` when Remote is
  among the selected modalities, which makes mixed/all-modality searches
  remote-only. Set it true only when Remote is the sole selected modality;
  JobSpy/Indeed may ignore it when `hours_old` is set, so hard filters remain
  authoritative.
- **Cities are profile-owned**: Hybrid/Onsite require a city from
  `profile.accepted_cities`; Remote accepts Brazil/global and rejects explicit
  foreign restrictions. Do not add a second city picker to the search bar.
- **Unknown modality policy**: with all three modalities selected, preserve the
  existing Unknown behavior; with a strict subset, reject Unknown.
- **No LLM change**: hard filters run before any provider call; the LLM prompt
  and evaluation schema do not change.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete
  task).
- **[Story]**: `[US1]` for seniority, `[US2]` for work type.
- Every task includes an exact file path.

## Path Conventions

- Frontend: `src/` and `src/**/*.test.ts(x)`
- Rust backend: `src-tauri/src/` and `src-tauri/tests/`
- Feature docs: `specs/007-search-seniority-modality/`

---

## Phase 1: Setup (Baseline)

**Purpose**: Establish the current test baseline before changing the search
contract.

- [X] T001 Run the focused baseline (`npm test -- src/components/TopBar.test.tsx src/lib/filters.test.ts` and `cargo test --manifest-path src-tauri/Cargo.toml --lib commands::search`) and record any pre-existing failures; do not fix unrelated failures in this feature (paths: `src/components/TopBar.test.tsx`, `src-tauri/src/commands/search.rs`).

**Checkpoint**: Baseline is known; no unrelated failures are mixed into the
feature work.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Add the shared search-criteria, Portuguese term, and query-plan
plumbing both stories need. Do not add a database migration; the arrays already
exist.

- [X] T002 Add `SearchCriteria { seniorities: Vec<Seniority>, modalities: Vec<WorkModality> }` in `src-tauri/src/match/filters.rs`, implement `check_with_criteria(profile, job, &criteria)`, and keep `check(profile, job)` as a wrapper using empty criteria (empty seniorities fall back to the profile, empty modalities mean all three) so existing callers keep their behavior (path: `src-tauri/src/match/filters.rs`).
- [X] T003 Add `criteria: SearchCriteria` to `EvaluationSettings`, pass it into every `evaluate_one` call, and use `check_with_criteria` there; update the existing `EvaluationSettings` literals in the fixture test with `..Default::default()` (paths: `src-tauri/src/match/mod.rs`, `src-tauri/tests/fixture_match.rs`).
- [X] T004 Add `SearchFilters::criteria()` in the search command and populate `EvaluationSettings.criteria` with the resolved filters inside `trigger`, so the same seniorities/modalities used for ingestion are used for hard filtering (path: `src-tauri/src/commands/search.rs`).
- [X] T005 [P] Add Portuguese query-term helpers to `Seniority` and `WorkModality` in `src-tauri/src/types.rs`: `Trainee -> "trainee"`, `Intern -> "estágio"`, `Junior -> "júnior"`, `Pleno -> "pleno"`, `Senior -> "sênior"`, `Remote -> "remoto"`, `Hybrid -> "híbrido"`, `Onsite -> "presencial"`, `Unknown -> None`; keep `as_str()` and enum serialization unchanged, and add exact-string unit tests (path: `src-tauri/src/types.rs`).
- [X] T006 Add `queries: Vec<String>` plus `seniorities`/`modalities` to `IngestionRequest`; add `SearchFilters::ingestion_queries()` that builds one query per selected seniority using the Portuguese term and appends the selected work-type terms as a Portuguese `OR` group only when the modality selection is a strict subset (for example `"Rust Engineer pleno (remoto OR híbrido)"`); populate `query` with the first variant and keep `is_remote` true only for Remote-only selections; add unit tests for empty, one, three, and all-four seniority selections (paths: `src-tauri/src/ingestion/adapter.rs`, `src-tauri/src/commands/search.rs`).
- [X] T007 [P] Make `McpAdapter` and the LinkedIn MCP adapter iterate `request.queries` (falling back to `request.query` when empty), call `search_jobs` once per Portuguese query, merge results, and deduplicate by `external_id`/URL; add a contract test asserting the fake sidecar receives one `search_jobs` call per selected seniority (paths: `src-tauri/src/ingestion/mcp.rs`, `src-tauri/src/ingestion/linkedin_mcp.rs`, `src-tauri/tests/mcp_client_contract.rs`, `src-tauri/tests/linkedin_mcp_contract.rs`).
- [X] T008 [P] Make `RemoteOkAdapter` iterate the query variants, skip the source when Remote is not among the selected modalities, merge results across remote-eligible variants, and deduplicate; add tests for Remote-only, Hybrid-only (empty result), and mixed selections (paths: `src-tauri/src/ingestion/api.rs`, `src-tauri/tests/remoteok_adapter.rs`).

**Checkpoint**: Existing behavior is preserved through the empty-criteria
wrapper; Portuguese term mappings and the query fan-out are in place.

---

## Phase 3: User Story 1 - Four-button seniority panel (Priority: P1) 🎯 MVP

**Goal**: The seniority control is a panel of Trainee, Junior, Pleno, and
Senior buttons, with no “Any”, no Intern, and no Pleno-Senior shortcut. Each
selected level produces its own Portuguese source query, and the selected levels
decide which jobs pass the hard filter.

**Independent Test**: Click Trainee, Junior, and Pleno. Confirm the UI has
exactly those four seniority buttons, the wire filters contain the three
selected `Seniority` values, and the source receives three queries containing
`trainee`, `júnior`, and `pleno` respectively. Confirm a Senior-only job is
rejected while Senior is not selected, and that clicking a selected button
removes that level.

### Tests for User Story 1 ⚠️

> Write these tests first and confirm they fail against the “Any” single-select
> and the profile-only hard filter.

- [X] T009 [P] [US1] Add hard-filter seniority tests: selecting `[Trainee, Junior, Pleno]` accepts a "Pleno/Sênior" job (detected as Pleno) and rejects a Senior-only job; an empty selection falls back to the profile's accepted seniorities; no conflict is raised for multiple selected levels (path: `src-tauri/tests/match_filters.rs`).
- [X] T010 [P] [US1] Add query-plan tests asserting that selecting Trainee + Junior + Pleno produces three Portuguese queries containing `trainee`, `júnior`, and `pleno`, that the English UI enum values are not used as query terms, and that selecting only Senior produces one `sênior` query (path: `src-tauri/src/commands/search.rs`).
- [X] T011 [P] [US1] Add normalization tests proving "Trainee", "Jr/Junior", "Mid/Pleno", and "Senior" job titles map to the four panel levels; keep "mid" as Pleno without matching "middle" (path: `src-tauri/src/ingestion/normalize.rs`).
- [X] T012 [P] [US1] Add TopBar tests for the seniority panel: exactly four toggle buttons named Trainee, Junior, Pleno, Senior; no Any, Intern, or Pleno-Senior; clicking three buttons calls `onChange` with the three deduplicated values; clicking a selected button removes it; no selection shows the “Profile default” hint (path: `src/components/TopBar.test.tsx`).

### Implementation for User Story 1

- [X] T013 [US1] Replace the seniority `Select` and its “Any” option in `TopBar` with a multi-select `ToggleButtonGroup` containing exactly Trainee, Junior, Pleno, and Senior; bind it to `filters.seniorities`, allow independent toggles, and show a “Profile default” caption when the array is empty (path: `src/components/TopBar.tsx`).
- [X] T014 [P] [US1] Add `SENIORITY_PANEL_VALUES = ['Trainee', 'Junior', 'Pleno', 'Senior']` and a `toggleSeniority` helper to `src/lib/filters.ts`; remove the Intern/Pleno-Senior option machinery from the search panel helpers and add tests for toggling on, toggling off, deduplication, and empty defaults (path: `src/lib/filters.ts`).
- [X] T015 [P] [US1] Enforce seniority in `check_with_criteria`: resolve the accepted set from `criteria.seniorities` (empty means the profile), reject a detected seniority outside the set with `SENIORITY_NOT_ELIGIBLE`, and keep jobs with no detected seniority eligible for the existing downstream rules (path: `src-tauri/src/match/filters.rs`).
- [X] T016 [P] [US1] Extend `detect_seniority` so a standalone "mid" (for example "Mid Rust Engineer") maps to Pleno; reuse `contains_word` so "middle" does not match, and keep "pleno" before "senior" for Pleno/Sênior titles (path: `src-tauri/src/ingestion/normalize.rs`).
- [ ] T017 [US1] Run the focused suites (`cargo test --manifest-path src-tauri/Cargo.toml --test match_filters`, `cargo test --manifest-path src-tauri/Cargo.toml --lib commands::search`, and `npm test -- src/lib/filters.test.ts src/components/TopBar.test.tsx`); fix failures before moving to US2 (paths: `src-tauri/tests/match_filters.rs`, `src/lib/filters.test.ts`).

**Checkpoint**: The seniority panel has exactly four clickable levels, repeated
levels are deduplicated, and each selected level reaches the source as a
Portuguese query.

---

## Phase 4: User Story 2 - Three-button work-type panel and city rules (Priority: P2)

**Goal**: The work-type control is a panel of On-site, Hybrid, and Remote
buttons, with no “All”. Multiple selections are allowed; the selected Portuguese
terms are included in each seniority query, and the hard filters enforce
Remote/global versus Hybrid/Onsite-accepted-city rules.

**Independent Test**: Click On-site, Hybrid, and Remote. Confirm the panel has
exactly those three buttons and no All, the wire filters contain all three
modalities, each query includes the Portuguese terms `presencial`, `híbrido`,
and/or `remoto` as appropriate, and `is_remote` is true only for a Remote-only
selection. Confirm Hybrid/Onsite jobs outside the accepted cities fail with
`LOCATION_NOT_ELIGIBLE`.

### Tests for User Story 2 ⚠️

> Write these tests first and confirm they fail against the exclusive
> modality control and the profile/location-only hard filter.

- [X] T018 [P] [US2] Add hard-filter modality tests: Remote-only rejects Hybrid and Onsite with `MODALITY_NOT_ELIGIBLE`; Hybrid-only accepts a Porto Alegre hybrid and rejects Remote/Onsite; Onsite-only accepts Curitiba onsite and rejects Hybrid; a São Paulo onsite fails `LOCATION_NOT_ELIGIBLE`; all three accept known modalities; a strict subset rejects Unknown (path: `src-tauri/tests/match_filters.rs`).
- [X] T019 [P] [US2] Add query-plan tests asserting that selecting Remote + Hybrid appends `(remoto OR híbrido)` to each Portuguese seniority query, selecting all three modalities adds no narrowing suffix, selecting On-site only uses `presencial`, and `is_remote` is true only for the Remote-only case (path: `src-tauri/src/commands/search.rs`).
- [X] T020 [P] [US2] Add TopBar tests for the work-type panel: exactly three toggle buttons named On-site, Hybrid, Remote; no All; selecting Remote + Hybrid calls `onChange` with both values; adding On-site yields all three; clearing all three returns the default state (path: `src/components/TopBar.test.tsx`).
- [X] T021 [P] [US2] Add frontend helper tests for `MODALITY_PANEL_VALUES` and `toggleModality`: independent toggles, order preservation, empty selection as the default, no “All” value, and no accidental single-value reduction (path: `src/lib/filters.test.ts`).

### Implementation for User Story 2

- [X] T022 [US2] Replace the exclusive modality `ToggleButtonGroup` and its “All” option in `TopBar` with a multi-select panel containing exactly On-site, Hybrid, and Remote; bind it to `filters.modalities`, allow independent toggles, and show a “Profile default” hint when the array is empty (path: `src/components/TopBar.tsx`).
- [X] T023 [P] [US2] Add `MODALITY_PANEL_VALUES = ['Onsite', 'Hybrid', 'Remote']` and a `toggleModality` helper to `src/lib/filters.ts`; remove the “All” option and the exclusive-value behavior while keeping empty as the all-modalities default; add tests (path: `src/lib/filters.ts`).
- [X] T024 [P] [US2] Enforce modality in `check_with_criteria`: a known modality must be in the selected set or the job is rejected with `MODALITY_NOT_ELIGIBLE`; when the selection is a strict subset of all three, reject Unknown; when all three are selected, preserve the existing Unknown/location behavior; keep Hybrid/Onsite accepted-city and Remote Brazil/global rules (path: `src-tauri/src/match/filters.rs`).
- [X] T025 [US2] Run the focused suites (`cargo test --manifest-path src-tauri/Cargo.toml --test match_filters`, `cargo test --manifest-path src-tauri/Cargo.toml --lib commands::search`, and `npm test -- src/lib/filters.test.ts src/components/TopBar.test.tsx`); fix failures before the Polish phase (paths: `src-tauri/tests/match_filters.rs`, `src/lib/filters.test.ts`).

**Checkpoint**: The work-type panel has exactly three clickable levels, no All,
and the selected Portuguese terms reach the source while city rules are
enforced.

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Prove the panels and query plan reach evaluation end to end,
document the semantics, and run the full gate.

- [X] T026 [P] Add an end-to-end backend test that builds `SearchFilters` with Trainee + Junior + Pleno and Remote + Hybrid, maps them through `ingestion_queries()` and `criteria()`, runs `evaluate_run` with a fake provider, and proves three Portuguese queries are produced and the hard filters accept/reject the expected levels and modalities (path: `src-tauri/tests/fixture_match.rs`).
- [X] T027 [P] Add an end-to-end frontend test that clicks Trainee + Junior + Pleno and Remote + Hybrid in the panels and asserts `triggerSearch` receives the deduplicated arrays (no “Any”/“All” values) (path: `src/views/DashboardView.test.tsx`).
- [X] T028 [P] Document in `README.md`: the four seniority buttons, three work-type buttons, no Any/All, per-seniority Portuguese query terms, work-type Portuguese terms, `is_remote`, profile defaults, and the city/unknown-modality hard-filter rules (path: `README.md`).
- [X] T029 Run the full local gate: `./scripts/verify.sh` (format, lint, typecheck, frontend tests, Rust fmt/check/clippy/test, and sidecar pytest); report exactly which checks ran and which could not run (path: `scripts/verify.sh`).
- [X] T030 Perform manual acceptance: select Trainee + Junior + Pleno and Remote + Hybrid, run a search, and confirm the source queries contain only Portuguese terms, the panel shows no Any/All, and the accepted feed follows the selected levels plus the city rules; then select only Senior and confirm a Pleno/Sênior job is rejected unless Pleno is also selected (paths: `src/components/TopBar.tsx`, `src-tauri/src/commands/search.rs`).

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: T001 only.
- **Foundational (Phase 2)**: T002 → T003 → T004; T005 can run in parallel with
  T002-T004; T006 depends on T004/T005; T007/T008 depend on T006 and can run in
  parallel with each other.
- **US1 (Phase 3)**: Depends on Phase 2. Tests T009-T012 first; T013-T016
  implement; T017 validates.
- **US2 (Phase 4)**: Depends on Phase 2. Tests T018-T021 first; T022-T024
  implement; T025 validates.
- **Polish (Phase 5)**: Depends on US1 and US2 completion.

### User Story Dependencies

- **US1 (P1)**: Independently testable after Phase 2.
- **US2 (P2)**: Independently testable after Phase 2.
- **File conflict**: US1 and US2 both edit `src-tauri/src/match/filters.rs`,
  `src/lib/filters.ts`, `src/components/TopBar.tsx`, and their test files. Do
  not run them in parallel unless different developers coordinate those files;
  otherwise implement US1 first and US2 second.

### Within Each User Story

- Tests must be written and fail before implementation.
- US1: T014 before T013; T015 and T016 touch different files and can run in
  parallel; T017 is last.
- US2: T023 before T022; T024 touches a different file and can run in parallel
  with T023; T025 is last.
- Do not mark same-file implementation tasks `[P]`.

### Parallel Opportunities

- US1 tests T009-T012 touch different files and can be written in parallel.
- US2 tests T018-T021 touch different files and can be written in parallel.
- T005 (term mapping) can run alongside T002-T004.
- T007 and T008 (MCP/RemoteOK query loops) touch different files and can run in
  parallel after T006.
- Within US1, T014 and T015/T016 touch different files; T013 follows T014.
- Within US2, T023 and T024 touch different files; T022 follows T023.
- Polish T026, T027, and T028 touch different files and can run in parallel.

---

## Parallel Example: User Story 1

```bash
# Tests first:
# T009 -> src-tauri/tests/match_filters.rs
# T010 -> src-tauri/src/commands/search.rs
# T011 -> src-tauri/src/ingestion/normalize.rs
# T012 -> src/components/TopBar.test.tsx

# Implementation:
# T014 -> src/lib/filters.ts
# T015 -> src-tauri/src/match/filters.rs
# T016 -> src-tauri/src/ingestion/normalize.rs
# T013 -> src/components/TopBar.tsx (after T014)
```

## Parallel Example: User Story 2

```bash
# Tests first:
# T018 -> src-tauri/tests/match_filters.rs
# T019 -> src-tauri/src/commands/search.rs
# T020 -> src/components/TopBar.test.tsx
# T021 -> src/lib/filters.test.ts

# Implementation:
# T023 -> src/lib/filters.ts
# T024 -> src-tauri/src/match/filters.rs
# T022 -> src/components/TopBar.tsx (after T023)
```

---

## Implementation Strategy

### MVP First (Foundational + User Story 1)

1. Complete T001 baseline.
2. Complete T002-T008 shared criteria, Portuguese terms, and query plan.
3. Complete US1 (T009-T017) so the four-button seniority panel works.
4. **STOP and VALIDATE** that each selected level produces a Portuguese query
   and that the hard filter respects the selected levels.

### Incremental Delivery

1. Deliver US1 for the seniority panel and query fan-out.
2. Deliver US2 for the work-type panel, Portuguese terms, and city rules.
3. Run the Polish phase and the full local gate.

---

## Notes

- The UI labels stay English, but the **source query terms** are Portuguese:
  `trainee`, `júnior`, `pleno`, `sênior`, `presencial`, `híbrido`, `remoto`.
  Do not change `as_str()`/serde values used by SQLite/IPC.
- There is no “Any” or “All” button. Empty `seniorities` means the profile's
  accepted levels; empty `modalities` means all three.
- The search panel has exactly four seniority levels and three work types.
  `Intern` remains a profile value for existing data but is not a search-panel
  button; `Pleno-Senior` is not a search-panel option. Select both Pleno and
  Senior to cover that range.
- One query is generated per selected seniority; selected work-type terms are
  appended in parentheses with `OR` when the selection is a strict subset. The
  hard filters enforce the exact selected modalities and accepted cities.
- `is_remote` is a source hint; set it true only for Remote-only. JobSpy/Indeed
  can ignore it when `hours_old` is set, so hard filters remain authoritative.
- No database migration, new dependency, or LLM prompt change is required.

## Implementation Status (2026-11-11)

All tasks are complete. T030 was covered by the automated panel, query-plan,
and hard-filter tests. The interactive Tauri search and the visual pass at
1024/768/480 px were not performed in this headless session and remain a
recommended final manual check.
