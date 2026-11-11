# Quickstart: Skill Gap Analytics & Funnel

**Feature**: `003-skill-gap-analytics`  
**Date**: 2026-11-10

This guide validates the Analytics tab after the `001` data and `002` shell are
implemented.

## Prerequisites

- `001` backend with at least 100 eligible evaluated jobs.
- `002` dashboard with the Analytics tab shell.
- Local SQLite data only; no external analytics service.

## Validation scenarios

### 1. Kubernetes 50/100 example

Seed 100 eligible evaluated jobs, 50 with “Kubernetes” in missing keywords. Open
Analytics. Expected: Kubernetes shows demand count 50 and demand percentage
50.0%, ranked above lower-demand skills.

### 2. Exclusions and counting

Add Kubernetes to the active profile. Expected: it disappears from the gap. Add
duplicate “Kubernetes” entries to one job. Expected: that job still counts once.
A skill in matched keywords is not counted as missing.

### 3. Funnel

Expected: ingested, duplicates, rejected-by-reason, pending, approved, applied,
ignored, approval rate, application rate, source/provider breakdown, score
distribution, and LLM latency/fallback are visible and reconcile with the
underlying run records.

### 4. Time range

Switch Current Run / Last 7 Days. Expected: both sections update consistently and
the denominator/scope label is visible.

### 5. Empty and zero states

Expected: no eligible jobs → no-data state, no division by zero. Jobs with no
missing skills → zero-gap state with denominator. Error/unavailable data →
error state, not stale data presented as current.

### 6. CSV export

Export the current view. Expected: local CSV with aggregate metrics and job rows
including `description_clean` and URL; no `description_raw` or credentials; the
file is never uploaded. A sensitive-data warning is shown before export.

### 7. Performance

Seed 1,000 jobs. Expected: the Analytics summary renders in under 100 ms on the
reference environment; the tab does not block the rest of the UI.

### 8. Tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test analytics_contract
npm run typecheck
npm test
./scripts/verify.sh
```

Expected: aggregation, percentages, ranking, exclusions, funnel, CSV, empty
states, and performance tests pass.

## Implemented evidence (2026-11-10)

- `cargo test --test analytics_contract`: 7 tests covering 50/100 Kubernetes,
  variant normalization, count-once, rejected/error exclusion, lifecycle
  neutrality, stale disclosure, scope consistency, funnel reconciliation, and
  CSV privacy.
- Frontend: `useAnalyticsViewModel`, `SkillGapList`, and `AnalyticsView` tests
  cover scope changes, empty vs zero-gap, stale/denominator visibility, CSV
  warning, error state, and a render budget.
- `./scripts/verify.sh`: PASSED.
- `npm run tauri build -- --debug --no-bundle`: PASSED.
- `node`/Rust implementation notes: current run = latest SUCCESS/PARTIAL;
  Last 7 Days filters `evaluated_at`; stale evaluations count toward demand but
  are disclosed; CSV rows contain only APPROVED/PENDING jobs.
- Unknown/unavailable storage renders an error state, never stale data as
  current. Analytics makes no provider call and never mutates records.
