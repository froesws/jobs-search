# Contract: Outbound provider HTTP

**Feature**: `014-provider-config-popup` | **Date**: 2026-11-11

These are the requests the Rust backend makes to providers for **model listing** and **connection
tests**. Chat-completion requests are unchanged and already covered by
`src-tauri/tests/llm_provider_contract.rs`.

All requests reuse the shared `reqwest::Client` in `AppState.http` and the existing
`classify_status` mapping (`llm/transport.rs`). A new `get_json` helper mirrors `post_json`:
missing URL in error messages, sanitized bodies, `LlmError` on non-2xx.

---

## Model listing

| Provider | Method & path | Auth | Chat-model filter |
|----------|---------------|------|-------------------|
| Gemini | `GET {gemini_base_url}/models` | `?key={api_key}` query parameter | keep `models[].name` when `supportedGenerationMethods` contains `"generateContent"`; strip the leading `models/` |
| OpenRouter | `GET {openrouter_base_url}/models` | none | keep `data[].id` for chat-capable entries; exclude non-chat modalities when the payload exposes them |
| DeepSeek | `GET {deepseek_base_url}/models` | `Authorization: Bearer {api_key}` | keep `data[].id` |
| Ollama | `GET {ollama_base_url}/api/tags` | none | keep `models[].name` |

**Response shape used by the backend** (internal, not exposed):

```text
ModelList { models: Vec<String> }
```

**Rules**

- Results are sorted and de-duplicated before being returned.
- An empty filtered list is treated as a failure, not as a valid catalog, so a provider that
  changes its schema cannot silently hide every model.
- HTTP errors map through `classify_status`: 401/403 → auth (fatal), 429 → rate limit (retryable),
  5xx → network (retryable), other → config.

---

## Connection test

| Provider | Probe |
|----------|-------|
| Gemini | `GET {base}/models?key=…` with a 15 s timeout |
| OpenRouter | `GET {base}/models` with a 15 s timeout |
| DeepSeek | `GET {base}/models` with `Bearer` and a 15 s timeout |
| Ollama | `GET {base}/api/tags` with a 5 s timeout |

The probe reuses the model-listing request, so a successful "Test connection" also validates that
the credential can reach the provider and that the response shape is understood.

**Sanitization**

- `message` never contains the key, the query string, or the response body.
- Success: `"connection ok"` plus latency.
- Failure: the `LlmError::message` value (already sanitized by the transport layer).

---

## Ollama chat completion (new provider)

| Field | Value |
|-------|-------|
| Method & path | `POST {ollama_base_url}/v1/chat/completions` |
| Auth | none |
| Body | same OpenAI-compatible JSON already built by `transport::chat_completion` |
| Response parsing | `transport::chat_content` (existing) |

The adapter reuses `transport::chat_completion` with an empty credential, so no new request/
response parser is introduced.
