# Feature Specification: Faster Search

**Feature Branch**: `[015-faster-search]`

**Created**: 2026-11-11

**Status**: Draft

**Input**: User description: "Speed-up the search on some way."

## Context

A search run currently blocks the operator for a long time with no feedback. The work
happens in two sequential stages:

1. **Collection** — four sources run at the same time. Inside the authenticated LinkedIn
   source, each listed posting needs a separate detail request, and the LinkedIn MCP server
   serializes those calls because only one call can drive the shared browser profile. On a
   repeat run most of the listed postings are already stored with their descriptions, so the
   detail requests repeat work the application has already done.
2. **Evaluation** — up to 100 postings are evaluated by the language model two at a time, and
   every posting must finish before the run reports anything.

The operator sees only "Searching…" until the whole run ends, so a slow source or a long
evaluation queue looks like a hang. The two LinkedIn sources also run at the same time, so the
same job board is collected twice before deduplication.

## User Scenarios & Testing _(mandatory)_

### User Story 1 - Reuse postings already collected (Priority: P1)

On repeat runs, most of the postings a source lists are already stored with their descriptions.
Today the application downloads every detail again. The operator wants that work skipped: a
posting already stored with a usable description should only refresh its "seen" timestamp,
while new postings and postings stored without a description are still collected in full.

**Why this priority**: Collection is the first blocking stage. Skipping known postings removes
most of the LinkedIn detail requests on repeat runs without changing what the operator sees in
the results.

**Independent Test**: Run the same search twice for the same profile and filters. The second run
performs no detail request for a posting already stored with a non-empty description, still
records the LinkedIn lineage for it, and fetches details for new or description-less postings.

**Acceptance Scenarios**:

1. **Given** a listed posting is already stored with a non-empty clean description, **When**
   collection runs, **Then** no detail request is made for it and its source lineage is marked
   as seen in this run.
2. **Given** a listed posting is stored without a description, **When** collection runs,
   **Then** its detail is fetched and the stored description is filled in.
3. **Given** a posting was first stored by another source with a `linkedin-` external ID and a
   non-empty description, **When** the LinkedIn source lists it, **Then** the detail is skipped
   and the LinkedIn lineage is added.
4. **Given** every listed posting is already known, **When** collection finishes, **Then** the
   source counts as delivered for the fallback decision and the run does not start the fallback.
5. **Given** a known posting has a changed description at the source, **When** the run lists it,
   **Then** the stored description is kept and no refresh is attempted.

---

### User Story 2 - Evaluate postings concurrently and configurable (Priority: P2)

Evaluation is the longest stage. The operator wants the queue drained faster without breaking
provider rate limits, and wants to adjust the limit from the provider settings.

**Why this priority**: It is the dominant cost on a large result set and has the largest effect
on total run time.

**Independent Test**: Seed more postings than the concurrency limit and confirm the evaluation
stage processes them with the configured number of simultaneous provider calls, and that the
final statuses and scores are unchanged.

**Acceptance Scenarios**:

1. **Given** a configured concurrency of N, **When** the evaluation stage runs, **Then** at most
   N provider calls are in flight at any moment.
2. **Given** the operator changes the concurrency in the provider settings, **When** the next
   run executes, **Then** the run uses the new value without restarting the application.
3. **Given** a provider starts returning rate-limit errors, **When** the run continues, **Then**
   results already produced are preserved and the failures are reported.

---

### User Story 3 - See progress while the run is working (Priority: P3)

The operator wants to know that the run is alive and how far it has progressed, instead of
staring at a disabled button.

**Why this priority**: It does not reduce wall-clock time, but it removes the impression of a
hang and makes long runs tolerable.

**Independent Test**: Start a run with many postings and confirm the interface reports progress
(stage and counts) before the run finishes.

**Acceptance Scenarios**:

1. **Given** a run is collecting postings, **When** collection is in progress, **Then** the
   interface shows that the run is collecting and how many postings were found so far.
2. **Given** a run is evaluating postings, **When** evaluations complete, **Then** the interface
   shows the evaluated count against the total.
3. **Given** the run finishes or fails, **When** it ends, **Then** the progress display stops and
   the existing summary appears.

---

### User Story 4 - Bound the time a single slow source can consume (Priority: P4)

One slow or unresponsive source must not hold the whole run hostage. The operator prefers
partial results to a long wait.

**Why this priority**: Protects the run from the worst case and keeps the other sources' results
usable.

**Independent Test**: Configure a source that never responds and confirm the run finishes within
the configured budget, reporting that source as a failure while keeping the other sources'
results.

**Acceptance Scenarios**:

1. **Given** a source exceeds its time budget, **When** the budget expires, **Then** that source
   is reported as failed and the run continues with the remaining sources.
2. **Given** every source fails, **When** the run ends, **Then** the run is reported as failed,
   as it is today.

### User Story 5 - Use the secondary LinkedIn source only when needed (Priority: P5)

Two sources currently cover the same job board and run at the same time, so the same postings
are collected twice and merged afterwards. The operator wants the secondary source to run only
when the primary one fails to deliver.

**Why this priority**: It removes duplicated collection work and the secondary source's browser
start-up cost on the common path, without losing the safety net.

**Independent Test**: Make the primary source fail (or return nothing) and confirm the secondary
source runs and returns postings; then make the primary succeed and confirm the secondary source
does not run.

**Acceptance Scenarios**:

1. **Given** the primary source delivers enough postings, **When** the run collects, **Then** the
   secondary source does not run and performs no requests.
2. **Given** the primary source fails, **When** the run collects, **Then** the secondary source
   runs and its postings are used.
3. **Given** the primary source succeeds but delivers no postings, **When** the run collects,
   **Then** the secondary source runs and its postings are used.
4. **Given** the primary source delivers fewer postings than the configured minimum, **When** the
   run collects, **Then** the secondary source runs and its postings are merged with the
   primary's.
5. **Given** the primary source lists only postings that are already stored, **When** the run
   collects, **Then** those postings count as delivered and the secondary source does not run.
6. **Given** the primary source is disabled or absent, **When** the run collects, **Then** the
   source configured as its fallback runs as a normal source.
7. **Given** both sources fail, **When** the run ends, **Then** the failure is reported once per
   source and the rest of the run continues.
8. **Given** the primary source fails quickly while an unrelated source is still running, **When**
   the fallback starts, **Then** it does not wait for the unrelated source to finish.

### Edge Cases

- A concurrency limit of one must behave exactly like today's serialized behavior.
- A provider that rejects parallel calls must surface its errors without discarding the
  evaluations that already succeeded.
- Progress reporting must not delay the run or hold a database lock while a provider call is in
  flight.
- A run interrupted by the operator must leave no partially written evaluation rows.
- Duplicate postings merged during collection must not be evaluated twice.
- An empty result set must finish immediately and report zero results.
- A fallback source that also fails must not be retried in a loop within the same run.
- A source configured as a fallback but whose primary is disabled must run as a normal source.
- The fallback must respect the same per-source time budget as any other source.
- A known posting with an empty stored description must still be collected in full.
- A known posting with a changed description keeps the stored description; no refresh is
  attempted.
- A source that lists only known postings still counts as delivered for the fallback decision.
- A fallback chain and two fallbacks for the same primary are rejected when the configuration
  loads.
- A skipped fallback must not change the source's recorded last-run status.

## Requirements _(mandatory)_

### Functional Requirements

- **FR-001**: Collection MUST NOT fetch the detail of a posting already stored with a non-empty
  clean description; the known-posting lookup MUST use a stable external identifier.
- **FR-002**: Skipping known postings MUST preserve the existing postings, lineage, deduplication,
  and hard-filter semantics. A skipped posting MUST still count as seen for recency and for the
  fallback delivery decision. Postings stored without a description MUST still be collected.
- **FR-003**: The evaluation stage MUST run provider calls concurrently, bounded by a configurable
  limit that applies to the whole stage.
- **FR-004**: The evaluation limit MUST be adjustable from the provider settings without
  restarting the application and MUST take effect on the next run.
- **FR-005**: Raising or lowering the limits MUST NOT change scores, statuses, thresholds, or
  hard-filter outcomes.
- **FR-006**: The interface MUST report the current stage and progress counts while a run is
  active. Progress MUST be aggregate; it MUST NOT expose job titles, URLs, or descriptions.
- **FR-007**: Progress reporting MUST NOT block the run and MUST NOT hold a database lock across a
  provider call.
- **FR-008**: Each source MUST be bounded by a configurable time budget; exceeding it MUST be
  reported as a source failure without discarding other sources' results.
- **FR-009**: A run that finishes with zero postings MUST complete immediately and report zero
  results.
- **FR-010**: Rate-limit or transient provider errors MUST preserve the evaluations that already
  completed.
- **FR-011**: Duplicate postings merged during collection MUST be evaluated at most once per run.
- **FR-012**: The limits MUST have conservative built-in defaults that do not exceed the
  providers' documented tolerances.
- **FR-013**: A source MUST be configurable as the fallback of another source.
- **FR-014**: A fallback source MUST run when its primary source fails **or** delivers fewer
  postings than the configured minimum, including the case where it delivers zero. "Delivered"
  means new postings plus postings that were listed and already known.
- **FR-015**: A fallback source MUST NOT run when its primary source delivers enough postings.
- **FR-016**: The fallback relationship MUST NOT change the per-source failure reporting: each
  source that failed is still reported individually.
- **FR-017**: A source declared as a fallback whose primary is disabled or absent MUST run as a
  normal source.
- **FR-018**: A fallback MUST start as soon as its own primary is decided, without waiting for
  unrelated primaries to finish.
- **FR-019**: Skipping a known posting MUST update its source lineage recency but MUST NOT
  re-evaluate an evaluation that is not stale.

### Key Entities _(include if feature involves data)_

- **Known Posting**: a persisted posting with a stable external identifier, its internal job ID,
  its URL, and a non-empty clean description. Used to decide whether a detail request can be
  skipped.
- **Fetch Outcome**: the result of one source fetch: the new postings with details, how many
  postings were listed, and which listed postings were skipped because they were already known.
- **Run Progress**: the stage a run is in (collecting or evaluating) and the counts observed so
  far (found, evaluated, total, failed).
- **Concurrency Limits**: the maximum number of simultaneous provider calls during evaluation,
  adjustable from the provider settings.
- **Source Time Budget**: the maximum time a single source may consume within a run.
- **Fallback Relationship**: a source declared as the fallback of another source, with a minimum
  delivery count that decides whether it runs.

## Success Criteria _(mandatory)_

### Measurable Outcomes

- **SC-001**: A run over the same result set completes at least 40% faster than the current
  baseline on the operator's machine. This is a product target measured manually; the automated
  gate verifies the mechanism in SC-002, not the wall-clock target.
- **SC-002**: On a second run with the same profile and filters, the LinkedIn source performs zero
  detail requests for postings already stored with a non-empty description.
- **SC-003**: The operator sees progress information within 5 seconds of starting a run, and the
  reported counts advance at least once every 10 seconds while work continues.
- **SC-004**: A run with one unresponsive source finishes within that source's time budget plus
  60 seconds, instead of waiting for the source's full timeout.
- **SC-005**: Evaluation outcomes (status, score, matched and missing keywords, rejection codes)
  are identical to the current behavior for the same inputs.
- **SC-006**: No run leaves partial evaluation rows behind after a failure.
- **SC-007**: When the primary source delivers enough postings, the secondary LinkedIn source
  performs zero collection requests in that run.
- **SC-008**: When the primary source fails or delivers nothing, the run still produces the
  secondary source's postings, with the same result quality as running it as a normal source.
- **SC-009**: A primary source that lists only already-known postings counts as delivered, and
  the fallback performs zero requests in that run.
- **SC-010**: A fallback starts without waiting for an unrelated slow primary; the delay between
  the primary's decision and the fallback's start is bounded by the fallback's own start-up.

## Assumptions

- The operator runs the app on a machine with enough network capacity for a handful of
  simultaneous requests; the limits stay conservative.
- Provider rate limits are respected by keeping the default evaluation concurrency at four, with
  an operator-adjustable ceiling of eight.
- The current single-flight rule stays: one run at a time.
- Existing retry and fallback semantics are preserved; concurrency only changes how many jobs are
  processed at once.
- Progress reporting is in-process; no external telemetry is added.
- The scoring rules, thresholds, hard filters, and deduplication semantics are unchanged.
- The authenticated LinkedIn MCP server serializes tool calls because only one call can drive the
  shared browser profile. Parallel detail fetching is not a viable path for this feature.
- The two LinkedIn sources are treated as primary and fallback of the same job board; the
  secondary one ignores date and experience filters, so a run that used it may include older
  postings that the primary would have excluded. The hard filters still bound the outcome.
- The fallback decision happens per run, not per query.
- A known posting keeps the description captured on its first successful collection. Refreshing
  changed descriptions is out of scope for this feature.
