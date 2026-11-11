# Quickstart: Career Intelligence & Job Sourcing Core

**Feature**: `001-career-job-sourcing`
**Date**: 2026-11-10

This guide validates the backend core after the `001` tasks are implemented. The
React/MUI UI is defined in `002`; the commands below exercise the backend through
tests and the Tauri command surface.

## Prerequisites

- Linux with Tauri v2 build dependencies.
- Rust `1.90+`.
- Python `3.14` for `sidecar/jobspy_mcp`.
- A local `.env` with provider keys:
  `GEMINI_API_KEY`, `OPENROUTER_API_KEY`, `DEEPSEEK_API_KEY`,
  `LLM_PROVIDER_ORDER=gemini,openrouter,deepseek`,
  `LLM_ALLOW_FALLBACK=false`, `OPENROUTER_FREE_MODELS`, `RUST_LOG=info`.
- `sources.yaml` with the JobSpy MCP source and the provisional public API
  source.
- Text-based PDF resume fixtures under `src-tauri/tests/fixtures/`.

## Setup

```bash
npm install
cargo build --manifest-path src-tauri/Cargo.toml
```

Start the app for a manual smoke check:

```bash
cargo run --manifest-path src-tauri/Cargo.toml
```

## Validation scenarios

### 1. PDF profile extraction and persistence

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test profile_pipeline
```

Expected: a text-based PDF produces a validated profile; the source document is
recorded; the profile survives a database reopen. A corrupt/image-only PDF
returns a specific error and leaves the previous profile unchanged.

### 2. Multi-resume merge and scalar conflicts

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test profile_pipeline
```

Expected: list-valued attributes are unioned without duplicates; conflicting
scalar values are flagged for review; the operator's manual technology tags and
accepted cities are preserved.

### 3. Multi-source discovery, cleaning, and dedup

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test ingestion_pipeline
cargo test --manifest-path src-tauri/Cargo.toml --test dedup_identity
```

Expected: the MCP JobSpy source and the Rust API source return normalized jobs;
one source failure does not halt the run; duplicates merge into one canonical
job with all source lineage; re-running creates no duplicates; all distinct URLs
are retained when the job passes hard filters.

### 4. Hard filters and explainable scoring

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test match_filters
cargo test --manifest-path src-tauri/Cargo.toml --test score_contract
```

Expected: location, seniority, technology, and mandatory-competency filters are
applied before scoring; an approved job has a 0–100 integer score, 40/25/20/15
breakdown, and N/M counts where matched + missing = total. Threshold 70 is
applied, including the exact-threshold boundary.

### 5. Audit, application, and ignore flows

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: the ranked approved list is sorted; the audit reconciles ingested,
duplicates, rejections, source failures, and approvals; applying/ignoring writes
an append-only lifecycle event; re-ingesting does not duplicate the opportunity.

### 6. Provider fallback

Use mocked provider tests in `src-tauri/tests/llm_provider_contract.rs`.

Expected: Gemini is tried first only when `LLM_ALLOW_FALLBACK=true`. Without the
flag, a transient 429/timeout/5xx/malformed response stops with a recorded error
and the job stays unapproved. With the flag, fallback goes to OpenRouter free
models, then DeepSeek, and every attempt records `fallback_from`.
Authentication/configuration errors always stop without fallback. The
provider/model are recorded with each evaluation.

### 7. Raw-description and privacy check

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test migration_contract
```

Expected: no `description_raw` column exists; logs/reports/audit contain no raw
description, credentials, or prompts; data remains local.

### 8. Full verification gate

```bash
./scripts/verify.sh
```

Expected: the verification gate passes or reports skipped areas explicitly. Do
not claim a skipped check passed.

## Validation evidence (2026-11-10)

- `./scripts/verify.sh`: PASSED (Prettier, oxlint, `tsc --noEmit`, Vitest 8,
  `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`, `cargo test`,
  `pytest` 8).
- `npm run build`: PASSED.
- `npm run tauri build -- --debug --no-bundle`: PASSED; binary at
  `src-tauri/target/debug/jobs-search`.
- Privacy audit: no `description_raw` column or writes, no credentials in
  `sources.yaml`, no external telemetry packages, and no raw prompt/description
  logging.
- `profile_pipeline`: 17 tests covering text/corrupt/image-only/oversized PDFs,
  merge/conflicts, versioning, upload/duplicate/delete, recalculation, and
  `RESUME_REQUIRED`.
- `application_lifecycle` and `audit_contract`: 9 tests covering apply/ignore/
  restore, manual approval snapshots, audit reconciliation, zero results, and
  ranked/stale feeds.

## Implemented scope (P1–P4)

PDF ingestion, multi-resume merge, versioned profile persistence, hybrid
ingestion, dedup, matching, lifecycle/apply/ignore/restore, manual approval, and
execution audit are implemented. The synthetic fixture remains available as a
development fallback, but an uploaded resume profile takes precedence in SQLite.

### Fixture profile

Set the fixture path in `.env` for a first run without PDFs:

```bash
JOBS_SEARCH_FIXTURE_PROFILE=/absolute/path/to/src-tauri/tests/fixtures/profile.json
```

The fixture is synthetic: Rust/C++/C#/React targets, Porto Alegre/Curitiba plus
remote Brazil, Trainee/Junior/Pleno, threshold 70, and no PII. A search without a
stored profile or fixture returns `PROFILE_REQUIRED`; after deleting the last
resume it returns `RESUME_REQUIRED` until a new resume is uploaded or the profile
is saved again.

### Profile and lifecycle commands

```text
cmd_upload_resume(file_path) -> CandidateProfile
cmd_delete_resume(file_id) -> CandidateProfile
cmd_get_profile() -> CandidateProfile
cmd_save_profile(profile) -> CandidateProfile
cmd_apply_job(job_id) -> JobApplication
cmd_ignore_job(job_id) -> JobApplication
cmd_restore_job(job_id) -> JobApplication
cmd_approve_job(job_id) -> JobEvaluationView
cmd_get_audit(run_id) -> ExecutionAudit
```

Uploading writes the PDF under app-data `resumes/` with `0600` permissions,
stores its SHA-256 and validated extraction, and merges list-valued fields.
Deleting the last resume preserves the profile but marks it unavailable for new
searches. Manual approval preserves the original score and breakdown and sets
`approval_source=manual`.

### Commands

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test ingestion_pipeline
cargo test --manifest-path src-tauri/Cargo.toml --test match_filters
cargo test --manifest-path src-tauri/Cargo.toml --test score_contract
cargo test --manifest-path src-tauri/Cargo.toml --test llm_provider_contract
cargo test --manifest-path src-tauri/Cargo.toml --test fixture_match
.venv/bin/pytest sidecar/jobspy_mcp/tests
./scripts/verify.sh
```

### Recorded results

- `ingestion_pipeline`: 2 passed. One MCP fake and one RemoteOK mock produce two
  normalized postings, merge into one canonical job, retain two distinct URLs,
  two lineage rows, and conflicting description values, reconcile one execution
  run, and isolate one failing source. A zero-result run is persisted as
  `SUCCESS`.
- `match_filters`: 8 passed. Covers remote Brazil, accepted cities, foreign
  remote restrictions, senior-only rejection, Pleno/Senior ranges, Trainee
  technology exemption, dealbreakers, and unknown mandatory criteria.
- `score_contract`: 3 passed. Covers 40/25/20/15, N/M counts, missing dimensions,
  and the inclusive 70 threshold.
- `llm_provider_contract`: 4 passed. Covers 429, timeout, malformed output, and
  authentication stopping without fallback.
- `fixture_match`: 3 passed. Produces one approved, one pending, three rejected,
  and one error job; stale evaluations are hidden from feeds and re-evaluated.
- `sidecar pytest`: 8 passed. Covers LinkedIn/Indeed normalization, rate
  limiting, bounded retries, exhausted retries, and MCP error handling.
- `./scripts/verify.sh`: passed. Frontend typecheck/tests were skipped because
  `002` does not exist yet; skipped checks are not counted as passing.

### First-milestone behavior

- One search at a time; a second call returns `SEARCH_IN_PROGRESS`.
- LLM attempts are 3 Gemini, 3 OpenRouter free-model attempts, and 2 DeepSeek
  attempts per job, in that order.
- Hard filters run before the LLM; matched + missing equals total requirements.
- `APPROVED` jobs appear in `New Jobs`; `PENDING` jobs appear in the read-only
  `Below Threshold` tab. Apply/ignore/manual approval remain hidden until P4.
- The search defaults are 7 days, profile seniority, the top core technology,
  all modalities, and all enabled sources.

## Manual Tauri smoke check

After the frontend `002` exists, use the commands in
[`contracts/tauri-commands.md`](./contracts/tauri-commands.md):

1. Upload a resume.
2. Save the verified profile and edit accepted cities/technologies.
3. Trigger a search with the default filters.
4. Inspect approved jobs, open a job detail, and apply/ignore.
5. Read the audit for the run.

Without `002`, the same behavior is covered by the Rust integration tests.
