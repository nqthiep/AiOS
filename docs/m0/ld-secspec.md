# ld-secspec — Viết trước test bất biến Cedar và test vector token

| Thuộc tính | Giá trị |
| --- | --- |
| Người làm | leader (phần định nghĩa an ninh, không giao worker) |
| Phụ thuộc | ld-setup |
| Chặn | policy, confirm |
| Nguồn thiết kế | tech-stack.md §4 (tập Cedar, bất biến 1–6); design.md §9.3, §9.5, §18.3; REQ-T-4-01, REQ-D-9.5-01..05 |

## Mục tiêu

Định nghĩa "đúng" cho hai phần an ninh của lõi tin cậy dưới dạng file test và dữ liệu, trước khi worker viết mã. Worker của policy và confirm chỉ được viết mã cho qua các test này, không sửa chúng.

## Phạm vi

Được tạo: `policy/auto.cedar`, `policy/confirmable.cedar`, `policy/forbid.cedar`, `policy/aios.cedarschema`, `testdata/policy-invariants/cases.json`, `crates/aios-policy/tests/invariants.rs`, `testdata/confirm-vectors/vectors.json`, `testdata/confirm-vectors/gen.py`, `crates/aios-confirm/tests/vectors.rs`.

## Thiết kế

### Chính sách Cedar

Ba file `.cedar` lấy nguyên từ tech-stack.md §4 (gồm luật `system_scope` của design §18.3). Schema khai entity `Agent` (thuộc tính `uid: Long`, `is_admin: Bool`), `Tool` (thuộc tính `name: String`, `risk: String`, `reversible`, `always_confirm`, `system_scope`, `writes_trusted_core: Bool`), action `invoke`, context `autonomy: Long`, `channel: String`, `tainted: Bool`.

### Bất biến bằng duyệt toàn bộ không gian

Không gian đầu vào hữu hạn: risk (3) × reversible (2) × always_confirm (2) × system_scope (2) × writes_trusted_core (2) × autonomy 0–3 (4) × tainted (2) × channel (4) × is_admin (2) = 3.072 tổ hợp. `invariants.rs` duyệt hết và kiểm:

| # | Bất biến (tech-stack §4) | Kiểm |
| --- | --- | --- |
| 1 | `auto` không Allow tool `high` | Mọi tổ hợp risk=high → verdict ≠ Allow |
| 2 | `auto` không Allow khi tainted | tainted=true → verdict ≠ Allow |
| 3 | Không tập nào Allow `high` khi channel ∈ {remote, schedule} | → verdict = Deny |
| 4 | `auto` Allow ⇒ `confirmable` Allow | Không tổ hợp nào Allow mà confirmable Deny |
| 5 | `system_scope` và không admin ⇒ Deny | |
| 6 | `system_scope` ⇒ không bao giờ Allow (luôn Confirm hoặc Deny) | |
| bổ sung | `writes_trusted_core` ⇒ Deny; không đảo ngược được ⇒ không Allow (ma trận design §9.3); `always_confirm` ⇒ không Allow | |

`cases.json` thêm khoảng 20 trường hợp cụ thể có kết quả mong đợi (Allow, Confirm(Dialog), Confirm(DialogWithPin), Deny) để đọc dễ. Test gọi API công khai của `aios-policy` (xem [policy.md](policy.md) §Giao diện); leader viết test theo giao diện đó và đánh dấu `#[ignore]` cho tới khi policy xong. \[Unverified\] Chứng minh bằng `cedar-policy-symcc` (REQ-T-4-01) để sau M0, vì cần solver; duyệt toàn bộ không gian là đủ chặt cho không gian hữu hạn này.

### Test vector cho token xác nhận

`gen.py` (Python 3.14, gói `cryptography`, chuẩn hóa JCS viết tay theo RFC 8785 cho tập con JSON dùng ở đây) sinh `vectors.json` từ khóa Ed25519 có seed cố định:

- 5 vector hợp lệ: một bước, theo lô 3 bước, args có chuỗi tiếng Việt chưa chuẩn hóa NFC (NFD), args có số và mảng lồng, có `dry_run_hash`.
- 10 vector không hợp lệ, mỗi cái một lỗi: sai chữ ký, hết hạn, nonce đã dùng, `args_hash` lệch, `tool` lệch, `principal` lệch, `uid` lệch, token theo lô chứa bước `high`, bước theo lô sai thứ tự, định dạng token hỏng.

Mỗi vector ghi: `claims`, `token` mong đợi, `pending` (bản Broker giữ), `now_unix`, kết quả mong đợi (`ok` hoặc tên lỗi trong enum `ConfirmError` của [confirm.md](confirm.md)).

## Kiểm thử và nghiệm thu

```bash
uv run --no-project --with cryptography python -I testdata/confirm-vectors/gen.py --check
cargo test -p aios-policy --features spec-tests   # chỉ chạy được sau khi task policy xong
cargo test -p aios-confirm --features spec-tests  # chỉ chạy được sau khi task confirm xong
```

Hai test đặc tả nằm sau feature `spec-tests` để workspace vẫn biên dịch khi chưa có mã; sau khi policy và confirm xong, leader thêm hai lệnh này vào `make check`.

## Ngoài phạm vi

Mã của `aios-policy`, `aios-confirm`.
