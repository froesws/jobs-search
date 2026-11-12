# Implementation Plan: Faster Search

**Branch**: `015-faster-search` | **Date**: 2026-11-11 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/015-faster-search/spec.md`

## Summary

Shorten a search run by attacking its two blocking stages and its feedback gap: parallelize
per-posting detail collection inside each source, raise the evaluation concurrency to a
configurable value, bound each source with a time budget, stream progress to the UI, and let
the secondary LinkedIn source run only when the primary fails or comes back empty. Results,
scores, and filter semantics stay byte-for-byte identical.

## Technical Context

**Language/Version**: Rust 1.90 (edition 2024); TypeScript 7.0.2 with React 19.3.1

**Primary Dependencies**: Tauri 2.12.5 (events), `tokio` 1.54 (`Semaphore`, `timeout`),
`futures` 0.3 (`buffer_unordered`), `reqwest` 0.13. **No new dependency.**

**Storage**: no schema change. Progress is in-memory and emitted as a Tauri event; nothing is
persisted. `sources.yaml` gains two optional keys.

**Testing**: `cargo test` (unit + `wiremock` contract tests), `vitest` + Testing Library,
`tsc --noEmit`, `oxlint`, `prettier`, `./scripts/verify.sh`.

**Target Platform**: Linux desktop (Tauri v2, WebKitGTK).

**Project Type**: desktop application (Rust backend, React SPA frontend).

**Performance Goals**: at least 40% faster than the current baseline on the same result set;
collection at least 3× faster for a 20-posting source; progress visible within 5 s.

**Constraints**: evaluation outcomes must be identical to today; one run at a time
(`search_lock`); no database lock held across an `.await`; provider rate limits respected by
conservative defaults.

**Scale/Scope**: 4 sources, up to 100 evaluated postings per run, 5 new configuration keys,
1 new Tauri event, 1 new frontend hook.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

`.specify/memory/constitution.md` is still the unfilled template, so the effective gates come
from `AGENTS.md` and `.opencode/rules/rules.md`.

| Gate | Status | Notes |
|------|--------|-------|
| No new dependency without concrete need | PASS | `tokio`, `futures`, and Tauri events already ship in the project |
| Privileged work in Rust | PASS | Collection, evaluation, and progress emission stay in Rust; the UI only listens |
| Typed IPC | PASS | The event payload is typed in `src/lib/types.ts`; no `invoke()` in components |
| Results unchanged (spec FR-002/FR-005) | PASS | Concurrency changes ordering of work, not of scoring or filtering |
| Single-flight preserved | PASS | `search_lock` is untouched |
| No `Db` lock across `.await` | PASS | Progress carries plain counters; no DB read inside the stream |
| Tests for new behavior and failure paths | PASS | See `research.md` §8 |
| `AGENTS.md` updated when conventions change | PASS | New config keys, event, and commands documented in the same change |

**Gate result**: PASS. No violations to justify.

## Project Structure

### Documentation (this feature)

```text
specs/015-faster-search/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (event payload + sources.yaml schema)
└── tasks.md             # Phase 2 output (/speckit.tasks — not created here)
```

### Source Code (repository root)

```text
src/
├── lib/
│   ├── ipc.ts                       # modified: no new command, re-exports the event name
│   └── types.ts                     # modified: SearchProgress payload
├── viewmodels/
│   └── useSearchProgress.ts         # new: subscribes to the progress event
└── views/
    ├── DashboardView.tsx            # modified: owns progress state, passes it to NewJobsView
    └── NewJobsView.tsx              # modified: renders the progress line while searching

src-tauri/src/
├── config/
│   ├── env.rs                       # modified: concurrency default 4, clamp 1..=8
│   └── sources.rs                   # modified: fallback_for, fallback_min_results, budget_seconds
├── ingestion/
│   ├── mod.rs                       # modified: budget timeout + fallback orchestration
│   ├── linkedin_mcp.rs              # modified: parallel detail fetches
│   └── mcp.rs                       # modified: parallel detail fetches where applicable
├── match/
│   └── mod.rs                       # modified: concurrency clamp + progress sink
├── commands/
│   └── search.rs                    # modified: emit progress, resolve fallback groups
└── lib.rs                           # modified: nothing beyond existing command registration

src-tauri/tests/
├── ingestion_fallback.rs            # new: fallback trigger matrix
├── ingestion_concurrency.rs         # new: parallel detail behavior and isolation
└── search_progress.rs               # new: progress emission sequence
```

**Structure Decision**: keep the existing Tauri layout. The fallback and budget logic belongs
in `ingestion/` (it is an ingestion concern), the concurrency and progress sink in `match/`,
and the event emission in `commands/search.rs` (the only place with an `AppHandle`).

## Verification Requirements

- **No new dependency**: `Cargo.toml` and `package.json` are unchanged.
- **Contract tests**: `wiremock` for the parallel detail fetches; a fallback matrix test.
- **Frontend tests**: the progress hook with a mocked event listener; `NewJobsView` rendering
  the progress line.
- **Full gate**: `./scripts/verify.sh`.
- **Docs**: `README.md` (`sources.yaml` keys, concurrency variable) and `AGENTS.md` §6/§8.

## Post-Design Constitution Re-check

*Re-evaluated after Phase 1 design.*

| Gate | Status | Evidence |
|------|--------|----------|
| No new dependency | PASS | `research.md` §1–§5 use only `tokio`, `futures`, and Tauri events |
| Privileged work in Rust | PASS | Collection, evaluation, budget, and emission are backend; the UI only listens |
| Typed IPC | PASS | `contracts/progress-event.md` defines `SearchProgress` in `src/lib/types.ts` |
| Results unchanged | PASS | `score_contract` stays green; concurrency only reorders work |
| Single-flight preserved | PASS | `search_lock` untouched; progress belongs to the active run |
| No DB lock across `.await` | PASS | Counters are plain integers; no DB read in the stream |
| Tests incl. failure paths | PASS | `research.md` §8 lists the concurrency, fallback matrix, and progress tests |
| Docs updated in the same change | PASS | `README.md` and `AGENTS.md` §6/§8 listed under Verification Requirements |

**Result**: PASS. No unjustified violations.

## Complexity Tracking

| Decision | Why needed | Simpler alternative rejected because |
|----------|------------|--------------------------------------|
| Fallback orchestration in `ingest` | FR-013–FR-017 require a primary/fallback relationship | Running every source in parallel wastes detail fetches on duplicates and cannot express "only if empty" |
| Progress sink passed into `evaluate_run` | FR-006/FR-007 require counts without blocking or holding the DB lock | Polling a command from the UI adds a command, a timer, and stale reads |
| Two new `sources.yaml` keys plus a budget | FR-008 and FR-014 need explicit, testable configuration | Hard-coded behavior would not be tunable per source and could not be tested as configuration |
