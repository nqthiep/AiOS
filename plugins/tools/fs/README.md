# Plugin `aios.fs`

Plugin MCP (stdio) của AiOS, cung cấp 3 tool cho cổng M0: `fs.search`, `fs.write_note`, `fs.undo_note`.
Thiết kế: [docs/m0/tools3.md](../../../docs/m0/tools3.md), lược đồ manifest: [docs/m0/registry.md](../../../docs/m0/registry.md).

## Chạy

```bash
uv sync --directory plugins/tools/fs --frozen --no-dev --python /usr/bin/python3
plugins/tools/fs/.venv/bin/python -I plugins/tools/fs/server.py
```

Dòng thứ hai chính là `entry` trong `plugin.toml` (Executor thay `$PLUGIN_DIR` bằng thư mục của plugin). Venv phải dựng sẵn lúc cài: trong sandbox, `uv run` không ghi được cache của uv.

## Biến môi trường

| Biến | Bắt buộc | Ý nghĩa |
| --- | --- | --- |
| `AIOS_WORKSPACE` | có | Thư mục làm việc. Mọi đường dẫn được chuẩn hóa bằng `os.path.realpath` và so với thư mục này bằng `os.path.commonpath`; symlink trỏ ra ngoài bị bỏ qua hoặc từ chối. Thiếu biến này thì tool báo lỗi rõ ràng. |

## Cấu trúc

| Đường dẫn | Vai trò |
| --- | --- |
| `plugin.toml` | Manifest: tool, mức rủi ro, `rollback_tool`, capability, sandbox, `result_privacy`/`result_provenance`. Chưa ký (`.sig` do leader tạo lúc tích hợp). |
| `server.py` | Điểm vào, chạy MCP server qua stdio. |
| `src/aios_fs/search.py` | `fs.search`: glob không phân biệt hoa thường và dấu tiếng Việt, tối đa 50 kết quả. |
| `src/aios_fs/notes.py` | `fs.write_note` và `fs.undo_note`: ghi `$WORKSPACE/notes/<name>.md`, giữ bản cũ ở `$WORKSPACE/.aios-undo/<undo_id>`. |
| `src/aios_fs/workspace.py` | Tìm workspace và kiểm tra không thoát khỏi workspace. |
| `tests/` | Test pytest, gồm test gọi tool qua MCP stdio thật bằng client của SDK `mcp`. |

## Kiểm tra

```bash
uv run --directory plugins/tools/fs pytest -q
uv run --directory plugins/tools/fs ruff check .
```
