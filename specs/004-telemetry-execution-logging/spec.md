# Feature Specification: Local Telemetry & Execution Logging

**Feature Branch**: `N/A (no branch hook configured)`

**Created**: 2026-11-10

**Status**: Draft

**Input**: User description: "Add local code-level telemetry and execution logging to
help diagnose errors, with INFO/WARN/ERROR by default, DEBUG opt-in, a rotating
local log file, and no raw payloads, model prompts/responses, or external log
repository."

This specification extends the local job sourcing system defined in
`specs/001-career-job-sourcing/spec.md`,
`specs/002-job-sourcing-dashboard/spec.md`, and
`specs/003-skill-gap-analytics/spec.md`. It defines local, code-oriented
telemetry for diagnosing failures. It does not store raw ingestion payloads,
complete model prompts/responses, URLs, filesystem paths, candidate data, or
credentials.

The ratified constitution v2.0.0 keeps profile, resume, and application data
local, removes mandatory application-level encryption at rest, and prohibits
external telemetry. Logs are technical diagnostics, separate from transactional
data and analytics.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Capture lifecycle and execution events for diagnosis (Priority: P1)

The System Operator and maintainers need a reliable record of what happened
during startup, job discovery, model calls, deduplication, and application
actions. Each event identifies when it happened, at what severity, in which run,
and with what outcome, so a failure can be diagnosed without reproducing it.

**Why this priority**: Event capture is the foundation of telemetry and can be
delivered independently of any UI or external log system.

**Independent Test**: Start the application, run a search with one failing source
and one successful source, and trigger a model evaluation. Verify that startup,
source failure, model latency, fallback, evaluation, and deduplication events
exist with timestamp, level, event type, run identifier, and outcome.

**Acceptance Scenarios**:

1. **Given** the application starts, **When** startup completes, **Then** an INFO
   event records the startup, application version, session identifier, and active
   log level.
2. **Given** one ingestion source fails, **When** the search run completes,
   **Then** a WARN or ERROR event records the source, search run identifier,
   failure category, and elapsed time without credentials or payload content.
3. **Given** a model call completes or falls back to another provider, **When**
   the evaluation finishes, **Then** an event records the provider, model,
   duration, outcome, fallback origin, and evaluation run identifier without
   prompts or responses.
4. **Given** duplicate postings are merged, **When** deduplication completes,
   **Then** an INFO event records the run identifier and number of duplicates
   merged.
5. **Given** the operator applies or ignores a job, **When** the action completes,
   **Then** an INFO event records the job reference, lifecycle transition, and
   timestamp without job title, company, URL, or description.

---

### User Story 2 - Control log levels and local retention (Priority: P2)

The operator can set the log level through local environment configuration and
rely on a bounded rotating log file. INFO, WARN, and ERROR are emitted by
default; DEBUG is opt-in for a deeper investigation.

**Why this priority**: Logs are useful only if they are adjustable and do not
grow indefinitely on a local desktop.

**Independent Test**: Run with the default level, then with `RUST_LOG=debug`.
Verify the level filtering, rotate the log beyond the retention limit, and
confirm that older files are removed.

**Acceptance Scenarios**:

1. **Given** the configured level is INFO, **When** INFO, WARN, ERROR, and DEBUG
   events are emitted, **Then** only INFO, WARN, and ERROR are written.
2. **Given** the configured level is DEBUG, **When** the same events are emitted,
   **Then** all four levels are written.
3. **Given** the configured level is invalid, **When** the application starts,
   **Then** it falls back to INFO and writes a warning about the invalid value.
4. **Given** the log directory exceeds 7 days or 50 MB, **When** rotation runs,
   **Then** the oldest files are removed and the total stays within the limit.
5. **Given** the operator inspects the log location, **When** logs exist, **Then**
   they are local files under the application data directory and no external log
   destination is configured or contacted.

---

### User Story 3 - Keep logging safe and non-blocking (Priority: P3)

Logging never blocks the desktop interface, search pipeline, evaluation, or
application actions. Log records contain only technical fields; they never
include credentials, raw payloads, prompts, responses, URLs, paths, or candidate
data.

**Why this priority**: A diagnostic subsystem MUST NOT become a privacy leak or
a source of UI stalls.

**Independent Test**: Emit a burst of 1,000 events, make the log file
unavailable, and verify that the UI and pipeline remain responsive. Inspect the
logs for credentials, URLs, paths, prompts, payloads, and candidate data.

**Acceptance Scenarios**:

1. **Given** 1,000 events are emitted during a user action, **When** they are
   written, **Then** the interface remains responsive and the action completes
   without waiting for the log file.
2. **Given** the log file is unavailable or the disk is full, **When** events are
   emitted, **Then** the pipeline continues, the failure is surfaced in the log
   health state, and the application does not hang.
3. **Given** the bounded queue is full, **When** more events arrive, **Then** the
   oldest events are dropped, the dropped count increases, and memory does not
   grow without bound.
4. **Given** the application shuts down with pending events, **When** the shutdown
   sequence runs, **Then** pending events are flushed within 5 seconds or counted
   as dropped, and shutdown is not blocked indefinitely.
5. **Given** any log event, **When** it is inspected, **Then** it contains no
   credentials, session material, raw ingestion payload, model prompt or
   response, URL, filesystem path, job title/description, or candidate data.

---

### Edge Cases

- The log file cannot be created, is locked, or the disk is full.
- The bounded queue fills during a DEBUG session or a large search.
- The application crashes or shuts down while events are buffered.
- Multiple runs or workers emit events concurrently.
- The system clock changes or events arrive out of order.
- Rotation runs while an event is being written.
- The operator sets an invalid or empty log level.
- DEBUG volume overwhelms the retention limit.
- A log event would otherwise contain a URL, path, prompt, payload, or candidate
  field.
- The operator opens, moves, or deletes the log file while the app is running.
- The application data directory is unavailable or read-only.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-LOG-001 (Lifecycle Events)**: The system MUST log startup, shutdown,
  ingestion-source failures, deduplication, search runs, model calls and
  fallback, evaluation runs, and application/ignore actions.
- **FR-LOG-002 (Event Fields)**: Every event MUST include a timestamp, level,
  event type, run or correlation identifier, component/source, outcome, and
  elapsed time where applicable.
- **FR-LOG-003 (Log Levels)**: The system MUST support INFO, WARN, ERROR, and
  DEBUG. INFO, WARN, and ERROR MUST be enabled by default; DEBUG MUST be opt-in.
- **FR-LOG-004 (Level Configuration)**: The active level MUST be configurable via
  `RUST_LOG` or the local environment configuration without a code change. An
  invalid value MUST fall back to INFO and emit a warning.
- **FR-LOG-005 (Local File Only)**: Logs MUST be written to a local rotating file
  under the application data directory. The system MUST NOT send logs to
  MongoDB, a remote log repository, or any third party.
- **FR-LOG-006 (Rotation and Retention)**: Log files MUST rotate daily and MUST
  be retained for at most 7 days or 50 MB total, whichever limit is reached
  first. Older files MUST be deleted automatically.
- **FR-LOG-007 (Asynchronous Dispatch)**: Log writes MUST be dispatched
  asynchronously and MUST NOT block the desktop interface, ingestion, evaluation,
  or application actions.
- **FR-LOG-008 (Bounded Queue)**: The dispatch queue MUST have a configured
  capacity and a documented overflow policy. Dropped events MUST be counted.
- **FR-LOG-009 (Failure Isolation)**: A missing, locked, full, or failing log
  file MUST NOT halt a search, evaluation, or application action. The failure
  MUST be surfaced in the log health state.
- **FR-LOG-010 (Shutdown Flush)**: Pending events MUST be flushed within 5
  seconds on shutdown or counted as dropped. Shutdown MUST NOT hang.
- **FR-LOG-011 (Safe Fields)**: Log events MUST NOT contain credentials, session
  material, raw ingestion payloads, complete model prompts/responses, URLs,
  filesystem paths, job titles/descriptions, or candidate data.
- **FR-LOG-012 (Local Protection)**: The log directory MUST use restricted
  filesystem permissions and rely on OS-level disk encryption where available.
  The application MUST NOT claim encryption it does not provide.
- **FR-LOG-013 (Correlation)**: Events MUST be correlatable by run identifier so
  a search, evaluation, model call, or application action can be reconstructed.
- **FR-LOG-014 (No Transactional Side Effects)**: Logging and rotation MUST NOT
  create, modify, or delete transactional job, profile, application, or analytics
  records.
- **FR-LOG-015 (Log Health)**: The system MUST expose log-file availability,
  queue depth, dropped-event count, and the last logging error to the operator or
  maintainer without exposing sensitive data.
- **FR-LOG-016 (File-Only Access)**: The MVP MUST NOT provide an in-app log
  viewer or export feature. Logs are inspected as local files.
- **FR-LOG-017 (No External Telemetry)**: The system MUST NOT transmit logs or
  diagnostics to external services, including during crashes or error reporting.

### Key Entities *(include if feature involves data)*

- **Log Event**: A structured technical record. Key attributes: timestamp, level,
  event type, run identifier, component/source, outcome, elapsed time, error
  category, and a sanitized message.
- **Log Configuration**: The active logging settings. Key attributes: level,
  file directory, rotation policy, retention limit, queue capacity, and overflow
  policy.
- **Log Store Health**: The current logging state. Key attributes: availability,
  queue depth, dropped-event count, last error, and last successful write.
- **Execution Run**: A search, evaluation, model call, or application action that
  groups related events. Key attributes: run identifier, start/end time, outcome,
  and correlation references.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of the specified lifecycle events produce a log event with
  timestamp, level, event type, run identifier, and outcome.
- **SC-002**: INFO, WARN, and ERROR are written by default; DEBUG is written only
  when explicitly enabled; an invalid level falls back to INFO with a warning.
- **SC-003**: A burst of 1,000 events does not block the interface or any search,
  evaluation, or application action.
- **SC-004**: A missing, locked, or full log file does not halt the pipeline; the
  failure is visible and the dropped count is tracked.
- **SC-005**: Log rotation keeps the total within 7 days or 50 MB in 100% of
  tested runs.
- **SC-006**: No log event contains credentials, raw payloads, prompts/responses,
  URLs, paths, job descriptions, or candidate data in 100% of inspected events.
- **SC-007**: No log data is sent to an external service in 100% of tested runs.
- **SC-008**: Shutdown with pending events completes within 5 seconds and does not
  hang.
- **SC-009**: Logging, rotation, and deletion never change transactional jobs,
  profiles, applications, or analytics data.
- **SC-010**: A maintainer can identify the failing source, error category, model
  latency, and affected run from the log file within 1 minute.

## Assumptions

- The MVP targets Linux only and stores logs under the application data
  directory.
- Logging uses `tracing` with `tracing-subscriber` and a local rotating file
  appender. `RUST_LOG` or the local environment configuration controls the level.
- The default level is INFO; DEBUG is opt-in and may produce a large volume.
- Rotation is daily, retention is 7 days or 50 MB total, and the queue capacity
  defaults to 10,000 events with oldest-first dropping.
- The log file has restricted permissions. The constitution v2.0.0 requires
  local-only storage and relies on OS-level disk encryption and filesystem
  permissions rather than application-level encryption.
- No in-app log viewer, export, MongoDB, external log repository, SIEM, alerting,
  distributed tracing, or raw payload/prompt retention is in scope.
- Logs are technical diagnostics only. They are separate from transactional data,
  audit records, and analytics metrics.
- The operator is responsible for protecting access to the local log directory.
- The feature extends `specs/001-career-job-sourcing/spec.md`,
  `specs/002-job-sourcing-dashboard/spec.md`, and
  `specs/003-skill-gap-analytics/spec.md` without changing their job, profile,
  application, or analytics behavior.
