# Data Model: Job Sourcing Dashboard

**Feature**: `002-job-sourcing-dashboard`  
**Date**: 2026-11-10

The frontend is stateless beyond the current view. It mirrors the `001` DTOs and
does not persist profiles, jobs, evaluations, or applications.

## Job Summary

Fields: `job_id`, `title`, `company`, `location`, `work_modality`, `url`,
`posted_at`, `evaluation_status` (`PENDING`, `APPROVED`, `REJECTED`, `ERROR`),
`lifecycle_state` (`DISCOVERED`, `APPLIED`, `IGNORED`), `match_score`,
`matching_items_count`, `total_requirements_count`, `summary`.

## Job Detail

Extends Job Summary with `description_clean`, `requirements`, `matched_keywords`,
`missing_keywords`, `hard_filter_rejections`, and `source_lineage`. Never
contains `description_raw`.

## Candidate Profile

Fields: `profile_id`, `version`, `target_roles`, `seniority_range`,
`work_modalities`, accepted cities, acceptable technologies, core/secondary
competencies, domain experience, dealbreakers, source-document references. The
first milestone reads a fixture profile; editing is deferred to P1.

## Application

Fields: `job_id`, `lifecycle_state`, `applied_at`, `approval_source`, and
evaluation snapshot reference. It is read-only in the first milestone; apply and
ignore are hidden until P4.

## ViewModel State

```text
idle -> loading -> ready | empty | error
```

Each ViewModel exposes data, error, pagination/filter state where applicable,
and actions. The drawer ViewModel adds selected job ID and detail state.

## Tab State

- `NEW_JOBS`: approved jobs from the backend, sorted by score.
- `BELOW_THRESHOLD`: PENDING jobs, read-only filter in the first milestone.
- `MY_APPLICATIONS`: empty in the first milestone; populated by P4.
- `MY_PROFILE`: fixture profile, read-only in the first milestone.
- `ANALYTICS`: shell; content in `003`.

## UI Constraints

- No raw descriptions, credentials, or source payloads in state.
- All labels are English: New Jobs, My Applications, My Profile, Analytics,
  I Applied, Refresh Search.
- Cards show title, company, location/modality, summary, score, and N/M.
- The drawer shows clean description, requirements, matched/missing, URL, and
  the action area (hidden until P4).
