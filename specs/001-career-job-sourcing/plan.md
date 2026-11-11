# Implementation Plan: Career Intelligence & Job Sourcing Core

**Branch**: `001-career-job-sourcing` | **Date**: 2026-11-10 |
**Spec**: [spec.md](./spec.md)

**Input**: Feature specification from
`/specs/001-career-job-sourcing/spec.md`

## Summary

Build the backend core for the Jobs Search MVP: ingest PDF resumes into one
verified candidate profile, discover jobs through a hybrid MCP JobSpy and
Rust-native API layer, normalize and deduplicate postings with source lineage,
apply strict location/seniority/technology filters, extract requirement evidence
with an LLM, compute the deterministic 40/25/20/15 score and N/M counts, and
expose ranked results, lifecycle actions, and execution audits through typed
Tauri commands.

The React/MUI presentation layer is specified in `002` and consumes the contracts
defined here. This plan is limited to the Rust backend, SQLite persistence, the
Python MCP sidecar, and the Tauri command surface.

## Technical Context

**Language/Version**: Rust `1.90+`; Tauri `2.12.5`; Python `3.14` for the JobSpy
MCP sidecar.

**Primary Dependencies**: `rusqlite` `0.41.1` (bundled SQLite), `reqwest`
`0.13.5`, `pdf-extract`, `tokio` `1.54.1`, `serde`/`serde_json`, `chrono`,
`zeroize`, `sha2`, `thiserror`, `tracing`/`tracing-appender` (004), MUI is not
part of this backend plan.

**Storage**: SQLite under the Tauri application data directory. Raw job
descriptions are never persisted; only clean descriptions and extracted
metadata. No app-level encryption; OS disk encryption and filesystem permissions
are the baseline.

**Testing**: `cargo test`, `cargo clippy --all-targets -- -D warnings`, contract
tests for SQLite migrations, Tauri commands, MCP JSON-RPC, and LLM JSON schemas;
Python `pytest` for the MCP sidecar. Frontend tests belong to `002`.

**Target Platform**: Linux desktop.

**Project Type**: Tauri desktop backend extension with a Python MCP sidecar.

**Performance Goals**: PDF profile under 2 minutes for a 10-page resume; at most
100 evaluations per run by default; 2 concurrent LLM calls; provider fallback
Gemini → OpenRouter free models → DeepSeek; per-source timeout 15 seconds.

**Constraints**: Single operator, single active profile; strict location (remote
Brazil or on-site/hybrid in Porto Alegre/Curitiba); seniority Trainee/Junior/
Pleno (Pleno/Senior ranges accepted); Trainee/Intern accept any technology;
threshold 70; no work-authorization/visa or salary hard filter; no external
telemetry; one task per commit.

**Scale/Scope**: Local SQLite database, two ingestion source types (MCP and
Rust-native API), up to 100 evaluated jobs per run, four user stories from the
spec.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Gate | Status | How the plan satisfies it |
|------|--------|---------------------------|
| I. Source abstraction and agnosticism | PASS | MCP and API sources implement one ingestion contract; adapters contain no matching/filtering rules. |
| II. Strict compatibility evaluation | PASS | Hard filters run first; scores use 40/25/20/15, N/M counts, threshold 70, and explainable breakdowns. |
| III. Privacy, security, and identity | PASS | Data stays local; no app encryption (constitution v2.0.0); API keys in `.env`; raw descriptions are memory-only. |
| IV. Operational resilience and rate control | PASS | Per-source rate limits, bounded backoff, 15-second timeout, failure isolation, and provider fallback only on transient failures. |
| V. Idempotency and data unification | PASS | Company+title+location canonical identity with URL fallback; source lineage; repeat-safe ingestion; append-only lifecycle. |
| Strict separation of concerns | PASS | Ingestion, decision, and presentation are separate modules; Tauri commands are the presentation boundary. |
| Technology neutrality and evolution | PASS | Versions are implementation details; no constitutional dependency on a library. |
| Governance and compliance | PASS | Spec `001` and this plan follow constitution v2.0.0. |

## Project Structure

### Documentation (this feature)

```text
specs/001-career-job-sourcing/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── README.md
│   ├── tauri-commands.md
│   ├── mcp-jobspy.md
│   ├── llm-profile-extraction.schema.json
│   └── llm-evaluation.schema.json
├── spec.md
└── checklists/requirements.md
```

### Source Code (repository root)

```text
src-tauri/src/
├── commands/
│   ├── mod.rs
│   ├── profile.rs
│   ├── search.rs
│   ├── jobs.rs
│   └── telemetry.rs
├── config/
│   ├── mod.rs
│   ├── env.rs
│   └── sources.rs
├── db/
│   ├── mod.rs
│   ├── connection.rs
│   ├── migrations.rs
│   ├── profile.rs
│   ├── jobs.rs
│   ├── sources.rs
│   ├── evaluations.rs
│   ├── applications.rs
│   └── runs.rs
├── pdf/
│   ├── mod.rs
│   └── extract.rs
├── profile/
│   ├── mod.rs
│   ├── fixture.rs
│   ├── merge.rs
│   └── schema.rs
├── ingestion/
│   ├── mod.rs
│   ├── adapter.rs
│   ├── normalize.rs
│   ├── mcp.rs
│   └── api.rs
├── dedup/
│   ├── mod.rs
│   └── identity.rs
├── llm/
│   ├── mod.rs
│   ├── provider.rs
│   ├── gemini.rs
│   ├── openrouter.rs
│   ├── deepseek.rs
│   └── schemas.rs
├── match/
│   ├── mod.rs
│   ├── filters.rs
│   ├── score.rs
│   └── explain.rs
├── telemetry/          # 004, already implemented
├── error.rs
├── lib.rs
└── main.rs

src-tauri/tests/
├── profile_pipeline.rs
├── ingestion_pipeline.rs
├── dedup_identity.rs
├── match_filters.rs
├── score_contract.rs
├── llm_provider_contract.rs
└── migration_contract.rs

sidecar/jobspy_mcp/
├── jobspy_mcp_server/
├── tests/
├── requirements.txt
└── pyproject.toml
```

**Structure Decision**: Extend the existing Tauri/Rust backend with explicit
ingestion, profile, matching, persistence, and LLM modules. The Python sidecar is
isolated behind the MCP contract; the frontend remains in `002`.

## Data Model

SQLite tables:

- `resumes`: file identity, filename, content hash, local path, uploaded time.
- `candidate_profiles`: one active profile, profile JSON, manual technology tags,
  accepted cities, profile version, updated time.
- `job_sources`: source name, type (`mcp`, `api`, `web_scraper`), config, enabled,
  rate-limit policy, credential reference.
- `job_postings`: canonical identity, title, company, location, modality, URL,
  posted time, clean description, extracted metadata. No raw description column.
- `job_posting_sources`: N:N lineage with external ID, URL, ingested time.
- `job_evaluations`: evaluation status, score, N/M counts, breakdown, matched/
  missing keywords, hard-filter reasons, rules/weights/threshold, profile version,
  provider/model, evaluated time, stale flag.
- `job_applications`: lifecycle state, applied time, approval source, evaluation
  snapshot.
- `job_status_events`: append-only evaluation/lifecycle history.
- `execution_runs`, `run_metrics`, `llm_metrics`: audit and funnel data.

Dedup identity is normalized company + title + location, with URL fallback. All
distinct URLs are retained when a job passes hard filters.

## Interface Contracts

- **Tauri commands**: profile upload/read/save/delete, search trigger, paginated
  jobs, job detail, apply/ignore/manual approval, audit read. See
  [`contracts/tauri-commands.md`](./contracts/tauri-commands.md).
- **MCP JobSpy**: stdio JSON-RPC tool for board search with query, timeframe,
  seniority, stack, modality, and limit. See
  [`contracts/mcp-jobspy.md`](./contracts/mcp-jobspy.md).
- **LLM profile extraction**: strict JSON schema for competencies, domain,
  history, education, certifications, and source evidence. See
  [`contracts/llm-profile-extraction.schema.json`](./contracts/llm-profile-extraction.schema.json).
- **LLM evaluation evidence**: strict JSON schema for extracted requirements and
  per-dimension evidence; the app computes the final score. See
  [`contracts/llm-evaluation.schema.json`](./contracts/llm-evaluation.schema.json).
- **LLM providers**: Gemini → OpenRouter free models → DeepSeek; fallback only on
  429, timeout, 5xx, or malformed output; authentication/config errors stop.

## Complexity Tracking

| Deviation | Why needed | Simpler alternative rejected because |
|-----------|------------|--------------------------------------|
| Python JobSpy sidecar | Broad board coverage through a proven scraper | Reimplementing board-specific scraping in Rust increases maintenance and anti-bot risk |
| LLM evidence extraction plus app-side score | Reproducible score and N/M counts | Letting the model compute the score makes arithmetic non-deterministic |
| Hybrid dedup identity | Same job across sources has different URLs | URL-only identity fails cross-source deduplication |
| No app-level encryption | Constitution v2.0.0 accepted OS-level protection | App-level encryption adds key-management complexity the user rejected |

## Phase 0: Research

See [research.md](./research.md). All technical unknowns are resolved; no
`NEEDS CLARIFICATION` remains.

## Phase 1: Design

- [data-model.md](./data-model.md) defines entities, relationships, constraints,
  and state transitions.
- [contracts/tauri-commands.md](./contracts/tauri-commands.md) defines the command
  surface consumed by `002`.
- [contracts/mcp-jobspy.md](./contracts/mcp-jobspy.md) defines the sidecar
  JSON-RPC contract.
- [contracts/llm-profile-extraction.schema.json](./contracts/llm-profile-extraction.schema.json)
  and [contracts/llm-evaluation.schema.json](./contracts/llm-evaluation.schema.json)
  define the LLM boundaries.
- [quickstart.md](./quickstart.md) defines the end-to-end validation scenarios.

## Post-Design Constitution Check

All gates remain **PASS**. The design adds no external telemetry, no raw
description persistence, and no coupling between source adapters and matching
rules. The score is deterministic and the lifecycle is append-only.

## Gate Decision

**PASS.** Ready for `/speckit.tasks`.

## Next Steps

1. Run `/speckit.tasks` for `001`.
2. Implement the backend core in task order.
3. Plan/implement `002` (React/MUI dashboard) against the Tauri contracts.
