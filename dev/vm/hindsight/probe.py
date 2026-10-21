"""Acceptance probe for spike S-4 (docs/m0/hindsight.md).

Run in the VM with the Hindsight venv:
    /opt/aios/hindsight/bin/python dev/vm/hindsight/probe.py

Steps: create the 7 banks, retain 20 Vietnamese sentences into `user-personal`,
recall "tôi làm việc lúc mấy giờ" (relevant sentence in top 3), check that
`ext-personal` does not see `user-personal`, check from the gateway journal that
every LLM and embedding call went to a local route, and print RAM numbers.

Config (environment): AIOS_HINDSIGHT_ENV (default /etc/aios/hindsight/<uid>.env),
AIOS_GATEWAY_UNIT (default aios-model-gateway.service, user unit).
Exit code 0 only when every check passes.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import threading
import time
from datetime import UTC, datetime
from pathlib import Path

from hindsight_client import Hindsight

BANKS = [
    "user-public",
    "user-personal",
    "user-secret",
    "system",
    "ext-public",
    "ext-personal",
    "ext-secret",
]
LOCAL_ROUTES = {"s1-local", "embed-local"}
QUERY = "tôi làm việc lúc mấy giờ"
# Documents holding the answer (working hours). Matched by document_id, since
# the extracted fact text is LLM-rewritten (and may be in another language).
RELEVANT_DOCS = {"up-00"}

USER_PERSONAL = [
    "Tôi làm việc từ 8 giờ sáng đến 5 giờ chiều, từ thứ Hai đến thứ Sáu.",
    "Tôi tên là Minh, năm nay 34 tuổi.",
    "Tôi sống ở quận Cầu Giấy, Hà Nội.",
    "Vợ tôi tên là Lan, cô ấy là giáo viên tiểu học.",
    "Con gái tôi tên là An, đang học lớp ba.",
    "Tôi làm kỹ sư phần mềm ở một công ty fintech.",
    "Mỗi sáng tôi uống một ly cà phê sữa đá trước khi đi làm.",
    "Tôi đi làm bằng xe máy, mất khoảng 25 phút.",
    "Tôi dị ứng với tôm và cua.",
    "Món ăn tôi thích nhất là phở bò tái.",
    "Tối thứ Ba và thứ Năm tôi chơi cầu lông với đồng nghiệp.",
    "Tôi đang học tiếng Nhật, mục tiêu là thi N3 vào tháng 12.",
    "Bác sĩ dặn tôi uống thuốc huyết áp mỗi tối sau bữa ăn.",
    "Sinh nhật mẹ tôi là ngày 14 tháng 2.",
    "Tôi thích nghe nhạc Trịnh Công Sơn khi lái xe.",
    "Cuối tuần tôi thường về quê ở Nam Định thăm bố mẹ.",
    "Tôi đang tiết kiệm tiền để mua một căn hộ hai phòng ngủ.",
    "Laptop của tôi là ThinkPad, chạy Fedora.",
    "Tôi ngủ lúc 11 giờ đêm và dậy lúc 6 giờ sáng.",
    "Tôi không thích họp vào buổi chiều thứ Sáu.",
]
# Tainted bank: external content on a different topic.
EXT_PERSONAL = [
    "Một trang web quảng cáo nói rằng khóa học yoga mở cửa lúc 9 giờ sáng.",
    "Email khuyến mãi từ siêu thị: giảm giá 30% rau củ vào cuối tuần.",
    "Bài báo viết rằng giá vàng tăng mạnh trong tháng này.",
]


def env_file() -> Path:
    default = f"/etc/aios/hindsight/{os.getuid()}.env"
    return Path(os.environ.get("AIOS_HINDSIGHT_ENV", default))


def read_env(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in path.read_text().splitlines():
        line = line.strip()
        if line and not line.startswith("#") and "=" in line:
            key, _, value = line.partition("=")
            values[key] = value
    return values


def service_main_pid(unit: str) -> int:
    out = subprocess.run(
        ["systemctl", "show", unit, "-p", "MainPID", "--value"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
    return int(out)


def memory_current(unit: str) -> int:
    out = subprocess.run(
        ["systemctl", "show", unit, "-p", "MemoryCurrent", "--value"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
    return int(out)


def status_kib(pid: int, field: str) -> int:
    for line in Path(f"/proc/{pid}/status").read_text().splitlines():
        if line.startswith(f"{field}:"):
            return int(line.split()[1])
    raise RuntimeError(f"no {field} for pid {pid}")


def vm_rss_kib(pid: int) -> int:
    return status_kib(pid, "VmRSS")


def mem_line(pid: int, unit: str) -> str:
    """VmRSS and VmSwap (the VM swaps under llama-server pressure) and the unit's MemoryCurrent."""
    return (
        f"VmRSS={vm_rss_kib(pid)} kB VmSwap={status_kib(pid, 'VmSwap')} kB "
        f"MemoryCurrent={mib(memory_current(unit))}"
    )


class RssSampler:
    """Poll VmRSS of one pid in a thread, keep the maximum."""

    def __init__(self, pid: int, interval: float = 0.1) -> None:
        self.pid, self.interval, self.peak = pid, interval, 0
        self._stop = threading.Event()
        self._thread = threading.Thread(target=self._run, daemon=True)

    def _run(self) -> None:
        while not self._stop.is_set():
            self.peak = max(self.peak, vm_rss_kib(self.pid))
            time.sleep(self.interval)

    def __enter__(self) -> RssSampler:
        self._thread.start()
        return self

    def __exit__(self, *_: object) -> None:
        self._stop.set()
        self._thread.join()


def gateway_records(unit: str, since: datetime) -> list[dict]:
    out = subprocess.run(
        [
            "journalctl",
            "--user",
            "-u",
            unit,
            "--since",
            since.strftime("%Y-%m-%d %H:%M:%S"),
            "-o",
            "cat",
            "--no-pager",
        ],
        capture_output=True,
        text=True,
        check=True,
        env={**os.environ, "TZ": "UTC"},
    ).stdout
    records = []
    for line in out.splitlines():
        try:
            records.append(json.loads(line).get("fields", {}))
        except json.JSONDecodeError:
            continue
    return records


def mib(n_bytes: int) -> str:
    return f"{n_bytes / 1048576:.0f} MiB"


def main() -> int:
    sys.stdout.reconfigure(line_buffering=True)
    env = read_env(env_file())
    unit = f"hindsight-api@{os.getuid()}.service"
    gateway_unit = os.environ.get("AIOS_GATEWAY_UNIT", "aios-model-gateway.service")
    url = f"http://{env['HINDSIGHT_API_HOST']}:{env['HINDSIGHT_API_PORT']}"
    hs = Hindsight(base_url=url, api_key=env["HINDSIGHT_API_TENANT_API_KEY"], timeout=1800)
    started = datetime.now(UTC)
    failures: list[str] = []
    pid = service_main_pid(unit)

    print(f"[probe] {unit} pid={pid} idle: {mem_line(pid, unit)}")

    # Fresh test banks, then make sure all 7 banks exist.
    for bank in ("user-personal", "ext-personal"):
        try:
            hs.delete_bank(bank)
        except Exception:  # noqa: BLE001 - bank may not exist yet
            pass
    # Test banks: no background observation consolidation. On a CPU-only 4B
    # model it takes minutes per memory and evicts the llama-server prompt cache
    # that retain relies on (docs/spikes/S-4-hindsight.md).
    test_banks = {"user-personal", "ext-personal"}
    listed = {
        hs.create_bank(bank, name=bank, enable_observations=False if bank in test_banks else None).bank_id
        for bank in BANKS
    }
    missing = sorted(set(BANKS) - listed)
    print(f"[probe] banks: {sorted(set(BANKS) & listed)}")
    if missing:
        failures.append(f"missing banks: {missing}")

    # Retain 20 sentences, one call each (sync retain = extraction done on return).
    t0 = time.monotonic()
    for i, text in enumerate(USER_PERSONAL):
        t = time.monotonic()
        r = hs.retain("user-personal", text, document_id=f"up-{i:02d}", metadata={"src": "probe"})
        if not r.success:
            failures.append(f"retain up-{i:02d} failed")
        print(f"[probe] retain up-{i:02d} ok={r.success} {time.monotonic() - t:.1f}s")
    retain_s = time.monotonic() - t0
    for i, text in enumerate(EXT_PERSONAL):
        hs.retain("ext-personal", text, document_id=f"ep-{i:02d}", metadata={"src": "probe"})
    print(
        f"[probe] retain 20 sentences: {retain_s:.1f}s total, {retain_s / 20:.1f}s/sentence; "
        f"after retain: {mem_line(pid, unit)}"
    )

    # Recall: relevant fact in the top 3. Repeat to sample RSS and latency.
    latencies = []
    with RssSampler(pid) as sampler:
        for _ in range(10):
            t = time.monotonic()
            res = hs.recall("user-personal", QUERY, budget="mid")
            latencies.append(time.monotonic() - t)
    top3 = res.results[:3]
    print(f"[probe] recall {QUERY!r} top3:")
    for n, r in enumerate(top3, 1):
        print(f"  {n}. [{r.document_id}] {r.text}")
    if not any(r.document_id in RELEVANT_DOCS for r in top3):
        failures.append("no working-hours fact in recall top 3")
    print(
        f"[probe] recall latency: avg={sum(latencies) / len(latencies):.2f}s "
        f"min={min(latencies):.2f}s max={max(latencies):.2f}s first={latencies[0]:.2f}s; "
        f"VmRSS peak during recall={sampler.peak} kB; after recall: {mem_line(pid, unit)}"
    )

    # Bank isolation: ext-personal must only return its own documents.
    ext = hs.recall("ext-personal", QUERY, budget="mid")
    leaked = [r.text for r in ext.results if not (r.document_id or "").startswith("ep-")]
    print(
        f"[probe] ext-personal recall: {len(ext.results)} results, "
        f"documents={sorted({r.document_id for r in ext.results})}"
    )
    if leaked:
        failures.append(f"ext-personal sees other bank: {leaked}")
    own = hs.recall("user-personal", "giá vàng khuyến mãi siêu thị yoga", budget="mid")
    leaked_back = [r.text for r in own.results if (r.document_id or "").startswith("ep-")]
    if leaked_back:
        failures.append(f"user-personal sees ext-personal: {leaked_back}")

    # Gateway journal: every Hindsight model call since start went to a local route.
    records = gateway_records(gateway_unit, started)
    chats = [r for r in records if r.get("message") == "model request completed"]
    embeds = [r for r in records if r.get("message") == "embedding request completed"]
    failed = [r for r in records if r.get("message") == "upstream call failed, trying next candidate"]
    non_local = [
        r for r in chats + embeds if r.get("route") not in LOCAL_ROUTES or r.get("kind", "local") != "local"
    ]
    tokens_in = sum(r.get("prompt_tokens", 0) for r in chats)
    tokens_out = sum(r.get("completion_tokens", 0) for r in chats)
    print(
        f"[probe] gateway: {len(chats)} chat (routes {sorted({r.get('route') for r in chats})}, "
        f"{tokens_in} in / {tokens_out} out tokens), {len(embeds)} embedding "
        f"(routes {sorted({r.get('route') for r in embeds})}), {len(failed)} upstream errors, "
        f"{len(non_local)} non-local"
    )
    if not chats or not embeds:
        failures.append("gateway journal has no chat or no embedding call from the probe")
    if non_local:
        failures.append(f"non-local gateway calls: {non_local}")

    print(f"[probe] end: {mem_line(pid, unit)}")
    hs.close()
    if failures:
        for failure in failures:
            print(f"[probe] FAIL: {failure}")
        return 1
    print("[probe] PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
