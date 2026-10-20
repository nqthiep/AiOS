"""MCP server of the `aios.fs` plugin (stdio transport).

Logs go to stderr because stdout carries the MCP protocol stream.
"""

from __future__ import annotations

import json
import logging
import sys
from datetime import UTC, datetime
from typing import Any

from mcp.server import MCPServer
from mcp.server.mcpserver.exceptions import ToolError

from .errors import FsError
from .notes import undo_note, write_note
from .search import MAX_RESULTS, search_files
from .workspace import workspace_root

LOGGER = logging.getLogger("aios.fs")

INSTRUCTIONS = (
    "Filesystem tools scoped to the workspace: search files by name, write markdown notes "
    "and undo the last write of a note."
)


class _JsonFormatter(logging.Formatter):
    """Format log records as one JSON object per line."""

    def format(self, record: logging.LogRecord) -> str:
        payload = {
            "ts": datetime.fromtimestamp(record.created, tz=UTC).isoformat(),
            "level": record.levelname,
            "logger": record.name,
            "message": record.getMessage(),
        }
        return json.dumps(payload, ensure_ascii=False)


def _configure_logging() -> None:
    """Send the plugin logs to stderr as JSON (never to stdout)."""
    if LOGGER.handlers:
        return
    handler = logging.StreamHandler(sys.stderr)
    handler.setFormatter(_JsonFormatter())
    LOGGER.addHandler(handler)
    LOGGER.setLevel(logging.INFO)
    LOGGER.propagate = False


mcp = MCPServer("aios.fs", version="0.1.0", instructions=INSTRUCTIONS)


@mcp.tool(
    name="fs.search",
    description=(
        "Search files inside the workspace by name. The pattern is a glob matched against file "
        "names, or against paths relative to the workspace when it contains a '/'. The comparison "
        "ignores case and diacritics. At most 50 results are returned."
    ),
)
def fs_search(pattern: str, limit: int | None = None) -> dict[str, Any]:
    """Search the files of the workspace.

    Args:
        pattern: glob pattern, for example `*.md`.
        limit: maximum number of results (default and hard cap: 50).

    Returns:
        The hits, each with its path relative to the workspace, its size and its modification time.
    """
    try:
        root = workspace_root()
        effective_limit = MAX_RESULTS if limit is None else limit
        results = search_files(root, pattern, effective_limit)
    except FsError as exc:
        raise ToolError(str(exc)) from exc
    LOGGER.info("fs.search returned %d results", len(results))
    return {
        "pattern": pattern,
        "count": len(results),
        "max_results": MAX_RESULTS,
        "results": results,
    }


@mcp.tool(
    name="fs.write_note",
    description=(
        "Write a markdown note into the notes directory of the workspace. When the note already "
        "exists, its previous content is kept so the write can be undone. The returned undo id is "
        "the argument of fs.undo_note."
    ),
)
def fs_write_note(name: str, content: str) -> dict[str, Any]:
    """Write a note into the workspace.

    Args:
        name: note name without extension (letters, digits, spaces, '_' and '-', at most 80).
        content: markdown content of the note, at most 64 KiB.

    Returns:
        The relative path of the note and the undo id of this write.
    """
    try:
        result = write_note(workspace_root(), name, content)
    except FsError as exc:
        raise ToolError(str(exc)) from exc
    LOGGER.info("fs.write_note wrote %s", result["path"])
    return result


@mcp.tool(
    name="fs.undo_note",
    description=(
        "Undo a previous fs.write_note call: the previous content of the note is restored, or the "
        "note is deleted when it did not exist before. An undo id can only be used once."
    ),
)
def fs_undo_note(undo_id: str) -> dict[str, Any]:
    """Undo a note write.

    Args:
        undo_id: undo id returned by fs.write_note.

    Returns:
        The relative path of the note and the action that was applied.
    """
    try:
        result = undo_note(workspace_root(), undo_id)
    except FsError as exc:
        raise ToolError(str(exc)) from exc
    LOGGER.info("fs.undo_note %s", result["action"])
    return result


def main() -> None:
    """Run the MCP server over stdio."""
    _configure_logging()
    mcp.run(transport="stdio")


if __name__ == "__main__":  # pragma: no cover - entry point
    main()
