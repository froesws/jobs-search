---

description: "Task list for Skill Gap Analytics & Funnel"
---

# Tasks: Skill Gap Analytics & Funnel

**Input**: Design documents from `/specs/003-skill-gap-analytics/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`,
`contracts/`, `quickstart.md`

**Tests**: Required by the plan, constitution, and project rules. Add the test
task before its implementation task and confirm it fails first.

**Organization**: Tasks are grouped by user story. Analytics is read-only over the
`001` tables and replaces the `002` Analytics shell. US2 covers metric trust,
including the Funnel and CSV requirements that make the numbers auditable. The
`002` tab shell, card grid, drawer, and profile editor are prerequisites and are
not reimplemented here.

**Current state**: No `src-tauri/src/analytics/` module, no
`src-tauri/src/commands/analytics.rs`, and no frontend `src/` exist yet. All tasks
start unchecked.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete
  task)
- **[Story]**: `[US1]`–`[US4]`
- Every task includes an exact file path.

## Path Conventions

- Backend analytics: `src-tauri/src/analytics/`, `src-tauri/src/commands/`
- Backend tests: `src-tauri/tests/`
- Frontend: `src/viewmodels/`, `src/views/`, `src/components/`, `src/lib/`
- Feature docs/contracts: `specs/003-skill-gap-analytics/`

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Analytics module skeletons, command registration, DTOs, and seed
fixtures.

- [x] T001 Create the backend analytics module skeleton `src-tauri/src/analytics/{mod.rs,skill_gap.rs,funnel.rs,csv.rs}` and declare `analytics` in `src-tauri/src/lib.rs`.
- [x] T002 Create `src-tauri/src/commands/analytics.rs`, declare it in `src-tauri/src/commands/mod.rs`, and register `cmd_get_analytics` and `cmd_export_analytics_csv` in `src-tauri/src/lib.rs`.
- [x] T003 [P] Create frontend component skeletons `src/components/SkillGapBar.tsx`, `src/components/SkillGapList.tsx`, `src/components/FunnelCards.tsx`, `src/components/ScoreDistribution.tsx`, and `src/components/LlmMetricsTable.tsx`.
- [x] T004 [P] Add analytics DTOs to `src/lib/types.ts`: `AnalyticsScope`, `AnalyticsRequest`, `AnalyticsPayload`, `SkillGapMetric`, `FunnelPayload`, `ScoreBucket`, `LlmMetricsPayload`, `ExportRequest`, and `ExportResult`.
- [x] T005 [P] Add typed IPC wrappers `getAnalytics(request)` and `exportAnalyticsCsv(request)` to `src/lib/ipc.ts` plus error mapping in `src/lib/errors.ts`.
- [x] T006 Create the deterministic analytics seed helper in `src-tauri/tests/common/analytics.rs` and register it from `src-tauri/tests/common/mod.rs`: eligible/rejected/error jobs, duplicate missing skills, profile/matched exclusions, lifecycle states, multi-source lineage, runs, and LLM metrics.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Eligibility, normalization, scope resolution, payload shape, test
harness, and the Analytics ViewModel skeleton.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [x] T007 Implement the analytics eligibility query in `src-tauri/src/analytics/mod.rs`: completed `APPROVED` or `PENDING` evaluations only, exclude `REJECTED`, `ERROR`, incomplete, stale, and deleted records; lifecycle state must not affect eligibility.
- [x] T008 Implement skill normalization in `src-tauri/src/analytics/skill_gap.rs`: trim, case-insensitive comparison, collapsed internal whitespace, documented alias handling, and a consistent display name.
- [x] T009 [P] Implement shared percentage and ranking helpers in `src-tauri/src/analytics/skill_gap.rs`: `count / denominator * 100` rounded to one decimal, count descending, display name ascending, and no division by zero.
- [x] T010 [P] Implement scope resolution in `src-tauri/src/analytics/mod.rs` and `src-tauri/src/db/runs.rs` for `CURRENT_RUN` (requires a stored `run_id`) and `LAST_7_DAYS`, with `last_updated`, `stale`, and `denominator`.
- [x] T011 [P] Define the `AnalyticsPayload` serialization in `src-tauri/src/analytics/mod.rs` with `scope`, `run_id`, `denominator`, `last_updated`, `stale`, `skill_gap`, `funnel`, `score_distribution`, and `llm_metrics`.
- [x] T012 Create the backend contract test harness in `src-tauri/tests/analytics_contract.rs` that opens an in-memory database, seeds `src-tauri/tests/common/analytics.rs`, and calls the analytics functions.
- [x] T013 Implement the `useAnalyticsViewModel` skeleton in `src/viewmodels/useAnalyticsViewModel.ts` with `idle|loading|ready|empty|no-gap|stale|error`, range state (`CURRENT_RUN`/`LAST_7_DAYS`), `refresh()`, `setScope()`, and `exportCsv()`.
- [x] T014 Replace the `002` Analytics shell in `src/views/AnalyticsView.tsx` and wire it in `src/views/DashboardView.tsx`; keep the fixed `Analytics` label and show the loading state while data loads.

**Checkpoint**: The tab renders from the typed payload, and the backend returns an empty valid payload for an empty database.

---

## Phase 3: User Story 1 - View ranked missing-skill demand (Priority: P1) 🎯 MVP

**Goal**: Aggregate missing skills from eligible evaluations, rank by demand, and
render progress bars/frequency cards.

**Independent Test**: Seed 100 eligible jobs with 50 missing `Kubernetes`; open
Analytics and verify count 50, percentage 50.0%, rank above lower-demand skills,
and no gap for profile or matched skills.

### Tests for User Story 1

- [x] T015 [P] [US1] Add the 50/100 Kubernetes test in `src-tauri/tests/analytics_contract.rs` for demand count 50 and percentage 50.0%.
- [x] T016 [P] [US1] Add ranking/normalization/count-once tests in `src-tauri/tests/analytics_contract.rs`: count descending, alphabetical ties, variants merged, and repeated entries counted once per job.
- [x] T017 [P] [US1] Add eligibility/empty/zero tests in `src-tauri/tests/analytics_contract.rs`: profile and matched-skill exclusion, `APPROVED`+`PENDING` only, lifecycle neutrality, no jobs → no-data, jobs with no gaps → zero-gap with denominator.
- [x] T018 [P] [US1] Add failing `useAnalyticsViewModel` skill-gap tests in `src/viewmodels/useAnalyticsViewModel.test.ts` for loading, populated, no-data, zero-gap, error, and scope change.
- [x] T019 [P] [US1] Add failing `src/components/SkillGapList.test.tsx` and `src/components/SkillGapBar.test.tsx` tests for rank, display name, count, one-decimal percentage, and accessible progress bars.

### Implementation for User Story 1

- [x] T020 [US1] Implement missing-skill aggregation in `src-tauri/src/analytics/skill_gap.rs`: read structured `missing_keywords_json`, exclude skills present in the active profile or in that job's matched keywords, and count each job once per skill.
- [x] T021 [US1] Implement `cmd_get_analytics` in `src-tauri/src/commands/analytics.rs` returning the ranked `skill_gap` payload with no raw text, prompts, or credentials.
- [x] T022 [US1] Implement `src/components/SkillGapBar.tsx` and `src/components/SkillGapList.tsx` with MUI LinearProgress/Card, rank, count, percentage, and a full-list/scroll path for long lists.
- [x] T023 [US1] Render the skill-gap section in `src/views/AnalyticsView.tsx` and expose the scope, denominator, and percentage definition in `src/components/SkillGapList.tsx`.
- [x] T024 [US1] Add the US1 end-to-end test in `src-tauri/tests/analytics_contract.rs` covering seed → aggregate → payload → reconciliation with the underlying missing-keyword rows.

**Checkpoint**: US1 delivers the feature's core value: ranked, explainable missing-skill demand.

---

## Phase 4: User Story 2 - Understand and trust the metrics (Priority: P2)

**Goal**: Add freshness/explainability, Funnel counts and rates, breakdowns, score
distribution, LLM metrics, and the explicit CSV export.

**Independent Test**: Change the profile, add an evaluation, and delete a job;
verify refresh, stale indication, denominator/definition visibility, funnel
reconciliation, score buckets, p50/p95 metrics, and CSV contents with no raw text
or credentials.

### Tests for User Story 2

- [x] T025 [P] [US2] Add stale/refresh tests in `src-tauri/tests/analytics_contract.rs` and `src/viewmodels/useAnalyticsViewModel.test.ts`: newer evaluation sets `stale`, refresh clears it, and last-updated is exposed.
- [x] T026 [P] [US2] Add exclusion/deletion tests in `src-tauri/tests/analytics_contract.rs`: deleted evaluations leave no demand or denominator contribution, and incomplete/error/stale evaluations are excluded.
- [x] T027 [P] [US2] Add funnel count/rate tests in `src-tauri/tests/analytics_contract.rs`: ingested, duplicates, rejected by reason, pending, approved, applied, ignored, approval rate, and application rate for both scopes.
- [x] T028 [P] [US2] Add breakdown/bucket tests in `src-tauri/tests/analytics_contract.rs`: source breakdown, provider/model breakdown, and the 0-49/50-59/60-69/70-79/80-89/90-100 score buckets.
- [x] T029 [P] [US2] Add LLM metric tests in `src-tauri/tests/analytics_contract.rs`: call/success/error/fallback counts and p50/p95 latency by provider/model.
- [x] T030 [P] [US2] Add CSV contract tests in `src-tauri/tests/analytics_contract.rs`: header order, aggregate section, one row per job, `description_clean` and URL present, `description_raw`/credentials absent, and UTF-8 quoting.
- [x] T031 [P] [US2] Add failing trust/export UI tests in `src/views/AnalyticsView.test.tsx` for scope, denominator, definition, stale indicator, refresh, sensitive-data warning, and export confirmation.

### Implementation for User Story 2

- [x] T032 [US2] Implement funnel counts and rates in `src-tauri/src/analytics/funnel.rs` from `execution_runs` and `run_metrics`; never parse log files.
- [x] T033 [US2] Implement the score distribution buckets in `src-tauri/src/analytics/funnel.rs` and expose them as `ScoreBucket`.
- [x] T034 [US2] Implement LLM call/error/fallback counts and p50/p95 latency in `src-tauri/src/analytics/funnel.rs` from `llm_metrics`, grouped by provider/model.
- [x] T035 [US2] Implement stale detection and `last_updated` in `src-tauri/src/analytics/mod.rs` using `job_evaluations.evaluated_at` and the active profile/rules version.
- [x] T036 [US2] Implement the analytics summary, percentage definition, last-updated, stale indicator, and refresh action in `src/views/AnalyticsView.tsx`.
- [x] T037 [US2] Implement `src/components/FunnelCards.tsx`, `src/components/ScoreDistribution.tsx`, and `src/components/LlmMetricsTable.tsx` for counts, rates, breakdowns, buckets, and latency/fallback.
- [x] T038 [US2] Implement RFC 4180 CSV writing in `src-tauri/src/analytics/csv.rs` with the documented stable column order and UTF-8 output.
- [x] T039 [US2] Implement `cmd_export_analytics_csv` in `src-tauri/src/commands/analytics.rs` and `src/lib/ipc.ts`: validate the operator-selected local destination, write the current view, show a sensitive-data warning, and never upload.
- [x] T040 [US2] Add the funnel/CSV end-to-end reconciliation test in `src-tauri/tests/analytics_contract.rs` covering both scopes, funnel totals, penalties, and exported rows.

**Checkpoint**: The operator can trust and export the analytics numbers without exposing raw descriptions or secrets.

---

## Phase 5: User Story 3 - Navigate the dashboard with refined views (Priority: P3)

**Goal**: Integrate Analytics into the four-tab dashboard and add the refinements:
24h/7d/30d timeframes, modality `All`, provider indicator, richer cards, and the
applied-jobs table.

**Independent Test**: Switch across all four tabs, change filters, apply a job,
return to Analytics; verify state preservation, top-bar controls and labels, card
location/modality, the applications table, and the visible provider name.

**⚠️ Depends on**: `002` Setup/Foundational and the `002` US4 applications table.

### Tests for User Story 3

- [x] T041 [P] [US3] Add failing timeframe/All tests in `src/components/TopBar.test.tsx` for `24h`, `7d`, `30d`, seniority, stack, and modality `All`.
- [x] T042 [P] [US3] Add failing provider-indicator tests in `src/components/TopBar.test.tsx` and `src/components/ProviderIndicator.test.tsx`: active provider name updates and no credentials appear.
- [x] T043 [P] [US3] Add failing card/table tests in `src/views/NewJobsView.test.tsx` and `src/views/ApplicationsView.test.tsx` for location/modality on cards, status badge, application date, sorting, and filters.
- [x] T044 [P] [US3] Add failing tab-state tests in `src/views/DashboardView.test.tsx`: filters, selected tab, and application state survive switching and never mix.

### Implementation for User Story 3

- [x] T045 [US3] Add `24h`, `7d`, and `30d` timeframe options in `src/components/TopBar.tsx` and `src/lib/filters.ts`, mapping to `SearchTimeframe.H24`, `D7`, and `D30`.
- [x] T046 [US3] Add the modality `All` option in `src/components/TopBar.tsx` and `src/lib/filters.ts` while keeping Remote/Hybrid/Onsite explicit selections.
- [x] T047 [US3] Implement the provider indicator in `src/components/ProviderIndicator.tsx` and render it in `src/components/TopBar.tsx` from `cmd_get_active_provider`.
- [x] T048 [US3] Show location or modality on `src/components/JobCard.tsx` in addition to title, company, summary, score badge, and N/M.
- [x] T049 [US3] Upgrade `src/views/ApplicationsView.tsx` to a table with a status badge, applied date, score, N/M, query, filters, sorting, and detail inspection.
- [x] T050 [US3] Centralize tab/filter/session state in `src/viewmodels/useDashboardState.ts` and `src/views/DashboardView.tsx` so navigation preserves state and keeps content isolated.
- [x] T051 [US3] Implement distinct Analytics states in `src/views/AnalyticsView.tsx`: loading, no-data, zero-gap, stale, error, and populated; keep the tab navigable while computing.
- [x] T052 [US3] Add the US3 integration test in `src/views/DashboardView.test.tsx` covering four tabs → filter change → apply → return to Analytics → preserved state.

**Checkpoint**: Analytics is a first-class tab with the refined dashboard controls, without regressing `002` behavior.

---

## Phase 6: User Story 4 - Keep analytics private and responsive (Priority: P4)

**Goal**: Meet the 1,000-record performance target, 15-second source timeout
behavior, local-only analytics, and safe storage-failure handling.

**Independent Test**: Seed 1,000 evaluated jobs, open the grid and Analytics, and
measure rendering; simulate one source timeout and verify other sources finish
within 15 seconds; inspect data paths for local-only storage and absent secrets or
raw text.

### Tests for User Story 4

- [x] T053 [P] [US4] Add the 1,000-job performance test in `src-tauri/tests/analytics_contract.rs` and `src/views/AnalyticsView.test.tsx`: summary under 100 ms, paginated/windowed rendering, and no blocking query in the render path.
- [x] T054 [P] [US4] Add the 15-second timeout/fault-isolation test in `src-tauri/tests/ingestion_pipeline.rs` (or extend the `001` test): one source times out, others complete, and the UI remains navigable.
- [x] T055 [P] [US4] Add the local-only/privacy test in `src-tauri/tests/analytics_contract.rs`: no network call, no `description_raw`, credentials, prompts, or payloads in analytics/CSV output.
- [x] T056 [P] [US4] Add the storage-failure test in `src/viewmodels/useAnalyticsViewModel.test.ts` and `src/views/AnalyticsView.test.tsx`: error state, no stale data presented as current, retry available.

### Implementation for User Story 4

- [x] T057 [US4] Keep analytics queries non-blocking in `src-tauri/src/commands/analytics.rs`: run aggregation off the UI thread, return a loading-compatible payload, and avoid blocking the Tauri command for large datasets.
- [x] T058 [US4] Implement the storage/error state in `src/viewmodels/useAnalyticsViewModel.ts` and `src/views/AnalyticsView.tsx` with a safe message and retry, without showing cached data as current.
- [x] T059 [US4] Add the sensitive-data warning and local-path validation to `src/views/AnalyticsView.tsx` and `src-tauri/src/commands/analytics.rs`; block exports outside an operator-selected local destination.
- [x] T060 [US4] Add supporting indexes (or verify existing indexes) for analytics aggregation in `src-tauri/src/db/migrations.rs` and document the query plan in `specs/003-skill-gap-analytics/research.md`.

**Checkpoint**: Analytics meets the privacy, timeout, storage-failure, and rendering targets.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, verification, manual evidence, and cleanup.

- [x] T061 [P] Sync `specs/003-skill-gap-analytics/contracts/analytics-commands.md` and `contracts/csv-export.md` with the final DTO/column names.
- [x] T062 [P] Update `README.md` with the Analytics tab, scopes, CSV export warning, and privacy model.
- [x] T063 [P] Update `specs/003-skill-gap-analytics/quickstart.md` with the seed command, actual test paths, and manual validation steps.
- [x] T064 Run `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `npm run typecheck`, `npm test`, and `./scripts/verify.sh`; fix failures and report skipped checks explicitly.
- [x] T065 [P] Add the manual seed utility `src-tauri/tests/seed_analytics.rs` (or a documented test-only helper) for 100/1,000-job validation without touching production data.
- [x] T066 [P] Audit the Analytics SQL and UI for provider-neutrality: changing the active provider must not rewrite historical analytics and must update only the indicator.
- [x] T067 Run the full `003` quickstart scenarios (50/100, exclusions, funnel, time range, empty/zero, CSV, performance) and record evidence in `specs/003-skill-gap-analytics/quickstart.md`.
- [x] T068 Add the cross-scope consistency test in `src-tauri/tests/analytics_contract.rs`: switching `CURRENT_RUN` and `LAST_7_DAYS` updates both Skill Gap and Funnel consistently with the selected scope.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: Can start immediately, but the frontend tasks depend on the `002` scaffold.
- **Foundational (Phase 2)**: Depends on Setup; blocks all stories.
- **US1 (Phase 3)**: Depends on Foundational; MVP.
- **US2 (Phase 4)**: Depends on US1 aggregation for shared payload and helpers.
- **US3 (Phase 5)**: Depends on `002` US2/US4 components; can start in parallel with US2.
- **US4 (Phase 6)**: Depends on US1/US2 payloads and the `001` ingestion behavior.
- **Polish (Phase 7)**: Depends on the desired stories.

### User Story Dependencies

- **US1**: No dependency on `001` beyond stored evaluations.
- **US2**: Needs US1's scope/normalization helpers and the `001` run/LLM metrics.
- **US3**: Needs the `002` dashboard shell and `002` US4 applications table.
- **US4**: Needs US1/US2 and the `001` source timeout behavior.

### Within Each Story

- Tests first; confirm they fail.
- Types/queries before commands; commands before ViewModels; ViewModels before Views.
- Run `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`, `npm run typecheck`, `npm test` before each commit.
- One task per commit with the task ID.

## Parallel Opportunities

- T003–T005 can run in parallel (components, DTOs, IPC).
- T015–T019 and T025–T031 are independent test files.
- T032–T034 can run in parallel (counts, buckets, LLM metrics).
- T037 and T038 can run in parallel (UI components vs. CSV writer).
- T041–T044 and T053–T056 are independent test groups.
- T061–T063, T065, and T066 are parallel polish tasks.

## Parallel Example: US2

```text
Task: T032 Implement funnel counts/rates in src-tauri/src/analytics/funnel.rs
Task: T033 Implement score buckets in src-tauri/src/analytics/funnel.rs
Task: T034 Implement p50/p95 LLM metrics in src-tauri/src/analytics/funnel.rs
```

## Implementation Strategy

### MVP (US1 only)

1. Complete Setup and Foundational (T001–T014).
2. Complete US1 skill-gap aggregation and UI (T015–T024).
3. **STOP and VALIDATE** the 50/100 Kubernetes scenario.

### Incremental delivery

1. Add US2 trust, funnel, and CSV (T025–T040).
2. Add US3 dashboard refinements (T041–T052).
3. Add US4 privacy/performance hardening (T053–T060).
4. Polish, verify, and record quickstart evidence (T061–T068).

## Notes

- Analytics is read-only; no profile, job, evaluation, or application mutation.
- Funnel values come from structured tables, never logs.
- Denominator includes eligible jobs with no missing skills; never divide by zero.
- APPLIED/IGNORED jobs remain in market demand.
- `description_raw`, credentials, prompts, and payloads never appear in responses or CSV.
- CSV export is explicit, local, and preceded by a sensitive-data warning.
- Provider changes update the indicator but never rewrite historical analytics.
