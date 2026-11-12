# Contract: `sources.yaml` schema additions

**Feature**: `015-faster-search` | **Date**: 2026-11-12

`SourceConfig` and `RateLimitConfig` use `deny_unknown_fields`, so every new key is declared
explicitly and validated at load time.

---

## New keys

```yaml
sources:
  - id: jobspy
    name: jobspy
    source_type: mcp
    enabled: true
    config:
      command: uv
      args: ['run', '--project', 'sidecar/jobspy_mcp', 'jobspy-mcp-server']
      cwd: '.'
      sites: ['linkedin', 'indeed', 'glassdoor', 'zip_recruiter', 'google']
      results_wanted: 10
      fetch_descriptions: true
    rate_limit:
      requests_per_minute: 10
      max_retries: 2
      backoff_ms: 500
      timeout_seconds: 180
      budget_seconds: 300            # new, optional, 30..=900, default 180

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
    rate_limit:
      requests_per_minute: 4
      max_retries: 1
      backoff_ms: 1000
      timeout_seconds: 300
      budget_seconds: 180            # new, optional

  - id: linkedin_scraper
    name: linkedin_scraper
    source_type: mcp
    enabled: true
    fallback_for: linkedin           # new, optional, references another source id
    config:
      command: uv
      args: ['run', '--project', 'sidecar/linkedin_scraper_mcp', 'linkedin-scraper-mcp']
      cwd: '.'
      sites: ['linkedin']
      results_wanted: 10
      fallback_min_results: 1        # new, optional, 1..=50, default 1
    rate_limit:
      requests_per_minute: 4
      max_retries: 1
      backoff_ms: 1000
      timeout_seconds: 300
      budget_seconds: 180            # new, optional

  - id: remoteok
    name: remoteok
    source_type: api
    enabled: true
    config:
      base_url: 'https://remoteok.com/api'
      results_wanted: 50
      adapter: remoteok
    rate_limit:
      requests_per_minute: 10
      max_retries: 2
      backoff_ms: 500
      timeout_seconds: 15
      budget_seconds: 30             # new, optional
```

## Validation

| Rule | Error |
|------|-------|
| `fallback_for` equals the source's own `id` | `AppError::Config` — a source cannot be its own fallback |
| `fallback_for` references an unknown id | `AppError::Config` — unknown fallback target |
| two sources declare the same `fallback_for` | `AppError::Config` — duplicate fallback target |
| a fallback is the primary of another fallback | `AppError::Config` — fallback chains are not supported |
| `budget_seconds` outside `30..=900` | clamped |
| `fallback_min_results` outside `1..=50` | clamped |
| `budget_seconds` absent | `180` |
| `fallback_min_results` absent | `1` |

## Behavior

| Situation | Result |
|-----------|--------|
| Primary delivers `>= fallback_min_results` postings | Fallback is `fallback_skipped` and performs no requests |
| Primary delivers fewer than the minimum (including zero) | Fallback runs; its postings are merged with the primary's |
| Primary fails | Fallback runs; its postings are merged with any the primary produced |
| Primary lists only postings already stored | Those postings count as delivered; fallback does not run |
| Primary lists some new postings and every new detail fails | If known postings were also delivered, the source is not an error; delivery is the known count |
| Primary lists new postings and every detail fails with no known postings | `LINKEDIN_DETAILS_FAILED`; fallback runs |
| `fallback_for` target is disabled or absent | The source runs as a normal primary and a configuration warning is logged |
| Both primary and fallback fail | Both failures are reported individually; the run continues |
| Fallback is skipped | `job_sources.last_run_status` is left unchanged |

**Delivery definition**: `delivered = new postings with details + listed postings skipped because
they were already known`. Listed postings whose detail failed do not count.

## Start timing

A fallback starts as soon as its own primary is decided. It does not wait for unrelated primaries
to finish. The per-source budget applies to each fallback independently.

## Known postings

The LinkedIn adapter skips `get_job_details` for any listed ID present in the global known-posting
set (see [known-postings.md](./known-postings.md)). A skipped posting still counts as delivered and
still updates its LinkedIn lineage recency.

## Backwards compatibility

Existing `sources.yaml` files load unchanged. Without `fallback_for`, every enabled source runs in
parallel exactly as today — so the secondary LinkedIn source keeps duplicating work until the
operator annotates it. `README.md` must show the annotated example. Existing `rate_limit` blocks
without `budget_seconds` use `180`.
