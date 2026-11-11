# Data Model: LLM Provider Configuration Popup

**Feature**: `014-provider-config-popup` | **Date**: 2026-11-11

---

## Entity: Provider Setting

Operator-editable configuration for one provider. Persisted as rows in the new
`app_settings` table. **Never stores a credential value.**

**Provider identifiers (closed set)**: `gemini`, `openrouter`, `deepseek`, `ollama`.

### Storage layout (`app_settings`)

| Key | Type | Scope | Validation |
|-----|------|-------|------------|
| `provider.order` | JSON array of ids | all | non-empty, unique, every entry in the closed set |
| `provider.fallback_enabled` | `"true"` / `"false"` | all | boolean literal |
| `provider.<id>.model` | string | gemini, deepseek, ollama | non-empty, ≤ 200 chars |
| `provider.<id>.models` | JSON array of strings | openrouter | 1–10 entries, none empty |
| `provider.<id>.attempts` | integer as string | gemini, deepseek, ollama | 1 ≤ n ≤ 10 |
| `provider.<id>.enabled` | `"true"` / `"false"` | ollama | boolean literal |
| `provider.ollama.base_url` | string | ollama | absolute `http`/`https` URL |
| `provider.<id>.models_cache` | JSON array of strings | gemini, openrouter, deepseek, ollama | array of non-empty strings |
| `provider.<id>.models_cached_at` | RFC 3339 string | gemini, openrouter, deepseek, ollama | parseable timestamp |

**OpenRouter attempts are derived, not stored.** For `openrouter`, the attempt budget is exactly
`len(provider.openrouter.models)`; there is no `provider.openrouter.attempts` row. The dialog shows
the value read-only. This keeps "each attempt uses a different model" true by construction.

### Built-in defaults (used when the row is absent and `.env` is silent)

| Provider | Default model(s) | Default attempts | Enabled |
|----------|------------------|------------------|---------|
| `gemini` | `gemini-3.5-flash-lite` | 3 | yes |
| `openrouter` | `openrouter/free` | 3 | yes |
| `deepseek` | `deepseek-flash` | 2 | yes |
| `ollama` | first model returned by `/api/tags` | 2 | **no** (opt-in) |

**Ollama has no fixed default model.** When the dialog opens with Ollama enabled it queries
`/api/tags` and pre-selects the first returned model. If the list is empty (no models pulled, or
the server is stopped), enabling Ollama is blocked with the message
`no Ollama models found — run ollama pull <model>`.

Default order: `gemini → openrouter → deepseek → ollama`. Default `fallback_enabled`: `false`.

### Reset semantics

Resetting a field deletes its `app_settings` row, so the resolver falls back to `.env`/defaults.
Resetting never touches the credential (FR-022).

---

## Entity: Provider Credential

The secret that authenticates a provider.

| Attribute | Value |
|-----------|-------|
| Store | OS credential store (`keyring`) |
| Service name | `jobs-search` |
| Account | `llm:<provider>` (gemini, openrouter, deepseek) |
| Value | the API key |
| Ollama | no credential row; authentication is not used |

**Invariants**

- The value is never returned by any command after it is saved.
- The value is never written to SQLite, logs, telemetry, or exports.
- A read falls back to `.env` when the credential store has no entry.
- A write to `.env` happens only after explicit operator consent and uses `0600` permissions.
- `remove_credential(provider)` deletes the store entry **and** the matching `.env` line, because
  otherwise the fallback would silently resurrect the removed key.

---

## Entity: Provider Chain

The ordered list of providers a run may try.

| Attribute | Source |
|-----------|--------|
| `order` | `provider.order`, else `.env` `LLM_PROVIDER_ORDER`, else default order |
| `fallback_enabled` | `provider.fallback_enabled`, else `.env` `LLM_ALLOW_FALLBACK`, else `false` |
| Preferred provider | first entry of `order` whose credential resolves and that is enabled |
| Attempt budget | `provider.<id>.attempts` per provider |

**Rules**

- `ollama` is skipped unless `provider.ollama.enabled` is `"true"` (FR-019).
- A cloud provider is skipped when no credential resolves.
- With `fallback_enabled = false`, only the first eligible provider is used, but it keeps its full
  attempt budget (FR-016).
- A fatal provider error (401/403/400) aborts the chain; only retryable errors advance it.

---

## Entity: Model Catalog Cache

The most recent successfully fetched model list for a provider.

| Attribute | Value |
|-----------|-------|
| Provider | one of the closed set |
| Models | `Vec<String>` of chat-capable model identifiers |
| Fetched at | RFC 3339 timestamp |

**Resolution order for the list shown in the UI** (FR-012):

1. `provider.<id>.models_cache` when present and non-empty
2. built-in catalog compiled into the backend
3. the operator's free-text custom identifier

A failed refresh leaves the existing cache untouched and reports which list is in use.

---

## Entity: Session Selection

An in-memory, non-persisted override of the active provider and its model order.

| Attribute | Value |
|-----------|-------|
| Provider | one of the closed set |
| Models | ordered `Vec<String>`, length 1 for single-model providers |
| Lifetime | until process exit or the next successful `cmd_save_provider_config` |

**Rules**

- Clearing happens on save (FR-025), never on read.
- For OpenRouter, promoting a model from the indicator menu moves it to index 0.
- `cmd_get_active_provider` reports the session selection when present, otherwise the persisted
  configuration.

---

## Relationships

```text
Provider Chain 1 ── * Provider Setting
Provider Setting 1 ── 0..1 Provider Credential   (ollama: none)
Provider Setting 1 ── 0..1 Model Catalog Cache
Session Selection 0..1 ── 1 Provider Setting
```

---

## Migration

`SCHEMA_V7` (additive, applied inside the existing transaction in `migrations.rs`):

```sql
CREATE TABLE app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

- `SCHEMA_VERSION` becomes `7`.
- No credential column is added anywhere.
- No existing table is altered.
