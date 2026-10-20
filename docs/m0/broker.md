# broker — aios-broker và binary aiosd-core

| Thuộc tính | Giá trị |
| --- | --- |
| Người làm | worker |
| Phụ thuộc | policy, confirm, audit, registry |
| Chặn | executor, cognition |
| Nguồn thiết kế | design.md §9.4, §9.5, §5.2 (nhãn theo session), §6.3 (taint), §18.1; integration.md §3.2; REQ-D-9.3-01..04, REQ-D-9.5-01..05, REQ-D-18.1-01, REQ-T-5-01 |

## Mục tiêu

Crate `aios-broker` là cửa duy nhất tới tool: xuất MCP endpoint `aios-tools`, với mỗi lần gọi thì tra registry, kiểm schema, hỏi Policy Engine, xin xác nhận khi cần, chạy tool qua một `Executor`, nâng nhãn và taint của session, ghi audit. Binary `aiosd-core` ghép Broker với socket quản trị.

## Phạm vi

Được sửa: `crates/aios-broker/**`, `crates/aiosd-core/**`. Không sửa crate khác; cần đổi API của crate khác thì `NEED_ADVICE`.

## Thiết kế

### Session

- Tạo qua socket quản trị `/run/aios/core.sock` (HTTP/1.1 trên Unix socket, `hyper` + `hyperlocal` hoặc tương đương). UID lấy bằng `SO_PEERCRED`; `is_admin` = UID thuộc nhóm `wheel`.
- `POST /v1/session` `{"channel":"local"}` → `{"session_id","broker_token","gateway_key","broker_url":"http://127.0.0.1:8091/mcp","gateway_url":"http://127.0.0.1:8090/v1"}`.
  - `broker_token`: 32 byte ngẫu nhiên, base64url; Broker giữ map token → `Principal` + `SessionState`.
  - `gateway_key`: Broker gọi API quản trị của Gateway ([gateway.md](gateway.md) §Quản trị) `POST /v1/sessions` để lấy.
  - Session hết hạn sau 8 giờ không dùng.
- `GET /v1/pending` → danh sách `PendingConfirm` của chính UID gọi. `POST /v1/confirm` `{"request_id","token"}`, `POST /v1/deny` `{"request_id"}`. `GET /v1/status` → phiên bản, số tool, `LoadReport` của registry, `policy_version`.

### MCP endpoint

- MCP streamable HTTP ở `127.0.0.1:8091/mcp` bằng `rmcp` (bản 3.5.x, đã kiểm ở review.md §5). Header `Authorization: Bearer <broker_token>` bắt buộc; sai hoặc thiếu → HTTP 401.
- `tools/list`: mọi tool trong registry, với `inputSchema` và mô tả; tool `system_scope` vẫn được liệt kê cho người không quản trị nhưng gọi sẽ bị Deny.
- `tools/call` phải có `_meta.idempotency_key` (chuỗi 1–128 ký tự); thiếu → lỗi MCP `invalid_params`.

### Luồng một lần gọi (design §9.4)

1. Tra `ToolSpec` trong registry; không có → lỗi `unknown_tool`.
2. Kiểm args theo `input_schema` (crate `jsonschema`); sai → `invalid_params`, không chạy.
3. Idempotency: cùng session và cùng key đã có kết quả → trả lại kết quả cũ, không chạy lại, không ghi audit lần 2 (ghi một dòng `kind=tool_call` với `"replay":true`). Cache giữ 1 giờ.
4. `PolicyEngine::evaluate` với `user_autonomy` (M0: đọc từ `/etc/aios/autonomy.yaml`, mặc định 0), `tainted` của session.
5. `Deny` → lỗi MCP `denied` kèm lý do, ghi audit.
6. `Confirm` → tạo `PendingConfirm` (Broker giữ bản args, REQ-D-9.5-01), chờ tối đa 120 s một token gửi qua `/v1/confirm`; `Verifier::verify_and_consume` với chính args Broker sẽ chạy; sai → lỗi `confirmation_invalid`; hết giờ → `confirmation_timeout`; `/v1/deny` → `confirmation_denied`. Mọi trường hợp ghi audit `kind=confirm`.
7. Chạy qua `Executor::run`.
8. Nâng session: `label = max(label, output.privacy)`; `taint = Tainted` nếu `output.provenance == External`. Nếu đổi thì gọi Gateway `POST /v1/sessions/{id}/raise` trước khi trả kết quả cho client (design §5.2). Gọi Gateway lỗi → trả lỗi `label_sync_failed` và không trả nội dung tool (fail closed).
9. Ghi audit `kind=tool_call` (REQ-D-9.3-03): tool, args (đã redact nếu label `secret`), level, verdict, reason, result (`ok`/`error`), approver (`trusted_ui`/null), session, idempotency_key.
10. Trả `ToolOutput.content` cho client, kèm `_meta.privacy`, `_meta.provenance`.

### Executor

```rust
#[async_trait]
pub trait Executor: Send + Sync {
    async fn run(&self, tool: &ToolSpec, args: &Value, ctx: &ExecCtx) -> Result<ToolOutput, ExecError>;
}
pub struct ExecCtx { pub principal: Principal, pub session_id: String, pub workspace: PathBuf, pub timeout: Duration }
```

Task này viết trait và `FakeExecutor` (trả kết quả cấu hình sẵn) để test. Executor thật là task [executor](executor.md).

### Cấu hình `aiosd-core`

`/etc/aios/core.yaml`: đường dẫn registry, policy dir, audit file, khóa công khai của trusted UI (`/etc/aios/dev-trusted-ui.pub` ở M0), socket Gateway quản trị, cổng MCP. Mọi đường dẫn ghi đè được bằng cờ dòng lệnh để test.

## Kiểm thử và nghiệm thu

Test tích hợp với `FakeExecutor` và Gateway giả (HTTP server trong test):

| Test | Mong đợi |
| --- | --- |
| Gọi không có Bearer | 401 |
| Tool low, autonomy 1 | Chạy, audit 1 dòng |
| Tool low, autonomy 0 | Confirm; gửi token hợp lệ → chạy |
| Gửi token ký cho args khác | `confirmation_invalid`, không chạy |
| Dùng lại token | `confirmation_invalid` |
| Cùng idempotency key 2 lần | Executor chỉ được gọi 1 lần |
| Tool trả `privacy=secret` | Gateway giả nhận `raise` với `secret` trước khi client nhận kết quả |
| Gateway giả trả lỗi khi raise | Client nhận `label_sync_failed`, không có nội dung |
| Tool trả `provenance=external`, gọi tool low tiếp theo ở autonomy 3 | Lần 2 là Confirm (session tainted) |
| UID A dùng token session của UID B qua `/v1/confirm` | Từ chối |
| `audit verify` sau toàn bộ test | ok |

```bash
cargo test -p aios-broker
cargo test -p aiosd-core
cargo clippy -p aios-broker -p aiosd-core -- -D warnings
```

## Ngoài phạm vi

REST `/v1/preflight` cho hook `aios-guard` (M4), nạp lại registry lúc chạy (M1), bộ nhớ (task hindsight).
