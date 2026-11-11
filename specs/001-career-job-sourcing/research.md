# Research: Career Intelligence & Job Sourcing Core

**Feature**: `001-career-job-sourcing`
**Date**: 2026-11-10

All items below are resolved. No `NEEDS CLARIFICATION` remains.

## Decision: PDF text extraction

**Decision**: Use `pdf-extract` for text extraction, then send the extracted text
to the active LLM for structured profile mapping.

**Rationale**: The V1 scope fixes `pdf-extract`; it is purpose-built for text
extraction and keeps the PDF work in Rust. Image-only/encrypted/corrupt PDFs fail
with a clear error; OCR is out of scope.

**Alternatives considered**:
- `lopdf` as the primary extractor: lower-level and more work for layout-heavy
  resumes.
- `pdftotext`/Poppler: robust, but adds an external binary and packaging burden.
- OCR fallback: out of scope for v1.

## Decision: Profile extraction and merge

**Decision**: The LLM returns a strict `ExtractedProfile` JSON object. The app
validates it, unions list-valued fields, flags conflicting scalar fields, and
persists one active profile with a monotonically increasing profile version.

**Rationale**: A single active profile is the V1 scope. Keeping IDs, merge rules,
and conflict flags in the app prevents the model from inventing identity or
silently overwriting operator edits.

**Alternatives considered**:
- Let the LLM write the final profile directly: rejected because it bypasses
  validation and conflict review.
- Multiple named profiles: deferred; V1 is single-operator with one profile.

## Decision: Hybrid ingestion

**Decision**: Implement one ingestion contract with two adapters:
- an MCP stdio client for `jobspy_mcp_server`;
- a Rust `reqwest` adapter for a public job API (Remotive/RemoteOK provisional).

Sources are configured in `sources.yaml`. Adapters return normalized postings
with source lineage and contain no matching, scoring, or filtering rules.

**Rationale**: The hybrid validates both ingestion vectors while preserving the
constitution's source abstraction. Rate limits and retries live in the adapter
layer, with a central call limit in Rust.

**Alternatives considered**:
- Python JobSpy subprocess without MCP: less reusable and outside the chosen
  source contract.
- Two MCP servers: redundant for the MVP.
- Direct browser scraping: fragile and subject to anti-bot/ToS constraints.

## Decision: Deduplication identity

**Decision**: Derive a stable canonical identity from normalized company name,
normalized title, and primary location. If location is missing, fall back to the
normalized posting URL. Compute a SHA-256 hash of the canonical key for indexing.
Keep every distinct source URL when the job passes hard filters.

**Rationale**: URL-only identity fails across sources; company+title+location
deduplicates the common cross-source case, with URL as a safe fallback.

**Alternatives considered**:
- URL-only hash: precise but weak across sources.
- Title-only matching: too many collisions.
- Location+URL always: fails when one source omits location.

## Decision: Match hard filters

**Decision**: Hard filters run before scoring:
- location: remote Brazil or on-site/hybrid in Porto Alegre/Curitiba;
- seniority: Trainee, Junior, Pleno, including Pleno/Senior ranges;
- technology: at least one core technology from the merged profile/manual list,
  except Trainee/Intern;
- mandatory domain competency dealbreakers.

Work authorization/visa and salary are not hard filters in V1. Unknown mandatory
criteria are not treated as matches and keep the job out of the approved list.

**Rationale**: The filters reflect the operator's explicit constraints and are
deterministic, testable, and independent of the LLM.

## Decision: Scoring and N/M counts

**Decision**: The LLM extracts requirements and per-dimension evidence. The app
groups requirements into the four dimensions, counts matched versus total, applies
40/25/20/15, rounds to an integer 0-100, and stores the breakdown. Hard filters
remain a separate prior decision. The threshold is 70 and configurable.

**Rationale**: App-side arithmetic makes scores reproducible and auditable.
Matched + missing MUST equal total extracted requirements.

**Alternatives considered**:
- LLM computes the final score: non-deterministic and hard to reproduce.
- Keyword-only score: loses semantic evidence.

## Decision: Evaluation and lifecycle states

**Decision**: Keep `evaluation_status` (`PENDING`, `APPROVED`, `REJECTED`,
`ERROR`) separate from `lifecycle_state` (`DISCOVERED`, `APPLIED`, `IGNORED`).
Store append-only `job_status_events`. Manual approval sets
`approval_source = manual` and preserves the score/breakdown.

**Rationale**: A rejected job never enters the application funnel; applied and
ignored jobs remain distinguishable. The append-only history satisfies the
constitution and the audit requirements.

## Decision: LLM providers and fallback

**Decision**: Provider order is Gemini → OpenRouter free list (2–3 models) →
DeepSeek. Fallback only on 429, timeout, 5xx, or malformed output; authentication
and configuration errors stop and surface. Run at most 2 evaluations in parallel
and evaluate at most 100 jobs per run by default.

**Rationale**: Free-tier quotas are a practical constraint; transient fallback
keeps runs moving without masking bad credentials. The cap controls latency and
cost.

## Decision: Persistence and privacy

**Decision**: Use SQLite via `rusqlite` with embedded migrations. Store only the
clean description and extracted metadata; never persist `description_raw`. Keep
all data local, rely on OS disk encryption and permissions, and keep API keys in
`.env`.

**Rationale**: Constitution v2.0.0 and spec `001` require local-only data and no
raw-description persistence. The clean text is sufficient for matching and
audit.

## Decision: Testing strategy

**Decision**: Rust unit tests plus contract/integration tests for migrations,
Tauri commands, MCP JSON-RPC, LLM schemas, dedup identity, filters, scoring, and
audit/apply flows. Python `pytest` for the sidecar. Frontend tests belong to
`002`.

**Rationale**: The backend contracts are the highest-risk integration seams. The
MCP sidecar and LLM schemas need explicit contract tests before the UI consumes
them.

**Alternatives considered**:
- End-to-end UI tests in this feature: premature; the UI is `002`.
- Mock-only tests: insufficient for SQLite migrations and MCP framing.
