"""JobSpy search wrapper with board limits and normalized output.

The sidecar returns normalized fields only. Matching and scoring stay in Rust.
"""

from __future__ import annotations

import hashlib
import math
import time
from dataclasses import dataclass, field
from typing import Any, Callable, Iterable, Mapping

DEFAULT_SITES = ("linkedin", "indeed")
SUPPORTED_SITES = frozenset(
    {
        "linkedin",
        "indeed",
        "glassdoor",
        "zip_recruiter",
        "google",
        "bayt",
        "naukri",
        "bdjobs",
        "hellowork",
    }
)


class JobSpyError(RuntimeError):
    """A sanitized source error safe to return over MCP."""


@dataclass
class RateLimiter:
    """Minimum interval between board calls, injectable for tests."""

    min_interval_seconds: float = 1.0
    clock: Callable[[], float] = time.monotonic
    sleep: Callable[[float], None] = time.sleep
    _last: float | None = field(default=None, init=False, repr=False)

    def wait(self) -> None:
        now = self.clock()
        if self._last is not None:
            delay = self.min_interval_seconds - (now - self._last)
            if delay > 0:
                self.sleep(delay)
        self._last = self.clock()


def _scrape_jobs(**kwargs: Any) -> Any:
    """Import JobSpy lazily so tests and packaging do not need a browser."""

    try:
        from importlib import import_module

        scrape_jobs = import_module("jobspy").scrape_jobs
    except Exception as error:  # pragma: no cover - depends on optional install
        raise JobSpyError("JobSpy is not installed in the sidecar runtime") from error

    return scrape_jobs(**kwargs)


def _records(result: Any) -> Iterable[Mapping[str, Any]]:
    if result is None:
        return ()
    if hasattr(result, "to_dict"):
        return result.to_dict(orient="records")
    if isinstance(result, Mapping):
        return (result,)
    return result


def _clean(value: Any) -> Any:
    if value is None:
        return None
    if isinstance(value, float) and math.isnan(value):
        return None
    if isinstance(value, str):
        trimmed = value.strip()
        return trimmed or None
    return value


def _external_id(site: str, row: Mapping[str, Any], url: str) -> str:
    raw = row.get("id") or row.get("job_id") or row.get("external_id")
    if raw is not None and str(raw).strip():
        return f"{site}-{str(raw).strip()}"
    digest = hashlib.sha256(url.encode("utf-8")).hexdigest()
    return f"{site}-{digest}"


def _normalize_row(site: str, row: Mapping[str, Any]) -> dict[str, Any] | None:
    title = _clean(row.get("title") or row.get("position"))
    url = _clean(row.get("job_url") or row.get("url") or row.get("apply_url"))
    if not title or not url:
        return None

    location = _clean(row.get("location")) or "Unknown"
    is_remote = row.get("is_remote")
    if is_remote is True or str(location).lower().startswith("remote"):
        modality = "Remote"
    elif "hybrid" in str(location).lower():
        modality = "Hybrid"
    elif "on-site" in str(location).lower() or "onsite" in str(location).lower():
        modality = "Onsite"
    else:
        modality = "Unknown"

    posted_at = _clean(row.get("date_posted") or row.get("posted_at"))
    if hasattr(posted_at, "isoformat"):
        posted_at = posted_at.isoformat()
    elif posted_at is not None:
        posted_at = str(posted_at)

    description = row.get("description") or row.get("job_description") or ""
    tags = row.get("tags") or []
    if isinstance(tags, str):
        tags = [item.strip() for item in tags.split(",") if item.strip()]

    return {
        "title": str(title),
        "company": str(_clean(row.get("company")) or "Unknown"),
        "location": str(location),
        "work_modality": modality,
        "url": str(url),
        "posted_at": posted_at,
        "description": str(description),
        "source_name": "jobspy",
        "external_id": _external_id(site, row, str(url)),
        "tags": list(tags),
    }


def search_jobs(
    *,
    query: str,
    location: str = "Brazil",
    sites: Iterable[str] = DEFAULT_SITES,
    results_wanted: int = 50,
    hours_old: int = 168,
    is_remote: bool = True,
    offset: int = 0,
    max_retries: int = 2,
    backoff_seconds: float = 0.5,
    scrape: Callable[..., Any] | None = None,
    limiter: RateLimiter | None = None,
) -> dict[str, Any]:
    """Search each requested board and return normalized postings."""

    requested = [str(site).strip().lower() for site in sites if str(site).strip()]
    if not requested:
        requested = list(DEFAULT_SITES)
    unknown = sorted(set(requested) - SUPPORTED_SITES)
    if unknown:
        raise JobSpyError(f"unsupported JobSpy site: {', '.join(unknown)}")

    scrape_fn = scrape or _scrape_jobs
    limiter = limiter or RateLimiter()
    results_wanted = max(1, min(int(results_wanted), 200))
    retries = max(0, min(int(max_retries), 5))
    jobs: list[dict[str, Any]] = []
    failures: list[dict[str, str]] = []

    for site in requested:
        result = None
        site_error: JobSpyError | None = None
        for attempt in range(retries + 1):
            limiter.wait()
            try:
                result = scrape_fn(
                    site_name=[site],
                    search_term=query,
                    location=location,
                    results_wanted=results_wanted,
                    hours_old=max(1, int(hours_old)),
                    is_remote=bool(is_remote),
                    offset=max(0, int(offset)),
                )
                break
            except JobSpyError as error:
                site_error = error
                break
            except Exception as error:  # sanitize provider internals
                if attempt >= retries:
                    site_error = JobSpyError(f"JobSpy {site} search failed")
                    break
                limiter.sleep(backoff_seconds * (attempt + 1))

        if site_error is not None:
            failures.append({"site": site, "message": str(site_error)})
            continue

        for row in _records(result):
            normalized = _normalize_row(site, row)
            if normalized is not None:
                jobs.append(normalized)

    # Stable dedup within one sidecar response by source external id.
    seen: set[str] = set()
    unique: list[dict[str, Any]] = []
    for job in jobs:
        key = f"{job['source_name']}:{job['external_id']}"
        if key not in seen:
            seen.add(key)
            unique.append(job)

    if failures and not unique:
        raise JobSpyError(failures[0]["message"])

    payload: dict[str, Any] = {"jobs": unique}
    if failures:
        payload["failures"] = failures
    return payload
