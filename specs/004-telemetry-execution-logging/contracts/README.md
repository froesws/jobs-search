# Telemetry Contracts

## Log file

- Location: Tauri application data directory, `logs/`.
- Naming: `jobs-search.YYYY-MM-DD.log` (daily rotation).
- Format: UTF-8 JSON Lines, one `Log Event` per line.
- Schema: [`log-event.schema.json`](./log-event.schema.json).
- Retention: 7 days or 50 MB total, whichever limit is reached first.
- Access: local files only. The MVP has no in-app log viewer or export.

## Tauri command

### `cmd_get_log_health`

Returns a `Log Health` payload:

```json
{
  "available": true,
  "queue_depth": 0,
  "queue_capacity": 10000,
  "dropped_count": 0,
  "last_error": null,
  "last_success_at": "2026-11-10T20:00:00Z",
  "current_file": "jobs-search.2026-11-10.log"
}
```

- Schema: [`log-health.schema.json`](./log-health.schema.json).
- The command is read-only.
- It returns no log content, credentials, URLs, paths, payloads, prompts, or
  candidate data.
- No UI viewer or export is part of this feature.

## Validation

- Contract tests validate serialized events against the JSON Schema.
- Tests reject unknown fields, including forbidden URL/path/payload/prompt fields.
- The health command payload is validated against its schema.
