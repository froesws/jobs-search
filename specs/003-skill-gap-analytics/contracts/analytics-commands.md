# Analytics Command Contracts

All commands are local, read-only, and return
`Result<AnalyticsPayload, ErrorPayload>` with errors as `{ code, message }`.

## `cmd_get_analytics`

```rust
struct AnalyticsRequest {
  scope: AnalyticsScope,   // CURRENT_RUN | LAST_7_DAYS
  run_id: Option<String>,  // required for CURRENT_RUN
}
```

```rust
struct AnalyticsPayload {
  scope: AnalyticsScope,
  run_id: Option<String>,
  denominator: u32,
  last_updated: String,
  stale: bool,
  skill_gap: Vec<SkillGapMetric>,
  funnel: FunnelPayload,
  score_distribution: Vec<ScoreBucket>,
  llm_metrics: LlmMetricsPayload,
}
```

- `SkillGapMetric`: skill, display_name, demand_count, demand_percentage, rank.
- `FunnelPayload`: ingested, duplicates, rejected_by_reason, pending, approved,
  error, applied, ignored, approval_rate, application_rate, source_failures,
  source_breakdown, provider_breakdown.
- `ScoreBucket`: range label (`0-49`, `50-59`, `60-69`, `70-79`, `80-89`,
  `90-100`) and count.
- `LlmMetricsPayload`: call_count, success_count, error_count, fallback_count,
  p50_latency_ms, p95_latency_ms, by_provider.

## `cmd_export_analytics_csv`

```rust
struct ExportRequest {
  scope: AnalyticsScope,
  run_id: Option<String>,
  destination: String, // operator-selected local path
}
```

Returns `ExportResult { path, rows, bytes }`. The command validates the local
destination, writes the current view, and never uploads the file.

## Rules

- No command mutates profile, jobs, evaluations, or applications.
- Rejected/error/incomplete evaluations are excluded from skill gap.
- Lifecycle state does not affect skill gap eligibility.
- Stale evaluations remain eligible; the payload exposes `stale=true` so the UI
  can disclose it until re-evaluation.
- `CURRENT_RUN` resolves to the supplied run or the latest SUCCESS/PARTIAL run.
- `LAST_7_DAYS` filters evaluations by `evaluated_at`.
- No raw descriptions, prompts, payloads, or credentials in responses.
- Analytics is computed in SQL/backend, not in the frontend.
