# Quickstart: LLM Provider Configuration Popup

**Feature**: `014-provider-config-popup` | **Date**: 2026-11-11

Validation guide for proving the feature works end to end. Implementation details live in
`tasks.md`; entity shapes and command signatures live in
[data-model.md](./data-model.md) and [contracts/](./contracts/).

---

## Prerequisites

- Node + npm (see `package.json`), Rust 1.90 toolchain.
- A local `.env` copied from `.env.example` (never commit real keys).
- For the Ollama scenarios: a running Ollama server (`ollama serve`) with at least one pulled
  model (`ollama pull llama3.2`).
- A Linux desktop session with a Secret Service daemon (GNOME Keyring or KWallet) for the
  credential-store paths. Use the fallback-consent scenario to exercise the unavailable path.

---

## Automated checks

The real gate is the project script:

```bash
./scripts/verify.sh
```

It runs Prettier, oxlint, TypeScript, Vitest, `cargo fmt`, `cargo check`, `cargo clippy`,
`cargo test`, and both sidecar suites, and reports skipped checks explicitly. A skipped check is
not a passing check.

Focused suites while iterating:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test llm_provider_contract
cargo test --manifest-path src-tauri/Cargo.toml settings
npm test -- src/components/ProviderConfigDialog.test.tsx src/components/ProviderIndicator.test.tsx
```

---

## Scenario 1 — Save a credential and see it apply without restart

1. Launch `npm run tauri dev`.
2. Click the gear button to the right of the provider indicator.
3. Enter a valid key for Gemini and save.
4. Close the dialog.

**Expected**: the indicator shows `gemini · <model>`. No restart. Reopening the dialog shows the
provider as configured and the key field empty (value never re-displayed).

---

## Scenario 2 — Select a model per provider

1. Open the dialog and expand Gemini.
2. Pick a different model from the list and save.
3. Start a run.

**Expected**: the run's provider/model in the audit report matches the selected model.

---

## Scenario 3 — OpenRouter ordered models

1. Configure OpenRouter and add two models to its ordered list.
2. Make the first model invalid (typo the identifier) and run an evaluation.

**Expected**: attempt 1 fails against the bad model, attempt 2 succeeds with the second model, and
the run does not leave OpenRouter.

---

## Scenario 4 — Test connection

1. Enter a deliberately invalid key and press **Test connection**.

**Expected**: a sanitized failure message appears; the key value never appears in the message, in
the console, or in `src-tauri` logs.

---

## Scenario 5 — Refresh models with and without network

1. Press **Refresh models** for a configured provider with network available.

**Expected**: the list updates and is cached; reopening the dialog is instant.

2. Disable networking and press **Refresh models** again.

**Expected**: the dialog keeps showing the cached list and states that the refresh failed. No empty
list, no error dialog that blocks the screen.

3. Clear the cache row and disable networking, then reopen the dialog.

**Expected**: the built-in catalog is shown and labelled accordingly.

---

## Scenario 6 — Attempts honored with fallback disabled

1. Set Gemini attempts to `4`, configure a bad Gemini key, and turn fallback off.
2. Run an evaluation and inspect the audit report.

**Expected**: exactly 1 attempt, because an invalid key is a *fatal* error (401) and aborts the
chain immediately. Then repeat with a transient failure (for example, point the base URL at a
wiremock that returns 500).

**Expected on the transient case**: exactly 4 attempts, then failure, and no other provider tried.

---

## Scenario 7 — Ollama opt-in

1. Ensure the Ollama toggle is **off** and run an evaluation with only Gemini configured.

**Expected**: Ollama is never contacted (check logs).

2. Turn the toggle on and run again.

**Expected**: Ollama appears in the chain; if the server is stopped, the run reports a connection
failure for Ollama without corrupting the other providers' results.

---

## Scenario 8 — Session-only override from the indicator

1. Click the provider indicator and choose a different model.
2. Run an evaluation.

**Expected**: the run uses the chosen model and the indicator reflects it.

3. Restart the application.

**Expected**: the previously saved configuration is restored — the session override is gone. This
is the accepted trade-off recorded in the spec.

4. With a session override active, save configuration in the dialog.

**Expected**: the override is discarded and the saved value applies.

---

## Scenario 9 — Credential store unavailable

1. Run the app in an environment without a Secret Service daemon.
2. Save a credential.

**Expected**: a consent dialog explains that the credential will be written to `.env` in plaintext
with owner-only permissions. Cancelling writes nothing. Accepting writes it and the dialog
reports `.env` as the credential source.

---

## Scenario 10 — Secret hygiene

After saving credentials:

```bash
grep -ri "your-api-key-fragment" src-tauri/ src/ specs/ 2>/dev/null   # expect no matches
sqlite3 <app_data_dir>/jobs-search.db "select * from app_settings;"   # expect no credential value
ls -l .env                                                            # expect -rw-------
```

**Expected**: no credential value in source, specs, SQLite, logs, or exported reports; `.env`
remains `0600`.

---

## Scenario 11 — Reset without losing the credential

1. Save a key, then reset the model and attempts fields independently.

**Expected**: the model and attempts revert to defaults while the provider still reports
`hasCredential: true`. Only **Remove credential** clears the key.

---

## Scenario 12 — OpenRouter free filter and "show all"

1. Configure OpenRouter and open its card.

**Expected**: only free chat models are listed.

2. Enable **Show all models**.

**Expected**: every chat model is listed with its price. Close and reopen the dialog.

**Expected**: the toggle is back to off (session-only).

---

## Scenario 13 — Empty indicator state

1. Remove every credential and restart the app.
2. Click the provider indicator.

**Expected**: the menu shows a single entry, `No provider configured — open settings`, which opens
the configuration dialog.

---

## Scenario 14 — Remove credential consent

1. Configure a key through the `.env` fallback path.
2. Press **Remove credential** without checking the `.env` option.

**Expected**: the provider still reports `credentialSource: 'env'` and remains usable.

3. Repeat with the `.env` option checked.

**Expected**: the `.env` line is removed, the file keeps `0600`, and the provider reports
`credentialSource: 'none'`. Any session override for that provider is cleared.

---

## Scenario 15 — Ollama model default

1. With Ollama running and at least one model pulled, enable Ollama.

**Expected**: the first model from `/api/tags` is pre-selected.

2. Stop the Ollama server and try to enable it.

**Expected**: enabling is blocked with `no Ollama models found — run ollama pull <model>`.

---

## Rollback

No destructive migration is involved: `SCHEMA_V7` only adds a table. Reverting the feature leaves
the table unused. Credentials stored in the OS credential store remain until removed manually via
the platform keyring tool or **Remove credential**.
