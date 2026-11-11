# Implementation Plan: Skill Gap Analytics & Funnel

**Branch**: `003-skill-gap-analytics` | **Date**: 2026-11-10 |
**Spec**: [spec.md](./spec.md)

**Input**: Feature specification from
`/specs/003-skill-gap-analytics/spec.md`

## Summary

Add the Analytics tab content to the `002` dashboard: a Skill Gap section and an
operational Funnel section over the `001` evaluation/run data. The backend exposes
typed analytics/CSV commands; the frontend uses the MVVM hook pattern with MUI
progress bars/cards/tables. The Analytics tab has Current Run/Last 7 Days, and CSV
export contains the current view with clean descriptions and URLs but no raw text
or credentials.

## Technical Context

**Language/Version**: Rust `1.90+`; React `19.3.1`; TypeScript `7.0.2`; Vite
`8.3.6`; MUI `9.5` + Emotion.

**Primary Dependencies**: `rusqlite` for aggregation, existing `001` SQLite
tables, Tauri commands, React/MUI, MVVM hooks. No new state library and no
external analytics service.

**Storage**: Read-only queries over the local `001` SQLite database; no new
external store. CSV is a local explicit export.

**Testing**: Rust aggregation/contract tests for skill gap, funnel, score
distribution, LLM metrics, and CSV; Vitest + Testing Library for the Analytics
ViewModel and View; `./scripts/verify.sh`.

**Target Platform**: Linux desktop.

**Project Type**: Tauri backend analytics + React/MUI frontend.

**Performance Goals**: Analytics summary under 100 ms for 1,000 eligible jobs;
non-blocking queries with loading state.

**Constraints**: Skill Gap scope is eligible evaluated jobs that passed hard
filters, lifecycle-neutral; denominator includes all eligible jobs; one decimal;
no raw descriptions/credentials; CSV local and explicit; English labels.

**Scale/Scope**: One operator, current run + last 7 days, skill gap + funnel,
CSV current view.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Gate | Status | How the plan satisfies it |
|------|--------|---------------------------|
| I. Source abstraction and agnosticism | PASS | Analytics consumes normalized/evaluated records, not sources or adapters. |
| II. Strict compatibility evaluation | PASS | Skill gap/funnel are derived from stored evaluations; they do not recalculate scores. |
| III. Privacy, security, and identity | PASS | Local-only queries; CSV excludes raw descriptions and credentials; no external analytics. |
| IV. Operational resilience and rate control | PASS | Queries are local and non-blocking; partial source failure does not invalidate stored analytics. |
| V. Idempotency and data unification | PASS | Aggregations use evaluated jobs/lineage records; no duplicate counting per job. |
| Strict separation of concerns | PASS | Analytics is a read-only reporting layer between `001` data and `002` views. |
| Technology neutrality and evolution | PASS | SQL/MUI are implementation details; no constitutional coupling. |
| Governance and compliance | PASS | Spec `003` and this plan follow constitution v2.0.0. |

## Project Structure

### Documentation

```text
specs/003-skill-gap-analytics/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── README.md
│   ├── analytics-commands.md
│   └── csv-export.md
├── spec.md
└── checklists/requirements.md
```

### Source Code

```text
src-tauri/src/
├── analytics/
│   ├── mod.rs
│   ├── skill_gap.rs
│   ├── funnel.rs
│   └── csv.rs
├── commands/
│   └── analytics.rs
└── db/
    └── runs.rs

src/
├── viewmodels/
│   └── useAnalyticsViewModel.ts
├── views/
│   └── AnalyticsView.tsx
└── components/
    ├── SkillGapBar.tsx
    ├── SkillGapList.tsx
    ├── FunnelCards.tsx
    ├── ScoreDistribution.tsx
    └── LlmMetricsTable.tsx
```

**Structure Decision**: Backend analytics queries live in `src-tauri/src/analytics/`
and read `001` tables; the frontend replaces the `002` Analytics shell with the
MVVM `useAnalyticsViewModel` and MUI components.

## Data Model

- **Skill Gap Metric**: normalized skill, display name, demand count, percentage,
  rank, last computed.
- **Analytics Dataset**: scope (`CURRENT_RUN` or `LAST_7_DAYS`), denominator,
  last updated, stale state.
- **Search Run**: run ID, start/end, filters, outcome, links to metrics.
- **Run Metrics**: ingested, duplicates, rejected by reason, pending, approved,
  applied, ignored, approval/application rates, score distribution, source/
  provider breakdowns.
- **LLM Metric**: run ID, provider, model, operation, latency, outcome, error
  category, fallback origin.

The Analytics tab is read-only and never mutates profile, jobs, evaluations, or
applications.

## Interface Contracts

- **Analytics commands**: `cmd_get_analytics` and
  `cmd_export_analytics_csv`. See
  [`contracts/analytics-commands.md`](./contracts/analytics-commands.md).
- **CSV**: current view, aggregate metrics plus job rows with title, company,
  location, modality, URL, source, statuses, score, N/M, keywords, and clean
  description. See [`contracts/csv-export.md`](./contracts/csv-export.md).
- **ViewModel**: `useAnalyticsViewModel` state/actions; refresh, range selector,
  and export.
- **Queries**: structured `execution_runs`, `run_metrics`, and `llm_metrics`;
  never parse log files.

## Complexity Tracking

| Deviation | Why needed | Simpler alternative rejected because |
|-----------|------------|--------------------------------------|
| Backend aggregation + frontend MVVM | Keeps SQL/heavy work out of React | Client-side aggregation over raw rows would load too much and duplicate rules |
| CSV includes clean description/URL | Operator requested full current-view export | Aggregate-only CSV loses job-level analysis |
| Lifecycle-neutral skill gap | APPLIED/IGNORED jobs remain market data | Lifecycle filtering would bias demand |
| Analytics tab shell in 002 | Four-tab layout is fixed | Omitting the tab would break navigation |

## Phase 0: Research

See [research.md](./research.md). All technical unknowns are resolved; no
`NEEDS CLARIFICATION` remains.

## Phase 1: Design

- [data-model.md](./data-model.md) defines analytics metrics and queries.
- [contracts/analytics-commands.md](./contracts/analytics-commands.md) defines
  the backend command surface.
- [contracts/csv-export.md](./contracts/csv-export.md) defines the CSV columns
  and privacy rules.
- [quickstart.md](./quickstart.md) defines the 50/100 Kubernetes validation and
  funnel/CSV scenarios.

## Post-Design Constitution Check

All gates remain **PASS**. Analytics is read-only, local, lifecycle-neutral, and
excludes raw descriptions/credentials. CSV is explicit and local.

## Gate Decision

**PASS.** Ready for `/speckit.tasks`.

## Next Steps

1. Run `/speckit.tasks` for `003`.
2. Implement backend analytics/CSV, then the Analytics ViewModel/View.
3. Validate the 50/100 Kubernetes example and the funnel/CSV scenarios.
