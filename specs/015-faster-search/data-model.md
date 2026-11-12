# Data Model: Faster Search

**Feature**: `015-faster-search` | **Date**: 2026-11-12

No SQLite schema change. The model below covers the new configuration keys, the in-memory
known-posting set, the extended source outcome, and the transient progress payload.

---

## Entity: Source configuration (additive)

`SourceConfig` in `src-tauri/src/config/sources.rs` gains two top-level fields and one
`RateLimitConfig` field. The structs use `deny_unknown_fields`, so all must be declared
explicitly.

| Field | Type | Default | Validation |
|-------|------|---------|------------|
| `fallback_for` | `Option<String>` | `None` | must reference another configured source id; must not reference itself; must not create a chain |
| `config.fallback_min_results` | `u32` | `1` | clamped to `1..=50` |
| `rate_limit.budget_seconds` | `u64` | `180` | clamped to `30..=900` |

**Rules**

- A source without `fallback_for` is a primary.
- A source with `fallback_for` is a fallback; it does not run unless its primary delivered fewer
  than `fallback_min_results` postings or failed.
- "Delivered" = new postings with details **plus** listed postings skipped because they were
  already known.
- A fallback whose `fallback_for` target is missing or disabled runs as a normal source and logs a
  configuration warning.
- A source cannot be the fallback of itself; two sources cannot target the same primary; a
  fallback cannot be the primary of another fallback.
- A skipped fallback keeps its previous `job_sources.last_run_status` unchanged.

---

## Entity: Known posting (in memory)

Built by `db::jobs::known_postings` before ingestion. Never persisted; the persisted sources are
`job_postings` and `job_posting_sources`.

| Field | Type | Meaning |
|-------|------|---------|
| `external_id` | `String` | stable source identifier; only values beginning with `linkedin-` are loaded |
| `job_id` | `String` | internal `job_postings.id` |
| `url` | `String` | persisted posting URL, used when touching lineage |

**Query rule**

```sql
SELECT l.external_id, l.job_id, l.url
FROM job_posting_sources l
JOIN job_postings j ON j.id = l.job_id
WHERE l.external_id LIKE 'linkedin-%'
  AND TRIM(j.description_clean) <> ''
GROUP BY l.external_id
```

**Rules**

- The set is global across sources, so a posting first stored by JobSpy also satisfies the
  LinkedIn lookup.
- A known posting is skipped only when its stored description is non-empty after trimming.
- A skipped posting updates `job_posting_sources.ingested_at` for the LinkedIn source; it does
  not rewrite `job_postings`.
- The stored description is frozen for this feature; no periodic refresh.

---

## Entity: Fetch outcome (adapter result)

`FetchOutcome` replaces the bare `Vec<RawJob>` returned by `SourceAdapter::fetch`.

| Field | Type | Meaning |
|-------|------|---------|
| `jobs` | `Vec<RawJob>` | new postings with details |
| `listed` | `u32` | unique postings the source listed, including skipped known ones |
| `known_skipped` | `Vec<String>` | external IDs skipped because they were already known |

**Rules**

- `listed` counts unique IDs after the adapter's own deduplication.
- `known_skipped` contains only IDs present in the known set with a non-empty description.
- A LinkedIn source with `listed > 0` and `known_skipped.len() > 0` does not return
  `LINKEDIN_DETAILS_FAILED` even when every new detail fails.
- A source with `listed == 0` returns an empty outcome, not an error, unless the search itself
  failed.

---

## Entity: Source outcome (extended)

`SourceOutcome` in `src-tauri/src/ingestion/mod.rs` gains fields:

| Field | Type | Meaning |
|-------|------|---------|
| `listed` | `u32` | postings listed by the source |
| `known_skipped` | `Vec<KnownPosting>` | listed postings skipped because they were already stored |
| `fallback_skipped` | `bool` | true when the source is a fallback that did not need to run |
| `jobs` | `Vec<RawJob>` | new postings (unchanged) |
| `failure` | `Option<SourceFailure>` | unchanged |

**Rules**

- `IngestionOutcome::jobs()` and `failures()` ignore `fallback_skipped` sources.
- `total_fetched()` continues to count new `jobs` only; skipped known postings are not ingested.
- `delivered = jobs.len() + known_skipped.len()` is the fallback trigger.
- The command touches lineage for every `known_skipped` entry using its `job_id`, `url`, and the
  source id.

---

## Entity: Search progress (transient)

Emitted as the `search://progress` Tauri event. Never persisted.

| Field | Type | Meaning |
|-------|------|---------|
| `stage` | `"collecting" \| "evaluating"` | current stage |
| `found` | `u32` | postings listed so far, including skipped known ones |
| `evaluated` | `u32` | postings evaluated so far |
| `total` | `u32` | postings queued for evaluation |
| `failed` | `u32` | sources that failed so far |

**Rules**

- `found` and `failed` only grow during `collecting`.
- `total` is fixed when the stage becomes `evaluating`.
- `evaluated` grows monotonically up to `total`.
- The final event is emitted before `cmd_trigger_search` resolves, so the UI always clears the
  progress line.
- Emissions are coalesced to at most one per 250 ms, plus stage transitions.
- The payload contains counters only: no job titles, URLs, descriptions, credentials, or provider
  payloads.

---

## Entity: Evaluation settings (extended)

`EvaluationSettings` in `src-tauri/src/match/mod.rs` and the persisted setting.

| Field | Before | After |
|-------|--------|-------|
| `concurrency` | default `2`, clamped `1..=4` | default `4`, clamped `1..=8` |
| `app_settings.evaluation.concurrency` | absent | optional string, parsed and clamped `1..=8` |

**Resolution order**

1. `app_settings["evaluation.concurrency"]` when present and valid.
2. `JOBS_SEARCH_LLM_CONCURRENCY` / `AppConfig.max_concurrency`.
3. Built-in default `4`.

The value is resolved at the start of each run; saving it in the provider dialog does not require
restarting the application. `AppState.replace_config` is not used because it currently has no
callers.

---

## Relationships

```text
SourceConfig (primary) 1 ── 0..* SourceConfig (fallback)   # one level, no chains
KnownPosting * ── 1 job_postings                         # read-only lookup
FetchOutcome 1 ── 1 SourceOutcome
SourceOutcome * ── 1 SourceConfig
SearchProgress 1 ── 1 active run (in-memory)
```

---

## State transitions

A run moves through:

```text
idle ──trigger──► collecting ──sources done──► evaluating ──queue drained──► finished
                                                          └──fatal provider error──► failed
```

A source moves through:

```text
pending ──(primary)──► running ──► delivered | short | failed
pending ──(fallback, primary delivered)──► skipped
pending ──(fallback, primary short/failed)──► running ──► delivered | failed
```

A listed posting moves through:

```text
listed ──known with description──► skipped (lineage touched)
listed ──new or empty description──► detail fetched ──► new posting | detail failure
```

---

## Migration

None. No table is added or altered. Existing databases and `sources.yaml` files keep working: the
new keys are optional and default to today's behavior except that the secondary LinkedIn source
must be annotated with `fallback_for` to stop running in parallel. The evaluation concurrency uses
the existing `app_settings` key/value table.
