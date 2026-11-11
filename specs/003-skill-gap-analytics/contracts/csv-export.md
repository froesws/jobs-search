# CSV Export Contract

## Trigger

`cmd_export_analytics_csv` is an explicit operator action. It writes a local file
to the operator-selected path and never uploads it.

## Aggregate section

Scope, run ID (if any), denominator, last updated, skill gap rows, funnel counts,
approval/application rates, rejection reasons, source/provider breakdown, score
distribution, LLM call/latency/fallback metrics.

## Job section

One row per job in the current view: title, company, location, work modality, URL,
source(s), evaluation status, lifecycle state, score, N/M counts, matched
keywords, missing keywords, `description_clean`.

## Forbidden

- `description_raw`
- credentials, tokens, cookies, API keys
- model prompts or responses
- source payloads

## Format

- UTF-8 CSV with a header row.
- Stable column order documented in the test fixture.
- Local file only; no automatic transfer.
- A sensitive-data warning is shown before export.
