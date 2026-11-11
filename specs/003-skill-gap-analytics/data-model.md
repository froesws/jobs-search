# Data Model: Skill Gap Analytics & Funnel

**Feature**: `003-skill-gap-analytics`  
**Date**: 2026-11-10

Analytics is read-only. It consumes the `001` tables (`job_postings`,
`job_evaluations`, `job_applications`, `job_posting_sources`, `execution_runs`,
`llm_metrics`) and does not create a second source of truth.

## Analytics Dataset

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `scope` | enum `CURRENT_RUN`, `LAST_7_DAYS` | yes | Fixed enum |
| `run_id` | string/null | current run only | Must reference a stored run |
| `denominator` | integer | yes | >= 0; all eligible evaluated jobs |
| `last_updated` | datetime | yes | UTC |
| `stale` | boolean | yes | True when newer evaluations exist |

Eligibility: completed evaluation, passed hard filters (`APPROVED` or
`PENDING`), not deleted. Rejected/error/incomplete jobs excluded. Lifecycle state
does not affect eligibility.

## Skill Gap Metric

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `skill` | string | yes | Normalized, trimmed, case-insensitive key |
| `display_name` | string | yes | Consistent display casing |
| `demand_count` | integer | yes | >= 0; each job counted once |
| `demand_percentage` | float | yes | Count / denominator * 100, one decimal |
| `rank` | integer | yes | Count desc, then display name asc |
| `last_computed` | datetime | yes | UTC |

Excluded per job: skills in the active profile and skills in `matched_keywords`.
A skill repeated in one job's missing keywords counts once.

## Run Metrics

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `run_id` | TEXT PK/FK | yes | `execution_runs.id` |
| `total_ingested` | integer | yes | >= 0 |
| `duplicates_merged` | integer | yes | >= 0 |
| `rejected_by_reason_json` | TEXT | yes | Map reason -> count |
| `pending_count` | integer | yes | >= 0 |
| `approved_count` | integer | yes | >= 0 |
| `applied_count` | integer | yes | >= 0 |
| `ignored_count` | integer | yes | >= 0 |
| `approval_rate` | float | yes | approved / evaluated, 0–100 |
| `application_rate` | float | yes | applied / approved, 0–100 |
| `score_distribution_json` | TEXT | yes | Buckets 0-49, 50-59, 60-69, 70-79, 80-89, 90-100 |
| `source_breakdown_json` | TEXT | yes | Source -> counts |
| `provider_breakdown_json` | TEXT | yes | Provider/model -> counts |
| `updated_at` | datetime | yes | UTC |

## LLM Metric

Uses the `001` `llm_metrics` table: run ID, provider, model, operation, latency,
outcome, error category, fallback origin, created time. Analytics exposes call
count, success/error count, fallback count, and p50/p95 latency by provider/model.

## CSV Export Row

Aggregate section: scope, denominator, last updated, skill gap rows, funnel
counts/rates, source/provider breakdown, score distribution, LLM metrics.

Job section: title, company, location, modality, URL, source(s),
evaluation_status, lifecycle_state, match_score, matching_items_count,
total_requirements_count, matched_keywords, missing_keywords, `description_clean`.

Forbidden: `description_raw`, credentials, prompts, payloads.

## State and Validation

- Empty scope: no-data state, no division by zero.
- Zero missing skills: zero-gap state with denominator.
- Denominator changes after new evaluations: stale flag until refresh.
- CSV is generated only on explicit operator action and never uploaded.
