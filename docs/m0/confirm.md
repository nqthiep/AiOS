# confirm — aios-confirm (token xác nhận gắn hành động)

| Thuộc tính | Giá trị |
| --- | --- |
| Người làm | worker |
| Phụ thuộc | ld-setup, ld-secspec |
| Chặn | broker |
| Nguồn thiết kế | design.md §9.5, §7.7 (chuẩn hóa args, duyệt theo lô), §18.3; REQ-D-9.5-01..05, REQ-T-5-02 |

## Mục tiêu

Crate `aios-confirm` tạo yêu cầu xác nhận, ký token (phía trusted UI) và kiểm rồi tiêu thụ token (phía Broker), đúng định dạng và qua toàn bộ test vector của ld-secspec.

## Phạm vi

Được sửa: `crates/aios-confirm/src/**`, `Cargo.toml` của crate, binary `aios-dev-confirm` trong crate. **Không được sửa** `testdata/confirm-vectors/**`, `crates/aios-confirm/tests/vectors.rs`.

## Thiết kế

**Chuẩn hóa.** `normalize(args)`: đệ quy, mọi chuỗi (cả khóa) chuẩn hóa Unicode NFC (crate `unicode-normalization`). `args_hash = SHA-256(JCS(normalize(args)))`. JCS theo RFC 8785; số chỉ chấp nhận số nguyên trong khoảng an toàn ±2^53 và số thực hữu hạn, định dạng số thực theo ECMAScript (crate JCS có sẵn nếu đáp ứng; nếu không, viết module riêng và so với vector).

**Claims** (design §9.5, bản có `steps`):

```rust
pub struct SignedStep { pub tool: String, pub args_hash: String }   // hash hex chữ thường
pub struct ConfirmClaims {
    pub v: u8,                       // = 1
    pub request_id: String,          // UUIDv7
    pub principal: String,           // "uid:<uid>/session:<id>"
    pub uid: u32,
    pub task_step: String,
    pub steps: Vec<SignedStep>,      // 1 phần tử = một bước; nhiều = theo lô
    pub dry_run_hash: Option<String>,
    pub policy_version: String,
    pub nonce: String,               // 16 byte ngẫu nhiên, base64url không padding
    pub exp_unix: u64,
}
```

**Định dạng token:** `base64url(JCS(claims)) + "." + base64url(ed25519_sig(JCS(claims)))`, không padding.

**Giao diện:**

```rust
pub struct PendingConfirm { pub request_id: String, pub uid: u32, pub principal: String, pub task_step: String,
                            pub steps: Vec<(String, serde_json::Value, Risk)>, pub dry_run_hash: Option<String>,
                            pub policy_version: String, pub created_unix: u64 }
pub fn pending_for(principal: &Principal, task_step: &str, steps: Vec<(String, Value, Risk)>, policy_version: &str) -> Result<PendingConfirm, ConfirmError>;
pub fn sign(claims: &ConfirmClaims, key: &SigningKey) -> String;                      // phía trusted UI
pub struct Verifier { key: VerifyingKey, nonces: NonceStore }
impl Verifier {
    pub fn new(key: VerifyingKey) -> Self;                             // NonceStore trong bộ nhớ
    pub fn verify_and_consume(&self, token: &str, pending: &PendingConfirm, now_unix: u64) -> Result<VerifiedToken, ConfirmError>;
    pub fn consume_step(&self, vt: &mut VerifiedToken, tool: &str, args: &Value) -> Result<(), ConfirmError>; // theo lô: đúng phần tử kế tiếp
}
pub enum ConfirmError { BadFormat, BadSignature, Expired, NonceReused, RequestMismatch, PrincipalMismatch,
                        UidMismatch, ToolMismatch, ArgsMismatch, StepOutOfOrder, HighInBatch, PolicyVersionMismatch, TooLong }
```

- `pending_for` trả `HighInBatch` nếu `steps.len() > 1` và có bước `high` (REQ-D-9.5-03). `verify_and_consume` cũng trả `HighInBatch` khi `pending` có hơn 1 bước và có bước `high`.
- Thứ tự kiểm trong `verify_and_consume`: độ dài (`TooLong`) → định dạng (`BadFormat`) → chữ ký (`BadSignature`) → `HighInBatch` → `request_id` → `principal` → `uid` → `policy_version` → hạn (`Expired`) → từng bước (`ToolMismatch`, `ArgsMismatch`) → nonce (`NonceReused`, tiêu thụ ở bước cuối).
- `ConfirmClaims` derive `Serialize`, `Deserialize`; khóa trong JSON đúng tên trường.
- Hạn: một bước tối đa 60 s kể từ `created_unix`; theo lô tối đa 3.600 s (M0). Vượt → `Expired`.
- `NonceStore`: trong bộ nhớ, giữ nonce đến khi hết hạn, dọn định kỳ; tiêu thụ nonce ngay khi `verify_and_consume` thành công lần đầu.
- So sánh hash dùng so sánh thời gian hằng (crate `subtle`).
- Token dài quá 16 KiB → `TooLong` trước khi giải mã.

**Bộ ký dev (chỉ M0):** binary `aios-dev-confirm` đọc `PendingConfirm` (JSON) từ `aiosd-core`, in nội dung đầy đủ (tool, args đã chuẩn hóa, ký tự điều khiển và bidi dạng mã thoát, design §7.7), hỏi `y/N` trên TTY, rồi ký bằng khóa `/etc/aios/dev-trusted-ui.key` và gửi token lại. Từ chối chạy khi stdin không phải TTY. M1 thay bằng `aios-trusted-ui`.

Khóa dev thuộc user hệ thống riêng `aios-trusted` (quyền `0600`), để `aiosd-cognition` chạy dưới user người dùng không đọc được khóa; `aiosctl confirm` gọi `sudo -u aios-trusted aios-dev-confirm <request_id>`. \[Inference\] Trong VM của Lima, user mặc định có sudo không cần mật khẩu, nên ở M0 cơ chế này chỉ tách quyền, không chứng minh người dùng có mặt; cổng M0 không phải cổng an ninh cho bước xác nhận.

## Kiểm thử và nghiệm thu

```bash
cargo test -p aios-confirm
cargo test -p aios-confirm --features spec-tests  # test của ld-secspec; mọi vector cho đúng kết quả
cargo clippy -p aios-confirm -- -D warnings
```

Thêm test đơn vị: token đúng dùng lần 2 → `NonceReused`; đổi 1 ký tự args sau khi ký → `ArgsMismatch`; args NFD và NFC cho cùng hash.

## Ngoài phạm vi

PIN (M1, cần bề mặt tin cậy), lưu nonce bền qua khởi động lại (M1).
