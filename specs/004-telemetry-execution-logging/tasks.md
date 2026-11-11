---

description: "Task list for Local Telemetry & Execution Logging"
---

# Tasks: Local Telemetry & Execution Logging

**Input**: Design documents from `/specs/004-telemetry-execution-logging/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`,
`contracts/`, `quickstart.md`

**Tests**: Required. The plan, constitution, and project rules require Rust unit
and integration tests for queue behavior, sanitization, retention, outcomes, and
shutdown. Tests are written first and MUST fail before implementation.

**Organization**: Tasks are grouped by user story so each story can be
implemented and validated independently.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete
  task)
- **[Story]**: `[US1]`, `[US2]`, or `[US3]`
- Every task includes an exact file path.

## Path Conventions

- Rust backend: `src-tauri/src/` and `src-tauri/tests/`
- Tauri config: `src-tauri/`
- Feature docs and contracts: `specs/004-telemetry-execution-logging/`

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create the minimal Tauri/Rust host and telemetry module tree.

- [X] T001 Create `src-tauri/Cargo.toml` with Tauri `2.12.5`, `serde`,
  `serde_json`, `tracing`, `tracing-subscriber` (`env-filter` and `json`),
  `tracing-appender`, `chrono`, and `thiserror`; add `tempfile` and `jsonschema`
  as dev-dependencies.
- [X] T002 Create `src-tauri/build.rs`, `src-tauri/tauri.conf.json` with
  identifier `com.willf.jobs-search`, product name `Jobs Search`, and
  `frontendDist` `../dist`; add a placeholder `dist/index.html`.
- [X] T003 Create `src-tauri/src/main.rs` and `src-tauri/src/lib.rs` with a
  minimal Tauri builder and module declarations for `commands` and `telemetry`.
- [X] T004 Create `src-tauri/capabilities/default.json` with the core default
  Tauri permissions.
- [X] T005 Create the telemetry module tree:
  `src-tauri/src/telemetry/{mod.rs,event.rs,config.rs,queue.rs,writer.rs,health.rs,sanitize.rs}`
  and `src-tauri/src/commands/telemetry.rs`; declare the modules.
- [X] T006 Create `src-tauri/tests/common/mod.rs` with a temporary log-directory
  helper using `tempfile`.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core types, configuration, queue, writer, health, and command
wiring shared by all stories.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T007 Implement `TelemetryConfig` in `src-tauri/src/telemetry/config.rs`:
  level `INFO/WARN/ERROR/DEBUG`, default `INFO`, daily rotation,
  `retention_days = 7`, `max_total_bytes = 52_428_800`, `queue_capacity = 10_000`,
  and `DROP_OLDEST` overflow. Include unit tests.
- [X] T008 Implement `LogEvent` and its enums in
  `src-tauri/src/telemetry/event.rs`: event types, components, outcomes, error
  categories, optional run/source/provider/model/fallback fields, `message`
  maximum 512 characters, and serde serialization. Include unit tests.
- [X] T009 [P] Implement the sanitizer in `src-tauri/src/telemetry/sanitize.rs`:
  reject or strip URLs, filesystem paths, known secret/PII patterns, and enforce
  the 512-character message limit. Include unit tests.
- [X] T010 [P] Implement the bounded queue in `src-tauri/src/telemetry/queue.rs`:
  capacity 10,000, oldest-first dropping, atomic dropped count, and condvar-based
  handoff. Include unit tests.
- [X] T011 Implement the JSONL writer in `src-tauri/src/telemetry/writer.rs`:
  daily file `jobs-search.YYYY-MM-DD.log`, restricted permissions, background
  drain, flush, and 7-day/52,428,800-byte retention cleanup. Include unit tests.
- [X] T012 Implement `LogHealth` in `src-tauri/src/telemetry/health.rs` with
  `available`, `queue_depth`, `queue_capacity`, `dropped_count`, `last_error`,
  `last_success_at`, and `current_file`. Include unit tests.
- [X] T013 Wire telemetry initialization and shutdown in
  `src-tauri/src/telemetry/mod.rs`: create the log directory, build the
  subscriber/writer, expose a facade, and flush for at most 5 seconds. Include
  unit tests.
- [X] T014 Implement `cmd_get_log_health` in
  `src-tauri/src/commands/telemetry.rs` and register it in
  `src-tauri/src/lib.rs`; add a command contract test.
- [X] T015 Add contract tests in `src-tauri/tests/telemetry_contract.rs` that
  validate generated events against `contracts/log-event.schema.json` and the
  health payload against `contracts/log-health.schema.json`.

**Checkpoint**: Foundation ready. The telemetry module builds, writes, rotates,
reports health, and exposes the command.

---

## Phase 3: User Story 1 - Capture lifecycle and execution events (P1) 🎯 MVP

**Goal**: Emit typed events for startup, shutdown, search runs, source failures,
deduplication, LLM calls/fallback, evaluation, and application actions.

**Independent Test**: Emit one event of each type with a shared run identifier,
read the log file, and verify the required fields and event types.

### Tests for User Story 1

- [X] T016 [P] [US1] Add failing tests for `APP_START`, `APP_SHUTDOWN`,
  `SEARCH_RUN_START`, and `SEARCH_RUN_END` in
  `src-tauri/tests/telemetry_events_lifecycle.rs`.
- [X] T017 [P] [US1] Add failing tests for `SOURCE_FAILURE` and `DEDUP_COMPLETE`
  in `src-tauri/tests/telemetry_events_ingestion.rs`.
- [X] T018 [P] [US1] Add failing tests for `LLM_CALL`, `LLM_FALLBACK`, and
  `EVALUATION_COMPLETE` in `src-tauri/tests/telemetry_events_llm.rs`.
- [X] T019 [P] [US1] Add failing tests for `APPLICATION_ACTION`, including
  `job_ref` and `lifecycle_transition`, in
  `src-tauri/tests/telemetry_events_app.rs`.

### Implementation for User Story 1

- [X] T020 [US1] Implement safe event constructors and facade methods for all
  US1 event types in `src-tauri/src/telemetry/mod.rs` and
  `src-tauri/src/telemetry/event.rs`.
- [X] T021 [US1] Emit `APP_START` during telemetry initialization and
  `APP_SHUTDOWN` during shutdown in `src-tauri/src/lib.rs` and
  `src-tauri/src/telemetry/mod.rs`.
- [X] T022 [US1] Add an end-to-end event sequence test with a shared `run_id` in
  `src-tauri/tests/telemetry_events.rs`; verify JSONL round-trip and required
  fields.

**Checkpoint**: User Story 1 is independently testable through the facade and
the real log file.

---

## Phase 4: User Story 2 - Control log levels and local retention (P2)

**Goal**: Enforce INFO/WARN/ERROR by default, DEBUG opt-in via `RUST_LOG`, and
keep logs within 7 days or 50 MB.

**Independent Test**: Run with default and DEBUG levels, verify filtering, and
backdate/oversize temp log files to verify retention cleanup.

### Tests for User Story 2

- [X] T023 [P] [US2] Add failing level-filtering tests for INFO default, DEBUG
  opt-in, and invalid-value fallback in
  `src-tauri/tests/telemetry_levels.rs`.
- [X] T024 [P] [US2] Add failing rotation/retention tests for daily files,
  `jobs-search.YYYY-MM-DD.log`, 7-day expiry, and 52,428,800-byte total cap in
  `src-tauri/tests/telemetry_retention.rs`.
- [X] T025 [P] [US2] Add a failing local-only test asserting no MongoDB or remote
  destination and no in-app viewer/export in
  `src-tauri/tests/telemetry_local_only.rs`.

### Implementation for User Story 2

- [X] T026 [US2] Wire `RUST_LOG` and local environment configuration to the
  subscriber and enforce level filtering in
  `src-tauri/src/telemetry/config.rs` and `src-tauri/src/telemetry/mod.rs`.
- [X] T027 [US2] Complete rotation and retention cleanup in
  `src-tauri/src/telemetry/writer.rs` until T023-T025 pass.

**Checkpoint**: User Story 2 works independently with default/DEBUG levels and
bounded retention.

---

## Phase 5: User Story 3 - Keep logging safe and non-blocking (P3)

**Goal**: Prevent sensitive content, bound memory, survive log-store failures,
flush on shutdown, and avoid UI/pipeline stalls.

**Independent Test**: Emit a 1,000-event burst, fill the queue, make the log
directory unavailable, inspect logs for forbidden content, and verify shutdown
within 5 seconds.

### Tests for User Story 3

- [X] T028 [P] [US3] Add failing sanitization tests for URLs, paths, prompts,
  payloads, credentials, candidate data, and the 512-character message limit in
  `src-tauri/tests/telemetry_safety.rs`.
- [X] T029 [P] [US3] Add failing queue-overflow tests for 1,000 events,
  oldest-first dropping, `dropped_count`, and queue depth at most 10,000 in
  `src-tauri/tests/telemetry_queue.rs`.
- [X] T030 [P] [US3] Add a failing outage test for a read-only or unavailable log
  directory; the pipeline MUST continue and health MUST be degraded/unavailable
  in `src-tauri/tests/telemetry_outage.rs`.
- [X] T031 [P] [US3] Add a failing shutdown-flush test with a 5-second bound in
  `src-tauri/tests/telemetry_shutdown.rs`.
- [X] T032 [P] [US3] Add a failing 1,000-event non-blocking burst test in
  `src-tauri/tests/telemetry_burst.rs`.
- [X] T033 [P] [US3] Add a failing health-command contract test for
  `cmd_get_log_health` in `src-tauri/tests/telemetry_health.rs`.

### Implementation for User Story 3

- [X] T034 [US3] Harden sanitization, oldest-first queue dropping, outage
  degradation, shutdown flush, burst handling, and health reporting in
  `src-tauri/src/telemetry/sanitize.rs`, `queue.rs`, `writer.rs`, `health.rs`,
  and `mod.rs` until T028-T033 pass.

**Checkpoint**: User Story 3 is independently testable under burst, overflow,
outage, and shutdown conditions.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, verification, and release checks that affect all
stories.

- [X] T035 [P] Update `README.md` with the log directory, retention policy,
  `RUST_LOG`, `cmd_get_log_health`, and the no-external-telemetry rule.
- [X] T036 [P] Update `specs/004-telemetry-execution-logging/quickstart.md` with
  the implemented commands and actual log path.
- [X] T037 Run `cargo fmt --check`,
  `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`,
  and `cargo test --manifest-path src-tauri/Cargo.toml`; fix findings in
  `src-tauri/src/telemetry/` and `src-tauri/tests/`.
- [X] T038 [P] Verify with grep that `src-tauri/src/telemetry/` and
  `src-tauri/tests/` contain no external log destination, raw payload/prompt
  fields, URLs, filesystem paths, candidate data, or credentials.
- [X] T039 Run quickstart scenarios 1-9 from
  `specs/004-telemetry-execution-logging/quickstart.md` and record the outcomes
  in the same file.
- [X] T040 Create the mandatory verification gate in `scripts/verify.sh`,
  `.husky/pre-commit`, and `.husky/commit-msg`; install Husky, commitlint,
  lint-staged, Prettier, and oxlint; configure the hook path.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies; create the Rust/Tauri host and module
  tree.
- **Foundational (Phase 2)**: Depends on Setup; blocks all user stories.
- **User Stories (Phases 3-5)**: Depend on Foundational. They can run in
  parallel, but the recommended order is P1 → P2 → P3.
- **Polish (Phase 6)**: Depends on the desired user stories being complete.

### User Story Dependencies

- **US1 (P1)**: No dependency on US2/US3 after Foundational.
- **US2 (P2)**: No dependency on US1/US3 after Foundational.
- **US3 (P3)**: No dependency on US1/US2 after Foundational; it validates the
  shared queue, writer, sanitizer, and health.

### Within Each Story

- Write the failing tests first.
- Implement the smallest behavior that makes them pass.
- Run Rust formatting, clippy, and tests before committing.
- Commit one completed task per commit with a Conventional Commit and task ID.

## Parallel Opportunities

- T009 and T010 can run in parallel (different files).
- T016-T019 can run in parallel (separate test files).
- T023-T025 can run in parallel (separate test files).
- T028-T033 can run in parallel (separate test files).
- T035, T036, and T038 can run in parallel after implementation.

## Parallel Example: User Story 1

```text
Task: T016 Add lifecycle event tests in src-tauri/tests/telemetry_events_lifecycle.rs
Task: T017 Add ingestion event tests in src-tauri/tests/telemetry_events_ingestion.rs
Task: T018 Add LLM event tests in src-tauri/tests/telemetry_events_llm.rs
Task: T019 Add application event tests in src-tauri/tests/telemetry_events_app.rs
```

## Implementation Strategy

### MVP First

1. Complete Setup and Foundational.
2. Complete US1 (event capture).
3. Validate US1 independently by reading the JSONL log.
4. Before release, complete US2 (levels/retention) and US3
   (safety/non-blocking); both are required for the security and operational
   guarantees in the spec.

### Incremental Delivery

1. Setup + Foundational → telemetry core builds and writes.
2. US1 → useful event stream (MVP).
3. US2 → safe level control and bounded disk use.
4. US3 → privacy, outage resilience, and non-blocking guarantees.
5. Polish → docs, checks, quickstart evidence.

## Notes

- Tests are required by the plan and constitution; do not skip them.
- Use the exact file paths and field constraints from `data-model.md`.
- Every task is one commit, using `feat(T0xx): ...`, `test(T0xx): ...`, or
  `docs(T0xx): ...`.
- Do not add MongoDB, remote logging, raw payloads, prompts, URLs, paths,
  candidate data, or credentials to the log contract.
