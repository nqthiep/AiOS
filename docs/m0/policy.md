# policy — aios-policy (Policy Engine Cedar)

| Thuộc tính | Giá trị |
| --- | --- |
| Người làm | worker |
| Phụ thuộc | ld-setup, ld-secspec |
| Chặn | broker |
| Nguồn thiết kế | tech-stack.md §4; design.md §9.3, §11.2, §18.3; REQ-D-9.3-01, REQ-D-9.3-04, REQ-D-9.5-04 |

## Mục tiêu

Crate `aios-policy` nạp ba tập chính sách Cedar do ld-secspec viết, dựng context từ dữ liệu do Broker cung cấp, và trả `Verdict` (aios-types). Qua toàn bộ test bất biến của ld-secspec.

## Phạm vi

Được sửa: `crates/aios-policy/src/**`, `crates/aios-policy/Cargo.toml`, test đơn vị mới trong `src/`. **Không được sửa** `policy/*.cedar`, `policy/aios.cedarschema`, `crates/aios-policy/tests/invariants.rs`, `testdata/policy-invariants/**`. Thấy chính sách hoặc test có vẻ sai thì dừng với `NEED_ADVICE`.

## Thiết kế

```rust
pub struct PolicyEngine { auto: PolicySet, confirmable: PolicySet, schema: Schema }

pub struct PolicyInput<'a> {
    pub principal: &'a Principal,     // aios-types
    pub tool: &'a ToolSpec,           // từ registry
    pub user_autonomy: u8,            // 0..=3, cấu hình của người dùng (design §11.2)
    pub tainted: bool,                // taint của session
}

impl PolicyEngine {
    pub fn load(dir: &Path) -> Result<Self, PolicyError>;   // đọc auto.cedar + forbid.cedar thành tập auto; confirmable.cedar + forbid.cedar thành tập confirmable; validate theo schema
    pub fn evaluate(&self, input: &PolicyInput) -> Verdict;
    pub fn policy_version(&self) -> String;                  // SHA-256 hex của nội dung 4 file, dùng trong token (design §9.5)
}
pub fn effective_autonomy(user: u8, channel: Channel, tool_writes: bool) -> u8;
pub fn confirm_level(tool: &ToolSpec, tainted: bool) -> ConfirmLevel;
```

- `effective_autonomy` = `min(user, trần kênh)`: trần `remote` = 1, `schedule` = 1, `voice` = 1 nếu tool ghi (risk ≥ medium hoặc không đảo ngược), `local` = 3 (design §11.2).
- `evaluate`: auto cho Allow → `Allow`; ngược lại confirmable cho Allow → `Confirm(confirm_level(...))`; ngược lại `Deny(lý do)`, lý do là id của chính sách forbid quyết định nếu có, hoặc `"no-permit"`.
- `confirm_level`: `high` và không đảo ngược → `DialogWithPin`; `high` và tainted → `DialogWithPin`; còn lại `Dialog` (tech-stack §4).
- Mọi trường context lấy từ `PolicyInput`; không có tham số nào cho client tự đặt `user_confirmed` (REQ-D-9.5-04).
- Lỗi khi nạp chính sách (cú pháp, sai schema) → `load` trả lỗi; Broker không chạy khi không nạp được chính sách (fail closed).
- Dùng `cedar-policy` 4.x (đã đối chiếu chữ ký 4.14 ở review.md §5).

## Kiểm thử và nghiệm thu

```bash
cargo test -p aios-policy                       # test đơn vị của worker
cargo test -p aios-policy --test invariants     # test của ld-secspec, bỏ #[ignore] khi bắt đầu task
cargo clippy -p aios-policy -- -D warnings
```

Đạt khi test bất biến chạy hết 3.072 tổ hợp và mọi trường hợp trong `cases.json` cho đúng kết quả.

## Ngoài phạm vi

Chứng minh bằng symcc (sau M0), chính sách theo từng tool tùy người dùng (`per_tool_override`, M1).
