#!/usr/bin/env python3
"""Trích yêu cầu (PHẢI / KHÔNG ĐƯỢC / NÊN / CÓ THỂ) từ tài liệu gốc vào docs/requirements.md.

Chạy: python3 scripts/extract_requirements.py          # ghi docs/requirements.md
      python3 scripts/extract_requirements.py --check  # chỉ báo file có lỗi thời không
"""
import datetime
import re
import sys
from pathlib import Path

DOCS = Path(__file__).resolve().parent.parent / "docs"
SOURCES = [("D", "design.md"), ("I", "integration.md"), ("R", "roadmap.md"), ("T", "tech-stack.md")]
MANDATORY, RECOMMENDED, OPTIONAL = "Bắt buộc", "Khuyến nghị", "Tùy chọn"
HEADING = re.compile(r"^#{2,4} (?:(\d+(?:\.\d+)*)\.? )?")
PIPE = re.compile(r"(?<!\\)\|")


def level(text):
    if "PHẢI" in text or "KHÔNG ĐƯỢC" in text:
        return MANDATORY
    if "NÊN" in text:
        return RECOMMENDED
    if "CÓ THỂ" in text:
        return OPTIONAL
    return None


def unit(line):
    """Một dòng nguồn → văn bản yêu cầu: hàng bảng nối ô bằng ' — ', mục danh sách bỏ ký hiệu đầu dòng."""
    line = line.strip()
    if line.startswith("|"):
        return " — ".join(c.strip() for c in PIPE.split(line.strip("|")))
    return re.sub(r"^(?:[-*]|\d+\.)\s+(?:\[[ x]\]\s+)?", "", line)


def extract(prefix, name):
    section, counters, fence, out = "0", {}, False, []
    for line in (DOCS / name).read_text(encoding="utf-8").splitlines():
        if line.lstrip().startswith("```"):
            fence = not fence
            continue
        if fence:
            continue
        m = HEADING.match(line)
        if m:
            if m.group(1):
                section = m.group(1)
            continue
        if line.startswith("Từ khóa") or line.startswith("File này được trích"):
            continue  # dòng định nghĩa từ khóa, không phải yêu cầu
        lv = level(line)
        if not lv:
            continue
        counters[section] = counters.get(section, 0) + 1
        out.append((f"REQ-{prefix}-{section}-{counters[section]:02d}", lv, unit(line), name, section))
    return out


def render(reqs):
    count = {lv: sum(r[1] == lv for r in reqs) for lv in (MANDATORY, RECOMMENDED, OPTIONAL)}
    links = ", ".join(f"[{n}]({n})" for _, n in SOURCES)
    head = [
        "# AiOS — Danh sách yêu cầu",
        "",
        f"_Cập nhật: {datetime.date.today().isoformat()}_",
        "",
        f"File này được trích tự động bằng `scripts/extract_requirements.py` từ {links}: mỗi dòng "
        "(đoạn văn, mục danh sách hoặc hàng bảng, ngoài khối code) chứa từ khóa **PHẢI**, **KHÔNG ĐƯỢC**, "
        "**NÊN** hoặc **CÓ THỂ** là một yêu cầu. Nguồn chuẩn là các tài liệu gốc; khi sửa yêu cầu, "
        "sửa ở tài liệu gốc rồi chạy lại script.",
        "",
        f"Tổng: {len(reqs)} yêu cầu — {count[MANDATORY]} bắt buộc, {count[RECOMMENDED]} khuyến nghị, "
        f"{count[OPTIONAL]} tùy chọn.",
        "",
        "Mã yêu cầu: `REQ-D-<mục>-<số>` lấy từ design.md, `REQ-I-<mục>-<số>` từ integration.md, "
        "`REQ-R-<mục>-<số>` từ roadmap.md, `REQ-T-<mục>-<số>` từ tech-stack.md. Mã có thể đổi khi tài liệu gốc thêm hoặc bớt câu.",
        "",
        "Mức: có PHẢI hoặc KHÔNG ĐƯỢC → bắt buộc; chỉ có NÊN → khuyến nghị; chỉ có CÓ THỂ → tùy chọn.",
        "",
        "| Mã | Mức | Yêu cầu | Nguồn |",
        "| --- | --- | --- | --- |",
    ]
    esc = lambda t: PIPE.sub(r"\\|", t)
    rows = [f"| {rid} | {lv} | {esc(text)} | [{name} §{sec}]({name}) |" for rid, lv, text, name, sec in reqs]
    return "\n".join(head + rows) + "\n"


def main():
    reqs = [r for prefix, name in SOURCES for r in extract(prefix, name)]
    ids = [r[0] for r in reqs]
    assert len(ids) == len(set(ids)), "mã yêu cầu bị trùng"
    assert all(r[2] for r in reqs), "có yêu cầu rỗng"
    text = render(reqs)
    target = DOCS / "requirements.md"
    strip_date = lambda t: re.sub(r"_Cập nhật: [\d-]+_", "", t)
    if "--check" in sys.argv:
        stale = not target.exists() or strip_date(target.read_text(encoding="utf-8")) != strip_date(text)
        print("requirements.md lỗi thời" if stale else "requirements.md đã khớp")
        sys.exit(1 if stale else 0)
    target.write_text(text, encoding="utf-8")
    print(f"{len(reqs)} yêu cầu → {target}")


if __name__ == "__main__":
    main()
