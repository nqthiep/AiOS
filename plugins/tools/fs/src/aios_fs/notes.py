"""Notes with a one-step undo (tools `fs.write_note` and `fs.undo_note`).

A note lives in `$WORKSPACE/notes/<name>.md`. When the note already exists, its previous
content is kept in `$WORKSPACE/.aios-undo/<undo_id>` and the undo id is returned to the
caller. Spec: docs/m0/tools3.md.
"""

from __future__ import annotations

import json
import os
import re
import uuid
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from .errors import FsError
from .workspace import safe_path

NOTES_DIR = "notes"
UNDO_DIR = ".aios-undo"
MAX_CONTENT_BYTES = 64 * 1024

# Equivalent of the documented pattern `^[\p{L}\p{N} _-]{1,80}$`: Python's `\w` covers
# Unicode letters and digits plus the underscore.
NAME_PATTERN = re.compile(r"[\w \-]{1,80}", re.UNICODE)
UNDO_ID_PATTERN = re.compile(r"[0-9a-f]{32}")


def validate_name(name: str) -> str:
    """Check the note name.

    Raises:
        FsError: if the name is empty, too long, or holds anything but letters, digits,
            spaces, underscores and hyphens (no path separator, no `..`).
    """
    if not NAME_PATTERN.fullmatch(name):
        raise FsError(
            "invalid note name: use 1 to 80 letters, digits, spaces, '_' or '-', without any path separator"
        )
    return name


def _atomic_write(path: Path, data: bytes) -> None:
    """Write `data` to `path` through a temporary file in the same directory."""
    temporary = path.with_name(f".{path.name}.{uuid.uuid4().hex}.tmp")
    try:
        with open(temporary, "wb") as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    except OSError:
        temporary.unlink(missing_ok=True)
        raise


def write_note(root: Path, name: str, content: str) -> dict[str, Any]:
    """Write a note into the workspace and keep the previous version for undo.

    Args:
        root: real path of the workspace.
        name: note name without extension.
        content: markdown content, at most 64 KiB.

    Returns:
        The relative path of the note, the undo id and whether the note was created.
    """
    validate_name(name)
    encoded = content.encode("utf-8")
    if len(encoded) > MAX_CONTENT_BYTES:
        raise FsError(f"the note content is too large: {len(encoded)} bytes, limit is {MAX_CONTENT_BYTES}")

    relative = f"{NOTES_DIR}/{name}.md"
    target = safe_path(root, relative)
    notes_dir = safe_path(root, NOTES_DIR)
    notes_dir.mkdir(parents=True, exist_ok=True)
    undo_dir = safe_path(root, UNDO_DIR)
    undo_dir.mkdir(parents=True, exist_ok=True)

    undo_id = uuid.uuid4().hex
    had_original = target.exists()
    if had_original:
        (undo_dir / undo_id).write_bytes(target.read_bytes())

    target.parent.mkdir(parents=True, exist_ok=True)
    _atomic_write(target, encoded)

    record = {
        "undo_id": undo_id,
        "target": relative,
        "had_original": had_original,
        "created_at": datetime.now(tz=UTC).isoformat(),
    }
    (undo_dir / f"{undo_id}.json").write_text(json.dumps(record, ensure_ascii=False), encoding="utf-8")

    return {
        "path": relative,
        "undo_id": undo_id,
        "created": not had_original,
        "size_bytes": len(encoded),
    }


def undo_note(root: Path, undo_id: str) -> dict[str, Any]:
    """Undo a previous write: restore the previous content, or delete the new note.

    The undo record is consumed, so an undo id can only be used once.

    Args:
        root: real path of the workspace.
        undo_id: undo id returned by `write_note`.

    Returns:
        The relative path of the note and the action that was applied.
    """
    if not UNDO_ID_PATTERN.fullmatch(undo_id):
        raise FsError(f"invalid undo id: {undo_id!r}")

    undo_dir = safe_path(root, UNDO_DIR)
    record_path = undo_dir / f"{undo_id}.json"
    if not record_path.is_file():
        raise FsError(f"unknown undo id: {undo_id}")
    try:
        record = json.loads(record_path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise FsError(f"the undo record of {undo_id} is unreadable") from exc

    relative = str(record.get("target", ""))
    target = safe_path(root, relative)
    backup = undo_dir / undo_id

    if record.get("had_original"):
        if not backup.is_file():
            raise FsError(f"the undo backup of {undo_id} is missing")
        target.parent.mkdir(parents=True, exist_ok=True)
        _atomic_write(target, backup.read_bytes())
        action = "restored"
    else:
        if target.exists():
            target.unlink()
        action = "deleted"

    backup.unlink(missing_ok=True)
    record_path.unlink(missing_ok=True)

    return {"path": relative, "undo_id": undo_id, "action": action}
