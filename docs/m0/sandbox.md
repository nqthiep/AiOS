# sandbox — aios-sandbox-exec

| Thuộc tính | Giá trị |
| --- | --- |
| Người làm | worker |
| Phụ thuộc | ld-setup, ld-vm |
| Chặn | executor, s14 |
| Nguồn thiết kế | tech-stack.md §8; design.md §9 (đường thực thi theo từng lần gọi); REQ-T-8-01 |

## Mục tiêu

Binary Linux `aios-sandbox-exec` chạy một lệnh trong sandbox theo `SandboxProfile` (aios-types): namespace bằng bubblewrap, rồi Landlock và seccomp áp trong một tiến trình đơn luồng ngay trước `execve`.

## Phạm vi

Được sửa: `crates/aios-sandbox-exec/**`. Chỉ build thật trên Linux (README.md §2 về crate chỉ-Linux).

## Thiết kế

**Dòng lệnh:**

```text
aios-sandbox-exec --profile <file.json> [--stage2] -- <cmd> [args...]
```

1. **Giai đoạn 1** (không có `--stage2`): đọc profile, dựng lệnh `bwrap` rồi `execve` nó:
   - `--unshare-all --share-net` chỉ khi `tcp_connect` không rỗng, ngược lại `--unshare-net`;
   - `--die-with-parent --new-session --cap-drop ALL`;
   - `--ro-bind /usr /usr`, symlink `/bin`, `/lib`, `/lib64`, `/sbin` theo Fedora; `--proc /proc`, `--dev /dev`, `--tmpfs /tmp`;
   - mỗi `ro_paths` → `--ro-bind`, mỗi `rw_paths` → `--bind`;
   - lệnh trong bwrap là chính binary này với `--stage2 --profile <file> -- <cmd...>`.
2. **Giai đoạn 2** (`--stage2`, đã ở trong bwrap):
   - Landlock với `ABI::V6`: `handle_access(AccessFs::from_all)` và `AccessNet::from_all`; quyền đọc cho `ro_paths` + `/usr` + `/etc/ssl`; quyền ghi cho `rw_paths` + `/tmp`; `NetPort` connect cho từng cổng trong `tcp_connect`; scope abstract UNIX socket và signal.
   - Nếu `landlock_required` và trạng thái không phải `FullyEnforced` → in lỗi, thoát mã 70, không chạy lệnh (fail closed, REQ-T-8-01). Nếu không bắt buộc và chỉ `PartiallyEnforced` → ghi cảnh báo ra stderr rồi chạy tiếp.
   - seccomp (crate `seccompiler`): mặc định cho phép; từ chối với `EPERM`: `socket` với domain khác `AF_UNIX`/`AF_INET`/`AF_INET6`, `socket` kiểu `SOCK_RAW` hoặc `SOCK_DGRAM` trên `AF_INET*`, `ptrace`, `bpf`, `perf_event_open`, `kexec_load`, `init_module`, `finit_module`, `mount`, `umount2`, `pivot_root`, `unshare`, `setns`.
   - `prctl(PR_SET_NO_NEW_PRIVS)` trước seccomp; rồi `execve(cmd)`.
- Chặn UDP cũng chặn DNS qua cổng 53. Tool có mạng phân giải tên qua varlink của systemd-resolved: thêm `/run/systemd/resolve/io.systemd.Resolve` vào `allow_unix_socket` của profile. M0 chưa có tool nào cần mạng.
- Giới hạn bộ nhớ `memory_max_mb` không làm ở đây: executor đặt qua cgroup.
- Mã thoát: lệnh con thoát mã nào thì trả mã đó; lỗi của sandbox dùng 64–79 (64 profile hỏng, 65 thiếu bwrap, 70 Landlock không đủ, 71 seccomp lỗi).

## Kiểm thử và nghiệm thu

Test tích hợp (chạy trong VM) dùng `sh -c` làm lệnh con:

| Test | Mong đợi |
| --- | --- |
| Đọc file trong `ro_paths` | Được |
| Ghi file trong `ro_paths` | Bị từ chối |
| Ghi file trong `rw_paths` | Được |
| Đọc `$HOME/.ssh` không có trong profile | Không thấy file (không bind) |
| `curl` tới cổng không có trong `tcp_connect` | Bị từ chối |
| Tạo socket UDP | `EPERM` |
| `landlock_required=true` trên kernel giả lập thiếu ABI (biến môi trường test ép ABI thấp) | Thoát 70, lệnh không chạy |
| Lệnh con thoát 3 | aios-sandbox-exec thoát 3 |

```bash
dev/vm-run.sh 'cargo test -p aios-sandbox-exec -- --test-threads=1'
dev/vm-run.sh 'cargo clippy -p aios-sandbox-exec -- -D warnings'
cargo check -p aios-sandbox-exec          # trên Mac vẫn biên dịch được (bản "linux only")
```

## Ngoài phạm vi

cgroup tạm (executor), BPF LSM (M2), sandbox OpenShell (M4).
