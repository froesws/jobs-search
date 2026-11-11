"""Create the Playwright storage state used by the LinkedIn scraper MCP.

Run this once from the wrapper environment:

    uv run linkedin-scraper-login

or, to use an existing ``li_at`` cookie without opening a login browser:

    uv run linkedin-scraper-login --cookie

The resulting session file contains LinkedIn cookies. Treat it like a
password and keep it outside the repository with mode 0600.
"""

from __future__ import annotations

import argparse
import asyncio
import getpass
import os
from pathlib import Path

DEFAULT_SESSION_FILE = Path("~/.linkedin-mcp/joeyism-session.json").expanduser()
SESSION_ENV = "LINKEDIN_SESSION_FILE"


async def _login(session_file: Path, cookie: str | None, timeout: int) -> None:
    from linkedin_scraper import BrowserManager, login_with_cookie, wait_for_manual_login

    async with BrowserManager(headless=False) as browser:
        if cookie:
            await login_with_cookie(browser.page, cookie)
        else:
            await browser.page.goto(
                "https://www.linkedin.com/login",
                wait_until="domcontentloaded",
            )
            await wait_for_manual_login(browser.page, timeout=timeout)
        await browser.save_session(str(session_file))


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Create a LinkedIn Playwright session for the scraper MCP."
    )
    parser.add_argument(
        "--session-file",
        default=os.environ.get(SESSION_ENV, str(DEFAULT_SESSION_FILE)),
        help="Where to save the Playwright storage state.",
    )
    parser.add_argument(
        "--cookie",
        action="store_true",
        help="Prompt for the li_at cookie instead of opening a login browser.",
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=300_000,
        help="Manual login timeout in milliseconds (default: 300000).",
    )
    args = parser.parse_args()

    session_file = Path(args.session_file).expanduser()
    cookie = None
    if args.cookie:
        cookie = getpass.getpass("li_at cookie (input hidden): ").strip()
        if not cookie:
            raise SystemExit("No cookie provided.")

    asyncio.run(_login(session_file, cookie, args.timeout))

    if os.name != "nt":
        os.chmod(session_file, 0o600)
    print(f"Session saved to {session_file}")
    print("Treat this file like a password; it contains LinkedIn cookies.")


if __name__ == "__main__":
    main()
