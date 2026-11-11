# Feature Specification: Skill Gap Analytics

**Feature Branch**: `N/A (no branch hook configured)`

**Created**: 2026-11-10

**Status**: Draft

**Input**: User description: "Add a real-time market skill-gap analytics view to the
local job sourcing dashboard, aggregating missing technologies across evaluated
jobs and ranking them by demand frequency and relative percentage, with
dashboard refinements and performance, privacy, and resilience targets."

This specification extends the desktop dashboard defined in
`specs/002-job-sourcing-dashboard/spec.md`, which in turn extends the core
pipeline in `specs/001-career-job-sourcing/spec.md`. It adds the "Analytics"
dashboard tab, which aggregates missing skills from evaluated jobs and ranks
them by market demand. It also captures the refinements in this request: a
30-day timeframe, an "All" modality option, an active language-model provider
indicator, location and modality on job cards, an applied-jobs table, and
performance, privacy, and resilience targets.

Requirements restated here take precedence for the analytics workflow. The
dashboard, application, ingestion, and evaluation behavior defined in 002
remains in force.

## V1 Scope Amendments

This section supersedes conflicting requirements below.

- Analytics is in the MVP. Tabs are New Jobs, My Applications, My Profile, and
  Analytics. UI labels are English: "New Jobs", "My Applications", "My Profile",
  "Analytics", "I Applied", and "Refresh Search".
- Analytics has two sections: **Skill Gap** and **Funnel**, with a time-range
  selector **Current Run / Last 7 Days**.
- **Skill Gap**: eligible evaluated jobs that passed hard filters,
  lifecycle-neutral (APPLIED/IGNORED still count), denominator all eligible
  jobs, one decimal, ranked by demand count. The manual technology list is
  merged into the active profile's core competencies.
- **Funnel**: counts (ingested, duplicates, rejected by reason, pending,
  approved, applied, ignored), approval/application rates, breakdown by source
  and provider, score distribution, and LLM latency/fallback.
- Analytics data comes from structured `execution_runs`, `run_metrics`, and
  `llm_metrics` tables, not from local log files. A search run is one "Refresh
  Search" execution; application actions are lifecycle events.
- CSV export exports the current Analytics view: aggregate metrics plus one row
  per job with title, company, location, modality, URL, source, evaluation and
  lifecycle status, score, N/M, keywords, and `description_clean`. It never
  exports `description_raw` or credentials. The file is local, explicitly
  generated, and shown with a sensitive-data warning.
- Stack: current Tauri/React/TypeScript/Rust versions with MUI 9.5 + Emotion; no
  Tailwind, TanStack Query, or Zustand. No app-level encryption at rest;
  local-only data; OS disk encryption and permissions; API keys in `.env`.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - View ranked missing-skill demand (Priority: P1)

The System Operator opens the "Analytics" tab to see which technologies are most
frequently required by target jobs but absent from the active CandidateProfile.
Each skill is shown with its demand count and relative percentage, ranked from
highest to lowest demand, using progress bars and frequency cards.

**Why this priority**: This is the new value delivered by the feature. It tells
the operator which skills are worth learning next, using evidence from the jobs
already evaluated.

**Independent Test**: Seed 100 jobs in the analytics scope, 50 of which list
"Kubernetes" as a missing skill. Open the Analytics tab and verify that
Kubernetes is shown with a demand count of 50 and a demand percentage of 50.0%,
ranked above lower-demand skills.

**Acceptance Scenarios**:

1. **Given** 100 jobs in the analytics scope and 50 of them have "Kubernetes" in
   their missing keywords, **When** the operator opens "Analytics", **Then**
   Kubernetes is displayed with demand count 50 and demand percentage 50.0%.
2. **Given** several missing skills with different demand counts, **When** the
   Analytics tab renders, **Then** skills are ranked by demand count descending,
   and equal counts are ordered alphabetically by skill name.
3. **Given** a skill is present in the active CandidateProfile or is listed as a
   matched keyword for a job, **When** analytics aggregates missing skills,
   **Then** that skill is not counted as a gap for that job.
4. **Given** a skill appears multiple times in one job's missing keywords,
   **When** analytics counts demand, **Then** that job contributes only once to
   that skill's count.
5. **Given** there are no jobs in the analytics scope, **When** the operator
   opens "Analytics", **Then** the tab shows a clear no-data state and does not
   divide by zero or show an invalid percentage.
6. **Given** every job in the analytics scope has no missing skills, **When** the
   operator opens "Analytics", **Then** the tab shows a zero-gap state and still
   displays the number of jobs used.
7. **Given** an APPLIED or IGNORED job in the analytics scope, **When** analytics
   aggregates demand, **Then** the job still contributes to market demand unless
   the job or its evaluation is deleted.

---

### User Story 2 - Understand and trust the analytics metrics (Priority: P2)

The System Operator can see what the analytics numbers mean: the scope of jobs
included, the total number of jobs used as the denominator, the percentage
formula, the last update time, and whether the displayed data is stale. The
operator can also see how skill variants are normalized and why a skill is
excluded from the gap list.

**Why this priority**: Analytics is only actionable if the operator can trust the
counts and percentages. This slice makes the metric explainable and auditable.

**Independent Test**: Open Analytics after a profile change and after a new
evaluation. Verify that the scope, denominator, definition, and last-updated
time are visible; that a newly added profile skill disappears from the gap list;
and that a deleted evaluation removes its contribution.

**Acceptance Scenarios**:

1. **Given** the Analytics tab is open, **When** the operator inspects the
   summary, **Then** the active scope and the total number of jobs used as the
   denominator are visible.
2. **Given** a demand percentage is displayed, **When** the operator inspects its
   definition, **Then** the interface explains that it is the number of jobs
   missing the skill divided by the total jobs in scope, multiplied by 100 and
   rounded to one decimal place.
3. **Given** a new evaluation completes after the Analytics tab was opened,
   **When** the tab refreshes or the operator requests a refresh, **Then** the
   metrics update, and a stale indicator is shown until the update completes.
4. **Given** the operator adds a missing skill to the active CandidateProfile,
   **When** analytics refreshes, **Then** that skill no longer appears as a gap.
5. **Given** a job or evaluation is deleted, **When** analytics refreshes,
   **Then** the deleted job no longer contributes to any demand count or to the
   denominator.
6. **Given** a job has an incomplete or failed evaluation, **When** analytics
   aggregates the dataset, **Then** the job is excluded from the analytics
   denominator and from missing-skill counts.
7. **Given** two spelling variants of the same skill (for example, different
   capitalization or surrounding whitespace), **When** analytics aggregates
   them, **Then** they are combined into one metric with a consistent display
   name.

---

### User Story 3 - Navigate the four-tab dashboard and use the refined views (Priority: P3)

The System Operator can switch between "New Jobs", "My Applications",
and "Analytics" without losing search filters or application state. The top bar
provides the search timeframe, seniority, stack, and modality controls, the
search trigger, resume management controls, and the active language-model
provider indicator. "New Jobs" cards include location and modality, and
"My Applications" presents applied jobs as a table with a status badge and
application date.

**Why this priority**: Analytics MUST integrate with the existing workflow
without disrupting the operator's search and application tracking.

**Independent Test**: Switch between all four tabs, change filters, apply a job,
and return to Analytics. Verify that state is preserved, the top bar controls are
present, the card and table fields are correct, and the active provider is
visible.

**Acceptance Scenarios**:

1. **Given** the dashboard is open, **When** the operator inspects the navigation,
   **Then** the tab bar shows "New Jobs", "My Applications", "My Profile", and
   "Analytics" with those exact labels.
2. **Given** the operator switches tabs, **When** the new tab loads, **Then** the
   search filters, the applied-jobs state, and the selected tab remain intact for
   the session.
3. **Given** the top bar is visible, **When** the operator inspects it, **Then**
   it contains the "24h", "7d", and "30d" timeframe options; seniority,
   stack, and modality controls including "All"; the search trigger; the resume
   file controls; and the active language-model provider indicator.
4. **Given** jobs appear in "New Jobs", **When** the operator views a card,
   **Then** the card shows title, company, location or modality, summary snippet,
   match score badge, and N/M requirement ratio.
5. **Given** applied jobs exist, **When** the operator opens
   "My Applications", **Then** the jobs appear in a table with a status badge
   and application date, and remain available for query, filtering, sorting, and
   detail inspection.
6. **Given** the Analytics data is loading, empty, stale, unavailable, or has no
   missing skills, **When** the operator opens the tab, **Then** the interface
   shows the corresponding clear state instead of an empty or misleading chart.

---

### User Story 4 - Keep analytics private and responsive under load and failures (Priority: P4)

The System Operator can use Analytics with up to 1,000 local job records while
the interface remains responsive. Source timeouts abort within 15 seconds per
source without hanging the application, and analytics data remains on the local
device.

**Why this priority**: Performance, privacy, and resilience are non-negotiable
operating conditions for a local career-intelligence tool.

**Independent Test**: Seed 1,000 evaluated jobs, open the card grid and Analytics,
and measure rendering. Simulate a source timeout and verify that other sources
continue and the application remains responsive. Inspect storage and logs for
local-only data and absent credentials or raw descriptions.

**Acceptance Scenarios**:

1. **Given** 1,000 local job records, **When** the operator opens the card grid or
   the Analytics summary, **Then** the view renders within 100 milliseconds on
   the reference environment.
2. **Given** one source times out, **When** a search runs, **Then** that source
   aborts within 15 seconds, the remaining sources continue, and the application
   does not hang.
3. **Given** analytics data is displayed, **When** storage and network activity
   are inspected, **Then** the analytics data remains local and is not sent to a
   language-model provider or any other third party.
4. **Given** analytics output, logs, or audit records, **When** they are
   inspected, **Then** they contain no credentials, session material, or raw
   job-description text.
5. **Given** the local analytics dataset is unavailable or corrupt, **When** the
   operator opens Analytics, **Then** the tab shows an error state and does not
   present stale data as current.

---

### Edge Cases

- The analytics dataset is empty, contains one job, or contains more than 1,000
  jobs.
- Every job has the same missing skill, or no job has any missing skill.
- A skill is present in the profile but also appears in a stale evaluation's
  missing keywords.
- A skill is listed as both matched and missing for the same job because of a
  data inconsistency.
- Skill names differ by capitalization, surrounding whitespace, punctuation, or
  common aliases.
- A demand percentage rounds to 0.0%, 100.0%, or a value exactly at a rounding
  boundary.
- New evaluations complete, a profile changes, or a job is deleted while the
  Analytics tab is open.
- A job is marked APPLIED or IGNORED while Analytics is open.
- The active language-model provider changes while Analytics is open.
- The denominator changes because a job is added, deleted, or re-evaluated.
- A job has no missing keywords because no requirements were extracted.
- The local storage is unavailable, full, or corrupt.
- A source times out, all sources fail, or a search completes with partial
  results.
- The dashboard has a large result set, and the analytics list is long enough to
  require scrolling or a full-list view.
- The operator opens Analytics while a search or evaluation run is still in
  progress.
- A stale evaluation exists after the profile, provider, model, rules, or
  threshold changes.

## Requirements *(mandatory)*

### Functional Requirements

#### Skill Gap Analytics (FR-ANA)

- **FR-ANA-001 (Analytics Scope)**: The system MUST aggregate missing skills
  across the locally stored jobs that have a completed, non-error evaluation and
  passed the configured hard filters. Rejected, error, incomplete, or deleted
  evaluations MUST be excluded from the analytics dataset.
- **FR-ANA-002 (Scope Visibility)**: The system MUST display the active analytics
  scope and the total number of jobs used as the denominator.
- **FR-ANA-003 (Missing Skill Source)**: The system MUST derive missing skills
  from the structured missing keywords stored with each evaluation.
- **FR-ANA-004 (Profile and Match Exclusion)**: A skill present in the active
  CandidateProfile or listed as a matched keyword for a job MUST NOT be counted
  as missing for that job.
- **FR-ANA-005 (Count Once Per Job)**: Each skill MUST be counted at most once
  per job, even if it appears multiple times in that job's missing keywords.
- **FR-ANA-006 (Demand Count)**: The system MUST compute each skill's demand
  count as the number of jobs in the analytics scope that have that skill in
  their missing keywords.
- **FR-ANA-007 (Demand Percentage)**: The system MUST compute each skill's
  demand percentage as the demand count divided by the total jobs in the
  analytics scope, multiplied by 100 and rounded to one decimal place. The
  denominator MUST include jobs with no missing skills.
- **FR-ANA-008 (Ranking)**: The system MUST rank skills by demand count
  descending. Ties MUST be ordered alphabetically by the displayed skill name.
- **FR-ANA-009 (Display)**: The Analytics tab MUST display each ranked skill's
  name, demand count, demand percentage, and a visual progress bar or frequency
  card.
- **FR-ANA-010 (Normalization)**: Skill names MUST be compared case-insensitively
  and with surrounding whitespace removed. Variants that normalize to the same
  skill MUST be combined into one metric with a consistent display name.
- **FR-ANA-011 (Data Freshness)**: The Analytics tab MUST refresh when opened,
  after a completed evaluation, and after a relevant profile change. If newer
  data exists but has not been incorporated, the tab MUST show a stale indicator
  and provide a refresh action.
- **FR-ANA-012 (Empty and Zero States)**: When there are no jobs in scope, the
  system MUST show a no-data state without dividing by zero. When there are jobs
  but no missing skills, the system MUST show a zero-gap state and still display
  the denominator.
- **FR-ANA-013 (Lifecycle Neutrality)**: APPLIED and IGNORED lifecycle states
  MUST NOT remove a job from the analytics dataset. A job contributes to market
  demand until the job or its evaluation is deleted.
- **FR-ANA-014 (Provider Neutrality)**: Analytics MUST use stored evaluations and
  MUST NOT depend on the currently active language-model provider. Changing the
  provider MUST NOT rewrite historical analytics.
- **FR-ANA-015 (Reconciliation)**: The displayed demand counts and percentages
  MUST reconcile with the underlying missing-keyword records for the same scope
  and denominator.
- **FR-ANA-016 (Scope Independence)**: The analytics dataset MUST be independent
  of the active search filters. Changing a search filter MUST NOT change the
  analytics scope unless the operator explicitly changes the analytics scope.
- **FR-ANA-017 (Data Errors)**: If a skill is recorded as both matched and
  missing for the same job, the system MUST treat it as matched and MUST NOT
  count it as a gap for that job.
- **FR-ANA-018 (Read-Only View)**: The Analytics tab MUST be read-only. It MUST
  NOT modify profile, job, evaluation, or application records.
- **FR-ANA-019 (Long Lists)**: The Analytics tab MUST provide a way to review the
  full ranked list when it does not fit in the initial viewport, without
  silently truncating results.
- **FR-ANA-020 (Error State)**: If the analytics dataset cannot be read or
  computed, the system MUST show an error state and MUST NOT present stale data
  as current.
- **FR-ANA-021 (Funnel Counts)**: The Funnel section MUST show total ingested,
  duplicates merged, hard-filter rejections by reason, pending, approved,
  applied, and ignored counts for the selected time range.
- **FR-ANA-022 (Rates)**: The Funnel section MUST show approval rate (approved
  divided by evaluated) and application rate (applied divided by approved).
- **FR-ANA-023 (Source and Provider Breakdown)**: The Funnel section MUST break
  down counts by job source and by language-model provider/model.
- **FR-ANA-024 (Score Distribution)**: The Funnel section MUST show a score
  distribution for evaluated jobs using the buckets 0-49, 50-59, 60-69, 70-79,
  80-89, and 90-100.
- **FR-ANA-025 (LLM Metrics)**: The Funnel section MUST show LLM call count,
  success/error count, fallback count, and p50/p95 latency by provider/model.
- **FR-ANA-026 (Structured Metrics Source)**: Funnel values MUST come from
  structured `execution_runs`, `run_metrics`, and `llm_metrics` tables, not from
  parsing local log files.
- **FR-ANA-027 (Search Run)**: A search run MUST represent one "Refresh Search"
  execution encompassing ingestion, deduplication, and evaluation. Application,
  ignore, and manual-approval actions are lifecycle events linked to the job,
  not new search runs.
- **FR-ANA-028 (CSV Export)**: The system MUST export the current Analytics view
  as a local CSV. The file MUST include aggregate metrics plus one row per job
  with title, company, location, modality, URL, source, evaluation and lifecycle
  status, score, N/M, keywords, and `description_clean`. It MUST NOT include
  `description_raw` or credentials and MUST show a sensitive-data warning before
  export.

#### Dashboard Integration (FR-UI)

- **FR-UI-001 (Four Tabs)**: The interface MUST provide tab navigation across
  "New Jobs", "My Applications", "My Profile", and "Analytics".
- **FR-UI-002 (Top Bar Controls)**: The fixed top bar MUST include the "24h",
  "7d", and "30d" timeframe options; seniority selection (Trainee, Estágio,
  Junior, Mid, Senior); primary stack or language input; modality selection
  (Remote, Hybrid, Onsite, All); the "Refresh Search" trigger; resume upload
  and management controls; and the active language-model provider indicator.
- **FR-UI-003 (Card Content)**: "New Jobs" cards MUST show job title, company,
  location or modality, summary snippet, match score badge, and N/M requirement
  ratio.
- **FR-UI-004 (Applied Jobs Table)**: "My Applications" MUST present applied
  jobs in a table with a status badge and application date, and MUST allow query,
  filtering, sorting, and detail inspection.
- **FR-UI-005 (Analytics Content)**: "Analytics" MUST render two sections:
  Skill Gap with ranked progress bars/frequency cards, and Funnel with counts,
  rates, source/provider breakdown, score distribution, and LLM latency/fallback.
  The scope, denominator, time range, last updated time, and percentage
  definition MUST be visible.
- **FR-UI-012 (Analytics Time Range)**: The Analytics tab MUST provide a
  "Current Run" / "Last 7 Days" selector and apply the selected range consistently
  to both sections.
- **FR-UI-013 (CSV Export UI)**: The Analytics tab MUST provide a CSV export
  action for the current view, with the sensitive-data warning required by
  FR-ANA-028. Export MUST be an explicit operator action and MUST NOT upload the
  file anywhere.
- **FR-UI-014 (My Profile Tab)**: The interface MUST provide a "My Profile" tab
  for editing acceptable technologies and accepted on-site/hybrid cities. The
  manual technology list is merged into the profile's core competencies and used
  in both the hard filter and the core-skills score.
- **FR-UI-006 (Tab State Preservation)**: Switching tabs MUST preserve the active
  search filters, selected tab context, and application state for the session.
- **FR-UI-007 (Interface States)**: The Analytics tab MUST show distinct loading,
  no-data, zero-gap, stale, error, and populated states.
- **FR-UI-008 (Fixed Labels)**: The interface MUST display "New Jobs",
  "My Applications", "My Profile", "Analytics", "I Applied", and "Refresh
  Search" exactly as written.
- **FR-UI-009 (Provider Indicator)**: The top bar MUST display the active
  language-model provider name and MUST update when the active provider changes.
  It MUST NOT display credentials.
- **FR-UI-010 (External Link)**: Job detail links MUST open the original posting
  in the system browser, and a broken link MUST be reported without losing the
  stored job data.
- **FR-UI-011 (Large Result Sets)**: The dashboard MUST provide a way to navigate
  large job and analytics lists without silently truncating them.

#### Performance and Resilience (FR-NFR)

- **FR-NFR-001 (Rendering Performance)**: The card grid and Analytics summary
  MUST render within 100 milliseconds for up to 1,000 local job records on the
  reference environment.
- **FR-NFR-002 (Ingestion Timeout)**: A source network timeout MUST abort
  gracefully within 15 seconds per source, MUST NOT hang the application, and
  MUST NOT prevent other sources from completing.
- **FR-NFR-003 (Non-Blocking Analytics)**: Analytics computation MUST NOT block
  the rest of the interface. The tab MUST show a loading state while computing
  and remain navigable.
- **FR-NFR-004 (Partial Failure)**: A partial source failure MUST NOT invalidate
  analytics computed from previously stored evaluations. The affected source's
  failure MUST be visible in the execution audit.
- **FR-NFR-005 (Storage Failure)**: If local storage is unavailable or corrupt,
  the dashboard MUST report the failure and MUST NOT show analytics or job data
  as current.

#### Privacy and Security (FR-SEC)

- **FR-SEC-001 (Local Storage)**: Analytics data, job evaluations, Candidate
  Profiles, stored resumes, credentials, and application history MUST remain on
  local storage. Application-level encryption at rest is not required; protection
  relies on OS-level disk encryption and restricted filesystem permissions.
- **FR-SEC-002 (No External Analytics Transfer)**: Analytics data MUST NOT be
  sent to a language-model provider or any other third party. Only the data
  necessary for an explicitly initiated parsing or evaluation call may leave the
  local environment.
- **FR-SEC-003 (No Secrets or Raw Text)**: Analytics output, logs, and audit
  records MUST NOT contain credentials, session material, or raw
  job-description text.
- **FR-SEC-004 (Minimum Access)**: Access to analytics data and the underlying
  evaluations MUST be limited to the components required to render the tab or
  perform an explicit operator-initiated operation.
- **FR-SEC-005 (Provider Credentials)**: Provider credentials MUST NOT appear in
  the Analytics tab, the provider indicator, logs, or exported analytics data.
- **FR-SEC-006 (Deletion Propagation)**: Deleting a job or its evaluation MUST
  remove that record from future analytics calculations and MUST be reflected
  after the next refresh.

#### Provider Visibility (FR-LLM)

- **FR-LLM-001 (Active Provider Indicator)**: The top bar MUST show the active
  language-model provider name, and nothing about its credentials.
- **FR-LLM-002 (Provider Change)**: Changing the active provider MUST update the
  indicator and apply to subsequent parsing and evaluation operations without
  altering historical analytics.
- **FR-LLM-003 (Historical Traceability)**: The provider and model recorded with
  each evaluation MUST remain available for audit, while analytics remains
  provider-neutral.
- **FR-LLM-004 (No Silent Fallback)**: A provider failure MUST be reported. The
  system MUST NOT silently route analytics or evaluation work to a different
  provider.

### Key Entities *(include if feature involves data)*

- **Skill Gap Metric**: An aggregated missing-skill result. Key attributes:
  normalized skill name, display name, demand count, demand percentage, rank,
  and last computed time.
- **Analytics Dataset**: The set of jobs eligible for analytics. Key attributes:
  scope definition, total eligible job count, last updated time, and stale state.
- **Search Run**: One "Refresh Search" execution. Key attributes: run identifier,
  start and end time, active filters, outcome, and links to run metrics.
- **Run Metrics**: Aggregated funnel data for a search run. Key attributes:
  ingested, duplicates merged, rejections by reason, pending, approved, applied,
  ignored, approval rate, application rate, and score distribution.
- **LLM Metric**: A provider-call metric. Key attributes: run identifier,
  provider, model, operation, latency, outcome, error category, and fallback
  origin.
- **Candidate Profile**: The active verified candidate profile used to determine
  which skills are already present. Key attributes: profile identifier, version,
  core competencies, secondary competencies, and dealbreakers.
- **Job Evaluation**: A completed evaluation record. Key attributes: job
  reference, outcome, matched keywords, missing keywords, score, provider, model,
  and evaluated time.
- **Job Posting**: A normalized opportunity. Key attributes: stable job identity,
  title, company, location, modality, URL, and lifecycle state.
- **Job Application**: The application lifecycle record. Key attributes: job
  reference, state (DISCOVERED, APPLIED, IGNORED), and applied timestamp.
- **LLM Provider Configuration**: The active provider settings. Key attributes:
  provider name, model, connection settings, and credential reference (not the
  secret itself).
- **Search Configuration**: The active dashboard search filters. Key attributes:
  timeframe, seniority, stack, modality, and selected sources. The analytics
  scope is independent of these filters.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Given 100 jobs in the analytics scope and 50 jobs missing
  "Kubernetes", the Analytics tab shows Kubernetes with demand count 50 and
  demand percentage 50.0%.
- **SC-002**: 100% of analytics rows are ranked by demand count descending, with
  ties ordered alphabetically.
- **SC-003**: Skills present in the active profile or matched for a job are not
  counted as gaps, and a skill repeated within one job is counted only once, in
  100% of test cases.
- **SC-004**: The analytics scope, denominator, percentage definition, and last
  updated time are visible on the tab in 100% of sessions.
- **SC-005**: When no jobs are in scope, the tab shows a no-data state without an
  invalid percentage; when no skills are missing, it shows a zero-gap state and
  the denominator.
- **SC-006**: When a profile changes, a new evaluation completes, or a job is
  deleted, the analytics metrics reflect the change after refresh, with stale
  data visibly marked until then.
- **SC-007**: The dashboard provides "New Jobs", "My Applications", "My Profile",
  and "Analytics" tabs, and switching tabs preserves the active search and
  application state in 100% of test runs.
- **SC-008**: The top bar shows the "24h", "7d", and "30d" timeframe options,
  seniority, stack, modality including "All", the search trigger, resume
  controls, and the active provider indicator in 100% of sessions.
- **SC-009**: "New Jobs" cards show location or modality in addition to title,
  company, summary, score, and N/M ratio; "My Applications" shows a table
  with status badge and application date.
- **SC-010**: The card grid and Analytics summary render within 100 milliseconds
  for 1,000 local job records on the reference environment.
- **SC-011**: A source timeout aborts within 15 seconds, other sources continue,
  and the application remains responsive in 100% of tested runs.
- **SC-012**: Analytics data remains local; no credentials, session material, or
  raw job-description text appear in analytics output, logs, or audit records.
- **SC-013**: Analytics demand counts and percentages reconcile with the
  underlying missing-keyword records in 100% of runs.
- **SC-014**: Changing the active language-model provider updates the top-bar
  indicator and does not rewrite historical analytics.
- **SC-015**: An operator can identify the top missing skills and their demand
  percentages within 1 minute of opening the Analytics tab.
- **SC-016**: Funnel counts and rates reconcile with the underlying job and
  evaluation records for the selected time range in 100% of runs.
- **SC-017**: Funnel breakdowns by source and provider, plus the score
  distribution, are present and match the underlying records.
- **SC-018**: LLM call count, errors, fallback count, and p50/p95 latency are
  visible by provider/model.
- **SC-019**: CSV export of the current view includes the required job fields,
  excludes `description_raw` and credentials, and shows the sensitive-data
  warning in 100% of exports.
- **SC-020**: The Analytics tab exposes both Skill Gap and Funnel sections and
  applies the Current Run / Last 7 Days selector consistently.

## Assumptions

- This specification extends `specs/001-career-job-sourcing/spec.md` and
  `specs/002-job-sourcing-dashboard/spec.md`. Requirements restated here take
  precedence for analytics; dashboard, application, ingestion, and evaluation
  requirements not restated remain in force.
- The analytics scope is the locally stored set of jobs with a completed,
  non-error evaluation that passed the configured hard filters. Rejected, error,
  incomplete, and deleted jobs are excluded. The scope is independent of the
  active search filters.
- The denominator is the total number of jobs in the analytics scope, including
  jobs that have no missing skills. A job with no extracted requirements still
  counts in the denominator but contributes no missing-skill demand.
- APPLIED and IGNORED jobs remain part of market demand unless the job or its
  evaluation is deleted. Application lifecycle state does not change analytics
  eligibility.
- Missing skills come from the structured missing keywords of completed
  evaluations. A skill present in the active profile or matched for a job is not
  a gap for that job.
- Skill names are compared case-insensitively and with surrounding whitespace
  removed. A documented normalization rule may merge common aliases; variants
  that normalize to the same skill appear as one metric.
- Demand percentages are calculated as demand count divided by the total jobs in
  scope, multiplied by 100, and rounded to one decimal place. When the
  denominator is zero, no percentage is displayed.
- Analytics refreshes when the tab is opened, after a completed evaluation, and
  after a relevant profile change. If newer data exists but has not been
  incorporated, the tab shows a stale indicator and a refresh action.
- The performance target of 100 milliseconds for 1,000 local records is measured
  on the reference environment. If the target cannot be met, the interface MUST
  show a loading state and remain navigable rather than blocking.
- Source timeouts abort within 15 seconds per source. Partial source failures do
  not invalidate analytics computed from previously stored evaluations.
- The system is a single-operator local desktop application. Multi-user access,
  shared accounts, and team workspaces are out of scope for v1.
- The interface labels "New Jobs", "My Applications", "Analytics",
  "I Applied", and "Refresh Search" are fixed. Broader localization is out of
  scope for v1.
- The supported providers remain Gemini, DeepSeek, and OpenRouter as defined in
  002. The top bar displays only the active provider name, never credentials.
- The Analytics tab is read-only. Learning recommendations, course suggestions,
  salary analytics, external market benchmarking, and automatic application
  submission are out of scope for v1.
- Dependency: local storage is available and has sufficient capacity for
  profiles, resumes, jobs, evaluations, applications, and analytics data;
  protection at rest relies on OS-level disk encryption and permissions.
- Dependency: the operator has network access to configured job sources and
  language-model providers, valid provider credentials, and source terms that
  permit the configured access.
