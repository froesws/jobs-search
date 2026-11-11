# Research: Job Sourcing Dashboard

**Feature**: `002-job-sourcing-dashboard`  
**Date**: 2026-11-10

## Decision: MVVM with hooks

**Decision**: View = MUI component, ViewModel = `use*ViewModel` hook, Model =
typed Tauri IPC wrappers and DTOs.

**Rationale**: This is React-idiomatic MVVM, testable with Testing Library, and
does not require a state library or class-based reactivity layer. It keeps views
presentational and all loading/error/action logic in ViewModels.

**Alternatives considered**:
- Class-based ViewModels with observable state: more boilerplate, needs a
  subscription layer.
- Zustand/TanStack Query: prohibited without explicit approval and unnecessary
  for four local tabs.

## Decision: UI stack

**Decision**: React 19.3.1, TypeScript 7.0.2 strict, Vite 8.3.6, MUI 9.5 +
Emotion, Lucide/Recharts only if already approved. No Tailwind or second UI
system.

**Rationale**: The project rules require MUI and the current stack. MUI provides
the top bar, tabs, cards, table, drawer, badges, and progress components without
a second design system.

## Decision: IPC boundary

**Decision**: All Tauri calls go through `src/lib/ipc.ts`; components never call
`invoke()` directly. The wrapper maps backend errors to typed frontend errors.

**Rationale**: The project rules require typed IPC wrappers. A single boundary
makes ViewModel tests easy to mock and prevents presentation code from depending
on command names.

## Decision: First milestone read-only

**Decision**: Implement the four-tab shell, approved New Jobs feed, and detail
drawer. Show the fixture profile in My Profile. Keep My Applications empty and
hide “I Applied”/“Ignore” until the P4 backend exists.

**Rationale**: The first backend milestone is P2+P3 with a fixture profile. Dead
actions or fake apply states would mislead the operator.

## Decision: ViewModel state model

**Decision**: Each ViewModel exposes `idle`, `loading`, `ready`, `empty`, and
`error` states, plus data/actions. The job feed adds pagination and filter state;
the drawer adds selected job/detail state.

**Rationale**: These states cover the backend failure/empty scenarios and make
the UI deterministic in tests.

## Decision: Testing

**Decision**: Vitest + Testing Library for ViewModels and Views. Mock
`src/lib/ipc.ts` for deterministic data/errors. Add accessibility checks for tab
labels, drawer focus, and keyboard navigation.

**Rationale**: The UI is a thin presentation layer; ViewModel tests plus
component tests cover the contract without a full Tauri process.

## Decision: Analytics shell

**Decision**: Include the Analytics tab in the tab bar with a “Coming in 003”
shell. The real Skill Gap/Funnel UI is defined in `003`.

**Rationale**: The four-tab specification requires the tab, but 003 owns the
data model and content.

## Decision: Performance

**Decision**: Use MUI list virtualization/pagination for 1,000 records; avoid
rendering all cards at once. Keep IPC calls out of render loops.

**Rationale**: The spec requires 100 ms for 1,000 local records; pagination and
windowed rendering keep the feed responsive.
