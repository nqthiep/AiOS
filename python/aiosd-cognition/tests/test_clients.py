import asyncio
import json

import httpx2
import pytest
from mcp import MCPError
from mcp_types import CallToolResult, ListToolsResult

from aiosd_cognition.clients import BrokerClient, BrokerError, CoreClient, GatewayClient, ServiceError

SESSION = {
    "session_id": "0190-s",
    "broker_token": "tok",
    "gateway_key": "aios-sk-x",
    "broker_url": "http://127.0.0.1:8091/mcp",
    "gateway_url": "http://127.0.0.1:8090/v1",
}


def mock(handler, base: str = "http://test") -> httpx2.AsyncClient:
    return httpx2.AsyncClient(transport=httpx2.MockTransport(handler), base_url=base)


def test_core_session_and_errors() -> None:
    seen = []

    def handler(req: httpx2.Request) -> httpx2.Response:
        seen.append((req.method, req.url.path, req.content))
        if req.url.path == "/v1/session":
            return httpx2.Response(200, json=SESSION)
        return httpx2.Response(404, json={"error": "not_found", "detail": "no such pending request"})

    async def go():
        core = CoreClient(mock(handler))
        info = await core.session()
        with pytest.raises(ServiceError) as e:
            await core.confirm("r1", "t")
        return info, e.value

    info, err = asyncio.run(go())
    assert info.session_id == "0190-s" and info.gateway_key == "aios-sk-x"
    assert "tok" not in repr(info)
    assert seen[0] == ("POST", "/v1/session", b'{"channel":"local"}')
    assert json.loads(seen[1][2]) == {"request_id": "r1", "token": "t"}
    assert (err.code, err.message) == ("not_found", "no such pending request")


def test_core_unreachable() -> None:
    async def go():
        with pytest.raises(ServiceError) as e:
            await CoreClient.connect("/nonexistent/core.sock").status()
        return e.value

    assert asyncio.run(go()).code == "unavailable"


def test_gateway_chat_models_and_error() -> None:
    def handler(req: httpx2.Request) -> httpx2.Response:
        assert req.headers["authorization"] == "Bearer k"
        if req.url.path == "/v1/models":
            return httpx2.Response(200, json={"object": "list", "data": [{"id": "s1"}, {"id": "s2"}]})
        body = json.loads(req.content)
        assert body["stream"] is False and body["model"] == "s2" and body["tools"]
        return httpx2.Response(409, json={"error": {"code": "no_route_privacy", "message": "secret"}})

    async def go():
        http = mock(handler, "http://test/v1/")
        http.headers["Authorization"] = "Bearer k"
        gw = GatewayClient(http)
        models = await gw.models()
        with pytest.raises(ServiceError) as e:
            await gw.chat("s2", [{"role": "user", "content": "x"}], [{"type": "function"}])
        return models, e.value

    models, err = asyncio.run(go())
    assert models == ["s1", "s2"] and err.code == "no_route_privacy"


class FakeMcp:
    def __init__(self) -> None:
        self.meta = None

    async def list_tools(self):
        return ListToolsResult.model_validate(
            {"tools": [{"name": "fs.search", "description": "d", "inputSchema": {"type": "object"}}]}
        )

    async def call_tool(self, name, args, meta=None):
        self.meta = meta
        if name == "fs.write_note":
            raise MCPError(-32012, "confirmation_timeout: no confirmation", {"error": "confirmation_timeout"})
        return CallToolResult.model_validate(
            {
                "content": [{"type": "text", "text": "hit"}],
                "isError": False,
                "_meta": {"privacy": "personal", "provenance": "external"},
            }
        )


def test_broker_client_meta_and_error_codes() -> None:
    fake = FakeMcp()
    broker = BrokerClient(fake)

    async def go():
        tools = await broker.list_tools()
        res = await broker.call("fs.search", {"pattern": "*"}, "s:1:0")
        meta = fake.meta
        with pytest.raises(BrokerError) as e:
            await broker.call("fs.write_note", {}, "s:1:1")
        return tools, res, meta, e.value

    tools, res, meta, err = asyncio.run(go())
    assert tools == [{"name": "fs.search", "description": "d", "input_schema": {"type": "object"}}]
    assert meta == {"idempotency_key": "s:1:0"}
    assert (res.text, res.privacy, res.provenance) == ("hit", "personal", "external")
    assert (err.code, err.message) == ("confirmation_timeout", "no confirmation")
