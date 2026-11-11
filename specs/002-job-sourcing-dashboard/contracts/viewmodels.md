# ViewModel Contracts

All ViewModels are hooks under `src/viewmodels/` and return a state object plus
actions. Views render only from these contracts.

## `useJobsViewModel`

State: `status` (`idle|loading|ready|empty|error`), `items: JobSummary[]`,
`total`, `limit`, `offset`, `error`, `filters`.

Actions: `refresh()`, `setFilters(filters)`, `nextPage()`, `previousPage()`,
`openJob(jobId)`.

## `useJobDetailViewModel`

State: `status`, `job: JobDetail | null`, `error`, `isDrawerOpen`.

Actions: `open(jobId)`, `close()`, `retry()`.

## `useProfileViewModel`

State: `status`, `profile: CandidateProfile | null`, `error`.

Actions: `refresh()`. Editing is deferred to P1; the fixture profile is
read-only in the first milestone.

## `useApplicationsViewModel`

State: `status`, `items: JobApplication[]`, `error`.

Actions: `refresh()`. Apply/ignore actions are not exposed until P4.

## `useAnalyticsViewModel`

Shell in `002`: `status` and a “Coming in 003” state. The full contract is
defined by `003`.

## Test seams

- Mock `src/lib/ipc.ts` in tests; do not call Tauri directly.
- ViewModel tests cover loading, ready, empty, error, pagination, and drawer
  open/close.
- View tests cover labels, cards, drawer, tables, and hidden actions.
