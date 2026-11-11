# Quickstart: Local Telemetry & Execution Logging

**Feature**: `004-telemetry-execution-logging`
**Date**: 2026-11-10

This guide validates the feature after the implementation tasks are complete. It
assumes the Tauri app shell and Rust backend exist.

**Implementation status**: the telemetry backend, contract tests, rotation,
queue, health, and shutdown behavior are implemented in `src-tauri/src/telemetry/`
and exercised by `cargo test`. Scenarios that require the search, LLM, and
application modules are validated through the telemetry facade tests until those
modules land in their own feature work.

## Prerequisites

- Linux with the Tauri v2 build dependencies.
- Rust `1.90+`.
- Node.js `24+` and the frontend dependencies installed.
- `jq` for inspecting JSON Lines (optional).
- A writable application data directory. On Linux this is typically
  `~/.local/share/com.willf.jobs-search/logs/`.

## Setup

```bash
npm install
cargo build --manifest-path src-tauri/Cargo.toml
```

For development, set the level in the environment:

```bash
RUST_LOG=info npm run tauri dev
```

For a DEBUG investigation:

```bash
RUST_LOG=debug npm run tauri dev
```

The log directory and file MUST appear under the application data directory as
`jobs-search.YYYY-MM-DD.log`.

## Validation scenarios

### 1. Default INFO filtering

1. Start the app with `RUST_LOG=info`.
2. Trigger a normal search and a successful evaluation.
3. Inspect the newest log file:

```bash
LOG_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/com.willf.jobs-search/logs"
tail -n 20 "$LOG_DIR"/jobs-search.*.log | jq -c .
```

Expected: INFO, WARN, and ERROR events are present; DEBUG events are absent.

### 2. DEBUG opt-in

1. Start the app with `RUST_LOG=debug`.
2. Repeat the search/evaluation.
3. Inspect the log file.

Expected: DEBUG events are present, and the same fields are still safe.

### 3. Event coverage and correlation

1. Run a search with one failing source and one successful source.
2. Trigger an LLM call and a forced provider fallback.
3. Merge at least one duplicate posting.
4. Apply one job and ignore another.

Expected event types include `APP_START`, `SOURCE_FAILURE`, `DEDUP_COMPLETE`,
`LLM_CALL`, `LLM_FALLBACK`, `EVALUATION_COMPLETE`, and `APPLICATION_ACTION`.
Every event has a timestamp, level, event type, component, and outcome; related
events share a run identifier.

### 4. Safe fields

Inspect the log for forbidden content:

```bash
if grep -Eqi \
  'https?://|/home/|prompt|response|description|candidate|api[_-]?key|secret' \
  "$LOG_DIR"/jobs-search.*.log; then
  echo "FAIL: forbidden content found"
else
  echo "PASS: no forbidden content"
fi
```

Expected: PASS. No URLs, paths, prompts, responses, descriptions, candidate data,
or credentials appear.

### 5. Rotation and retention

Use the telemetry test helper or generate enough events to cross a day boundary
in a test fixture. Then inspect the directory:

```bash
ls -1 "$LOG_DIR"
du -sh "$LOG_DIR"
```

Expected: at most 7 daily files and at most 50 MB total. Older files are removed
automatically.

### 6. Non-blocking burst

Run the burst test:

```bash
cargo test --manifest-path src-tauri/Cargo.toml telemetry_burst_is_non_blocking -- --nocapture
```

Expected: the test emits 1,000 events, the pipeline/UI remains responsive, and
the queue stays within its configured capacity.

### 7. Log store outage

1. Make the log directory read-only or rename it.
2. Trigger a search and an evaluation.

Expected: the pipeline completes, the app does not hang, and the health command
reports `available: false` or a degraded state with a sanitized error.

Restore the directory and verify health returns to available.

### 8. Shutdown flush

1. Generate several events.
2. Close the application.

Expected: the process exits within 5 seconds. Recent events are flushed or
counted as dropped; shutdown does not hang.

### 9. Health command

Call `cmd_get_log_health` from the frontend/devtools or the Rust test harness.
Validate the payload against
[`contracts/log-health.schema.json`](./contracts/log-health.schema.json).

Expected:

```json
{
  "available": true,
  "queue_depth": 0,
  "queue_capacity": 10000,
  "dropped_count": 0,
  "last_error": null,
  "last_success_at": "2026-11-10T20:00:00Z",
  "current_file": "jobs-search.2026-11-10.log"
}
```

## Validation evidence

On 2026-11-10, `cargo test --manifest-path src-tauri/Cargo.toml` passed with the
unit and integration suites, including contract, queue, retention, safety,
outage, burst, and shutdown tests. Scenarios that require the full search, LLM,
and application modules are exercised through the telemetry facade tests until
those features land in their own feature work.

## Checks

Run before committing:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run typecheck
npm test
```

The telemetry feature is backend-only; frontend tests are still required for the
app shell, but they do not test log internals.
