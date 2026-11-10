# Jobs Search

Local Linux desktop app for discovering, deduplicating, scoring, and tracking
job opportunities. The app ingests PDF resumes, builds one active candidate
profile, searches through a hybrid MCP JobSpy and Rust-native API ingestion
layer, evaluates jobs with hard filters plus LLM-assisted evidence extraction,
and presents ranked jobs in a MUI dashboard.

## Status

Specification and planning are complete for the MVP. The application source has
not been created yet. The current implementation plan is
[`specs/004-telemetry-execution-logging/plan.md`](specs/004-telemetry-execution-logging/plan.md).

## Stack

- Tauri `2.12.5`, Rust `1.90+`, `rusqlite` `0.41.1`, `tokio` `1.54.1`,
  `reqwest` `0.13.5`
- React `19.3.1`, TypeScript `7.0.2`, Vite `8.3.6`
- MUI `9.5` + Emotion
- Python `3.14` sidecar for the JobSpy MCP server
- SQLite for transactional data and analytics metrics
- Local rotating logs via `tracing`/`tracing-appender`

No Tailwind, TanStack Query, Zustand, MongoDB, or external telemetry.

## Repository layout

```text
src/              React/MUI frontend
src-tauri/        Rust/Tauri backend
sidecar/          Python JobSpy MCP sidecar
specs/            Spec Kit feature specifications and plan
.specify/         Spec Kit project configuration
.opencode/        OpenCode commands, rules, and skills
```

## Prerequisites

- Linux
- Node.js `24+`
- Rust `1.90+`
- Python `3.14`
- Tauri system dependencies for Linux
- OS-level disk encryption enabled and filesystem permissions restricted

The app does not add application-level encryption at rest. Profile, resume,
application, and log data rely on the OS disk encryption and filesystem
permissions described in the constitution.

## Configuration

Create a local `.env` in the repository root for development. It is ignored by
Git and must have restricted permissions (`chmod 600`).

```dotenv
GEMINI_API_KEY=
OPENROUTER_API_KEY=
DEEPSEEK_API_KEY=
LLM_PROVIDER_ORDER=gemini,openrouter,deepseek
OPENROUTER_FREE_MODELS=
RUST_LOG=info
```

In production, the app reads `.env` from the application data directory first,
falls back to the current directory, and lets OS environment variables take
precedence. Do not commit `.env` or provider credentials.

Job sources are configured in `sources.yaml` (MCP and API adapters). Keep local
source credentials in `.env`.

## Commands (planned)

```bash
npm install
npm run typecheck
npm test
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
pytest sidecar/jobspy_mcp/tests
```

## Commit policy

Each completed task from `tasks.md` gets its own commit, using Conventional
Commits with the task ID, for example:

```text
feat(T012): add Gemini provider adapter
```

Run the Rust, frontend, and Python checks before committing. The initial commit
contains only `.gitignore`, `requirements.txt`, and this README; specs and
Spec Kit/OpenCode configuration follow in a separate commit.

## Specifications

- [001 — Career Intelligence & Job Sourcing System](specs/001-career-job-sourcing/spec.md)
- [002 — Job Sourcing Dashboard & Application Tracker](specs/002-job-sourcing-dashboard/spec.md)
- [003 — Skill Gap Analytics](specs/003-skill-gap-analytics/spec.md)
- [004 — Local Telemetry & Execution Logging](specs/004-telemetry-execution-logging/spec.md)
