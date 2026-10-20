"""Tests of the read-only tool `sys.info` (docs/m0/tools3.md)."""

from __future__ import annotations

import builtins
import os
from pathlib import Path
from typing import Any

import pytest

from aios_sys.info import collect_info
from aios_sys.workspace import WorkspaceError, workspace_root


def test_info_reads_only_proc_and_sys(workspace: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """`sys.info` must not open any file outside `/proc` and `/sys`."""
    opened: list[str] = []
    real_open = builtins.open

    def recording_open(file: Any, *args: Any, **kwargs: Any) -> Any:
        try:
            opened.append(os.fspath(file))
        except TypeError:
            opened.append(repr(file))
        return real_open(file, *args, **kwargs)

    monkeypatch.setattr(builtins, "open", recording_open)
    info = collect_info(workspace)

    assert info["hostname"]
    assert info["kernel"]
    assert info["uptime_seconds"] > 0
    assert info["memory"]["total_bytes"] > 0
    assert info["cpu"]["cores"] > 0
    assert info["disk"]["root"]["total_bytes"] > 0
    assert info["disk"]["workspace"]["total_bytes"] > 0

    assert opened, "sys.info must read something"
    for path in opened:
        assert path.startswith(("/proc/", "/sys/")), path


def test_workspace_environment_variable_is_mandatory(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv("AIOS_WORKSPACE", raising=False)
    with pytest.raises(WorkspaceError):
        workspace_root()
    with pytest.raises(WorkspaceError):
        collect_info()


def test_workspace_must_be_a_directory(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("AIOS_WORKSPACE", str(tmp_path / "missing"))
    with pytest.raises(WorkspaceError):
        workspace_root()
