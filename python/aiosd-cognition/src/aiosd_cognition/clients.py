"""Clients of aiosd-core (admin Unix socket), the Tool Broker (MCP) and the Model Gateway.

Wire formats follow the merged code: crates/aios-broker/src/{admin,mcp}.rs and
crates/aios-model-gateway/src/api.rs.
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass, field
from typing import Any

import httpx2
from mcp import Client, MCPError
from mcp.client.streamable_http import streamable_http_client

# Defaults follow docs/m0/README.md §4; every value can be overridden by environment.
DEFAULT_CORE_SOCKET = "/run/aios/core.sock"
DEFAULT_GATEWAY_URL = "http://127.0.0.1:8090/v1"
DEFAULT_AUDIT_FILE = "/var/lib/aios/audit/audit.jsonl"
DEFAULT_CONFIRM_CMD = "sudo -u aios-trusted aios-dev-confirm --key /etc/aios/dev-trusted-ui.key"
# systemd nests `aios-agent.slice` under `aios.slice` (the dash is a hierarchy separator).
DEFAULT_AGENT_CGROUP = (
    "/sys/fs/cgroup/user.slice/user-{uid}.slice/user@{uid}.service/aios.slice/aios-agent.slice"
)


@dataclass(frozen=True)
class Settings:
    """Runtime settings, read from `AIOS_*` environment variables."""

    core_socket: str = DEFAULT_CORE_SOCKET
    gateway_url: str = DEFAULT_GATEWAY_URL
    audit_file: str = DEFAULT_AUDIT_FILE
    audit_cmd: str = "aios-audit"
    confirm_cmd: str = DEFAULT_CONFIRM_CMD
    agent_cgroup: str = DEFAULT_AGENT_CGROUP
    l2_model: str = "s2"
    max_rounds: int = 8
    token_budget: int = 20_000

    @classmethod
    def from_env(cls, env: dict[str, str] | None = None) -> Settings:
        env = dict(os.environ if env is None else env)
        d = cls()
        return cls(
            core_socket=env.get("AIOS_CORE_SOCKET", d.core_socket),
            gateway_url=env.get("AIOS_GATEWAY_URL", d.gateway_url),
            audit_file=env.get("AIOS_AUDIT_FILE", d.audit_file),
            audit_cmd=env.get("AIOS_AUDIT_CMD", d.audit_cmd),
            confirm_cmd=env.get("AIOS_CONFIRM_CMD", d.confirm_cmd),
            agent_cgroup=env.get("AIOS_AGENT_CGROUP", d.agent_cgroup),
            l2_model=env.get("AIOS_L2_MODEL", d.l2_model),
            max_rounds=int(env.get("AIOS_L2_MAX_ROUNDS", d.max_rounds)),
            token_budget=int(env.get("AIOS_TASK_TOKEN_BUDGET", d.token_budget)),
        )


class ServiceError(Exception):
    """An AiOS service answered with an error or could not be reached."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(f"{code}: {message}")
        self.code = code
        self.message = message


@dataclass(frozen=True)
class SessionInfo:
    """Answer of `POST /v1/session` (aios_broker::SessionInfo)."""

    session_id: str
    broker_token: str = field(repr=False)
    gateway_key: str = field(repr=False)
    broker_url: str
    gateway_url: str


def _error_of(resp: httpx2.Response) -> ServiceError:
    try:
        body = resp.json()
    except ValueError:
        body = {}
    err = body.get("error") if isinstance(body, dict) else None
    if isinstance(err, dict):  # gateway: {"error": {"code", "message"}}
        return ServiceError(str(err.get("code", resp.status_code)), str(err.get("message", "")))
    if isinstance(err, str):  # aiosd-core: {"error", "detail"}
        return ServiceError(err, str(body.get("detail", "")))
    return ServiceError(f"http_{resp.status_code}", resp.text[:200])


async def _request(http: httpx2.AsyncClient, method: str, url: str, **kw: Any) -> Any:
    try:
        resp = await http.request(method, url, **kw)
    except httpx2.HTTPError as e:
        raise ServiceError("unavailable", f"{type(e).__name__}: {e}") from e
    if resp.status_code >= 400:
        raise _error_of(resp)
    return resp.json() if resp.content else None


class CoreClient:
    """aiosd-core admin API over its Unix socket; the uid comes from SO_PEERCRED."""

    def __init__(self, http: httpx2.AsyncClient) -> None:
        self.http = http

    @classmethod
    def connect(cls, socket_path: str) -> CoreClient:
        transport = httpx2.AsyncHTTPTransport(uds=socket_path)
        return cls(httpx2.AsyncClient(transport=transport, base_url="http://aiosd-core", timeout=10))

    async def session(self, channel: str = "local") -> SessionInfo:
        body = await _request(self.http, "POST", "/v1/session", json={"channel": channel})
        return SessionInfo(**{k: body[k] for k in SessionInfo.__dataclass_fields__})

    async def status(self) -> dict[str, Any]:
        return await _request(self.http, "GET", "/v1/status")

    async def pending(self) -> list[dict[str, Any]]:
        return await _request(self.http, "GET", "/v1/pending")

    async def confirm(self, request_id: str, token: str) -> None:
        await _request(self.http, "POST", "/v1/confirm", json={"request_id": request_id, "token": token})

    async def deny(self, request_id: str) -> None:
        await _request(self.http, "POST", "/v1/deny", json={"request_id": request_id})

    async def revoke_all(self) -> None:
        # Not served by aiosd-core at M0 (crates/aios-broker/src/admin.rs); callers report the 404.
        await _request(self.http, "POST", "/v1/revoke-all")

    async def aclose(self) -> None:
        await self.http.aclose()


@dataclass
class ToolResult:
    """A successful `tools/call` answer (the tool itself may still report `is_error`)."""

    text: str
    is_error: bool
    privacy: str | None
    provenance: str | None


class BrokerError(ServiceError):
    """A Broker JSON-RPC error; `code` is `error.data.error` (`denied`, `confirmation_*`, ...)."""


def content_text(content: list[Any]) -> str:
    """Flatten MCP content blocks to text for the model and the terminal."""
    parts = []
    for block in content:
        text = getattr(block, "text", None)
        parts.append(text if isinstance(text, str) else json.dumps(block.model_dump(mode="json")))
    return "\n".join(parts)


class BrokerClient:
    """Tool Broker `aios-tools` over an MCP client session."""

    def __init__(self, client: Client) -> None:
        self.client = client

    @staticmethod
    def http_for(session: SessionInfo) -> httpx2.AsyncClient:
        """HTTP client carrying the session's Bearer token; read timeout above the 120 s confirm wait."""
        return httpx2.AsyncClient(
            headers={"Authorization": f"Bearer {session.broker_token}"},
            timeout=httpx2.Timeout(30, read=300),
        )

    @staticmethod
    def mcp_for(session: SessionInfo, http: httpx2.AsyncClient) -> Client:
        """An MCP streamable HTTP client to the session's broker URL."""
        return Client(streamable_http_client(session.broker_url, http_client=http))

    async def list_tools(self) -> list[dict[str, Any]]:
        result = await self.client.list_tools()
        return [
            {"name": t.name, "description": t.description or "", "input_schema": t.input_schema}
            for t in result.tools
        ]

    async def call(self, name: str, args: dict[str, Any], idempotency_key: str) -> ToolResult:
        try:
            result = await self.client.call_tool(name, args, meta={"idempotency_key": idempotency_key})
        except MCPError as e:
            data = e.data if isinstance(e.data, dict) else {}
            code = str(data.get("error", f"mcp_{e.code}"))
            # The broker's message is "<code>: <detail>"; keep only the detail.
            raise BrokerError(code, e.message.removeprefix(f"{code}: ")) from e
        meta = result.meta or {}
        return ToolResult(
            text=content_text(result.content),
            is_error=bool(result.is_error),
            privacy=meta.get("privacy"),
            provenance=meta.get("provenance"),
        )


class GatewayClient:
    """Model Gateway, OpenAI-compatible HTTP (`stream: false` only in M0)."""

    def __init__(self, http: httpx2.AsyncClient) -> None:
        self.http = http

    @classmethod
    def connect(cls, base_url: str, key: str | None) -> GatewayClient:
        headers = {"Authorization": f"Bearer {key}"} if key else {}
        return cls(httpx2.AsyncClient(base_url=base_url.rstrip("/") + "/", headers=headers, timeout=300))

    async def chat(self, model: str, messages: list[dict[str, Any]], tools: list[dict[str, Any]]) -> dict:
        body: dict[str, Any] = {"model": model, "messages": messages, "stream": False}
        if tools:
            body["tools"] = tools
        return await _request(self.http, "POST", "chat/completions", json=body)

    async def models(self) -> list[str]:
        body = await _request(self.http, "GET", "models")
        return [m["id"] for m in body.get("data", [])]

    async def aclose(self) -> None:
        await self.http.aclose()
