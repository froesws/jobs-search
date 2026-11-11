# Research: Skill Gap Analytics & Funnel

**Feature**: `003-skill-gap-analytics`  
**Date**: 2026-11-10

## Decision: Skill Gap scope

**Decision**: Aggregate missing skills over locally stored evaluated jobs that
completed evaluation and passed hard filters (`APPROVED` or `PENDING`), regardless
of lifecycle state (`DISCOVERED`, `APPLIED`, `IGNORED`). Exclude `REJECTED`,
`ERROR`, incomplete, and deleted evaluations. The denominator is all eligible jobs,
including jobs with no missing skills.

**Rationale**: The metric answers “what does the target market ask for?” Applied
and ignored jobs are still market data. Rejected jobs (e.g., VP Sales) would skew
demand and are excluded.

## Decision: Counting and percentages

**Decision**: Count each normalized skill at most once per job. Exclude skills
present in the active profile or listed as matched for that job. Percentage =
job count / eligible job count * 100, rounded to one decimal. Rank by count
descending, then alphabetically.

**Rationale**: Prevents double-counting and makes the 50/100 → 50.0% example
testable. The denominator stays stable even for jobs with no missing skills.

## Decision: Funnel scope and metrics

**Decision**: Use one row per search run. Show ingested, duplicates merged,
rejections by reason, pending, approved, applied, ignored, approval rate,
application rate, source/provider breakdown, score distribution, and LLM
latency/fallback. Time range: Current Run or Last 7 Days.

**Rationale**: The funnel answers “where do jobs die?” and “which source/provider
performs?” without log parsing. Metrics come from structured tables.

## Decision: Structured metrics source

**Decision**: Read `execution_runs`, `run_metrics`, and `llm_metrics`; never parse
local log files. `004` logs remain technical diagnostics only.

**Rationale**: Logs are not a stable analytics source and may be rotated. The
structured tables are queryable and reconcile with the audit.

## Decision: CSV export

**Decision**: Export the current Analytics view as a local CSV: aggregate
metrics plus one row per job with title, company, location, modality, URL,
source, evaluation/lifecycle status, score, N/M, matched/missing keywords, and
`description_clean`. Exclude `description_raw` and credentials. Require an
explicit export action and show a sensitive-data warning.

**Rationale**: The operator requested full current-view export. Clean text and
URL are job-facing data; raw text and secrets remain excluded. The file is local
and never uploaded.

## Decision: Frontend MVVM

**Decision**: Replace the `002` Analytics shell with
`useAnalyticsViewModel` and MUI components. The ViewModel owns range selection,
loading/error/empty/stale state, and export; the View renders skill bars/frequency
cards and funnel tables/cards.

**Rationale**: Consistent with the chosen hook-based MVVM and no state library.

## Decision: Performance and privacy

**Decision**: Aggregate in SQL, keep queries non-blocking, and render the summary
under 100 ms for 1,000 jobs. All data stays local; no provider calls are made for
analytics.

**Rationale**: Meets the spec's performance target and avoids sending evaluation
data to an external service.
