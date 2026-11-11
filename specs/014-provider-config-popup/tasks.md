---

description: "Task list for the LLM provider configuration popup (credentials, provider chain, model selection, attempts, indicator menu)"

# Tasks: LLM Provider Configuration Popup

**Input**: Design documents from `/specs/014-provider-config-popup/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/`, `quickstart.md`

**Tests**: Required. `AGENTS.md` §10 mandates a test for every behavior change, including the
failure path. Test tasks are included per story.

**Organization**: Tasks are grouped by user story so each story is independently implementable and
testable.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: `[US1]`–`[US4]`, mapping to `spec.md`
- Every task names an exact file path

## Path Conventions

- Frontend: `src/components/`, `src/viewmodels/`, `src/lib/`, `src/views/`
- Backend: `src-tauri/src/{commands,config,credentials,db,llm}/`
- Rust tests: `src-tauri/tests/`, `src-tauri/tests/fixtures/contracts/`
- Feature docs: `specs/014-provider-config-popup/`

---

## Phase 1: Setup

**Purpose**: Dependencies and shared type surface.

- [ ] T001 Add `keyring = "3"` to `[dependencies]` in `src-tauri/Cargo.toml` (default `v1` feature selects zbus Secret Service on Linux) and confirm `cargo check --manifest-path src-tauri/Cargo.toml` passes.
- [ ] T002 [P] Add `OLLAMA_BASE_URL` (default `http://localhost:11434`) and `OLLAMA_MODEL` to `.env.example` and document them in `README.md`.
- [ ] T003 [P] Add `ProviderId`, `ProviderConfigView`, `ProviderSettingView`, `ModelOption`, `ModelCatalogView`, `ProviderTestResult`, `SessionSelection`, and `ProviderConfigInput` to `src/lib/types.ts`.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Storage, credential access, configuration resolution, and provider plumbing that every
user story depends on.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [ ] T004 Add `SCHEMA_V7` creating `app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL)` in `src-tauri/src/db/migrations.rs`, bump `SCHEMA_VERSION` to `7`, and apply it in `apply()`.
- [ ] T005 [P] Add a migration contract test that opens a v6 schema and applies v7 in `src-tauri/tests/migrations.rs`.
- [ ] T006 [P] Implement `get`, `put`, `delete`, and `list_prefix` in `src-tauri/src/db/settings.rs` using `Db::with_connection`, with unit tests for insert, upsert, delete, and missing-key behavior.
- [ ] T007 [P] Implement `src-tauri/src/credentials/mod.rs`: `get(provider)`, `set(provider, key)`, `delete(provider)`, `source(provider)`, a `discover_env_file` helper mirroring `discover_sources` in `src-tauri/src/config/env.rs`, and the consented `.env` write/remove path with `0600` permissions. Register `pub mod credentials;` in `src-tauri/src/lib.rs`. Unit-test with a fake store and a `tempfile` `.env`.
- [ ] T008 [P] Extend `AppConfig` in `src-tauri/src/config/env.rs` with `ollama_base_url` and `ollama_model`, parse `OLLAMA_BASE_URL`/`OLLAMA_MODEL`, and accept `"ollama"` in `validate_provider`. Add unit tests for the new fields and for `LLM_PROVIDER=ollama`.
- [ ] T009 Implement `src-tauri/src/config/provider_settings.rs` resolving effective settings in the order session → `app_settings` → `.env`/OS env → built-in defaults, and credentials as credential store → `.env`. Unit-test every precedence layer and the reset-to-default path.
- [ ] T010 Change `AppState.config` to `RwLock<AppConfig>` and add `session_selection: Mutex<Option<SessionSelection>>` in `src-tauri/src/state.rs`; update the read sites in `src-tauri/src/commands/search.rs`, `src-tauri/src/commands/profile.rs`, and `src-tauri/src/commands/config.rs` to take a read guard.
- [ ] T011 Change `LlmRouter::without_fallback()` in `src-tauri/src/llm/provider.rs` to truncate the plan list without forcing `attempts = 1`, and add a unit test proving the configured attempt count survives with fallback disabled.
- [ ] T012 Update `router_from_config` in `src-tauri/src/llm/mod.rs` to build plans from the resolved settings (order, per-provider attempts, OpenRouter list length) and to include the Ollama plan only when enabled.
- [ ] T013 [P] Implement `src-tauri/src/llm/ollama.rs` against `{base_url}/v1/chat/completions` reusing `transport::chat_completion`, and add a `wiremock` contract test in `src-tauri/tests/llm_provider_contract.rs`.
- [ ] T014 [P] Add `get_json` to `src-tauri/src/llm/transport.rs` mirroring `post_json` (sanitized errors, no URL in messages) with a unit test for non-2xx mapping.
- [ ] T015 Implement `src-tauri/src/llm/models.rs` with per-provider catalog fetch and chat-model filtering (Gemini `supportedGenerationMethods`, OpenRouter free-by-default with `includePaid`, DeepSeek `data[].id`, Ollama `/api/tags`), plus `wiremock` contract tests for each provider and for an empty filtered result.

**Checkpoint**: Storage, credentials, resolution, and provider plumbing are ready.

---

## Phase 3: User Story 1 - Configure credentials and the provider chain (Priority: P1) 🎯 MVP

**Goal**: The operator configures credentials and the provider chain from the app, and the change
applies to the next run without restarting.

**Independent Test**: Open the popup from the gear button, save a key, close, and confirm the
indicator shows the provider without a restart.

### Implementation for User Story 1

- [ ] T016 [P] [US1] Implement `cmd_get_provider_config` and `cmd_save_provider_config` in `src-tauri/src/commands/config.rs` with the validation rules from `contracts/commands.md` (order non-empty/unique, attempts `1..=10` for gemini/deepseek/ollama, model ≤ 200 chars, openrouter models 1–10, ollama `baseUrl` absolute http/https, at most one of model/models).
- [ ] T017 [P] [US1] Implement `cmd_save_provider_credential` in `src-tauri/src/commands/config.rs`, returning `CREDENTIAL_STORE_UNAVAILABLE` when the store is unreachable and `allowEnvFallback` is not set.
- [ ] T018 [P] [US1] Implement `cmd_remove_provider_credential` in `src-tauri/src/commands/config.rs` with the `removeEnvEntry` opt-in flag and session-override clearing.
- [ ] T019 [P] [US1] Implement `cmd_reset_provider_setting` in `src-tauri/src/commands/config.rs` for `model`, `models`, `baseUrl`, `attempts`, `enabled`, `order`, and `fallbackEnabled`, without touching credentials.
- [ ] T020 [US1] Register T016–T019 in the `tauri::generate_handler!` list in `src-tauri/src/lib.rs`.
- [ ] T021 [P] [US1] Add typed wrappers for T016–T019 in `src/lib/ipc.ts`.
- [ ] T022 [US1] Implement `src/viewmodels/useProviderConfig.ts` owning load, save, credential save/remove, reset, and error state, and instantiate it in `src/views/DashboardView.tsx`, passing props to `TopBar`.
- [ ] T023 [US1] Implement `src/components/ProviderConfigDialog.tsx` as a MUI `Dialog` with one card per provider, unconfigured providers collapsed, and the credential consent prompt for the `.env` fallback.
- [ ] T024 [US1] Add the gear button immediately to the right of `<ProviderIndicator />` in `src/components/TopBar.tsx`, wired to open the dialog.
- [ ] T025 [P] [US1] Add `src/components/ProviderConfigDialog.test.tsx` covering save, credential consent accept/cancel, remove credential with and without `removeEnvEntry`, and reset preserving the credential.
- [ ] T026 [P] [US1] Add Rust unit tests for the four US1 commands in `src-tauri/src/commands/config.rs`, including the `CREDENTIAL_STORE_UNAVAILABLE` path and validation failures.

**Checkpoint**: US1 is functional and testable on its own.

---

## Phase 4: User Story 2 - Select the model per provider (Priority: P2)

**Goal**: The operator picks one model per provider, maintains the ordered OpenRouter list, and can
refresh catalogs.

**Independent Test**: Select a model, run an evaluation, and confirm the run reports that model; for
OpenRouter, confirm a failing first model advances to the second.

### Implementation for User Story 2

- [ ] T027 [P] [US2] Implement `cmd_refresh_provider_models` in `src-tauri/src/commands/config.rs` with `includePaid`, cache write, degraded fallback to the built-in catalog, and the Ollama empty-list message.
- [ ] T028 [P] [US2] Implement `cmd_test_provider` in `src-tauri/src/commands/config.rs` reusing the model-listing probe with a 15 s cloud / 5 s Ollama timeout and sanitized messages.
- [ ] T029 [US2] Register T027–T028 in `src-tauri/src/lib.rs`.
- [ ] T030 [P] [US2] Add typed wrappers for T027–T028 in `src/lib/ipc.ts`.
- [ ] T031 [US2] Add the model selector, custom model field, "Refresh models" button, and the session-only "Show all models" toggle with price to `src/components/ProviderConfigDialog.tsx`.
- [ ] T032 [US2] Auto-fetch `/api/tags` when the dialog opens with Ollama enabled, pre-select the first model, and block enabling when the list is empty, in `src/viewmodels/useProviderConfigViewModel.ts`.
- [ ] T033 [P] [US2] Add `src/components/ProviderConfigDialog.test.tsx` cases for refresh success, degraded refresh keeping the cache, free-only filtering, "show all" with price, custom model entry, and the Ollama empty-list block.
- [ ] T034 [P] [US2] Add Rust tests for `cmd_refresh_provider_models` and `cmd_test_provider` in `src-tauri/src/commands/config.rs`, including the degraded path and the sanitized error message.

**Checkpoint**: US1 and US2 work independently.

---

## Phase 5: User Story 3 - Tune attempts and fallback (Priority: P3)

**Goal**: The operator sets per-provider attempts, toggles fallback, and reorders the chain.

**Independent Test**: Set N attempts, force a transient failure, and confirm exactly N attempts with
fallback both on and off.

### Implementation for User Story 3

- [ ] T035 [US3] Add the reorderable provider order list, the "Use fallback" switch, and the per-provider attempts fields to `src/components/ProviderConfigDialog.tsx`.
- [ ] T036 [US3] Render the OpenRouter attempts field read-only, derived from `len(models)`, and omit it from the save payload in `src/viewmodels/useProviderConfigViewModel.ts`.
- [ ] T037 [P] [US3] Add `src/components/ProviderConfigDialog.test.tsx` cases for order persistence, fallback toggle persistence, attempts validation bounds, and the read-only OpenRouter attempts value.
- [ ] T038 [P] [US3] Add a Rust test in `src-tauri/src/llm/provider.rs` proving N attempts are honored with fallback disabled and that a fatal error aborts without advancing the chain.

**Checkpoint**: US1–US3 work independently.

---

## Phase 6: User Story 4 - Quick model switch from the indicator (Priority: P4)

**Goal**: The indicator is a button that lists models grouped by provider and applies a session-only
selection.

**Independent Test**: Choose a model from the indicator, run an evaluation, restart, and confirm the
saved configuration is restored.

### Implementation for User Story 4

- [ ] T039 [US4] Implement `cmd_set_session_selection` in `src-tauri/src/commands/config.rs` and extend `cmd_get_active_provider` with `session_only`.
- [ ] T040 [US4] Register T039 in `src-tauri/src/lib.rs` and add the typed wrapper in `src/lib/ipc.ts`.
- [ ] T041 [US4] Convert `src/components/ProviderIndicator.tsx` from a `Chip` to a MUI `Button` opening a grouped `Menu`, including the `No provider configured — open settings` empty state.
- [ ] T042 [US4] Clear the session override on `cmd_save_provider_config` and on `cmd_remove_provider_credential` in `src-tauri/src/commands/config.rs`.
- [ ] T043 [P] [US4] Add `src/components/ProviderIndicator.test.tsx` cases for grouping, session selection, the empty state, and refresh after a popup save.
- [ ] T044 [P] [US4] Add Rust tests for session selection precedence and clearing in `src-tauri/src/commands/config.rs`.

**Checkpoint**: All four user stories are independently functional.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T045 [P] Update the JSON contract schemas in `src-tauri/tests/fixtures/contracts/` for the new command payloads and model-listing responses, and keep the contract tests in sync.
- [ ] T046 [P] Update `README.md` with the provider popup, credential store, Ollama setup, and the new `.env` variables.
- [ ] T047 [P] Verify `AGENTS.md` §2/§3/§8/§9 reflect the credential store, the `credentials/` module, and the persisted provider chain.
- [ ] T048 Run `./scripts/verify.sh` and fix every failure; report any skipped check explicitly.
- [ ] T049 Execute the `quickstart.md` scenarios 1–15 manually and record the results.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies.
- **Foundational (Phase 2)**: depends on Setup; blocks all user stories.
- **US1 (Phase 3)**: depends on Foundational.
- **US2 (Phase 4)**: depends on Foundational; integrates with US1's dialog.
- **US3 (Phase 5)**: depends on Foundational and US1's dialog.
- **US4 (Phase 6)**: depends on Foundational; integrates with US1's save path.
- **Polish (Phase 7)**: depends on all desired stories.

### Within Each Story

- Backend command before its IPC wrapper.
- IPC wrapper before the viewmodel/component that consumes it.
- Tests written alongside the implementation; the failure path is mandatory.

### Parallel Opportunities

- T002/T003 in Setup.
- T005/T006/T007/T008 in Foundational (different files).
- T013/T014 in Foundational.
- T016–T019 in US1 (different functions, same file — sequence them if editing concurrently).
- T021/T025/T026 in US1.
- T027/T028 in US2.
- T030/T033/T034 in US2.
- T037/T038 in US3.
- T043/T044 in US4.
- T045/T046/T047 in Polish.

---

## Parallel Example: Foundational

```bash
# Independent files, safe to run together:
Task: "Add migration contract test in src-tauri/tests/migrations.rs (T005)"
Task: "Implement db/settings.rs (T006)"
Task: "Implement credentials/mod.rs (T007)"
Task: "Extend AppConfig with Ollama fields in config/env.rs (T008)"
```

---

## Implementation Strategy

### MVP First (User Story 1 only)

1. Phase 1 (Setup).
2. Phase 2 (Foundational) — this is the largest block; do not skip.
3. Phase 3 (US1).
4. **STOP and VALIDATE**: save a credential, confirm the indicator updates without restart.
5. Ship the MVP.

### Incremental Delivery

1. Setup + Foundational → foundation ready.
2. US1 → credentials and chain configurable (MVP).
3. US2 → model selection and catalog refresh.
4. US3 → attempts, fallback, and order tuning.
5. US4 → quick session-only switch from the indicator.
6. Polish → contracts, docs, full gate, manual acceptance.

---

## Notes

- `[P]` means different files and no incomplete dependency.
- OpenRouter attempts are derived from `len(models)`; never persist `provider.openrouter.attempts`.
- Never write to `.env` without the explicit consent path.
- Never hold the `Db` mutex across an `.await`; credential-store and HTTP calls happen outside it.
- Commit one task per commit using Conventional Commits with the task ID, for example
  `feat(T016): add provider config commands`.
