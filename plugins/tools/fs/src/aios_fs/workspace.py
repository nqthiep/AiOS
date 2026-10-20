"""Workspace discovery and containment checks for the `aios.fs` plugin.

Every path the plugin touches is resolved with `os.path.realpath` and compared with
`os.path.commonpath` against the workspace root, so a symlink pointing outside the
workspace is refused or ignored.
"""

from __future__ import annotations

import os
from pathlib import Path

from .errors import FsError

WORKSPACE_ENV = "AIOS_WORKSPACE"


def workspace_root() -> Path:
    """Return the real path of the workspace directory.

    Raises:
        FsError: if `AIOS_WORKSPACE` is unset, empty or not a directory.
    """
    raw = os.environ.get(WORKSPACE_ENV, "").strip()
    if not raw:
        raise FsError(f"{WORKSPACE_ENV} is not set; the aios.fs plugin only works inside the workspace")
    root = Path(raw).expanduser()
    if not root.is_dir():
        raise FsError(f"{WORKSPACE_ENV} does not point to a directory: {raw}")
    return Path(os.path.realpath(root))


def resolve_within(root: Path, candidate: Path) -> Path | None:
    """Return the real path of `candidate`, or `None` when it escapes `root`.

    Symlinks are resolved first, so a link inside the workspace that points outside of it
    is reported as an escape.
    """
    real = Path(os.path.realpath(candidate))
    try:
        common = os.path.commonpath([str(root), str(real)])
    except ValueError:
        return None
    if common != str(root):
        return None
    return real


def safe_path(root: Path, relative: str) -> Path:
    """Resolve `relative` inside `root`.

    Raises:
        FsError: if the path is absolute, contains `..`, or resolves outside the workspace.
    """
    parts = Path(relative).parts
    if Path(relative).is_absolute() or ".." in parts:
        raise FsError(f"path escapes the workspace: {relative}")
    candidate = root / relative
    parent = resolve_within(root, candidate.parent)
    if parent is None:
        raise FsError(f"path escapes the workspace: {relative}")
    if candidate.exists() or candidate.is_symlink():
        real = resolve_within(root, candidate)
        if real is None:
            raise FsError(f"path escapes the workspace: {relative}")
        return real
    return parent / candidate.name
