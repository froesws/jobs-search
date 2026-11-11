from __future__ import annotations

import io
import json
import os

from linkedin_scraper_mcp import server


def _call(name: str, arguments: dict | None = None, request_id: str = "1") -> dict:
    return {
        "jsonrpc": "2.0",
        "id": request_id,
        "method": "tools/call",
        "params": {"name": name, "arguments": arguments or {}},
    }


def _payload(response: dict) -> dict:
    return json.loads(response["result"]["content"][0]["text"])


def test_initialize_and_tools_list():
    response = server.handle({"jsonrpc": "2.0", "id": 1, "method": "initialize"})
    assert response["result"]["protocolVersion"] == server.PROTOCOL_VERSION
    assert response["result"]["serverInfo"]["name"] == "linkedin-scraper-mcp"

    response = server.handle({"jsonrpc": "2.0", "id": 2, "method": "tools/list"})
    names = {tool["name"] for tool in response["result"]["tools"]}
    assert {"search_jobs", "get_job_details", "get_session_status"} <= names


def test_search_jobs_serializes_results(monkeypatch):
    async def fake_search(keywords, location, limit, fetch_details):
        assert keywords == "Rust"
        assert location == "Brazil"
        assert limit == 5
        assert fetch_details is True
        return {
            "source": server.SOURCE,
            "count": 1,
            "jobs": [
                {
                    "url": "https://www.linkedin.com/jobs/view/1",
                    "title": "Rust Engineer",
                }
            ],
        }

    monkeypatch.setattr(server, "_run_search", fake_search)
    response = server.handle(
        _call("search_jobs", {"keywords": "Rust", "location": "Brazil", "limit": 5})
    )
    payload = _payload(response)
    assert payload["count"] == 1
    assert payload["jobs"][0]["title"] == "Rust Engineer"


def test_search_jobs_accepts_jobspy_style_arguments(monkeypatch):
    async def fake_search(keywords, location, limit, fetch_details):
        assert keywords == "Rust Engineer"
        assert limit == 10
        assert fetch_details is True
        return {
            "jobs": [
                {
                    "title": "Rust Engineer",
                    "company": "Acme",
                    "location": "Brazil",
                    "url": "https://www.linkedin.com/jobs/view/1",
                    "description": "Rust",
                    "source_name": "linkedin",
                }
            ]
        }

    monkeypatch.setattr(server, "_run_search", fake_search)
    response = server.handle(
        _call(
            "search_jobs",
            {
                "query": "Rust Engineer",
                "results_wanted": 10,
                "sites": ["linkedin"],
                "hours_old": 168,
                "is_remote": True,
            },
        )
    )
    assert _payload(response)["jobs"][0]["title"] == "Rust Engineer"


def test_app_job_maps_jobspy_compatible_fields():
    job = server._app_job(
        "https://www.linkedin.com/jobs/view/123/",
        {
            "job_title": "Rust Engineer",
            "company": "Acme",
            "location": "Porto Alegre, Brasil",
            "job_description": "Rust and Linux. Presencial.",
        },
    )
    assert job is not None
    assert job["title"] == "Rust Engineer"
    assert job["company"] == "Acme"
    assert job["location"] == "Porto Alegre, Brasil"
    assert job["work_modality"] == "Onsite"
    assert job["external_id"] == "linkedin-123"
    assert job["source_name"] == "linkedin"


def test_get_job_details_serializes_results(monkeypatch):
    async def fake_details(url):
        return {
            "source": server.SOURCE,
            "job": {"linkedin_url": url, "job_title": "Rust Engineer"},
        }

    monkeypatch.setattr(server, "_run_details", fake_details)
    response = server.handle(
        _call("get_job_details", {"url": "https://www.linkedin.com/jobs/view/1"})
    )
    assert _payload(response)["job"]["job_title"] == "Rust Engineer"


def test_session_status_reports_presence_and_permissions(tmp_path, monkeypatch):
    session = tmp_path / "session.json"
    session.write_text("{}")
    os.chmod(session, 0o600)
    monkeypatch.setenv(server.SESSION_ENV, str(session))

    response = server.handle(_call("get_session_status"))
    payload = _payload(response)
    assert payload["session_file_present"] is True
    assert payload["permissions_ok"] is True


def test_missing_session_is_a_tool_error(monkeypatch):
    async def fake_search(*_args, **_kwargs):
        raise server.SessionMissing("LinkedIn session is missing")

    monkeypatch.setattr(server, "_run_search", fake_search)
    response = server.handle(_call("search_jobs", {"keywords": "Rust"}))
    assert response["error"]["code"] == -32001
    assert "session" in response["error"]["message"].lower()


def test_unknown_tool_and_missing_keywords():
    response = server.handle(_call("other", {}))
    assert response["error"]["code"] == -32602

    response = server.handle(_call("search_jobs", {}))
    assert response["error"]["code"] == -32602


def test_invalid_request_shape_and_serve_continues():
    assert server.handle([])["error"]["code"] == -32600

    stdin = io.StringIO('[]\n{"jsonrpc":"2.0","id":2,"method":"ping"}\n')
    stdout = io.StringIO()
    server.serve(stdin=stdin, stdout=stdout)
    responses = [json.loads(line) for line in stdout.getvalue().splitlines()]
    assert responses[0]["error"]["code"] == -32600
    assert responses[1]["result"] == {}
