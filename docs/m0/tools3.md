# tools3 — Ba tool cho cổng M0

| Thuộc tính | Giá trị |
| --- | --- |
| Người làm | worker |
| Phụ thuộc | ld-setup |
| Chặn | cognition |
| Nguồn thiết kế | design.md §9.1, §9.2, §15.2; roadmap.md §2.1; [registry.md](registry.md) (lược đồ, ký) |

## Mục tiêu

Hai plugin MCP stdio viết bằng Python, mỗi plugin có `plugin.toml` đã ký, đủ để chạy cổng M0: một tool đọc thông tin, một tool tìm kiếm, một tool ghi có hoàn tác.

## Phạm vi

Được sửa: `plugins/tools/sys/**`, `plugins/tools/fs/**`.

## Thiết kế

| Plugin | Tool | Rủi ro | Đảo ngược | Hành vi |
| --- | --- | --- | --- | --- |
| `aios.sys` | `sys.info` | low | có | Trả hostname, kernel, uptime, RAM tổng và trống, tải CPU, dung lượng `/` và workspace. Chỉ đọc `/proc`, `/sys`, `os.statvfs` |
| `aios.fs` | `fs.search` | low | có | Tìm file theo tên (glob không phân biệt hoa thường, bỏ dấu tiếng Việt khi so) trong `$WORKSPACE`; tối đa 50 kết quả; trả đường dẫn tương đối, kích thước, thời gian sửa |
| `aios.fs` | `fs.write_note` | medium | có | Ghi file `$WORKSPACE/notes/<name>.md`; nếu file đã tồn tại thì giữ bản cũ ở `$WORKSPACE/.aios-undo/<id>`; trả `undo_id` |
| `aios.fs` | `fs.undo_note` | low | có | Hoàn tác theo `undo_id`: khôi phục bản cũ hoặc xóa file mới |

- `fs.write_note` khai `rollback_tool = "fs.undo_note"`. `name` theo mẫu `^[\p{L}\p{N} _-]{1,80}$`, không có `/` hay `..`; nội dung tối đa 64 KiB.
- `capabilities`: `aios.sys` → `["fs.read:/proc", "fs.read:/sys"]`; `aios.fs` → `["fs.read:$WORKSPACE", "fs.write:$WORKSPACE"]`.
- `sandbox`: `ro_paths` / `rw_paths` tương ứng; `tcp_connect = []`; `landlock_required = true`.
- `result_privacy = "personal"`; `result_provenance`: `observed` cho `sys.info`; `external` cho `fs.search` (tên file do bên ngoài đặt, design §6.3).
- Dùng SDK Python `mcp` (FastMCP) qua stdio; Python ≥ 3.14; mỗi plugin là một uv project nhỏ, entry trong `plugin.toml` là `["/usr/bin/env", "python3", "-I", "<đường dẫn>/server.py"]` \[Inference\] trong M0 cài phụ thuộc vào venv của plugin, entry trỏ vào python của venv đó.
- Ký `plugin.toml` bằng `aios-plugin-sign` với khóa dev; khóa dev không commit (sinh lúc test).

## Kiểm thử và nghiệm thu

```bash
uv run --directory plugins/tools/fs pytest
uv run --directory plugins/tools/sys pytest
cargo run -p aios-registry --example load -- --core plugins/tools   # registry nạp được 4 tool, đúng mức rủi ro
```

Test bắt buộc: `fs.write_note` với `name="../x"` bị từ chối; ghi đè rồi `fs.undo_note` khôi phục đúng nội dung; `fs.search` không trả file ngoài workspace kể cả qua symlink; `sys.info` không đọc file ngoài `/proc`, `/sys`.

## Ngoài phạm vi

Khoảng 20 tool của M1 (Wi-Fi, Bluetooth, âm thanh, Flatpak...).
