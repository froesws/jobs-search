---

description: "Task list for graceful MCP shutdown and exit-143 diagnostics"
---

# Tasks: MCP LinkedIn Exit 143 and Graceful Shutdown

**Input**: Bug report: the `linkedin-scraper` and `linkedin` MCP servers report
`Status: failed` / `Connection closed: MCP server process exited with code 143`.
The `linkedin-scraper` log includes the benign
`VIRTUAL_ENV=.venv does not match ...` warning, and the `linkedin` log shows the
FastMCP banner followed by termination.

No `spec.md` or `plan.md` exists for this feature, so the tasks are derived from
the report and the current source. The active feature pointer still targets
`specs/008-profile-layout-role-conflicts`, which is unrelated; this list is
self-contained and must not modify that feature.

**Prerequisites**: Rust toolchain, Python `3.14`, `uv`, and both sidecar test
suites. Never print or commit `.env`, the saved LinkedIn session, credentials, or
raw job descriptions. Do not kill processes by command name; always verify the
parent PID or process group first.

**Tests**: Required by the project rules for new behavior and failure paths.

**Organization**: `US1` (P1) makes our sidecars exit 0 on SIGTERM/SIGINT. `US2`
(P1) fixes child-environment hygiene. `US3` (P2) records the exit signal in
sanitized diagnostics. `US4` (P2) documents the OpenCode configuration and safe
cleanup. The upstream `linkedin` FastMCP server is third-party and cannot be
patched here; its task is documentation/configuration only.

## Root Cause Notes (verified 2026-11-11)

- **143 means SIGTERM, not a crash**: 128 + 15. The processes were children of
  `opencode serve` (configuration keys `mcp.servers.linkedin` and
  `mcp.servers.linkedin-scraper` in `.opencode/opencode.jsonc`), started at
  01:13, and were still running until an external `kill` sent SIGTERM during a
  manual cleanup. The FastMCP banner and the completed `uv` install prove the
  servers had started.
- **OpenCode reports any non-zero exit as failed**: a graceful SIGTERM with exit
  code 143 is rendered as `Status: failed` / `Connection closed`.
- **Our sidecars do not trap SIGTERM**: `main()` in
  `sidecar/jobspy_mcp/jobspy_mcp_server/server.py` and
  `sidecar/linkedin_scraper_mcp/linkedin_scraper_mcp/server.py` calls `serve()`
  with no signal handling, so the default action terminates with 143.
- **The `linkedin` server is third-party**: `.opencode/opencode.jsonc` runs
  `uvx mcp-server-linkedin@latest` (FastMCP 4.1.8). This repository cannot add
  signal handlers to that package; only documentation/configuration can change.
- **`VIRTUAL_ENV` leaks into children**: the app spawns `uv run` without
  clearing `VIRTUAL_ENV`, so `uv` prints the mismatch warning and may choose the
  wrong environment. `.opencode/opencode.jsonc` also inherits OpenCode's env, so
  the warning appears there too.
- **No exit-status diagnostics**: `read_message` returns a generic
  `MCP_PIPE_FAILED` / `sidecar exited before responding` when stdout closes early
  (`src-tauri/src/ingestion/mcp.rs`), so 143/SIGTERM cannot be distinguished
  from a real crash. This complements `specs/006-mcp-sidecar-diagnostics`.
- **Unsafe cleanup caused the incident**: killing by PID from `ps | grep` hit
  processes owned by `opencode serve`, not the app. Cleanup must be scoped to the
  app's own children/process group.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete
  task).
- **[Story]**: `[US1]` graceful signals, `[US2]` env hygiene, `[US3]`
  diagnostics, `[US4]` documentation/safe operations.
- Every task includes an exact file path.

## Path Conventions

- Python sidecars: `sidecar/jobspy_mcp/` and `sidecar/linkedin_scraper_mcp/`
- Rust backend: `src-tauri/src/` and `src-tauri/tests/`
- OpenCode configuration: `.opencode/opencode.jsonc`
- Feature docs: `specs/011-mcp-exit-143-shutdown/`

---

## Phase 1: Setup (Baseline)

**Purpose**: Reproduce the reported exit code and prove process ownership before
changing anything.

- [x] T001 Reproduce the report without killing anything: start
  `jobspy-mcp-server` and `linkedin-scraper-mcp`, send `SIGTERM` to the child you
  started, and record `$?` (expect `143`) plus stderr. Start
  `uvx mcp-server-linkedin@latest` the same way and record its code. Also record
  the `PPID`/`PGID` of the existing OpenCode-managed servers so cleanup can be
  scoped (paths: `sidecar/jobspy_mcp/jobspy_mcp_server/server.py`,
  `sidecar/linkedin_scraper_mcp/linkedin_scraper_mcp/server.py`,
  `.opencode/opencode.jsonc`).

**Checkpoint**: The signal and the owning parent are known; no unrelated process
was touched.

---

## Phase 2: Foundational (Blocking Prerequisites)

No shared implementation prerequisites are required. US1 and US2 are
independent; US3 depends on US1's shutdown reason being observable; US4 is
documentation. Do not add a new dependency for any story.

**Checkpoint**: T001 is complete; US1 and US2 can start in parallel.

---

## Phase 3: User Story 1 - Sidecars exit cleanly on SIGTERM (Priority: P1)

**Goal**: `jobspy-mcp-server` and `linkedin-scraper-mcp` stop on SIGTERM/SIGINT
with exit code 0, a single sanitized stderr line, and no traceback, so a
supervisor does not report a crash.

**Independent Test**: Spawn each installed entry point, complete `initialize`,
send `SIGTERM`, and assert exit code 0, no traceback, protocol-only stdout, and
one shutdown diagnostic on stderr.

### Tests for User Story 1

- [x] T002 [P] [US1] Create
  `sidecar/jobspy_mcp/tests/test_signals.py`: spawn `jobspy-mcp-server`, read the
  `initialize` response, send `SIGTERM`, and assert exit code 0, no `Traceback`
  in stderr, stdout contains only JSON-RPC lines, and a shutdown line reaches
  stderr.
- [x] T003 [P] [US1] Create
  `sidecar/linkedin_scraper_mcp/tests/test_signals.py` with the same contract
  for `linkedin-scraper-mcp`, including the case where a `tools/call` has not
  finished yet.

### Implementation for User Story 1

- [x] T004 [US1] Add a `SIGTERM`/`SIGINT` handler in
  `sidecar/jobspy_mcp/jobspy_mcp_server/server.py`: set a shutdown event used by
  `serve`, flush stdout, write one sanitized line to stderr, and exit 0; install
  it in `main()` before `serve()` and keep `serve` callable with `StringIO` in
  tests.
- [x] T005 [US1] Apply the same handler contract to
  `sidecar/linkedin_scraper_mcp/linkedin_scraper_mcp/server.py`, including
  cancelling an in-flight `asyncio` search without writing an error frame after
  the process starts shutting down.
- [x] T006 [US1] Run `pytest sidecar/jobspy_mcp/tests` and
  `pytest sidecar/linkedin_scraper_mcp/tests`, then repeat the T001 SIGTERM probe
  and confirm `0` instead of `143` (paths:
  `sidecar/jobspy_mcp/jobspy_mcp_server/server.py`,
  `sidecar/linkedin_scraper_mcp/linkedin_scraper_mcp/server.py`).

**Checkpoint**: Our sidecars exit 0 on an orderly shutdown; only the third-party
`linkedin` server still exits 143.

---

## Phase 4: User Story 2 - Do not leak `VIRTUAL_ENV` to children (Priority: P1)

**Goal**: The app starts sidecars without the host's `VIRTUAL_ENV`, so `uv` never
prints the mismatch warning or resolves the wrong environment.

**Independent Test**: A fake sidecar reports its `VIRTUAL_ENV`; a child spawned by
both MCP adapters reports it unset even when the parent process has it set.

### Tests for User Story 2

- [x] T007 [P] [US2] Add a Rust contract test in
  `src-tauri/tests/mcp_client_contract.rs` with a fake sidecar that puts
  `os.environ.get("VIRTUAL_ENV")` in the job title; assert the adapter-spawned
  child sees `None` while the test process sets `VIRTUAL_ENV=.venv`.
- [x] T008 [P] [US2] Add the equivalent test for
  `src-tauri/tests/linkedin_mcp_contract.rs` so the specialized adapter is
  covered too.

### Implementation for User Story 2

- [x] T009 [US2] Call `.env_remove("VIRTUAL_ENV")` on the spawned `Command` in
  `src-tauri/src/ingestion/mcp.rs` and
  `src-tauri/src/ingestion/linkedin_mcp.rs`, before `spawn`; keep the rest of the
  environment inherited.
- [x] T010 [US2] Run both contract test files and start one real source; confirm
  the `VIRTUAL_ENV` mismatch warning no longer appears in the app's captured
  sidecar stderr (paths: `src-tauri/src/ingestion/mcp.rs`,
  `src-tauri/src/ingestion/linkedin_mcp.rs`).

**Checkpoint**: App-spawned sidecars run in the project environment without
warnings; the OpenCode-launched servers are unchanged.

---

## Phase 5: User Story 3 - Report the exit signal in diagnostics (Priority: P2)

**Goal**: An unexpected MCP exit reports `code=143`/`signal=SIGTERM` in the
sanitized failure message and telemetry, so shutdown is distinguishable from a
crash without exposing stderr.

**Independent Test**: A fake sidecar that exits with a known code before answering
`initialize` produces `MCP_PIPE_FAILED` containing the sanitized status, and a
`SOURCE_FAILURE` telemetry event with `error_category = "IO"`.

### Tests for User Story 3

- [x] T011 [P] [US3] Add a contract test in
  `src-tauri/tests/mcp_client_contract.rs` where a fake sidecar exits with code
  `143` before responding; assert the error keeps `MCP_PIPE_FAILED` and includes
  `code=143`/`SIGTERM` without raw stderr.
- [ ] T012 [P] [US3] Add a telemetry assertion in
  `src-tauri/tests/telemetry_events_ingestion.rs` for the sanitized exit status.

### Implementation for User Story 3

- [x] T013 [US3] Capture `child.wait()` status in
  `src-tauri/src/ingestion/mcp.rs` and
  `src-tauri/src/ingestion/linkedin_mcp.rs`; when a read fails because stdout
  closed, append a sanitized `status=<code>`/`signal=<n>` to the existing
  `MCP_PIPE_FAILED` message and emit the existing ingestion telemetry event. Do
  not change the retryability or the error code.
- [x] T014 [US3] Run the MCP and telemetry suites and confirm the diagnostic is
  present for a pre-handshake exit and absent for a normal successful exchange
  (paths: `src-tauri/src/ingestion/mcp.rs`,
  `src-tauri/src/ingestion/linkedin_mcp.rs`).

**Checkpoint**: 143/SIGTERM is visible in the failure without leaking stderr.

---

## Phase 6: User Story 4 - Document OpenCode configuration and safe cleanup (Priority: P2)

**Goal**: Users can tell which servers belong to which process, why 143 happens,
and how to restart or disable the duplicate LinkedIn servers without killing
unrelated processes.

**Independent Test**: Following the README, a user can identify the owning
parent, avoid killing the app's or OpenCode's processes by name, restart the
reported servers, and know that the `VIRTUAL_ENV` warning is benign.

### Implementation for User Story 4

- [x] T015 [P] [US4] Add a README troubleshooting section: `143 = SIGTERM`, it
  is an orderly shutdown (supervisor, config reload, or manual cleanup), the
  FastMCP banner/`uv` install output is startup noise, and an app restart or
  OpenCode reload reconnects the server (path: `README.md`).
- [ ] T016 [US4] Document the `.opencode/opencode.jsonc` entries for `linkedin`
  and `linkedin-scraper`: they are independent from `sources.yaml`, the
  `linkedin` one is third-party and cannot trap signals, and either can be
  disabled if duplicate servers are undesirable (path:
  `.opencode/opencode.jsonc`, `README.md`).
- [ ] T017 [P] [US4] Add a safe cleanup recipe (no new script unless needed):
  list `pid,ppid,pgid,args`, confirm the parent is the app/`opencode serve`, and
  never use `pkill -f linkedin`; scope termination to the child's process group
  (paths: `README.md`, `scripts/verify.sh`).
- [ ] T018 [US4] Re-read the documentation and verify every referenced command
  and path exists; do not include machine-specific absolute paths (path:
  `README.md`).

**Checkpoint**: The report's failure mode is explainable and reproducible from
the docs, and cleanup cannot hit unrelated servers.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T019 Run `./scripts/verify.sh`, `pytest sidecar/jobspy_mcp/tests`, and
  `pytest sidecar/linkedin_scraper_mcp/tests`; report exactly which checks ran
  and which could not run (path: `scripts/verify.sh`).
- [ ] T020 Manual acceptance: start both local sidecars, send SIGTERM, confirm
  exit 0; start the upstream `linkedin` server, confirm 143 is documented and
  recoverable; run an app search and confirm no `VIRTUAL_ENV` warning; confirm
  app shutdown terminates only its own children (paths:
  `src-tauri/src/ingestion/mcp.rs`, `.opencode/opencode.jsonc`).

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: T001 must pass before code changes.
- **US1 (Phase 3)**: Independent of US2; Python only.
- **US2 (Phase 4)**: Independent of US1; Rust adapters only.
- **US3 (Phase 5)**: Depends on US1 for a meaningful shutdown reason; shares
  `mcp.rs` with US2, so run after US2.
- **US4 (Phase 6)**: Independent; documentation only.
- **Polish (Phase 7)**: Depends on US1-US4.

### User Story Dependencies

- **US1 (P1)**: No dependency.
- **US2 (P1)**: No dependency.
- **US3 (P2)**: After US1/US2 to avoid conflicting edits in `mcp.rs`.
- **US4 (P2)**: No code dependency; can run in parallel.

### Within Each User Story

- Tests fail before implementation.
- US1: T002/T003 -> T004/T005 -> T006.
- US2: T007/T008 -> T009 -> T010.
- US3: T011/T012 -> T013 -> T014.
- US4: T015 -> T016/T017 -> T018.

### Parallel Opportunities

- T002 and T003 touch different sidecar test suites.
- T007 and T008 touch different contract files.
- T011 and T012 touch different test files.
- T015, T016, and T017 are documentation-only and can run in parallel.
- US1 and US2 (Python vs Rust) can run in parallel after T001.

---

## Parallel Example: User Stories 1 and 2

```bash
# Signals (Python):
# T002 -> sidecar/jobspy_mcp/tests/test_signals.py
# T003 -> sidecar/linkedin_scraper_mcp/tests/test_signals.py
# T004/T005 -> both server.py files

# Environment hygiene (Rust):
# T007 -> src-tauri/tests/mcp_client_contract.rs
# T008 -> src-tauri/tests/linkedin_mcp_contract.rs
# T009 -> src-tauri/src/ingestion/{mcp,linkedin_mcp}.rs
```

---

## Implementation Strategy

### MVP First (User Story 1 + US2)

1. Complete T001.
2. Ship US1 so our servers exit 0 on SIGTERM instead of 143.
3. Ship US2 so `uv` stops warning about `VIRTUAL_ENV`.
4. Add US3 diagnostics and US4 documentation.

### Incremental Delivery

1. US1: local sidecars become supervisor-friendly.
2. US2: clean child environment; no warning noise.
3. US3: 143 becomes visible and diagnosable.
4. US4: users can restart/disable servers safely.

---

## Notes

- The upstream `linkedin` server is `uvx mcp-server-linkedin@latest`; do not try
  to patch its signal handling in this repository. Document it.
- Exit `143` must not be treated as a job-source crash; only an exit *before* a
  response is a source failure.
- Never kill by command name. Use `ps -o pid,ppid,pgid` and the owning parent or
  process group.
- Keep stdout reserved for JSON-RPC. Shutdown diagnostics go to stderr.
- Do not log or commit the saved LinkedIn session, tokens, raw stderr, or job
  descriptions.

---

---

## Execution Notes (2026-11-11)

- **T001 baseline**: both sidecars answered `initialize` and exited with
  signal 15 (shell code `143`) on `SIGTERM`. After the handlers they exit `0`
  with `shutting down on SIGTERM` on stderr.
- **T006**: signal contract suites passed (jobspy 14, linkedin-scraper 11).
- **T010**: child-environment tests passed (generic 8, LinkedIn 3).
- **T014**: exit-status test passed; the message includes `status=143`.
