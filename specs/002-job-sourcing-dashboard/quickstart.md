# Quickstart: Job Sourcing Dashboard

**Feature**: `002-job-sourcing-dashboard`  
**Date**: 2026-11-10

The dashboard is implemented against the `001` backend. It includes the
read-only feed/drawer, resume management, editable profile, apply/ignore/manual
approval, execution audit, and the active provider indicator.

## Prerequisites

- Linux with Tauri v2 build dependencies.
- Node.js `24+`, Rust `1.90+`.
- `001` fixture profile at `src-tauri/tests/fixtures/profile.json`.
- `.env` with `JOBS_SEARCH_FIXTURE_PROFILE=/absolute/path/to/profile.json` and
  `RUST_LOG=info` when using the fixture instead of uploaded resumes.
- Provider keys plus `LLM_PROVIDER_ORDER` and `LLM_ALLOW_FALLBACK` for live
  extraction/evaluation.

## Setup

```bash
npm install
npm run dev
```

For the desktop shell:

```bash
npm run tauri dev
```

## Validation scenarios

### 1. Four tabs

Expected: the tab bar shows exactly `New Jobs`, `My Applications`, `My Profile`,
and `Analytics`. Switching tabs preserves filter state and does not mix content.

### 2. Approved feed

Trigger a search from New Jobs. Expected: approved jobs render as MUI cards sorted
by score, each with title, company, location/modality, snippet, score badge, and
N/M ratio. No direct `invoke()` call is made from a component.

### 3. Detail drawer

Click a card. Expected: the drawer opens with clean description, structured
requirements, matched/missing skills, external URL, and the action area. The
action area is hidden until P4.

### 4. Profile fixture

Open My Profile. Expected: the fixture profile is shown read-only; target roles,
seniority, cities, and acceptable technologies are visible. Editing is deferred
to P1.

### 5. Applications and Analytics shells

Expected: My Applications shows an empty state; Analytics shows a “Coming in
003” state. Apply/ignore buttons are absent.

### 6. Loading, empty, error, and partial failure

Mock IPC failures and empty responses. Expected: each tab shows a clear
loading/empty/error state with a retry action; a partial source failure is
surfaced without hiding successful results.

### 7. Tests

```bash
npm run typecheck
npm test
./scripts/verify.sh
```

Expected: ViewModel tests cover loading/ready/empty/error/pagination/drawer;
component tests cover labels, cards, drawer, tables, profile editing, resume
management, lifecycle actions, audit, and the provider indicator.

## Implemented workflow

### Resumes and profile

- `Upload resume` selects a text-based PDF through the native dialog. The
  backend stores it under app-data `resumes/` with `0600`, extracts the profile,
  merges list-valued fields, and recalculates the profile.
- `Resumes (n)` lists stored resumes and allows deletion. Deleting the last
  resume preserves the profile, marks it unavailable, and blocks new searches
  with `RESUME_REQUIRED`.
- `My Profile` edits acceptable technologies, accepted cities, target roles,
  seniority, modalities, dealbreakers, salary, and work authorization. Saving
  increments the profile version and marks prior evaluations stale.

### Lifecycle and audit

- An APPROVED card drawer exposes `I Applied` and `Ignore`.
- PENDING cards expose manual approval for scored jobs; the original score and
  breakdown are preserved and `approval_source` becomes `manual`.
- IGNORED jobs can be restored.
- `View audit` opens the run audit with totals, rejection reasons, source
  failures, provider/source breakdowns, and score distribution.

### Provider indicator

The top bar shows the first configured provider/model and whether fallback is
enabled. The indicator never renders credentials. Provider configuration is read
from the local `.env`; restart the app after changing it. With
`LLM_ALLOW_FALLBACK=false` (default), a transient provider error is reported
without switching provider. With `true`, fallback is recorded and disclosed.
