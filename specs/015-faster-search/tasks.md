---
description: 'Task list for the faster-search pipeline (known-posting skip, configurable evaluation concurrency, progress events, source budget, LinkedIn fallback)'
---

# Tasks: Faster Search

**Input**: Design documents from `/specs/015-faster-search/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/`, `quickstart.md`

**Tests**: Required. `AGENTS.md` §10 mandates a test for every behavior change, including the
failure path. Test tasks are included per story.

**Organization**: Tasks are grouped by user story so each story is independently implementable and
testable.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: `[US1]`–`[US5]`, mapping to `spec.md`
- Every task names an exact file path

## Path Conventions

- Backend: `src-tauri/src/{commands,config,db,ingestion,match}/`
- Rust tests: `src-tauri/tests/`
- Frontend: `src/{components,lib,viewmodels,views}/`
- Feature docs: `specs/015-faster-search/`

---

## Phase 1: Setup

**Purpose**: Configuration surface for the new limits, fallback keys, and persisted concurrency.

- [ ] T001 Add `budget_seconds: u64` to `RateLimitConfig` (default `180`, clamp `30..=900`) and `fallback_for: Option<String>` plus the `fallback_min_results` reader (default `1`, clamp `1..=50`) to `SourceConfig` in `src-tauri/src/config/sources.rs`; update the hard-coded `default_sources()` entries and every rate-limit literal.
- [ ] T002 [P] Validate the fallback graph in `src-tauri/src/config/sources.rs`: reject a self-reference, an unknown target, two sources targeting the same primary, and a fallback that is the primary of another fallback; add unit tests for each rejection and for a valid pair.
- [ ] T003 [P] Add `evaluation_concurrency: usize` to `ProviderSettings` in `src-tauri/src/config/provider_settings.rs`, resolved from `app_settings["evaluation.concurrency"]` → `AppConfig.max_concurrency` → `4`, clamped `1..=8`; add tests for precedence, invalid values, and the fallback path.
- [ ] T004 [P] Add `SearchProgress`, `SearchStage`, and `evaluationConcurrency` to `src/lib/types.ts`, and extend `ProviderConfigView`/`ProviderConfigInput` with `evaluationConcurrency: number`; keep the shapes identical to `contracts/progress-event.md`.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared plumbing every user story depends on.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [ ] T005 Add `KnownPosting`, `FetchOutcome`, and `IngestionRequest.known_postings` to `src-tauri/src/ingestion/adapter.rs`; add a `FetchOutcome::from_jobs` helper that sets `listed = jobs.len()` and empty `known_skipped`; update `Default`.
- [ ] T006 [P] Change `SourceAdapter::fetch` in `src-tauri/src/ingestion/adapter.rs` to return `Result<FetchOutcome, SourceError>`, then update `src-tauri/src/ingestion/mcp.rs`, `src-tauri/src/ingestion/api.rs`, and every test fake under `src-tauri/tests/` to wrap their `Vec<RawJob>` with `FetchOutcome::from_jobs`.
- [ ] T007 [P] Extend `SourceOutcome` in `src-tauri/src/ingestion/mod.rs` with `listed: u32`, `known_skipped: Vec<KnownPosting>`, and `fallback_skipped: bool`; update `jobs()`, `failures()`, and `total_fetched()`; add `delivered()` and unit tests for the counts and the ignored fallback.
- [ ] T008 [P] Add `known_postings` and `touch_source_lineage` to `src-tauri/src/db/jobs.rs`: the global `external_id LIKE 'linkedin-%'` query joined to non-empty `description_clean`, deduplicated by `external_id`, and an UPSERT that updates `job_posting_sources.ingested_at` without creating a duplicate row; add tests for the prefix filter, the empty-description exclusion, and the repeated touch.
- [ ] T009 [P] Add `ProgressSink` and `EvaluationProgress` to `src-tauri/src/match/mod.rs` and thread an optional sink through `evaluate_run` without changing its result shape; add a counting-closure test that sees one update per completed job.
- [ ] T010 [P] Add `fetch_with_budget(adapter, request, seconds)` to `src-tauri/src/ingestion/mod.rs` that maps `tokio::time::timeout` elapsed to `SourceFailure { code: "SOURCE_TIMEOUT" }`; add a unit test with a fake adapter that never resolves and one that resolves before the budget.

**Checkpoint**: Configuration, adapter outcome, known lookup, progress sink, and the budget helper exist.

---

## Phase 3: User Story 1 - Reuse postings already collected (Priority: P1) 🎯 MVP

**Goal**: The LinkedIn adapter skips detail requests for postings already stored with a non-empty
description, still records lineage, and leaves scores and filters unchanged.

**Independent Test**: Run the same search twice; the second run performs no `get_job_details` calls
for postings already stored with a description, still updates their lineage, and fetches details
for new or description-less postings.

### Tests for User Story 1

> Write these tests first; they must fail before T015.

- [ ] T011 [P] [US1] Add `src-tauri/tests/ingestion_skip.rs` covering a known posting with a non-empty description: zero detail calls, `listed` includes it, `known_skipped` contains its ID, lineage `ingested_at` advances, and `description_clean` is unchanged.
- [ ] T012 [P] [US1] Add a case to `src-tauri/tests/ingestion_skip.rs` where an unknown ID and a known ID with an empty description both trigger `get_job_details`; assert the empty description is filled in.
- [ ] T013 [P] [US1] Add a case to `src-tauri/tests/ingestion_skip.rs` where a `linkedin-<id>` row was stored under the JobSpy source; assert the global lookup satisfies the LinkedIn adapter, the detail is skipped, and the LinkedIn lineage is added without a duplicate.
- [ ] T014 [P] [US1] Add a call-counting fake server case to `src-tauri/tests/linkedin_mcp_contract.rs`: a known ID produces zero `get_job_details` calls and an unknown ID produces exactly one.

### Implementation for User Story 1

- [ ] T015 [US1] Implement the skip in `src-tauri/src/ingestion/linkedin_mcp.rs`: normalize each search ID to `linkedin-<id>`, build a `HashSet` from `request.known_postings`, skip `get_job_details` for known IDs, count `listed` and `known_skipped`, return `FetchOutcome`, and change `LINKEDIN_DETAILS_FAILED` to fire only when `jobs.is_empty() && known_skipped.is_empty() && detail_failures > 0`.
- [ ] T016 [US1] In `src-tauri/src/commands/search.rs`, touch lineage for every `known_skipped` entry with `touch_source_lineage`, keep `total_ingested` counting new postings only, and log the `skipped_known` count without IDs, URLs, or descriptions.
- [ ] T017 [P] [US1] Load the known set in `src-tauri/src/commands/search.rs` before `ingest` via `db::jobs::known_postings`, assign it to `IngestionRequest.known_postings`, and keep the database lock out of every `.await`.
- [ ] T018 [P] [US1] Confirm `src-tauri/tests/score_contract.rs` and `src-tauri/tests/fixture_match.rs` pass unchanged; do not edit their expectations.

**Checkpoint**: Repeat runs skip known LinkedIn details and keep identical outcomes.

---

## Phase 4: User Story 2 - Evaluate postings concurrently and configurable (Priority: P2)

**Goal**: Evaluation honors a persisted concurrency between 1 and 8, adjustable from the provider
settings, applied on the next run without restarting.

**Independent Test**: Seed more postings than the limit and confirm no more than N provider calls
are in flight, N is reached when enough jobs are queued, and the outcomes match the serialized run.

### Tests for User Story 2

- [ ] T019 [P] [US2] Add `src-tauri/tests/ingestion_concurrency.rs` with a fake provider that counts in-flight calls; assert the observed maximum never exceeds `settings.concurrency`, equals it when enough jobs are queued, and the final evaluations are unchanged.
- [ ] T020 [P] [US2] Add cases to `src-tauri/tests/ingestion_concurrency.rs` proving a rate-limit error on one job preserves completed evaluations and that `0` clamps to `1` and `99` clamps to `8`.
- [ ] T021 [P] [US2] Add precedence and round-trip tests in `src-tauri/src/config/provider_settings.rs` and `src-tauri/src/commands/config.rs`: persisted `evaluation.concurrency` wins, the env default is second, invalid values clamp, and `cmd_save_provider_config` returns the saved value.
- [ ] T022 [P] [US2] Add a case to `src/components/ProviderConfigDialog.test.tsx`: the field loads, rejects values outside `1..=8`, saves the value, and the existing draft fallback still works.

### Implementation for User Story 2

- [ ] T023 [US2] Change `src-tauri/src/config/env.rs` to default `JOBS_SEARCH_LLM_CONCURRENCY` to `4` and accept `1..=8`; update the tests that assert the old bounds.
- [ ] T024 [US2] In `src-tauri/src/match/mod.rs`, clamp `EvaluationSettings::concurrency` to `1..=8`, keep `buffer_unordered(settings.concurrency)`, and emit one `EvaluationProgress` update per completed job through the sink.
- [ ] T025 [US2] In `src-tauri/src/commands/search.rs`, resolve `ProviderSettings` at the start of the run and build `EvaluationSettings` from `settings.evaluation_concurrency` instead of `config.max_concurrency`.
- [ ] T026 [US2] In `src-tauri/src/commands/config.rs`, add `evaluation_concurrency` to `ProviderConfigView` and `ProviderConfigInput`, validate `1..=8`, persist it under `app_settings["evaluation.concurrency"]`, and return it from `build_view`.
- [ ] T027 [US2] Add the global numeric field to `src/components/ProviderConfigDialog.tsx` with an accessible label, helper text, `1..=8` validation, and save wiring; update `src/lib/types.ts` and `src/lib/ipc.ts` if the shape drifted.
- [ ] T028 [P] [US2] Update `src/test/fixtures.ts` (and any provider-config mock) with `evaluationConcurrency` so the dialog tests and `cmd_get_provider_config` mocks stay type-correct.

**Checkpoint**: US1 and US2 work independently.

---

## Phase 5: User Story 3 - See progress while the run is working (Priority: P3)

**Goal**: The UI shows the current stage and aggregate counts while a run is active, then clears.

**Independent Test**: Start a run with many postings and confirm the interface reports the stage
and counts before the run ends, then clears.

### Tests for User Story 3

- [ ] T029 [P] [US3] Add `src-tauri/tests/search_progress.rs` asserting the emitted sequence starts with `collecting`, transitions once to `evaluating`, has non-decreasing counts, ends with a final event before the command resolves, and handles an empty result set.
- [ ] T030 [P] [US3] Add `src/viewmodels/useSearchProgress.test.ts` proving the hook subscribes when active, updates on event, unsubscribes on inactive/unmount, returns `null` on subscription failure, and ignores malformed payloads.
- [ ] T031 [P] [US3] Add a `src/views/NewJobsView.test.tsx` case for the progress line rendering while searching, showing `Collecting…`/`Evaluating…`, and disappearing when the run ends.

### Implementation for User Story 3

- [ ] T032 [US3] Emit `search://progress` from `src-tauri/src/commands/search.rs` using `AppHandle::emit`, coalesced to at most one event per 250 ms plus stage transitions and one final event; counters are plain integers owned by the run.
- [ ] T033 [US3] Build the collection side of the sink in `src-tauri/src/ingestion/mod.rs`: add to `found` as each source or fallback finishes, add to `failed` for each failure, and never read the database inside the sink.
- [ ] T034 [US3] Implement `src/viewmodels/useSearchProgress.ts` against `listen()` from `@tauri-apps/api/event`, with cleanup and the `null` behavior from `contracts/progress-event.md`.
- [ ] T035 [US3] Wire the hook in `src/views/DashboardView.tsx` while `searching` is true and render the aggregate progress line in `src/views/NewJobsView.tsx`; clear it when the run resolves.
- [ ] T036 [P] [US3] Verify `src-tauri/capabilities/default.json` grants event listening with a real `listen()` test; add `core:event:default` only if the existing `core:default` is insufficient.

**Checkpoint**: US1–US3 work independently.

---

## Phase 6: User Story 4 - Bound a slow source (Priority: P4)

**Goal**: One unresponsive source cannot hold the run hostage.

**Independent Test**: Configure a source that never responds and confirm the run finishes within
the budget plus overhead, reporting that source as a failure and keeping the others' postings.

### Tests for User Story 4

- [ ] T037 [P] [US4] Add `src-tauri/tests/ingestion_budget.rs` where one adapter never resolves; assert it becomes `SOURCE_TIMEOUT` after its budget, the other adapter's postings are kept, the run completes, and a drop guard proves the timed-out future was dropped.
- [ ] T038 [P] [US4] Add unit tests in `src-tauri/src/config/sources.rs` for the `budget_seconds` default `180`, the clamp `30..=900`, and an explicit override.

### Implementation for User Story 4

- [ ] T039 [US4] Wire `fetch_with_budget` into `src-tauri/src/ingestion/mod.rs` so every primary and fallback fetch uses its own `budget_seconds`; a timeout produces `SOURCE_TIMEOUT` and does not abort the group.
- [ ] T040 [US4] Add `budget_seconds` to `sources.yaml` (JobSpy `300`, LinkedIn `180`, scraper `180`, RemoteOK `30`) and document the key and defaults in `README.md`.

**Checkpoint**: US1–US4 work independently.

---

## Phase 7: User Story 5 - Secondary LinkedIn source only when needed (Priority: P5)

**Goal**: The fallback source runs only when its primary fails or delivers fewer than the
configured minimum, starts as soon as its own primary decides, and merges its results.

**Independent Test**: Make the primary fail (or deliver nothing) and confirm the fallback runs;
make the primary deliver enough and confirm the fallback performs no requests.

### Tests for User Story 5

> Write these tests first; they must fail before T045.

- [ ] T041 [P] [US5] Add `src-tauri/tests/ingestion_fallback.rs` covering the matrix: primary delivers enough (fallback skipped), primary empty (fallback runs), primary below minimum (fallback runs and results merge), primary fails (fallback runs), primary disabled (fallback runs as a normal source), and both fail (both reported).
- [ ] T042 [P] [US5] Add the known-only case to `src-tauri/tests/ingestion_fallback.rs`: a primary with `jobs = []` and `known_skipped` meeting the minimum counts as delivered, and the fallback performs zero requests.
- [ ] T043 [P] [US5] Add a timing case to `src-tauri/tests/ingestion_fallback.rs` proving the fallback starts before an unrelated slow primary finishes (use `tokio::time::pause` or a channel).
- [ ] T044 [P] [US5] Add a case in `src-tauri/tests/ingestion_fallback.rs` or `src-tauri/src/commands/search.rs` tests proving a `fallback_skipped` outcome leaves `job_sources.last_run_status` unchanged.

### Implementation for User Story 5

- [ ] T045 [US5] Replace the flat `join_all` in `src-tauri/src/ingestion/mod.rs` with per-primary groups: run each primary, decide whether it delivered enough, then run its fallbacks immediately while unrelated primaries continue; merge every outcome and set `fallback_skipped` for fallbacks that did not run.
- [ ] T046 [US5] In `src-tauri/src/ingestion/mod.rs`, compute delivery as `jobs.len() + known_skipped.len()`, trigger the fallback when the primary failed or `delivered < fallback_min_results`, run a fallback with a missing/disabled primary as a normal source with a configuration warning, and never retry a failed fallback within the run.
- [ ] T047 [US5] In `src-tauri/src/commands/search.rs`, skip `set_last_run_status` for `fallback_skipped` sources and keep the per-source failure messages and the `PARTIAL`/`FAILED` run status behavior.
- [ ] T048 [P] [US5] Add `fallback_for: linkedin` and `fallback_min_results: 1` to the `linkedin_scraper` entry in `sources.yaml`, and update the example and the delivery definition in `README.md`.

**Checkpoint**: All five stories work independently.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [ ] T049 [P] Consolidate the new `sources.yaml` keys, the evaluation concurrency field, the progress line, and the known-posting behavior in `README.md`.
- [ ] T050 [P] Update `AGENTS.md` §6 (ingestion skip, fallback delivery, budget), §7 (progress hook and dialog field), and §8 (concurrency range and setting) so the rulebook matches the code.
- [ ] T051 [P] Update `specs/015-faster-search/contracts/*` and `specs/015-faster-search/data-model.md` if any shape drifted during implementation.
- [ ] T052 Run `./scripts/verify.sh` and fix every failure; report any skipped check explicitly.
- [ ] T053 [P] Run `npm run format:check`, `cargo fmt --manifest-path src-tauri/Cargo.toml --check`, and a final `cargo clippy` on the touched crates; fix formatting and warnings.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies.
- **Foundational (Phase 2)**: depends on Setup; blocks all user stories.
- **US1 (Phase 3)**: depends on Foundational (T005–T008) and is the MVP.
- **US2 (Phase 4)**: depends on Foundational (T009) and Setup (T003).
- **US3 (Phase 5)**: depends on Foundational (T009) and on US1's `listed` counts for `found`.
- **US4 (Phase 6)**: depends on Foundational (T010).
- **US5 (Phase 7)**: depends on Foundational (T007–T008), US1's delivery counts, and US4's budget helper.
- **Polish (Phase 8)**: depends on all desired stories.

### Within Each Story

- Tests before implementation for US1 and US5.
- Configuration before orchestration.
- Backend before the frontend consumer.

### Parallel Opportunities

- T002, T003, T004 in Setup.
- T006, T007, T008, T009, T010 in Foundational.
- T011–T014 (US1 tests; T011–T013 share a file, sequence them).
- T019–T022 (US2 tests; T019–T020 share a file).
- T029–T031 (US3 tests).
- T037–T038 (US4 tests).
- T041–T044 (US5 tests; they share a file, sequence them).
- T049, T050, T051, T053 in Polish.

---

## Parallel Example: Foundational

```bash
Task: "Change the adapter contract and wrap the MCP/API adapters (T006)"
Task: "Extend SourceOutcome in src-tauri/src/ingestion/mod.rs (T007)"
Task: "Add known_postings and touch_source_lineage in src-tauri/src/db/jobs.rs (T008)"
Task: "Add the progress sink in src-tauri/src/match/mod.rs (T009)"
Task: "Add fetch_with_budget in src-tauri/src/ingestion/mod.rs (T010)"
```

---

## Implementation Strategy

### MVP First (User Story 1 only)

1. Phase 1 (Setup).
2. Phase 2 (Foundational).
3. Phase 3 (US1).
4. **STOP and VALIDATE**: run the ingestion_skip and score_contract suites; confirm a repeat run
   performs zero detail requests for known postings and identical outcomes.

### Incremental Delivery

1. Setup + Foundational → plumbing ready.
2. US1 → skip known details (MVP, the largest collection win).
3. US2 → faster evaluation queue with a persisted knob.
4. US3 → progress visibility.
5. US4 → per-source time budget.
6. US5 → LinkedIn fallback (removes duplicated collection).

---

## Notes

- `[P]` means different files and no incomplete dependency.
- Never change scoring, thresholds, hard filters, or dedup semantics; `score_contract` is the
  guard.
- Never hold the `Db` mutex across an `.await`; the known set is loaded before ingestion and the
  progress counters are plain integers.
- The fallback decision is configuration-driven; do not hard-code the LinkedIn pair.
- Skipped known postings are not new ingestion; `total_ingested` counts new postings only.
- Commit one task per commit using Conventional Commits with the task ID, for example
  `feat(T015): skip known LinkedIn details`.
