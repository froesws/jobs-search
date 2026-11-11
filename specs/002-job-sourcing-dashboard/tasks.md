---

description: "Task list for Job Sourcing Dashboard & Application Tracker"
---

# Tasks: Job Sourcing Dashboard & Application Tracker

**Input**: Design documents from `/specs/002-job-sourcing-dashboard/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`,
`contracts/`, `quickstart.md`

**Tests**: Required by the plan, constitution, and project rules. Add the test
task before its implementation task and confirm it fails first.

**Organization**: Tasks are grouped by user story. The plan's agreed first
milestone is the read-only four-tab shell, approved New Jobs feed, and detail
drawer backed by `001`; resume/profile editing, apply/ignore, manual approval, and
audit follow after the matching `001` US1/US4 backend work exists. The frontend
never reimplements ingestion, matching, or persistence.

**Current state**: The frontend does not exist yet (`src/`, `index.html`,
`tsconfig.json`, and `vite.config.ts` are missing), and `package.json` has no
React, MUI, Vite, or Vitest dependencies. All tasks start unchecked.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete
  task)
- **[Story]**: `[US1]`–`[US5]`
- Every task includes an exact file path.

## Path Conventions

- Frontend: `src/` with `lib/`, `viewmodels/`, `views/`, `components/`, `theme/`,
  `test/`
- Frontend tooling: `index.html`, `vite.config.ts`, `tsconfig.json`,
  `package.json`
- Backend contract gaps: `src-tauri/src/commands/`, `src-tauri/src/lib.rs`
- Feature docs/contracts: `specs/002-job-sourcing-dashboard/`

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Scaffold the React/Vite/MUI frontend and its test tooling.

- [x] T001 Create the Vite SPA entry points `index.html`, `src/main.tsx`, and `src/App.tsx` with a root element and strict-mode bootstrap.
- [x] T002 Add React `19.3.1`, React DOM, MUI `9.5`, Emotion, `@tauri-apps/api`, `@tauri-apps/plugin-dialog`, Vitest, jsdom, and Testing Library to `package.json`; add `dev`, `build`, `test`, `test:watch`, and `typecheck` scripts.
- [x] T003 [P] Create `tsconfig.json` with `strict`, `noUncheckedIndexedAccess`, `noImplicitOverride`, and the `@/*` path alias, plus `vite.config.ts` with React and Vitest/jsdom configuration.
- [x] T004 [P] Create the MUI theme and design tokens in `src/theme/theme.ts` and `src/theme/tokens.ts` (score colors, N/M colors, spacing, typography); do not add a second UI system.
- [x] T005 [P] Create the Vitest setup in `src/test/setup.ts` (Testing Library cleanup, jest-dom matchers, `matchMedia`/`ResizeObserver` stubs) and deterministic fixtures in `src/test/fixtures.ts`.
- [x] T006 [P] Create DTOs in `src/lib/types.ts` mirroring `specs/001-career-job-sourcing/contracts/tauri-commands.md`: `JobSummary`, `JobDetail`, `PaginatedJobs`, `CandidateProfile`, `JobApplication`, `JobEvaluation`, `SearchFilters`, `SearchMetrics`, `JobTab`, `EvaluationStatus`, `LifecycleState`, `WorkModality`, and `Seniority`.
- [x] T007 [P] Create backend error mapping in `src/lib/errors.ts` for `{ code, message }` into a typed `AppError` with safe user-facing messages and retryability.
- [x] T008 [P] Create formatting helpers in `src/lib/format.ts` for score display, N/M ratio (`0/0` as "No requirements"), dates, modality and seniority labels, and safe external URLs.
- [x] T009 Create typed IPC wrappers in `src/lib/ipc.ts` for `cmd_get_profile`, `cmd_trigger_search`, `cmd_get_jobs`, and `cmd_get_job`; no component may call `invoke()` directly.
- [x] T010 Create the application shell in `src/App.tsx`, `src/views/DashboardView.tsx`, `src/components/TopBar.tsx`, and `src/components/TabBar.tsx` with the exact labels `New Jobs`, `My Applications`, `My Profile`, and `Analytics`.
- [x] T011 [P] Create the shared state components `src/components/LoadingState.tsx`, `src/components/EmptyState.tsx`, and `src/components/ErrorState.tsx` with accessible roles, safe messages, and retry actions.
- [x] T012 Add `tauri-plugin-dialog` to `src-tauri/Cargo.toml`, register it in `src-tauri/src/lib.rs`, and grant dialog permissions in `src-tauri/capabilities/default.json` so resume upload can select a local PDF path.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Session state, ViewModels, job presentation, and missing backend
contract commands used by every story.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [x] T013 Create session-level tab and filter state in `src/viewmodels/useDashboardState.ts` so tab switches preserve filters and do not mix tab content.
- [x] T014 Implement `SearchFilters` validation and defaults in `src/lib/filters.ts`: timeframe, seniority, stack keyword, modalities, and clear validation messages for contradictory filters.
- [x] T015 Implement `useJobsViewModel` in `src/viewmodels/useJobsViewModel.ts` with `idle|loading|ready|empty|error`, items, total, limit, offset, filters, `refresh()`, `setFilters()`, `nextPage()`, `previousPage()`, and `openJob()`.
- [x] T016 Implement `useJobDetailViewModel` in `src/viewmodels/useJobDetailViewModel.ts` with drawer open/close, selected ID, detail state, and retry.
- [x] T017 [P] Implement `src/components/JobCard.tsx`, `src/components/ScoreBadge.tsx`, and `src/components/RequirementRatio.tsx` using MUI Card, Chip, Badge, and LinearProgress primitives.
- [x] T018 [P] Implement `src/components/JobDrawer.tsx` with the right-side MUI Drawer, focus trap, close action, clean description, structured requirements, matched/missing skills, and external link.
- [x] T019 Wire loading, empty, error, and partial-failure states into `src/views/DashboardView.tsx` using `LoadingState`, `EmptyState`, and `ErrorState`.
- [x] T020 Add the `cmd_list_resumes` backend contract gap in `src-tauri/src/commands/profile.rs` and register it in `src-tauri/src/lib.rs` (depends on `001` FR-PRF-005) and add its typed wrapper plus `ResumeSummary` in `src/lib/types.ts` and `src/lib/ipc.ts`.
- [x] T021 Add the `cmd_get_active_provider` backend contract gap in `src-tauri/src/commands/config.rs` and register it in `src-tauri/src/lib.rs`; return only provider name and model, never credentials.
- [x] T022 Add accessibility behavior in `src/components/TabBar.tsx`, `src/components/JobDrawer.tsx`, and `src/views/DashboardView.tsx`: keyboard tab navigation, drawer focus management, labelled controls, and live-region status announcements.
- [x] T023 Add the mocked IPC test harness in `src/test/ipc-mock.ts` for deterministic command responses and failures, including `{ code, message }` errors.

**Checkpoint**: The shell renders, IPC is typed, and every tab can show loading/empty/error states without story-specific logic.

---

## Phase 3: User Story 1 - Manage resumes and keep the profile current (Priority: P1)

**Goal**: Resume upload/list/delete, automatic recalculation feedback, and an
editable My Profile tab.

**Independent Test**: Upload a fixture PDF and verify the resume appears in the
list and the profile refreshes; delete it and verify recalculation; simulate a
recalculation failure and verify the last valid profile remains visible; upload
the same file twice and verify the duplicate message.

### Tests for User Story 1

- [x] T024 [P] [US1] Add failing `useProfileViewModel` tests in `src/viewmodels/useProfileViewModel.test.ts` for loading, fixture/profile ready, save success, save error, and stale notice.
- [x] T025 [P] [US1] Add failing resume manager tests in `src/components/ResumeManager.test.tsx` for upload selection, list rendering, delete confirmation, duplicate warning, unsupported-file error, and last-resume warning.
- [x] T026 [P] [US1] Add failing Profile view tests in `src/views/ProfileView.test.tsx` for editable technology tags, accepted cities, dealbreakers, modalities, salary threshold, work authorization, and the "no resumes" blocking state.

### Implementation for User Story 1

- [x] T027 [US1] Implement `useProfileViewModel` in `src/viewmodels/useProfileViewModel.ts` with `idle|loading|ready|empty|error`, profile, resumes, recalculation state, `refresh()`, `save()`, `uploadResume()`, and `deleteResume()`.
- [x] T028 [US1] Implement the editable `src/views/ProfileView.tsx` with MUI Autocomplete/Chip for acceptable technologies and accepted cities, plus dealbreakers, preferred modalities, salary threshold, and work-authorization fields.
- [x] T029 [US1] Implement `src/components/ResumeManager.tsx` in the top bar area: upload via the dialog plugin, list stored resumes with metadata, delete with confirmation, and show parse status.
- [x] T030 [US1] Implement recalculation feedback in `src/views/ProfileView.tsx` and `src/components/ResumeManager.tsx`: in-progress indicator, failure message, retained last valid profile, and a retry action.
- [x] T031 [US1] Implement the last-resume state and duplicate-resume messaging in `src/viewmodels/useProfileViewModel.ts` and `src/components/ResumeManager.tsx`; block "Refresh Search" with a clear message while no resumes remain.
- [x] T032 [US1] Wire `cmd_save_profile` into `src/lib/ipc.ts` and `useProfileViewModel`, and show that changes apply on the next search with prior evaluations marked stale.
- [x] T033 [US1] Add the US1 integration test in `src/views/ProfileView.test.tsx` covering upload → recalculation → edit → save → delete → last-valid-profile retention using the mocked IPC harness.

**Checkpoint**: US1 works with the `001` profile backend; no profile or resume data is invented in the frontend.

---

## Phase 4: User Story 2 - Configure searches and ingest jobs (Priority: P2)

**Goal**: Top-bar search controls, validation, search execution feedback, partial
source failures, and empty/failed runs.

**Independent Test**: Set timeframe "Last 7 Days", seniority "Senior", stack
"Rust", modality "Remote", run a search with one failing source, and verify the
feed shows successful results, a visible partial-failure notice, and a preserved
run record.

### Tests for User Story 2

- [x] T034 [P] [US2] Add failing search-control tests in `src/components/TopBar.test.tsx` for timeframe, seniority, stack, modality, and the exact `Refresh Search` label.
- [x] T035 [P] [US2] Add failing filter-validation tests in `src/lib/filters.test.ts` for empty stack, contradictory modality/seniority combinations, and invalid-timeframe prevention.
- [x] T036 [P] [US2] Add failing search-run tests in `src/viewmodels/useJobsViewModel.test.ts` for loading, partial source failure, all-sources-failed, zero results, and disabled trigger while running.

### Implementation for User Story 2

- [x] T037 [US2] Implement the top-bar search controls in `src/components/TopBar.tsx` with MUI Select/TextField controls for timeframe, seniority, stack, and modalities.
- [x] T038 [US2] Implement `Refresh Search` execution in `src/components/TopBar.tsx` and `src/viewmodels/useJobsViewModel.ts`: validate, disable while running, show progress, and refresh the feed.
- [x] T039 [US2] Implement the partial-source-failure banner in `src/views/NewJobsView.tsx` from `SearchMetrics.source_failures` without hiding successful jobs.
- [x] T040 [US2] Implement all-sources-failed, zero-result, and evaluation-error states in `src/views/NewJobsView.tsx` with a clear message and audit reference.
- [x] T041 [US2] Implement unknown-posted-time handling in `src/lib/format.ts` and `src/components/JobCard.tsx`: label the age Unknown and do not imply it matches a strict timeframe.
- [x] T042 [US2] Implement large-result-set navigation in `src/viewmodels/useJobsViewModel.ts` and `src/views/NewJobsView.tsx` with pagination or windowed rendering; never silently truncate.
- [x] T043 [US2] Add the US2 integration test in `src/views/NewJobsView.test.tsx` covering filter change → search → partial failure → preserved results → empty run.

**Checkpoint**: The operator can run a search and always sees a truthful success, partial, empty, or failure state.

---

## Phase 5: User Story 3 - Evaluate jobs with explainable scores (Priority: P3)

**Goal**: Display backend evaluation status, integer score, 40/25/20/15 breakdown,
N/M counts, matched/missing skills, provider/model, and evaluation errors without
client-side scoring.

**Independent Test**: Render jobs with APPROVED, PENDING, REJECTED, and ERROR
evaluations plus a job with no requirements; verify the badge, N/M, breakdown,
reasons, and "No requirements" state match the DTOs and no score is computed in
the browser.

### Tests for User Story 3

- [x] T044 [P] [US3] Add failing score-badge/N-M tests in `src/components/ScoreBadge.test.tsx` and `src/components/RequirementRatio.test.tsx` for 0–100 integers, colors, and `0/0` rendering.
- [x] T045 [P] [US3] Add failing drawer breakdown tests in `src/components/JobDrawer.test.tsx` for the four dimensions, matched/missing keywords, hard-filter reasons, and exact/inferred labels.
- [x] T046 [P] [US3] Add failing evaluation-error tests in `src/viewmodels/useJobDetailViewModel.test.ts` for ERROR status, no score, no approval, and retry.

### Implementation for User Story 3

- [x] T047 [US3] Render evaluation status, score, N/M, and the four-dimension breakdown in `src/components/JobDrawer.tsx` strictly from `JobDetail` DTO fields.
- [x] T048 [US3] Implement the no-requirements state in `src/lib/format.ts`, `src/components/RequirementRatio.tsx`, and `src/components/JobDrawer.tsx` as `0/0` or "No requirements" without implying a perfect match.
- [x] T049 [US3] Display the evaluation provider/model and hard-filter reasons in `src/components/JobDrawer.tsx` for auditability.
- [x] T050 [US3] Add the below-threshold toggle in `src/views/NewJobsView.tsx` using `JobTab.BELOW_THRESHOLD` and expose manual approval only when `cmd_approve_job` exists.
- [x] T051 [US3] Implement evaluation-error retry in `src/viewmodels/useJobDetailViewModel.ts` and `src/components/JobDrawer.tsx` without assigning a score locally.
- [x] T052 [US3] Remove any accidental client-side scoring or threshold comparison from `src/lib/`, `src/viewmodels/`, and `src/components/`; the UI displays backend decisions only.
- [x] T053 [US3] Add the US3 integration test in `src/components/JobDrawer.test.tsx` covering approved, rejected, error, and no-requirement DTOs.

**Checkpoint**: Every visible score and reason is traceable to a stored `001` evaluation.

---

## Phase 6: User Story 4 - Review matches and track applications (Priority: P4)

**Goal**: Ranked card grid, detail drawer, apply/ignore/restore, My Applications
table, audit panel, tab state preservation, and zero-result behavior.

**Independent Test**: Load approved, applied, ignored, rejected, and error jobs;
verify card order and contents, drawer details, apply/ignore transitions, the
applications table, audit totals, tab persistence, and the zero-approved state.

**⚠️ Depends on**: `001` US4 `cmd_apply_job`, `cmd_ignore_job`, `cmd_restore_job`,
`cmd_approve_job`, and `cmd_get_audit`.

### Tests for User Story 4

- [x] T054 [P] [US4] Add failing card-grid tests in `src/views/NewJobsView.test.tsx` for descending score order, title/company/location/summary/N-M/score fields, and exclusion of rejected/error jobs.
- [x] T055 [P] [US4] Add failing apply/ignore tests in `src/components/JobDrawer.test.tsx` and `src/viewmodels/useJobDetailViewModel.test.ts` for immediate drawer update, removal from New Jobs, and append-only lifecycle feedback.
- [x] T056 [P] [US4] Add failing applications-table tests in `src/views/ApplicationsView.test.tsx` for query, filtering, sorting, applied date, score, N/M, and detail inspection.
- [x] T057 [P] [US4] Add failing audit and zero-result tests in `src/views/NewJobsView.test.tsx` for zero approved jobs, source failures, rejection reasons, and run identification.

### Implementation for User Story 4

- [x] T058 [US4] Implement `src/viewmodels/useApplicationsViewModel.ts` with `idle|loading|ready|empty|error`, items, filters, sorting, `refresh()`, and `openJob()`.
- [x] T059 [US4] Implement `src/views/ApplicationsView.tsx` with an MUI DataGrid/Table, query input, status badge, applied date, score, N/M, sorting, and detail drawer.
- [x] T060 [US4] Implement apply, ignore, and restore actions in `src/lib/ipc.ts`, `src/components/JobDrawer.tsx`, and `src/viewmodels/useJobDetailViewModel.ts`; update the drawer and both feeds immediately.
- [x] T061 [US4] Implement the run audit panel in `src/components/AuditPanel.tsx` using `cmd_get_audit`: totals, duplicates, rejection reasons, source failures, approved count, provider/model, and run start/end.
- [x] T062 [US4] Implement tab and filter preservation in `src/views/DashboardView.tsx` so switching tabs never mixes content or resets search filters.
- [x] T063 [US4] Enforce card/drawer consistency in `src/components/JobCard.tsx` and `src/components/JobDrawer.tsx` for the same job ID, score, N/M, and post-action state.
- [x] T064 [US4] Implement external-link handling in `src/components/JobDrawer.tsx` using the Tauri opener/dialog-safe path; report broken URLs without losing stored data.
- [x] T065 [US4] Add the US4 integration test in `src/views/ApplicationsView.test.tsx` covering apply → New Jobs removal → applications row → audit reconciliation → zero-result state.

**Checkpoint**: US4 is functional once the `001` lifecycle commands land; before that, T060/T061 stay behind a feature guard and the actions remain hidden.

---

## Phase 7: User Story 5 - Switch the language-model provider locally (Priority: P5)

**Goal**: Show the active provider/model in the top bar and surface configuration
errors without exposing credentials.

**Independent Test**: Mock the active provider as DeepSeek, verify the indicator;
change it to Gemini and verify the indicator updates after refresh; simulate an
invalid configuration and verify a clear error with no credential text.

### Tests for User Story 5

- [x] T066 [P] [US5] Add failing provider-indicator tests in `src/components/TopBar.test.tsx` for provider/model display, refresh, missing configuration, and absence of credentials.
- [x] T067 [P] [US5] Add failing config-error tests in `src/views/NewJobsView.test.tsx` for a provider configuration error that blocks evaluation without producing a partial score.

### Implementation for User Story 5

- [x] T068 [US5] Implement `src/components/ProviderIndicator.tsx` and wire it into `src/components/TopBar.tsx` using the `cmd_get_active_provider` wrapper.
- [x] T069 [US5] Display provider configuration errors and retry guidance in `src/views/NewJobsView.tsx` and `src/components/ProviderIndicator.tsx`; never render keys, tokens, or prompt content.
- [x] T070 [US5] Document provider switching through local `.env`/configuration in `specs/002-job-sourcing-dashboard/quickstart.md`, including the optional restart note and the no-silent-fallback behavior.

**Checkpoint**: The operator can see which provider will be used and why an operation failed, with no secrets in the UI.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, verification, accessibility, performance, and privacy.

- [x] T071 [P] Update `README.md` with frontend setup, scripts, the four tabs, the read-only first milestone, and the IPC boundary.
- [x] T072 [P] Update `specs/002-job-sourcing-dashboard/quickstart.md` with the implemented paths, commands, and the exact first-milestone scope.
- [x] T073 Run `npm run format:check`, `npm run lint`, `npm run typecheck`, `npm test`, and `./scripts/verify.sh`; fix failures and report skipped checks explicitly.
- [x] T074 [P] Add an accessibility audit test in `src/views/DashboardView.test.tsx` for tab labels, keyboard navigation, drawer focus, and status announcements.
- [x] T075 [P] Add the 1,000-record performance test in `src/views/NewJobsView.test.tsx` using pagination/windowing and assert no IPC call happens during render.
- [x] T076 [P] Audit `src/lib/`, `src/viewmodels/`, `src/views/`, and `src/components/` for `description_raw`, credentials, provider keys, and direct `invoke()` calls; fix any finding.
- [x] T077 Run the full `002` quickstart scenario list against the fixture backend and record evidence in `specs/002-job-sourcing-dashboard/quickstart.md`.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: Can start immediately.
- **Foundational (Phase 2)**: Depends on Setup; blocks all stories.
- **US1 (Phase 3)**: Depends on Foundational and the `001` profile/resume backend.
- **US2 (Phase 4)**: Depends on Foundational and the `001` search/ingestion backend.
- **US3 (Phase 5)**: Depends on Foundational and `001` evaluation DTOs.
- **US4 (Phase 6)**: Depends on US3 display plus `001` US4 lifecycle commands.
- **US5 (Phase 7)**: Depends on Foundational and `cmd_get_active_provider`.
- **Polish (Phase 8)**: Depends on the desired stories.

### User Story Dependencies

- **US1**: Independent of US2–US5 after Foundational; needs `001` US1.
- **US2**: Can run without US1; needs `001` US2.
- **US3**: Can run without US1/US2 using mocked DTOs; display-only.
- **US4**: Needs US3 display components and `001` US4.
- **US5**: Independent; only displays provider configuration.

### Within Each Story

- Tests first; confirm they fail.
- Types/helpers before ViewModels; ViewModels before Views.
- Run `npm run typecheck`, `npm test`, and `./scripts/verify.sh` before each commit.
- One task per commit with the task ID.

## Parallel Opportunities

- T003–T008 can run in parallel (config, theme, test setup, DTOs, errors, format).
- T017–T018 can run in parallel (card vs. drawer).
- T024–T026, T034–T036, T044–T046, T054–T057, and T066–T067 are independent test files.
- T059–T064 touch different components and can split across developers.
- T071, T072, T074, T075, and T076 are parallel polish tasks.

## Parallel Example: US2

```text
Task: T037 Implement top-bar search controls in src/components/TopBar.tsx
Task: T039 Implement the partial-source-failure banner in src/views/NewJobsView.tsx
Task: T042 Implement pagination in src/viewmodels/useJobsViewModel.ts
```

## Implementation Strategy

### Milestone A (plan): read-only dashboard

1. Complete Setup and Foundational (T001–T023).
2. Complete the US2 search UI and US3 evaluation display (T034–T053).
3. Complete the read-only feed, drawer, and tabs from US4 (T054, T058, T059, T062–T064).
4. Validate against the `001` fixture backend.

### Milestone B: full workflow

1. US1 resume/profile editing when `001` US1 lands (T024–T033).
2. US4 apply/ignore/audit when `001` US4 lands (T055, T060, T061, T065).
3. US5 provider indicator (T066–T070).
4. Polish and full quickstart (T071–T077).

## Notes

- No direct `invoke()` outside `src/lib/ipc.ts`.
- No client-side scoring, filtering, ingestion, or persistence.
- No `description_raw`, credentials, or provider secrets in UI state or renders.
- The Analytics tab is a shell in `002`; `003` replaces it.
- Spec-level P1 is US1, but the plan deliberately ships the read-only feed first.
