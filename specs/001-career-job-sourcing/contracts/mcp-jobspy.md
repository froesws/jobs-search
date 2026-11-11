# MCP JobSpy Contract

The sidecar is a local stdio MCP JSON-RPC server. The Rust client launches
`jobspy_mcp_server` and communicates over stdin/stdout. No network calls to the
sidecar are made.

## Tool: `search_jobs`

**Request**

```json
{
  "jsonrpc": "2.0",
  "id": "run-1",
  "method": "tools/call",
  "params": {
    "name": "search_jobs",
    "arguments": {
      "query": "Rust engineer",
      "location": "Brazil",
      "sites": ["linkedin", "indeed"],
      "results_wanted": 50,
      "hours_old": 168,
      "is_remote": true,
      "job_type": null,
      "offset": 0
    }
  }
}
```

**Response**

```json
{
  "jsonrpc": "2.0",
  "id": "run-1",
  "result": {
    "jobs": [
      {
        "title": "Senior Rust / Systems Engineer",
        "company": "Example",
        "location": "Remote - Brazil",
        "work_modality": "Remote",
        "url": "https://example.com/jobs/1",
        "posted_at": "2026-11-08T12:00:00Z",
        "description": "cleaned description",
        "source_name": "jobspy",
        "external_id": "linkedin-1"
      }
    ]
  }
}
```

## Error

```json
{
  "jsonrpc": "2.0",
  "id": "run-1",
  "error": {
    "code": -32000,
    "message": "source rate limited"
  }
}
```

## Rules

- The sidecar returns normalized fields only; matching and filtering stay in
  Rust.
- `description` is the cleaned description; the sidecar may return raw content
  only to the Rust process, which discards it after cleaning.
- Per-board rate limits and retries live in the sidecar; the Rust client applies
  an additional call-level limit.
- A sidecar failure is isolated to that source; remaining sources continue.
- The sidecar never receives credentials, profile data, or prompts.

## Sidecar configuration

`sources.yaml` is parsed by `src-tauri/src/config/sources.rs`. Each entry has an
`id`, `name`, `source_type` (`mcp`, `api`, or `web_scraper`), `enabled` flag,
type-specific `config`, `rate_limit`, and an optional `credential_ref` that names
an `.env` variable. Credential values are never written to the file or database.

```yaml
sources:
  - id: jobspy
    name: jobspy
    source_type: mcp
    enabled: true
    config:
      command: python3
      args: ["-m", "jobspy_mcp_server.server"]
      sites: ["linkedin", "indeed"]
      results_wanted: 50
    rate_limit:
      requests_per_minute: 20
      max_retries: 2
      backoff_ms: 500
      timeout_seconds: 60

  - id: remoteok
    name: remoteok
    source_type: api
    enabled: true
    config:
      base_url: "https://remoteok.com/api"
      results_wanted: 50
    rate_limit:
      requests_per_minute: 10
      max_retries: 2
      backoff_ms: 500
      timeout_seconds: 15
```

The Rust client applies a central concurrency limit and per-source timeout,
retries transient API/MCP failures with bounded backoff, and marks a source as
`FAILED` without aborting the remaining sources. Disable a source by setting
`enabled: false` rather than deleting it.
