"""Fake Broker, Gateway and aiosd-core for router tests (no network)."""

from __future__ import annotations

import asyncio
import json
from collections.abc import Callable
from typing import Any

from aiosd_cognition.clients import BrokerError, ServiceError, ToolResult

TOOLS = [
    {"name": "sys.info", "description": "System info", "input_schema": {"type": "object"}},
    {"name": "fs.search", "description": "Search files", "input_schema": {"type": "object"}},
    {"name": "fs.write_note", "description": "Write a note", "input_schema": {"type": "object"}},
]

Handler = Callable[[dict[str, Any], str], Any]


class FakeBroker:
    def __init__(self, handlers: dict[str, Handler] | None = None) -> None:
        self.handlers = handlers or {}
        self.calls: list[tuple[str, dict[str, Any], str]] = []

    async def list_tools(self) -> list[dict[str, Any]]:
        return TOOLS

    async def call(self, name: str, args: dict[str, Any], key: str) -> ToolResult:
        self.calls.append((name, args, key))
        handler = self.handlers.get(name)
        if handler is None:
            return ToolResult(text=f"{name} ok", is_error=False, privacy="personal", provenance="observed")
        out = handler(args, key)
        return await out if asyncio.iscoroutine(out) else out


class NoGateway:
    """A gateway that must not be called."""

    async def chat(self, *a: Any) -> dict:
        raise AssertionError("gateway must not be called")


class DownGateway:
    async def chat(self, *a: Any) -> dict:
        raise ServiceError("unavailable", "ConnectError: connection refused")


def tool_call(name: str, args: dict[str, Any], id_: str = "c1") -> dict:
    return {"id": id_, "type": "function", "function": {"name": name, "arguments": json.dumps(args)}}


def reply(content: str | None = None, calls: list[dict] | None = None, tokens: int = 100) -> dict:
    msg: dict[str, Any] = {"role": "assistant", "content": content}
    if calls:
        msg["tool_calls"] = calls
    return {"choices": [{"message": msg}], "usage": {"total_tokens": tokens}}


class FakeGateway:
    """Answers with scripted replies; the last one repeats forever."""

    def __init__(self, replies: list[dict]) -> None:
        self.replies = replies
        self.requests: list[tuple[str, list[dict], list[dict]]] = []

    async def chat(self, model: str, messages: list[dict], tools: list[dict]) -> dict:
        self.requests.append((model, json.loads(json.dumps(messages)), tools))
        return self.replies[min(len(self.requests), len(self.replies)) - 1]


class FakeCore:
    def __init__(self, pending: list[dict] | None = None) -> None:
        self.pending_items = pending or []
        self.polls = 0

    async def pending(self) -> list[dict]:
        self.polls += 1
        return self.pending_items


def denied(_args: dict, _key: str) -> ToolResult:
    raise BrokerError("denied", "policy forbids it")
