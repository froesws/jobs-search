# Contract: Search progress event

**Feature**: `015-faster-search` | **Date**: 2026-11-12

The backend emits one Tauri event while a run is active. The frontend listens with `listen()` from
`@tauri-apps/api/event`; no command is polled.

---

## Event

**Name**: `search://progress`

**Direction**: Rust → frontend

**Payload**:

```ts
interface SearchProgress {
  stage: 'collecting' | 'evaluating';
  found: number;
  evaluated: number;
  total: number;
  failed: number;
}
```

**Guarantees**

| # | Guarantee |
|---|-----------|
| 1 | The first event is emitted within 1 s of `cmd_trigger_search` starting. |
| 2 | `stage` only moves `collecting → evaluating`; it never goes back. |
| 3 | `found` and `failed` are non-decreasing while `collecting`. |
| 4 | `found` counts listed postings, including known postings whose detail was skipped. |
| 5 | `total` is set when `evaluating` starts and does not change afterwards. |
| 6 | `evaluated` is non-decreasing and never exceeds `total`. |
| 7 | At most one event per 250 ms, plus one per stage transition. |
| 8 | A final event is emitted before the command resolves, including on failure. |

**Rules**

- The payload contains counters only: no job titles, URLs, descriptions, credentials, provider
  payloads, or source names.
- Emission must not hold the database lock; the counters are plain integers owned by the run.
- If the frontend is not listening, emissions are dropped silently.
- An empty result set may emit only the final event; `evaluating` with `total = 0` is valid.

---

## Frontend contract

`src/viewmodels/useSearchProgress.ts`:

```ts
function useSearchProgress(active: boolean): SearchProgress | null;
```

- Subscribes on mount when `active` is true and unsubscribes when it becomes false or the
  component unmounts.
- Returns `null` before the first event and after the run finishes.
- Never throws: a failed subscription leaves the value `null`.
- Ignores malformed payloads instead of rendering `NaN` or throwing.

`NewJobsView` renders the progress line only while `active` is true:

```text
Collecting… 42 found · 1 source failed
Evaluating… 37 of 100
```

`DashboardView` owns the hook while the search is running and passes the value down. The line
disappears when the run resolves, whether it succeeded or failed.

---

## Capability

`src-tauri/capabilities/default.json` already grants `core:default`, which includes the event
permissions. Implementation must verify a real `listen()` call succeeds; if it does not, add
`core:event:default` explicitly.
