# Research: Local Telemetry & Execution Logging

**Feature**: `004-telemetry-execution-logging`
**Date**: 2026-11-10

All items below are resolved. No `NEEDS CLARIFICATION` remains.

## Decision: Logging framework

**Decision**: Use `tracing` for instrumentation, `tracing-subscriber` with a
custom JSONL layer, and `tracing-appender`'s daily rolling file appender as the
drain target.

**Rationale**: `tracing` provides levels, structured fields, and span context for
search/evaluation runs. `tracing-subscriber` integrates `RUST_LOG` through
`EnvFilter`. `tracing-appender` provides daily file rotation without a custom
rotation implementation.

**Alternatives considered**:
- `log` + `tauri-plugin-log`: simpler Tauri integration, but weaker structured
  fields and no native span context for LLM latency/fallback correlation.
- Plain `log` + custom writer: fewer dependencies, but more code and no
  structured event model.

## Decision: Log file format

**Decision**: JSON Lines (JSONL), one event per line, UTF-8.

**Rationale**: The event contract is already structured. JSONL is machine-
parseable with `jq`, supports schema validation, and remains readable line by
line during debugging. It avoids ambiguous text parsing.

**Alternatives considered**:
- Human-readable text: easier to scan, but fragile for field extraction and
  tests.
- CSV: tabular but poor for optional fields and nested context.

## Decision: Non-blocking dispatch and queue policy

**Decision**: A bounded in-process queue with a dedicated writer thread. The
producer serializes each event and pushes it with `try_push`. When the queue is
full, remove the oldest event and increment an atomic dropped counter. The writer
thread drains into the daily rolling appender.

**Rationale**: `tracing-appender`'s stock non-blocking writer does not expose a
reliable dropped-event count or guarantee oldest-first behavior. A small custom
queue satisfies FR-LOG-008 and the spec's 10,000-event default while keeping the
UI and pipeline non-blocking.

**Alternatives considered**:
- Unbounded channel: simpler, but memory grows under DEBUG volume.
- Drop-newest bounded channel: simpler, but loses the most recent context, which
  is usually the most useful for debugging.
- `tokio::sync::mpsc`: works, but couples the writer to a runtime and does not
  provide oldest-first semantics by itself.

## Decision: Rotation and retention

**Decision**: Rotate daily. On startup and every 24 hours, delete files older
than 7 days and then delete oldest files until the directory total is at most
50 MB.

**Rationale**: Matches FR-LOG-006 and SC-005. Daily files are easy to identify;
the size cap prevents a DEBUG session from filling the disk.

**Alternatives considered**:
- Size-only rotation: can produce many files and complicates the 7-day rule.
- No automatic deletion: violates the bounded local retention requirement.

## Decision: Health visibility

**Decision**: Expose a Tauri command `cmd_get_log_health` returning a
`LogHealth` JSON object. Do not add an in-app log viewer or export.

**Rationale**: FR-LOG-015 requires health visibility without exposing log
content; FR-LOG-016 forbids a viewer/export in the MVP. A command lets the
frontend or a maintainer query availability, queue depth, dropped count, and the
last error without reading log content.

**Alternatives considered**:
- Status bar indicator: useful UX, but adds frontend scope and is not required
  for the MVP.
- Health file only: workable, but a typed command is easier to test and consume.

## Decision: Sanitization boundary

**Decision**: Log through typed event constructors that accept only safe fields:
level, event type, component, outcome, run ID, elapsed time, error category,
provider/model context, job reference, lifecycle transition, and a short
sanitized message. A `sanitize` function strips URLs, filesystem paths, and
known secret patterns and caps message length. Raw payloads, prompts, responses,
and descriptions are never passed to the logger.

**Rationale**: Prevention at the API boundary is safer than post-hoc regex
cleanup of arbitrary strings. Tests can assert that forbidden inputs are rejected
or redacted before serialization.

**Alternatives considered**:
- Log arbitrary `Debug` strings and sanitize later: fragile and easy to regress.
- Forbid all free-form messages: too restrictive for error categories that need
  a short explanation.

## Decision: Shutdown and failure behavior

**Decision**: Store a telemetry guard in Tauri state. On shutdown, stop accepting
new events, drain the queue for at most 5 seconds, flush the appender, and count
any remaining events as dropped. If the log directory is unavailable, mark health
as degraded/unavailable and continue the application.

**Rationale**: Satisfies FR-LOG-009/010 and SC-004/008. Logging MUST never block
or crash the job pipeline.

**Alternatives considered**:
- Flush indefinitely: can hang shutdown.
- No flush: loses the most recent events, which are usually the most valuable.

## Decision: Verification strategy

**Decision**: Rust unit tests plus one file integration test. Cover level
filtering, sanitization, queue overflow/drop count, retention cleanup, health
state, and shutdown flush. Use `cargo test` and `cargo clippy -D warnings` before
committing. No frontend or Python tests are needed for this backend-only feature.

**Rationale**: The feature has no UI and no Python changes. The highest-risk
behaviors are queue semantics, sanitization, and file lifecycle, all testable in
Rust.

**Alternatives considered**:
- Frontend tests: no frontend surface is added.
- End-to-end tests through the UI: valuable later, but not required for this
  internal backend feature.
