---

description: "Task list for MCP sidecar stderr diagnostics and crash handling"
---

# Tasks: MCP Sidecar Diagnostics and Crash Handling

**Input**: Bug report `BUG-003`. No `spec.md` or `plan.md` exists for this
feature yet, so the tasks are derived from the report, the current Rust/Python
source, and the project telemetry architecture.

**Prerequisites**: Rust toolchain, Python `3.14`, the sidecar tests under
`sidecar/jobspy_mcp/tests/`, and a local `.env` only when exercising the real
sidecar. Never print or commit credentials.

**Tests**: Required. The project rules require tests for new behavior and
failure paths; both fixes are regressions, so each test must fail before its
implementation and pass after it.

**Organization**: `US1` covers the Rust stderr capture/telemetry path;
`US2` covers the Python sidecar JSON-RPC error handling. They are independent
and can be implemented in parallel after the baseline task.

## Root Cause Notes (verified 2026-11-11)

- **Rust discards stderr**: `McpAdapter::fetch` starts the sidecar with
  `.stderr(Stdio::null())` in `src-tauri/src/ingestion/mcp.rs`. When the child
  exits, `read_message` sees EOF and returns the generic
  `MCP_PIPE_FAILED`/`sidecar exited before responding`, so the Python traceback
  or missing-dependency message is lost.
- **Deadlock risk**: simply switching to `Stdio::piped()` is not enough. The
  stderr pipe must be drained concurrently with the stdout JSON-RPC exchange;
  otherwise a sidecar that fills the stderr buffer blocks before it can answer.
- **Python has no top-level guard**: `sidecar/jobspy_mcp/jobspy_mcp_server/server.py`
  catches `JobSpyError` and generic exceptions inside `tools/call`, but
  `serve`/`main` do not guard against malformed request shapes, writer failures,
  or unexpected exceptions. A traceback can terminate the process and corrupt
  the protocol stream.
- **Telemetry constraint**: this project intentionally has no MongoDB.
  `README.md` states "No ... MongoDB, or external telemetry", and the project
  rules require local rotating JSONL logs. Route captured stderr to the
  existing local `tracing`/`telemetry` layer under `src-tauri/src/telemetry/`;
  do not add MongoDB or an external telemetry dependency.
- **Sensitive diagnostics**: stderr can contain absolute paths, URLs, tokens,
  or traceback frames. Never send raw stderr to stdout, the frontend, SQLite,
  or the log file. Strip control characters, drop traceback frame/path lines,
  redact forbidden patterns, and cap the retained excerpts.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete
  task).
- **[Story]**: `[US1]` for the Rust stderr/telemetry path, `[US2]` for the
  Python JSON-RPC crash handling.
- Every task includes an exact file path.

## Path Conventions

- Rust backend: `src-tauri/src/` and `src-tauri/tests/`
- Python sidecar: `sidecar/jobspy_mcp/jobspy_mcp_server/` and
  `sidecar/jobspy_mcp/tests/`
- Feature docs: `specs/006-mcp-sidecar-diagnostics/`
- Local telemetry: `src-tauri/src/telemetry/` and the application data
  `logs/` directory

---

## Phase 1: Setup (Baseline)

**Purpose**: Establish the current test baseline before changing the MCP
transport or the sidecar.

- [ ] T001 Run `./scripts/verify.sh` and `pytest sidecar/jobspy_mcp/tests` from the repo root; record any pre-existing failures in the first commit message and do not fix unrelated failures in this feature (path: `scripts/verify.sh`).

**Checkpoint**: Baseline is known; no unrelated failures are mixed into the
bugfix work.

---

## Phase 2: Foundational (Blocking Prerequisites)

No shared implementation prerequisites are required. US1 and US2 are
independent after T001; do not add a MongoDB dependency, an external telemetry
service, or a new database migration for either story.

**Checkpoint**: T001 is complete; US1 and US2 can start in parallel.

---

## Phase 3: User Story 1 - Rust captures and routes sidecar stderr (Priority: P1) 🎯 MVP

**Goal**: A sidecar that crashes or writes diagnostics no longer fails
silently. Rust drains stderr concurrently with stdout, emits a sanitized
`SOURCE_FAILURE` event to the local telemetry layer, and includes a short
sanitized stderr excerpt in `MCP_PIPE_FAILED`.

**Independent Test**: Run a fake MCP sidecar that writes
`ModuleNotFoundError: No module named 'jobspy'` to stderr and exits before
answering `initialize`. Confirm `fetch` returns `MCP_PIPE_FAILED` with that
sanitized cause, a `SOURCE_FAILURE` telemetry event is emitted, and no path,
URL, token, or raw stderr reaches stdout/the frontend/SQLite. Run a second fake
sidecar that floods stderr before answering; confirm the handshake still
completes within the timeout.

### Tests for User Story 1 ⚠️

> Write these tests first and confirm they fail against the current
> `Stdio::null()` behavior.

- [ ] T002 [P] [US1] Extend the Rust test helpers with a fake sidecar builder that can write arbitrary stderr, exit before or after the handshake, and optionally flood stderr; keep all fixtures free of real paths and credentials (path: `src-tauri/tests/common/mod.rs`).
- [ ] T003 [US1] Add MCP contract tests for a fatal stderr + EOF (`MCP_PIPE_FAILED` includes the sanitized `ModuleNotFoundError` cause), redaction of paths/URLs/tokens, and a stderr flood that still completes the handshake (depends on T002; path: `src-tauri/tests/mcp_client_contract.rs`).
- [ ] T004 [P] [US1] Add a telemetry test asserting that `Telemetry::mcp_stderr` emits a `SOURCE_FAILURE` event with `component = "ingestion"`, the sidecar source id, `error_category = "IO"`, and a sanitized message, and that the event validates against `log-event.schema.json` (path: `src-tauri/tests/telemetry_events_ingestion.rs`).

### Implementation for User Story 1

- [ ] T005 [P] [US1] Add `Telemetry::mcp_stderr(source: &str, message: &str)` to the local telemetry layer: emit `event_type = "SOURCE_FAILURE"`, `component = "ingestion"`, `source`, `outcome = "PARTIAL"`, `error_category = "IO"`, and a sanitized/truncated `message`; do not add a new database or external service (path: `src-tauri/src/telemetry/mod.rs`).
- [ ] T006 [US1] Change `McpAdapter::fetch` to use `Stdio::piped()` for stderr and spawn a concurrent reader before the stdout exchange; keep draining after the retention cap so the child can never block, await the reader after killing/waiting for the child, and never leave a detached stderr task behind (path: `src-tauri/src/ingestion/mcp.rs`).
- [ ] T007 [US1] Add a bounded stderr retention/sanitization helper in the MCP module: strip control characters, drop traceback frame/path lines, redact URLs/secrets with `telemetry::sanitize`, retain only the last N safe lines/bytes, and cap the final excerpt at 512 characters (path: `src-tauri/src/ingestion/mcp.rs`).
- [ ] T008 [US1] Emit one sanitized `Telemetry::mcp_stderr` event per fetch when stderr is non-empty, and append the sanitized excerpt to stdin/stdout/EOF `MCP_PIPE_FAILED` errors; keep the original error code and retryability, and never write stderr to stdout or the frontend directly (path: `src-tauri/src/ingestion/mcp.rs`).
- [ ] T009 [US1] Run the focused Rust suites (`cargo test --manifest-path src-tauri/Cargo.toml --test mcp_client_contract` and `cargo test --manifest-path src-tauri/Cargo.toml --test telemetry_events_ingestion`) and verify the telemetry event schema and redaction assertions (paths: `src-tauri/tests/mcp_client_contract.rs`, `src-tauri/tests/telemetry_events_ingestion.rs`).
- [ ] T010 [P] [US1] Document in `README.md` that sidecar stderr is captured, sanitized, and written to the local JSONL telemetry layer, not MongoDB, and that `MCP_PIPE_FAILED` now carries a safe diagnostic excerpt (path: `README.md`).

**Checkpoint**: Sidecar crashes are diagnosable without leaking sensitive data,
and a noisy stderr stream cannot deadlock the JSON-RPC handshake.

---

## Phase 4: User Story 2 - Sidecar returns JSON-RPC errors instead of crashing (Priority: P2)

**Goal**: The Python sidecar handles malformed requests and unexpected
exceptions without exiting abruptly. It returns a valid JSON-RPC error, keeps
serving subsequent requests, and writes diagnostics to stderr only.

**Independent Test**: Feed `serve` a non-object JSON value and a request whose
handler raises. Confirm stdout contains only valid JSON-RPC errors (`-32600`
for invalid request, `-32603` for internal error), the sidecar continues with a
following valid request, and stderr contains a short diagnostic with no
traceback on stdout. Call `main` with a raising `serve` and confirm it emits a
final `-32603` error instead of a traceback.

### Tests for User Story 2 ⚠️

- [ ] T011 [P] [US2] Create `sidecar/jobspy_mcp/tests/test_server.py` with tests for invalid JSON-RPC shapes (`[]`, `"text"`, `null`) returning `-32600`, an unexpected handler exception returning `-32603` with the request id and then continuing, `main`/global-hook fatal handling returning a final error instead of raising, normal requests still round-tripping, and diagnostics going to stderr only (path: `sidecar/jobspy_mcp/tests/test_server.py`).

### Implementation for User Story 2

- [ ] T012 [US2] Validate the decoded request shape in `handle`: a non-object JSON value returns `_error(None, -32600, "invalid request")` instead of raising `AttributeError` (path: `sidecar/jobspy_mcp/jobspy_mcp_server/server.py`).
- [ ] T013 [US2] Wrap per-request handling in `serve` with a top-level `except Exception` guard: write a `-32603` error with the current request id, emit a short diagnostic to stderr, and continue with the next line instead of terminating (path: `sidecar/jobspy_mcp/jobspy_mcp_server/server.py`).
- [ ] T014 [US2] Add a top-level `main` guard and a `sys.excepthook` global exception handler: emit a final `{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"internal error"}}` to stdout, write a sanitized diagnostic to stderr, and return normally instead of printing a traceback or exiting abruptly (path: `sidecar/jobspy_mcp/jobspy_mcp_server/server.py`).
- [ ] T015 [US2] Handle broken stdout/pipe writes and control-character-containing diagnostics safely: never write a traceback to stdout, flush successful responses, and fall back to stderr when the protocol stream is unavailable (path: `sidecar/jobspy_mcp/jobspy_mcp_server/server.py`).
- [ ] T016 [US2] Run `pytest sidecar/jobspy_mcp/tests` and the Rust MCP contract tests; confirm the sidecar still answers `initialize`/`tools/call` and that the Python changes do not alter the JSON-RPC success payloads (path: `sidecar/jobspy_mcp/tests/test_server.py`).
- [ ] T017 [P] [US2] Document the sidecar error contract in `README.md`: `-32600` invalid request, `-32603` internal error, diagnostics on stderr only, and the fact that the Rust client sanitizes and forwards stderr to local telemetry (path: `README.md`).

**Checkpoint**: A malformed request or unexpected Python exception produces a
JSON-RPC error and does not take down the sidecar; stdout remains protocol-only.

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: End-to-end validation, troubleshooting docs, and secret/path
safety across both stories.

- [ ] T018 [P] Add a README troubleshooting section for `MCP_PIPE_FAILED`: how to reproduce with a missing sidecar dependency, where the `SOURCE_FAILURE` event is written, the JSON-RPC error codes, and why this project uses local JSONL telemetry instead of MongoDB (path: `README.md`).
- [ ] T019 Run the full local gate: `./scripts/verify.sh` and `pytest sidecar/jobspy_mcp/tests`; report exactly which checks ran and which could not run (path: `scripts/verify.sh`).
- [ ] T020 [P] Scan the new fake-sidecar fixtures, Python server tests, and README for real filesystem paths, API keys, tokens, or raw provider output; replace them with placeholders and confirm the Rust sanitizer redacts them (paths: `src-tauri/tests/common/mod.rs`, `sidecar/jobspy_mcp/tests/test_server.py`).
- [ ] T021 Perform manual acceptance: run a temporary sidecar that raises on import or startup, confirm `MCP_PIPE_FAILED` contains the sanitized cause and a `SOURCE_FAILURE` telemetry event is written; then run the real JobSpy sidecar and confirm a normal `search_jobs` call still succeeds (paths: `src-tauri/src/ingestion/mcp.rs`, `sidecar/jobspy_mcp/jobspy_mcp_server/server.py`).

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies; T001 must pass before code changes.
- **Foundational (Phase 2)**: No tasks; both stories start after T001.
- **US1 (Phase 3)**: Independent of US2. T002-T004 are tests/helpers; T005-T010
  implement and validate the Rust path.
- **US2 (Phase 4)**: Independent of US1. T011 is the Python test contract;
  T012-T017 implement and validate the sidecar.
- **Polish (Phase 5)**: Depends on US1 and US2 completion.

### User Story Dependencies

- **US1 (P1)**: Can start immediately after T001. No dependency on US2 because
  its tests use fake Python sidecars.
- **US2 (P2)**: Can start immediately after T001. No dependency on US1 because
  the Python tests use `StringIO` and monkeypatched handlers.

### Within Each User Story

- Tests must be written and fail before implementation.
- US1: T002 before T003; T004 is independent of T002/T003; T006 before T007 and
  T008; T005 can run in parallel with T006; T009 is the validation gate; T010 is
  documentation.
- US2: T011 before T012-T014; T012 before T013; T014 and T015 are independent
  of the per-request guard but share `server.py`, so run them sequentially;
  T016 is the validation gate; T017 is documentation.

### Parallel Opportunities

- T002 and T004 can run in parallel (different files); T003 follows T002.
- T005 can run in parallel with T006-T008 (different files).
- T010 can run in parallel with T009 once the implementation is complete.
- US1 and US2 can run in parallel after T001 (Rust vs Python files).
- T011 can run in parallel with the US1 tasks after T001.
- T017 and T018 can run in parallel with remaining implementation tasks in their
  phases because they touch `README.md` only; do not start them before the
  behavior they document is settled.
- T018 and T020 can run in parallel during Polish.

---

## Parallel Example: User Story 1

```bash
# Tests/helpers first:
# T002 -> src-tauri/tests/common/mod.rs
# T004 -> src-tauri/tests/telemetry_events_ingestion.rs
# Then T003 -> src-tauri/tests/mcp_client_contract.rs

# Implementation:
# T005 -> src-tauri/src/telemetry/mod.rs
# T006-T008 -> src-tauri/src/ingestion/mcp.rs (sequential)
```

## Parallel Example: User Story 2

```bash
# Python contract tests first:
pytest sidecar/jobspy_mcp/tests

# Implementation in server.py is sequential:
# T012 -> request shape validation
# T013 -> per-request guard
# T014 -> main/global handler
# T015 -> broken-pipe handling
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete T001 baseline.
2. Complete US1 tests and implementation (T002-T010).
3. **STOP and VALIDATE**: run the fake crashing sidecar and the stderr-flood
   sidecar; confirm the sanitized cause, telemetry event, and no deadlock.
4. Deliver the Rust diagnostics fix.

### Incremental Delivery

1. Deliver US1 so a sidecar crash is no longer silent.
2. Deliver US2 so the sidecar returns JSON-RPC errors instead of crashing.
3. Run the Polish phase and the full local gate.
4. Each story is independently testable and leaves the other transport path
   untouched.

---

## Notes

- `[P]` tasks touch different files and can be assigned in parallel.
- The bug report mentions MongoDB, but this project has no MongoDB and forbids
  external telemetry. The tasks intentionally use the local
  `tracing`/`telemetry` JSONL layer; do not add a MongoDB dependency.
- Keep stdout reserved for JSON-RPC messages. Sidecar diagnostics belong on
  stderr; Rust captures them and sanitizes before telemetry or error display.
- Do not log or commit raw stderr, credentials, absolute paths, URLs, resume
  text, job descriptions, or full tracebacks. Use placeholders in fixtures.
- A sidecar stderr flood must still be drained after the retention cap; dropping
  the reader would reintroduce the deadlock the bug report describes.
- Preserve the existing `MCP_PIPE_FAILED` code for compatibility unless a test
  explicitly requires a new code. The diagnostic excerpt is additive.
