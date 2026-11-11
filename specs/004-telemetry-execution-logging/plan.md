# Implementation Plan: Local Telemetry & Execution Logging

**Branch**: `004-telemetry-execution-logging` | **Date**: 2026-11-10 |
**Spec**: [spec.md](./spec.md)

**Input**: Feature specification from
`/specs/004-telemetry-execution-logging/spec.md`

## Summary

Add local, non-blocking execution logging to the Jobs Search backend. The feature
captures structured lifecycle and error events, writes JSON Lines to a rotating
file under the application data directory, enforces INFO/WARN/ERROR by default
with DEBUG opt-in via `RUST_LOG`, and keeps logs free of credentials, payloads,
prompts, URLs, paths, and candidate data. It replaces the rejected external
MongoDB/raw-payload design.

## Technical Context

**Language/Version**: Rust `1.90+` (Tauri `2.12.5` backend).

**Primary Dependencies**: `tracing` `0.1`, `tracing-subscriber` `0.3` with JSON
support, `tracing-appender` `0.2`, `serde`/`serde_json`, `chrono`, and a bounded
in-process queue built on `std` synchronization primitives.

**Storage**: Local rotating JSON Lines file under the Tauri application data
directory. No SQLite writes, no MongoDB, and no external log repository.

**Testing**: `cargo test` and
`cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
Unit tests cover level filtering, sanitization, queue overflow, rotation,
retention, health, and flush behavior. An integration test writes and reads a
real log file. No frontend or Python tests are needed for this feature.

**Target Platform**: Linux desktop.

**Project Type**: Tauri desktop backend feature.

**Performance Goals**: 1,000 events dispatched without blocking the UI or job
pipeline; 5-second maximum shutdown flush; 7 days or 50 MB total retention.

**Constraints**: Local-only; no app-level encryption; no raw payloads, model
prompts/responses, URLs, paths, job descriptions, candidate data, or credentials
in logs; no external telemetry; one commit per completed task.

**Scale/Scope**: Single operator, single application instance, default queue
capacity of 10,000 events, oldest-first dropping.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Gate | Status | How the plan satisfies it |
|------|--------|---------------------------|
| I. Source abstraction and agnosticism | PASS | Logging instruments the ingestion contract without containing matching or filtering rules. |
| II. Strict compatibility evaluation | PASS | Events record evaluation outcomes, scores, and run identifiers without changing the match engine or its threshold. |
| III. Privacy, security, and identity | PASS | Logs stay local and exclude credentials, payloads, prompts, URLs, paths, job content, and candidate data. Constitution v2.0.0 permits OS-level protection and `.env` credentials. |
| IV. Operational resilience and rate control | PASS | Logging is non-blocking, has a bounded queue, isolates file failures, and never halts ingestion or evaluation. |
| V. Idempotency and data unification | PASS | Log events carry run identifiers and do not mutate opportunity, profile, application, or lifecycle records. |
| Strict separation of concerns | PASS | Telemetry lives in its own backend module and observes the other layers through typed events. |
| Technology neutrality and evolution | PASS | `tracing` and the file format are implementation details; the spec requires behavior, not a vendor. |
| Governance and compliance | PASS | The spec is aligned with constitution v2.0.0 and has no unresolved clarification. |

## Project Structure

### Documentation (this feature)

```text
specs/004-telemetry-execution-logging/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   ├── log-event.schema.json
│   ├── log-health.schema.json
│   └── README.md
├── spec.md
└── checklists/requirements.md
```

### Source Code (repository root)

```text
src-tauri/src/
├── telemetry/
│   ├── mod.rs           # setup, initialization, shutdown guard
│   ├── event.rs         # safe event types and field constructors
│   ├── config.rs        # RUST_LOG/level, paths, retention, queue defaults
│   ├── queue.rs         # bounded queue, oldest-first drop, counters
│   ├── writer.rs        # JSONL writer, rotation, retention cleanup
│   ├── health.rs        # availability, depth, dropped count, last error
│   └── sanitize.rs      # reject/strip URLs, paths, secrets, PII
├── commands/
│   └── telemetry.rs     # cmd_get_log_health
├── error.rs             # project error type (unchanged)
├── lib.rs               # initialize telemetry and register command
└── main.rs

src-tauri/tests/
└── telemetry.rs         # file write/read, rotation, flush, outage behavior
```

**Structure Decision**: Backend-only feature in the existing Tauri/Rust
structure. No frontend viewer, no Python changes, and no new application
directory beyond `src-tauri/src/telemetry/`.

## Complexity Tracking

| Deviation | Why needed | Simpler alternative rejected because |
|-----------|------------|--------------------------------------|
| Custom bounded queue instead of `tracing-appender` non-blocking writer | The spec requires an explicit dropped-event count and oldest-first policy | The stock appender does not expose a reliable drop count or oldest-first semantics |
| Tauri health command instead of an in-app log viewer | The spec requires health visibility without a viewer/export UI | A viewer adds UI scope and exposes log content beyond the MVP |
| No application-level encryption | Constitution v2.0.0 accepted OS-level protection | App-level encryption was explicitly removed by amendment |

## Phase 0: Research

See [research.md](./research.md). All technical unknowns are resolved; no
`NEEDS CLARIFICATION` remains.

## Phase 1: Design

- [data-model.md](./data-model.md) defines Log Event, Log Configuration, Log
  Store Health, and Execution Run.
- [contracts/log-event.schema.json](./contracts/log-event.schema.json) defines the
  JSONL event contract.
- [contracts/log-health.schema.json](./contracts/log-health.schema.json) defines
  the health payload returned by the Tauri command.
- [contracts/README.md](./contracts/README.md) documents the Tauri command
  surface and file naming.
- [quickstart.md](./quickstart.md) contains the end-to-end validation scenarios.

## Post-Design Constitution Check

All gates remain **PASS**. The design adds no external service, no raw content,
and no transactional side effects. Logging is isolated in the telemetry module
and can fail without halting ingestion, matching, or application actions.

## Gate Decision

**PASS.** Ready for `/speckit.tasks`.

## Next Steps

1. Run `/speckit.tasks` for `004`.
2. Implement the telemetry module and tests.
3. Run Rust checks and commit the completed task.
