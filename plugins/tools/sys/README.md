# Plugin `aios.sys`

Plugin MCP (stdio) của AiOS, cung cấp tool `sys.info` cho cổng M0.
Thiết kế: [docs/m0/tools3.md](../../../docs/m0/tools3.md), lược đồ manifest: [docs/m0/registry.md](../../../docs/m0/registry.md).

## Chạy

```bash
uv run --directory plugins/tools/sys python -I server.py
```

Đây chính là `entry` trong `plugin.toml` (registry thay `$PLUGIN_DIR` bằng thư mục của plugin).

## Biến môi trường

| Biến | Bắt buộc | Ý nghĩa |
| --- | --- | --- |
| `AIOS_WORKSPACE` | có | Thư mục làm việc; `sys.info` dùng nó để báo dung lượng ổ đĩa của workspace. Thiếu biến này thì tool báo lỗi rõ ràng. |

## Cấu trúc

| Đường dẫn | Vai trò |
| --- | --- |
| `plugin.toml` | Manifest: tool, mức rủi ro, capability, sandbox, `result_privacy`/`result_provenance`. Chưa ký (`.sig` do leader tạo lúc tích hợp). |
| `server.py` | Điểm vào, chạy MCP server qua stdio. |
| `src/aios_sys/info.py` | `sys.info`: hostname, kernel, uptime, RAM, tải CPU, dung lượng `/` và workspace. Chỉ đọc `/proc`, `/sys` và `os.statvfs`. |
| `src/aios_sys/workspace.py` | Tìm workspace qua `AIOS_WORKSPACE`. |
| `tests/` | Test pytest, gồm test gọi tool qua MCP stdio thật bằng client của SDK `mcp`. |

## Kiểm tra

```bash
uv run --directory plugins/tools/sys pytest -q
uv run --directory plugins/tools/sys ruff check .
```

Test cần Linux (đọc `/proc`), chạy trong VM `aios-dev`:

```bash
dev/vm-run.sh 'uv run --directory plugins/tools/sys pytest -q'
```
