"""Shared fixtures for the `aios.sys` plugin tests."""

from __future__ import annotations

import os
import sys
from pathlib import Path

import pytest


def pytest_collection_modifyitems(config: pytest.Config, items: list[pytest.Item]) -> None:
    """sys.info reads /proc and /sys: the tests only make sense on Linux (run them in the VM)."""
    if sys.platform != "linux":
        skip = pytest.mark.skip(reason="sys.info needs Linux /proc and /sys; run via dev/vm-run.sh")
        for item in items:
            item.add_marker(skip)


@pytest.fixture
def workspace(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    """An empty workspace directory, exported through `AIOS_WORKSPACE`."""
    root = tmp_path / "workspace"
    root.mkdir()
    monkeypatch.setenv("AIOS_WORKSPACE", str(root))
    return Path(os.path.realpath(root))
