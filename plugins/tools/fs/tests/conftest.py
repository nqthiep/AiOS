"""Shared fixtures for the `aios.fs` plugin tests."""

from __future__ import annotations

import os
from pathlib import Path

import pytest


@pytest.fixture
def workspace(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    """An empty workspace directory, exported through `AIOS_WORKSPACE`."""
    root = tmp_path / "workspace"
    root.mkdir()
    monkeypatch.setenv("AIOS_WORKSPACE", str(root))
    return Path(os.path.realpath(root))
