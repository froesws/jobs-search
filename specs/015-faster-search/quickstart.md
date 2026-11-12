# Quickstart: Faster Search

**Feature**: `015-faster-search` | **Date**: 2026-11-11

Validation guide. Shapes live in [data-model.md](./data-model.md) and
[contracts/](./contracts/); implementation detail belongs in `tasks.md`.

---

## Prerequisites

- Node + npm, Rust 1.90 toolchain.
- `.env` with at least one provider key.
- At least one resume uploaded (a search needs a resume-backed profile).
- For the fallback scenarios: the LinkedIn scraper sidecar logged in
  (`uv run linkedin-scraper-login` under `sidecar/linkedin_scraper_mcp`).

---

## Automated checks

```bash
./scripts/verify.sh
```

Focused suites while iterating:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test ingestion_fallback
cargo test --manifest-path src-tauri/Cargo.toml --test ingestion_concurrency
cargo test --manifest-path src-tauri/Cargo.toml --test search_progress
cargo test --manifest-path src-tauri/Cargo.toml --test score_contract
npm test -- src/viewmodels/useSearchProgress.test.ts src/views/NewJobsView.test.tsx
```

---

## Scenario 1 — Baseline comparison

1. Record a run with the current code: note wall-clock time from pressing **Refresh Search**
   to the summary appearing, plus `total_ingested` and the evaluated count.
2. Apply the feature and repeat with the same profile and filters.

**Expected**: at least 40% shorter for the same result set (SC-001), and identical statuses,
scores, matched/missing keywords, and rejection codes (SC-005).

---

## Scenario 2 — Parallel collection

1. Run a search against a source returning around 20 postings.
2. Watch the backend logs for overlapping detail requests, or use the progress event timing.

**Expected**: collection finishes at least 3× faster than the serialized baseline (SC-002),
with the same postings and lineage rows as before.

---

## Scenario 3 — Concurrency is configurable without restart

1. Start a run with `JOBS_SEARCH_LLM_CONCURRENCY=2` and note the evaluation duration.
2. Change the value to `4` in the provider settings or environment and run again without
   restarting.

**Expected**: the second run uses the new value (FR-004) and produces identical outcomes.

---

## Scenario 4 — Progress visibility

1. Start a run with enough postings to last more than 10 seconds.
2. Observe the interface.

**Expected**: within 5 seconds the UI shows `Collecting…` with a found count; when evaluation
starts it switches to `Evaluating… N of M`, and the count advances at least every 10 seconds
(SC-003). The line disappears when the run ends.

---

## Scenario 5 — Source time budget

1. Point a source at an endpoint that never responds (for example a `wiremock` that delays
   beyond the budget) or stop a sidecar mid-run.
2. Run a search.

**Expected**: that source is reported as `SOURCE_TIMEOUT` after its `budget_seconds`, the run
continues, and other sources' postings are kept (SC-004).

---

## Scenario 6 — Fallback: primary succeeds

1. Annotate `linkedin_scraper` with `fallback_for: linkedin` and keep the primary healthy.
2. Run a search.

**Expected**: the scraper performs zero requests; its Playwright browser is not started
(SC-007). Check the run metrics show only the primary's source breakdown.

---

## Scenario 7 — Fallback: primary fails

1. Break the primary (for example an invalid `uvx` package name).
2. Run a search.

**Expected**: the fallback runs, its postings are used, and both the primary failure and the
fallback's success are reported (FR-016, SC-008).

---

## Scenario 8 — Fallback: primary succeeds but returns nothing

1. Configure the primary with a query that matches nothing (or mock an empty payload).
2. Run a search.

**Expected**: the fallback runs because the primary returned fewer than
`fallback_min_results` (FR-014). This is the expired-session case.

---

## Scenario 9 — Fallback with a disabled primary

1. Set `enabled: false` on `linkedin` while `linkedin_scraper` still declares
   `fallback_for: linkedin`.
2. Run a search.

**Expected**: the scraper runs as a normal source and a configuration warning is logged
(FR-017).

---

## Scenario 10 — Empty result set

1. Configure filters that cannot match anything.

**Expected**: the run finishes immediately with zero results, and the progress line clears
(FR-009).

---

## Rollback

No migration is involved. Removing the new `sources.yaml` keys restores the previous parallel
behavior; the concurrency variable falls back to its default. Nothing is persisted by the
progress feature.
