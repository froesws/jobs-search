---

description: "Task list for the faster-search pipeline (parallel collection, configurable evaluation concurrency, progress events, source budget, LinkedIn fallback)"

# Tasks: Faster Search

**Input**: Design documents from `/specs/015-faster-search/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/`, `quickstart.md`

**Tests**: Required. `AGENTS.md` §10 mandates a test for every behavior change, including the
failure path. Test tasks are included per story.

**Organization**: Tasks are grouped by user story so each story is independently
implementable and testable.

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

**Purpose**: Configuration surface for the new limits and keys.

- [ ] T001 Add `fallback_for: Option<String>` and `config.fallback_min_results` (default `1`, clamped `1..=50`) to `src-tauri/src/config/sources.rs`, and add `budget_seconds: u64` to `RateLimitConfig` (default `300`, clamped `1..=900`).
- [ ] T002 [P] Validate the fallback graph at load time in `src-tauri/src/config/sources.rs`: reject a self-reference, an unknown target, and two sources targeting the same primary; add unit tests for each rejection.
- [ ] T003 [P] Add `detail_concurrency` (default `4`, clamped `1..=4`) to the adapter constructors in `src-tauri/src/ingestion/linkedin_mcp.rs` (read via `config_u32`).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared plumbing every user story depends on.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [ ] T004 Add `skipped: bool` to `SourceOutcome` in `src-tauri/src/ingestion/mod.rs` and make `jobs()`/`failures()` ignore skipped sources.
- [ ] T005 [P] Add `SearchProgress`, `SearchStage`, and `EvaluationProgress` to `src/lib/types.ts` exactly as `contracts/progress-event.md` defines them (`stage: 'collecting' | 'evaluating'`, `found`, `evaluated`, `total`, `failed`).
- [ ] T006 [P] Add a `ProgressSink` type (`Arc<dyn Fn(EvaluationProgress) + Send + Sync>`) in `src-tauri/src/match/mod.rs` and thread an optional sink through `evaluate_run` without changing its result shape.
- [ ] T007 [P] Raise the evaluation concurrency default from `2` to `4` and widen the accepted range to `1..=8` in `src-tauri/src/config/env.rs` (`JOBS_SEARCH_LLM_CONCURRENCY`) and in `EvaluationSettings::concurrency` clamping in `src-tauri/src/match/mod.rs`; update the existing config tests that assert the old bounds.
- [ ] T008 Add a helper in `src-tauri/src/ingestion/mod.rs` that runs one adapter under `tokio::time::timeout(ctx.budget_seconds)` and maps an elapsed timeout to `SourceFailure { code: "SOURCE_TIMEOUT" }`, with a unit test using a fake adapter that never resolves.

**Checkpoint**: Configuration, skip semantics, progress types, concurrency bounds, and the budget helper exist.

---

## Phase 3: User Story 1 - Collect postings in parallel (Priority: P1) 🎯 MVP

**Goal**: Detail fetches inside a source run concurrently, bounded by `detail_concurrency`, with
identical results and isolated failures.

**Independent Test**: Run a fake source returning 20 postings and verify multiple detail
requests are in flight, the returned postings are identical to the serialized run, and one
failing detail does not drop the others.

### Tests for User Story 1

> Write these tests first; they must fail before T010.

- [ ] T009 [P] [US1] Add `src-tauri/tests/ingestion_concurrency.rs` with a fake adapter that resolves details out of order and records the maximum in-flight count; assert the count is greater than 1 and at most `detail_concurrency`, and that all postings are returned.
- [ ] T010 [P] [US1] Add a failure-isolation case to `src-tauri/tests/ingestion_concurrency.rs`: one detail fails, the rest succeed, and the source still returns the successful postings with the failure counted.

### Implementation for User Story 1

- [ ] T011 [US1] Replace the sequential `get_job_details` loop in `src-tauri/src/ingestion/linkedin_mcp.rs` with `futures::stream::iter(...).buffer_unordered(self.detail_concurrency)` while preserving de-duplication by `external_id` and the `LINKEDIN_DETAILS_FAILED` behavior when every detail fails.
- [ ] T012 [US1] Apply the same bounded parallel pattern to the per-posting detail step in `src-tauri/src/ingestion/mcp.rs` where details are fetched client-side; leave the per-query loop sequential so rate limits stay intact.
- [ ] T013 [P] [US1] Confirm `src-tauri/tests/fixture_match.rs` and `src-tauri/tests/score_contract.rs` still pass unchanged as the proof that results are identical (FR-002).

**Checkpoint**: Collection is parallel and behavior-preserving.

---

## Phase 4: User Story 2 - Evaluate postings concurrently and configurable (Priority: P2)

**Goal**: The evaluation stage honors a configurable 1–8 concurrency and applies changes without
a restart.

**Independent Test**: Seed more postings than the limit and confirm no more than N provider calls
are in flight, and that statuses and scores match the serialized run.

### Tests for User Story 2

- [ ] T014 [P] [US2] Add `src-tauri/tests/evaluation_concurrency.rs` with a fake provider that counts concurrent calls; assert the observed maximum never exceeds `settings.concurrency` and equals it when enough jobs are queued.
- [ ] T015 [P] [US2] Add a case to `src-tauri/tests/evaluation_concurrency.rs` proving a rate-limit error on one job preserves the evaluations that already completed (FR-010).

### Implementation for User Story 2

- [ ] T016 [US2] Ensure `evaluate_run` in `src-tauri/src/match/mod.rs` uses `buffer_unordered(settings.concurrency)` with the new `1..=8` clamp and emits an `EvaluationProgress` count per completed job through the sink.
- [ ] T017 [US2] Verify the resolved settings flow end to end: `src-tauri/src/commands/search.rs` builds `EvaluationSettings` from the current config snapshot so a changed value applies to the next run without restarting (FR-004).
- [ ] T018 [P] [US2] Add a retry-free note to `src-tauri/src/config/env.rs` tests: a concurrency of `0` clamps to `1` and `99` clamps to `8`.

**Checkpoint**: US1 and US2 work independently.

---

## Phase 5: User Story 3 - See progress while the run is working (Priority: P3)

**Goal**: The UI shows the current stage and counts while a run is active.

**Independent Test**: Start a run with many postings and confirm the interface reports the stage
and counts before the run ends, then clears.

### Tests for User Story 3

- [ ] T019 [P] [US3] Add `src-tauri/tests/search_progress.rs` asserting the emitted sequence starts with `collecting`, transitions once to `evaluating`, has non-decreasing counts, and ends with a final event before the command resolves.
- [ ] T020 [P] [US3] Add `src/viewmodels/useSearchProgress.test.ts` proving the hook subscribes when active, updates on event, unsubscribes on inactive/unmount, and returns `null` if the subscription fails.

### Implementation for User Story 3

- [ ] T021 [US3] Emit `search://progress` from `src-tauri/src/commands/search.rs` using `AppHandle::emit`, coalesced to at most one event per 250 ms plus stage transitions, with counters owned by the run (no DB reads).
- [ ] T022 [US3] Build the progress sink in `src-tauri/src/commands/search.rs` and pass it to `evaluate_run`; report `found` when each source finishes and `failed` from the ingestion outcome.
- [ ] T023 [US3] Implement `src/viewmodels/useSearchProgress.ts` against `listen()` from `@tauri-apps/api/event` with the contract in `contracts/progress-event.md`.
- [ ] T024 [US3] Render the progress line in `src/views/NewJobsView.tsx` (`Collecting… N found`, `Evaluating… N of M`) and pass the value from `src/views/DashboardView.tsx`, which owns the hook while `searching` is true.
- [ ] T025 [P] [US3] Add a `NewJobsView` test in `src/views/NewJobsView.test.tsx` for the progress line rendering while searching and disappearing when the run ends.

**Checkpoint**: US1–US3 work independently.

---

## Phase 6: User Story 4 - Bound a slow source (Priority: P4)

**Goal**: One unresponsive source cannot hold the run hostage.

**Independent Test**: Configure a source that never responds and confirm the run finishes within
the budget plus overhead, reporting that source as a failure and keeping the others' postings.

### Tests for User Story 4

- [ ] T026 [P] [US4] Add a case in `src-tauri/tests/ingestion_concurrency.rs` where one adapter exceeds its budget; assert it becomes `SOURCE_TIMEOUT`, the run completes, and the other adapter's postings are returned.

### Implementation for User Story 4

- [ ] T027 [US4] Wire the budget helper from T008 into `ingest` in `src-tauri/src/ingestion/mod.rs` so every adapter is bounded by its own `budget_seconds`.
- [ ] T028 [P] [US4] Document the new `budget_seconds` key in `README.md` and in `contracts/sources-config.md` if the wording drifted.

**Checkpoint**: US1–US4 work independently.

---

## Phase 7: User Story 5 - Secondary LinkedIn source only when needed (Priority: P5)

**Goal**: The fallback source runs only when its primary fails or returns fewer than the
configured minimum.

**Independent Test**: Make the primary fail (or return nothing) and confirm the fallback runs;
make the primary succeed and confirm the fallback performs no requests.

### Tests for User Story 5

> Write these tests first; they must fail before T031.

- [ ] T029 [P] [US5] Add `src-tauri/tests/ingestion_fallback.rs` covering the matrix: primary succeeds with enough postings (fallback skipped), primary succeeds empty (fallback runs), primary returns fewer than the minimum (fallback runs and results merge), primary fails (fallback runs), and primary disabled (fallback runs as a normal source).
- [ ] T030 [P] [US5] Add the both-fail case to `src-tauri/tests/ingestion_fallback.rs`: each failure is reported individually and the run continues.

### Implementation for User Story 5

- [ ] T031 [US5] Split `ingest` in `src-tauri/src/ingestion/mod.rs` into two phases: run primaries concurrently, then run fallbacks whose primary failed or returned fewer than `fallback_min_results` (zero included); mark skipped fallbacks with `skipped: true`.
- [ ] T032 [US5] Merge a triggered fallback's postings with any the primary produced instead of replacing them, in `src-tauri/src/ingestion/mod.rs`.
- [ ] T033 [US5] Log a configuration warning and run the source as a normal primary when `fallback_for` points at a disabled or missing source, in `src-tauri/src/ingestion/mod.rs`.
- [ ] T034 [P] [US5] Annotate `linkedin_scraper` with `fallback_for: linkedin` in `sources.yaml` and update the example block in `README.md`.

**Checkpoint**: All five stories work independently.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [ ] T035 [P] Document the new keys (`fallback_for`, `fallback_min_results`, `budget_seconds`, `detail_concurrency`) and the concurrency variable in `README.md`.
- [ ] T036 [P] Update `AGENTS.md` §6 (ingestion fallback + budget) and §8 (concurrency range) so the rulebook matches the code.
- [ ] T037 Run `./scripts/verify.sh` and fix every failure; report any skipped check explicitly.
- [ ] T038 Execute `quickstart.md` scenarios 1–10 manually on a desktop session and record the results.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies.
- **Foundational (Phase 2)**: depends on Setup; blocks all user stories.
- **US1 (Phase 3)**: depends on Foundational.
- **US2 (Phase 4)**: depends on Foundational.
- **US3 (Phase 5)**: depends on Foundational and on the sink shape from US2's `evaluate_run`.
- **US4 (Phase 6)**: depends on Foundational.
- **US5 (Phase 7)**: depends on Foundational (T004 `skipped`) and T008 (budget helper).
- **Polish (Phase 8)**: depends on all desired stories.

### Within Each Story

- Tests before implementation for US1 and US5 (they encode the current behavior).
- Configuration before orchestration.
- Backend before the frontend consumer.

### Parallel Opportunities

- T002, T003 in Setup.
- T005, T006, T007 in Foundational.
- T009 and T010 (same file — sequence them, but both are test-only).
- T013, T018, T025, T028, T034, T035, T036 are independent-file tasks.
- T014 and T015 compile together; T019 and T020 are independent.

---

## Parallel Example: Foundational

```bash
Task: "Add progress types in src/lib/types.ts (T005)"
Task: "Add ProgressSink in src-tauri/src/match/mod.rs (T006)"
Task: "Raise the evaluation concurrency bounds (T007)"
```

---

## Implementation Strategy

### MVP First (User Story 1 only)

1. Phase 1 (Setup).
2. Phase 2 (Foundational).
3. Phase 3 (US1).
4. **STOP and VALIDATE**: compare collection time against the serialized baseline and confirm
   identical postings.

### Incremental Delivery

1. Setup + Foundational → plumbing ready.
2. US1 → parallel collection (MVP, biggest win for a 20-posting source).
3. US2 → faster evaluation queue.
4. US3 → progress visibility.
5. US4 → source time budget.
6. US5 → LinkedIn fallback (removes duplicated collection).

---

## Notes

- `[P]` means different files and no incomplete dependency.
- Never change scoring, thresholds, hard filters, or dedup semantics; `score_contract` is the
  guard.
- Never hold the `Db` mutex across an `.await`; progress counters are plain integers.
- The fallback decision is configuration-driven; do not hard-code the LinkedIn pair.
- Commit one task per commit using Conventional Commits with the task ID, for example
  `feat(T011): parallelize LinkedIn detail fetches`.
