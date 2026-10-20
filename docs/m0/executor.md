# executor — Executor thật qua aios-sandbox-exec và cgroup

| Thuộc tính | Giá trị |
| --- | --- |
| Người làm | worker |
| Phụ thuộc | broker, sandbox |
| Chặn | ld-gate |
| Nguồn thiết kế | design.md §9 (đường thực thi theo từng lần gọi), §9.4, §18.4; tech-stack.md §8 |

## Mục tiêu

`SandboxExecutor` (trong `crates/aios-broker/src/executor/`) hiện thực trait `Executor`: chạy MCP server của plugin như một tiến trình ngắn trong cgroup tạm dưới `aios-agent.slice` của người dùng, bọc bởi `aios-sandbox-exec`, rồi gọi đúng một `tools/call` qua stdio.

## Phạm vi

Được sửa: `crates/aios-broker/src/executor/**`, test trong `crates/aios-broker/tests/executor_*.rs`, đăng ký executor trong `crates/aiosd-core/src/main.rs` (chỉ dòng chọn executor).

## Thiết kế

1. Ghi `ToolSpec.sandbox` ra file JSON tạm (quyền `0600`), thay `$WORKSPACE` bằng `ExecCtx.workspace`.
2. Chạy:
   ```text
   systemd-run --user --scope --quiet --collect --slice=aios-agent.slice \
     --unit=aios-tool-<uuid> -p MemoryMax=<memory_max_mb>M -p TasksMax=64 \
     -- aios-sandbox-exec --profile <file> -- <entry...>
   ```
   M0 chạy cho chính người dùng của `aiosd-core` (systemd `--user`). \[Unverified\] Chạy cho UID khác từ dịch vụ hệ thống (design §18) để sau M0.
3. Nói MCP qua stdin/stdout của tiến trình con bằng client stdio của `rmcp`: `initialize` → `tools/call` → đọc kết quả → đóng stdin; tiến trình con phải thoát trong 2 s sau đó, không thì gửi SIGTERM rồi SIGKILL.
4. Hết `timeout_ms` → hủy, `systemctl --user kill aios-tool-<uuid>`, trả `ExecError::Timeout`.
5. Kết quả: `ToolOutput { content, is_error, privacy: max(spec.result_privacy, _meta.privacy nếu tool tự nâng), provenance: spec.result_provenance hoặc External nếu tool báo }`. Tool không được hạ nhãn: `_meta.privacy` thấp hơn bị bỏ qua.
6. stderr của tiến trình con ghi log (tối đa 64 KiB), không gửi cho client.
7. Mã thoát 64–79 của sandbox → `ExecError::Sandbox(code)` với thông điệp rõ ("Landlock không áp được đầy đủ" cho 70).

## Kiểm thử và nghiệm thu

Chạy trong VM với một plugin test (MCP server Python tối giản có 3 tool: echo, ghi file vào workspace, ngủ quá timeout):

| Test | Mong đợi |
| --- | --- |
| echo | Kết quả đúng; tiến trình nằm dưới `aios-agent.slice` (kiểm `/proc/<pid>/cgroup` trong tool) |
| Ghi file trong workspace | Có file |
| Ghi file ngoài workspace | Lỗi từ tool, file không được tạo |
| Ngủ quá timeout | `Timeout`; không còn tiến trình `aios-tool-*` sau 3 s |
| Tool tự báo `privacy=public` khi spec là `personal` | Kết quả mang `personal` |

```bash
dev/vm-run.sh 'cargo test -p aios-broker --test executor_vm -- --test-threads=1'
cargo test -p aios-broker          # trên Mac: test executor_vm tự bỏ qua
```

## Ngoài phạm vi

MCP server chạy lâu và OpenShell (M4), đăng ký cgroup vào map BPF (M2).
