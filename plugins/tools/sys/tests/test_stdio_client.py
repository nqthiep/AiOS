"""Call the `aios.sys` tools through a real MCP stdio client (SDK `mcp`)."""

from __future__ import annotations

import asyncio
import sys
from pathlib import Path

from mcp import Client, StdioServerParameters

PLUGIN_DIR = Path(__file__).resolve().parents[1]


def _parameters(workspace: Path | None) -> StdioServerParameters:
    """Start the plugin like the plugin host does: `python -I server.py` over stdio."""
    return StdioServerParameters(
        command=sys.executable,
        args=["-I", "server.py"],
        cwd=PLUGIN_DIR,
        env={} if workspace is None else {"AIOS_WORKSPACE": str(workspace)},
    )


async def _scenario(workspace: Path | None) -> tuple[list[str], object, object]:
    async with Client(_parameters(workspace)) as client:
        tools = await client.list_tools()
        names = sorted(tool.name for tool in tools.tools)
        result = await client.call_tool("sys.info", {})
        return names, result, tools.tools[0].input_schema


def test_sys_info_over_stdio(workspace: Path) -> None:
    names, result, schema = asyncio.run(_scenario(workspace))
    assert names == ["sys.info"]
    assert schema["type"] == "object"
    assert not result.is_error
    assert result.structured_content is not None
    payload = result.structured_content
    assert payload["hostname"]
    assert payload["memory"]["total_bytes"] > 0


def test_sys_info_over_stdio_without_workspace(workspace: Path) -> None:
    names, result, _ = asyncio.run(_scenario(None))
    assert names == ["sys.info"]
    assert result.is_error
    assert "AIOS_WORKSPACE" in result.content[0].text
