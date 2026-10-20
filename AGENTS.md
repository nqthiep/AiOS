# Quy tắc cho coding agent làm việc trên AiOS

Đọc trước khi sửa bất cứ gì:

1. [HANDOFF.md](HANDOFF.md): trạng thái dự án.
2. [docs/m0/README.md](docs/m0/README.md): quy ước chung, cấu trúc repo, kiểu dùng chung `aios-types`, cổng và socket, lệnh kiểm tra, giao thức báo cáo. File này thắng khi mâu thuẫn với file task.
3. File thiết kế của task bạn được giao: `docs/m0/<id>.md`, và `.tasks/<id>/task.md` nếu có.

## Bắt buộc

- Chỉ sửa file trong mục "Phạm vi" của task. Không sửa `docs/`, `policy/`, `testdata/`, `crates/aios-types/` trừ khi task cho phép. Cần đổi `aios-types` hay API của crate khác thì dừng với `NEED_ADVICE`.
- Không dùng `git add -A`; không commit file sinh ra (`target/`, `.venv/`, `__pycache__/`, `.omo/`).
- Rust: edition 2024, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`; lỗi bằng `thiserror` trong thư viện, `anyhow` trong binary; `tokio`; không `unsafe` (trừ `aios-sandbox-exec`, mỗi khối có comment lý do).
- Python: 3.14, `uv`, `ruff`, `pytest`, type hint ở hàm công khai.
- Phụ thuộc mới chỉ dùng giấy phép tương thích Apache-2.0 (MIT, Apache-2.0, BSD, ISC, Unicode, Zlib, MPL-2.0); ghi vào "Quyết định tự đưa ra".
- Không log nội dung nhãn `secret`; không viết cứng đường dẫn hay cổng trong logic (đọc từ cấu hình hoặc biến môi trường).
- Mã, comment, thông báo lỗi trong mã bằng tiếng Anh; báo cáo bằng tiếng Việt.
- Test cần Linux chạy qua `dev/vm-run.sh '<lệnh>'` (VM `aios-dev`), không chạy trực tiếp trên Mac.

## Báo cáo

- Không rõ hoặc mâu thuẫn, nhất là ở chỗ liên quan an ninh (policy, token, nhãn, sandbox, registry): dừng, dòng đầu `NEED_ADVICE`, nêu câu hỏi và phương án. Không tự đoán.
- Xong: dòng đầu `DONE`, kèm danh sách file đã sửa, output của các lệnh trong mục "Nghiệm thu", và mục "Quyết định tự đưa ra".
