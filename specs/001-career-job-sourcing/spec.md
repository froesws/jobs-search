# Feature Specification: Career Intelligence & Job Sourcing System

**Feature Branch**: `N/A (no branch hook configured)`

**Created**: 2026-11-10

**Status**: Draft

**Input**: User description: "Automated career intelligence and job sourcing
pipeline covering PDF resume ingestion, multi-source job discovery,
deduplication, deterministic and semantic compatibility scoring, and ranked
match reporting."

This specification defines the Career Intelligence & Job Sourcing System. The
system ingests candidate credentials from PDF resumes, discovers jobs across
multiple registered sources, deduplicates postings, evaluates compatibility with
deterministic hard filters and weighted semantic scoring, and presents ranked,
explainable opportunities. The primary actor is the System Operator, who
configures searches, verifies the profile, sets constraints and dealbreakers,
and reviews results.

## V1 Scope Amendments

This section supersedes conflicting requirements below. The MVP is a new
Linux-only desktop app in `jobs-search`; `contra_maquina` remains a separate
product.

- **Stack**: Tauri 2.12.5, React 19.3.1, TypeScript 7.0.2, Vite 8.3.6, Rust
  1.90+, MUI 9.5 + Emotion. No Tailwind, TanStack Query, or Zustand.
- **Profile**: one active CandidateProfile; PDF text is extracted with
  `pdf-extract`; `description_raw` is never persisted. Only the clean
  description and extracted metadata are stored.
- **Privacy**: constitution v2.0.0; local-only data; no app-level encryption at
  rest. OS disk encryption and filesystem permissions are the baseline. API keys
  live in a local `.env` file.
- **Ingestion**: hybrid MCP JobSpy plus a Rust-native public-API adapter
  (Remotive/RemoteOK provisional). Sources are configured in `sources.yaml`;
  lineage uses `job_sources` and `job_posting_sources`. Rate limits apply both
  centrally and inside each adapter.
- **Dedup**: canonical identity uses company + title + location, with URL
  fallback. All distinct URLs are preserved when a job passes the hard filters.
- **Matching**: location is strict (remote Brazil or on-site/hybrid in Porto
  Alegre or Curitiba); seniority accepts Trainee, Junior, and Pleno, including
  Pleno/Senior ranges. Core technologies come from the merged profile/manual
  list; Trainee/Intern accept any technology. Work authorization/visa and salary
  are not hard filters. The approval threshold is 70 and configurable.
- **Evaluation**: the LLM extracts requirements and per-dimension evidence; the
  app computes the 40/25/20/15 score and N/M count. At most 100 jobs are
  evaluated per run by default, 2 evaluations run in parallel, and fallback
  order is Gemini → OpenRouter free models → DeepSeek for transient failures
  only.
- **Status**: `evaluation_status` is PENDING/APPROVED/REJECTED/ERROR and
  `lifecycle_state` is DISCOVERED/APPLIED/IGNORED, with append-only history.
  Manual approval sets `approval_source = manual`.
- **Analytics and telemetry** are defined in `003` and `004`; Analytics is in
  the MVP and telemetry is local code logging only.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Build a verified candidate profile from PDF resumes (Priority: P1)

The System Operator submits one or more PDF resumes. The system extracts text
and structure, maps technical competencies, domain experience, work history,
education, and certifications into a single CandidateProfile, and presents the
result for review. The operator can correct fields and add constraints that
cannot be extracted, including work authorization, visa needs, salary
thresholds, target roles, target seniority, preferred modalities, and
dealbreakers.

**Why this priority**: Every downstream decision depends on a trustworthy
profile. This slice alone delivers a usable, editable candidate profile and is
the foundation for all matching.

**Independent Test**: Submit a text-based PDF resume and verify that a valid
CandidateProfile appears with source-document lineage and editable fields.
Submit two resume versions and verify the merge behavior. Submit a corrupt or
image-only PDF and verify a clear failure without a partial profile.

**Acceptance Scenarios**:

1. **Given** a text-based PDF resume containing Linux kernel work, Rust
   dataframes, C# migration tools, and an EF SET C1 certification, **When** the
   operator submits it, **Then** the system extracts and maps Rust, Linux, C#,
   and Embedded C into core competencies, maps EF SET C1 into secondary
   competencies, records the source document, and produces a valid
   CandidateProfile ready for review.
2. **Given** two resume versions with overlapping competencies and conflicting
   scalar values, **When** the operator submits both, **Then** list-valued
   fields are merged without duplicates, conflicting scalar values are flagged
   for review, and no value is silently discarded.
3. **Given** a parsed profile, **When** the operator edits a competency, adds a
   dealbreaker, sets a salary threshold, or declares work authorization or visa
   needs, **Then** the updated profile is saved and used by the next matching
   run.
4. **Given** a profile missing a field required for hard filtering, **When** the
   operator starts matching, **Then** the system flags the gap and does not
   silently treat the missing value as a match.
5. **Given** an encrypted, corrupt, unsupported, or image-only PDF with no
   extractable text, **When** the operator submits it, **Then** the system
   reports a specific failure and does not create or overwrite a profile with
   partial data.

---

### User Story 2 - Discover and consolidate jobs from multiple sources (Priority: P2)

The System Operator defines one or more search configurations containing target
job titles, technical skills, geographic constraints, and work modalities,
selects registered job sources, and starts a discovery run. The system collects
postings independently and concurrently, cleans and normalizes them,
deduplicates the payloads, and preserves source lineage. A failure in one source
does not stop the others.

**Why this priority**: Discovery creates the opportunity pool. It can be tested
with a fixed search configuration and delivers a normalized, deduplicated
dataset even before scoring.

**Independent Test**: Register three sources, make one fail, run a discovery
with a fixed query, and verify that postings from the remaining sources are
normalized, deduplicated, and traceable to their sources; verify that the
failure is recorded.

**Acceptance Scenarios**:

1. **Given** three operational sources and a search configuration, **When** the
   operator starts discovery, **Then** the system ingests postings from all
   sources, returns normalized JobPosting records with source lineage, and keeps
   matching and filtering rules out of the source integrations.
2. **Given** one source that times out or returns a server error, **When**
   discovery runs, **Then** the system records the failure for that source,
   completes using the remaining sources, and does not halt the run.
3. **Given** the same opportunity published by two sources, **When** both are
   ingested, **Then** the system merges them into one canonical JobPosting with
   both source identifiers, a single stable job identity, and any conflicting
   values retained for audit.
4. **Given** a posting containing markup, control characters, and irregular
   whitespace, **When** it is ingested, **Then** the clean description used
   downstream contains none of those artifacts while the original content
   remains available only for the active processing run.
5. **Given** a source that enforces a request limit, **When** discovery runs,
   **Then** the system throttles requests, applies bounded backoff, does not
   bypass the source's access controls or terms, and records exhausted retries
   without retrying indefinitely.
6. **Given** a search that returns no jobs or a run where every source fails,
   **When** discovery completes, **Then** the system reports the empty or failed
   outcome clearly and preserves the execution record.
7. **Given** a registered source is removed, **When** removal completes,
   **Then** its credentials and session material are deleted and it no longer
   participates in discovery.
8. **Given** a posting missing the company, title, location, and URL needed for
   its primary identity, **When** it is ingested, **Then** the system uses the
   documented fallback identity and flags the record for operator review.

---

### User Story 3 - Evaluate and rank jobs with explainable scores (Priority: P3)

The Match Engine extracts requirements from each deduplicated posting and
compares them with the verified CandidateProfile. It applies deterministic hard
filters first, then computes a 0-100 match score from four weighted dimensions
and produces a criterion-level breakdown. Postings that trigger a hard filter
are rejected immediately; postings that pass but fall below the approval
threshold are excluded from the approved list.

**Why this priority**: This is the core decision capability that turns a raw job
pool into a prioritized set of opportunities. It depends on P1 and P2 but can be
tested independently with fixed profile and posting fixtures.

**Independent Test**: Provide a fixed profile and a set of postings covering high
match, seniority mismatch, modality mismatch, missing mandatory competency,
missing mandatory technology, and low score. Verify statuses, scores,
weighted breakdowns, reasons, and reproducibility.

**Acceptance Scenarios**:

1. **Given** a "Senior Rust / Systems Engineer" posting requiring Rust, Linux,
   and gRPC, offering remote work in Brazil, and a candidate profile targeting
   Mid/Senior engineering with Rust and Linux, **When** evaluated, **Then** the
   job is APPROVED with a score above 85, a technical-overlap breakdown that
   identifies exact and inferred matches, no hard-filter flags, and a result
   that is independent of which source supplied the posting.
2. **Given** a posting for a 10+ year Vice President of Sales role and a
   candidate profile targeting Individual Contributor engineering roles, **When**
   evaluated, **Then** the job is immediately REJECTED, the reason "Seniority
   out of target range: Executive/VP" is recorded, and the job is excluded from
   the approved output.
3. **Given** a posting whose modality or location conflicts with a
   non-negotiable profile constraint, **When** evaluated, **Then** the job is
   REJECTED with a specific reason and excluded from the approved output.
4. **Given** a posting missing a core domain competency that the profile marks
   as a mandatory dealbreaker, **When** evaluated, **Then** the job is REJECTED
   with the missing requirement recorded.
5. **Given** a posting that names none of the active profile's acceptable
   technologies and is not Trainee/Intern, **When** evaluated, **Then** the job
   is REJECTED with a specific technology reason.
6. **Given** a posting that passes all hard filters but scores below the active
   approval threshold, **When** evaluated, **Then** the job is not APPROVED,
   remains PENDING for operator review, and is excluded from the primary
   approved list.
7. **Given** a posting with unknown seniority, modality, location, or
   work-authorization status, **When** the profile marks that criterion as
   mandatory, **Then** the system does not treat the unknown value as a match,
   records the gap, and keeps the job out of the approved list.
8. **Given** the same profile version, posting, and active rules, **When**
   evaluated twice, **Then** the match score, status, and breakdown are
   identical.
9. **Given** any evaluated posting, **When** the evaluation is inspected,
   **Then** the score is an integer from 0 to 100 and the four weighted
   dimension contributions reconcile to the total score (40% core technical,
   25% domain/architectural, 20% seniority/role, 15% secondary).
10. **Given** a posting whose score is exactly equal to the approval threshold,
    **When** evaluated, **Then** the posting is treated as meeting the threshold
    and is eligible for approval if no hard filter is triggered.

---

### User Story 4 - Review ranked matches and auditable execution results (Priority: P4)

The operator reviews a ranked list of APPROVED opportunities, ordered by match
score descending, with matched keywords, missing requirements, and score
breakdowns. The operator also sees an execution audit covering ingestion
totals, duplicates merged, rejections by reason, source failures, and final
approved candidates, and can mark an approved opportunity as applied.

**Why this priority**: The review and audit experience is the visible outcome of
the pipeline and the basis for trust. It can be tested with a completed
evaluation run.

**Independent Test**: Run a fixed pipeline with known totals and statuses;
verify that the approved list is sorted and filtered, the audit reconciles with
the records, rejected jobs remain visible in the audit but not the approved
list, and an applied mark creates a lifecycle event.

**Acceptance Scenarios**:

1. **Given** three approved jobs with scores 92, 84, and 76, **When** the
   operator opens the results, **Then** the jobs appear in descending score
   order with their breakdowns and matched/missing requirements, and equal
   scores are ordered by the documented tie-breaker.
2. **Given** a completed run, **When** the operator opens the audit, **Then**
   total ingested, duplicates merged, hard-filtered rejections by reason, source
   failures, and approved count reconcile exactly with the underlying records,
   and the audit identifies the run with its start and end times.
3. **Given** one or more rejected jobs, **When** the operator reviews results,
   **Then** rejected jobs are absent from the approved list but their rejection
   reasons are available in the audit or rejection report.
4. **Given** a run with no approved jobs, **When** the operator opens the
   results, **Then** the system shows a clear zero-result state and still
   exposes the audit.
5. **Given** an approved job, **When** the operator marks it applied, **Then**
   the lifecycle history records the transition with a timestamp, and re-running
   ingestion or evaluation does not create a duplicate opportunity.
6. **Given** any report, log, or audit output, **When** inspected, **Then** it
   contains no credentials, session material, or raw job-description text.
7. **Given** a completed evaluation, **When** the operator changes the profile,
   weights, threshold, or dealbreakers, **Then** prior evaluations are marked
   stale and are not presented as current until re-evaluated.

---

### Edge Cases

- The PDF is password-protected, encrypted, corrupt, image-only, unsupported,
  or larger than the supported input limit.
- The PDF contains multiple columns, tables, headers/footers, or text in a
  language other than the operator's working language.
- Two resumes contain conflicting values for the same scalar field, such as
  years of experience or current title.
- A resume is submitted twice, or a newer version is submitted after matching
  has already run.
- A source returns zero results, malformed records, missing fields, duplicates,
  or stale postings.
- All sources fail, one source fails repeatedly, or a source rate limit is
  exhausted.
- The same opportunity appears with slightly different company names, titles,
  locations, or URLs.
- A posting has an unknown or ambiguous seniority, modality, location, or
  work-authorization requirement.
- A job passes every hard filter but scores exactly at the approval threshold.
- Multiple jobs receive the same score, or a large result set contains many
  ties.
- The operator changes the profile, dealbreakers, weights, or threshold after a
  run; prior results become stale.
- The operator marks a job applied, then the source republishes or updates the
  posting.
- A report or audit is generated while a discovery run is still in progress or
  after a partial failure.

## Requirements *(mandatory)*

### Functional Requirements

#### Candidate Profile Requirements (FR-PRF)

- **FR-PRF-001 (PDF Ingestion)**: The system MUST accept PDF resume files as an
  input source for creating or updating a CandidateProfile.
- **FR-PRF-002 (Text Extraction and Structuring)**: The system MUST extract text
  and structural metadata from an ingested PDF and map the content into a
  structured CandidateProfile covering technical competencies, domain
  experience, work history, education, and certifications.
- **FR-PRF-003 (Multi-Resume Merging)**: When multiple resumes are provided, the
  system MUST merge their attributes into one master profile without creating
  duplicate entries. List-valued attributes MUST be unioned; conflicting scalar
  values MUST be flagged for operator review rather than silently overwritten.
- **FR-PRF-004 (Profile Verification and Override)**: The system MUST present
  the extracted CandidateProfile in an editable form so the operator can review,
  correct, or add non-extractable constraints, including work authorization and
  visa needs, target roles, target seniority, preferred modalities, salary
  thresholds, and dealbreakers, before matching runs.
- **FR-PRF-005 (Profile Validation)**: The system MUST validate a profile against
  the required structure before matching and MUST identify missing or ambiguous
  fields that affect hard filtering.
- **FR-PRF-006 (Profile Versioning)**: The system MUST record which profile
  version was used for each evaluation so results remain traceable after the
  operator edits the profile.
- **FR-PRF-007 (Safe Failure)**: If PDF parsing fails, the system MUST report a
  specific error and MUST NOT create or overwrite a profile with partial or
  unverified data.
- **FR-PRF-008 (Source Traceability)**: The system MUST record which source
  document or documents contributed to each extracted competency.

#### Job Ingestion Requirements (FR-ING)

- **FR-ING-001 (Search Criteria)**: The system MUST accept search configurations
  containing target job titles, technical skills, geographic constraints, and
  work-modality preferences (Remote, Hybrid, On-site).
- **FR-ING-002 (Independent, Concurrent Ingestion)**: The system MUST ingest
  postings from multiple registered sources independently and concurrently.
- **FR-ING-003 (Fault Isolation)**: A failure in one source MUST NOT halt the
  run or prevent processing of data from remaining operational sources.
- **FR-ING-004 (Content Cleaning)**: The system MUST strip markup, control
  characters, and non-standard whitespace from raw job descriptions before
  downstream use.
- **FR-ING-005 (Rate Control)**: Every external ingestion task MUST enforce
  per-source rate limiting, throttling, and bounded exponential backoff.
- **FR-ING-006 (Source Lifecycle)**: The system MUST allow sources to be
  registered, enabled, disabled, or removed without changing matching or
  filtering logic.
- **FR-ING-007 (Source Lineage)**: The system MUST record the source name,
  external identifier, URL, and ingestion time for every posting.
- **FR-ING-008 (Empty and Failed Runs)**: The system MUST report zero-result and
  all-sources-failed outcomes clearly and preserve an execution record without
  crashing.
- **FR-ING-009 (Terms and Access)**: Ingestion MUST NOT intentionally bypass
  source rate limits, access controls, or terms of service.
- **FR-ING-010 (Search Configuration Versioning)**: The system MUST record which
  search configuration was used for each discovery run.
- **FR-ING-011 (Source Boundary)**: Registered sources and their integrations
  MUST NOT contain matching, scoring, or filtering rules.

#### Deduplication and Normalization Requirements (FR-DED)

- **FR-DED-001 (Canonical Identity)**: The system MUST derive a stable canonical
  identity for each posting from normalized company name, normalized job title,
  and primary location or posting URL.
- **FR-DED-002 (Duplicate Merge)**: Duplicate postings detected across sources
  MUST be merged into one canonical JobPosting that retains all source lineage.
- **FR-DED-003 (Pre-Evaluation Deduplication)**: Deduplication MUST occur before
  match evaluation.
- **FR-DED-004 (Controlled Normalization)**: The system MUST normalize work
  modality, seniority, and location into controlled values and MUST represent
  missing or unrecognized values as Unknown rather than guessing.
- **FR-DED-005 (Repeat-Safe Ingestion)**: Re-ingesting the same opportunity MUST
  NOT create duplicate canonical records, duplicate evaluation work, or duplicate
  lifecycle entries.
- **FR-DED-006 (Merge Completeness)**: When duplicates disagree, the canonical
  record MUST retain the most complete available descriptive content and MUST
  record conflicting values for audit.
- **FR-DED-007 (Identity Fallback)**: If the inputs required for canonical
  identity are missing, the system MUST use a documented fallback and flag the
  record for operator review.

#### Match and Filtering Requirements (FR-MAT)

- **FR-MAT-001 (Profile Comparison)**: The Match Engine MUST compare each
  posting against a structured CandidateProfile containing technical
  competencies, years of experience, target roles, preferred modalities,
  location constraints, and non-negotiable dealbreakers.
- **FR-MAT-002 (Hard Filtering)**: The system MUST immediately set a posting to
  REJECTED when any hard-filter condition is triggered:
  - Seniority is outside the candidate's target bounds, such as an Executive/VP
    role when the candidate targets Individual Contributor roles.
  - Work modality or location violates a non-negotiable constraint.
  - A core domain competency marked as a mandatory dealbreaker is absent.
- **FR-MAT-003 (Scoring Algorithm)**: For postings that pass hard filters, the
  system MUST compute an integer Match Score from 0 to 100 using four weighted
  dimensions: core technical skill overlap (40%), domain and architectural
  alignment (25%), seniority and role fit (20%), and secondary or desirable
  competencies (15%).
- **FR-MAT-004 (Explainable Breakdown)**: The system MUST generate a structured
  breakdown for every evaluated posting containing the match score, matched
  keywords, missing requirements, hard-filter rejection reasons, and the four
  dimension scores.
- **FR-MAT-005 (Hard-Filter Precedence)**: Hard filters MUST run before scoring
  for approval. A posting that triggers any hard filter MUST NOT be approved
  regardless of its score.
- **FR-MAT-006 (Score Integrity)**: The four dimension weights MUST sum to 100%,
  every dimension contribution MUST be reflected in the final score, and the
  final score MUST remain an integer within 0-100.
- **FR-MAT-007 (Approval Threshold)**: The active approval threshold MUST be
  explicit, versioned, and recorded with each evaluation. A posting that passes
  hard filters but falls below the threshold MUST NOT be approved.
- **FR-MAT-008 (Unknown Criteria)**: An unknown value for a criterion the profile
  marks as mandatory MUST NOT be treated as a match. The gap MUST be recorded,
  and the posting MUST remain out of the approved list.
- **FR-MAT-009 (Reproducibility)**: The same profile version, posting, and active
  rules MUST produce the same status and score.
- **FR-MAT-010 (Stale Results)**: When the profile, weights, threshold, or
  dealbreakers change, prior evaluations MUST be marked stale and MUST NOT be
  presented as current until re-evaluated.
- **FR-MAT-011 (Consistent Tie-Breaking)**: Jobs with equal scores MUST be ordered
  by a documented deterministic tie-breaker.
- **FR-MAT-012 (Requirement Extraction)**: The system MUST extract seniority,
  required skills, desirable skills, work modality, and location requirements
  from each clean posting before comparison.
- **FR-MAT-013 (Exact and Semantic Matching)**: The system MUST consider both
  exact and semantically related competency matches and MUST record which matched
  terms were exact versus inferred.
- **FR-MAT-014 (Threshold Boundary)**: A score exactly equal to the approval
  threshold MUST be treated as meeting the threshold.
- **FR-MAT-015 (Source Independence)**: The Match Engine MUST consume normalized
  JobPosting records and MUST NOT depend on source-specific integrations or
  their formats.

#### Output and Reporting Requirements (FR-REP)

- **FR-REP-001 (Ranked Approved List)**: The system MUST output a ranked list of
  APPROVED opportunities sorted by Match Score in descending order.
- **FR-REP-002 (Execution Audit)**: The system MUST maintain an execution audit
  containing total jobs ingested, duplicates merged, hard-filtered rejections
  with reasons, source failures, and final approved candidates.
- **FR-REP-003 (Audit Reconciliation)**: Audit totals MUST reconcile exactly with
  the underlying records for the same run.
- **FR-REP-004 (Match Detail)**: Each approved opportunity MUST expose its score,
  dimension breakdown, matched keywords, and missing requirements.
- **FR-REP-005 (Lifecycle History)**: The system MUST persist an append-only
  lifecycle history for each opportunity, recording at least discovered,
  analyzed, approved, rejected, and applied states with timestamps and reasons.
  History MUST NOT be silently overwritten.
- **FR-REP-006 (Application Marking)**: The operator MUST be able to mark an
  approved opportunity as applied, and that transition MUST be recorded without
  creating a duplicate opportunity.
- **FR-REP-007 (Discreet Output)**: Reports, logs, and audit output MUST NOT
  contain credentials, session material, or raw job-description text.
- **FR-REP-008 (Zero Results)**: When no opportunities are approved, the system
  MUST show a clear zero-result state and still provide the audit.
- **FR-REP-009 (Rejection Visibility)**: Rejection reasons MUST remain available
  in the audit or a rejection report even though rejected opportunities are
  excluded from the approved list.
- **FR-REP-010 (Run Identification)**: Every execution audit MUST be associated
  with a unique run identifier, start time, and end time.

#### Security and Privacy Requirements (FR-SEC)

- **FR-SEC-001 (Local Storage)**: Candidate profiles, credentials, access
  tokens, session cookies, and application history MUST be stored locally.
  Application-level encryption at rest is not required; protection relies on
  OS-level disk encryption and restricted filesystem permissions.
- **FR-SEC-002 (Authorized Exposure)**: Personal information and authentication
  material MUST NOT be exposed to any third party the operator has not
  authorized.
- **FR-SEC-003 (Secret Handling)**: Credentials and session material MUST NOT be
  written to logs, telemetry, error reports, or build artifacts.
- **FR-SEC-004 (Revocation)**: Removing a source or account MUST revoke and
  delete its associated credentials and session material.
- **FR-SEC-005 (Minimum Necessary Access)**: Access to stored profile and credential data
  MUST be limited to the components that require it for an explicit
  operator-initiated operation.
- **FR-SEC-006 (Raw Description Handling)**: Raw job-description text MUST NOT be
  persisted in SQLite, written to logs, telemetry, or reports, and MUST be
  discarded when no longer needed for the active processing run.

### Key Entities *(include if feature involves data)*

- **Candidate Profile**: The consolidated, operator-verified representation of
  the candidate. Key attributes: stable profile identifier, source-document
  references, target roles, target seniority range, preferred work modalities,
  location preferences, work authorization and visa needs, salary threshold,
  core competencies, secondary competencies, domain experience, dealbreakers,
  and profile version.
- **Source Document**: A resume used to build the profile. Key attributes: file
  name, source-document identity, parse time, parse status, and the profile
  attributes it contributed to.
- **Search Configuration**: A named set of discovery criteria. Key attributes:
  target titles, technical skills, geography, modalities, selected sources, and
  last-run reference.
- **Ingestion Source**: A registered job provider. Key attributes: source name,
  source type, enabled state, rate-limit policy, credential reference (not the
  secret itself), and last-run status.
- **Job Posting**: A normalized opportunity. Key attributes: job record
  identifier, stable job identity, title, company, location, work modality, raw
  and cleaned description, extracted seniority and skills, source lineage,
  evaluation status (PENDING, APPROVED, REJECTED), and lifecycle state.
- **Source Lineage**: The record of where a posting came from. Key attributes:
  source name, external identifier, URL, and ingestion time.
- **Match Evaluation**: The decision record for a posting. Key attributes:
  status, match score, hard-filter rejections, four dimension scores, matched
  keywords, missing requirements, active rules and weights version, threshold,
  profile version, and evaluation time.
- **Lifecycle Event**: A state transition for an opportunity. Key attributes:
  opportunity reference, state (discovered, analyzed, approved, rejected,
  applied), timestamp, and reason.
- **Execution Audit**: The record of one pipeline run. Key attributes: run
  identifier, start and end time, total ingested, duplicates merged, rejections
  by reason, source failures, approved count, and the search configuration used.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: An operator can submit a text-based PDF resume of up to 10 pages
  and obtain a valid, structured, editable CandidateProfile in under 2 minutes, with
  every extracted competency traceable to source evidence.
- **SC-002**: When three sources are configured and one fails, 100% of discovery
  runs complete with results from the remaining sources and a recorded failure
  for the unavailable source.
- **SC-003**: 100% of duplicate postings across sources are consolidated into one
  canonical record with complete source lineage, and re-running the same
  discovery creates no duplicate records.
- **SC-004**: 100% of postings that trigger a hard filter are marked REJECTED
  with a specific reason and are excluded from the approved list.
- **SC-005**: For every approved posting, the score is an integer from 0 to 100,
  the four dimension weights sum to 100%, and the weighted contributions
  reconcile to the reported score with a breakdown containing matched and
  missing requirements.
- **SC-006**: Re-evaluating the same posting with the same profile version and
  active rules produces an identical status and score in 100% of test cases.
- **SC-007**: The approved list is sorted in descending score order, contains
  only APPROVED postings, and resolves ties deterministically in 100% of runs.
- **SC-008**: The execution audit reconciles exactly with the underlying records
  for total ingested, duplicates merged, rejections by reason, source failures,
  and approved count in 100% of runs.
- **SC-009**: An operator can identify the top 10 approved opportunities and the
  primary reason for each match within 1 minute of opening the results.
- **SC-010**: No report, log, or audit output contains credentials, session
  material, or raw job-description text in 100% of tested runs.

## Assumptions

- The system is used by a single operator on a local installation; shared
  accounts and team workspaces are out of scope for v1.
- Ingested resumes are text-based PDFs. Password-protected, encrypted, corrupt,
  or image-only PDFs are rejected with a clear error; OCR is out of scope for v1
  unless added by a later specification.
- Multiple resumes may describe the same candidate in different versions or
  languages. List-valued attributes are merged; conflicting scalar values are
  flagged for operator review, with the most recently parsed non-conflicting
  value used until resolved.
- The parser may infer adjacent competencies from resume evidence. Inferred
  items MUST be marked as inferred, traceable to source evidence, and editable
  by the operator.
- Salary thresholds and dealbreakers are operator-provided preferences.
  Work authorization and visa sponsorship are out of scope for the v1 hard
  filters; the location filter already restricts the search to Brazil plus Porto
  Alegre/Curitiba.
- The default approval threshold is 70 on the 0-100 scale and is
  operator-configurable. Postings below the threshold remain PENDING and are
  excluded from the approved list.
- The v1 scoring weights are fixed at 40% core technical, 25%
  domain/architectural, 20% seniority/role, and 15% secondary competencies. The
  active weights version is recorded with each evaluation.
- Unknown seniority, modality, location, or mandatory competency is not treated
  as a match and is flagged for operator review.
- Equal scores are ordered by higher core technical skill score, then most recent
  ingestion time, then canonical title in ascending order.
- The system presents opportunities ready for application; it does not submit
  applications automatically. The operator may mark an opportunity as applied.
- Source credentials and candidate data remain local. External sources are
  accessed only for operator-initiated discovery, subject to their rate limits
  and terms.
- Source documents and job postings may be in English or Portuguese.
  Cross-language translation is out of scope for v1; the system preserves the
  source language and does not guarantee cross-language equivalence.
- Dependency: the operator has network access to configured job sources and a
  local environment capable of parsing PDFs and storing profile, evaluation, and
  audit data.
- Dependency: external source availability, content quality, and terms are
  outside the system's control and are handled through fault isolation and rate
  control.
- Out of scope: interview tracking, offer management, salary negotiation,
  employer communication, and automatic application submission.
