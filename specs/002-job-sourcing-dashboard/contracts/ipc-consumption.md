# IPC Consumption

The frontend consumes the commands defined in
`specs/001-career-job-sourcing/contracts/tauri-commands.md` through
`src/lib/ipc.ts`. Components and Views do not call `invoke()` directly.

## First milestone

- `cmd_get_profile()` — fixture profile for My Profile.
- `cmd_trigger_search(filters)` — triggers backend ingestion + match.
- `cmd_get_jobs(tab, limit, offset)` — New Jobs and Below Threshold.
- `cmd_get_job(jobId)` — detail drawer.

## Deferred to later milestones

- `cmd_apply_job`, `cmd_ignore_job`, `cmd_restore_job` — P4; actions hidden.
- `cmd_approve_job` — P4/manual approval.
- `cmd_get_audit` — P4.
- Analytics commands — `003`.

## Error mapping

Backend errors `{ code, message }` map to a typed frontend error. Views show a
safe message and a retry action; no raw description, credential, or payload is
rendered.

## Rules

- No direct `invoke()` outside `src/lib/ipc.ts`.
- No source adapters, SQLite, or LLM calls in the frontend.
- No client-side scoring or lifecycle mutation.
- All IDs and DTOs are typed in `src/lib/types.ts`.
