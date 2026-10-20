"""Tests of the tools `fs.write_note` and `fs.undo_note` (docs/m0/tools3.md)."""

from __future__ import annotations

from pathlib import Path

import pytest

from aios_fs.errors import FsError
from aios_fs.notes import undo_note, write_note
from aios_fs.workspace import workspace_root


def test_workspace_environment_variable_is_mandatory(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv("AIOS_WORKSPACE", raising=False)
    with pytest.raises(FsError):
        workspace_root()


def test_write_note_refuses_path_traversal(workspace: Path) -> None:
    with pytest.raises(FsError):
        write_note(workspace, "../x", "content")
    assert not (workspace.parent / "x.md").exists()
    assert not (workspace / "x.md").exists()


@pytest.mark.parametrize("name", ["a/b", "..", ".", "x" * 81, "x.md", "note;rm -rf /"])
def test_write_note_refuses_invalid_names(workspace: Path, name: str) -> None:
    with pytest.raises(FsError):
        write_note(workspace, name, "content")


def test_write_note_creates_the_note(workspace: Path) -> None:
    result = write_note(workspace, "ghi chú", "nội dung")
    assert result["path"] == "notes/ghi chú.md"
    assert result["created"] is True
    assert result["size_bytes"] == len("nội dung".encode())
    assert (workspace / "notes" / "ghi chú.md").read_text(encoding="utf-8") == "nội dung"


def test_undo_restores_the_previous_content(workspace: Path) -> None:
    first = write_note(workspace, "note", "first")
    assert first["created"] is True
    second = write_note(workspace, "note", "second")
    assert second["created"] is False

    note = workspace / "notes" / "note.md"
    assert note.read_text(encoding="utf-8") == "second"

    result = undo_note(workspace, second["undo_id"])
    assert result["action"] == "restored"
    assert note.read_text(encoding="utf-8") == "first"


def test_undo_deletes_a_note_that_did_not_exist(workspace: Path) -> None:
    created = write_note(workspace, "fresh", "hello")
    note = workspace / "notes" / "fresh.md"
    assert note.exists()

    result = undo_note(workspace, created["undo_id"])
    assert result["action"] == "deleted"
    assert not note.exists()


def test_undo_id_can_be_used_only_once(workspace: Path) -> None:
    created = write_note(workspace, "note", "hello")
    undo_note(workspace, created["undo_id"])
    with pytest.raises(FsError):
        undo_note(workspace, created["undo_id"])


def test_unknown_undo_id_is_refused(workspace: Path) -> None:
    with pytest.raises(FsError):
        undo_note(workspace, "0" * 32)


def test_invalid_undo_id_is_refused(workspace: Path) -> None:
    with pytest.raises(FsError):
        undo_note(workspace, "../escape")


def test_content_is_limited_to_64_kib(workspace: Path) -> None:
    write_note(workspace, "big", "x" * (64 * 1024))
    with pytest.raises(FsError):
        write_note(workspace, "big", "x" * (64 * 1024 + 1))


def test_write_note_refuses_a_notes_directory_outside_the_workspace(workspace: Path, tmp_path: Path) -> None:
    outside = tmp_path / "outside"
    outside.mkdir()
    (workspace / "notes").symlink_to(outside, target_is_directory=True)
    with pytest.raises(FsError):
        write_note(workspace, "note", "content")
    assert not (outside / "note.md").exists()


def test_write_note_refuses_a_note_symlinked_outside_the_workspace(workspace: Path, tmp_path: Path) -> None:
    outside = tmp_path / "outside.md"
    outside.write_text("original", encoding="utf-8")
    (workspace / "notes").mkdir()
    (workspace / "notes" / "note.md").symlink_to(outside)
    with pytest.raises(FsError):
        write_note(workspace, "note", "content")
    assert outside.read_text(encoding="utf-8") == "original"
