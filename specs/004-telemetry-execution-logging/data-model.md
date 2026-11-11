# Data Model: Local Telemetry & Execution Logging

**Feature**: `004-telemetry-execution-logging`
**Date**: 2026-11-10

The model is intentionally small. Logs are append-only JSONL records; there is no
relational table and no external log store.

## Log Event

A structured technical record emitted by the backend.

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `timestamp` | string (RFC 3339 UTC) | yes | Must parse as UTC; milliseconds allowed |
| `level` | enum `INFO`, `WARN`, `ERROR`, `DEBUG` | yes | Must match the active filter before writing |
| `event_type` | enum | yes | One of the supported event types below |
| `run_id` | string or null | no | Opaque identifier; no URLs, paths, or personal data |
| `component` | enum `app`, `ingestion`, `llm`, `match`, `db`, `ui`, `telemetry` | yes | Safe component name only |
| `source` | string or null | no | Configured safe source name (e.g. `jobspy`, `remotive`) |
| `provider` | string or null | no | Safe provider name (e.g. `gemini`, `openrouter`, `deepseek`) |
| `model` | string or null | no | Safe model identifier |
| `fallback_from` | string or null | no | Previous provider/model, if a fallback occurred |
| `outcome` | enum `SUCCESS`, `FAILURE`, `PARTIAL`, `SKIPPED` | yes | Outcome of the event |
| `elapsed_ms` | integer or null | no | Non-negative milliseconds |
| `error_category` | enum or null | no | `TIMEOUT`, `RATE_LIMIT`, `AUTH`, `NETWORK`, `PARSE`, `IO`, `CONFIG`, `UNKNOWN` |
| `job_ref` | string or null | no | Opaque job reference; never title, company, URL, or description |
| `lifecycle_transition` | enum or null | no | `DISCOVERED`, `APPLIED`, `IGNORED` |
| `dedup_count` | integer or null | no | Non-negative count |
| `message` | string | no | Sanitized, max 512 characters; no URL/path/PII/secret |

**Supported event types**:
`APP_START`, `APP_SHUTDOWN`, `SEARCH_RUN_START`, `SEARCH_RUN_END`,
`SOURCE_FAILURE`, `DEDUP_COMPLETE`, `LLM_CALL`, `LLM_FALLBACK`,
`EVALUATION_COMPLETE`, `APPLICATION_ACTION`.

**Forbidden fields**: raw ingestion payload, model prompt, model response, URL,
filesystem path, job title, job description, company name, candidate name,
email, phone, credential, token, cookie, or API key.

## Log Configuration

Application-level settings resolved at startup.

| Field | Type | Default | Validation |
|-------|------|---------|------------|
| `level` | enum `INFO`, `WARN`, `ERROR`, `DEBUG` | `INFO` | Invalid/empty falls back to `INFO` with a warning |
| `log_dir` | path | Tauri app data `/logs` | Must be a local directory; created with restricted permissions |
| `rotation` | enum `DAILY` | `DAILY` | Fixed for the MVP |
| `retention_days` | integer | `7` | Positive; files older than this are deleted |
| `max_total_bytes` | integer | `52_428_800` (50 MB) | Positive; oldest files deleted until the cap is met |
| `queue_capacity` | integer | `10_000` | Positive; bounded in-process queue |
| `overflow_policy` | enum `DROP_OLDEST` | `DROP_OLDEST` | Fixed for the MVP |

## Log Store Health

A singleton snapshot returned by `cmd_get_log_health`.

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `available` | boolean | yes | Current writer/appender availability |
| `queue_depth` | integer | yes | 0 <= depth <= capacity |
| `queue_capacity` | integer | yes | Positive |
| `dropped_count` | integer | yes | Monotonic non-negative counter |
| `last_error` | string or null | no | Sanitized error category/message, max 512 chars |
| `last_success_at` | string or null | no | RFC 3339 UTC |
| `current_file` | string or null | no | File name only, not a full path |

Health states:

- `AVAILABLE`: writer is accepting events and the appender is writable.
- `DEGRADED`: events are being dropped or the appender had a recoverable error.
- `UNAVAILABLE`: the log directory/file cannot be written; the app continues.

## Execution Run

A correlation record used to group events. It is not persisted as a separate
table in this feature; it is represented by the `run_id` on events.

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `run_id` | string | yes | Opaque stable identifier |
| `kind` | enum `SEARCH`, `EVALUATION`, `MODEL_CALL`, `APPLICATION_ACTION` | yes | Event family |
| `started_at` | string | yes | RFC 3339 UTC |
| `ended_at` | string or null | no | RFC 3339 UTC, >= started_at |
| `outcome` | enum `SUCCESS`, `FAILURE`, `PARTIAL` | no | Final outcome when known |
| `correlation_refs` | array of strings | no | Opaque parent/child run IDs |

## Relationships

- A `Log Event` MAY reference one `Execution Run` through `run_id`.
- `Log Store Health` is a singleton for the process; it is not associated with a
  run.
- `Log Configuration` is application-level and applies to all events.
- An `Execution Run` MAY have many events; an event has at most one run.
- Log records are independent of SQLite job, profile, application, evaluation,
  and analytics records.

## State Transitions

**Log Store Health**

```text
AVAILABLE --(queue full / writer error)--> DEGRADED
AVAILABLE --(file/dir unwritable)--> UNAVAILABLE
DEGRADED --(writer recovers)--> AVAILABLE
DEGRADED --(file/dir unwritable)--> UNAVAILABLE
UNAVAILABLE --(dir/file writable again)--> AVAILABLE
```

**Event lifecycle**

```text
created -> filtered by level -> queued -> written or dropped
```

A dropped event increments `dropped_count`; it is never written later.
