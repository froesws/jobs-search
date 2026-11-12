# Implementation Plan: Faster Search

**Branch**: `main` (feature 015) | **Date**: 2026-11-12 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/015-faster-search/spec.md`

## Summary

Shorten a search run by removing work the application already did and by draining the evaluation
queue faster: skip LinkedIn detail requests for postings already stored with a description,
raise the evaluation concurrency to a persisted, operator-adjustable value, bound each source with
a time budget, stream aggregate progress to the UI, and run the secondary LinkedIn source only
when the primary fails or delivers too little. Results, scores, and filter semantics stay
identical.

## Technical Context

**Language/Version**: Rust 1.90 (edition 2024); TypeScript 7.0.2 with React 19.3.1

**Primary Dependencies**: Tauri 2.12.5 (events), `tokio` 1.54 (`timeout`), `futures` 0.3
(`join_all`), `reqwest` 0.13. **No new dependency.**

**Storage**: no schema change. The evaluation concurrency is stored in the existing
`app_settings` key/value table. The known-posting lookup reads `job_postings` and
`job_posting_sources`. `sources.yaml` gains three optional keys.

**Testing**: `cargo test` (unit + `wiremock`/stdio contract tests), `vitest` + Testing Library,
`tsc --noEmit`, `oxlint`, `prettier`, `./scripts/verify.sh`.

**Target Platform**: Linux desktop (Tauri v2, WebKitGTK).

**Project Type**: desktop application (Rust backend, React SPA frontend).

**Performance Goals**: at least 40% faster than the current baseline on the same result set
(product target); zero LinkedIn detail requests for known postings on a repeat run; progress
visible within 5 s.

**Constraints**: evaluation outcomes must be identical to today; one run at a time
(`search_lock`); no database lock held across an `.await`; provider rate limits respected by a
conservative default of four and a ceiling of eight.

**Scale/Scope**: 4 sources, up to 100 evaluated postings per run, 3 new `sources.yaml` keys, 1 new
`app_settings` key, 1 new Tauri event, 1 new frontend hook.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

`.specify/memory/constitution.md` is still the unfilled template, so the effective gates come from
`AGENTS.md` and `.opencode/rules/rules.md`.

| Gate | Status | Notes |
|------|--------|-------|
| No new dependency without concrete need | PASS | `tokio`, `futures`, and Tauri events already ship in the project |
| Privileged work in Rust | PASS | Known lookup, collection, evaluation, and progress emission stay in Rust; the UI only listens |
| Typed IPC | PASS | The event payload is typed in `src/lib/types.ts`; no `invoke()` in components |
| Results unchanged (spec FR-005) | PASS | Skipping known postings reuses the persisted description; scoring and filtering are untouched |
| Single-flight preserved | PASS | `search_lock` is untouched |
| No `Db` lock across `.await` | PASS | The known set is loaded before ingestion; progress carries plain counters |
| Tests for new behavior and failure paths | PASS | See `research.md` §9 |
| `AGENTS.md` updated when conventions change | PASS | New config keys, event, and setting documented in the same change |

**Gate result**: PASS. No violations to justify.

## Project Structure

### Documentation (this feature)

```text
specs/015-faster-search/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output (automated validation)
├── contracts/           # Phase 1 output
│   ├── known-postings.md
│   ├── progress-event.md
│   └── sources-config.md
└── tasks.md             # Phase 2 output (/speckit.tasks — not created here)
```

### Source Code (repository root)

```text
src/
├── lib/
│   ├── ipc.ts                       # modified: no new command; event name re-exported
│   └── types.ts                     # modified: SearchProgress, evaluationConcurrency
├── components/
│   └── ProviderConfigDialog.tsx     # modified: evaluation concurrency field
├── viewmodels/
│   └── useSearchProgress.ts         # new: subscribes to the progress event
└── views/
    ├── DashboardView.tsx            # modified: owns progress state
    └── NewJobsView.tsx              # modified: renders the progress line

src-tauri/src/
├── config/
│   ├── env.rs                       # modified: concurrency default 4, clamp 1..=8
│   ├── provider_settings.rs         # modified: effective evaluation concurrency
│   └── sources.rs                   # modified: fallback_for, fallback_min_results, budget_seconds
├── db/
│   ├── jobs.rs                      # modified: known postings query, touch lineage
│   └── settings.rs                  # unchanged: key/value storage
├── ingestion/
│   ├── adapter.rs                   # modified: KnownPosting, FetchOutcome, request field
│   ├── mod.rs                       # modified: per-primary fallback, budget, known outcomes
│   ├── linkedin_mcp.rs              # modified: skip known details, listed/skipped counts
│   ├── mcp.rs                       # modified: return FetchOutcome (no behavior change)
│   └── api.rs                       # modified: return FetchOutcome (no behavior change)
├── match/
│   └── mod.rs                       # modified: concurrency clamp + progress sink
└── commands/
    ├── config.rs                    # modified: provider config view/input field
    └── search.rs                    # modified: known set, fallback groups, progress emission

src-tauri/tests/
├── ingestion_skip.rs                # new: known-posting skip matrix
├── ingestion_concurrency.rs         # new: evaluation concurrency bound
├── ingestion_budget.rs              # new: source budget and timeout
├── ingestion_fallback.rs            # new: fallback trigger matrix
├── search_progress.rs               # new: progress emission sequence
└── linkedin_mcp_contract.rs         # modified: detail-call counting
```

**Structure Decision**: keep the existing Tauri layout. The known-posting lookup and lineage touch
belong in `db/jobs.rs`, the skip and fallback decisions in `ingestion/`, the concurrency clamp and
progress sink in `match/`, and the event emission in `commands/search.rs` (the only place with an
`AppHandle`).

## Phase 0: Research

See [research.md](./research.md). All `NEEDS CLARIFICATION` items are resolved.

## Phase 1: Design

See [data-model.md](./data-model.md), [contracts/](./contracts/), and
[quickstart.md](./quickstart.md).

## Verification Requirements

- **No new dependency**: `Cargo.toml` and `package.json` are unchanged.
- **Contract tests**: the LinkedIn stdio fake server counts detail calls; the fallback matrix uses
  fake adapters; the progress sink is tested with a counting closure.
- **Frontend tests**: the progress hook with a mocked event listener; `NewJobsView` rendering the
  progress line; the provider dialog saving and validating the concurrency field.
- **Full gate**: `./scripts/verify.sh`.
- **Docs**: `README.md` (`sources.yaml` keys, concurrency field, progress line) and `AGENTS.md`
  §6/§7/§8.

## Post-Design Constitution Re-check

*Re-evaluated after Phase 1 design.*

| Gate | Status | Evidence |
|------|--------|----------|
| No new dependency | PASS | `research.md` uses only `tokio`, `futures`, and Tauri events |
| Privileged work in Rust | PASS | Known lookup, collection, evaluation, budget, and emission are backend; the UI only listens |
| Typed IPC | PASS | `contracts/progress-event.md` defines `SearchProgress` in `src/lib/types.ts` |
| Results unchanged | PASS | Skipping reuses the persisted description; `score_contract` stays green |
| Single-flight preserved | PASS | `search_lock` untouched; progress belongs to the active run |
| No DB lock across `.await` | PASS | The known set is loaded before ingestion; counters are plain integers |
| Tests incl. failure paths | PASS | `research.md` §9 lists the skip, fallback, budget, progress, and concurrency tests |
| Docs updated in the same change | PASS | `README.md` and `AGENTS.md` §6/§7/§8 listed under Verification Requirements |

**Result**: PASS. No unjustified violations.

## Complexity Tracking

| Decision | Why needed | Simpler alternative rejected because |
|----------|------------|--------------------------------------|
| Skip known details instead of parallel collection | The LinkedIn MCP server serializes tool calls around one browser profile; parallel detail calls cannot help | A multiplexed client would still queue in `SequentialToolExecutionMiddleware`; multiple browser profiles corrupt the shared session |
| Global known set by `linkedin-` prefix | Catches postings first stored by JobSpy, which also scans LinkedIn | A per-source set would re-download every LinkedIn posting that JobSpy already stored |
| `FetchOutcome` on the adapter trait | The fallback decision needs delivered counts, not just new jobs | Returning only `Vec<RawJob>` cannot distinguish "listed 10 known" from "listed nothing" |
| Per-primary fallback orchestration | A fast LinkedIn failure must not wait for a slow unrelated source | A global two-phase barrier delays the fallback by the slowest primary |
| Progress sink passed into `evaluate_run` | Progress must not block or hold the DB lock | Polling a command adds a command, a timer, and stale reads |
| Persisted evaluation concurrency | The operator must adjust it without restarting | `.env` alone is only read at startup; `replace_config` currently has no callers |
