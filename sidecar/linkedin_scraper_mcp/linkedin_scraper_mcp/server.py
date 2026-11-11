"""Line-delimited JSON-RPC MCP server for joeyism/linkedin_scraper.

stdout carries protocol messages only; diagnostics go to stderr. The
linkedin_scraper package is imported lazily so the MCP handshake and tests
work before the optional browser stack is installed.
"""

from __future__ import annotations

import asyncio
import json
import os
import sys
from pathlib import Path
from typing import Any, Callable, TextIO

from . import __version__

PROTOCOL_VERSION = "2024-11-05"
DEFAULT_SESSION_FILE = Path("~/.linkedin-mcp/joeyism-session.json").expanduser()
DEFAULT_LIMIT = 10
MAX_LIMIT = 50
MAX_DETAIL_LIMIT = 10
SESSION_ENV = "LINKEDIN_SESSION_FILE"
SOURCE = "joeyism/linkedin_scraper"

TOOLS = [
    {
        "name": "search_jobs",
        "description": (
            "Search LinkedIn jobs with an authenticated browser session. "
            "Returns the Jobs Search RawJob-compatible fields when details are "
            "enabled; set fetch_details=false for URL-only results."
        ),
        "inputSchema": {
            "type": "object",
            "additionalProperties": True,
            "required": [],
            "properties": {
                "keywords": {"type": "string", "minLength": 1, "maxLength": 512},
                "query": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 512,
                    "description": "Alias used by the Jobs Search MCP adapter.",
                },
                "location": {"type": "string", "maxLength": 256},
                "limit": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": MAX_LIMIT,
                    "default": DEFAULT_LIMIT,
                },
                "results_wanted": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": MAX_LIMIT,
                    "description": "Alias used by the Jobs Search MCP adapter.",
                },
                "hours_old": {"type": "integer", "minimum": 1},
                "is_remote": {"type": "boolean"},
                "job_type": {"type": ["string", "null"]},
                "offset": {"type": "integer", "minimum": 0},
                "sites": {"type": "array", "items": {"type": "string"}},
                "fetch_details": {"type": "boolean", "default": True},
            },
        },
    },
    {
        "name": "get_job_details",
        "description": (
            "Read a LinkedIn job posting by URL using the authenticated browser "
            "session."
        ),
        "inputSchema": {
            "type": "object",
            "additionalProperties": False,
            "required": ["url"],
            "properties": {
                "url": {"type": "string", "minLength": 1, "maxLength": 2048},
            },
        },
    },
    {
        "name": "get_session_status",
        "description": (
            "Report whether the saved LinkedIn browser session exists and has "
            "safe permissions."
        ),
        "inputSchema": {
            "type": "object",
            "additionalProperties": False,
            "properties": {},
        },
    },
]


class SessionMissing(RuntimeError):
    """The saved LinkedIn session is missing or not private."""


class ScraperUnavailable(RuntimeError):
    """The linkedin_scraper dependency or browser runtime is unavailable."""


def session_file() -> Path:
    return Path(os.environ.get(SESSION_ENV, str(DEFAULT_SESSION_FILE))).expanduser()


def _require_session() -> Path:
    path = session_file()
    if not path.is_file():
        raise SessionMissing(
            "LinkedIn session is missing; run `linkedin-scraper-login` in "
            "sidecar/linkedin_scraper_mcp"
        )
    if os.name != "nt" and path.stat().st_mode & 0o077:
        raise SessionMissing(
            "LinkedIn session permissions are too open; run chmod 600 on the "
            "session file"
        )
    return path


def _import_scraper():
    try:
        from linkedin_scraper import BrowserManager, JobScraper, JobSearchScraper
    except Exception as error:  # pragma: no cover - optional browser dependency
        raise ScraperUnavailable(
            "linkedin-scraper is not installed; run `uv sync` in "
            "sidecar/linkedin_scraper_mcp"
        ) from error
    return BrowserManager, JobScraper, JobSearchScraper


def _headless() -> bool:
    return os.environ.get("LINKEDIN_HEADLESS", "true").strip().lower() not in {
        "0",
        "false",
        "no",
        "off",
    }


def _error_message(error: Exception) -> str:
    return type(error).__name__


def _linkedin_job_id(url: str) -> str:
    marker = "/jobs/view/"
    if marker in url:
        value = url.split(marker, 1)[1].split("/", 1)[0].split("?", 1)[0]
        if value:
            return f"linkedin-{value}"
    return ""


def _infer_modality(location: str, description: str) -> str | None:
    text = f"{location} {description}".lower()
    if "hybrid" in text or "híbrido" in text or "hibrido" in text:
        return "Hybrid"
    if "remote" in text or "remoto" in text or "remota" in text:
        return "Remote"
    if "on-site" in text or "onsite" in text or "presencial" in text:
        return "Onsite"
    return None


def _app_job(url: str, detail: dict[str, Any]) -> dict[str, Any] | None:
    title = str(detail.get("job_title") or "").strip()
    if not title:
        return None
    location = str(detail.get("location") or "Unknown").strip() or "Unknown"
    company = str(detail.get("company") or "Unknown").strip() or "Unknown"
    description = str(detail.get("job_description") or "").strip()
    return {
        "title": title,
        "company": company,
        "location": location,
        "work_modality": _infer_modality(location, description),
        "url": url,
        "posted_at": None,
        "description": description,
        "source_name": "linkedin",
        "external_id": _linkedin_job_id(url),
        "tags": [],
    }


async def _run_search(
    keywords: str,
    location: str | None,
    limit: int,
    fetch_details: bool,
) -> dict[str, Any]:
    path = _require_session()
    BrowserManager, JobScraper, JobSearchScraper = _import_scraper()
    limit = max(1, min(limit, MAX_LIMIT))
    if fetch_details:
        limit = min(limit, MAX_DETAIL_LIMIT)

    async with BrowserManager(headless=_headless()) as browser:
        await browser.load_session(str(path))
        urls = await JobSearchScraper(browser.page).search(
            keywords=keywords,
            location=location,
            limit=limit,
        )
        jobs: list[dict[str, Any]] = []
        if not fetch_details:
            jobs = [{"url": url, "source": SOURCE} for url in urls]
        else:
            details = JobScraper(browser.page)
            for url in urls:
                try:
                    scraped = await details.scrape(url)
                    job = _app_job(url, scraped.to_dict())
                    if job is not None:
                        jobs.append(job)
                except Exception:
                    continue
        return {"source": SOURCE, "count": len(jobs), "jobs": jobs}


async def _run_details(url: str) -> dict[str, Any]:
    path = _require_session()
    BrowserManager, JobScraper, _ = _import_scraper()
    if "linkedin.com/jobs" not in url:
        raise ValueError("url must be a LinkedIn job URL")
    async with BrowserManager(headless=_headless()) as browser:
        await browser.load_session(str(path))
        job = await JobScraper(browser.page).scrape(url)
        return {"source": SOURCE, "job": job.to_dict()}


def _session_status() -> dict[str, Any]:
    path = session_file()
    present = path.is_file()
    permissions_ok = bool(
        present and (os.name == "nt" or (path.stat().st_mode & 0o077) == 0)
    )
    return {
        "source": SOURCE,
        "session_file_present": present,
        "permissions_ok": permissions_ok,
    }


def _error(request_id: Any, code: int, message: str) -> dict[str, Any]:
    return {
        "jsonrpc": "2.0",
        "id": request_id,
        "error": {"code": code, "message": message},
    }


def _result(request_id: Any, result: Any) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": request_id, "result": result}


def _tool_result(request_id: Any, payload: Any) -> dict[str, Any]:
    return _result(
        request_id,
        {
            "content": [
                {
                    "type": "text",
                    "text": json.dumps(payload, ensure_ascii=False, default=str),
                }
            ],
            "isError": False,
        },
    )


def handle(
    request: Any,
    search: Callable[..., Any] | None = None,
    details: Callable[..., Any] | None = None,
    status: Callable[[], Any] | None = None,
) -> dict[str, Any] | None:
    """Handle one JSON-RPC message. Notifications return None."""

    if not isinstance(request, dict):
        return _error(None, -32600, "invalid request")

    method = request.get("method")
    request_id = request.get("id")
    search = search or _run_search
    details = details or _run_details
    status = status or _session_status

    if method == "initialize":
        return _result(
            request_id,
            {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {"tools": {}},
                "serverInfo": {
                    "name": "linkedin-scraper-mcp",
                    "version": __version__,
                },
            },
        )

    if method in {"notifications/initialized", "notifications/cancelled"}:
        return None

    if method == "ping":
        return _result(request_id, {})

    if method == "tools/list":
        return _result(request_id, {"tools": TOOLS})

    if method != "tools/call":
        return _error(request_id, -32601, "method not found")

    params = request.get("params") or {}
    if not isinstance(params, dict):
        return _error(request_id, -32602, "invalid params")
    name = params.get("name")
    arguments = params.get("arguments") or {}
    if not isinstance(arguments, dict):
        return _error(request_id, -32602, "invalid arguments")

    try:
        if name == "search_jobs":
            keywords = str(
                arguments.get("keywords") or arguments.get("query") or ""
            ).strip()
            if not keywords:
                return _error(request_id, -32602, "keywords or query is required")
            location_value = arguments.get("location")
            location = str(location_value).strip() if location_value else None
            limit = int(
                arguments.get("limit")
                or arguments.get("results_wanted")
                or DEFAULT_LIMIT
            )
            fetch_details = bool(arguments.get("fetch_details", True))
            payload = asyncio.run(search(keywords, location, limit, fetch_details))
            return _tool_result(request_id, payload)

        if name == "get_job_details":
            url = str(arguments.get("url") or "").strip()
            if not url:
                return _error(request_id, -32602, "url is required")
            payload = asyncio.run(details(url))
            return _tool_result(request_id, payload)

        if name == "get_session_status":
            return _tool_result(request_id, status())

        return _error(request_id, -32602, "unknown tool")
    except SessionMissing as error:
        return _error(request_id, -32001, str(error))
    except ScraperUnavailable as error:
        return _error(request_id, -32003, str(error))
    except ValueError as error:
        return _error(request_id, -32602, str(error))
    except Exception as error:  # noqa: BLE001 - MCP boundary
        _diagnostic(error)
        return _error(request_id, -32603, "LinkedIn scrape failed")


def _diagnostic(error: BaseException) -> None:
    print(f"LinkedIn scraper MCP error: {type(error).__name__}", file=sys.stderr)


def _write(stdout: TextIO, response: dict[str, Any]) -> bool:
    try:
        stdout.write(
            json.dumps(response, ensure_ascii=False, separators=(",", ":")) + "\n"
        )
        stdout.flush()
        return True
    except (BrokenPipeError, OSError) as error:
        _diagnostic(error)
        return False


def serve(
    stdin: TextIO = sys.stdin,
    stdout: TextIO = sys.stdout,
) -> None:
    for raw in stdin:
        line = raw.strip()
        if not line:
            continue
        try:
            request = json.loads(line)
        except json.JSONDecodeError:
            response = _error(None, -32700, "parse error")
        else:
            try:
                response = handle(request)
            except Exception as error:  # noqa: BLE001 - keep the process alive
                _diagnostic(error)
                response = _error(
                    request.get("id") if isinstance(request, dict) else None,
                    -32603,
                    "internal error",
                )
        if response is not None and not _write(stdout, response):
            return


def _excepthook(exc_type, exc, _traceback) -> None:  # pragma: no cover
    _diagnostic(exc)
    _write(sys.stdout, _error(None, -32603, "internal error"))


def main() -> None:
    sys.excepthook = _excepthook
    try:
        serve()
    except Exception as error:  # noqa: BLE001 - global MCP guard
        _diagnostic(error)
        _write(sys.stdout, _error(None, -32603, "internal error"))


if __name__ == "__main__":
    main()
