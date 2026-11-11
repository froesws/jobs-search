# Implementation Plan: LLM Provider Configuration Popup

**Branch**: `014-provider-config-popup` | **Date**: 2026-11-11 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/014-provider-config-popup/spec.md`

## Summary

Add an in-app provider configuration surface so the operator can manage LLM credentials, the
provider chain, per-provider attempt budgets, and model selection without editing `.env` or
restarting the app. Credentials move to the OS credential store, non-secret settings move to a
new `app_settings` SQLite table, and the runtime configuration becomes mutable so saves apply to
the next run. The existing top-bar `ProviderIndicator` becomes a button with a grouped model menu
that applies a session-only override, and a new gear button opens the full configuration dialog.
Local Ollama is added as a first-class provider, gated behind an explicit opt-in.

## Technical Context

**Language/Version**: Rust 1.90 (edition 2024); TypeScript 7.0.2 with React 19.3.1

**Primary Dependencies**: Tauri 2.12.5, `reqwest` 0.13 (rustls), `rusqlite` 0.41 (bundled),
`serde`/`serde_json`, MUI 9.5 + Emotion. **New**: `keyring` 3.6 with the default `v1` feature,
which selects `zbus-secret-service-keyring-store` on Linux.

**Storage**: SQLite (`app_settings` key/value table, schema v7) for non-secret settings; the OS
credential store for API keys; `.env` remains the factory default and consented plaintext
fallback. Model catalogs are cached as JSON values in `app_settings`.

**Testing**: `cargo test` (unit tests plus `wiremock`-based contract tests in
`src-tauri/tests/llm_provider_contract.rs`), `vitest` + Testing Library for components and
viewmodels, `tsc --noEmit`, `oxlint`, `prettier --check`.

**Target Platform**: Linux desktop (Tauri v2) with a system Secret Service daemon available in
the normal desktop session; headless sessions fall back to the consented `.env` path.

**Project Type**: desktop application (Rust backend, React SPA frontend).

**Performance Goals**: the configuration dialog renders the model list in under 1 second with no
network access; opening the dialog performs zero provider requests; a catalog refresh is a
single explicit request per provider.

**Constraints**: credentials must never be logged, persisted to SQLite, or rendered after save;
saved settings must apply to the next run without restarting; existing fatal-vs-retryable error
classification must be preserved; the canonical provider order stays a hard allow-list.

**Scale/Scope**: 4 providers, 1 new SQLite table, 1 new provider adapter, 1 new dialog component,
1 modified indicator component, 8 new Tauri commands, 1 new crate dependency.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

`.specify/memory/constitution.md` is still the unfilled template (placeholder principles, no
ratified version), so it defines no enforceable gates. The effective project gates come from
`.opencode/rules/rules.md` and the repository conventions; they are evaluated below.

| Gate (source) | Status | Notes |
|---------------|--------|-------|
| Stack guardrails: Tauri v2 + React 19 + TS + Vite, no Next.js/App Router/Tailwind/Radix (rules) | PASS | No new frontend framework; MUI/Emotion only |
| MUI 9 + Emotion for UI (rules) | PASS | Dialog, Chip, Menu, TextField, Switch, Select from MUI |
| Privileged work in Rust: filesystem, SQLite, HTTP (rules) | PASS | Keychain, SQLite, HTTP and model listing all live in the Rust backend |
| Call Tauri through typed wrappers in `src/lib/ipc.ts` (rules) | PASS | New commands get typed wrappers; no direct `invoke()` in components |
| Keep TypeScript strict, avoid `any` (rules) | PASS | New IPC types added to `src/lib/types.ts` |
| Never log/expose API key or secrets (rules) | PASS | FR-005; credentials never enter logs, DB, or Debug output |
| Errors serialized as `{code, message}` (rules) | PASS | New commands return `Result<T, ErrorPayload>` |
| No new dependency without concrete need (rules) | PASS (justified) | `keyring` is the concrete need for FR-006; see Complexity Tracking |
| Do not add a new directory without concrete need (rules) | PASS | New modules reuse `db/`, `llm/`, `commands/`, `src/components/` |

**Gate result**: PASS. One dependency addition requires justification (recorded below).

## Project Structure

### Documentation (this feature)

```text
specs/014-provider-config-popup/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (Tauri command contracts)
└── tasks.md             # Phase 2 output (/speckit.tasks — not created here)
```

### Source Code (repository root)

```text
src/
├── components/
│   ├── ProviderIndicator.tsx        # modified: Chip -> Button + grouped model menu
│   ├── ProviderConfigDialog.tsx     # new: settings dialog with per-provider cards
│   └── TopBar.tsx                   # modified: gear button next to the indicator
├── lib/
│   ├── ipc.ts                       # modified: typed wrappers for the new commands
│   └── types.ts                     # modified: provider config/session types
└── viewmodels/
    ├── useProviderConfig.ts          # new: shared load/save/session-selection state
    └── useProviderConfigViewModel.ts # new: dialog state, dirty tracking, test/refresh actions

src-tauri/
├── src/
│   ├── config/
│   │   ├── env.rs                   # modified: add ollama fields, accept "ollama" provider
│   │   └── provider_settings.rs     # new: resolve effective settings (env -> DB -> session)
│   ├── credentials/
│   │   └── mod.rs                   # new: OS credential store wrapper + .env fallback consent
│   ├── db/
│   │   ├── migrations.rs            # modified: SCHEMA_V7 adds app_settings
│   │   └── settings.rs              # new: get/put/delete settings rows
│   ├── llm/
│   │   ├── mod.rs                   # modified: build plans from resolved settings, ollama
│   │   ├── ollama.rs                # new: Ollama provider (OpenAI-compatible)
│   │   ├── models.rs                # new: per-provider model catalog fetch + filtering
│   │   └── provider.rs              # modified: decouple attempts from without_fallback()
│   ├── commands/
│   │   └── config.rs                # modified: new provider configuration commands
│   ├── state.rs                     # modified: mutable config + session selection
│   └── lib.rs                       # modified: register new commands
└── tests/
    └── llm_provider_contract.rs     # modified: model-listing and Ollama contract tests
```

**Structure Decision**: keep the existing single-repository Tauri layout. Backend work lands in
the existing `config/`, `db/`, `llm/`, and `commands/` boundaries plus one new `credentials/`
module (no existing module owns secret storage); frontend work lands in `src/components/` and
`src/viewmodels/`, matching the project rule that components stay presentational and stateful
logic lives in viewmodels. `useProviderConfig` is instantiated in `DashboardView` and passed by
props to `TopBar`, mirroring `useDashboardState`.

## Verification Requirements

These are mandated by `AGENTS.md` and are part of the feature, not optional polish:

- **Migration contract test**: open an old schema and apply `SCHEMA_V7` (`AGENTS.md` §8/§10).
- **Contract schemas**: update `src-tauri/tests/fixtures/contracts/` together with the wire shapes
  for the new commands and the model-listing responses (`AGENTS.md` §10).
- **Doc comments**: new public items need doc comments; do not silence `#![warn(missing_docs)]`
  globally (`AGENTS.md` §8/§13).
- **Database lock discipline**: never hold the `Db` mutex across an `.await`; credential-store and
  HTTP calls happen outside the lock (`AGENTS.md` §8/§13).
- **Full gate**: `./scripts/verify.sh` is the real gate; `npm test` alone is not (`AGENTS.md` §5).
- **Rulebook updates**: `AGENTS.md` §2/§3/§8/§9 and `README.md` must be updated in the same change
  (`AGENTS.md` §15).

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| New dependency: `keyring` 3.6 | FR-006 requires an OS-backed credential store; this is the only crate that covers keychain/Credential Manager/Secret Service uniformly | Storing plaintext in `.env` or SQLite was rejected: it degrades security silently and contradicts the project's documented "no credential values in any table" rule |
| `AppState.config` becomes mutable (`RwLock<AppConfig>`) | FR-023 requires saved settings to apply without restart | Re-reading and rebuilding config on every command was rejected: it spreads I/O across call sites and still needs a lock for the session override |
| Second configuration source (SQLite over `.env`) | FR-020/FR-021 require persisted, operator-editable settings | Editing `.env` from the app was rejected: it makes the app rewrite its own bootstrap file and breaks hand-edited setups |

## Post-Design Constitution Re-check

*Re-evaluated after Phase 1 design (data-model, contracts, quickstart).*

| Gate | Status | Evidence |
|------|--------|----------|
| No secrets in SQLite, logs, telemetry, or exports | PASS | `data-model.md` defines no credential column; `contracts/commands.md` never returns a key; quickstart Scenario 10 verifies it |
| Privileged work stays in Rust | PASS | Credential store, SQLite, HTTP listing and Ollama adapter are all backend modules |
| Typed IPC only | PASS | Every new command has a typed wrapper in `src/lib/ipc.ts` and a type in `src/lib/types.ts` |
| MUI/Emotion only | PASS | Dialog, Menu, Switch, TextField, Select, Chip |
| Additive, reversible migration | PASS | `SCHEMA_V7` only creates `app_settings`; no table altered |
| No new directory without need | PASS | New modules live in existing `config/`, `db/`, `llm/`, `commands/`; one new `credentials/` module is required because no existing module owns secret storage |
| Tests for new behaviour, including failure paths | PASS | `research.md` §10 lists unit, contract, and component tests including degraded-refresh and fatal-error paths |

**Result**: PASS. No unjustified violations remain.
