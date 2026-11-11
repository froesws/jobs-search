# Implementation Plan: Job Sourcing Dashboard & Application Tracker

**Branch**: `002-job-sourcing-dashboard` | **Date**: 2026-11-10 |
**Spec**: [spec.md](./spec.md)

**Input**: Feature specification from
`/specs/002-job-sourcing-dashboard/spec.md`

## Summary

Build the React/MUI desktop presentation layer for Jobs Search with MVVM:
MUI Views, hook-based ViewModels, and Tauri IPC/type definitions as the Model.
The first milestone is a read-only New Jobs feed and detail drawer backed by
`001` (fixture profile + ingestion/dedup + match). Profile, applications, and
Analytics are present as shells; apply/ignore actions are hidden until `004`-style
P4 lifecycle work lands.

The frontend consumes the Tauri command contracts from `001`; it does not
implement ingestion, matching, SQLite, or LLM calls.

## Technical Context

**Language/Version**: React `19.3.1`, TypeScript `7.0.2`, Vite `8.3.6`.

**Primary Dependencies**: MUI `9.5` + Emotion, React DOM, Tauri API; Vitest +
Testing Library, oxlint, Prettier. No Tailwind, TanStack Query, Zustand, or
second UI/state library.

**Storage**: No frontend database. Profile, jobs, evaluations, and analytics are
read through Tauri commands; only ephemeral view state lives in memory.

**Testing**: Vitest + Testing Library for ViewModels and Views; mocked IPC for
deterministic tests; accessibility assertions for tabs, drawer, and forms.

**Target Platform**: Linux desktop via Tauri.

**Project Type**: React/TypeScript SPA embedded in the Tauri desktop shell.

**Performance Goals**: Feed/card rendering under 100 ms for 1,000 local records;
no blocking IPC in the render path; loading/empty/error states for every view.

**Constraints**: English UI labels (`New Jobs`, `My Applications`, `My Profile`,
`Analytics`, `I Applied`, `Refresh Search`); first milestone read-only; hide
apply/ignore actions; MVVM with hooks; no direct `invoke()` in components; no
raw job descriptions in the UI.

**Scale/Scope**: One operator, four tabs, one profile, paginated jobs, one
detail drawer, future analytics.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Gate | Status | How the plan satisfies it |
|------|--------|---------------------------|
| I. Source abstraction and agnosticism | PASS | The UI consumes normalized job DTOs; it never talks to sources or adapters. |
| II. Strict compatibility evaluation | PASS | Views display backend evaluation status, score, N/M, and breakdown; no client-side scoring. |
| III. Privacy, security, and identity | PASS | No credentials or raw descriptions in UI state; data comes through typed IPC. |
| IV. Operational resilience and rate control | PASS | Loading/error/partial-failure states; UI does not retry source calls independently. |
| V. Idempotency and data unification | PASS | Job IDs/canonical identity and lifecycle state are displayed from backend records. |
| Strict separation of concerns | PASS | Model = IPC/types, ViewModel = hooks, View = MUI components. |
| Technology neutrality and evolution | PASS | MUI/React versions are implementation details; no constitutional coupling. |
| Governance and compliance | PASS | Spec `002` and this plan follow constitution v2.0.0. |

## Project Structure

### Documentation (this feature)

```text
specs/002-job-sourcing-dashboard/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── README.md
│   ├── viewmodels.md
│   └── ipc-consumption.md
├── spec.md
└── checklists/requirements.md
```

### Source Code (repository root)

```text
src/
├── main.tsx
├── App.tsx
├── theme/
│   ├── theme.ts
│   └── tokens.ts
├── lib/
│   ├── ipc.ts            # typed Tauri command wrappers, no direct invoke in views
│   ├── types.ts          # DTOs mirroring 001 contracts
│   ├── errors.ts
│   └── format.ts
├── viewmodels/
│   ├── useJobsViewModel.ts
│   ├── useJobDetailViewModel.ts
│   ├── useProfileViewModel.ts
│   ├── useApplicationsViewModel.ts
│   └── useAnalyticsViewModel.ts  # shell; populated by 003
├── views/
│   ├── DashboardView.tsx
│   ├── NewJobsView.tsx
│   ├── ApplicationsView.tsx
│   ├── ProfileView.tsx
│   └── AnalyticsView.tsx
├── components/
│   ├── TopBar.tsx
│   ├── TabBar.tsx
│   ├── JobCard.tsx
│   ├── JobDrawer.tsx
│   ├── ScoreBadge.tsx
│   ├── RequirementRatio.tsx
│   ├── EmptyState.tsx
│   ├── ErrorState.tsx
│   └── LoadingState.tsx
└── test/
    ├── setup.ts
    └── fixtures.ts

vite.config.ts
tsconfig.json
index.html
package.json
```

**Structure Decision**: Frontend-only feature in the existing Tauri project.
MVVM is implemented as `viewmodels/*` hooks, `views/*` MUI screens, and
`lib/ipc.ts`/`lib/types.ts` as the Model boundary. No business logic or scoring
in the frontend.

## Data Model

The frontend mirrors the `001` DTOs; it does not own persistence.

- **JobSummary**: job ID, title, company, location, modality, URL, posted time,
  evaluation status, lifecycle state, score, N/M counts, snippet.
- **JobDetail**: summary plus clean description, structured requirements,
  matched/missing keywords, hard-filter reasons, source lineage.
- **CandidateProfile**: profile ID/version, target roles, seniority, modalities,
  accepted cities, acceptable technologies, core/secondary competencies, domain
  experience, dealbreakers.
- **JobApplication**: job ID, lifecycle state, applied time, approval source
  (read-only in the first milestone; apply/ignore hidden).
- **ViewModel state**: `idle | loading | ready | empty | error`, data, error
  message, and actions.
- **Analytics**: shell in this feature; metrics content is defined by `003`.

## Interface Contracts

- **ViewModel contract**: state shape, actions, and test seams for each tab. See
  [`contracts/viewmodels.md`](./contracts/viewmodels.md).
- **IPC consumption**: typed wrappers for `cmd_get_profile`, `cmd_trigger_search`,
  `cmd_get_jobs`, `cmd_get_job`; future `cmd_apply_job`/`cmd_ignore_job` stay
  hidden. See [`contracts/ipc-consumption.md`](./contracts/ipc-consumption.md).
- **Error contract**: backend `{ code, message }` mapped to user-facing states;
  no raw descriptions or credentials rendered.
- **UI contract**: four tabs, English labels, MUI components, drawer, cards,
  loading/empty/error states.

## Complexity Tracking

| Deviation | Why needed | Simpler alternative rejected because |
|-----------|------------|--------------------------------------|
| Hook-based MVVM | User selected MVVM; hooks align with React/rules | Class ViewModels need a reactivity layer and more boilerplate |
| No state library | Project rules prohibit TanStack/Zustand without approval | A global store adds dependency and cross-view coupling for a small SPA |
| Read-only first milestone | Backend P4 is deferred | Showing active apply buttons would produce dead controls/errors |
| Analytics shell in 002 | The tab is part of the dashboard | Omitting it would break the four-tab layout; 003 fills the content |

## Phase 0: Research

See [research.md](./research.md). All technical unknowns are resolved; no
`NEEDS CLARIFICATION` remains.

## Phase 1: Design

- [data-model.md](./data-model.md) defines frontend DTOs and ViewModel state.
- [contracts/viewmodels.md](./contracts/viewmodels.md) defines ViewModel
  state/actions and test seams.
- [contracts/ipc-consumption.md](./contracts/ipc-consumption.md) defines the
  backend commands consumed by the frontend.
- [quickstart.md](./quickstart.md) defines the UI validation scenarios with the
  fixture backend.

## Post-Design Constitution Check

All gates remain **PASS**. The UI remains presentational, consumes typed IPC,
and never accesses sources, credentials, raw descriptions, or the matching
engine.

## Gate Decision

**PASS.** Ready for `/speckit.tasks`.

## Next Steps

1. Run `/speckit.tasks` for `002`.
2. Implement the read-only feed/drawer first; keep apply/ignore hidden.
3. Wire the profile fixture view and Analytics shell.
4. Populate Analytics in `003`.
