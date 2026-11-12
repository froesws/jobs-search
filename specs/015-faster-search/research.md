# Research: Faster Search

**Feature**: `015-faster-search` | **Date**: 2026-11-11

All `NEEDS CLARIFICATION` items from the Technical Context are resolved below.

---

## 1. Parallel detail collection inside a source

**Decision**: Replace the sequential per-posting loops with
`futures::stream::iter(...).buffer_unordered(N)`, where `N` comes from a new per-source
`detail_concurrency` config (default `4`, clamped `1..=4`). Failures stay isolated: a failed
detail becomes a counted `detail_failures` increment, exactly as today.

**Rationale**: `buffer_unordered` is already used in `match/mod.rs` for evaluation, so the
pattern is established. A `Semaphore` would work too, but `buffer_unordered` bounds the
in-flight futures without extra bookkeeping.

**Alternatives considered**:
- `tokio::spawn` per posting: loses ordering guarantees, complicates error aggregation, and
  spawns unboundedly unless a semaphore is added anyway.
- Raising the source-level concurrency only: does nothing for the per-posting loop, which is
  where the time goes.

**Affected files**: `ingestion/linkedin_mcp.rs` (`get_job_details` loop),
`ingestion/mcp.rs` (per-query loop stays sequential; detail handling is provider-side there).

---

## 2. Evaluation concurrency

**Decision**: Raise the default from `2` to `4` and widen the accepted range to `1..=8`
(`config/env.rs` and `match/mod.rs`). Keep `JOBS_SEARCH_LLM_CONCURRENCY` as the knob and keep
the `RwLock`-backed config so a change applies on the next run without a restart.

**Rationale**: The evaluation queue is the dominant cost; the code already parallelizes with
`buffer_unordered`, so the change is a bound, not a rewrite. `4` is twice today's value, which
the spec's assumption allows.

**Alternatives considered**:
- Going straight to `8` by default: risks provider rate limits and 429 storms; better as an
  operator opt-in.
- Adaptive concurrency driven by observed 429s: valuable but a larger change; recorded as a
  future improvement.

---

## 3. Progress reporting

**Decision**: Emit a Tauri event `search://progress` from `commands/search.rs` using
`AppHandle::emit`. Payload: `{ stage, found, evaluated, total, failed }`, where `stage` is
`collecting` or `evaluating`. Emissions are coalesced: at most one per 250 ms, plus one on
every stage transition and one final event. `evaluate_run` receives a
`ProgressSink = Arc<dyn Fn(EvaluationProgress) + Send + Sync>` so the matching layer stays
independent of Tauri.

**Rationale**: Events avoid a polling command and give the UI a push model. The sink keeps
`match/` free of Tauri types, which preserves testability (tests pass a counting closure).

**Alternatives considered**:
- `cmd_get_search_progress` polled by the UI: adds a command, a timer, and reads that can
  lag behind reality.
- Writing progress rows to SQLite: violates the "no DB lock across awaits" rule and adds
  write amplification for a transient signal.
- A channel the UI reads: Tauri events are the idiomatic transport and need no extra
  capability beyond `core:default`.

**Capability note**: `core:default` in `src-tauri/capabilities/default.json` already grants
the event permissions; verify at implementation with a real `listen()` call.

---

## 4. Per-source time budget

**Decision**: Add `budget_seconds` to `RateLimitConfig` (default `300`, clamped `1..=900`)
and wrap each adapter call in `tokio::time::timeout`. A timeout becomes a
`SourceFailure { code: "SOURCE_TIMEOUT" }` and does not abort the run.

**Rationale**: `timeout_seconds` already exists but bounds individual requests inside an
adapter; the budget bounds the whole source, which is what FR-008 needs.

**Alternatives considered**:
- Reusing `timeout_seconds` as the budget: conflates a request timeout with a source budget
  and would shorten individual requests.
- Cancelling the run entirely on the slowest source: throws away the other sources' results.

---

## 5. Primary/fallback LinkedIn sources

**Decision**: Add two optional keys to `SourceConfig`:

- `fallback_for: <source_id>` — this source runs only when that primary did not deliver.
- `fallback_min_results` (inside `config`, default `1`) — the primary must return at least
  this many postings for the fallback to be skipped.

`ingest` becomes two-phase: run the primaries concurrently, then run the fallbacks whose
primary failed or returned fewer than the minimum. A fallback whose primary is missing or
disabled runs as a normal source. Results from a triggered fallback are **merged** with the
primary's (the primary may have returned some postings before failing).

**Rationale**: This is exactly the operator's request and it removes the duplicated
collection that currently happens on every run. Merging instead of replacing avoids losing
partial primary results.

**Alternatives considered**:
- Keeping both in parallel with a pre-detail dedup check against `job_posting_sources`:
  better coverage, but a larger refactor of the adapter contract (the adapter currently
  returns complete postings). Recorded as the next step if coverage ever matters.
- Disabling the scraper entirely: simplest, but loses the safety net.

**Known semantic difference**: the scraper source ignores date and experience filters, so a
run that used it can include older postings. Hard filters still bound the outcome; documented
in the spec assumptions.

---

## 6. Where the fallback decision is made

**Decision**: In `ingestion::ingest`, not in `commands/search.rs`. `ingest` receives the
adapters plus their `SourceConfig` metadata and returns `SourceOutcome`s for every source,
including the ones that were skipped (with `skipped: true` and no failure).

**Rationale**: Keeps the orchestration testable without Tauri or a database, and keeps the
command thin.

**Alternatives considered**:
- Deciding in the command: mixes transport with policy and makes the matrix hard to test.

---

## 7. Empty-primary detection

**Decision**: Treat a primary as "did not deliver" when it failed **or** returned fewer than
`fallback_min_results` postings, including zero. The default minimum is `1`, so a successful
but empty response triggers the fallback.

**Rationale**: The most likely failure mode for the LinkedIn source is a silent empty success
(expired session, silent rate limit, layout change). FR-014 was written to make this explicit.

**Alternatives considered**:
- Failure-only trigger: would miss the most common case.

---

## 8. Testing strategy

**Decision**:
- `src-tauri/tests/ingestion_concurrency.rs`: a fake adapter whose details resolve out of
  order proves multiple fetches are in flight and that one failure does not drop the others.
- `src-tauri/tests/ingestion_fallback.rs`: matrix of primary success / empty / failure ×
  fallback configured / not configured / primary disabled.
- `src-tauri/tests/search_progress.rs`: the sink receives `collecting` then `evaluating`, the
  counts are monotonic, and a final event is emitted.
- `src-tauri/tests/score_contract.rs` stays green as the proof that concurrency does not
  change outcomes.
- `src/viewmodels/useSearchProgress.test.ts`: the hook subscribes, updates, and unsubscribes.
- `src/views/NewJobsView.test.tsx`: the progress line renders while searching and disappears
  when the run ends.

**Rationale**: Matches the existing split between `wiremock` contract tests and Vitest
component tests, and covers the failure paths the spec requires.

**Alternatives considered**:
- Timing-based assertions on real providers: non-deterministic and requires keys.
