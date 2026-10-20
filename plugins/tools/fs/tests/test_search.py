"""Tests of the tool `fs.search` (docs/m0/tools3.md)."""

from __future__ import annotations

from pathlib import Path

import pytest

from aios_fs.errors import FsError
from aios_fs.search import MAX_RESULTS, search_files


def test_search_never_leaves_the_workspace(workspace: Path, tmp_path: Path) -> None:
    (workspace / "inside.md").write_text("inside", encoding="utf-8")

    outside = tmp_path / "outside.md"
    outside.write_text("secret", encoding="utf-8")
    (workspace / "escape.md").symlink_to(outside)

    outside_dir = tmp_path / "outside-dir"
    outside_dir.mkdir()
    (outside_dir / "deep.md").write_text("secret", encoding="utf-8")
    (workspace / "escape-dir").symlink_to(outside_dir, target_is_directory=True)

    results = search_files(workspace, "*.md")
    assert [item["path"] for item in results] == ["inside.md"]


def test_search_ignores_case_and_diacritics(workspace: Path) -> None:
    (workspace / "Báo cáo.md").write_text("x", encoding="utf-8")
    assert [item["path"] for item in search_files(workspace, "bao cao*.md")] == ["Báo cáo.md"]
    assert [item["path"] for item in search_files(workspace, "BAO*.md")] == ["Báo cáo.md"]


def test_search_returns_size_and_modification_time(workspace: Path) -> None:
    (workspace / "note.md").write_text("hello", encoding="utf-8")
    item = search_files(workspace, "*.md")[0]
    assert item["path"] == "note.md"
    assert item["size_bytes"] == 5
    assert item["modified_at"].endswith("+00:00")


def test_search_caps_the_number_of_results(workspace: Path) -> None:
    for index in range(MAX_RESULTS + 10):
        (workspace / f"file{index:03d}.md").write_text("x", encoding="utf-8")
    assert len(search_files(workspace, "*.md")) == MAX_RESULTS
    assert len(search_files(workspace, "*.md", limit=5)) == 5


def test_search_matches_paths_relative_to_the_workspace(workspace: Path) -> None:
    (workspace / "notes").mkdir()
    (workspace / "notes" / "a.md").write_text("x", encoding="utf-8")
    (workspace / "b.md").write_text("x", encoding="utf-8")
    assert [item["path"] for item in search_files(workspace, "notes/*.md")] == ["notes/a.md"]


def test_search_refuses_an_empty_pattern(workspace: Path) -> None:
    with pytest.raises(FsError):
        search_files(workspace, "   ")


def test_search_refuses_a_limit_below_one(workspace: Path) -> None:
    with pytest.raises(FsError):
        search_files(workspace, "*.md", limit=0)
