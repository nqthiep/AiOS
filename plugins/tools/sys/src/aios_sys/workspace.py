"""Workspace discovery for the `aios.sys` plugin.

The workspace directory comes from the `AIOS_WORKSPACE` environment variable and is
mandatory: without it the plugin cannot report the workspace disk usage.
"""

from __future__ import annotations

import os
from pathlib import Path

WORKSPACE_ENV = "AIOS_WORKSPACE"


class WorkspaceError(Exception):
    """Raised when `AIOS_WORKSPACE` is missing or does not point to a directory."""


def workspace_root() -> Path:
    """Return the real path of the workspace directory.

    Raises:
        WorkspaceError: if `AIOS_WORKSPACE` is unset, empty or not a directory.
    """
    raw = os.environ.get(WORKSPACE_ENV, "").strip()
    if not raw:
        raise WorkspaceError(
            f"{WORKSPACE_ENV} is not set; the aios.sys plugin needs it to report the workspace disk usage"
        )
    root = Path(raw).expanduser()
    if not root.is_dir():
        raise WorkspaceError(f"{WORKSPACE_ENV} does not point to a directory: {raw}")
    return Path(os.path.realpath(root))
