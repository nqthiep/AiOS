# ld-setup — Dựng workspace, quy ước và công cụ kiểm tra

| Thuộc tính | Giá trị |
| --- | --- |
| Người làm | leader |
| Phụ thuộc | — |
| Chặn | ld-secspec, audit, registry, sandbox, policy, confirm, gateway, tools3, image |
| Nguồn thiết kế | [README.md](README.md) §2–§6; roadmap.md §6 |

## Mục tiêu

Tạo khung repo để mọi task sau chỉ cần viết mã trong thư mục của mình: Cargo workspace với đủ crate rỗng, project Python, crate `aios-types` hoàn chỉnh, `Makefile`, `dev/vm-run.sh`, `AGENTS.md`.

## Phạm vi

Được tạo: mọi file ở cấp gốc (`Cargo.toml`, `rust-toolchain.toml`, `Makefile`, `AGENTS.md`, `.gitignore`), `crates/*/Cargo.toml` và `src/lib.rs` hoặc `src/main.rs` rỗng, `crates/aios-types/` đầy đủ, `python/aiosd-cognition/pyproject.toml`, `dev/vm-run.sh`.

## Thiết kế

1. **Rust.** Cài toolchain stable trên Mac (`rustup default stable`), ghim phiên bản cụ thể trong `rust-toolchain.toml` (thành phần `rustfmt`, `clippy`). Workspace resolver 3, `members = ["crates/*"]`.
2. **Crate chỉ-Linux.** `aios-sandbox-exec` khai `[target.'cfg(target_os = "linux")'.dependencies]` cho `landlock`, `seccompiler`; trên Mac, `main.rs` biên dịch thành chương trình báo lỗi "linux only" để `cargo test --workspace` vẫn chạy được.
3. **`aios-types`.** Đúng như README.md §3, có `serde`, test round-trip JSON cho mọi enum (giá trị chữ thường) và test thứ tự `Privacy`, `Risk`.
4. **Python.** `python/aiosd-cognition` là uv project, Python `>=3.14`, gói `aiosd_cognition`, entry point `aiosctl = "aiosd_cognition.cli:main"`; dev dependency `pytest`, `ruff`.
5. **`dev/vm-run.sh <lệnh>`.** Đóng gói worktree hiện tại bằng `git ls-files -co --exclude-standard | tar -cf - -T -`, giải nén vào `~/build/$(basename $PWD)` trong VM qua `limactl shell aios-dev`, rồi chạy lệnh ở đó. Thoát với mã thoát của lệnh.
6. **`Makefile`.** Mục tiêu `check`, `vm-test`, `fmt`, như README.md §6.
7. **`AGENTS.md`.** Tóm tắt README.md §5–§7, cộng: đọc `docs/m0/README.md` và file task trước khi làm; không sửa ngoài phạm vi; không `git add -A`.

## Kiểm thử và nghiệm thu

```bash
make check
dev/vm-run.sh 'cargo test --workspace'
```

Cả hai đạt trên workspace rỗng (chỉ test của `aios-types` chạy).

## Ngoài phạm vi

CI trên dịch vụ ngoài (chưa có remote); mã nghiệp vụ của các crate khác.
