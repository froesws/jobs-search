# Tauri Command Contracts

All commands return `Result<T, ErrorPayload>` and serialize errors as
`{ "code": string, "message": string }`. Commands never return raw job
descriptions, credentials, or tokens.

## Profile

```rust
cmd_upload_resume(file_path: String) -> CandidateProfile
cmd_delete_resume(file_id: String) -> CandidateProfile
cmd_get_profile() -> CandidateProfile
cmd_save_profile(profile: CandidateProfile) -> CandidateProfile
```

`cmd_save_profile` increments the profile version and marks existing evaluations
stale.

## Search

```rust
cmd_trigger_search(filters: SearchFilters) -> SearchMetrics
```

```rust
struct SearchFilters {
  titles: Vec<String>,
  stack_keyword: String,
  timeframe: SearchTimeframe,       // H24 | D7 | D30
  seniorities: Vec<Seniority>,      // Trainee | Intern | Junior | Pleno | Senior
  modalities: Vec<Modality>,        // Remote | Hybrid | Onsite
  source_ids: Vec<String>,
}
```

`SearchMetrics` includes `run_id`, `status`, `total_ingested`,
`duplicates_merged`, `rejected_count`, `pending_count`, `approved_count`,
`error_count`, `started_at`, and `finished_at`.

## Jobs

```rust
cmd_get_jobs(tab: JobTab, limit: u32, offset: u32) -> PaginatedJobs
cmd_get_job(job_id: String) -> JobDetail
cmd_apply_job(job_id: String) -> JobApplication
cmd_ignore_job(job_id: String) -> JobApplication
cmd_restore_job(job_id: String) -> JobApplication
cmd_approve_job(job_id: String) -> JobEvaluation
```

`JobTab`: `NEW_JOBS`, `APPLICATIONS`, `BELOW_THRESHOLD`.

`JobSummary`: job ID, title, company, location, modality, URL, posted time,
evaluation status, lifecycle state, score, N/M counts, and a short summary.

`JobDetail`: summary plus clean description, extracted requirements, matched and
missing keywords, hard-filter reasons, and source lineage.

`JobApplication`: job ID, lifecycle state, applied time, approval source, and
evaluation snapshot reference.

`JobEvaluationView`: job ID, evaluation status, score, and approval source.

`ExecutionAudit`: run ID, status, start/end time, stored search configuration,
total ingested, duplicates merged, rejected/pending/approved/error counts,
applied/ignored counts, approval/application rates, rejection reasons and counts,
source failures, score distribution, and source/provider breakdown.

Lifecycle rules:

- `cmd_apply_job` requires an APPROVED, non-stale evaluation and captures an
  immutable evaluation snapshot in `job_applications`.
- `cmd_ignore_job` and `cmd_restore_job` transition lifecycle state without
  changing the evaluation.
- `cmd_approve_job` moves a scored PENDING evaluation to APPROVED, sets
  `approval_source=manual`, and preserves the original score/breakdown.
- Every mutation appends a `job_status_events` row; history is never rewritten.

## Audit

```rust
cmd_get_audit(run_id: String) -> ExecutionAudit
```

`ExecutionAudit` includes run ID, start/end time, total ingested, duplicates
merged, rejection reasons and counts, source failures, approved count, and the
search configuration.

## Health

`cmd_get_log_health` is defined by feature `004` and returns the local log-store
health payload. It is not part of the job pipeline contracts.

## Rules

- No command returns `description_raw`.
- URL and title/company fields are allowed only in job-facing responses.
- All mutating commands record lifecycle/evaluation events.
- Pagination is required for job lists; `limit` is capped by the backend.
- Errors never include credentials, prompts, payloads, or raw descriptions.
