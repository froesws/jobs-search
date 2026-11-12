# Quickstart: Faster Search

**Feature**: `015-faster-search` | **Date**: 2026-11-12

Automated validation guide. Shapes live in [data-model.md](./data-model.md) and
[contracts/](./contracts/); implementation detail belongs in `tasks.md`.

---

## Prerequisites

- Node.js 24+, npm, Rust 1.90 toolchain.
- Python 3.14 + `uv` for the sidecar tests.
- No provider key or LinkedIn session is required for the automated suites; they use fake
  adapters and stdio fake servers.

---

## Full gate

```bash
./scripts/verify.sh
```

A skipped check is not a passing check. Report every skipped check explicitly.

---

## Focused suites

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test ingestion_skip
cargo test --manifest-path src-tauri/Cargo.toml --test ingestion_fallback
cargo test --manifest-path src-tauri/Cargo.toml --test ingestion_budget
cargo test --manifest-path src-tauri/Cargo.toml --test ingestion_concurrency
cargo test --manifest-path src-tauri/Cargo.toml --test search_progress
cargo test --manifest-path src-tauri/Cargo.toml --test linkedin_mcp_contract
cargo test --manifest-path src-tauri/Cargo.toml --test score_contract
npm test -- src/viewmodels/useSearchProgress.test.ts src/views/NewJobsView.test.tsx
npm test -- src/components/ProviderConfigDialog.test.tsx
```

---

## Scenario 1 — Known posting skips the detail

**Command**:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test ingestion_skip known_posting_skips_detail
```

**Expected**: the fake LinkedIn server records zero `get_job_details` calls for the known ID, the
lineage row's `ingested_at` is updated, and the stored description is unchanged.

---

## Scenario 2 — Unknown or description-less posting is fetched

**Command**:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test ingestion_skip fetches_unknown_and_empty
```

**Expected**: an unknown ID and a known ID with an empty description both trigger
`get_job_details`; the empty description is filled in.

---

## Scenario 3 — Cross-source known posting

**Command**:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test ingestion_skip jobspy_lineage_satisfies_lookup
```

**Expected**: a `linkedin-<id>` row stored under the JobSpy source is found by the global lookup;
the LinkedIn adapter skips the detail and adds the LinkedIn lineage.

---

## Scenario 4 — Known-only primary does not start the fallback

**Command**:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test ingestion_fallback known_only_primary_counts_as_delivered
```

**Expected**: the primary's `listed` and `known_skipped` counts make `delivered >=
fallback_min_results`; the fallback outcome is `fallback_skipped` and performs zero requests.

---

## Scenario 5 — Fallback trigger matrix

**Command**:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test ingestion_fallback
```

**Expected**: the matrix covers delivered enough, empty, below minimum, failure, disabled primary,
both fail, and the per-primary start timing. Each failed source is reported individually.

---

## Scenario 6 — Source budget

**Command**:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test ingestion_budget
```

**Expected**: an adapter that never resolves becomes `SOURCE_TIMEOUT` after its budget; the other
adapter's postings are returned; the run completes.

---

## Scenario 7 — Evaluation concurrency

**Command**:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test ingestion_concurrency
```

**Expected**: the observed in-flight count never exceeds the configured value, reaches it when
enough jobs are queued, and a rate-limit error preserves completed evaluations.

---

## Scenario 8 — Progress event

**Command**:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test search_progress
npm test -- src/viewmodels/useSearchProgress.test.ts src/views/NewJobsView.test.tsx
```

**Expected**: the sequence starts with `collecting`, moves once to `evaluating`, is monotonic, and
ends with a final event. The hook subscribes, updates, unsubscribes, and handles a failed
subscription. The progress line renders while searching and disappears when the run ends.

---

## Scenario 9 — Concurrency setting persists

**Command**:

```bash
cargo test --manifest-path src-tauri/Cargo.toml provider_config
npm test -- src/components/ProviderConfigDialog.test.tsx
```

**Expected**: the effective value loads from `app_settings`, falls back to
`JOBS_SEARCH_LLM_CONCURRENCY`, rejects values outside `1..=8`, and the saved value is used by the
next run without restarting.

---

## Scenario 10 — Outcomes unchanged

**Command**:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test score_contract
cargo test --manifest-path src-tauri/Cargo.toml --test fixture_match
```

**Expected**: statuses, scores, matched/missing keywords, and rejection codes are identical to the
current behavior.

---

## Manual runs

Manual desktop runs are optional and are **not** part of the definition of done for this feature.
The automated gate above is the validation contract. If a manual run is performed, record the
desktop session and LinkedIn session used, but do not treat a skipped manual run as a failure of
the automated gate.

---

## Rollback

No migration is involved. Removing the new `sources.yaml` keys restores the previous parallel
behavior; the evaluation concurrency falls back to its default. Nothing is persisted by the
progress feature.
