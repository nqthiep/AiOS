# audit — Audit log append-only có hash chain

| Thuộc tính | Giá trị |
| --- | --- |
| Người làm | worker |
| Phụ thuộc | ld-setup |
| Chặn | broker |
| Nguồn thiết kế | design.md §9.3, §11.4, §18.5; REQ-D-9.3-03, REQ-D-15.7-02 |

## Mục tiêu

Crate `aios-audit` ghi mọi sự kiện cần kiểm toán vào file JSONL chỉ thêm, mỗi dòng nối hash với dòng trước, và có lệnh kiểm tra phát hiện mọi sửa đổi.

## Phạm vi

Được sửa: `crates/aios-audit/**`. Không sửa crate khác.

## Thiết kế

**Định dạng một dòng** (JSON trên một dòng, kết thúc bằng `\n`):

```json
{"seq":42,"ts":"2026-10-20T06:30:14.123Z","uid":1000,"kind":"tool_call",
 "data":{"tool":"fs.search","args":{"q":"hop dong"},"level":2,"verdict":"allow","reason":"auto",
         "result":"ok","approver":null,"session":"0192...","idempotency_key":"t1:0"},
 "prev":"<64 hex>","hash":"<64 hex>"}
```

- `hash = SHA-256(prev_hash_bytes || JCS(dòng không có trường "hash"))`, viết hex chữ thường. `prev` của dòng đầu là 64 chữ số `0`.
- JCS: chuẩn hóa JSON theo RFC 8785 (khóa sắp theo UTF-16, không khoảng trắng). Dùng chung hàm JCS với `aios-confirm` nếu crate đó có trước; nếu chưa, viết module `jcs` riêng trong crate này (chỉ cần đối tượng, mảng, chuỗi, số nguyên, bool, null).
- `kind` thuộc tập đóng: `tool_call`, `confirm`, `plugin_load`, `plugin_unload`, `capability_change`, `session_create`, `freeze`, `unfreeze`.
- `data` không được chứa giá trị nhãn `secret`: người gọi truyền vào bản đã thay bằng `"<secret:sha256:...>"`. Crate cung cấp hàm `redact_secret(value, paths)`.

**Giao diện:**

```rust
pub struct AuditLog { /* file, seq kế tiếp, hash cuối */ }
impl AuditLog {
    pub fn open(path: &Path) -> Result<Self, AuditError>;          // đọc dòng cuối để tiếp chuỗi; kiểm dòng cuối hợp lệ
    pub fn append(&self, uid: u32, kind: AuditKind, data: serde_json::Value) -> Result<u64, AuditError>; // trả seq
}
pub fn verify(path: &Path) -> Result<VerifyReport, AuditError>;   // dòng hỏng đầu tiên (seq, lý do) hoặc ok
```

- Ghi bằng `O_APPEND`, mỗi `append` một lần `write` cho cả dòng, rồi `fsync` (cấu hình được, mặc định bật).
- `AuditLog` an toàn khi dùng từ nhiều task tokio (khóa bên trong); chỉ một tiến trình ghi (Broker).
- File mới tạo với quyền `0600`.
- Không xoay vòng trong M0 (design §12.5: audit log không bị xoay vòng tự động).

**Binary:** `aios-audit verify <file>` in `ok N dòng` và thoát 0, hoặc in `hỏng ở seq=K: <lý do>` và thoát 1.

## Kiểm thử và nghiệm thu

Test bắt buộc: ghi 1.000 dòng rồi verify ok; sửa 1 byte ở dòng giữa → verify báo đúng seq; xóa 1 dòng → báo; đổi thứ tự 2 dòng → báo; mở lại file đã có và ghi tiếp → chuỗi liền; ghi đồng thời từ 8 task → không dòng nào xen lẫn.

```bash
cargo test -p aios-audit
cargo clippy -p aios-audit -- -D warnings
cargo run -p aios-audit -- verify /tmp/x.jsonl   # trên file do test sinh ra
```

## Ngoài phạm vi

Ký hash chain bằng khóa (sau M0), xem audit theo người dùng trên UI (M1).
