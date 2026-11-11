from __future__ import annotations

import json

import pytest

from jobspy_mcp_server.search import (
    SUPPORTED_SITES,
    JobSpyError,
    RateLimiter,
    search_jobs,
)
from jobspy_mcp_server.server import handle


def fake_scrape(**kwargs):
    site = kwargs["site_name"][0]
    return [
        {
            "id": f"{site}-1",
            "title": "Rust Engineer",
            "company": "Acme",
            "location": "Remote - Brazil",
            "is_remote": True,
            "job_url": f"https://{site}.example/1",
            "description": "<p>Rust</p>",
            "date_posted": "2026-11-08",
            "tags": "rust, backend",
        }
    ]


def test_searches_linkedin_and_indeed_with_normalized_output():
    calls = []

    def scrape(**kwargs):
        calls.append(kwargs)
        return fake_scrape(**kwargs)

    limiter = RateLimiter(min_interval_seconds=0, clock=lambda: 0.0, sleep=lambda _: None)
    payload = search_jobs(
        query="Rust Engineer",
        sites=["linkedin", "indeed"],
        limiter=limiter,
        scrape=scrape,
    )

    assert [call["site_name"][0] for call in calls] == ["linkedin", "indeed"]
    assert len(payload["jobs"]) == 2
    job = payload["jobs"][0]
    assert job["title"] == "Rust Engineer"
    assert job["company"] == "Acme"
    assert job["work_modality"] == "Remote"
    assert job["source_name"] == "jobspy"
    assert job["external_id"].startswith("linkedin-")
    assert job["tags"] == ["rust", "backend"]


def test_rate_limiter_paces_board_calls():
    clock_values = iter([0.0, 0.0, 0.4, 0.4])
    sleeps = []
    limiter = RateLimiter(
        min_interval_seconds=1.0,
        clock=lambda: next(clock_values),
        sleep=sleeps.append,
    )
    limiter.wait()
    limiter.wait()
    assert sleeps == [0.6]


def test_rejects_unknown_site():
    with pytest.raises(JobSpyError):
        search_jobs(query="Rust", sites=["unsupported"])
    assert "unsupported" not in str(JobSpyError("x"))


def test_source_failure_is_sanitized():
    def failing_scrape(**_kwargs):
        raise RuntimeError("secret internal detail")

    limiter = RateLimiter(min_interval_seconds=0, clock=lambda: 0.0, sleep=lambda _: None)
    with pytest.raises(JobSpyError) as error:
        search_jobs(
            query="Rust",
            limiter=limiter,
            scrape=failing_scrape,
            max_retries=1,
        )
    assert "secret internal detail" not in str(error.value)


def test_retries_are_bounded():
    calls = []

    def flaky_scrape(**kwargs):
        calls.append(kwargs["site_name"][0])
        if len(calls) < 3:
            raise RuntimeError("temporary")
        return fake_scrape(**kwargs)

    limiter = RateLimiter(min_interval_seconds=0, clock=lambda: 0.0, sleep=lambda _: None)
    payload = search_jobs(
        query="Rust",
        sites=["linkedin"],
        limiter=limiter,
        scrape=flaky_scrape,
        max_retries=2,
    )
    assert len(calls) == 3
    assert len(payload["jobs"]) == 1


def test_partial_site_failure_keeps_other_results():
    def scrape(**kwargs):
        if kwargs["site_name"][0] == "linkedin":
            raise RuntimeError("secret internal detail")
        return fake_scrape(**kwargs)

    limiter = RateLimiter(min_interval_seconds=0, clock=lambda: 0.0, sleep=lambda _: None)
    payload = search_jobs(
        query="Rust",
        sites=["linkedin", "indeed"],
        limiter=limiter,
        scrape=scrape,
        max_retries=0,
    )

    assert len(payload["jobs"]) == 1
    assert payload["jobs"][0]["source_name"] == "jobspy"
    assert payload["failures"][0]["site"] == "linkedin"
    assert "secret internal detail" not in payload["failures"][0]["message"]


def test_supported_sites_cover_all_jobspy_boards():
    assert {
        "linkedin",
        "indeed",
        "glassdoor",
        "zip_recruiter",
        "google",
        "bayt",
        "naukri",
        "bdjobs",
        "hellowork",
    } <= SUPPORTED_SITES


def test_exhausted_retries_return_a_sanitized_error():
    calls = []

    def failing_scrape(**_kwargs):
        calls.append("call")
        raise RuntimeError("secret internal detail")

    limiter = RateLimiter(min_interval_seconds=0, clock=lambda: 0.0, sleep=lambda _: None)
    with pytest.raises(JobSpyError) as error:
        search_jobs(
            query="Rust",
            sites=["linkedin"],
            limiter=limiter,
            scrape=failing_scrape,
            max_retries=1,
        )
    assert len(calls) == 2
    assert "secret internal detail" not in str(error.value)


def test_mcp_initialize_and_tools_call_round_trip():
    initialize = handle({"jsonrpc": "2.0", "id": 1, "method": "initialize"})
    assert initialize["result"]["serverInfo"]["name"] == "jobspy-mcp-server"

    called = handle(
        {
            "jsonrpc": "2.0",
            "id": "run-1",
            "method": "tools/call",
            "params": {
                "name": "search_jobs",
                "arguments": {"query": "Rust", "sites": ["linkedin"]},
            },
        },
        search=lambda **_kwargs: {"jobs": [{"title": "Rust"}]},
    )
    assert called["result"]["jobs"][0]["title"] == "Rust"
    assert json.dumps(called).find("secret") == -1


def test_mcp_rejects_missing_query_and_unknown_tool():
    missing = handle(
        {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {"name": "search_jobs", "arguments": {}},
        }
    )
    assert missing["error"]["code"] == -32602

    unknown = handle(
        {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {"name": "other", "arguments": {}},
        }
    )
    assert unknown["error"]["code"] == -32602
