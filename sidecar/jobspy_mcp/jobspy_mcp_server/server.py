"""Line-delimited JSON-RPC MCP server over stdio.

stdout carries protocol messages only; diagnostics must go to stderr.
"""

from __future__ import annotations

import json
import sys
from typing import Any, Callable, TextIO

from . import __version__
from .search import JobSpyError, search_jobs

PROTOCOL_VERSION = "2024-11-05"

TOOLS = [
    {
        "name": "search_jobs",
        "description": "Search LinkedIn and Indeed Brazil through JobSpy.",
        "inputSchema": {
            "type": "object",
            "additionalProperties": False,
            "required": ["query"],
            "properties": {
                "query": {"type": "string", "minLength": 1, "maxLength": 512},
                "location": {"type": "string", "maxLength": 256},
                "sites": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": [
                            "linkedin",
                            "indeed",
                            "glassdoor",
                            "zip_recruiter",
                            "google",
                            "bayt",
                            "naukri",
                            "bdjobs",
                            "hellowork",
                        ],
                    },
                },
                "results_wanted": {"type": "integer", "minimum": 1, "maximum": 200},
                "hours_old": {"type": "integer", "minimum": 1, "maximum": 720},
                "is_remote": {"type": "boolean"},
                "job_type": {"type": ["string", "null"]},
                "offset": {"type": "integer", "minimum": 0},
            },
        },
    }
]


def _error(request_id: Any, code: int, message: str) -> dict[str, Any]:
    return {
        "jsonrpc": "2.0",
        "id": request_id,
        "error": {"code": code, "message": message},
    }


def _result(request_id: Any, result: Any) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": request_id, "result": result}


def handle(
    request: dict[str, Any],
    search: Callable[..., dict[str, Any]] = search_jobs,
) -> dict[str, Any] | None:
    """Handle one JSON-RPC message. Notifications return None."""

    method = request.get("method")
    request_id = request.get("id")

    if method == "initialize":
        return _result(
            request_id,
            {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "jobspy-mcp-server", "version": __version__},
            },
        )

    if method in {"notifications/initialized", "notifications/cancelled"}:
        return None

    if method == "ping":
        return _result(request_id, {})

    if method == "tools/list":
        return _result(request_id, {"tools": TOOLS})

    if method == "tools/call":
        params = request.get("params") or {}
        name = params.get("name")
        if name != "search_jobs":
            return _error(request_id, -32602, "unknown tool")
        arguments = params.get("arguments") or {}
        query = str(arguments.get("query") or "").strip()
        if not query:
            return _error(request_id, -32602, "query is required")

        try:
            payload = search(
                query=query,
                location=str(arguments.get("location") or "Brazil"),
                sites=arguments.get("sites") or ["linkedin", "indeed"],
                results_wanted=int(arguments.get("results_wanted") or 50),
                hours_old=int(arguments.get("hours_old") or 168),
                is_remote=bool(arguments.get("is_remote", True)),
                offset=int(arguments.get("offset") or 0),
            )
        except JobSpyError as error:
            return _error(request_id, -32000, str(error))
        except Exception:
            return _error(request_id, -32000, "JobSpy search failed")

        return _result(request_id, payload)

    return _error(request_id, -32601, "method not found")


def serve(
    stdin: TextIO = sys.stdin,
    stdout: TextIO = sys.stdout,
    search: Callable[..., dict[str, Any]] = search_jobs,
) -> None:
    for line in stdin:
        line = line.strip()
        if not line:
            continue
        try:
            request = json.loads(line)
        except json.JSONDecodeError:
            response = _error(None, -32700, "parse error")
        else:
            response = handle(request, search=search)
        if response is not None:
            stdout.write(json.dumps(response, separators=(",", ":")) + "\n")
            stdout.flush()


def main() -> None:
    serve()


if __name__ == "__main__":
    main()
