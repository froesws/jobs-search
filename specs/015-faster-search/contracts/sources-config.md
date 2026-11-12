# Contract: `sources.yaml` schema additions

**Feature**: `015-faster-search` | **Date**: 2026-11-11

`SourceConfig` and `RateLimitConfig` use `deny_unknown_fields`, so both new keys are declared
explicitly and validated at load time.

---

## New keys

```yaml
sources:
  - id: linkedin
    name: linkedin
    source_type: mcp
    enabled: true
    config:
      command: uvx
      args: ['mcp-server-linkedin@latest']
      adapter: linkedin
      results_wanted: 10
      max_pages: 1
      detail_concurrency: 4        # new, optional, 1..=4, default 4
    rate_limit:
      requests_per_minute: 4
      max_retries: 1
      backoff_ms: 1000
      timeout_seconds: 300
      budget_seconds: 300          # new, optional, 1..=900, default 300

  - id: linkedin_scraper
    name: linkedin_scraper
    source_type: mcp
    enabled: true
    fallback_for: linkedin         # new, optional, references another source id
    config:
      command: uv
      args: ['run', '--project', 'sidecar/linkedin_scraper_mcp', 'linkedin-scraper-mcp']
      cwd: '.'
      sites: ['linkedin']
      results_wanted: 10
      fallback_min_results: 1      # new, optional, 1..=50, default 1
    rate_limit:
      requests_per_minute: 4
      max_retries: 1
      backoff_ms: 1000
      timeout_seconds: 300
```

## Validation

| Rule | Error |
|------|-------|
| `fallback_for` equals the source's own `id` | `AppError::Config` — a source cannot be its own fallback |
| `fallback_for` references an unknown id | `AppError::Config` — unknown fallback target |
| two sources declare the same `fallback_for` | `AppError::Config` — duplicate fallback target |
| `detail_concurrency` outside `1..=4` | clamped, not rejected (matches existing `clamp` style) |
| `budget_seconds` outside `1..=900` | clamped |
| `fallback_min_results` outside `1..=50` | clamped |

## Behavior

| Situation | Result |
|-----------|--------|
| Primary succeeds with `>= fallback_min_results` postings | Fallback is `skipped` and performs no requests |
| Primary succeeds with fewer than the minimum (including zero) | Fallback runs; its postings are merged with the primary's |
| Primary fails | Fallback runs; its postings are merged with any the primary produced |
| `fallback_for` target is disabled or absent | The source runs as a normal primary and a configuration warning is logged |
| Both primary and fallback fail | Both failures are reported individually; the run continues |

## Backwards compatibility

Existing `sources.yaml` files load unchanged. Without `fallback_for`, every enabled source
runs in parallel exactly as today — so the secondary LinkedIn source keeps duplicating work
until the operator annotates it. `README.md` must show the annotated example.
