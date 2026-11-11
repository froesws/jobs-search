# Feature Specification: LLM Provider Configuration Popup

**Feature Branch**: `[014-provider-config-popup]`

**Created**: 2026-11-11

**Status**: Draft (decisions closed via `/questiona`)

**Input**: User description: "add a new feature: a popup to permit config api keys, runs, select the main
model. For now we have only to use gemini, openrouter, deepseek and local ollama, we should pass info for
connect to this servers, adjust how many tries before fail, select main model, select available models, the
pop will be opened clicking in a gear button on the right side of the <ProviderIndicator /> the
ProviderIndicator per si, must be a button when clicked show a list of availables models on the providers
selected"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Configure provider credentials and pick the working provider chain (Priority: P1)

The System Operator opens a provider configuration popup from the top bar, enters the API key for one or
more supported providers, configures the local Ollama endpoint, and confirms which provider is used first.
The change takes effect on the next run without restarting the application.

**Why this priority**: No LLM-driven work (job evaluation or profile extraction) can happen without valid
credentials, and the provider order determines which backend actually runs.

**Independent Test**: Open the popup, save an API key for one provider, close the popup, and confirm the
top-bar indicator shows the newly configured provider without an application restart.

**Acceptance Scenarios**:

1. **Given** no provider is configured, **When** the operator saves a valid API key for a supported
   provider, **Then** that provider becomes usable and appears in the provider chain.
2. **Given** a provider already has a stored credential, **When** the operator reopens the popup, **Then**
   the credential is shown as configured without revealing its value.
3. **Given** the operator saves a change, **When** the popup closes, **Then** the new configuration is used
   by the next run without restarting the application.

---

### User Story 2 - Select the model used for each provider (Priority: P2)

The operator chooses one model per provider. For OpenRouter specifically, the operator maintains an ordered
list of models, and each attempt uses the next model in that list.

**Why this priority**: Providers retire model identifiers frequently; without an in-app model selection the
operator is forced to edit environment files and restart.

**Independent Test**: With a provider configured, select a model, run an evaluation, and confirm the run
reports the selected model. For OpenRouter, order two models and confirm the first failing attempt moves to
the second model.

**Acceptance Scenarios**:

1. **Given** a configured provider, **When** the operator selects a model and saves, **Then** subsequent runs
   use that model.
2. **Given** OpenRouter is configured with an ordered model list, **When** the first model fails with a
   retryable error, **Then** the next attempt uses the next model in the list before leaving the provider.
3. **Given** the model list lacks the desired model, **When** the operator enters a custom model identifier,
   **Then** it can be selected and persisted.

---

### User Story 3 - Tune attempts and provider fallback (Priority: P3)

The operator adjusts how many attempts each provider gets before a run fails, controls whether fallback
across providers is enabled, and reorders the provider chain.

**Why this priority**: Flaky or rate-limited endpoints need a bounded retry budget, and the operator needs
control over whether a failing provider is replaced by the next one.

**Independent Test**: Set a retry count for a provider, force a transient failure, and confirm the number of
attempts matches the configured value both with fallback on and off.

**Acceptance Scenarios**:

1. **Given** a provider is configured with N attempts, **When** every attempt fails with a retryable error,
   **Then** the provider is attempted exactly N times.
2. **Given** fallback is disabled, **When** the preferred provider exhausts its attempts, **Then** the run
   fails without trying another provider — but still uses all N configured attempts.
3. **Given** fallback is enabled, **When** a provider exhausts its attempts, **Then** the next provider in
   the configured order is tried.
4. **Given** Ollama is not enabled, **When** a run executes, **Then** Ollama is never contacted.

---

### User Story 4 - Switch the active model quickly from the top bar (Priority: P4)

The provider indicator in the top bar is a button. Clicking it lists the models available on the configured
providers, grouped by provider. Choosing a model applies it for the current session only.

**Why this priority**: Quick experimentation without committing a change, and without opening the full
configuration popup.

**Independent Test**: Click the indicator, choose a different model, run an evaluation, and confirm the run
uses it. Restart the application and confirm the previously saved configuration is back.

**Acceptance Scenarios**:

1. **Given** at least one provider is configured, **When** the operator clicks the indicator, **Then** a menu
   lists the models of the configured providers grouped by provider.
2. **Given** the menu is open, **When** the operator chooses a model, **Then** that model becomes active for
   the session and the indicator reflects it.
3. **Given** a session-only model is active, **When** the operator saves configuration in the popup, **Then**
   the session-only selection is discarded and the saved value applies.
4. **Given** the active provider is OpenRouter, **When** the operator chooses a model from the menu, **Then**
   that model is promoted to the first position of the attempt order.

---

### Edge Cases

- The operator submits an empty or invalid API key: the popup MUST reject the save with a clear message and
  a "Test connection" action to diagnose it.
- The OS credential store is unavailable: the popup MUST ask for explicit consent before writing the
  credential to the plaintext local environment file, and MUST allow cancelling.
- A provider's model listing fails (offline, invalid key, rate limit): the popup MUST fall back to the
  previously fetched list, then to the built-in list, and MUST state which list is in use.
- The local Ollama server is stopped but the Ollama toggle is on: the run reports a connection failure for
  Ollama without corrupting the rest of the run.
- The operator removes a credential while a run is in progress: the in-flight run completes with the
  configuration it started with; the next run uses the new configuration.
- The operator resets model and attempts to defaults: the stored credential MUST remain intact unless the
  operator explicitly removes it.
- A provider returns models that cannot serve chat completions (embeddings, vision, audio): those MUST be
  excluded from the selectable list.

## Requirements *(mandatory)*

### Functional Requirements

**Popup entry points**

- **FR-001**: The top bar MUST render a gear button immediately to the right of the provider indicator;
  activating it MUST open the provider configuration popup.
- **FR-002**: The provider indicator MUST be an interactive button; activating it MUST open a menu listing
  available models grouped by configured provider. When no provider is configured, the menu MUST
  offer an entry that opens the configuration popup.

**Credentials and connection**

- **FR-003**: The popup MUST accept an API key for each supported cloud provider: Gemini, OpenRouter and
  DeepSeek.
- **FR-004**: The popup MUST accept a base endpoint for the local Ollama provider, pre-filled with the
  conventional local default, and MUST NOT require an API key for it.
- **FR-005**: Stored credentials MUST never be displayed, logged, or written to reports after saving; the UI
  MUST show only a configured/not-configured state.
- **FR-006**: Credentials MUST be stored in the operating system credential store by default. The
  application MUST NOT write to or remove entries from the local environment file without an
  explicit operator consent prompt; when the credential store is unavailable, the popup MUST offer
  that consent and MUST let the operator cancel.
- **FR-007**: The popup MUST provide a per-provider "Test connection" action that performs a minimal real
  request and reports success or a sanitized failure reason.
- **FR-008**: The popup MUST provide a per-provider "Remove credential" action that deletes the stored
  credential without altering any other setting, and MUST clear any session-only selection that
  points at that provider.

**Model selection**

- **FR-009**: The popup MUST let the operator choose exactly one model per provider, except OpenRouter,
  where the operator maintains an ordered list of models.
- **FR-010**: For OpenRouter, each attempt MUST use the next model in the ordered list before the router
  leaves the provider.
- **FR-011**: The popup MUST offer a "Refresh models" action for each cloud provider and for the
  local Ollama server; the Ollama list MUST be fetched automatically when the dialog opens, with a
  short timeout and a free-text fallback when the server is unreachable.
- **FR-012**: The model list shown to the operator MUST resolve in this order: last successfully
  fetched cache, then the built-in list, then a free-text custom model identifier.
- **FR-013**: The operator MUST be able to enter an arbitrary custom model identifier when the catalog does
  not contain it.
- **FR-014**: Only models capable of chat completions MUST be offered for selection. For OpenRouter,
  the list MUST default to models with zero prompt and completion price, with an explicit
  session-only option to show all chat models together with their price.

**Attempts and fallback**

- **FR-015**: The popup MUST let the operator set the number of attempts for Gemini, DeepSeek and
  Ollama. For OpenRouter the attempt count is derived from the ordered model list and is shown
  read-only.
- **FR-016**: The configured attempt count MUST be honored whether provider fallback is enabled or disabled.
- **FR-017**: The popup MUST expose an on/off control for cross-provider fallback.
- **FR-018**: The popup MUST let the operator order the provider chain; the first entry is the preferred
  provider and its selected model is the primary model.
- **FR-019**: Ollama MUST only participate in the chain when the operator explicitly enables it. When
  enabled, the model MUST default to the first model reported by the local server; if the server
  reports no models, enabling MUST be blocked with an actionable message.

**Persistence and precedence**

- **FR-020**: Non-secret provider settings MUST be persisted in the local application database.
- **FR-021**: Saved settings MUST take precedence over values from the local environment file, which remains
  the factory default.
- **FR-022**: Each individual setting MUST be resettable to its environment/default value without affecting
  the other settings, and resetting a setting MUST NOT delete the stored credential.
- **FR-023**: Saved changes MUST apply to the next run without restarting the application.

**Session override**

- **FR-024**: A model chosen from the indicator menu MUST apply for the current session only and MUST NOT be
  persisted.
- **FR-025**: Saving configuration in the popup MUST discard any active session-only selection.

### Key Entities *(include if feature involves data)*

- **Provider Setting**: the operator-editable configuration of one provider. Holds the selected model (or
  the ordered model list for OpenRouter), the base endpoint, the attempt count, and whether the provider is
  enabled in the chain. Never holds the credential value.
- **Provider Credential**: the secret that authenticates a provider. Stored in the operating system
  credential store, with the local environment file as a consented fallback. Never rendered after saving.
- **Provider Chain**: the ordered list of providers that a run may try, plus the fallback-enabled flag. The
  first enabled provider is the preferred provider.
- **Model Catalog Cache**: the most recent model list retrieved from a provider, retained so the popup stays
  responsive and usable offline.
- **Session Selection**: an in-memory, non-persisted override of the active provider and model, discarded on
  application exit or when configuration is saved.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: The operator can configure a provider credential and select its model entirely through the
  application interface, without editing files or restarting.
- **SC-002**: 100% of saved setting changes are in effect on the next run, with zero application restarts
  required.
- **SC-003**: The model list is populated within 1 second of opening the popup, even with no network access.
- **SC-004**: A refresh that fails for any reason never leaves the operator without a usable model list.
- **SC-005**: A run against a provider configured with N attempts performs exactly N attempts before
  reporting failure, regardless of the fallback setting.
- **SC-006**: Zero credential values appear in the interface after saving, in application logs, in
  telemetry, or in exported reports.

## Out of Scope

- OAuth or account-based sign-in with any provider.
- Using more than one provider concurrently within a single evaluation.
- Automatic rotation of API keys.
- Automatic background refresh of model catalogs without operator action.

## Assumptions

- Only Gemini, OpenRouter, DeepSeek and local Ollama are supported in this version.
- The local Ollama endpoint defaults to its conventional local address and is user-editable.
- The local environment file remains the bootstrap and factory-default source of configuration.
- The built-in model lists are curated snapshots and may lag behind provider catalogs; the "Refresh models"
  action and the custom model field cover the gap.
- Interactive feedback for OS credential-store availability is acceptable as a blocking prompt at save time.
- Existing retry error classification (retryable vs fatal) is preserved; fatal errors such as an invalid key
  continue to abort without trying another provider.
