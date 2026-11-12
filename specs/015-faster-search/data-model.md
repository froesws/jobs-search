# Data Model: Faster Search

**Feature**: `015-faster-search` | **Date**: 2026-11-11

No SQLite schema change. The model below covers the new configuration keys and the transient
progress payload.

---

## Entity: Source configuration (additive)

`SourceConfig` in `src-tauri/src/config/sources.rs` gains two optional keys. The struct uses
`deny_unknown_fields`, so both must be declared explicitly.

| Field | Type | Default | Validation |
|-------|------|---------|------------|
| `fallback_for` | `Option<String>` | `None` | must reference another configured source id, must not reference itself |
| `config.fallback_min_results` | `u32` | `1` | `1..=50` |
| `rate_limit.budget_seconds` | `u64` | `300` | `1..=900` |

**Rules**

- A source without `fallback_for` is a primary.
- A source with `fallback_for` is a fallback; it does not run unless its primary is
  "did not deliver".
- "Did not deliver" = the primary failed **or** returned fewer than `fallback_min_results`
  postings (zero included).
- A fallback whose `fallback_for` target is missing or disabled runs as a normal source and
  logs a configuration warning.
- A source cannot be the fallback of itself, and two fallbacks cannot target the same primary
  (rejected at load time with `AppError::Config`).

---

## Entity: Source outcome (extended)

`SourceOutcome` in `src-tauri/src/ingestion/mod.rs` gains one field:

| Field | Type | Meaning |
|-------|------|---------|
| `skipped` | `bool` | true when the source is a fallback that did not need to run |

`IngestionOutcome::failures()` and `jobs()` ignore skipped sources, so run metrics and
lineage are unaffected.

---

## Entity: Search progress (transient)

Emitted as the `search://progress` Tauri event. Never persisted.

| Field | Type | Meaning |
|-------|------|---------|
| `stage` | `"collecting" \| "evaluating"` | current stage |
| `found` | `u32` | postings collected so far |
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

---

## Entity: Evaluation settings (extended range)

`EvaluationSettings` in `src-tauri/src/match/mod.rs`.

| Field | Before | After |
|-------|--------|-------|
| `concurrency` | default `2`, clamped `1..=4` | default `4`, clamped `1..=8` |

Everything else (`threshold`, `max_jobs`, `criteria`) is unchanged.

---

## Relationships

```text
SourceConfig (primary) 1 ── 0..1 SourceConfig (fallback)
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
pending ──(primary)──► running ──► succeeded | failed
pending ──(fallback, primary ok)──► skipped
pending ──(fallback, primary did not deliver)──► running ──► succeeded | failed
```

---

## Migration

None. No table is added or altered. Existing databases and `sources.yaml` files keep working:
the new keys are optional and default to today's behavior except that the secondary LinkedIn
source must be annotated to become a fallback (otherwise it still runs in parallel).
