# ld-vm — Cài công cụ vào VM aios-dev

| Thuộc tính | Giá trị |
| --- | --- |
| Người làm | leader |
| Phụ thuộc | — |
| Chặn | sandbox, llm, hindsight |
| Nguồn thiết kế | roadmap.md §6; [README.md](README.md) §4, §6 |

## Mục tiêu

VM `aios-dev` (Fedora 44, kernel 7.2.11) có đủ công cụ để build, test và chạy spike của M0, cài bằng một script chạy lại được.

## Phạm vi

Được tạo: `dev/vm/setup.sh` (chạy được nhiều lần, không lỗi lần 2), `dev/vm/README.md`.

## Thiết kế

Gói cài bằng `dnf` (Fedora 44):

| Nhóm | Gói |
| --- | --- |
| Build | `gcc`, `clang`, `llvm`, `make`, `pkgconf`, `git`, `rustup` (rồi `rustup default` theo `rust-toolchain.toml`) |
| BPF (chuẩn bị cho M2) | `libbpf-devel`, `bpftool`, `kernel-devel` khớp `uname -r` |
| Sandbox | `bubblewrap`, `libseccomp-devel` |
| Python | `python3`, `uv` |
| Dịch vụ | `postgresql-server`, `postgresql18-pgvector` (tên theo mdapi F44), `llama-cpp` |
| Đo | `sysstat`, `smem` \[Unverified có trong F44\] |

- Tạo thư mục `/var/lib/aios/{audit,plugins/vendor,plugins/local}`, `/usr/share/aios/{plugins,trust}`, `/etc/aios`, `/run/aios` (qua `tmpfiles.d`), chủ sở hữu là user mặc định của VM (`quangthiep.guest`) cho M0.
- Tạo slice người dùng `aios-agent.slice` (file `~/.config/systemd/user/aios-agent.slice`).
- Không mở cổng nào ra ngoài VM.

## Kiểm thử và nghiệm thu

```bash
limactl shell aios-dev -- bash -lc 'bash ~/build/AiOS/dev/vm/setup.sh && bash ~/build/AiOS/dev/vm/setup.sh'   # chạy 2 lần
limactl shell aios-dev -- bash -lc 'cargo --version && bwrap --version && bpftool version && llama-server --version && psql --version && uv --version'
limactl shell aios-dev -- bash -lc 'systemctl --user cat aios-agent.slice'
```

## Ngoài phạm vi

Tải mô hình (task llm), cài Hindsight (task hindsight).
