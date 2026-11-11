---

description: "Task list for Career Intelligence & Job Sourcing Core"
---

# Tasks: Career Intelligence & Job Sourcing Core

**Input**: Design documents from `/specs/001-career-job-sourcing/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`,
`contracts/`, `quickstart.md`

**Tests**: Required by the plan, constitution, and project rules. Write the test
task first for each story and confirm it fails before completing the matching
implementation task.

**Organization**: Tasks are grouped by user story. Spec priority is P1 → P4, but
the previously agreed delivery milestone is **US2 (ingestion/dedup) + US3 (match
with fixture profile)**, so those phases come first. US1 (PDF/profile) and US4
(apply/audit) follow.

**Status snapshot (2026-11-10)**: The first milestone (**T001-T052**, US2 + US3)
is implemented, tested, and committed. US1 (PDF/profile) and US4
(apply/audit/lifecycle) remain deferred. Every checked task has passing tests;
the full `./scripts/verify.sh` gate passes with the frontend checks skipped
because `002` does not exist yet.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete
  task)
- **[Story]**: `[US1]`, `[US2]`, `[US3]`, or `[US4]`
- Every task includes an exact file path.

## Path Conventions

- Rust backend: `src-tauri/src/`, `src-tauri/tests/`
- Python sidecar: `sidecar/jobspy_mcp/`
- Feature docs/contracts: `specs/001-career-job-sourcing/`

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Dependencies, module skeletons, sidecar structure, fixtures.

- [x] T001 Confirm runtime dependencies `rusqlite` (bundled SQLite), `reqwest` (rustls, json), `pdf-extract`, `sha2`, `zeroize`, `tokio`, `uuid`, and dev-dependencies `jsonschema`, `tempfile`, `wiremock` in `src-tauri/Cargo.toml`.
- [x] T002 Create `src-tauri/src/config/{mod.rs,env.rs,sources.rs}` and declare `config` in `src-tauri/src/lib.rs`.
- [x] T003 Create `src-tauri/src/db/{mod.rs,connection.rs,migrations.rs}` and declare `db` in `src-tauri/src/lib.rs`.
- [x] T004 Create module skeletons `src-tauri/src/profile/mod.rs`, `src-tauri/src/pdf/mod.rs`, `src-tauri/src/ingestion/mod.rs`, `src-tauri/src/dedup/mod.rs`, `src-tauri/src/match/mod.rs`, and `src-tauri/src/llm/mod.rs`.
- [x] T005 Create `sidecar/jobspy_mcp/requirements.txt`, `sidecar/jobspy_mcp/pyproject.toml`, `sidecar/jobspy_mcp/jobspy_mcp_server/__init__.py`, and the `sidecar/jobspy_mcp/tests/` directory; pin `python-jobspy`, `mcp`, and `pydantic` from `requirements.txt`.
- [x] T006 Create the synthetic fixture `src-tauri/tests/fixtures/profile.json` and shared test helpers in `src-tauri/tests/common/mod.rs`.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Schema, configuration, adapter contracts, identity, LLM boundary,
fixture loading, and command registration.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [x] T007 Implement SQLite connection and embedded migrations in `src-tauri/src/db/connection.rs` and `src-tauri/src/db/migrations.rs` with every `001` table, index, and constraint from `data-model.md`; keep `description_raw` absent from `job_postings`.
- [x] T008 Implement `.env`/provider configuration in `src-tauri/src/config/env.rs` with OS-env precedence, app-data path with working-directory fallback, provider order, `OPENROUTER_FREE_MODELS`, `JOBS_SEARCH_FIXTURE_PROFILE`, match threshold, concurrency, and the 100-job evaluation cap.
- [x] T009 Implement `sources.yaml` parsing and validation in `src-tauri/src/config/sources.rs` for `mcp`, `api`, and `web_scraper` entries, enabled state, rate limits, and `credential_ref` (never the secret itself).
- [x] T010 Define the source adapter trait and normalized job payload in `src-tauri/src/ingestion/adapter.rs` with no matching or filtering logic.
- [x] T011 Implement canonical identity, SHA-256 key, and URL fallback in `src-tauri/src/dedup/identity.rs`.
- [x] T012 Define the LLM provider trait, provider/model plans, and evaluation response schema boundary in `src-tauri/src/llm/provider.rs` and `src-tauri/src/llm/schemas.rs`.
- [x] T013 Create typed Tauri command registration, shared DTOs, and `{ code, message }` error mapping in `src-tauri/src/commands/mod.rs`, `src-tauri/src/types.rs`, `src-tauri/src/state.rs`, `src-tauri/src/error.rs`, and `src-tauri/src/lib.rs`; preserve the existing `004` telemetry command.
- [x] T014 Implement fixture profile loading from `JOBS_SEARCH_FIXTURE_PROFILE` in `src-tauri/src/profile/fixture.rs` and expose it through `cmd_get_profile`.
- [x] T015 Add the migration contract test in `src-tauri/tests/migration_contract.rs` (quickstart scenario 7): all tables/indexes exist, `PRAGMA table_info(job_postings)` has no `description_raw`, reopening is idempotent, and no credential columns exist.
- [x] T016 Add MCP JSON-RPC and LLM schema contract harnesses in `src-tauri/tests/contract_helpers.rs` for fake stdio servers and validating fixtures against `contracts/llm-evaluation.schema.json` and `contracts/llm-profile-extraction.schema.json`.

**Checkpoint**: Schema, config, fixture, contracts, and the command surface compile. Story implementation can begin.

---

## Phase 3: User Story 2 - Discover and consolidate jobs (Priority: P2) 🎯 Milestone 1

**Goal**: Hybrid ingestion (JobSpy MCP + RemoteOK API), cleaning, source lineage,
deduplication, repeat-safe persistence, and read-only job feeds.

**Independent Test**: Run a discovery with a fake MCP source and a `wiremock`
RemoteOK source, make one source fail, and verify normalized jobs, lineage, dedup,
distinct URL retention, persisted run metrics, and that the failure is recorded
without stopping the run.

### Tests for User Story 2

- [x] T017 [P] [US2] Add failing pytest tests in `sidecar/jobspy_mcp/tests/test_search.py` for LinkedIn/Indeed Brazil search, normalized output, rate limiting, and bounded retries.
- [x] T018 [P] [US2] Add failing Rust MCP client tests in `src-tauri/tests/mcp_client_contract.rs` using a fake stdio server: initialize handshake, `tools/call`, timeout, malformed JSON-RPC, and fault isolation.
- [x] T019 [P] [US2] Add failing RemoteOK adapter tests in `src-tauri/tests/remoteok_adapter.rs` with `wiremock`: normalized mapping, 429/5xx retry, timeout, and exhausted-retry failure.
- [x] T020 [P] [US2] Add failing normalization/cleaning contract tests in `src-tauri/tests/normalize.rs` for markup, control characters, irregular whitespace, modality/seniority/location normalization, and `Unknown` instead of guessing.
- [x] T021 [P] [US2] Add failing dedup/lineage/reingestion tests in `src-tauri/tests/dedup_identity.rs` and `src-tauri/tests/ingestion_pipeline.rs` covering cross-source merge, URL fallback, all distinct URLs retained, conflicting values recorded, and no duplicate records on re-run.

### Implementation for User Story 2

- [x] T022 [US2] Implement the Python MCP sidecar stdio server in `sidecar/jobspy_mcp/jobspy_mcp_server/server.py` for LinkedIn and Indeed Brazil with the `search_jobs` tool from `contracts/mcp-jobspy.md`; per-board rate limits and bounded retries live in the sidecar.
- [x] T023 [US2] Implement the Rust MCP stdio client in `src-tauri/src/ingestion/mcp.rs` including initialize handshake, per-call timeout, rate limiting, and fault isolation.
- [x] T024 [US2] Implement the RemoteOK adapter in `src-tauri/src/ingestion/api.rs` with bounded retries, timeout, and normalized output.
- [x] T025 [US2] Implement cleaning and controlled normalization in `src-tauri/src/ingestion/normalize.rs`: strip markup/control characters/whitespace, normalize modality, seniority, and location, and map unrecognized values to `Unknown`.
- [x] T026 [US2] Implement source and lineage persistence in `src-tauri/src/db/sources.rs` and `src-tauri/src/db/jobs.rs`; preserve every distinct URL and record conflicting descriptive values for audit.
- [x] T027 [US2] Implement canonical merge/repeat-safe upsert in `src-tauri/src/db/jobs.rs` and `src-tauri/src/dedup/identity.rs` keyed by `canonical_hash`/`canonical_key` with URL fallback.
- [x] T028 [US2] Implement backend search defaults and filter validation in `src-tauri/src/commands/search.rs`: timeframe `D7`, profile target roles, top core technology, profile seniority, all modalities, and enabled sources.
- [x] T029 [US2] Implement the ingestion path in `cmd_trigger_search` and execution-run metrics in `src-tauri/src/commands/search.rs` and `src-tauri/src/db/runs.rs`, including `total_ingested`, `duplicates_merged`, source failures, and `SUCCESS`/`PARTIAL`/`FAILED` status.
- [x] T030 [US2] Implement read-only `cmd_get_jobs` and `cmd_get_job` for `NEW_JOBS`, `APPLICATIONS`, and `BELOW_THRESHOLD` in `src-tauri/src/commands/jobs.rs` and `src-tauri/src/db/jobs.rs` with backend-capped `limit` and required pagination.
- [x] T031 [US2] Implement per-source pacing, bounded exponential backoff, and the central concurrency limit in `src-tauri/src/ingestion/mod.rs`, `src-tauri/src/ingestion/mcp.rs`, and `src-tauri/src/ingestion/api.rs`.
- [x] T032 [US2] Implement source removal with credential/session deletion in `src-tauri/src/config/sources.rs`, `src-tauri/src/db/sources.rs`, and `src-tauri/src/commands/search.rs`; a removed source must delete its `credential_ref` and stop participating in discovery.
- [x] T033 [US2] Complete the end-to-end ingestion test in `src-tauri/tests/ingestion_pipeline.rs`: one MCP fake, the RemoteOK `wiremock` source, one forced failure, dedup across sources, full lineage, URL retention, execution-run reconciliation, and a zero-result run.
- [x] T034 [US2] Add the `sources.yaml` example with the `jobspy` MCP source and the `remoteok` API source and no secrets.
- [x] T035 [US2] Document source configuration, rate limits, and failure semantics in `specs/001-career-job-sourcing/contracts/mcp-jobspy.md`.
- [x] T036 [US2] Run `pytest sidecar/jobspy_mcp/tests` and `cargo test --manifest-path src-tauri/Cargo.toml --test ingestion_pipeline`; fix failures and record the exact commands and results in `specs/001-career-job-sourcing/quickstart.md`.

**Checkpoint**: US2 is independently testable with the fixture profile; normalized, deduplicated jobs and lineage are persisted with run metrics.

---

## Phase 4: User Story 3 - Evaluate and rank jobs (Priority: P3) 🎯 Milestone 1

**Goal**: Hard filters, LLM evidence extraction, deterministic 40/25/20/15 score,
N/M counts, threshold 70, provider fallback, and the approved feed.

**Independent Test**: Evaluate fixed jobs with the fixture profile: one approved
above 85, one seniority rejection, one modality/location rejection, one missing
technology, one below-threshold PENDING, one unknown mandatory criterion, and one
provider-failure ERROR; confirm the same input reproduces the same status/score.

### Tests for User Story 3

- [x] T037 [P] [US3] Add failing hard-filter tests in `src-tauri/tests/match_filters.rs` for location, seniority, missing core technology, dealbreaker, Trainee/Intern technology exemption, and unknown mandatory criteria.
- [x] T038 [P] [US3] Add failing score/N/M/breakdown tests in `src-tauri/tests/score_contract.rs` for 40/25/20/15 weights, integer 0–100 score, matched + missing = total, exact-threshold approval, reproducibility, and no hard-filter approvals.
- [x] T039 [P] [US3] Add failing provider/fallback tests in `src-tauri/tests/llm_provider_contract.rs` for Gemini → OpenRouter free models → DeepSeek on 429/timeout/5xx/malformed output, auth/config stop, and provider/model metric recording.
- [x] T040 [P] [US3] Add failing fixture-profile evaluation tests in `src-tauri/tests/fixture_match.rs` with the synthetic profile and fixed postings, asserting statuses, breakdowns, matched/missing keywords, and `stale` behavior.

### Implementation for User Story 3

- [x] T041 [US3] Implement Gemini, OpenRouter free-list, and DeepSeek adapters in `src-tauri/src/llm/gemini.rs`, `src-tauri/src/llm/openrouter.rs`, and `src-tauri/src/llm/deepseek.rs`.
- [x] T042 [US3] Implement provider routing/fallback in `src-tauri/src/llm/provider.rs`: transient failures only, per-provider attempt budgets, fatal auth/config errors, and per-attempt metrics.
- [x] T043 [US3] Implement strict evaluation-schema validation in `src-tauri/src/llm/schemas.rs` using `contracts/llm-evaluation.schema.json`, including bounded field lengths, unique requirement IDs, and reject-unknown-fields.
- [x] T044 [US3] Implement location, seniority, technology, and mandatory-domain hard filters in `src-tauri/src/match/filters.rs`, with hard filters running before any LLM call.
- [x] T045 [US3] Implement requirement grouping, N/M counts, 40/25/20/15 score, integer rounding, and threshold comparison in `src-tauri/src/match/score.rs`.
- [x] T046 [US3] Implement evaluation persistence and stale marking in `src-tauri/src/db/evaluations.rs` with `rules_version`, `threshold`, provider/model, and audit-safe matched/missing keyword lists.
- [x] T047 [US3] Implement evaluation orchestration in `src-tauri/src/match/mod.rs`: persisted jobs, 2 concurrent LLM calls, 100-job cap, hard-filter precedence, and `APPROVED`/`PENDING`/`REJECTED`/`ERROR`.
- [x] T048 [US3] Wire the evaluation path into `cmd_trigger_search` and the approved/PENDING feeds in `src-tauri/src/commands/search.rs` and `src-tauri/src/commands/jobs.rs`.
- [x] T049 [US3] Implement unknown-mandatory-criterion handling in `src-tauri/src/match/filters.rs`: when the profile marks seniority, modality, location, or a domain competency as mandatory, an `Unknown` value must record a gap, must not count as a match, and must keep the job out of the approved list.
- [x] T050 [US3] Exclude stale evaluations from `src-tauri/src/db/jobs.rs` feeds and add them back to the evaluation queue in `src-tauri/src/db/jobs.rs` so a changed profile/rules version is never presented as current.
- [x] T051 [US3] Document `JOBS_SEARCH_FIXTURE_PROFILE` behavior and the fixture profile example in `specs/001-career-job-sourcing/quickstart.md`.
- [x] T052 [US3] Run `cargo test --manifest-path src-tauri/Cargo.toml --test match_filters --test score_contract --test llm_provider_contract --test fixture_match`; fix failures and record results in `specs/001-career-job-sourcing/quickstart.md`.

**Checkpoint**: Milestone 1 complete. The feed shows real approved jobs from the fixture profile; apply/ignore/audit remain deferred.

---

## Phase 5: User Story 1 - Build a verified candidate profile (Priority: P1)

**Goal**: PDF ingestion, LLM profile extraction, merge/manual precedence, profile
persistence/versioning, and resume management.

**Independent Test**: Upload a text-based fixture PDF and verify a valid
`CandidateProfile` with source lineage and editable fields; upload two versions and
verify list union and scalar-conflict flags; upload a corrupt/image-only PDF and
verify a specific failure that leaves the existing profile unchanged.

### Tests for User Story 1

- [x] T053 [P] [US1] Add failing PDF extraction tests in `src-tauri/tests/profile_pipeline.rs` for text PDFs, corrupt, encrypted, unsupported, and image-only files using fixtures under `src-tauri/tests/fixtures/`.
- [x] T054 [P] [US1] Add failing profile merge tests in `src-tauri/tests/profile_pipeline.rs` for list union without duplicates, conflicting scalar flags, manual operator precedence, and per-competency source traceability.
- [x] T055 [P] [US1] Add failing profile persistence/versioning tests in `src-tauri/tests/profile_pipeline.rs` for one active profile, monotonic `version`, resume `content_hash` uniqueness, SQLite reopen, and safe failure without partial writes.

### Implementation for User Story 1

- [x] T056 [US1] Implement `pdf-extract` text extraction in `src-tauri/src/pdf/extract.rs` and `src-tauri/src/pdf/mod.rs`; keep raw extracted text in `Zeroizing<String>` and drop it after extraction.
- [x] T057 [US1] Implement the profile extraction boundary in `src-tauri/src/profile/schema.rs` and `src-tauri/src/llm/prompt.rs` against `contracts/llm-profile-extraction.schema.json`; treat resume text as untrusted, delimit it, and instruct the model to ignore embedded instructions.
- [x] T058 [US1] Implement merge/manual precedence in `src-tauri/src/profile/merge.rs`: union list-valued fields, flag conflicting scalars for review, never silently discard a value, preserve the most recent non-conflicting value, and mark inferred competencies.
- [x] T059 [US1] Implement profile persistence/versioning in `src-tauri/src/db/profile.rs` and `src-tauri/src/db/migrations.rs`: exactly one active profile, monotonic `version`, `acceptable_technologies_json` as normalized manual tags, and `accepted_cities_json` defaulting to `["Porto Alegre", "Curitiba"]`.
- [x] T060 [US1] Implement resume storage in `src-tauri/src/db/profile.rs`: `id`, original `filename`, SHA-256 `content_hash UNIQUE`, app-data `local_path` never logged, and UTC `uploaded_at`.
- [x] T061 [US1] Implement `cmd_upload_resume`, `cmd_delete_resume`, `cmd_get_profile`, and `cmd_save_profile` in `src-tauri/src/commands/profile.rs` with path, extension, PDF signature, and size validation; `cmd_save_profile` increments the version and marks existing evaluations stale.
- [x] T062 [US1] Implement LLM profile extraction in `src-tauri/src/profile/mod.rs` using the provider router in `src-tauri/src/llm/provider.rs` with the same Gemini → OpenRouter → DeepSeek fallback, `profile_extraction` operation metrics, and provider/model recording.
- [x] T063 [US1] Implement safe failure in `src-tauri/src/commands/profile.rs` and `src-tauri/src/db/profile.rs`: a parse or validation failure returns a specific error and never creates or overwrites a profile with partial data.
- [x] T064 [US1] Implement last-resume deletion behavior in `src-tauri/src/commands/profile.rs` and `src-tauri/src/commands/search.rs`: preserve the existing profile, mark that no resumes remain, and block new searches with a clear error until a resume or verified profile is supplied.
- [x] T065 [US1] Call `mark_stale` from `src-tauri/src/commands/profile.rs` whenever the profile, manual technology tags, accepted cities, threshold, or dealbreakers change, so prior evaluations are not presented as current.
- [x] T066 [US1] Implement hard-filter profile validation in `src-tauri/src/profile/mod.rs` and `src-tauri/src/commands/search.rs`: `target_roles`, accepted seniorities, accepted cities, and at least one core/acceptable technology are required; missing values are flagged and never treated as a match.
- [x] T067 [US1] Complete the end-to-end profile pipeline test in `src-tauri/tests/profile_pipeline.rs`: PDF → validated profile → versioned persistence → edit/save → stale marking → reopen, including the corrupt/image-only failure path.

**Checkpoint**: US1 replaces the fixture with a real, verified, versioned profile and keeps all prior profile rules testable.

---

## Phase 6: User Story 4 - Audit, application, and lifecycle (Priority: P4)

**Goal**: Apply/ignore/restore, manual approval, append-only lifecycle, ranked
approved output, and reconciled execution audit.

**Independent Test**: Run a fixed pipeline with known totals and statuses; verify
the approved list is filtered/sorted with the documented tie-breaker, the audit
reconciles exactly, rejected jobs remain visible in the audit, and apply/ignore
writes a lifecycle event without duplicating the opportunity.

### Tests for User Story 4

- [x] T068 [P] [US4] Add failing apply/ignore/restore/manual-approval tests in `src-tauri/tests/application_lifecycle.rs`, including append-only events, snapshot preservation, and no duplicate opportunity on re-ingestion.
- [x] T069 [P] [US4] Add failing audit/reconciliation tests in `src-tauri/tests/audit_contract.rs` for total ingested, duplicates merged, rejections by reason, source failures, approved count, run ID, and start/end times.
- [x] T070 [P] [US4] Add failing ranked-list tests in `src-tauri/tests/audit_contract.rs` for descending score, approved-only output, the documented tie-breaker (core technical score, then ingestion time, then canonical title ascending), and the zero-result state.

### Implementation for User Story 4

- [x] T071 [US4] Implement application lifecycle persistence in `src-tauri/src/db/applications.rs` and `src-tauri/src/db/mod.rs`: `job_id` PK/FK, `lifecycle_state` in `DISCOVERED`/`APPLIED`/`IGNORED`, `applied_at` required when `APPLIED`, `approval_source` in `automatic`/`manual`, and `evaluation_snapshot_json`.
- [x] T072 [US4] Implement append-only lifecycle events in `src-tauri/src/db/applications.rs` for `job_status_events`; never update or delete existing events and sanitize `reason`.
- [x] T073 [US4] Implement `cmd_apply_job`, `cmd_ignore_job`, and `cmd_restore_job` in `src-tauri/src/commands/jobs.rs` returning the `JobApplication` contract shape and writing lifecycle events atomically.
- [x] T074 [US4] Implement manual approval for PENDING jobs via `cmd_approve_job` in `src-tauri/src/commands/jobs.rs` and `src-tauri/src/db/evaluations.rs`; set `approval_source = manual` and preserve the original score/breakdown.
- [x] T075 [US4] Record `EVALUATED`/`APPROVED`/`REJECTED` status events from `src-tauri/src/match/mod.rs` and `src-tauri/src/db/evaluations.rs` with the active `rules_version` and threshold.
- [x] T076 [US4] Implement `cmd_get_audit` and `ExecutionAudit` in `src-tauri/src/commands/jobs.rs` and `src-tauri/src/db/runs.rs`: run ID, start/end time, totals, rejection reasons and counts, source failures, approved count, and the recorded search configuration.
- [x] T077 [US4] Fix ranked approved output in `src-tauri/src/db/jobs.rs` to use the documented deterministic tie-breaker, exclude rejected/stale/ignored rows, and expose matched/missing keywords plus the score breakdown in `JobDetail`.
- [x] T078 [US4] Implement the zero-result state and rejection visibility in `src-tauri/src/db/jobs.rs` and `src-tauri/src/commands/jobs.rs`: zero approved results return an empty page plus the audit reference, while rejection reasons remain queryable.
- [x] T079 [US4] Implement the application snapshot in `src-tauri/src/db/applications.rs`: capture the evaluation at `APPLIED` time, keep it immutable across later profile/rule changes, and preserve it on re-ingestion.
- [x] T080 [US4] Register all new commands in `src-tauri/src/lib.rs` and `src-tauri/src/commands/mod.rs`, then complete the end-to-end audit/apply test and record results in `specs/001-career-job-sourcing/quickstart.md`.

**Checkpoint**: All four user stories are independently functional and the execution audit reconciles with persisted records.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, full verification, privacy audit, and config checks.

- [x] T081 [P] Update `README.md` with backend commands, `sources.yaml`, the fixture profile, environment configuration, and the local-only/privacy model.
- [x] T082 [P] Update `specs/001-career-job-sourcing/quickstart.md` with implemented commands, actual paths, and the sidecar/test invocation.
- [x] T083 Run `./scripts/verify.sh` from the repository root; fix failures in `src-tauri/src/`, `src-tauri/tests/`, and `sidecar/jobspy_mcp/`, and report explicitly which checks ran and which were skipped.
- [x] T084 [P] Run `pytest sidecar/jobspy_mcp/tests` and record the result; the sidecar directory must exist before claiming this check passed.
- [x] T085 [P] Audit `src-tauri/src/`, `src-tauri/tests/`, and `sidecar/` for `description_raw` persistence, raw descriptions/credentials/prompts in logs or reports, external telemetry, and secret leakage in `sources.yaml` or `.env`; fix any finding.
- [x] T086 Run the full quickstart scenario list and record evidence (commands, outcomes, skipped areas) in `specs/001-career-job-sourcing/quickstart.md`.
- [x] T087 [P] Sync `specs/001-career-job-sourcing/contracts/tauri-commands.md` with the implemented DTOs, commands, and lifecycle/audit behavior.
- [x] T088 [P] Add regression tests in `src-tauri/tests/score_contract.rs` and `src-tauri/tests/fixture_match.rs` that assert the 100-job cap, 2-way concurrency, 15-second per-source timeout, and configurable threshold 70 read from `src-tauri/src/config/env.rs` and `src-tauri/src/match/mod.rs`.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: Can start immediately.
- **Foundational (Phase 2)**: Depends on Setup; blocks all stories.
- **US2 (Phase 3)**: Depends on Foundational; first milestone.
- **US3 (Phase 4)**: Depends on Foundational and US2 jobs; first milestone.
- **US1 (Phase 5)**: Depends on Foundational; later milestone, replaces the fixture.
- **US4 (Phase 6)**: Depends on US3 evaluation records; later milestone.
- **Polish (Phase 7)**: Depends on the desired stories.

### User Story Dependencies

- **US2**: No dependency on US1/US3/US4 after Foundational.
- **US3**: Needs US2 normalized jobs and the fixture profile.
- **US1**: Independent of US2/US3 after Foundational, but its saved profile replaces the fixture in later runs.
- **US4**: Needs US3 evaluation/status records.

### Within Each Story

- Tests first; confirm they fail.
- Types/models before services; services before commands.
- Run `cargo fmt`, `cargo check`, `cargo clippy -- -D warnings`, and `cargo test` before each commit.
- One task per commit, with the task ID in the message.

## Parallel Opportunities

- T005, T015, and T016 are independent foundation files.
- T017-T021 run in parallel (separate test files).
- T022-T031: sidecar, MCP client, API adapter, normalization, and persistence touch different files.
- T032 and T035 are independent of the adapter work.
- T037-T040 run in parallel (separate test files).
- T041-T048 providers vs. filters/score/persistence can split across developers.
- T053-T055 and T068-T070 are independent test files at their milestones.
- T081, T082, T084, T085, T087, and T088 are parallel polish tasks.

## Parallel Example: US2

```text
Task: T022 Implement the Python MCP sidecar in sidecar/jobspy_mcp/jobspy_mcp_server/server.py
Task: T023 Implement the Rust MCP client in src-tauri/src/ingestion/mcp.rs
Task: T024 Implement the RemoteOK adapter in src-tauri/src/ingestion/api.rs
```

## Parallel Example: US3

```text
Task: T041 Implement provider adapters in src-tauri/src/llm/gemini.rs, openrouter.rs, deepseek.rs
Task: T044 Implement hard filters in src-tauri/src/match/filters.rs
Task: T045 Implement the deterministic score in src-tauri/src/match/score.rs
Task: T046 Implement evaluation persistence in src-tauri/src/db/evaluations.rs
```

---

## Implementation Strategy

### Milestone 1 (agreed): US2 + US3

1. Complete Setup and Foundational (T001-T016).
2. Complete US2 ingestion/dedup (T017-T036).
3. Complete US3 evaluation with the fixture profile (T037-T052).
4. Validate the read-only approved feed.
5. Stop before US1/US4.

### Later milestones

1. US1: real PDF/profile flow replacing the fixture (T053-T067).
2. US4: apply/ignore/restore, manual approval, and audit (T068-T080).
3. Polish and full quickstart (T081-T088).

## Notes

- Tests are required; do not skip failing-path tests.
- Never persist `description_raw`; discard it after cleaning.
- No external telemetry; local logs are handled by `004`.
- The first milestone intentionally uses a synthetic fixture profile.
- Commit each completed task separately with `feat(T0xx)`, `test(T0xx)`, or `docs(T0xx)`.
- Spec-level MVP is US1; the agreed engineering milestone is US2 + US3 with the fixture profile.
