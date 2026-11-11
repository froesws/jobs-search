# Data Model: Career Intelligence & Job Sourcing Core

**Feature**: `001-career-job-sourcing`
**Date**: 2026-11-10

SQLite is the transactional store. Raw job descriptions are never persisted.
`job_posting_sources` preserves lineage; evaluations and lifecycle events are
separate and append-only.

## Resume

A stored PDF used to build the candidate profile.

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `id` | TEXT PK | yes | Opaque stable ID |
| `filename` | TEXT | yes | Original filename only |
| `content_hash` | TEXT UNIQUE | yes | SHA-256 of file bytes |
| `local_path` | TEXT | yes | App data path; never exposed in logs |
| `uploaded_at` | DATETIME | yes | UTC |

## Candidate Profile

Exactly one active profile is supported in V1.

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `id` | TEXT PK | yes | Stable profile ID |
| `profile_json` | TEXT | yes | Validated JSON matching the profile schema |
| `acceptable_technologies_json` | TEXT | yes | List of normalized manual technology tags |
| `accepted_cities_json` | TEXT | yes | Default `["Porto Alegre", "Curitiba"]` |
| `version` | INTEGER | yes | Monotonic; incremented on each save |
| `updated_at` | DATETIME | yes | UTC |

`profile_json` contains: source-document references, target roles, target
seniority, preferred modalities, location preferences (remote Brazil + accepted
cities), work-authorization notes, salary threshold, core/secondary competencies,
domain experience, dealbreakers, mandatory criteria (`location`, `modality`,
`seniority`, or `domain:<term>`), and inferred/source-evidence metadata.

## Job Source

A registered ingestion provider.

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `id` | TEXT PK | yes | Stable source ID |
| `name` | TEXT UNIQUE | yes | e.g. `jobspy`, `remotive` |
| `source_type` | TEXT | yes | `mcp`, `api`, `web_scraper` |
| `config_json` | TEXT | yes | Type-specific config, no secrets |
| `enabled` | BOOLEAN | yes | Default true |
| `rate_limit_json` | TEXT | yes | Per-source limits and backoff policy |
| `credential_ref` | TEXT | no | `.env` key name, never the secret |
| `last_run_status` | TEXT | no | `SUCCESS`, `PARTIAL`, `FAILED` |

## Job Posting

The canonical normalized opportunity.

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `id` | TEXT PK | yes | Stable job record ID |
| `canonical_key` | TEXT UNIQUE | yes | Normalized company + title + location, or URL fallback |
| `canonical_hash` | TEXT UNIQUE | yes | SHA-256 of `canonical_key` |
| `title` | TEXT | yes | Sanitized display title |
| `company` | TEXT | yes | Sanitized company name |
| `location` | TEXT | yes | Controlled/display location; `Unknown` allowed |
| `work_modality` | TEXT | yes | `Remote`, `Hybrid`, `Onsite`, `Unknown` |
| `url` | TEXT | yes | Primary posting URL |
| `posted_at` | DATETIME | no | Publication time; null means Unknown |
| `description_clean` | TEXT | yes | Cleaned description, never raw |
| `extracted_metadata_json` | TEXT | no | Seniority, required/desirable skills, modality, location requirements |
| `conflicts_json` | TEXT | yes | Append-only bounded audit of conflicting source values; default `[]` |
| `created_at` | DATETIME | yes | UTC |
| `updated_at` | DATETIME | yes | UTC |

**Forbidden column**: `description_raw` does not exist.

## Job Posting Source

N:N lineage between canonical jobs and sources.

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `id` | TEXT PK | yes | Stable lineage ID |
| `job_id` | TEXT FK | yes | `job_postings.id` |
| `source_id` | TEXT FK | yes | `job_sources.id` |
| `external_id` | TEXT | yes | Source-specific ID |
| `url` | TEXT | yes | Source URL |
| `ingested_at` | DATETIME | yes | UTC |

Unique constraint: `(job_id, source_id, external_id)`. Every distinct URL is
retained when the job passes hard filters.

## Job Evaluation

The decision record for a job/profile/rule version.

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `id` | TEXT PK | yes | Stable evaluation ID |
| `job_id` | TEXT FK | yes | `job_postings.id` |
| `profile_version` | INTEGER | yes | Profile version used |
| `evaluation_status` | TEXT | yes | `PENDING`, `APPROVED`, `REJECTED`, `ERROR` |
| `match_score` | INTEGER | no | 0–100 when evaluated |
| `matching_items_count` | INTEGER | no | N; >= 0 |
| `total_requirements_count` | INTEGER | no | M; >= 0; N + missing = M |
| `score_breakdown_json` | TEXT | no | Four dimension contributions |
| `matched_keywords_json` | TEXT | no | Safe keyword list |
| `missing_keywords_json` | TEXT | no | Safe keyword list |
| `hard_filter_rejections_json` | TEXT | no | Specific reasons |
| `rules_version` | TEXT | yes | Active filter/weights version |
| `threshold` | INTEGER | yes | Active approval threshold; default 70 |
| `provider` | TEXT | no | Active LLM provider |
| `model` | TEXT | no | Active model |
| `evaluated_at` | DATETIME | no | UTC |
| `stale` | BOOLEAN | yes | Default false; set when profile/rules change |

Validation: if status is `APPROVED`, score MUST be 0–100, N/M counts MUST be
consistent, and no hard-filter rejection may be present.

## Job Application

The application lifecycle record for an approved job.

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `job_id` | TEXT PK/FK | yes | `job_postings.id` |
| `lifecycle_state` | TEXT | yes | `DISCOVERED`, `APPLIED`, `IGNORED` |
| `applied_at` | DATETIME | no | Required when state is `APPLIED` |
| `approval_source` | TEXT | yes | `automatic` or `manual` |
| `evaluation_snapshot_json` | TEXT | yes | Evaluation used at application time |
| `updated_at` | DATETIME | yes | UTC |

## Job Status Event

Append-only history for evaluation and lifecycle transitions.

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `id` | TEXT PK | yes | Stable event ID |
| `job_id` | TEXT FK | yes | `job_postings.id` |
| `event_type` | TEXT | yes | `EVALUATED`, `APPROVED`, `REJECTED`, `APPLIED`, `IGNORED`, `RESTORED` |
| `old_value` | TEXT | no | Previous state/status |
| `new_value` | TEXT | yes | New state/status |
| `reason` | TEXT | no | Sanitized reason |
| `created_at` | DATETIME | yes | UTC |

Events are never updated or deleted by normal application flows.

## Execution Run

One discovery + evaluation run.

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `id` | TEXT PK | yes | Stable run ID |
| `started_at` | DATETIME | yes | UTC |
| `finished_at` | DATETIME | no | UTC |
| `search_config_json` | TEXT | yes | Active filters and sources |
| `status` | TEXT | yes | `RUNNING`, `SUCCESS`, `PARTIAL`, `FAILED` |
| `total_ingested` | INTEGER | yes | >= 0 |
| `duplicates_merged` | INTEGER | yes | >= 0 |
| `rejected_count` | INTEGER | yes | >= 0 |
| `pending_count` | INTEGER | yes | >= 0 |
| `approved_count` | INTEGER | yes | >= 0 |
| `error_count` | INTEGER | yes | >= 0 |

## LLM Metric

Per-provider-call audit data.

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `id` | TEXT PK | yes | Stable metric ID |
| `run_id` | TEXT FK | yes | `execution_runs.id` |
| `provider` | TEXT | yes | `gemini`, `openrouter`, `deepseek` |
| `model` | TEXT | yes | Safe model ID |
| `operation` | TEXT | yes | `profile_extraction`, `job_evaluation` |
| `latency_ms` | INTEGER | yes | >= 0 |
| `outcome` | TEXT | yes | `SUCCESS`, `ERROR`, `FALLBACK` |
| `error_category` | TEXT | no | `TIMEOUT`, `RATE_LIMIT`, `AUTH`, `NETWORK`, `PARSE`, `CONFIG`, `UNKNOWN` |
| `fallback_from` | TEXT | no | Previous provider/model |
| `created_at` | DATETIME | yes | UTC |

## Relationships

- A profile has many resume references through `profile_json.source_documents`.
- A job posting has many source-lineage rows.
- A job posting has one current evaluation and one application.
- A job has many append-only status events.
- An execution run has many LLM metrics and one run-metrics summary.

## State Transitions

```text
Evaluation: PENDING -> APPROVED | REJECTED | ERROR
            PENDING -> APPROVED (manual approval, approval_source = manual)
            APPROVED -> stale when profile/rules change

Lifecycle:  APPROVED evaluation -> DISCOVERED
            DISCOVERED -> APPLIED | IGNORED
            IGNORED -> DISCOVERED (restore)
            APPLIED -> (terminal for the MVP; snapshot preserved)

Application snapshot: captured when the operator marks APPLIED; later profile
or rule changes do not rewrite it.
```

## Indexes and Constraints

- Unique `job_postings.canonical_hash` and `canonical_key`.
- Unique `job_posting_sources(job_id, source_id, external_id)`.
- Index `job_evaluations(job_id, evaluated_at)`.
- Index `job_applications(lifecycle_state)`.
- Index `job_status_events(job_id, created_at)`.
- Foreign keys with `ON DELETE CASCADE` for lineage/evaluation/status events.
- No raw-description column; no credential values in any table.
