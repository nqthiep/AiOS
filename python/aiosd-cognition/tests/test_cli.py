import json
import socket
import stat
import sys
from pathlib import Path

import httpx2
import pytest

from aiosd_cognition import cli
from aiosd_cognition.clients import CoreClient


def closed_port() -> int:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


@pytest.fixture
def env(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    monkeypatch.setenv("AIOS_CORE_SOCKET", str(tmp_path / "core.sock"))  # nothing listens
    monkeypatch.setenv("AIOS_GATEWAY_URL", f"http://127.0.0.1:{closed_port()}/v1")
    monkeypatch.setenv("AIOS_AGENT_CGROUP", str(tmp_path / "user-{uid}" / "aios-agent.slice"))
    return tmp_path


def script(path: Path, body: str) -> str:
    path.write_text(f"#!{sys.executable}\n{body}")
    path.chmod(path.stat().st_mode | stat.S_IEXEC)
    return str(path)


def test_status_runs_with_core_and_gateway_down(env: Path, capsys: pytest.CaptureFixture) -> None:
    assert cli.main(["status"]) == 0
    out = capsys.readouterr().out
    assert "aiosd-core: unavailable" in out and "gateway: unavailable" in out


def test_freeze_without_core(env: Path, capsys: pytest.CaptureFixture) -> None:
    import os

    slice_dir = env / f"user-{os.getuid()}" / "aios-agent.slice"
    slice_dir.mkdir(parents=True)
    assert cli.main(["freeze"]) == 0
    assert (slice_dir / "cgroup.freeze").read_text() == "1"
    assert "NOT revoked" in capsys.readouterr().err


def test_unfreeze_refuses_without_tty(env: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    import os

    slice_dir = env / f"user-{os.getuid()}" / "aios-agent.slice"
    slice_dir.mkdir(parents=True)
    (slice_dir / "cgroup.freeze").write_text("1")
    monkeypatch.setattr(sys.stdin, "isatty", lambda: False)
    assert cli.main(["unfreeze"]) == 2
    assert (slice_dir / "cgroup.freeze").read_text() == "1"


def test_ask_reports_core_down(env: Path, capsys: pytest.CaptureFixture) -> None:
    assert cli.main(["ask", "trạng thái máy"]) == 1
    assert "aiosd-core unavailable" in capsys.readouterr().err


def test_audit_verify_runs_tool(env: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    log = env / "args.json"
    cmd = script(
        env / "fake-audit", f"import json,sys; open({str(log)!r},'w').write(json.dumps(sys.argv[1:]))"
    )
    monkeypatch.setenv("AIOS_AUDIT_CMD", cmd)
    monkeypatch.setenv("AIOS_AUDIT_FILE", "/x/audit.jsonl")
    assert cli.main(["audit", "verify"]) == 0
    assert json.loads(log.read_text()) == ["verify", "/x/audit.jsonl"]


PENDING = {"request_id": "r1", "principal": "uid:1/session:s", "task_step": "s:1:0", "steps": []}


def fake_core(monkeypatch: pytest.MonkeyPatch) -> list:
    posted = []

    def handler(req: httpx2.Request) -> httpx2.Response:
        if req.url.path == "/v1/pending":
            return httpx2.Response(200, json=[PENDING])
        posted.append((req.url.path, json.loads(req.content)))
        return httpx2.Response(200, json={"ok": True})

    http = httpx2.AsyncClient(transport=httpx2.MockTransport(handler), base_url="http://core")
    monkeypatch.setattr(CoreClient, "connect", classmethod(lambda cls, _p: cls(http)))
    return posted


def test_confirm_signs_and_posts_token(env: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    posted = fake_core(monkeypatch)
    # The signer gets `--pending <file>`, checks the request, prints the token.
    signer = script(
        env / "signer",
        "import json,sys\np=json.load(open(sys.argv[sys.argv.index('--pending')+1]))\n"
        "assert sys.argv[1] == '--key'\nprint('token-for-' + p['request_id'])",
    )
    monkeypatch.setenv("AIOS_CONFIRM_CMD", f"{signer} --key k")
    assert cli.main(["confirm"]) == 0
    assert posted == [("/v1/confirm", {"request_id": "r1", "token": "token-for-r1"})]


def test_confirm_declined_denies(env: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    posted = fake_core(monkeypatch)
    monkeypatch.setenv("AIOS_CONFIRM_CMD", script(env / "signer", "raise SystemExit(1)"))
    assert cli.main(["confirm"]) == 1
    assert posted == [("/v1/deny", {"request_id": "r1"})]
