"""aiosctl: the M0 command line of AiOS (docs/m0/cognition.md §Lệnh).

Every path, URL and command comes from `AIOS_*` environment variables (see `Settings`).
"""

from __future__ import annotations

import argparse
import asyncio
import json
import os
import shlex
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

from .clients import BrokerClient, CoreClient, GatewayClient, ServiceError, Settings
from .router import Router


def _err(msg: str) -> None:
    print(f"aiosctl: {msg}", file=sys.stderr)


def _print_step(step: dict[str, Any]) -> None:
    ev = step["event"]
    if ev == "tool_call":
        print(f"-> {step['tool']} {json.dumps(step['args'], ensure_ascii=False)}", file=sys.stderr)
    elif ev == "waiting_confirm":
        print(
            f"   waiting for your confirmation (request {step['request_id']}): run `aiosctl confirm`",
            file=sys.stderr,
        )
    elif ev == "tool_error":
        print(f"   error {step['error']}: {step['message']}", file=sys.stderr)
    elif ev == "tool_result":
        print(
            f"   {step['status']} (privacy={step['privacy']}, provenance={step['provenance']})",
            file=sys.stderr,
        )


async def cmd_ask(s: Settings, text: str, as_json: bool) -> int:
    core = CoreClient.connect(s.core_socket)
    try:
        session = await core.session()
    except ServiceError as e:
        _err(f"aiosd-core unavailable: {e}")
        await core.aclose()
        return 1
    gateway = GatewayClient.connect(session.gateway_url, session.gateway_key)
    try:
        async with BrokerClient.http_for(session) as http, BrokerClient.mcp_for(session, http) as mcp:
            router = Router(
                session_id=session.session_id,
                broker=BrokerClient(mcp),
                gateway=gateway,
                core=core,
                model=s.l2_model,
                max_rounds=s.max_rounds,
                token_budget=s.token_budget,
                on_step=(lambda _s: None) if as_json else _print_step,
            )
            out = await router.ask(text)
    except (OSError, ServiceError) as e:
        _err(f"broker unavailable: {e}")
        return 1
    finally:
        await gateway.aclose()
        await core.aclose()
    if as_json:
        print(json.dumps({"session": session.session_id, **out.to_json()}, ensure_ascii=False, indent=2))
    elif out.error:
        _err(f"[{out.route}] {out.error}")
    else:
        print(out.answer)
    return 1 if out.error else 0


async def cmd_status(s: Settings, as_json: bool) -> int:
    report: dict[str, Any] = {}
    core = CoreClient.connect(s.core_socket)
    key = None
    gateway_url = s.gateway_url
    try:
        report["core"] = {"ready": True, **await core.status()}
        # /v1/models needs a virtual key, which only aiosd-core hands out.
        session = await core.session()
        key, gateway_url = session.gateway_key, session.gateway_url
    except ServiceError as e:
        report.setdefault("core", {"ready": False, "error": str(e)})
        if report["core"]["ready"]:
            report["core"]["session_error"] = str(e)
    finally:
        await core.aclose()
    gw = GatewayClient.connect(gateway_url, key)
    try:
        report["gateway"] = {"ready": True, "url": gateway_url, "models": await gw.models()}
    except ServiceError as e:
        report["gateway"] = {"ready": False, "url": gateway_url, "error": str(e)}
    finally:
        await gw.aclose()
    if as_json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
        return 0
    c, g = report["core"], report["gateway"]
    if c["ready"]:
        print(
            f"aiosd-core: ready (version {c.get('version')}, {c.get('tools')} tools, "
            f"policy {c.get('policy_version')})"
        )
    else:
        print(f"aiosd-core: unavailable ({c['error']})")
    if g["ready"]:
        print(f"gateway: ready at {g['url']} (models: {', '.join(g['models'])})")
    else:
        print(f"gateway: unavailable at {g['url']} ({g['error']})")
    return 0


def _show_pending(items: list[dict[str, Any]]) -> None:
    for n, p in enumerate(items, 1):
        steps = ", ".join(st.get("tool", "?") for st in p.get("steps", []))
        print(f"{n}. {p['request_id']}  {steps}  ({p.get('principal')}, step {p.get('task_step')})")


def _run_signer(s: Settings, pending: dict[str, Any]) -> tuple[int, str]:
    """Run the dev signer on our TTY; it prints the token on stdout. Returns (exit code, token)."""
    # The signer runs as another user (aios-trusted) and reads the request from a file, since its
    # stdin must be the TTY: an unlisted directory (0711) holding a 0644 file.
    tmp = Path(tempfile.mkdtemp(prefix="aiosctl-confirm-"))
    try:
        tmp.chmod(0o711)
        path = tmp / "pending.json"
        path.write_text(json.dumps(pending), encoding="utf-8")
        path.chmod(0o644)
        cmd = [*shlex.split(s.confirm_cmd), "--pending", str(path)]
        proc = subprocess.run(cmd, check=False, stdout=subprocess.PIPE, text=True)
        return proc.returncode, proc.stdout.strip()
    finally:
        for f in tmp.iterdir():
            f.unlink()
        tmp.rmdir()


async def cmd_confirm(s: Settings, list_only: bool, request_id: str | None) -> int:
    core = CoreClient.connect(s.core_socket)
    try:
        items = await core.pending()
        if list_only:
            if items:
                _show_pending(items)
            else:
                print("no pending confirmation requests")
            return 0
        if request_id:
            items = [p for p in items if p["request_id"] == request_id]
        if not items:
            print("no pending confirmation requests")
            return 1 if request_id else 0
        chosen = items[0]
        if len(items) > 1:
            if not sys.stdin.isatty():
                _err("several requests are pending; pass --id REQUEST_ID")
                return 2
            _show_pending(items)
            choice = input("Open which request? [1] ").strip() or "1"
            if not choice.isdigit() or not 1 <= int(choice) <= len(items):
                _err("no such request")
                return 2
            chosen = items[int(choice) - 1]
        try:
            code, token = await asyncio.to_thread(_run_signer, s, chosen)
        except OSError as e:
            _err(f"cannot run the confirmation signer: {e}")
            return 2
        if code == 0 and token:
            await core.confirm(chosen["request_id"], token)
            print(f"confirmed {chosen['request_id']}")
            return 0
        if code == 1:
            await core.deny(chosen["request_id"])
            print(f"denied {chosen['request_id']}")
            return 1
        _err(f"confirmation signer failed (exit {code}); request left pending")
        return 2
    except ServiceError as e:
        _err(str(e))
        return 1
    finally:
        await core.aclose()


def cmd_audit_verify(s: Settings, file: str | None) -> int:
    try:
        return subprocess.run(
            [*shlex.split(s.audit_cmd), "verify", file or s.audit_file], check=False
        ).returncode
    except OSError as e:
        _err(f"cannot run {s.audit_cmd}: {e}")
        return 2


async def _revoke_all(s: Settings) -> None:
    core = CoreClient.connect(s.core_socket)
    try:
        await core.revoke_all()
        print("gateway keys revoked")
    except ServiceError as e:
        why = "aiosd-core has no /v1/revoke-all endpoint" if e.code == "http_404" else str(e)
        _err(f"gateway keys NOT revoked via aiosd-core: {why}")
    finally:
        await core.aclose()


def cmd_freeze(s: Settings, value: str) -> int:
    path = Path(s.agent_cgroup.format(uid=os.getuid())) / "cgroup.freeze"
    if value == "0":
        # M0 stand-in for authentication on the trusted surface (design.md §11.3, M1).
        if not sys.stdin.isatty():
            _err("unfreeze needs an interactive terminal")
            return 2
        if input("Unfreeze the agent slice? [y/N] ").strip().lower() != "y":
            print("not unfrozen")
            return 1
    # The freeze itself never depends on aiosd-core (REQ-D-11.3-01).
    try:
        path.write_text(value, encoding="ascii")
    except OSError as e:
        _err(f"cannot write {path}: {e}")
        return 1
    print(f"{'frozen' if value == '1' else 'unfrozen'}: {path}")
    if value == "1":
        asyncio.run(_revoke_all(s))
    return 0


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(prog="aiosctl", description="AiOS command line (M0)")
    sub = p.add_subparsers(dest="cmd", required=True)
    a = sub.add_parser("ask", help="route a request through the L0 grammar or the L2 loop")
    a.add_argument("text")
    a.add_argument("--json", action="store_true")
    c = sub.add_parser("confirm", help="answer a pending confirmation request")
    c.add_argument("--list", action="store_true")
    c.add_argument("--id", dest="request_id")
    st = sub.add_parser("status", help="aiosd-core and Model Gateway status")
    st.add_argument("--json", action="store_true")
    au = sub.add_parser("audit", help="audit log tools")
    au_sub = au.add_subparsers(dest="audit_cmd", required=True)
    av = au_sub.add_parser("verify", help="verify the audit hash chain")
    av.add_argument("--file")
    sub.add_parser("freeze", help="freeze the agent slice, then revoke gateway keys")
    sub.add_parser("unfreeze", help="unfreeze the agent slice")
    args = p.parse_args(argv)
    s = Settings.from_env()
    match args.cmd:
        case "ask":
            return asyncio.run(cmd_ask(s, args.text, args.json))
        case "confirm":
            return asyncio.run(cmd_confirm(s, args.list, args.request_id))
        case "status":
            return asyncio.run(cmd_status(s, args.json))
        case "audit":
            return cmd_audit_verify(s, args.file)
        case "freeze":
            return cmd_freeze(s, "1")
        case "unfreeze":
            return cmd_freeze(s, "0")
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
