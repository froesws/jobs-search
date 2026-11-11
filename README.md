# Jobs Search

Local Linux desktop app for discovering, deduplicating, scoring, and tracking
job opportunities. The app ingests PDF resumes, builds one active candidate
profile, searches through a hybrid MCP JobSpy and Rust-native API ingestion
layer, evaluates jobs with hard filters plus LLM-assisted evidence extraction,
and presents ranked jobs in a MUI dashboard.

## Status

The `001` backend core and the read-only `002` dashboard are implemented:
PDF resume ingestion and profile merging, hybrid MCP/API discovery, deduplication,
hard filters, deterministic scoring, application lifecycle, execution audit, and
a React/MUI dark-first dashboard. Analytics (`003`) and full runtime packaging of
the Python sidecar remain in progress.

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
specs/            Spec Kit feature docs (local-only; ignored by Git)
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
LLM_PROVIDER=
LLM_PROVIDER_ORDER=gemini,openrouter,deepseek
GEMINI_API_KEY=
GEMINI_MODEL=gemini-3.5-flash-lite
OPENROUTER_API_KEY=
OPENROUTER_FREE_MODELS=openrouter/free
DEEPSEEK_API_KEY=
DEEPSEEK_MODEL=deepseek-flash
DEEPSEEK_BASE_URL=https://api.deepseek.com
OLLAMA_BASE_URL=http://localhost:11434
OLLAMA_MODEL=
LLM_ALLOW_FALLBACK=true
LLM_TIMEOUT_SECONDS=45
MATCH_THRESHOLD=50
JOBS_SEARCH_MAX_EVALUATIONS=100
JOBS_SEARCH_LLM_CONCURRENCY=2
RUST_LOG=info
```

`LLM_PROVIDER` selects the preferred provider (`gemini`, `openrouter`, or
`deepseek`) and moves it to the front of `LLM_PROVIDER_ORDER`; the remaining
providers stay available as fallbacks. Leave it empty to use the order as-is.
The current defaults are `gemini-3.5-flash-lite` (the documented replacement
for the retired `gemini-2.5-flash`; `gemini-3.8-flash` is a higher-quality
override), `openrouter/free`, and `deepseek-flash` at
`https://api.deepseek.com`. Override them with `GEMINI_MODEL`,
`OPENROUTER_FREE_MODELS`, and `DEEPSEEK_MODEL` without changing code. The
example enables fallback; set `LLM_ALLOW_FALLBACK=false` to pin a single
provider.

In production, the app reads `.env` from the application data directory first,
falls back to the current directory, and lets OS environment variables take
precedence. Do not commit `.env` or provider credentials.

Ollama runs locally and needs no API key. Set `OLLAMA_BASE_URL` (default
`http://localhost:11434`) and `OLLAMA_MODEL`; Ollama only joins the provider
chain when it is enabled in the provider configuration popup.

Job sources are configured in `sources.yaml` (MCP and API adapters). Keep local
source credentials in `.env`.

## Job sources

The default search runs all enabled sources concurrently:

- `jobspy` (MCP): LinkedIn, Indeed, Glassdoor, ZipRecruiter, Google, Bayt,
  Naukri, BDJobs, and HelloWork through `python-jobspy`. A failing board is
  reported as a partial sidecar result instead of discarding the other boards.
- `remoteok` (API): RemoteOK's public feed.
- `linkedin` (MCP): the authenticated
  `stickerdaniel/linkedin-mcp-server` server, using its own browser session
  under `~/.linkedin-mcp`.
- `linkedin_scraper` (MCP): the local
  `sidecar/linkedin_scraper_mcp` wrapper around `joeyism/linkedin_scraper`. It
  needs a one-time `linkedin-scraper-login` and is subject to LinkedIn rate
  limits and account restrictions.

The two authenticated LinkedIn sources are optional. Set `enabled: false` in
`sources.yaml` to remove either one without affecting JobSpy or RemoteOK.
JobSpy and the local wrapper are started with `uv run --project`, so `uv` must
be available in the environment that starts the app.

JobSpy fetches posting bodies by default (`fetch_descriptions: true`); LinkedIn
and Glassdoor otherwise return metadata only, leaving the hard filters and the
LLM with just a title. Fetching descriptions is slower and more likely to hit a
board's rate limit, so set `fetch_descriptions: false` for title-only results.

## Search filters

The search bar uses two independent multi-select panels:

- Seniority: Trainee, Junior, Pleno, Senior. There is no “Any”; an empty panel
  uses the profile's accepted seniorities.
- Work type: On-site, Hybrid, Remote. There is no “All”; an empty panel uses the
  profile/default modalities.

The UI labels are English, but the external source query terms are Portuguese:
`trainee`, `júnior`, `pleno`, `sênior`, `presencial`, `híbrido`, `remoto`. One
query is generated per selected seniority, for example
`Rust Engineer pleno (remoto OR híbrido)`. The hard filters enforce the exact
selected levels and modalities before the LLM call: Hybrid/Onsite require a
city from the profile's accepted cities, Remote requires a Brazil/Brasil
mention, and `is_remote` is true only for a Remote-only selection. Unknown
metadata never rejects a posting on its own: an absent modality, location, or
seniority is a gap to review, and an unknown location is kept only when no
foreign region is named; `NO_CORE_TECHNOLOGY` only applies when the description
is present but lacks an accepted technology.

The job drawer opens the original posting URL in the default browser via an
"Open original posting" button, with the normalized URL kept visible and
selectable below it as a copy fallback. The Seniority and Work type footers keep
their line even when a value is selected, so the filter row and the cards below
stay aligned. Only postings whose location proves Brazil are accepted by default.

## Analytics

The `Analytics` tab is read-only. It aggregates eligible evaluations
(`APPROVED`/`PENDING`) for the latest completed run or the last 7 days:

- Skill Gap: missing keywords grouped case-insensitively, excluding skills in
  the active profile or matched for the same job. Ranked by demand count, then
  display name; percentages use the full eligible-job denominator.
- Funnel: ingested, duplicates, rejections by reason, pending, approved,
  applied, ignored, rates, and source/provider breakdowns.
- Score distribution: 0-49, 50-59, 60-69, 70-79, 80-89, 90-100.
- LLM metrics: call/success/error/fallback counts and p50/p95 latency.
- CSV export writes the current view locally after a sensitive-data warning.
  It contains `description_clean`, never `description_raw` or credentials.

Analytics never calls a provider and never mutates jobs, evaluations, or
applications. It reads `execution_runs`, `run_metrics`, `llm_metrics`, and
`job_evaluations`; `job_evaluations.run_id` links an evaluation to its run.

## Local telemetry

The backend writes local JSON Lines logs to the application data directory
(`logs/jobs-search.YYYY-MM-DD.log`). Logs rotate daily and are retained for at
most 7 days or 50 MB. The default level is INFO; set `RUST_LOG=debug` for
DEBUG. The health command `cmd_get_log_health` returns queue depth, dropped
count, last error, and the current file.

Logs never contain credentials, raw payloads, model prompts/responses, URLs,
filesystem paths, job descriptions, or candidate data. No MongoDB or external
telemetry is used.

## MCP troubleshooting

An MCP server that stops with exit code `143` received `SIGTERM` (`128 + 15`).
That is an orderly shutdown from a supervisor, a configuration reload, or a
manual cleanup — not a sidecar crash. The FastMCP banner, the `uv` environment
lines, and `Starting MCP server` are startup diagnostics. Reload the client or
restart the app to launch the server again.

The desktop app reads `sources.yaml`; OpenCode reads `.opencode/opencode.jsonc`.
The `linkedin` and `linkedin-scraper` entries there are independent from the
app's sources and can be disabled if duplicate servers are not wanted. The
`linkedin` server is the third-party `mcp-server-linkedin` package and cannot
trap signals in this repository; `linkedin-scraper-mcp` now exits `0` on
`SIGTERM`/`SIGINT`. The `VIRTUAL_ENV=.venv does not match` warning is benign;
app-spawned sidecars no longer inherit `VIRTUAL_ENV`.

To stop a sidecar safely, list `pid`, `ppid`, and `pgid` first
(`ps -o pid,ppid,pgid,args`) and confirm the parent is the app or
`opencode serve`. Never use `pkill -f linkedin` or kill by command name; it can
terminate an unrelated server that shares the same name.

## Commands

```bash
npm install
npm run dev                 # Vite frontend
npm run tauri dev           # desktop shell
npm run build               # frontend production build
npm run tauri build -- --debug --no-bundle
npm test                    # Vitest
npm run typecheck
./scripts/verify.sh
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
pytest sidecar/jobspy_mcp/tests
```

The pre-commit hook runs `lint-staged` and `./scripts/verify.sh`. Enable it once
per clone with `git config core.hooksPath .husky/_` if `npm install` did not set
it. Do not bypass the hook with `--no-verify`.

## Backend commands

Profile: `cmd_get_profile`, `cmd_upload_resume`, `cmd_delete_resume`,
`cmd_save_profile`.
Search/jobs: `cmd_trigger_search`, `cmd_get_jobs`, `cmd_get_job`,
`cmd_apply_job`, `cmd_ignore_job`, `cmd_restore_job`, `cmd_approve_job`,
`cmd_get_audit`.
Health: `cmd_get_log_health`.

All commands return `{ code, message }` errors and never expose raw job
descriptions, credentials, prompts, or filesystem paths.

## Resume uploads

Uploading a PDF validates the file, hashes it, extracts its text, writes the
file under `resumes/`, and inserts the `resumes` row before calling the LLM. If
extraction fails, the command returns a partial success with a warning and
keeps the file and row with `extracted_json = NULL`. The resume remains listed
and can be deleted or replaced; deleting such a resume when no active profile
exists succeeds without an error. Fix the provider configuration and upload a
different PDF (or delete and re-add the same file) to rebuild the profile.
Duplicate uploads are reported as duplicates and are not reprocessed.

## Local data and privacy

Profiles, stored resumes, jobs, evaluations, applications, and audit records
live in local SQLite under the Tauri application data directory. Resume files
are stored under `resumes/` with `0600` permissions. Raw job descriptions are
never persisted; only clean descriptions and extracted metadata are stored.
Provider keys stay in `.env`; logs and reports contain no credentials or raw
payloads.

## Development seed

Tests use in-memory SQLite and generated PDF fixtures. The first-milestone
profile fixture lives at `src-tauri/tests/fixtures/profile.json` and can be
loaded with `JOBS_SEARCH_FIXTURE_PROFILE=/absolute/path/to/profile.json`.

## Commit policy

Each completed task from `tasks.md` gets its own commit, using Conventional
Commits with the task ID, for example:

```text
feat(T012): add Gemini provider adapter
```

Run the Rust, frontend, and Python checks before committing. Spec Kit,
OpenCode, Husky, and editor configuration are local-only and ignored by Git;
the feature documents under `specs/` are versioned with the code.

## Specifications

The Spec Kit feature documents under `specs/` are versioned so the design,
requirements, and task history can be reviewed alongside the implementation.

The local Spec Kit workspace (`.specify/`), OpenCode configuration
(`.opencode/`), Husky hooks, and editor configuration remain local-only and
are ignored by Git. A clean clone contains the application source, tests,
sidecar, `sources.yaml`, and the `specs/` documents.
