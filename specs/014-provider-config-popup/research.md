# Research: LLM Provider Configuration Popup

**Feature**: `014-provider-config-popup` | **Date**: 2026-11-11

All `NEEDS CLARIFICATION` items from the Technical Context are resolved below. Each entry records
the decision, the rationale, and the alternatives that were rejected.

---

## 1. Credential storage

**Decision**: Use the `keyring` crate (v3.6) with its default `v1` feature. On Linux this selects
`zbus-secret-service-keyring-store`, which talks to the Secret Service API (GNOME Keyring, KWallet,
or any compatible daemon). Entries are addressed as service `jobs-search`, account
`llm:<provider>`.

**Rationale**: This is the industry-standard desktop pattern and the only option that keeps
secrets out of the repository, out of the app data directory, and out of SQLite. The `v1` API is
synchronous (`Entry::new`, `set_password`, `get_password`, `delete_credential`), which fits the
existing synchronous Tauri command style; calls are wrapped in `spawn_blocking` where they run
inside async commands.

**Alternatives considered**:
- `tauri-plugin-stronghold` — a self-contained encrypted vault, but it introduces a master-key
  bootstrap problem and a much larger surface for a single-user local app.
- Plaintext `.env` only — the current state; rejected because it is a CLI convention, not a
  desktop-app standard, and it lives inside the repository working tree.
- Plaintext SQLite — rejected: no industry precedent, leaks through backups and any DB read, and
  contradicts the project's documented "no credential values in any table" rule.

**Fallback**: when the Secret Service is unavailable, the backend returns a distinct error code so
the UI can ask for explicit consent before writing the credential to `.env` (FR-006). The write
uses the existing `.env` path with `0600` permissions.

**Rulebook conflict and resolution**: `AGENTS.md` §2/§9 originally said "keep secrets in `.env`".
The operator chose the OS credential store, so `AGENTS.md` §2/§9 must be updated in the same change
to: *"Provider keys live in the OS credential store by default; `.env` remains the bootstrap and
consented plaintext fallback and must stay `0600`. The app reads `.env` and writes or removes
entries only after an explicit consent prompt."* `AGENTS.md` §15 requires this update.

---

## 2. Non-secret settings storage

**Decision**: Add a `app_settings(key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT
NULL)` table via `SCHEMA_V7` in `src-tauri/src/db/migrations.rs`, with a new
`src-tauri/src/db/settings.rs` module following the existing `db/<domain>.rs` pattern.

**Rationale**: The project already persists everything in SQLite through `Db::with_connection`,
and migrations are additive and versioned. A key/value table needs no new dependency and keeps the
settings next to the data they configure.

**Alternatives considered**:
- Rewriting `.env` from the app — rejected: the app would mutate its own bootstrap file, break
  hand-edited setups, and complicate the parser.
- A `settings.json` file in the app data directory — rejected: creates a third store alongside
  `.env` and SQLite.
- `tauri-plugin-store` — rejected: a new dependency for something SQLite already does.

---

## 3. Effective configuration resolution

**Decision**: Introduce `src-tauri/src/config/provider_settings.rs` that resolves the effective
configuration in this order: **session override → SQLite `app_settings` → `.env`/OS environment →
built-in defaults**. Credentials resolve as **OS credential store → `.env`**.

**Rationale**: FR-021 requires saved settings to win over `.env`, and FR-024/FR-025 require a
session-only override that a popup save discards. A single resolver keeps that precedence in one
place instead of scattering it across commands.

**Alternatives considered**:
- `.env` wins over the database — rejected: the popup would appear to save and then not apply.
- Per-field merge with environment variables winning — rejected: same silent-failure problem.

---

## 4. Applying changes without restart

**Decision**: Change `AppState.config` from `AppConfig` to `RwLock<AppConfig>` (or
`Arc<RwLock<AppConfig>>`) and add `session_selection: Mutex<Option<SessionSelection>>`. The save
command rebuilds the resolved `AppConfig` and swaps it in; the session-selection command only
touches the mutex.

**Rationale**: FR-023 requires immediate application. The read sites are few and localized:
`commands/search.rs:177`, `commands/profile.rs:437`, `commands/config.rs:68`, plus
`commands/search.rs:158-181` for the other config fields. Each becomes a `.read()` guard.

**Alternatives considered**:
- Requiring an app restart — rejected: unacceptable for a settings screen.
- Rebuilding config on every command — rejected: spreads I/O and still needs a lock for the
  session override.

---

## 5. Ollama provider

**Decision**: Add `src-tauri/src/llm/ollama.rs` implementing `LlmProvider` against Ollama's
OpenAI-compatible endpoint (`{base_url}/v1/chat/completions`), reusing
`transport::chat_completion` with an empty/absent bearer token. Add `ollama_base_url` and
`ollama_model` to `AppConfig`, and extend `validate_provider` in `config/env.rs` to accept
`"ollama"`.

**Rationale**: `deepseek.rs` and `openrouter.rs` are ~90 lines each and already use the same
OpenAI-compatible transport, so the adapter is small and low-risk. Ollama's native `/api/chat`
would require a second response parser for no benefit.

**Alternatives considered**:
- Native `/api/chat` — rejected: duplicate parsing logic.
- Deferring Ollama — rejected by the operator; the provider is in scope.

**Opt-in**: Ollama participates in the chain only when `provider.ollama.enabled` is true (FR-019),
because a default local endpoint exists even when the server is stopped.

---

## 6. Model catalog discovery and filtering

**Decision**: Add `src-tauri/src/llm/models.rs` with one fetch function per provider and a shared
filter. Endpoints:

| Provider | Request | Auth | Filter |
|----------|---------|------|--------|
| Gemini | `GET {base}/models` | `?key=` query parameter | keep entries whose `supportedGenerationMethods` contains `generateContent`; strip the `models/` prefix |
| OpenRouter | `GET {base}/models` | none (public) | keep chat models; by default only those with `pricing.prompt == "0"` and `pricing.completion == "0"`; a session-only "show all" toggle returns every chat model with its price |
| DeepSeek | `GET {base}/models` | `Authorization: Bearer` | keep all returned `data[].id` |
| Ollama | `GET {base}/api/tags` | none | keep `models[].name` |

**Rationale**: FR-014 requires excluding non-chat models; Gemini in particular returns embeddings,
vision, and audio models that would fail at run time with an opaque error. The exact response
shapes must be pinned by `wiremock` contract tests during implementation, since provider schemas
change without notice.

**Alternatives considered**:
- No filtering — rejected: produces selectable models that always fail.
- Fetching on dialog open — rejected by the operator (explicit "Refresh models" action only).

**Caching**: the successful result is stored in `app_settings` as
`provider.<name>.models_cache` plus `provider.<name>.models_cached_at`. Resolution order is cache →
built-in catalog → custom identifier (FR-012). The built-in catalog lives in the Rust backend so
there is a single source of truth; the frontend only renders what the command returns.

---

## 7. Attempts decoupled from fallback

**Decision**: Change `LlmRouter::without_fallback()` in `src-tauri/src/llm/provider.rs` so it only
truncates the plan list to one provider and no longer forces `plan.attempts = 1`. Attempts come
from the resolved settings per provider.

**Rationale**: FR-016 requires the configured attempt count to be honored with fallback on or off.
The current implementation silently discards the configured value.

**Alternatives considered**:
- Keeping the coupling — rejected: the popup field would be decorative when fallback is off.

**Defaults**: preserve today's budgets — Gemini 3, OpenRouter 3, DeepSeek 2, Ollama 2 — as the
built-in defaults when no setting is stored.

**OpenRouter exception**: the attempt budget for OpenRouter is not a stored setting. It is exactly
`len(provider.openrouter.models)`, so "each attempt uses a different model" holds by construction.
The dialog shows the value read-only.

---

## 8. Session-only model override

**Decision**: Store the override in `AppState.session_selection: Mutex<Option<SessionSelection>>`.
`cmd_get_active_provider` reports the override when present. `cmd_save_provider_config` clears the
override before applying the saved value.

**Rationale**: FR-024/FR-025. The override must not be persisted, and a popup save must win.

**Alternatives considered**:
- Persisting the indicator selection — rejected by the operator (explicit session-only choice).
- Frontend-only state — rejected: the backend router would not see the override.

**Known risk (accepted)**: a session-only selection silently reverts on restart. This is recorded
in the spec as an accepted trade-off, not a defect.

---

## 9. Frontend structure

**Decision**: `ProviderConfigDialog.tsx` (presentational, MUI `Dialog` with one card per provider,
unconfigured providers collapsed) plus `useProviderConfigViewModel.ts` and
`useProviderConfig.ts` for state. `ProviderIndicator.tsx` becomes a MUI `Button` wrapping the
existing chip content, opening a grouped `Menu`.

**Rationale**: Matches the project rule that components stay presentational and stateful logic
lives in hooks/viewmodels, and reuses the `AuditDialog.tsx` dialog pattern.

**Alternatives considered**:
- Tabs or a two-screen flow — rejected: unnecessary navigation state for four providers.
- Putting fetch logic in the component — rejected by the project rules.

---

## 10. Testing strategy

**Decision**:
- Rust unit tests for the settings repository, the resolver precedence, the credential wrapper
  (with a fake store), and the attempts/fallback decoupling.
- `wiremock` contract tests for each model-listing endpoint and for the Ollama chat completion.
- Vitest + Testing Library for the dialog (save, reset, remove credential, refresh fallback) and
  the indicator menu (grouping, session override, popup-save clearing).
- `tsc --noEmit`, `oxlint`, `prettier --check`, `cargo clippy --all-targets -- -D warnings`.

**Rationale**: The project already uses `wiremock` for provider contracts and Vitest for
components; no new test tooling is introduced.

**Alternatives considered**:
- Live provider calls in tests — rejected: non-deterministic and would require real keys.
