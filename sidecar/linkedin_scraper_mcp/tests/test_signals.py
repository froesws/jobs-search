from __future__ import annotations

import json
import os
import signal
import subprocess
import sys
from pathlib import Path

import pytest

MODULE = "linkedin_scraper_mcp.server"
PROJECT_ROOT = Path(__file__).resolve().parents[1]


def _start() -> subprocess.Popen[str]:
    env = os.environ.copy()
    env["PYTHONPATH"] = str(PROJECT_ROOT)
    return subprocess.Popen(
        [sys.executable, "-m", MODULE],
        cwd=PROJECT_ROOT,
        env=env,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
    )


@pytest.mark.parametrize("signum", [signal.SIGTERM, signal.SIGINT])
def test_shutdown_signal_exits_zero(signum: int) -> None:
    process = _start()
    try:
        assert process.stdin is not None
        process.stdin.write(
            json.dumps(
                {
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {},
                        "clientInfo": {"name": "test", "version": "0"},
                    },
                }
            )
            + "\n"
        )
        process.stdin.flush()
        assert process.stdout is not None
        initialize = process.stdout.readline()
        assert json.loads(initialize)["result"]["serverInfo"]["name"] == (
            "linkedin-scraper-mcp"
        )

        process.send_signal(signum)
        assert process.wait(timeout=10) == 0

        assert process.stdout is not None
        remainder = process.stdout.read().strip()
        for line in remainder.splitlines():
            json.loads(line)

        assert process.stderr is not None
        stderr = process.stderr.read()
        assert "Traceback" not in stderr
        assert signal.Signals(signum).name in stderr
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=10)
