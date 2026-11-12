# Feature Specification: Faster Search

**Feature Branch**: `[015-faster-search]`

**Created**: 2026-11-11

**Status**: Draft

**Input**: User description: "Speed-up the search on some way."

## Context

A search run currently blocks the operator for a long time with no feedback. The work
happens in two sequential stages:

1. **Collection** — four sources run at the same time, but inside each source the
   per-posting detail fetches run one after another. A source that returns N postings
   costs N round trips in series.
2. **Evaluation** — up to 100 postings are evaluated by the language model two at a
   time, and every posting must finish before the run reports anything.

The operator sees only "Searching…" until the whole run ends, so a slow source or a
long evaluation queue looks like a hang.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Collect postings in parallel within each source (Priority: P1)

A search run fetches the details of many postings. Today those fetches are serialized
inside each source, so the run time grows linearly with the number of postings. The
operator wants the same postings collected in a fraction of the time.

**Why this priority**: Collection is the first blocking stage; parallelizing it shortens
every run and does not change what the operator sees in the results.

**Independent Test**: Run a search against a source that returns many postings and
compare the collection duration against the previous serialized behavior, verifying the
same postings (same count, same identifiers) are returned.

**Acceptance Scenarios**:

1. **Given** a source returns several postings, **When** the run collects their details,
   **Then** multiple detail requests are in flight at the same time, up to a configured
   limit.
2. **Given** one detail request fails, **When** the others succeed, **Then** the run keeps
   the successful postings and reports the failure for the failed one, as it does today.
3. **Given** the configured limit is reached, **When** more postings remain, **Then** the
   remaining requests wait instead of exceeding the limit.

---

### User Story 2 - Evaluate postings concurrently and configurable (Priority: P2)

Evaluation is the longest stage. The operator wants the queue drained faster without
breaking provider rate limits.

**Why this priority**: It is the dominant cost on a large result set and has the largest
effect on total run time.

**Independent Test**: Seed more postings than the concurrency limit and confirm the
evaluation stage processes them with the configured number of simultaneous provider
calls, and that the final statuses and scores are unchanged.

**Acceptance Scenarios**:

1. **Given** a configured concurrency of N, **When** the evaluation stage runs, **Then** at
   most N provider calls are in flight at any moment.
2. **Given** the operator raises the concurrency, **When** a run executes, **Then** the run
   uses the new value without restarting.
3. **Given** a provider starts returning rate-limit errors, **When** the run continues,
   **Then** results already produced are preserved and the failures are reported.

---

### User Story 3 - See progress while the run is working (Priority: P3)

The operator wants to know that the run is alive and how far it has progressed, instead of
staring at a disabled button.

**Why this priority**: It does not reduce wall-clock time, but it removes the impression of
a hang and makes long runs tolerable.

**Independent Test**: Start a run with many postings and confirm the interface reports
progress (stage and counts) before the run finishes.

**Acceptance Scenarios**:

1. **Given** a run is collecting postings, **When** collection is in progress, **Then** the
   interface shows that the run is collecting and how many postings were found so far.
2. **Given** a run is evaluating postings, **When** evaluations complete, **Then** the
   interface shows the evaluated count against the total.
3. **Given** the run finishes or fails, **When** it ends, **Then** the progress display
   stops and the existing summary appears.

---

### User Story 4 - Bound the time a single slow source can consume (Priority: P4)

One slow or unresponsive source must not hold the whole run hostage. The operator prefers
partial results to a long wait.

**Why this priority**: Protects the run from the worst case and keeps the other sources'
results usable.

**Independent Test**: Configure a source that never responds and confirm the run finishes
within the configured budget, reporting that source as a failure while keeping the other
sources' results.

**Acceptance Scenarios**:

1. **Given** a source exceeds its time budget, **When** the budget expires, **Then** that
   source is reported as failed and the run continues with the remaining sources.
2. **Given** every source fails, **When** the run ends, **Then** the run is reported as
   failed, as it is today.

### User Story 5 - Use the secondary LinkedIn source only when needed (Priority: P5)

Two sources currently cover the same job board and run at the same time, so the same
postings are collected twice and merged afterwards. The operator wants the secondary
source to run only when the primary one fails to deliver.

**Why this priority**: It removes duplicated collection work and the secondary source's
browser start-up cost on the common path, without losing the safety net.

**Independent Test**: Make the primary source fail (or return nothing) and confirm the
secondary source runs and returns postings; then make the primary succeed and confirm the
secondary source does not run.

**Acceptance Scenarios**:

1. **Given** the primary source succeeds with postings, **When** the run collects, **Then**
   the secondary source does not run.
2. **Given** the primary source fails, **When** the run collects, **Then** the secondary
   source runs and its postings are used.
3. **Given** the primary source succeeds but returns no postings, **When** the run
   collects, **Then** the secondary source runs and its postings are used.
4. **Given** the primary source returns fewer postings than the configured minimum, **When**
   the run collects, **Then** the secondary source runs and its postings are merged with
   the primary's.
5. **Given** both sources fail, **When** the run ends, **Then** the failure is reported
   once per source and the rest of the run continues.

### Edge Cases

- A concurrency limit of one must behave exactly like today's serialized behavior.
- A provider that rejects parallel calls must surface its errors without discarding the
  evaluations that already succeeded.
- Progress reporting must not delay the run or hold a database lock while a provider call
  is in flight.
- A run interrupted by the operator must leave no partially written evaluation rows.
- Duplicate postings merged during collection must not be evaluated twice.
- An empty result set must finish immediately and report zero results.
- A fallback source that also fails must not be retried in a loop within the same run.
- A source configured as a fallback but whose primary is disabled must run as a normal
  source.
- The fallback must respect the same per-source time budget as any other source.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Detail collection inside a source MUST issue multiple requests at the same
  time, bounded by a configurable limit.
- **FR-002**: The collection result MUST be identical to the serialized result: the same
  postings, identifiers, and lineage, with per-posting failures isolated as they are today.
- **FR-003**: The evaluation stage MUST run provider calls concurrently, bounded by a
  configurable limit that applies to the whole stage.
- **FR-004**: The evaluation limit MUST be adjustable without restarting the application
  and MUST take effect on the next run.
- **FR-005**: Raising or lowering the limits MUST NOT change scores, statuses, thresholds,
  or hard-filter outcomes.
- **FR-006**: The interface MUST report the current stage and progress counts while a run is
  active.
- **FR-007**: Progress reporting MUST NOT block the run and MUST NOT hold a database lock
  across a provider call.
- **FR-008**: Each source MUST be bounded by a configurable time budget; exceeding it MUST
  be reported as a source failure without discarding other sources' results.
- **FR-009**: A run that finishes with zero postings MUST complete immediately and report
  zero results.
- **FR-010**: Rate-limit or transient provider errors MUST preserve the evaluations that
  already completed.
- **FR-011**: Duplicate postings merged during collection MUST be evaluated at most once per
  run.
- **FR-012**: The limits MUST have conservative built-in defaults that do not exceed the
  providers' documented tolerances.
- **FR-013**: A source MUST be configurable as the fallback of another source.
- **FR-014**: A fallback source MUST run when its primary source fails **or** returns fewer
  postings than the configured minimum, including the case where it returns zero postings
  successfully.
- **FR-015**: A fallback source MUST NOT run when its primary source succeeds with enough
  postings.
- **FR-016**: The fallback relationship MUST NOT change the per-source failure reporting:
  each source that failed is still reported individually.
- **FR-017**: A source declared as a fallback whose primary is disabled or absent MUST run
  as a normal source.

### Key Entities *(include if feature involves data)*

- **Run Progress**: the stage a run is in (collecting or evaluating) and the counts
  observed so far (found, evaluated, failed).
- **Concurrency Limits**: the maximum number of simultaneous detail requests per source and
  the maximum number of simultaneous provider calls during evaluation.
- **Source Time Budget**: the maximum time a single source may consume within a run.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A run over the same result set completes at least 40% faster than the current
  serialized baseline on the operator's machine.
- **SC-002**: With a source returning 20 postings, collection time is at least 3× shorter
  than the serialized baseline.
- **SC-003**: The operator sees progress information within 5 seconds of starting a run, and
  the reported counts advance at least once every 10 seconds while work continues.
- **SC-004**: A run with one unresponsive source finishes within that source's time budget
  plus 60 seconds, instead of waiting for the source's full timeout.
- **SC-005**: Evaluation outcomes (status, score, matched and missing keywords, rejection
  codes) are identical to the current behavior for the same inputs.
- **SC-006**: No run leaves partial evaluation rows behind after a failure.
- **SC-007**: When the primary source succeeds, the secondary LinkedIn source performs zero
  collection requests in that run.
- **SC-008**: When the primary source fails or returns nothing, the run still produces the
  secondary source's postings, with the same result quality as running it as a normal
  source.

## Assumptions

- The operator runs the app on a machine with enough network capacity for a handful of
  simultaneous requests; the limits stay conservative.
- Provider rate limits are respected by keeping the default evaluation concurrency at or
  below the value already tolerated today multiplied by two.
- The current single-flight rule stays: one run at a time.
- Existing retry and fallback semantics are preserved; concurrency only changes how many
  jobs are processed at once.
- Progress reporting is in-process; no external telemetry is added.
- The scoring rules, thresholds, hard filters, and deduplication semantics are unchanged.
- The two LinkedIn sources are treated as primary and fallback of the same job board; the
  secondary one ignores date and experience filters, so a run that used it may include
  older postings that the primary would have excluded. The hard filters still bound the
  outcome.
- The fallback decision happens per run, not per query.
