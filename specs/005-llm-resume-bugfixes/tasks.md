---

description: "Task list for LLM provider 404 and resume persistence bugfixes"
---

# Tasks: LLM Provider 404 and Resume Persistence Bugfixes

**Input**: Bug reports `BUG-001` and `BUG-002`. No `spec.md` or `plan.md`
exists for this feature yet, so the tasks are derived from the reports, the
current Rust source, and the provider documentation checked on 2026-11-11.

**Prerequisites**: A local `.env` with provider keys (never print, log, or
commit values), a text-based PDF resume for manual testing, and the existing
Rust/frontend test suites.

**Tests**: Required. Both bugs are regressions, so each fix must include tests
that fail before the implementation and pass after it.

**Organization**: Tasks are grouped by bug so each fix can be implemented and
validated independently. `US1` is `BUG-001`; `US2` is `BUG-002`.

## Root Cause Notes (verified 2026-11-11)

- **Gemini**: `AppConfig::from_lookup` defaults `GEMINI_MODEL` to
  `gemini-2.5-flash`. Google lists that model as deprecated and unavailable to
  new API keys; the documented replacement for `gemini-2.5-flash` is
  `gemini-3.5-flash-lite` (effective 2026-12-01). `gemini-3.8-flash` is a
  higher-quality override, but it is the replacement for `gemini-2.5-pro`. The
  base URL and `/v1beta/models/{model}:generateContent` route are otherwise
  correct. Gemini 3.x deprecates `temperature` and other sampling parameters,
  so stop sending them.
- **OpenRouter**: the default `OPENROUTER_FREE_MODELS` values
  (`deepseek/deepseek-chat-v3-0324:free`, `meta-llama/llama-3.3-70b-instruct:free`,
  `qwen/qwen-2.5-72b-instruct:free`) are retired. Retired `:free` IDs return
  HTTP 404 "No endpoints found". `openrouter/free` is the stable free router;
  it selects a random free model that supports the requested features.
- **DeepSeek**: the legacy `deepseek-chat` model name was discontinued on
  2026-07-24. The current direct-API model is `deepseek-flash`, and the
  documented base URL is `https://api.deepseek.com` (the adapter appends
  `/chat/completions`). `deepseek-flash` enables thinking by default; this
  bugfix does not change that, but operators should expect extra latency and
  billed output tokens.
- **Fallback behavior**: `classify_status` currently maps 404 to a
  non-retryable `CONFIG` error, so the router stops instead of trying the next
  provider/model. A 404 caused by a retired model should be fallback-eligible;
  401/403 stay fatal and 429/5xx stay retryable.
- **Provider selection**: `AppConfig` reads only `LLM_PROVIDER_ORDER`; a
  `LLM_PROVIDER` value in `.env` is currently ignored. The local `.env` has
  keys for the supported providers, so the default order selects Gemini first.
- **Resume upload**: `upload_resume_with` calls `extractor.extract` before
  writing the PDF and inserting the `resumes` row, so any provider error
  prevents all persistence. The row and file must be persisted first, and
  extraction can be backfilled. A failed first upload can leave a resume with
  `extracted_json = NULL` and no active profile, so duplicate and delete paths
  must not assume a profile exists.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete
  task).
- **[Story]**: `[US1]` for `BUG-001`, `[US2]` for `BUG-002`.
- Every task includes an exact file path.

## Path Conventions

- Rust backend: `src-tauri/src/` and `src-tauri/tests/`
- Frontend: `src/`
- Feature docs: `specs/005-llm-resume-bugfixes/`
- Local secrets: `.env` (ignored by Git; never read values into tasks, logs,
  or commits)

---

## Phase 1: Setup (Baseline)

**Purpose**: Establish a clean baseline before changing provider or upload code.

- [X] T001 Run `./scripts/verify.sh` from the repo root and record any pre-existing failures in the first commit message; do not fix unrelated failures in this feature (path: `scripts/verify.sh`).

**Checkpoint**: Baseline is known and no unrelated failures are mixed into the
bugfix work.

---

## Phase 2: Foundational (Blocking Prerequisites)

No shared implementation prerequisites are required. Both user stories are
independent after T001; do not add a database migration or a new dependency for
either fix.

**Checkpoint**: T001 is complete; US1 and US2 can start in parallel.

---

## Phase 3: User Story 1 - LLM provider returns 404 (Priority: P1) 🎯 MVP

**Goal**: A configured provider can evaluate jobs and extract profiles without
404 errors from stale model IDs, and a provider-level 404 can fall back to the
next configured provider when fallback is enabled.

**Independent Test**: With `LLM_PROVIDER=gemini` and a valid key, call
`cmd_get_active_provider` and confirm the model is `gemini-3.5-flash-lite`;
repeat for `openrouter` (`openrouter/free`) and `deepseek` (`deepseek-flash`).
Then call an evaluation or resume extraction and confirm no HTTP 404. Do not
print key values.

### Tests for User Story 1 ⚠️

> Write these tests first and confirm they fail for the current defaults.

- [X] T002 [P] [US1] Add a unit test for `classify_status` asserting that HTTP 404 is retryable, remains a configuration error, and includes a message that tells the operator to check the model and endpoint (path: `src-tauri/src/llm/transport.rs`).
- [X] T003 [P] [US1] Extend the wiremock contract tests to assert the exact request path and model for each adapter (`/models/{model}:generateContent` for Gemini, `/chat/completions` with the configured model for OpenRouter and DeepSeek), assert that the Gemini request body omits the deprecated `temperature` field, and add a test where a 404 from the first provider falls back to the second provider when fallback is enabled (path: `src-tauri/tests/llm_provider_contract.rs`).
- [X] T004 [P] [US1] Add configuration tests asserting the new defaults `GEMINI_MODEL=gemini-3.5-flash-lite`, `OPENROUTER_FREE_MODELS=openrouter/free`, and `DEEPSEEK_MODEL=deepseek-flash`; add tests for `LLM_PROVIDER` precedence over `LLM_PROVIDER_ORDER` and rejection of an unknown `LLM_PROVIDER` (path: `src-tauri/src/config/env.rs`).

### Implementation for User Story 1

- [X] T005 [US1] Update provider defaults in `AppConfig`: `gemini_model` to `gemini-3.5-flash-lite`, `openrouter_free_models` to `openrouter/free`, and `deepseek_base_url` to `https://api.deepseek.com`; keep existing environment overrides and empty-value fallbacks (path: `src-tauri/src/config/env.rs`).
- [X] T006 [US1] Implement the optional `LLM_PROVIDER` environment variable in `AppConfig::from_lookup`: validate `gemini|openrouter|deepseek`, store it in `AppConfig::llm_provider`, and move it to the front of the parsed `LLM_PROVIDER_ORDER` (deduplicated, remaining providers kept as fallbacks); return a clear `AppError::Config` when it is unknown (path: `src-tauri/src/config/env.rs`).
- [X] T007 [US1] Add a configurable `deepseek_model` field (default `deepseek-flash`, env `DEEPSEEK_MODEL`) and use it in the router and active provider command instead of the hard-coded `deepseek-chat` string (paths: `src-tauri/src/config/env.rs`, `src-tauri/src/llm/mod.rs`, `src-tauri/src/commands/config.rs`).
- [X] T008 [P] [US1] Change `classify_status` so HTTP 404 produces a retryable configuration error with a clear "endpoint or model not found" message; keep 401/403 fatal and 429/5xx retryable (path: `src-tauri/src/llm/transport.rs`).
- [X] T009 [P] [US1] Harden Gemini URL construction in `GeminiProvider`: add a helper that trims a trailing slash, strips a leading `models/` from the model name, avoids duplicating `/models/`, and appends `/v1beta` when `GEMINI_BASE_URL` has no path; use the helper for both `evaluate` and `extract_profile`, remove the deprecated `temperature` field from the request body, and add unit tests for bare-host, versioned, and already-prefixed inputs (path: `src-tauri/src/llm/gemini.rs`).
- [X] T010 [US1] Improve `active_provider` diagnostics: report the selected provider, model, and the missing environment variable name (for example `GEMINI_API_KEY`) without ever exposing a key value; when `LLM_PROVIDER` is set but its key is missing, return a configuration error naming that variable; add tests for `LLM_PROVIDER` selection and missing keys (path: `src-tauri/src/commands/config.rs`).
- [X] T011 [P] [US1] Document the current provider configuration in `README.md` and create a secret-free `.env.example` that mirrors the README block with placeholders, including `LLM_PROVIDER`, `LLM_PROVIDER_ORDER`, `GEMINI_API_KEY`, `GEMINI_MODEL`, `OPENROUTER_API_KEY`, `OPENROUTER_FREE_MODELS`, `DEEPSEEK_API_KEY`, `DEEPSEEK_MODEL`, `DEEPSEEK_BASE_URL`, `LLM_ALLOW_FALLBACK`, `LLM_TIMEOUT_SECONDS`, and `MATCH_THRESHOLD` (paths: `README.md`, `.env.example`).
- [X] T012 [US1] Run the focused Rust suites (`cargo test --manifest-path src-tauri/Cargo.toml --test llm_provider_contract` and the `config::env`/`commands::config` unit tests), then launch `npm run tauri dev` and confirm the provider indicator shows a current model; inspect `.env` variable names only and never print values (paths: `src/components/ProviderIndicator.tsx`, `src-tauri/tests/llm_provider_contract.rs`).

**Checkpoint**: All three adapters use current model IDs; a 404 no longer
blocks fallback; the active provider reports a usable model.

---

## Phase 4: User Story 2 - Resume is lost when LLM extraction fails (Priority: P2)

**Goal**: Uploading a PDF always persists the PDF file and the `resumes` row
before the LLM is called. If extraction fails, the command returns a partial
success with a warning, and the resume remains listed and deletable even when
no active profile exists.

**Independent Test**: Upload a PDF with an intentionally invalid provider key.
Confirm the resume appears in `cmd_list_resumes`, its file exists under
`{app_data}/resumes/`, its `extracted_json` is `NULL`, and the frontend shows a
warning instead of an error. Delete the failed resume and confirm it is removed
without an error. Fix the key, upload a different PDF, and confirm the
successful path still updates the profile.

### Tests for User Story 2 ⚠️

> Write these tests first and confirm they fail against the current ordering.
> T013-T016 all edit the same test file, so run them sequentially.

- [X] T013 [US2] Replace `failed_extraction_does_not_create_a_partial_profile` with a regression test asserting that a failed extractor returns `warning.is_some()`, `duplicate == false`, `profile.is_none()`, one row in `resumes`, a file on disk, `extracted_json` equal to `None`, and no saved active profile (path: `src-tauri/tests/profile_pipeline.rs`).
- [X] T014 [US2] Extend the successful upload test to assert the stored resume has non-null `extracted_json`, the profile contains the extracted competency, and the file and row both exist; add a unit test for the new `update_resume_extracted` helper, including the not-found path (paths: `src-tauri/tests/profile_pipeline.rs`, `src-tauri/src/db/profile.rs`).
- [X] T015 [US2] Keep the duplicate-upload test and add an assertion that the second upload returns `duplicate == true`, does not insert a second row, and does not invoke the extractor again; add a duplicate-after-failure case where the stored resume has `extracted_json = NULL`, the result is `duplicate == true` with a warning, `profile.is_none()`, no error, and no extractor call (path: `src-tauri/tests/profile_pipeline.rs`).
- [X] T016 [US2] Add a delete regression test: with only a failed-extraction resume and no active profile, `delete_resume_with` returns `Ok(None)`, removes the row and file, and does not error; keep the existing last-successful-resume test asserting `Some(profile)`, `resumes_available == false`, and blocked searches (path: `src-tauri/tests/profile_pipeline.rs`).

### Implementation for User Story 2

- [X] T017 [US2] Add `update_resume_extracted(db, resume_id, extracted_json: Option<&str>)` to `src-tauri/src/db/profile.rs` and make the T014 unit test pass; return `AppError::not_found("resume")` when the row does not exist (path: `src-tauri/src/db/profile.rs`).
- [X] T018 [US2] Refactor `upload_resume_with` in `src-tauri/src/commands/profile.rs` to this order: validate/read/hash → duplicate lookup → extract PDF text → write the PDF to `{app_data}/resumes/{hash}.pdf` → insert the `resumes` row with `extracted_json = None` → call `ProfileExtractor::extract`. On LLM failure, keep the row and file, load the active profile if one exists, and return `warning = Some(...)` instead of `Err`. On success, backfill `extracted_json`, merge, and save the profile. After the insert succeeds, never delete the row/file; return a warning for extraction, backfill, or profile-save failures. Remove the file only if the insert itself fails (path: `src-tauri/src/commands/profile.rs`).
- [X] T019 [US2] Extend `UploadOutcome` with `resume_id: String`, `profile: Option<CandidateProfile>`, and `warning: Option<String>` and add a serializable `UploadResumeResult` for the Tauri boundary; map `cmd_upload_resume` to that result while keeping hard failures (invalid extension, unreadable PDF, DB errors before insert) as `ErrorPayload` (path: `src-tauri/src/commands/profile.rs`).
- [X] T020 [US2] Change `delete_resume_with` to return `Result<Option<CandidateProfile>, AppError>` and `cmd_delete_resume` to `Result<Option<CandidateProfile>, ErrorPayload>`: after deleting the row and file, load the active profile and the remaining resumes; if there is no active profile and no remaining resume has `extracted_json`, return `Ok(None)`; otherwise rebuild from the active profile (with extracted fields reset) or from `None`, merge every remaining `extracted_json`, set `resumes_available`, save, and return `Some(profile)` (path: `src-tauri/src/commands/profile.rs`).
- [X] T021 [US2] Add the `UploadResumeResult` interface to `src/lib/types.ts` with `profile: CandidateProfile | null`, `resume_id: string`, `duplicate: boolean`, and `warning: string | null`; change `uploadResume` to return `Promise<UploadResumeResult>` and `deleteResume` to return `Promise<CandidateProfile | null>` (paths: `src/lib/types.ts`, `src/lib/ipc.ts`).
- [X] T022 [US2] Update `src/components/ResumeManager.test.tsx` for the new `UploadResumeResult` shape: warning result shows a warning notification, success result shows the processed message, duplicate result does not claim the profile was recalculated, a `null` profile is handled, and deleting a resume with a `null` profile shows a warning instead of an error (path: `src/components/ResumeManager.test.tsx`).
- [X] T023 [US2] Update `useResumeViewModel.upload` to return `UploadResumeResult`, `useResumeViewModel.remove` to return `CandidateProfile | null`, and `ResumeManager` to show `result.warning` as a warning notification, a duplicate-specific message when `result.duplicate` is true, the existing success message otherwise, and a warning when a delete returns `null` (paths: `src/viewmodels/useResumeViewModel.ts`, `src/components/ResumeManager.tsx`).

**Checkpoint**: A failed LLM extraction leaves a usable, visible resume and a
warning; a successful extraction still updates the profile exactly once; a
failed resume can be deleted without an error.

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Logging, documentation, and end-to-end validation across both
stories.

- [X] T024 [P] Emit a sanitized `tracing::warn!` when extraction fails after the resume is saved: log `RouterError.category` and the sanitized message in `LlmProfileExtractor::extract` before mapping to `AppError`, and log that the resume was kept in `upload_resume_with`; never log the resume text, file path, API key, prompt, or full provider response (path: `src-tauri/src/commands/profile.rs`).
- [X] T025 [P] Document the partial-success behavior in `README.md`: a saved resume with a failed extraction remains listed, has no extracted profile data, can be deleted without an error, and can be replaced; document that the LLM must be fixed before extraction succeeds (path: `README.md`).
- [X] T026 Run the full local gate: `cargo fmt --manifest-path src-tauri/Cargo.toml --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npm test`, `npm run typecheck`, and `./scripts/verify.sh` (path: `scripts/verify.sh`).
- [X] T027 Perform manual acceptance: with an invalid key, upload a PDF and confirm the resume is listed and the warning is shown; delete it and confirm no error; with a valid key, upload a second PDF and confirm the profile updates; inspect local logs and confirm no secrets, resume text, or filesystem paths were written (paths: `src/components/ResumeManager.tsx`, `src-tauri/src/commands/profile.rs`).
- [X] T028 [P] Confirm `.env.example` contains only placeholders and that `.env` remains ignored by Git; verify no provider key or raw provider response appears in `README.md`, `specs/`, or test fixtures (paths: `.env.example`, `.gitignore`).

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies; T001 must pass before code changes.
- **Foundational (Phase 2)**: No tasks; both stories start after T001.
- **US1 (Phase 3)**: Independent of US2. Tests T002-T004 come first; T005-T011
  implement the fix; T012 validates the story.
- **US2 (Phase 4)**: Independent of US1. Tests T013-T016 come first; T017-T023
  implement the fix.
- **Polish (Phase 5)**: Depends on US1 and US2 completion.

### User Story Dependencies

- **US1 (P1)**: Can start immediately after T001. No dependency on US2.
- **US2 (P2)**: Can start immediately after T001. No dependency on US1,
  because it uses `ProfileExtractor` rather than a live provider in tests.

### Within Each User Story

- Tests must be written and fail before implementation.
- US1: T005 → T006 → T007 (same config file); T008, T009, and T011 are
  independent; T010 depends on T006 and T007; T012 is last.
- US2: T013-T016 touch the same test file and must run sequentially;
  T017 → T018 → T019 → T020 (T018-T020 share `commands/profile.rs`);
  T021 depends on T019 and T020; T022 depends on T021; T023 depends on T022.

### Parallel Opportunities

- T002, T003, and T004 can run in parallel (different test surfaces).
- T008, T009, and T011 can run in parallel with T005-T007 (different files).
- US1 and US2 can run in parallel after T001.
- T024, T025, and T028 can run in parallel during Polish.
- US2 is mostly sequential: T013-T016 share one test file, T018-T020 share
  `commands/profile.rs`, and the frontend tests/UI depend on the new contract.
  Do not mark those tasks `[P]`.

---

## Parallel Example: User Story 1

```bash
# Tests first:
cargo test --manifest-path src-tauri/Cargo.toml --test llm_provider_contract
cargo test --manifest-path src-tauri/Cargo.toml config::env
cargo test --manifest-path src-tauri/Cargo.toml commands::config

# Then independent implementation tasks:
# T008 -> src-tauri/src/llm/transport.rs
# T009 -> src-tauri/src/llm/gemini.rs
# T011 -> README.md and .env.example
```

## Parallel Example: User Story 2

```bash
# T013-T016 edit the same file, so run them in order:
cargo test --manifest-path src-tauri/Cargo.toml --test profile_pipeline

# The frontend path starts after the backend contract is defined:
# T021 -> src/lib/types.ts and src/lib/ipc.ts
# T022 -> src/components/ResumeManager.test.tsx
# T023 -> src/viewmodels/useResumeViewModel.ts and src/components/ResumeManager.tsx
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete T001 baseline.
2. Complete US1 tests and implementation (T002-T012).
3. **STOP and VALIDATE**: confirm the active model is current and a live
   evaluation/extraction no longer returns 404.
4. Deploy or demo the provider fix.

### Incremental Delivery

1. Deliver US1 so the app can call a provider again.
2. Deliver US2 so a failed extraction never loses the uploaded PDF.
3. Run the Polish phase and the full local gate.
4. Each story is independently testable and leaves the other path untouched.

---

## Notes

- `[P]` tasks touch different files and can be assigned in parallel.
- Model IDs are provider defaults, not hard guarantees. Operators can override
  them with `GEMINI_MODEL`, `OPENROUTER_FREE_MODELS`, and `DEEPSEEK_MODEL`
  without a code change.
- Use `gemini-3.5-flash-lite` as the default for extraction. If higher quality
  is needed, `gemini-3.8-flash` is a supported override; it is the documented
  replacement for `gemini-2.5-pro`, not for `gemini-2.5-flash`.
- Do not log or commit `.env`, API keys, full provider responses, resume text,
  or filesystem paths.
- The resume row is intentionally persisted with `extracted_json = NULL` on
  failure. `delete_resume_with` skips NULL rows when rebuilding and returns
  `None` when no active profile exists, so deleting such a resume remains safe.
- Recovery after a failed extraction currently requires uploading a different
  PDF or deleting and re-adding the same file; a retry-on-duplicate flow for a
  NULL row is a possible follow-up, not part of this bugfix scope.

## Implementation Status (2026-11-11)

All implementation tasks in this file were completed and validated with
`./scripts/verify.sh` (Rust fmt/check/clippy/tests, frontend format/lint/
typecheck/tests, and sidecar pytest). T027 was covered by automated acceptance
tests and live provider smoke checks; the interactive GUI upload flow was not
repeated manually.
