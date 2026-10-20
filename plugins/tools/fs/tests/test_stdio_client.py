"""Call the `aios.fs` tools through a real MCP stdio client (SDK `mcp`)."""

from __future__ import annotations

import asyncio
import sys
from pathlib import Path

from mcp import Client, StdioServerParameters

PLUGIN_DIR = Path(__file__).resolve().parents[1]


def _parameters(workspace: Path) -> StdioServerParameters:
    """Start the plugin like the plugin host does: `python -I server.py` over stdio."""
    return StdioServerParameters(
        command=sys.executable,
        args=["-I", "server.py"],
        cwd=PLUGIN_DIR,
        env={"AIOS_WORKSPACE": str(workspace)},
    )


async def _scenario(workspace: Path) -> dict[str, object]:
    async with Client(_parameters(workspace)) as client:
        tools = await client.list_tools()
        names = sorted(tool.name for tool in tools.tools)
        written = await client.call_tool("fs.write_note", {"name": "note", "content": "hello"})
        content_on_disk = (workspace / "notes" / "note.md").read_text(encoding="utf-8")
        searched = await client.call_tool("fs.search", {"pattern": "*.md"})
        undo_id = written.structured_content["undo_id"]
        undone = await client.call_tool("fs.undo_note", {"undo_id": undo_id})
        refused = await client.call_tool("fs.write_note", {"name": "../x", "content": "nope"})
        return {
            "names": names,
            "written": written,
            "content_on_disk": content_on_disk,
            "searched": searched,
            "undone": undone,
            "refused": refused,
        }


def test_fs_tools_over_stdio(workspace: Path) -> None:
    outcome = asyncio.run(_scenario(workspace))

    assert outcome["names"] == ["fs.search", "fs.undo_note", "fs.write_note"]

    written = outcome["written"]
    assert not written.is_error
    assert written.structured_content["path"] == "notes/note.md"
    assert outcome["content_on_disk"] == "hello"

    searched = outcome["searched"]
    assert not searched.is_error
    assert searched.structured_content["count"] == 1

    undone = outcome["undone"]
    assert not undone.is_error
    assert undone.structured_content["action"] == "deleted"
    assert not (workspace / "notes" / "note.md").exists()

    refused = outcome["refused"]
    assert refused.is_error
    assert "invalid note name" in refused.content[0].text
