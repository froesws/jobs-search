# Research: Faster Search

**Feature**: `015-faster-search` | **Date**: 2026-11-12

All `NEEDS CLARIFICATION` items from the Technical Context are resolved below.

---

## 1. Collection: skip known details instead of parallel fetches

**Decision**: Do not parallelize per-posting detail requests. Instead, the LinkedIn adapter
receives a known-posting set and skips `get_job_details` for any listed posting already stored
with a non-empty clean description. New postings and postings stored without a description are
still fetched in full.

**Rationale**: The authenticated LinkedIn MCP server is an external package that installs a
`SequentialToolExecutionMiddleware`; it serializes every tool call around a single Chromium
profile because two calls cannot drive the shared browser and two processes overwrite each
other's cookies. The Rust client is also a single stdio pipe with `call_tool(&mut self)` and a
`read_response(id)` loop, so concurrent calls would interleave responses. The scraper sidecar's
`serve()` loop handles one request at a time, and the JobSpy adapter has no client-side detail
loop (descriptions arrive in `search_jobs`). Parallel detail collection is therefore not
deliverable without replacing the browser model, which is out of scope.

**Alternatives considered**:
- Multiplex the Rust client and fork the server: the server would still serialize on the browser
  lock, and forking risks the login session.
- Spawn one sidecar per detail: N browser processes against one profile corrupt the cookies.
- Parallelize the JobSpy sidecar by site: possible, but a separate Python change with uncertain
  thread safety and rate-limit behavior; recorded as a future improvement.

**Affected files**: `ingestion/adapter.rs`, `ingestion/linkedin_mcp.rs`, `ingestion/mod.rs`,
`db/jobs.rs`.

---

## 2. Known-posting lookup

**Decision**: Load a global set of postings whose `external_id` starts with `linkedin-` and whose
`description_clean` is non-empty, regardless of which source stored them. The set carries
`external_id`, `job_id`, and `url`. The LinkedIn adapter normalizes each search ID to
`linkedin-<id>` before lookup. Skipped postings are returned as IDs; the ingestion layer uses the
set to touch the LinkedIn lineage.

**Rationale**: JobSpy already scans LinkedIn and stores `linkedin-<id>` external IDs. A
per-source lookup would re-download those postings through the LinkedIn adapter; the global
prefix catches them. The prefix avoids collisions with RemoteOK numeric IDs or URL hashes.
Descriptions are required so a posting first stored without a body can be enriched on a later
run.

**Alternatives considered**:
- Per-source `(source_id, external_id)`: simpler, but misses JobSpy → LinkedIn reuse.
- Global without a prefix: risks matching an unrelated board's ID.
- Sharing the set within the run: requires persisting during collection, which inverts the
  current collect-then-persist pipeline.

**Affected files**: `db/jobs.rs`, `ingestion/adapter.rs`, `ingestion/linkedin_mcp.rs`,
`commands/search.rs`.

---

## 3. Fallback delivery and start timing

**Decision**: A primary "delivers" when `new postings + already-known postings >=
fallback_min_results`. The fallback starts as soon as its own primary is decided; it does not
wait for unrelated primaries. Results are merged.

**Rationale**: Counting raw listed IDs would treat failed detail requests as delivery; counting
only new postings would start the scraper on every run whose LinkedIn top results are already
stored. Delivery must include known postings but not failures. Per-primary orchestration avoids
a fast LinkedIn failure waiting behind a slow JobSpy run.

**Alternatives considered**:
- Global two-phase barrier: simpler, but delays the fallback by the slowest primary.
- Speculative parallel fallback: wastes the scraper browser on every healthy run.
- Counting only new postings: reintroduces the exact cost the feature removes.

**Affected files**: `ingestion/mod.rs`, `config/sources.rs`, `commands/search.rs`.

---

## 4. Known skip and error semantics

**Decision**: If all new details fail but at least one known posting was skipped, the source is
not an error: `delivered` includes the known postings. If there are no known postings and every
detail fails, the existing `LINKEDIN_DETAILS_FAILED` behavior applies and the fallback runs.
Skipped postings update `job_posting_sources.ingested_at` but do not rewrite the posting.

**Rationale**: The source did deliver the known postings. Returning an error would start the
fallback unnecessarily and misreport the source. Updating lineage keeps New Jobs ordering and
Analytics windows truthful, matching the current "seen in this run" semantics.

**Alternatives considered**:
- Treat any detail failure as a source failure: noisy and starts the fallback on partial runs.
- Do not touch lineage: skipped postings silently age out of the list and analytics.
- Rewrite `updated_at`: would reorder postings without a content change.

**Affected files**: `ingestion/linkedin_mcp.rs`, `ingestion/mod.rs`, `db/jobs.rs`.

---

## 5. Evaluation concurrency

**Decision**: Raise the default from `2` to `4` and widen the accepted range to `1..=8`. Resolve
the effective value at the start of each run from `app_settings` key `evaluation.concurrency`,
falling back to `JOBS_SEARCH_LLM_CONCURRENCY` (default `4`). Surface it as a global numeric field
in the provider settings dialog and persist it through `cmd_save_provider_config`.

**Rationale**: The evaluation queue is the dominant cost; the code already uses
`buffer_unordered`, so the change is a bound. The provider settings dialog is the existing
settings surface. `AppState` has a `RwLock<AppConfig>` and a `replace_config` method, but the
method currently has **no callers**; relying on the lock would not apply a saved change. Reading
the persisted key at run start is the reliable path.

**Alternatives considered**:
- Going straight to `8` by default: risks 429 storms; better as an operator opt-in.
- Per-provider concurrency: requires separate semaphores and complicates the queue; the gain over
  a conservative global value is small.
- `.env` only: requires editing a file and a restart, contradicting FR-004.

**Affected files**: `config/env.rs`, `config/provider_settings.rs`, `commands/config.rs`,
`commands/search.rs`, `match/mod.rs`, `src/components/ProviderConfigDialog.tsx`.

---

## 6. Progress reporting

**Decision**: Emit a Tauri event `search://progress` from `commands/search.rs` using
`AppHandle::emit`. Payload: `{ stage, found, evaluated, total, failed }`, where `stage` is
`collecting` or `evaluating`. `found` counts listed postings, including skipped known ones;
`total` is the number of postings queued for evaluation. Emissions are coalesced to at most one
per 250 ms, plus one per stage transition and one final event. `evaluate_run` and `ingest` receive
a `ProgressSink = Arc<dyn Fn(SearchProgress) + Send + Sync>` so the domain layers stay independent
of Tauri.

**Rationale**: Events avoid a polling command and give the UI a push model. The sink keeps
`match/` and `ingestion/` testable (tests pass a counting closure). Aggregate counters avoid
exposing posting data and keep the payload small.

**Alternatives considered**:
- `cmd_get_search_progress` polled by the UI: adds a command, a timer, and stale reads.
- Writing progress rows to SQLite: violates the "no DB lock across awaits" rule.
- Per-source progress: useful for diagnosis but adds state and UI for a problem the final summary
  already reports.

**Capability note**: `core:default` in `src-tauri/capabilities/default.json` already grants the
event permissions; verify a real `listen()` call during implementation. If it fails, add
`core:event:default` explicitly.

**Affected files**: `commands/search.rs`, `ingestion/mod.rs`, `match/mod.rs`, `src/lib/types.ts`,
`src/viewmodels/useSearchProgress.ts`, `src/views/DashboardView.tsx`,
`src/views/NewJobsView.tsx`.

---

## 7. Per-source time budget

**Decision**: Add `budget_seconds` to `RateLimitConfig` (default `180`, clamped `30..=900`) and
wrap each source fetch in `tokio::time::timeout`. A timeout becomes
`SourceFailure { code: "SOURCE_TIMEOUT" }` and does not abort the run. `sources.yaml` raises
JobSpy to `300`, keeps LinkedIn and the scraper at `180`, and lowers RemoteOK to `30`.

**Rationale**: `timeout_seconds` already exists but bounds individual requests inside an adapter;
the budget bounds the whole source. The previous default of `300` equaled the current LinkedIn
request timeout and would rarely trigger. Sidecars use `kill_on_drop(true)`, so dropping a
timed-out fetch does not leave an orphan process.

**Alternatives considered**:
- Reusing `timeout_seconds` as the budget: conflates a request timeout with a source budget.
- Cancelling the run on the slowest source: throws away the other sources' results.
- Keeping `300` as the default: protects only against very long hangs.

**Affected files**: `config/sources.rs`, `ingestion/mod.rs`, `sources.yaml`, `README.md`.

---

## 8. Fallback result quality

**Decision**: Accept the scraper's postings as-is. It does not provide `posted_at` and ignores
`date_posted`/`experience_level`; the hard filters still bound the outcome. Do not mark them
`needs_review` and do not filter by date.

**Rationale**: The fallback is a safety net; partial results are better than none. The scraper
always sets `posted_at: None`, so a date filter would be unpredictable. `needs_review` currently
means metadata conflict/uncertainty; overloading it for source quality would make the badge noisy
and ambiguous.

**Alternatives considered**:
- Filter by `posted_at`: the field is always absent.
- Mark `needs_review`: conflates two different signals.
- Disable the fallback when date filters are active: defeats the safety net in the runs most
  likely to need it.

**Affected files**: `ingestion/mod.rs`, `README.md`, `specs/015-faster-search/spec.md`.

---

## 9. Testing strategy

**Decision**:
- `src-tauri/tests/ingestion_skip.rs`: known with a non-empty description skips the detail and
  touches lineage; known with an empty description fetches; a `linkedin-` posting stored by
  JobSpy satisfies the lookup; all-known counts as delivered.
- `src-tauri/tests/ingestion_fallback.rs`: primary delivers enough, empty, below minimum,
  failure, disabled primary, both fail, known-only primary, and per-primary start timing.
- `src-tauri/tests/ingestion_budget.rs`: an adapter that never resolves becomes
  `SOURCE_TIMEOUT` and the other adapter's postings survive.
- `src-tauri/tests/ingestion_concurrency.rs`: a fake provider counts concurrent calls; the max
  never exceeds the configured value and reaches it when enough jobs are queued; a rate-limit
  error preserves completed evaluations.
- `src-tauri/tests/search_progress.rs`: the emitted sequence starts with `collecting`, moves once
  to `evaluating`, is monotonic, and ends with a final event.
- `src-tauri/tests/linkedin_mcp_contract.rs`: the fake server counts `get_job_details` calls.
- `src/viewmodels/useSearchProgress.test.ts`: subscribes, updates, unsubscribes, and handles a
  failed subscription.
- `src/views/NewJobsView.test.tsx`: the progress line renders while searching and disappears when
  the run ends.
- `src/components/ProviderConfigDialog.test.tsx`: the concurrency field loads, validates `1..=8`,
  and saves.
- `src-tauri/tests/score_contract.rs` stays green as the proof that the skip and concurrency do
  not change outcomes.

**Rationale**: Matches the existing split between stdio contract tests, fake-adapter integration
tests, and Vitest component tests, and covers the failure paths the spec requires.

**Alternatives considered**:
- Timing assertions against real providers: non-deterministic and requires keys.
- Manual quickstart scenarios as a gate: the operator chose automated validation; manual runs are
  not part of the definition of done.

---

## 10. Configuration validation

**Decision**: Validate the fallback graph at load time. Reject a self-reference, an unknown
target, two fallbacks targeting the same primary, and a chain (a fallback that is itself the
primary of another fallback). Clamp `fallback_min_results` to `1..=50` and `budget_seconds` to
`30..=900`.

**Rationale**: A malformed graph should fail loudly at startup, not silently run the wrong source.
One level of fallback is enough for the LinkedIn pair; chains add ordering ambiguity and
unbounded run time.

**Alternatives considered**:
- Allow chains: more flexible, but harder to reason about and test; YAGNI.
- Ignore invalid targets: hides configuration mistakes.

**Affected files**: `config/sources.rs`, `sources.yaml`, `README.md`.
