# Contract: Tauri commands

**Feature**: `014-provider-config-popup` | **Date**: 2026-11-11

All commands follow the existing convention: typed wrapper in `src/lib/ipc.ts`, `Result<T,
ErrorPayload>` on the Rust side, errors serialized as `{ code, message }`.

`ProviderId` is the closed set `"gemini" | "openrouter" | "deepseek" | "ollama"`.

---

## `cmd_get_provider_config`

Returns the full configuration view. No credential value is ever included.

**Request**: no arguments

**Response** `ProviderConfigView`:

```ts
{
  providers: Array<{
    id: ProviderId;
    label: string;
    requiresApiKey: boolean;      // false for ollama
    hasCredential: boolean;       // never the value
    credentialSource: 'credential_store' | 'env' | 'none';
    model: string | null;         // single-model providers
    models: string[] | null;      // openrouter ordered list
    baseUrl: string | null;       // ollama only
    attempts: number;
    enabled: boolean;             // meaningful for ollama
  }>;
  order: ProviderId[];
  fallbackEnabled: boolean;
  sessionOverride: { provider: ProviderId; models: string[] } | null;
}
```

**Errors**: `DB_ERROR`, `CONFIG_ERROR`

---

## `cmd_save_provider_config`

Persists non-secret settings and applies them immediately. Clears any session override on success
(FR-025). Credentials are **not** written by this command.

**Request**:

```ts
{
  order: ProviderId[];
  fallbackEnabled: boolean;
  providers: Array<{
    id: ProviderId;
    model?: string;
    models?: string[];
    baseUrl?: string;
    attempts?: number;
    enabled?: boolean;
  }>;
}
```

**Response**: `ProviderConfigView` (same shape as `cmd_get_provider_config`)

**Validation** (rejects with `VALIDATION_ERROR` and a specific message):

- `order` non-empty, unique, every entry a `ProviderId`
- `attempts` in `1..=10` for gemini, deepseek and ollama; **not accepted for openrouter** (derived
  from `len(models)`)
- `model` non-empty, ≤ 200 chars
- openrouter `models` has 1–10 non-empty entries
- `baseUrl` is an absolute `http`/`https` URL (ollama only)
- at most one of `model` / `models` per provider

---

## `cmd_save_provider_credential`

Stores a credential in the OS credential store.

**Request**:

```ts
{ provider: ProviderId; apiKey: string; allowEnvFallback?: boolean }
```

**Response**: `ProviderConfigView`

**Errors**:

| Code | Meaning | UI behaviour |
|------|---------|--------------|
| `VALIDATION_ERROR` | empty or malformed key | inline field error |
| `CREDENTIAL_STORE_UNAVAILABLE` | OS store not reachable and `allowEnvFallback` was not set | show consent dialog, then retry with `allowEnvFallback: true` |
| `IO_ERROR` | `.env` write failed | inline error, no partial write |

The credential value is never echoed back in any response or error message.

---

## `cmd_remove_provider_credential`

Deletes the credential from the OS store. Removing the matching `.env` line is **opt-in**: the
operator must confirm it explicitly, because the app never edits `.env` without consent.

**Request**: `{ provider: ProviderId; removeEnvEntry?: boolean }`

**Response**: `ProviderConfigView`

**Rules**:

- `removeEnvEntry` defaults to `false`; when `false`, a `.env` entry keeps the provider configured
  and the response reports `credentialSource: 'env'`.
- When `true`, the matching `.env` line is removed and the file keeps its `0600` permissions.
- Removing the credential clears any session override that points at that provider.
- Never touches other settings.

**Errors**: `NOT_FOUND` (nothing configured), `IO_ERROR`

---

## `cmd_reset_provider_setting`

Deletes one stored setting so it resolves to `.env`/defaults. Does not touch credentials.

**Request**: `{ provider: ProviderId; field: 'model' | 'models' | 'baseUrl' | 'attempts' | 'enabled' | 'order' | 'fallbackEnabled' }`

**Response**: `ProviderConfigView`

---

## `cmd_test_provider`

Performs one minimal real request against the provider to validate the connection (FR-007). Uses
a draft credential when supplied, so the operator can test before saving.

**Request**:

```ts
{ provider: ProviderId; draftApiKey?: string; draftBaseUrl?: string; model?: string }
```

**Response**:

```ts
{ ok: boolean; latencyMs: number; message: string }  // message is sanitized
```

**Rules**:

- Never includes the key, the full URL, or the provider response body in `message`.
- Ollama is tested by `GET {baseUrl}/api/tags`.
- Cloud providers are tested by the same endpoint used for model listing.

---

## `cmd_refresh_provider_models`

Fetches the provider catalog and stores it as the cache (FR-011). Covers the three cloud providers
and the local Ollama server.

**Request**: `{ provider: ProviderId; includePaid?: boolean }`

**Response**:

```ts
{
  provider: ProviderId;
  models: Array<{ id: string; price: string | null }>;
  source: 'cache' | 'builtin';
  fetchedAt: string | null;
  degraded: boolean;      // true when the refresh failed and a fallback list is returned
  message: string | null; // sanitized reason for the degradation
}
```

**Rules**:

- A failed fetch never clears the existing cache.
- Non-chat models are filtered out before the response is built (FR-014).
- For OpenRouter, `includePaid` defaults to `false`: only models whose `pricing.prompt` and
  `pricing.completion` are `"0"` are returned. When `true`, all chat models are returned and each
  entry carries a human-readable `price`. The toggle is session-only and is not persisted.
- For Ollama, the request is `GET {baseUrl}/api/tags` with a 5 s timeout; an empty result is
  reported as `degraded` with a message telling the operator to pull a model.
- When both cache and network are unavailable, the built-in catalog is returned with
  `source: 'builtin'`.

---

## `cmd_set_session_selection`

Applies an in-memory selection for the current session (FR-024).

**Request**: `{ provider: ProviderId; models: string[] }`

**Response**: `ProviderConfigView`

**Rules**:

- Not persisted.
- `models` must be non-empty; for a single-model provider exactly one entry is allowed.
- Replaces any previous session selection.

---

## `cmd_get_active_provider` (existing, extended)

Already used by `ProviderIndicator`. Response gains no secret data; the reported provider/model
reflects the session selection when one exists.

**Response** `ActiveProvider`:

```ts
{ provider: string; model: string; fallback_enabled: boolean; session_only?: boolean }
```

`session_only: true` marks a session override so the UI can indicate it.

---

## UI contract: provider indicator menu

- The indicator is always a button, including the "not configured" state.
- When at least one provider is configured, the menu lists models grouped by provider, in the
  configured provider order.
- When no provider is configured, the menu contains a single item:
  `No provider configured — open settings`, which opens the configuration dialog.
- Choosing a model calls `cmd_set_session_selection`; the selection is not persisted.
- The menu never displays a credential value.
