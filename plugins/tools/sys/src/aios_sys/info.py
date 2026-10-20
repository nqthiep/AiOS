"""Read-only system information for the `aios.sys` plugin.

Only files under `/proc` and `/sys` are read; disk usage comes from `os.statvfs`.
Spec: docs/m0/tools3.md.
"""

from __future__ import annotations

import os
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from .workspace import workspace_root

PROC_DIR = Path("/proc")
SYS_DIR = Path("/sys")

HOSTNAME_FILE = PROC_DIR / "sys" / "kernel" / "hostname"
KERNEL_FILE = PROC_DIR / "sys" / "kernel" / "osrelease"
UPTIME_FILE = PROC_DIR / "uptime"
MEMINFO_FILE = PROC_DIR / "meminfo"
LOADAVG_FILE = PROC_DIR / "loadavg"

ROOT_PATH = Path("/")


def _read_text(path: Path) -> str:
    """Read a text file. Callers must only pass paths under `/proc` or `/sys`."""
    with open(path, encoding="utf-8", errors="replace") as handle:
        return handle.read()


def _read_meminfo(path: Path) -> dict[str, int]:
    """Parse `/proc/meminfo` into a mapping of field name to bytes."""
    values: dict[str, int] = {}
    for line in _read_text(path).splitlines():
        key, separator, rest = line.partition(":")
        if not separator:
            continue
        parts = rest.split()
        if not parts:
            continue
        try:
            amount = int(parts[0])
        except ValueError:
            continue
        unit = parts[1].lower() if len(parts) > 1 else "b"
        if unit == "kb":
            amount *= 1024
        elif unit == "mb":
            amount *= 1024 * 1024
        values[key.strip()] = amount
    return values


def _read_uptime(path: Path) -> float:
    """Return the uptime in seconds from `/proc/uptime`."""
    fields = _read_text(path).split()
    if not fields:
        return 0.0
    try:
        return float(fields[0])
    except ValueError:
        return 0.0


def _read_loadavg(path: Path) -> tuple[float, float, float]:
    """Return the 1/5/15 minute load averages from `/proc/loadavg`."""
    fields = _read_text(path).split()
    values: list[float] = []
    for field in fields[:3]:
        try:
            values.append(float(field))
        except ValueError:
            values.append(0.0)
    while len(values) < 3:
        values.append(0.0)
    return values[0], values[1], values[2]


def _disk_usage(path: Path) -> dict[str, Any]:
    """Return the total and free bytes of the filesystem holding `path`."""
    stat = os.statvfs(path)
    block_size = stat.f_frsize or stat.f_bsize
    return {
        "path": str(path),
        "total_bytes": stat.f_blocks * block_size,
        "free_bytes": stat.f_bavail * block_size,
    }


def collect_info(workspace: Path | None = None) -> dict[str, Any]:
    """Collect the system information reported by the `sys.info` tool.

    Args:
        workspace: workspace directory; read from `AIOS_WORKSPACE` when omitted.

    Returns:
        A JSON-serialisable mapping with hostname, kernel, uptime, memory, CPU load and disk usage.
    """
    root = workspace_root() if workspace is None else Path(os.path.realpath(workspace))
    memory = _read_meminfo(MEMINFO_FILE)
    load_1m, load_5m, load_15m = _read_loadavg(LOADAVG_FILE)
    return {
        "hostname": _read_text(HOSTNAME_FILE).strip(),
        "kernel": _read_text(KERNEL_FILE).strip(),
        "uptime_seconds": _read_uptime(UPTIME_FILE),
        "memory": {
            "total_bytes": memory.get("MemTotal", 0),
            "free_bytes": memory.get("MemFree", 0),
            "available_bytes": memory.get("MemAvailable", 0),
        },
        "cpu": {
            "load_1m": load_1m,
            "load_5m": load_5m,
            "load_15m": load_15m,
            "cores": os.cpu_count() or 0,
        },
        "disk": {
            "root": _disk_usage(ROOT_PATH),
            "workspace": _disk_usage(root),
        },
        "collected_at": datetime.now(tz=UTC).isoformat(),
    }
