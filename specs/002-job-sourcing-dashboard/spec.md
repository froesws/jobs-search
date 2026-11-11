# Feature Specification: Job Sourcing Dashboard & Application Tracker

**Feature Branch**: `N/A (no branch hook configured)`

**Created**: 2026-11-10

**Status**: Draft

**Input**: User description: "Local desktop job sourcing workflow with persisted
PDF resumes, configurable search controls, a card dashboard and side-drawer
review, application lifecycle tracking, and configurable language-model
provider routing."

This specification extends the core pipeline defined in
`specs/001-career-job-sourcing/spec.md`. It adds resume persistence and
automatic profile recalculation, the desktop control bar and dashboard, job
detail review, application lifecycle tracking, and configurable language-model
provider routing. Requirements restated here take precedence for the desktop
workflow; core requirements not restated remain in force.

The primary actor is the System Operator, who uploads resumes, configures
searches, triggers runs, reviews matches, and tracks applications. Supporting
capabilities are resume ingestion, job-source integrations, match evaluation,
language-model provider routing, local persistence, and the desktop interface.

## V1 Scope Amendments

This section supersedes conflicting requirements below.

- **Product**: a new Linux-only Jobs Search app; `contra_maquina` is separate.
- **Stack**: Tauri 2.12.5, React 19.3.1, TypeScript 7.0.2, Vite 8.3.6, Rust
  1.90+, MUI 9.5 + Emotion; hooks + Tauri wrappers. No Tailwind, TanStack Query,
  or Zustand.
- **Tabs**: New Jobs, My Applications, My Profile, Analytics. UI labels are
  English: "New Jobs", "My Applications", "My Profile", "I Applied", and
  "Refresh Search".
- **My Profile**: minimal editor for acceptable technologies and accepted
  cities. The manual technology list is merged into the profile's core
  competencies and used in both the hard filter and the core-skills score.
  Changes apply on the next search; there is no automatic historical
  re-evaluation.
- **Profile**: single active profile; no multi-profile selector. Deleting the
  last resume preserves the profile, marks it "no resumes", and blocks new
  searches until a new resume is uploaded.
- **Privacy**: constitution v2.0.0; local-only; no app-level encryption at rest;
  OS disk encryption and permissions are the baseline. API keys live in `.env`.
  No `description_raw` persistence; only clean description and metadata.
- **Status**: separate `evaluation_status` (PENDING/APPROVED/REJECTED/ERROR) and
  `lifecycle_state` (DISCOVERED/APPLIED/IGNORED), with append-only history.
  "Below threshold" is a filter in New Jobs; manual approval sets APPROVED with
  `approval_source = manual`.
- **Matching**: strict location (remote Brazil or on-site/hybrid in Porto Alegre
  or Curitiba); Trainee/Junior/Pleno (Pleno/Senior ranges accepted); core tech
  from the merged profile/manual list; Trainee/Intern any technology; no work
  authorization/visa or salary hard filter; threshold 70.
- **Search**: default 100 evaluations per run, configurable; 2 concurrent;
  provider fallback Gemini → OpenRouter free → DeepSeek.
- **Tests**: Rust, frontend, and Python; one commit per completed task.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Manage resumes and keep the candidate profile current (Priority: P1)

The System Operator uploads one or more PDF resumes from the top control bar.
The system stores each file and its metadata locally, extracts competencies,
and merges the results into a single CandidateProfile. Adding or deleting a
resume triggers a background recalculation. The operator can review and edit
the profile, including constraints that cannot be extracted, before running a
search.

**Why this priority**: The profile is the foundation of every match. Resume
management is the operator's entry point and the smallest independently useful
slice of the desktop workflow.

**Independent Test**: Upload a text-based PDF resume and verify that the file and
its metadata are stored locally and the profile is recalculated. Delete the
resume and verify that the profile is recalculated without its unique
contributions. Simulate a recalculation failure and verify that the last valid
profile remains available.

**Acceptance Scenarios**:

1. **Given** an operator uploads a new resume PDF, **When** the Resume Manager
   processes it, **Then** the file and its content-identity metadata are stored
   locally, the active language-model provider is invoked for extraction, and
   the consolidated CandidateProfile is updated with the merged skill set.
2. **Given** two resumes with overlapping competencies and conflicting scalar
   values, **When** both are added, **Then** list-valued attributes are merged
   without duplicates, conflicting scalar values are flagged for review, and no
   value is silently discarded.
3. **Given** an existing profile, **When** the operator edits competencies,
   dealbreakers, target modalities, salary thresholds, or work-authorization
   constraints, **Then** the updated profile is saved and used by the next
   search.
4. **Given** a stored resume, **When** the operator deletes it, **Then** the
   file and its metadata are removed locally, the deleted resume is removed from
   the profile's source set, and the profile is recalculated.
5. **Given** a recalculation failure, **When** the operation completes,
   **Then** the system reports the failure, retains the last valid profile, and
   does not persist a partial profile.
6. **Given** the same resume is uploaded twice, **When** the second upload is
   processed, **Then** the system does not create a duplicate source document or
   corrupt the profile, and it informs the operator.
7. **Given** an unsupported or unreadable file, **When** the operator uploads
   it, **Then** the system rejects it with a specific error and leaves the
   existing profile unchanged.

---

### User Story 2 - Configure searches and ingest deduplicated jobs (Priority: P2)

The System Operator configures the top control bar with a posting timeframe, a
seniority level, a primary language or stack keyword, and work modalities, then
triggers a search. The system queries registered job sources independently and
concurrently, cleans and normalizes the results, deduplicates them, and
preserves source lineage. A failure in one source does not stop the others.

**Why this priority**: Search and ingestion create the opportunity pool. This
slice can be tested without the dashboard or application tracking and delivers a
normalized, deduplicated dataset.

**Independent Test**: Set the filters to "Last 7 Days", "Senior", "Rust", and
"Remote", run a search with three sources where one fails, and verify that the
returned jobs match the filters, duplicates are merged, source lineage is
preserved, and the failure is recorded.

**Acceptance Scenarios**:

1. **Given** the operator sets the posting timeframe to "Last 7 Days", the
   seniority to "Senior", the stack keyword to "Rust", and the modality to
   "Remote", **When** the search runs, **Then** only jobs satisfying the active
   filters are returned to the dashboard.
2. **Given** three registered sources and one source times out or returns a
   server error, **When** the search runs, **Then** the system records the
   failure for that source, completes using the remaining sources, and does not
   halt the run.
3. **Given** the same opportunity published by two sources, **When** both are
   ingested, **Then** the system merges them into one canonical JobPosting with
   both source identifiers, one stable canonical identity, and conflicting
   values retained for audit.
4. **Given** a posting containing markup tags, control characters, and irregular
   whitespace, **When** it is ingested, **Then** the clean description used
   downstream contains none of those artifacts while the original content
   remains available only for the active processing run.
5. **Given** a source that enforces a request limit, **When** the search runs,
   **Then** the system throttles requests, applies bounded backoff, respects the
   source's access controls and terms, and records exhausted retries without
   retrying indefinitely.
6. **Given** a search that returns no jobs or a run where every source fails,
   **When** the run completes, **Then** the system reports the empty or failed
   outcome clearly and preserves the execution record.
7. **Given** a posting with no posted time, **When** a timeframe filter is
   active, **Then** the system labels the posting's age as Unknown and does not
   silently treat it as inside the timeframe.
8. **Given** a registered source is removed, **When** removal completes,
   **Then** its credentials and session material are deleted and it no longer
   participates in searches.

---

### User Story 3 - Evaluate jobs with hard filters and explainable scores (Priority: P3)

The Match Engine compares each deduplicated job with the active CandidateProfile.
It applies hard filters first, then uses deterministic rules and the active
language-model provider to extract requirements and produce a 0-100 match score,
a breakdown across four weighted dimensions, and an explicit matched-versus-total
requirement count (N out of M). Jobs that trigger a hard filter are rejected and
kept out of the dashboard.

**Why this priority**: This is the core decision capability that turns a job
pool into a prioritized set of opportunities. It depends on P1 and P2 but can be
tested independently with fixed profile and posting fixtures.

**Independent Test**: Provide a fixed profile and a set of jobs covering high
match, seniority mismatch, modality mismatch, missing mandatory competency,
missing mandatory technology, unknown metadata, and provider failure. Verify
evaluation outcomes, scores, N/M counts, breakdowns, and reasons.

**Acceptance Scenarios**:

1. **Given** a "Senior Rust / Systems Engineer" job requiring Rust, Linux, and
   gRPC, offering remote work in Brazil, and a candidate profile targeting
   Mid/Senior engineering with Rust and Linux, **When** evaluated, **Then** the
   job receives an APPROVED evaluation with a score above 85, a technical-overlap
   breakdown, an accurate N/M requirement count, and no hard-filter flags.
2. **Given** a job for a 10+ year Vice President of Sales role and a candidate
   profile targeting Individual Contributor engineering roles, **When**
   evaluated, **Then** the evaluation outcome is REJECTED, the reason "Seniority
   out of target range: Executive/VP" is recorded, and the job is excluded from
   the dashboard.
3. **Given** a job whose modality or location conflicts with a non-negotiable
   profile constraint, **When** evaluated, **Then** the evaluation is REJECTED
   with a specific reason and the job is excluded from the dashboard.
4. **Given** a job missing a core domain competency that the profile marks as a
   mandatory dealbreaker, **When** evaluated, **Then** the evaluation is
   REJECTED with the missing requirement recorded.
5. **Given** a job that names none of the active profile's acceptable
   technologies and is not Trainee/Intern, **When** evaluated, **Then** the
   evaluation is REJECTED with a specific technology reason.
6. **Given** a job passes all hard filters, **When** the evaluation completes,
   **Then** the score is an integer from 0 to 100, the four dimension weights
   sum to 100%, and the weighted contributions reconcile to the reported score.
7. **Given** a job with extracted requirements, **When** the evaluation
   completes, **Then** the matched requirement count plus the missing
   requirement count equals the total extracted requirement count, and the
   N/M ratio is displayed without double-counting.
8. **Given** a job with no extracted requirements, **When** evaluated, **Then**
   the system does not divide by zero and does not imply a perfect match.
9. **Given** a job whose requirements or metadata are unknown, **When** the
   profile marks that criterion as mandatory, **Then** the system does not treat
   the unknown value as a match and keeps the job out of the approved set.
10. **Given** the same profile version, job, provider, model, and active rules,
    **When** evaluated twice, **Then** the evaluation outcome, score, N/M count,
    and breakdown are identical.
11. **Given** the active provider fails or returns malformed output, **When**
    evaluation runs, **Then** the system records an evaluation error, assigns no
    score, does not approve the job, and allows the operator to retry after
    correcting the provider configuration.

---

### User Story 4 - Review matches and track applications in the dashboard (Priority: P4)

The operator reviews "New Jobs" as a card grid sorted by match score. Each
card shows the job title, company, a brief summary, the N/M requirement match
ratio, and the score badge. Clicking a card opens a side drawer with the full
description, structured requirements, matched and missing skills, the original
posting link, and the "I Applied" action. Applying moves the job to
"My Applications" with an applied timestamp and removes it from the active feed.

**Why this priority**: This is the visible user outcome of the pipeline and the
basis for application tracking. It can be tested with a fixed dataset and
evaluation results.

**Independent Test**: Load a fixed dataset with approved, rejected, applied, and
ignored jobs; verify the card contents and ordering, the drawer details, the
apply and ignore actions, the tab contents, and the applied-job history.

**Acceptance Scenarios**:

1. **Given** approved jobs with different scores, **When** the operator opens
   "New Jobs", **Then** the jobs appear as cards sorted by score descending,
   each showing title, company, summary, N/M ratio, and score badge.
2. **Given** a job card, **When** the operator clicks it, **Then** a side drawer
   opens on the right with the full description, structured requirements,
   matched and missing skills, the original posting link, and the available
   actions.
3. **Given** a job card or open drawer, **When** the operator clicks
   "I Applied", **Then** the job state becomes APPLIED, the applied timestamp
   is stored locally, the job disappears from "New Jobs", and it appears in
   "My Applications".
4. **Given** a job marked APPLIED, **When** a later search runs, **Then** the
   job is excluded from "New Jobs" and its stored application state remains
   intact.
5. **Given** one or more APPLIED jobs, **When** the operator opens
   "My Applications", **Then** the jobs can be queried, filtered, sorted, and
   opened for detail, and each shows its applied timestamp, score, and N/M ratio.
6. **Given** a job in "New Jobs", **When** the operator chooses "Ignore",
   **Then** the job state becomes IGNORED, it disappears from "New Jobs", and
   it does not appear in "My Applications" unless the operator explicitly
   includes ignored jobs in a filter.
7. **Given** the operator switches between "New Jobs" and
   "My Applications", **Then** the active tab and its filters are preserved
   for the session and the other tab's content is not mixed in.
8. **Given** a completed search and evaluation run, **When** the operator opens
   the audit, **Then** total ingested, duplicates merged, hard-filter rejections
   by reason, final matches surfaced, provider/model used, and source failures
   reconcile with the underlying records.
9. **Given** a search or evaluation produces no approved jobs, **When** the
   operator opens "New Jobs", **Then** the interface shows a clear empty
   state and still exposes the audit and any source-failure information.

---

### User Story 5 - Switch the language-model provider through local configuration (Priority: P5)

The operator configures the active language-model provider, credentials,
connection settings, and model locally. Resume parsing and semantic evaluation
route to the active provider. Changing the configuration changes the provider
used by subsequent operations without modifying application code.

**Why this priority**: Provider choice affects cost, availability, and output
quality. The operator MUST be able to switch providers without a code change or
data migration.

**Independent Test**: Configure DeepSeek, run resume parsing and a match
evaluation, then switch the configuration to Gemini and rerun. Verify that the
requests route to the configured provider with its credentials and model, and
that no code change is required.

**Acceptance Scenarios**:

1. **Given** the active provider is configured as DeepSeek, **When** resume
   parsing or semantic evaluation runs, **Then** the request routes to DeepSeek
   using its configured credentials and model.
2. **Given** the active provider is changed from DeepSeek to Gemini, **When**
   the next parsing or evaluation runs, **Then** the request routes to Gemini
   without modifying application code or losing existing profile or job data.
3. **Given** the active provider configuration is missing or invalid, **When** an
   operation starts, **Then** the system reports a clear configuration error,
   does not send the request to the wrong provider, and preserves the last valid
   data.
4. **Given** the active provider is unavailable or returns malformed output,
   **When** an operation runs, **Then** the system reports a provider error,
   does not assign a partial score or profile, and allows the operator to switch
   providers and retry.
5. **Given** any parsing or evaluation operation, **When** it completes,
   **Then** the provider and model used are recorded with the result.
6. **Given** provider credentials, **When** logs, reports, or audit output are
   inspected, **Then** the credentials are not present.

---

### Edge Cases

- The resume PDF is password-protected, encrypted, corrupt, image-only,
  unsupported, or larger than the supported input limit.
- The same resume is uploaded twice, or a newer resume version is uploaded while
  a recalculation is already running.
- The last stored resume is deleted while a search or evaluation is running.
- Two resumes disagree about the candidate's current title, years of experience,
  or seniority.
- The selected timeframe has no upper or lower bound, or the posting age falls
  exactly on a boundary.
- A job source returns zero results, malformed records, missing fields,
  duplicates, stale postings, or a rate-limit response.
- All sources fail, one source fails repeatedly, or one source is much slower
  than the others.
- The same opportunity appears with different company names, titles, URLs, or
  locations.
- A job has no posted time, no location, no modality, no seniority, or no
  extractable requirements.
- A job passes every hard filter but the score is exactly at the approval
  threshold, or multiple jobs share the same score.
- The active language-model provider times out, is rate-limited, returns
  malformed output, or changes between operations.
- The operator changes the profile, provider, model, weights, or threshold while
  jobs are being evaluated.
- A job is applied or ignored while its detail drawer is open or while a search
  is running.
- A job already marked APPLIED is republished or updated by a source.
- The dashboard receives a large result set, a job has no summary, or the
  original posting URL is broken or no longer resolves.
- "My Applications" is opened while a search is still in progress or after a
  partial source failure.
- Local storage is unavailable, full, or corrupted.
- The audit is opened before a run finishes or after an evaluation error.

## Requirements *(mandatory)*

### Functional Requirements

#### Candidate Profile & Resume Management (FR-PRF)

- **FR-PRF-001 (PDF Ingestion)**: The system MUST accept PDF resume files as
  input for creating or updating a CandidateProfile. It MUST reject non-PDF or
  unreadable files with a specific error and leave the existing profile
  unchanged.
- **FR-PRF-002 (Text Extraction and Structuring)**: The system MUST extract text
  and structural metadata from an ingested PDF and map the content into a
  structured CandidateProfile covering technical competencies, domain
  experience, work history, education, and certifications.
- **FR-PRF-003 (Multi-Resume Merging)**: When multiple resumes are provided, the
  system MUST merge their attributes into one master profile without duplicate
  entries. List-valued attributes MUST be unioned; conflicting scalar values
  MUST be flagged for operator review rather than silently overwritten.
- **FR-PRF-004 (Profile Verification and Override)**: The system MUST present
  the extracted profile in an editable state so the operator can review, correct,
  or add constraints, including dealbreakers, target modalities, salary
  thresholds, and work-authorization or visa needs, before running a search.
- **FR-PRF-005 (Resume Persistence and Management)**: The system MUST store
  uploaded resume files and their metadata locally, list stored resumes, and
  allow the operator to upload or remove them. Stored resumes MUST survive an
  application restart.
- **FR-PRF-006 (Automatic Profile Recalculation)**: Adding or deleting a resume
  MUST automatically trigger a background recalculation of the consolidated
  CandidateProfile without requiring the operator to start a search. Deleting a
  resume MUST also remove it from the profile's source set, so the profile never
  references a deleted file.
- **FR-PRF-007 (Recalculation Safety)**: If recalculation fails, the system MUST
  report the failure and retain the last valid CandidateProfile. It MUST NOT
  persist a partial or unverified profile.
- **FR-PRF-008 (Source Traceability)**: The system MUST record which stored
  resume or resumes contributed to each extracted competency.
- **FR-PRF-009 (Duplicate Resume Handling)**: Re-uploading the same resume
  content MUST NOT create a duplicate source document or corrupt the profile;
  the operator MUST be informed of the duplicate.
- **FR-PRF-010 (Profile Versioning)**: The system MUST record which profile
  version was used for each evaluation so results remain traceable after later
  profile edits. Earlier application snapshots MUST NOT be altered by later
  profile changes.
#### Job Ingestion & Search (FR-ING)

- **FR-ING-001 (Search Controls)**: The system MUST accept search filters for
  target job titles, primary language or stack, posting timeframe (including at
  least the last 24 hours and the last 7 days), seniority level (Trainee,
  Intern/Estágio, Junior, Mid, Senior), and work modality (Remote, Hybrid,
  Onsite). Timeframes MUST use the posting's published time; a posting with no
  published time MUST be labeled Unknown and MUST NOT be silently included in a
  strict timeframe.
- **FR-ING-002 (Independent, Concurrent Ingestion)**: The system MUST ingest
  jobs from multiple registered sources independently and concurrently.
- **FR-ING-003 (Fault Isolation)**: A failure in one source MUST NOT halt the
  search or prevent processing of data from remaining operational sources.
- **FR-ING-004 (Content Cleaning)**: The system MUST strip markup tags, control
  characters, and non-standard whitespace from raw job descriptions before
  downstream use.
- **FR-ING-005 (Rate Control)**: Every external ingestion task MUST enforce
  per-source rate limiting, throttling, and bounded backoff.
- **FR-ING-006 (Source Lifecycle)**: The system MUST allow sources to be
  registered, enabled, disabled, or removed without changing matching or
  filtering logic.
- **FR-ING-007 (Source Lineage)**: The system MUST record the source name,
  external identifier, URL, and ingestion time for every job.
- **FR-ING-008 (Empty and Failed Runs)**: The system MUST report zero-result and
  all-sources-failed outcomes clearly and preserve an execution record without
  crashing.
- **FR-ING-009 (Terms and Access)**: Ingestion MUST NOT intentionally bypass
  source rate limits, access controls, or terms of service.
- **FR-ING-010 (Search Configuration Versioning)**: The system MUST record which
  search configuration was used for each run.
- **FR-ING-011 (Source Boundary)**: Registered sources and their integrations
  MUST NOT contain matching, scoring, or filtering rules. The Match Engine MUST
  consume normalized JobPosting records and MUST NOT depend on source-specific
  integrations or formats.

#### Deduplication & Normalization (FR-DED)

- **FR-DED-001 (Canonical Identity)**: The system MUST derive a stable canonical
  identity for each job from normalized company name, normalized job title, and
  primary posting URL.
- **FR-DED-002 (Duplicate Merge)**: Duplicate jobs detected across sources MUST
  be merged into one canonical JobPosting that retains all source lineage.
- **FR-DED-003 (Pre-Evaluation Deduplication)**: Deduplication MUST occur before
  match evaluation.
- **FR-DED-004 (Controlled Normalization)**: The system MUST normalize work
  modality, seniority, and location into controlled values and MUST represent
  missing or unrecognized values as Unknown rather than guessing.
- **FR-DED-005 (Repeat-Safe Ingestion)**: Re-ingesting the same opportunity MUST
  NOT create duplicate canonical records, duplicate evaluation work, or
  duplicate lifecycle entries.
- **FR-DED-006 (Merge Completeness)**: When duplicates disagree, the canonical
  record MUST retain the most complete available descriptive content and MUST
  record conflicting values for audit.
- **FR-DED-007 (Identity Fallback)**: If the inputs required for canonical
  identity are missing, the system MUST use a documented fallback and flag the
  record for operator review.

#### Match & Filtering (FR-MAT)

- **FR-MAT-001 (Profile Comparison)**: The Match Engine MUST compare each job
  against the active CandidateProfile, including technical competencies, years
  of experience, target roles, preferred modalities, location constraints, and
  non-negotiable dealbreakers.
- **FR-MAT-002 (Hard Filtering)**: The system MUST record a match evaluation
  outcome of REJECTED when any hard-filter condition is triggered:
  - Seniority is outside the candidate's configured target bounds.
  - Work modality or location violates a non-negotiable constraint.
  - A core domain competency marked as a mandatory dealbreaker is absent.
- **FR-MAT-003 (Scoring and Item Matching)**: For jobs that pass hard filters,
  the system MUST compute an integer Match Score from 0 to 100 using four
  weighted dimensions: core technical skill overlap (40%), domain and
  architectural alignment (25%), seniority and role fit (20%), and secondary
  competencies (15%). The four weights MUST sum to 100%, and every dimension
  contribution MUST be reflected in the final score. The system MUST also
  compute an explicit count of matched versus total extracted job requirements
  (N matched out of M total). The matched count plus the missing count MUST equal
  the total extracted requirement count, each requirement MUST be counted once,
  and when no requirements are extracted the system MUST represent the ratio as
  0/0 or an explicit "no requirements" state without dividing by zero or
  implying a perfect match.
- **FR-MAT-004 (Structured Evaluation)**: The system MUST produce a structured
  evaluation for every job containing the match score, matched requirement
  count, total requirement count, matched keywords, missing keywords, hard-filter
  rejection reasons, and the four dimension scores.
- **FR-MAT-005 (Hard-Filter Precedence)**: Hard filters MUST run before scoring
  for approval. A job that triggers any hard filter MUST NOT be approved
  regardless of its score.
- **FR-MAT-007 (Approval Threshold)**: The active approval threshold MUST be
  explicit, versioned, and recorded with each evaluation. A job that passes hard
  filters but falls below the threshold MUST NOT be approved. A score exactly
  equal to the threshold MUST be treated as meeting it.
- **FR-MAT-008 (Unknown Criteria)**: An unknown value for a criterion the
  profile marks as mandatory MUST NOT be treated as a match. The gap MUST be
  recorded, and the job MUST remain out of the approved set.
- **FR-MAT-009 (Reproducibility)**: The same profile version, job, provider,
  model, and active rules MUST produce the same evaluation outcome, score, N/M
  count, and breakdown.
- **FR-MAT-010 (Stale Results)**: When the profile, provider, model, weights,
  threshold, or dealbreakers change, prior evaluations MUST be marked stale and
  MUST NOT be presented as current until re-evaluated.
- **FR-MAT-011 (Consistent Tie-Breaking)**: Jobs with equal scores MUST be
  ordered by a documented deterministic tie-breaker.
- **FR-MAT-012 (Requirement Extraction)**: The system MUST extract seniority,
  required skills, desirable skills, work modality, and location requirements
  from each clean job description before comparison.
- **FR-MAT-013 (Exact and Semantic Matching)**: The system MUST consider both
  exact and semantically related competency matches and MUST record which
  matched terms were exact versus inferred.
#### Job Lifecycle & Application Tracking (FR-APP)

- **FR-APP-001 (Lifecycle States)**: The evaluation outcome (APPROVED, REJECTED,
  or ERROR) MUST be distinct from the application lifecycle state. Each job that
  receives an APPROVED evaluation and enters the application workflow MUST
  maintain a lifecycle state of DISCOVERED, APPLIED, or IGNORED. A REJECTED or
  ERROR job MUST NOT enter the lifecycle or appear in "New Jobs".
- **FR-APP-002 (Application Action)**: Triggering "I Applied" MUST set the job
  state to APPLIED, store the application timestamp, and persist the updated
  record locally.
- **FR-APP-003 (Applied Exclusion)**: Jobs marked APPLIED MUST be excluded from
  "New Jobs" and from subsequent search results in the active feed.
- **FR-APP-004 (Applied Jobs View)**: The system MUST provide a
  "My Applications" view for querying, filtering, sorting, and managing all
  APPLIED jobs.
- **FR-APP-005 (Ignore Action)**: The system MUST provide an "Ignore" action
  that sets the job state to IGNORED and removes it from "New Jobs". IGNORED
  jobs MUST NOT appear in either tab by default, but the operator MUST be able to
  include them through a filter and restore them to DISCOVERED.
- **FR-APP-007 (State Preservation)**: APPLIED and IGNORED states MUST survive
  application restarts, new searches, "Refresh Search" refreshes, and later
  source updates.
- **FR-APP-008 (Application Snapshot)**: The system MUST preserve the job,
  profile, and evaluation snapshot used when the operator marked a job APPLIED.
  Later profile or rule changes MUST NOT alter that historical record.
- **FR-APP-009 (Lifecycle History)**: The system MUST persist an append-only
  lifecycle history recording state transitions with timestamps and reasons.
  History MUST NOT be silently overwritten.
- **FR-APP-010 (Applied View Detail)**: "My Applications" MUST show the
  applied timestamp, match score, N/M ratio, company, title, and a link to the
  original posting for each applied job.
- **FR-APP-011 (No Automatic Submission)**: "I Applied" is an application
  tracking action only. The system MUST NOT submit an application automatically.
- **FR-APP-013 (Single Active Profile)**: The system MUST maintain one active
  CandidateProfile. A profile identifier exists for versioning and audit, not for
  switching between multiple profiles.
- **FR-APP-014 (Last Resume Deletion)**: Deleting the last stored resume MUST
  preserve the profile, mark it "no resumes", and block new searches until a new
  resume is uploaded.

#### Desktop Interface (FR-UI)

- **FR-UI-001 (Top Control Bar)**: The top bar MUST include a search refresh
  action, controls for posting timeframe, seniority, and primary language or
  stack, a modality selector, and controls to upload, list, and delete stored
  resumes.
- **FR-UI-002 (Dashboard Card Grid)**: "New Jobs" MUST render jobs as cards
  showing the job title, company name, a brief summary, the N/M requirement
  match ratio, and the match score badge, sorted by score descending.
- **FR-UI-003 (Side Drawer)**: Selecting a card MUST open a side drawer on the
  right showing the full job description, structured requirements, matched and
  missing skills, a link to the original posting, the "I Applied" action, and
  the "Ignore" action.
- **FR-UI-004 (Navigation Tabs)**: The interface MUST provide tab navigation
  across "New Jobs", "My Applications", "My Profile", and "Analytics".
- **FR-UI-005 (New Jobs Content)**: "New Jobs" MUST contain only jobs in
  the DISCOVERED state with an APPROVED evaluation. It MUST exclude APPLIED,
  IGNORED, REJECTED, and ERROR jobs.
- **FR-UI-006 (My Applications Content)**: "My Applications" MUST contain
  only APPLIED jobs and MUST support query, filtering, sorting, and detail
  inspection.
- **FR-UI-007 (External Link)**: The original posting link MUST open the source
  URL in the system browser. A broken or unavailable URL MUST be reported without
  losing the stored job data.
- **FR-UI-008 (Interface States)**: The interface MUST provide clear loading,
  empty, partial-source-failure, evaluation-error, and zero-approved-results
  states.
- **FR-UI-009 (Fixed Labels)**: The interface MUST display the labels "New
  Jobs", "My Applications", "My Profile", "I Applied", and "Refresh Search"
  exactly as written.
- **FR-UI-010 (Card-Drawer Consistency)**: The detail drawer MUST show the same
  job, score, and N/M ratio as its card, and MUST update immediately after the
  operator applies or ignores the job.
- **FR-UI-011 (Search Validation)**: Invalid or contradictory search filters MUST
  produce a clear validation message and MUST NOT start a run.
- **FR-UI-012 (Large Result Sets)**: The dashboard MUST provide a way to navigate
  large result sets without silently truncating jobs.
- **FR-UI-013 (My Profile)**: The interface MUST provide a "My Profile" tab with
  editable acceptable-technology tags and accepted on-site/hybrid cities. The
  manual technology list MUST be merged into the profile's core competencies and
  used in both the hard filter and the core-skills score. Changes apply on the
  next search, with no automatic historical re-evaluation.
- **FR-UI-014 (Below Threshold)**: "New Jobs" MUST provide a toggle/filter that
  reveals PENDING jobs below the approval threshold so the operator can review
  them. Manual approval MUST set the evaluation to APPROVED with
  `approval_source = manual` and preserve the original score/breakdown.

#### LLM Provider Configuration (FR-LLM)

- **FR-LLM-001 (Supported Providers)**: The system MUST support Gemini, DeepSeek,
  and OpenRouter as selectable language-model providers.
- **FR-LLM-002 (Local Configuration)**: The active provider, credentials,
  connection settings, and model identifier MUST be configurable locally without
  changing or rebuilding application source code.
- **FR-LLM-003 (Dynamic Routing)**: Resume parsing and semantic match evaluation
  MUST route to the active provider using its configured credentials and model.
- **FR-LLM-004 (Provider Switch)**: Changing the active provider MUST apply to
  subsequent parsing and evaluation operations without code changes and without
  losing existing profile, job, or application data.
- **FR-LLM-005 (Configuration Validation)**: Missing or invalid provider
  configuration MUST produce a clear error before a request is sent. The system
  MUST NOT send a request to an unintended provider.
- **FR-LLM-006 (Provider Failure)**: A provider timeout, rate limit, malformed
  response, or unavailable service MUST produce a clear error and MUST NOT create
  a partial score or profile. The system MUST assign no score and keep the job
  out of the approved set until a successful retry. The operator MUST be able to
  retry after correcting the configuration.
- **FR-LLM-007 (Provider Traceability)**: The provider and model used MUST be
  recorded with each parsing or evaluation result for audit and reproducibility.
- **FR-LLM-010 (Provider Isolation)**: Resume parsing and match workflows MUST
  NOT depend on provider-specific response formats or provider-specific business
  rules.
- **FR-LLM-011 (No Silent Fallback)**: If the active provider fails, the system
  MUST NOT silently switch to a different provider. A fallback is permitted only
  when explicitly configured and disclosed to the operator.

#### Output & Reporting (FR-REP)

- **FR-REP-001 (Ranked Results)**: The system MUST present ranked job
  opportunities sorted by Match Score descending.
- **FR-REP-002 (Audit Log)**: The system MUST maintain audit records containing
  total jobs ingested, duplicates deduplicated, hard-filter rejections with
  reasons, final matches surfaced, source failures, and the provider and model
  used.
- **FR-REP-003 (Audit Reconciliation)**: Audit totals MUST reconcile exactly with
  the underlying records for the same run.
- **FR-REP-005 (Audit Persistence)**: Audit records MUST survive application
  restarts and MUST be associated with a unique run identifier, start time, and
  end time.
- **FR-REP-008 (Zero Results)**: When no jobs are approved, the system MUST show a
  clear zero-result state and still provide the audit and source-failure
  information.

#### Security & Privacy (FR-SEC)

- **FR-SEC-001 (Local Storage)**: Candidate profiles, stored resumes,
  credentials, application history, and audit records MUST be stored locally.
  Application-level encryption at rest is not required; protection relies on
  OS-level disk encryption and restricted filesystem permissions.
- **FR-SEC-002 (Authorized Exposure)**: Personal information and authentication
  material MUST NOT be exposed to any third party the operator has not
  authorized.
- **FR-SEC-003 (Secret Handling)**: Credentials and session material MUST NOT be
  written to logs, telemetry, error reports, or build artifacts.
- **FR-SEC-004 (Revocation and Deletion)**: Removing a source or account MUST
  revoke and delete its associated credentials and session material. Deleting a
  resume MUST delete its stored file and metadata.
- **FR-SEC-005 (Minimum Necessary Access)**: Access to stored profile, resume,
  credential, and application data MUST be limited to the components that require
  it for an explicit operator-initiated operation.
- **FR-SEC-006 (Raw Description Handling)**: Raw job-description text MUST NOT be
  persisted in SQLite, written to logs, reports, or audit output, and MUST be
  discarded when no longer needed for the active processing run.
- **FR-SEC-007 (Provider Data Minimization)**: Only the minimum data necessary for
  parsing or evaluation may be sent to the active provider; credentials and
  unrelated personal data MUST NOT be included.
- **FR-SEC-008 (Local-Only History)**: Profile, resume, application, and audit
  history MUST remain local unless the operator explicitly authorizes an
  external transfer.

### Key Entities *(include if feature involves data)*

- **Candidate Profile**: The consolidated, operator-verified representation of
  the candidate. Key attributes: stable profile identifier, source-document
  references, target roles, target seniority range, preferred work modalities,
  location preferences, work authorization and visa needs, salary threshold,
  core competencies, secondary competencies, domain experience, dealbreakers,
  and profile version.
- **Resume Document**: A stored resume used to build the profile. Key attributes:
  stable file identifier, file name, content identity, upload time, parse status,
  and the profile attributes it contributed to.
- **Search Configuration**: A named set of discovery criteria. Key attributes:
  target titles, primary language or stack, posting timeframe, seniority levels,
  modalities, selected sources, and last-run reference.
- **Ingestion Source**: A registered job provider. Key attributes: source name,
  source type, enabled state, rate-limit policy, credential reference (not the
  secret itself), and last-run status.
- **Job Posting**: A normalized opportunity. Key attributes: stable job
  identifier, canonical identity, title, company, location, work modality, URL,
  posted time, raw and cleaned description, extracted seniority and skills, and
  source lineage.
- **Source Lineage**: The record of where a job came from. Key attributes:
  source name, external identifier, URL, and ingestion time.
- **Match Evaluation**: The decision record for a job. Key attributes: outcome
  (APPROVED, REJECTED, ERROR), match score, matched count, total requirement
  count, four dimension scores, matched and missing keywords, hard-filter
  rejection reasons, active rules and weights version, threshold, profile
  version, provider and model, and evaluation time.
- **Job Application**: The application lifecycle record for a job. Key
  attributes: job reference, profile reference, lifecycle state (DISCOVERED,
  APPLIED, IGNORED), applied timestamp, evaluation snapshot at application time,
  and last updated time.
- **Lifecycle Event**: A state transition for an opportunity. Key attributes:
  opportunity reference, state, timestamp, and reason.
- **Execution Audit**: The record of one search or evaluation run. Key
  attributes: run identifier, start and end time, total ingested, duplicates
  merged, rejections by reason, final matches surfaced, source failures,
  provider and model used, and the search configuration used.
- **LLM Provider Configuration**: The active language-model connection settings.
  Key attributes: provider name, credential reference (not the secret itself),
  connection settings, model identifier, and active state.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: An operator can upload a text-based PDF resume of up to 10 pages
  and see the profile recalculation complete in under 2 minutes; the resume and
  profile persist across an application restart.
- **SC-002**: Deleting a stored resume triggers recalculation within 2 minutes;
  if recalculation fails, the last valid profile remains available and the
  failure is visible to the operator.
- **SC-003**: With three sources configured and one failing, 100% of searches
  complete with results from the remaining sources, and 100% of returned jobs
  satisfy the active filters.
- **SC-004**: 100% of duplicate jobs across sources consolidate into one canonical
  record with complete source lineage, and repeating the same search creates no
  duplicate records.
- **SC-005**: 100% of jobs that trigger a hard filter receive a REJECTED
  evaluation with a specific reason and are excluded from "New Jobs".
- **SC-006**: For every approved job, the score is an integer from 0 to 100, the
  four dimension weights sum to 100%, the N/M counts reconcile to the extracted
  requirements, and a breakdown with matched and missing items is present.
- **SC-007**: 100% of "New Jobs" cards show title, company, summary, N/M
  ratio, and score badge, sorted by score descending.
- **SC-008**: Selecting any job card opens a detail drawer with the full
  description, structured requirements, matched and missing skills, the original
  posting link, and the available actions in 100% of cases.
- **SC-009**: Marking "I Applied" changes the job to APPLIED, records an applied
  timestamp, removes it from "New Jobs", and adds it to
  "My Applications" in 100% of cases.
- **SC-010**: "My Applications" supports query, filtering, sorting, and detail
  inspection, and shows the applied timestamp, score, and N/M ratio for 100% of
  applied jobs.
- **SC-011**: Changing the configured language-model provider changes routing for
  subsequent parsing and evaluation without application code changes in 100% of
  test runs.
- **SC-012**: A provider failure produces a clear error and no partial score; after
  switching to a working provider, retry succeeds in 100% of test runs.
- **SC-013**: Audit totals reconcile exactly with the underlying records for
  total ingested, duplicates, rejections by reason, final matches, and source
  failures in 100% of runs.
- **SC-014**: No report, log, or audit output contains credentials or raw
  job-description text in 100% of tested runs.
- **SC-015**: An operator can go from a completed search to marking a job applied
  in under 2 minutes using the dashboard and detail drawer.

## Assumptions

- The system is a single-operator local desktop application. Multi-user access,
  shared accounts, and team workspaces are out of scope for v1.
- Local persistent storage is available; stored resumes, profiles, applications,
  and audit records survive an application restart. Protection at rest relies on
  OS-level disk encryption and filesystem permissions, not application-level
  encryption.
- Resumes are text-based PDFs. Password-protected, encrypted, corrupt, or
  image-only PDFs are rejected with a clear error; OCR is out of scope for v1.
- Multiple resumes may describe the same candidate in different versions or
  languages. List-valued attributes are merged; conflicting scalar values are
  flagged for review, with the most recently parsed non-conflicting value used
  until resolved.
- The parser may infer adjacent competencies from resume evidence. Inferred
  items MUST be marked as inferred, traceable to source evidence, and editable by
  the operator.
- Salary thresholds and dealbreakers are operator-provided preferences.
  Work authorization and visa sponsorship are out of scope for the v1 hard
  filters; the location filter already restricts the search to Brazil plus Porto
  Alegre/Curitiba.
- The default approval threshold is 70 on the 0-100 scale and is
  operator-configurable. Jobs below the threshold remain out of the approved set.
- The v1 scoring weights are fixed at 40% core technical, 25%
  domain/architectural, 20% seniority/role, and 15% secondary competencies. The
  active weights version is recorded with each evaluation.
- Seniority values are Trainee, Intern/Estágio, Junior, Mid, and Senior. "Intern"
  and "Estágio" are aliases for the same level.
- Work modalities are Remote, Hybrid, and Onsite. A job with an unrecognized or
  missing modality is labeled Unknown.
- A job's timeframe is based on its published time. Missing published time is
  labeled Unknown and is not silently included in a strict timeframe.
- Evaluation outcome and application lifecycle are separate. A job with a
  REJECTED or ERROR evaluation never enters the DISCOVERED/APPLIED/IGNORED
  lifecycle and never appears in "New Jobs". The "Ignore" action is the
  operator-initiated way to enter the IGNORED state.
- IGNORED jobs are excluded from both tabs by default. The operator can include
  them through a filter and restore them to DISCOVERED.
- Applied jobs preserve the evaluation snapshot from the moment of application.
  Later profile, provider, or rule changes do not rewrite that historical record.
- The system supports Gemini, DeepSeek, and OpenRouter as language-model
  providers. The active provider, credentials, connection settings, and model are
  loaded from local configuration; the exact configuration mechanism is an
  implementation detail.
- A provider switch applies to subsequent operations. An operation already in
  progress may finish with the provider it started with. A configuration change
  may require an application restart, but MUST NOT require a code change.
- The system does not silently fall back to another provider. A fallback is
  permitted only when explicitly configured and disclosed to the operator.
- "I Applied" is a tracking action. The system does not submit applications
  automatically.
- The interface labels "New Jobs", "My Applications", "I Applied", and
  "Refresh Search" are fixed. Broader localization is out of scope for v1.
- Dependency: the operator has network access to configured job sources and
  language-model providers, valid provider credentials, and source terms that
  permit the configured access.
- Dependency: local storage is available and has sufficient capacity for stored
  resumes, job records, evaluations, and audit history.
- This specification extends `specs/001-career-job-sourcing/spec.md`.
  Requirements restated here take precedence for the desktop workflow; core
  requirements not restated remain in force.
- Out of scope: OCR for image-only PDFs, multi-user access, web or mobile
  clients, automatic application submission, interview tracking, offer
  management, salary negotiation, and employer communication.
