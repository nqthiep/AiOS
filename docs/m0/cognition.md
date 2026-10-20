# cognition — aiosd-cognition tối thiểu và aiosctl

| Thuộc tính | Giá trị |
| --- | --- |
| Người làm | worker |
| Phụ thuộc | broker, gateway, tools3 |
| Chặn | s16, ld-gate |
| Nguồn thiết kế | roadmap.md §2.3 (router 2 cấp, vòng L2); design.md §4.4, §11.3, §7.10 (aiosctl chạy không cần mô hình); integration.md §3.1 |

## Mục tiêu

Gói Python `aiosd_cognition` có lệnh `aiosctl` để chạy cổng M0: `aiosctl ask "<câu>"` đi qua grammar Reflex hoặc vòng L2, gọi tool qua Broker, gọi mô hình qua Gateway.

## Phạm vi

Được sửa: `python/aiosd-cognition/**`.

## Thiết kế

### Lệnh

| Lệnh | Hành vi |
| --- | --- |
| `aiosctl ask "<câu>" [--json]` | Tạo session qua `/run/aios/core.sock`, chạy router, in kết quả và các bước |
| `aiosctl confirm [--list]` | Liệt kê yêu cầu chờ xác nhận của người dùng; chọn một để mở `aios-dev-confirm` (bộ ký dev) |
| `aiosctl status` | Gọi `GET /v1/status` của `aiosd-core` và `/v1/models` của Gateway; chạy được khi Gateway hoặc mô hình tắt (báo "không sẵn sàng") |
| `aiosctl audit verify` | Chạy `aios-audit verify` trên file audit |
| `aiosctl freeze` / `unfreeze` | Ghi `1`/`0` vào `cgroup.freeze` của `aios-agent.slice` của người dùng, rồi gọi Gateway `revoke-all` qua `aiosd-core` nếu còn sống; freeze không phụ thuộc `aiosd-core` (REQ-D-11.3-01). M0: `unfreeze` hỏi lại `y/N` trên TTY (bề mặt tin cậy có ở M1) |

### Router 2 cấp (roadmap §2.3)

1. **L0 grammar.** Bảng regex có phiên bản trong `aiosd_cognition/reflex/vi.yaml` (design §17: tài nguyên ngôn ngữ tách khỏi mã). M0 có 2 lệnh: `^(trạng thái|tình trạng) (máy|hệ thống)$` → `sys.info`; `^tìm (file|tệp) (?P<q>.+)$` → `fs.search {q}`. So khớp sau khi chuẩn hóa NFC, chữ thường.
2. **L1 tập đóng** chưa có ở M0 (cần mô hình System 1 đã chọn); mọi câu không khớp L0 lên L2.
3. **L2 vòng lặp** (design §4.4, bản tối giản): gọi `s2` qua Gateway với danh sách tool lấy từ `tools/list` của Broker (định dạng `tools` của OpenAI); tối đa 8 vòng tool call; mỗi tool call mang `idempotency_key = "<session>:<vòng>:<chỉ số>"`; kết quả tool đưa lại cho mô hình trong vai `tool`, có tiền tố nguồn khi `provenance=external` (`[external] ...`); dừng khi mô hình trả lời không có tool call, hết vòng, hoặc hết ngân sách token (mặc định 20.000 token mỗi task).
4. Mọi lỗi của Broker (`denied`, `confirmation_*`) đưa lại cho mô hình như kết quả tool lỗi; không thử lại cùng tool với cùng args quá 1 lần.

### Kết nối

- Session: `POST /v1/session` qua Unix socket (`httpx` với transport UDS) → `broker_token`, `gateway_key`.
- Broker: client MCP streamable HTTP của SDK `mcp`, header Bearer.
- Gateway: HTTP OpenAI-compatible (`httpx`), không dùng SDK của provider.

## Kiểm thử và nghiệm thu

Test đơn vị với Broker giả và Gateway giả (fixture pytest):

| Test | Mong đợi |
| --- | --- |
| "trạng thái máy" | Đi L0, gọi `sys.info`, không gọi Gateway |
| "Tìm file hợp đồng" (hoa, có dấu) | Đi L0, `fs.search {q:"hợp đồng"}` |
| "ghi chú: mai họp 9h" | Đi L2; mô hình giả trả tool call `fs.write_note`; Broker giả trả `waiting/confirm` rồi ok |
| Mô hình giả lặp tool call 20 lần | Dừng ở 8 vòng, báo lỗi rõ |
| Gateway tắt | `aiosctl status` vẫn chạy, báo Gateway không sẵn sàng; `ask` với câu L0 vẫn chạy |
| `freeze` khi `aiosd-core` tắt | Vẫn ghi được `cgroup.freeze` |

```bash
uv run --directory python/aiosd-cognition pytest
uv run --directory python/aiosd-cognition ruff check .
```

## Ngoài phạm vi

L1 với mô hình System 1 (M1), `aiosd-memory` (M1), app chat (M1), giọng nói (M2).
