---

description: "Task list for parallelizing MCP job searches"
---

# Tasks: Parallel MCP Job Search

**Input**: Request "the search between multiples MCP could be parallelized?".
No `spec.md` or `plan.md` exists for this feature, so the tasks are derived
from the request and the current Rust/Python source. The active feature pointer
still targets `specs/008-profile-layout-role-conflicts`, which is unrelated; this
list is intentionally self-contained and must not modify that feature.

**Prerequisites**: Rust toolchain, Python `3.14`, the sidecar test suites under
`sidecar/jobspy_mcp/tests/`, and `uv`. No credentials are needed for the fake
sidecar tests; a real run uses the local `.env` and must never print it.

**Tests**: Required by the project rules for new behavior and failure paths.
Each behavior change needs a test that fails before the implementation.

**Organization**: `US1` (P1) parallelizes the boards inside the JobSpy sidecar,
which is the largest wall-clock win. `US2` (P2) pipelines the per-seniority
query variants and therefore depends on making the sidecar server concurrent.
The two stories touch different transports and can ship independently.

## Root Cause Notes (verified 2026-11-11)

- **Source-level parallelism already exists**: `ingest()` builds one future per
  adapter, runs them through `join_all`, and caps concurrency at
  `adapters.len().clamp(1, 4)` in `src-tauri/src/ingestion/mod.rs`. With the four
  enabled sources, every adapter already runs concurrently. Do not re-parallelize
  this layer; document it instead.
- **JobSpy boards are sequential**: `search_jobs` loops `for site in requested`
  and waits for each `scrape_fn` before starting the next in
  `sidecar/jobspy_mcp/jobspy_mcp_server/search.py`. Nine boards at 10-40s each
  dominate the run.
- **Query variants are sequential**: `McpAdapter::exchange` writes one
  `tools/call`, waits for its id, then writes the next query in
  `src-tauri/src/ingestion/mcp.rs`. `MAX_RESPONSE_LINES = 20` assumes one
  request at a time.
- **The sidecar server is synchronous**: `serve` reads one line, calls `handle`,
  writes one response, then reads the next in
  `sidecar/jobspy_mcp/jobspy_mcp_server/server.py`. Client-side pipelining alone
  would only queue inside the child process.
- **Rate limiting is not thread-safe**: `RateLimiter._last` is a plain field on
  a dataclass. Parallel board calls sharing one limiter race and can burst past
  `min_interval_seconds`.
- **Response matching is id-based**: one stdout stream is shared by all calls,
  so parallel requests must carry unique ids and the reader must buffer
  out-of-order responses. A failed id must not discard the other results.
- **Descriptions make boards heavier**: the recent `fetch_descriptions` default
  adds one detail request per posting, increasing both latency and 429 risk.
  Concurrency must be bounded and configurable, not unbounded per board.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete
  task).
- **[Story]**: `[US1]` for board concurrency, `[US2]` for query pipelining.
- Every task includes an exact file path.

## Path Conventions

- Rust backend: `src-tauri/src/` and `src-tauri/tests/`
- Python sidecar: `sidecar/jobspy_mcp/jobspy_mcp_server/` and
  `sidecar/jobspy_mcp/tests/`
- Configuration: `sources.yaml`, `src-tauri/src/config/sources.rs`
- Feature docs: `specs/009-parallel-mcp-search/`

---

## Phase 1: Setup (Baseline)

**Purpose**: Establish the current timing and test baseline before changing
concurrency.

- [ ] T001 Run `pytest sidecar/jobspy_mcp/tests` and
  `cargo test --manifest-path src-tauri/Cargo.toml --test mcp_client_contract`;
  record the exact commands, pass counts, and the wall-clock time of one
  `search_jobs` call with the current sequential board loop (path:
  `specs/009-parallel-mcp-search/tasks.md`).

**Checkpoint**: Baseline is known and no unrelated failures are mixed in.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Make the shared rate limiter safe before any board runs in
parallel.

- [ ] T002 Make `RateLimiter` thread-safe in
  `sidecar/jobspy_mcp/jobspy_mcp_server/search.py`: guard `_last` with a
  `threading.Lock`, keep `min_interval_seconds` semantics, and add a test in
  `sidecar/jobspy_mcp/tests/test_search.py` that starts two threads and proves
  the second call waits out the interval instead of bursting.

**Checkpoint**: Board work can run in parallel without violating per-board
pacing.

---

## Phase 3: User Story 1 - Search JobSpy boards concurrently (Priority: P1) MVP

**Goal**: A single JobSpy `tools/call` searches up to `site_concurrency` boards
at once, preserves per-board failure isolation, and returns results in a
deterministic order.

**Independent Test**: Run the sidecar suite with a fake scrape function that
blocks on a `threading.Barrier`; confirm two boards overlap when
`site_concurrency=2`, never exceed the limit, keep submitted order, and still
isolate one failing board.

### Tests for User Story 1

- [ ] T003 [P] [US1] Add sidecar tests in
  `sidecar/jobspy_mcp/tests/test_search.py` for: two boards overlapping with
  `site_concurrency=2` (barrier/event), the limit never exceeded when four
  boards are requested, deterministic output order, one failing board not
  discarding the others, and `fetch_description=True` still forwarded to every
  board.
- [ ] T004 [P] [US1] Add a Rust contract test in
  `src-tauri/tests/mcp_client_contract.rs` asserting the adapter sends
  `site_concurrency` (default `2`) and the value configured on the source.

### Implementation for User Story 1

- [ ] T005 [US1] Implement bounded board concurrency in
  `sidecar/jobspy_mcp/jobspy_mcp_server/search.py` with
  `concurrent.futures.ThreadPoolExecutor(max_workers=site_concurrency)` and
  `executor.map`; give each board its own `RateLimiter`, collect failures per
  board, and keep the existing dedup by `source_name:external_id`.
- [ ] T006 [US1] Add `site_concurrency` to the `search_jobs` input schema as an
  integer with minimum `1`, maximum `8`, default `2`, and pass it through in
  `sidecar/jobspy_mcp/jobspy_mcp_server/server.py`.
- [ ] T007 [US1] Add a `site_concurrency: u32` field to `McpAdapter` in
  `src-tauri/src/ingestion/mcp.rs`, read with
  `source.config_u32("site_concurrency", 2).clamp(1, 8)`, and include it in the
  `tools/call` arguments.
- [ ] T008 [P] [US1] Add `site_concurrency: 2` with a short comment to the
  `jobspy` source in `sources.yaml` and to `default_sources()` in
  `src-tauri/src/config/sources.rs`.
- [ ] T009 [US1] Run the focused suites
  (`pytest sidecar/jobspy_mcp/tests` and
  `cargo test --manifest-path src-tauri/Cargo.toml --test mcp_client_contract`)
  and a real one-query probe; record the sequential vs `site_concurrency=2`
  wall-clock time and any 429s (path:
  `sidecar/jobspy_mcp/jobspy_mcp_server/search.py`).

**Checkpoint**: The JobSpy sidecar searches boards concurrently with bounded,
configurable load and no behavior change in the payload shape.

---

## Phase 4: User Story 2 - Pipeline MCP query variants (Priority: P2)

**Goal**: Send all Trainee/Junior/Pleno query variants to one MCP source without
waiting for each response, while the sidecar processes requests concurrently and
every response is matched by id.

**Independent Test**: A fake sidecar answers three ids out of order; `fetch`
returns every job, merges them in query order, and remains under the read
timeout. A sidecar test proves two `tools/call` requests can be in flight at
once.

### Tests for User Story 2

- [ ] T010 [P] [US2] Create `sidecar/jobspy_mcp/tests/test_server.py` with tests
  that feed two requests into `serve`, confirm both are handled with matching
  ids (out-of-order allowed), diagnostics stay on stderr, and stdout contains
  only JSON-RPC lines.
- [ ] T011 [P] [US2] Add a Rust contract test in
  `src-tauri/tests/mcp_client_contract.rs` using a fake sidecar that answers
  ids out of order; assert `fetch` returns all query results in query order and
  does not treat a late response as an invalid response.

### Implementation for User Story 2

- [ ] T012 [US2] Make `serve` dispatch each request to a bounded
  `ThreadPoolExecutor` and serialize stdout writes with a `threading.Lock` in
  `sidecar/jobspy_mcp/jobspy_mcp_server/server.py`; keep stdout protocol-only
  and stderr for diagnostics.
- [ ] T013 [US2] Pipeline query calls in `McpAdapter::exchange`
  (`src-tauri/src/ingestion/mcp.rs`): write every `tools/call` with a stable
  unique id first, read until all ids are answered, buffer out-of-order
  responses, and replace the fixed `MAX_RESPONSE_LINES = 20` cap with a bound
  derived from the number of pending ids plus a small notification allowance.
- [ ] T014 [US2] Keep the source pacer effective with pipelining: document and
  implement whether `SourcePacer::wait` runs once per batch or once per request
  in `src-tauri/src/ingestion/mcp.rs` and
  `src-tauri/src/ingestion/mod.rs`; never bypass `requests_per_minute`.
- [ ] T015 [US2] Make pipelining opt-in with `queries_parallel: bool` (default
  `false`) on the MCP source, wired through `sources.yaml` and
  `default_sources()` in `src-tauri/src/config/sources.rs`, so synchronous
  sidecars keep the current behavior.
- [ ] T016 [US2] Run the focused suites and a manual probe with a fake slow
  handler; record the sequential vs pipelined timings and confirm no lost or
  duplicated jobs (path:
  `src-tauri/src/ingestion/mcp.rs`).

**Checkpoint**: Query variants can overlap end-to-end when both the client and
the sidecar opt in, without losing response matching or pacing.

---

## Phase 5: Polish & Cross-Cutting Concerns

- [ ] T017 [P] Update `README.md`: state that sources already run concurrently
  (cap 4), document `site_concurrency` and `queries_parallel`, and add a
  troubleshooting note for 429s (lower `site_concurrency`, disable
  `fetch_descriptions`, or raise `timeout_seconds`).
- [ ] T018 Run `./scripts/verify.sh` and both sidecar pytest suites; report
  exactly which checks ran and which could not run.
- [ ] T019 Manual acceptance: run one real search with `site_concurrency: 1`
  and then `2`, compare wall time, source failures, ingested count, and
  duplicates; confirm no job is lost or duplicated.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies; T001 must pass before code changes.
- **Foundational (Phase 2)**: T002 blocks T005 because parallel boards share the
  limiter behavior.
- **US1 (Phase 3)**: Depends on T002. T003/T004 are tests; T005-T008 implement;
  T009 validates. US1 is the MVP.
- **US2 (Phase 4)**: Depends on T002. T010/T011 are tests written first; T012
  makes the server concurrent and must land before T013 yields a real speedup;
  T014/T015 guard pacing and opt-in behavior.
- **Polish (Phase 5)**: Depends on US1 and US2.

### User Story Dependencies

- **US1 (P1)**: Independent of US2; only the sidecar and the generic MCP adapter
  change.
- **US2 (P2)**: Independent of US1's board loop, but shares `server.py` and
  `mcp.rs`; run the two stories sequentially to avoid conflicting edits.

### Within Each User Story

- Tests fail before implementation.
- US1: T002 -> T003/T004 -> T005 -> T006/T007 -> T008 -> T009.
- US2: T010/T011 -> T012 -> T013 -> T014/T015 -> T016.

### Parallel Opportunities

- T003 and T004 touch different files and can run in parallel.
- T008 is documentation/config and can run in parallel with T006/T007.
- T010 and T011 touch different files and can run in parallel; both are tests
  written before T012.
- T017 is README-only and can run while the final validation runs.

---

## Parallel Example: User Story 1

```bash
# Tests first (different files):
# T003 -> sidecar/jobspy_mcp/tests/test_search.py
# T004 -> src-tauri/tests/mcp_client_contract.rs

# Implementation:
# T005/T006 -> sidecar/jobspy_mcp/jobspy_mcp_server/search.py and server.py
# T007 -> src-tauri/src/ingestion/mcp.rs
# T008 -> sources.yaml and src-tauri/src/config/sources.rs
```

## Parallel Example: User Story 2

```bash
# Tests first (different files):
# T010 -> sidecar/jobspy_mcp/tests/test_server.py
# T011 -> src-tauri/tests/mcp_client_contract.rs

# Then the server and the client:
# T012 -> sidecar/jobspy_mcp/jobspy_mcp_server/server.py
# T013/T014/T015 -> src-tauri/src/ingestion/mcp.rs and config/sources.rs
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete T001 and T002.
2. Add the failing sidecar and Rust tests (T003/T004).
3. Implement bounded board concurrency and the config plumbing (T005-T008).
4. Validate with T009 and ship US1. This alone gives the largest speedup.

### Incremental Delivery

1. Ship US1 with `site_concurrency` defaulting to `2`.
2. Ship US2 behind `queries_parallel: false`; enable it only after the server
   concurrency tests pass.
3. Run the Polish phase and compare real run metrics.

---

## Notes

- Do not add a second layer of source-level parallelism: `ingest()` already runs
  adapters concurrently with a cap of 4.
- `site_concurrency` must stay bounded; unbounded boards multiply 429s and
  memory use, especially with `fetch_descriptions: true`.
- Keep stdout reserved for JSON-RPC messages. Diagnostics belong on stderr.
- Preserve failure isolation: one board or query failing must not discard the
  results already collected.
- Do not log or commit raw descriptions, credentials, absolute paths, or full
  provider responses; reuse the existing sanitization.
- `queries_parallel` is opt-in because sidecars other than JobSpy may still be
  strictly synchronous.
