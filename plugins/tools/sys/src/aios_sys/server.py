"""MCP server of the `aios.sys` plugin (stdio transport).

The server only exposes the read-only tool `sys.info`. Logs go to stderr because
stdout carries the MCP protocol stream.
"""

from __future__ import annotations

import json
import logging
import sys
from datetime import UTC, datetime
from typing import Any

from mcp.server import MCPServer
from mcp.server.mcpserver.exceptions import ToolError

from .info import collect_info
from .workspace import WorkspaceError

LOGGER = logging.getLogger("aios.sys")

INSTRUCTIONS = (
    "Read-only system information of the AiOS host: hostname, kernel, uptime, memory, "
    "CPU load and disk usage of / and of the workspace."
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


mcp = MCPServer("aios.sys", version="0.1.0", instructions=INSTRUCTIONS)


@mcp.tool(
    name="sys.info",
    description=(
        "Report the host system information: hostname, kernel release, uptime, total/free/available "
        "RAM, CPU load averages and the disk usage of / and of the workspace."
    ),
)
def sys_info() -> dict[str, Any]:
    """Collect the system information of the AiOS host.

    Only `/proc`, `/sys` and `os.statvfs` are read; no file outside them is opened.
    """
    try:
        info = collect_info()
    except WorkspaceError as exc:
        raise ToolError(str(exc)) from exc
    LOGGER.info("sys.info collected")
    return info


def main() -> None:
    """Run the MCP server over stdio."""
    _configure_logging()
    mcp.run(transport="stdio")


if __name__ == "__main__":  # pragma: no cover - entry point
    main()
