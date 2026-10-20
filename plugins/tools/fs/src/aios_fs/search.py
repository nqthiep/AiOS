"""Glob search inside the workspace (tool `fs.search`).

Spec: docs/m0/tools3.md.
"""

from __future__ import annotations

import fnmatch
import os
import unicodedata
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from .errors import FsError
from .workspace import resolve_within

MAX_RESULTS = 50


def normalize(text: str) -> str:
    """Case-fold a string and drop its diacritics so comparisons ignore them.

    The Vietnamese letter `đ` has no canonical decomposition, so it is mapped to `d`
    explicitly.
    """
    decomposed = unicodedata.normalize("NFD", text.casefold())
    without_marks = "".join(char for char in decomposed if not unicodedata.combining(char))
    return without_marks.replace("đ", "d")


def matches(pattern: str, name: str) -> bool:
    """Return whether `name` matches the glob `pattern`, ignoring case and diacritics."""
    return fnmatch.fnmatchcase(normalize(name), normalize(pattern))


def search_files(root: Path, pattern: str, limit: int = MAX_RESULTS) -> list[dict[str, Any]]:
    """Search files of the workspace by name.

    The pattern is matched against the file name, or against the path relative to the
    workspace when it contains a `/`. Files that resolve outside the workspace (symlinks
    pointing outside) are skipped.

    Args:
        root: real path of the workspace.
        pattern: glob pattern.
        limit: maximum number of results, capped at `MAX_RESULTS`.

    Returns:
        One mapping per hit: relative path, size in bytes and modification time.
    """
    if not pattern.strip():
        raise FsError("the search pattern must not be empty")
    if limit < 1:
        raise FsError(f"the search limit must be at least 1, got {limit}")
    limit = min(limit, MAX_RESULTS)
    match_on_path = "/" in pattern
    results: list[dict[str, Any]] = []
    for dirpath, dirnames, filenames in os.walk(root, followlinks=False):
        dirnames.sort()
        for filename in sorted(filenames):
            path = Path(dirpath) / filename
            real = resolve_within(root, path)
            if real is None or not real.is_file():
                continue
            relative = path.relative_to(root).as_posix()
            subject = relative if match_on_path else filename
            if not matches(pattern, subject):
                continue
            stat = real.stat()
            results.append(
                {
                    "path": relative,
                    "size_bytes": stat.st_size,
                    "modified_at": datetime.fromtimestamp(stat.st_mtime, tz=UTC).isoformat(),
                }
            )
            if len(results) >= limit:
                return results
    return results
