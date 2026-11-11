# LinkedIn Scraper MCP

Local MCP wrapper around [`joeyism/linkedin_scraper`](https://github.com/joeyism/linkedin_scraper).
It gives OpenCode a second, independent LinkedIn source alongside
`stickerdaniel/linkedin-mcp-server`.

This is a **development/agent tool**, not part of the Rust application runtime.
It is not started by the app and its dependencies are not part of the Tauri
bundle.

## Tools

- `search_jobs` — search LinkedIn with `keywords`/`query`, `location`,
  `limit`/`results_wanted`, and `fetch_details` (default `true`). With details
  enabled, it returns the Jobs Search `RawJob`-compatible fields (`title`,
  `company`, `location`, `work_modality`, `url`, `description`,
  `source_name`, `external_id`, `tags`). The app's generic MCP adapter can
  consume this source directly.
- `get_job_details` — read a LinkedIn job URL and return title, company,
  location, posted date, applicant count, and description.
- `get_session_status` — report whether the saved session exists and has
  `0600` permissions.

## Setup

```bash
cd sidecar/linkedin_scraper_mcp
uv sync
uv run playwright install chromium
uv run linkedin-scraper-login
```

The login command opens a browser. Sign in manually, including any 2FA step.
The Playwright storage state is saved to:

```text
~/.linkedin-mcp/joeyism-session.json
```

To reuse an existing `li_at` cookie instead of opening the login browser:

```bash
uv run linkedin-scraper-login --cookie
```

The cookie prompt hides the input. The cookie is never written to `.env` or
the repository; only the session file (which contains cookies) is written, with
mode `0600`.

Use `LINKEDIN_SESSION_FILE` to choose a different session path. Use
`LINKEDIN_HEADLESS=false` if LinkedIn blocks the headless browser.

## OpenCode configuration

The project `opencode.jsonc` registers this wrapper as `linkedin-scraper`:

```jsonc
"linkedin-scraper": {
  "type": "local",
  "command": [
    "uv",
    "run",
    "--project",
    "sidecar/linkedin_scraper_mcp",
    "linkedin-scraper-mcp"
  ],
  "cwd": ".",
  "timeout": {
    "startup": 300000,
    "catalog": 120000,
    "execution": 900000
  }
}
```

Tools are exposed as `linkedin_scraper_search_jobs`,
`linkedin_scraper_get_job_details`, and
`linkedin_scraper_get_session_status`.

## Tests

The MCP protocol tests do not need `linkedin_scraper` or a browser:

```bash
pytest sidecar/linkedin_scraper_mcp/tests
```

## Security and terms

- The session file is equivalent to a LinkedIn password. Never commit it, log
  it, paste it into an issue, or store it in `.env`.
- Prefer a dedicated browser profile and a stable IP address. LinkedIn can
  checkpoint or ban accounts that automate authenticated scraping.
- Respect rate limits and use small result limits.
- Scraping authenticated LinkedIn pages may violate LinkedIn's User Agreement.
  You are responsible for deciding whether to use this tool.

## License

`linkedin_scraper` is published on PyPI with Apache-2.0 metadata, but the
repository's `LICENSE` file is GPL-3.0. Treat the dependency as **GPL-3.0**
until the upstream licensing is clarified. Keep this wrapper as an optional
development tool and do not bundle it into a distributed application without
legal review.
