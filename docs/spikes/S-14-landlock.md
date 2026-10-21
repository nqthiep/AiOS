# S-14 (M0, phần Landlock) — Đo hành vi thật của `aios-sandbox-exec` trên kernel 7.2

_Ngày đo: 2026-10-21 · Task: `s14` (thiết kế: [docs/m0/s14.md](../m0/s14.md)) · Script: [dev/vm/spikes/s14/run.sh](../../dev/vm/spikes/s14/run.sh)_

> Phần BPF LSM của S-14 làm ở M2, không nằm trong báo cáo này.
> Mọi số liệu dưới đây lấy từ lần chạy cuối của
> `dev/vm-run.sh 'bash dev/vm/spikes/s14/run.sh'` (chạy 3 lần, ma trận giống nhau ở
> cả 3 lần; số đo thời gian dao động giữa các lần, xem §4). Cổng TCP là cổng ngẫu nhiên
> do script chọn mỗi lần chạy.

## Môi trường đo

| Mục | Giá trị |
| --- | --- |
| VM | Lima `aios-dev`, Fedora Linux 44 (Cloud Edition), aarch64 |
| Kernel | `7.2.11-200.fc44.aarch64`; LSM: `lockdown,capability,yama,selinux,bpf,landlock,ipe,ima,evm` |
| Header kernel | `kernel-headers-7.2.4-200.fc44.aarch64` (`/usr/include/linux/landlock.h`) |
| bubblewrap | 0.12.0 |
| Crate | `landlock` 0.4.7, `seccompiler` 0.5.0 (theo `Cargo.lock`) |
| Toolchain | rustc 1.99.1, cargo 1.99.1; Python 3.14.3 (dùng cho probe) |
| Helper | `aios-sandbox-exec` build debug từ nhánh `task/s14`, `REQUESTED_ABI = ABI::V6` |

## 1. ABI Landlock

| Mục | Kết quả đo | Cách đo |
| --- | --- | --- |
| ABI kernel báo | **10** | `landlock_create_ruleset(NULL, 0, LANDLOCK_CREATE_RULESET_VERSION)` (syscall 444) qua `ctypes` |
| Errata kernel | `0xf` | cùng syscall với cờ `LANDLOCK_CREATE_RULESET_ERRATA` |
| Bit `handled_access_fs` kernel nhận | 0–16 (17 quyền, bit 16 = `RESOLVE_UNIX`) | tạo ruleset với từng bit; bit lạ trả `EINVAL` |
| Bit `handled_access_net` kernel nhận | 0–3 (`BIND_TCP`, `CONNECT_TCP`, `BIND_UDP`, `CONNECT_SEND_UDP`) | như trên |
| Bit `scoped` kernel nhận | 0–1 (abstract UNIX socket, signal) | như trên |
| ABI lớn nhất crate `landlock` 0.4.7 biết | **V9** (`enum ABI` trong `src/compat.rs`; `ABI::from(n ≥ 9)` trả `V9`) | đọc mã nguồn crate trong `~/.cargo/registry` của VM |
| ABI helper yêu cầu | **V6** | `crates/aios-sandbox-exec/src/linux/landlock.rs` |

Theo header kernel 7.2: quyền UDP (`BIND_UDP`, `CONNECT_SEND_UDP`) có từ ABI 10,
`RESOLVE_UNIX` (kiểm soát `connect` tới UNIX socket có đường dẫn) có từ ABI 9.
Crate 0.4.7 có `AccessFs::ResolveUnix` (V9) nhưng `AccessNet` chỉ có `BindTcp` và
`ConnectTcp`: **crate chưa áp được quyền UDP của ABI 10**.

Kernel thực sự chặn UDP ở ABI 10. Đo bằng Landlock đơn thuần (không bwrap, không
seccomp; một tiến trình Python con tự gọi `landlock_restrict_self`):

| Ruleset mạng | `sendto` cổng được phép | `sendto` cổng khác | `bind` cổng được phép | `bind` cổng khác |
| --- | --- | --- | --- | --- |
| Chỉ bit TCP (như V6) | OK | OK | OK | OK |
| TCP + UDP (ABI 10); cho `CONNECT_SEND_UDP` tới cổng `in`, `BIND_UDP` cổng 0 và cổng `bind_in` | OK | `EACCES` | OK | `EACCES` |

## 2. Ma trận quyền

Hai cột kết quả:

- **bwrap-only**: chạy probe trong `bwrap` với đúng các cờ của
  `crates/aios-sandbox-exec/src/linux/bwrap.rs`, không có stage 2 (không Landlock,
  không seccomp). Cột này cho biết bubblewrap một mình chặn gì.
- **sandbox**: chạy qua `aios-sandbox-exec` (bwrap + Landlock V6 + seccomp),
  `landlock_required = true`.

Profile dùng cho ma trận (mọi đường dẫn là thư mục, xem lỗi L-1 ở §7):
`ro_paths = [<D>/ro]`, `rw_paths = [<D>/rw]`, `allow_unix_socket = [<D>/sock]`,
`tcp_connect = [in, bind_in]`, với `<D>` là thư mục `mktemp` dưới `/var/tmp`.
Hàng ghi "`tcp_connect=[]`" và "`--unshare-net`" dùng cùng profile nhưng `tcp_connect`
rỗng. Tiến trình nghe phía host (Python): TCP trên cổng `in` và `out`, abstract UNIX
socket, UNIX socket tại `<D>/sock/in.sock` và `<D>/ro/out.sock`.

Kết quả là `OK` hoặc tên errno; `bước:errno` nghĩa là lỗi ở bước đó (ví dụ
`socket:EPERM` là lỗi ở lời gọi `socket()`).

| Quyền | Phạm vi | Đích | bwrap-only | sandbox | Lớp chặn (§3) |
| --- | --- | --- | --- | --- | --- |
| đọc | trong | `ro_paths/file.txt` | OK | OK | — |
| đọc | ngoài | `/proc/self/status` (thấy được, không cấp) | OK | `EACCES` | Landlock |
| đọc | ngoài | thư mục host không có trong profile | `ENOENT` | `ENOENT` | bwrap (không mount) |
| ghi | trong | `rw_paths/write.txt` | OK | OK | — |
| ghi | ngoài | `/proc/self/comm` (thấy được, ghi được) | OK | `EACCES` | Landlock |
| ghi | ngoài | `ro_paths/file.txt` | `EROFS` | `EROFS` | bwrap (`--ro-bind`) |
| tạo | trong | `rw_paths/new.txt` | OK | OK | — |
| tạo | ngoài | `/dev/s14-new` (tmpfs của bwrap) | OK | `EACCES` | Landlock |
| tạo | ngoài | `ro_paths/new.txt` | `EROFS` | `EROFS` | bwrap (`--ro-bind`) |
| xóa | trong | `rw_paths/del.txt` | OK | OK | — |
| xóa | ngoài | `/dev/stdin` (symlink của bwrap) | OK | `EACCES` | Landlock |
| xóa | ngoài | `ro_paths/file.txt` | `EROFS` | `EROFS` | bwrap (`--ro-bind`) |
| đổi tên | trong | `rw_paths/ren.txt → ren2.txt` | OK | OK | — |
| đổi tên | ngoài | `/dev/stdout → /dev/stdout2` | OK | `EACCES` | Landlock |
| đổi tên | ngoài | `ro_paths/file.txt → file2.txt` | `EROFS` | `EROFS` | bwrap (`--ro-bind`) |
| truncate | trong | `rw_paths/trunc.txt` | OK | OK | — |
| truncate | ngoài | `/proc/self/comm` | OK | `EACCES` | Landlock |
| truncate | ngoài | `ro_paths/file.txt` | `EROFS` | `EROFS` | bwrap (`--ro-bind`) |
| `ioctl` | trong | `TCGETS` trên file thường trong `rw_paths` | OK (driver trả `ENOTTY`) | OK (driver trả `ENOTTY`) | — |
| `ioctl` thiết bị | ngoài | `TCGETS` trên `/dev/null` của `bwrap --dev` | OK (driver trả `ENOTTY`) | `open:EACCES` | Landlock (chặn ngay lúc `open`) |
| `ioctl` thiết bị | ngoài | `TCGETS` trên fd 0 = `/dev/null` của host (fd kế thừa) | OK (driver trả `ENOTTY`) | OK (driver trả `ENOTTY`) | không lớp nào chặn |
| connect TCP | trong | `127.0.0.1:in` (có trong `tcp_connect`) | OK | OK | — |
| connect TCP | ngoài | `127.0.0.1:out` (không có trong `tcp_connect`) | OK | `EACCES` | Landlock |
| connect TCP | ngoài | `127.0.0.1:in`, profile `tcp_connect=[]` | `ECONNREFUSED` | `EACCES` | Landlock (trước cả netns riêng) |
| bind TCP | trong | `127.0.0.1:bind_in` (có trong `tcp_connect`) | OK | `EACCES` | Landlock |
| bind TCP | ngoài | `127.0.0.1:bind_out` | OK | `EACCES` | Landlock |
| UDP `sendto` | trong | `127.0.0.1:in` (cổng có trong `tcp_connect`) | OK | `socket:EPERM` | seccomp |
| UDP `sendto` | ngoài | `127.0.0.1:out` | OK | `socket:EPERM` | seccomp |
| UDP `bind` | trong | `127.0.0.1:bind_in` | OK | `socket:EPERM` | seccomp |
| UDP `bind` | ngoài | `127.0.0.1:bind_out` | OK | `socket:EPERM` | seccomp |
| abstract UNIX socket | trong | listener tạo trong sandbox | OK | OK | — |
| abstract UNIX socket | ngoài | listener của host, `--share-net` | OK | `EPERM` | Landlock (scope) |
| abstract UNIX socket | ngoài | listener của host, `--unshare-net` | `ECONNREFUSED` | `ECONNREFUSED` | bwrap (netns riêng) |
| UNIX socket có đường dẫn | trong | `<D>/sock/in.sock` (thư mục trong `allow_unix_socket`) | OK | OK | — |
| UNIX socket có đường dẫn | ngoài | `<D>/ro/out.sock` (trong `ro_paths`, không có trong `allow_unix_socket`) | OK | **OK** | **không lớp nào chặn** (lỗi L-2) |
| gửi signal | trong | `fork` một con trong sandbox rồi `SIGTERM` | OK | OK | — |
| gửi signal | ngoài | PID 1 = tiến trình init của bwrap (ngoài miền Landlock) | OK | `EPERM` | Landlock (scope) |
| gửi signal | ngoài | PID của tiến trình host | `ESRCH` | `ESRCH` | bwrap (PID namespace) |

Signal được gửi bằng `kill(pid, 0)` ở hai hàng "ngoài" để không làm hại tiến trình đích.

Thêm: profile có đường dẫn **không phải thư mục** (chỉ chạy qua sandbox):

| Profile | Mã thoát | stderr / kết quả probe |
| --- | --- | --- |
| `ro_paths=[file]`, `landlock_required=true` | 70 | `landlock is not fully enforced: refusing to run the command (fail closed)` |
| `ro_paths=[file]`, `landlock_required=false` | 0 | `warning: landlock is only partially enforced`, đọc file OK |
| `rw_paths=[file]`, `landlock_required=true` | 70 | như hàng 1 |
| `allow_unix_socket=[socket]`, `landlock_required=true` | 70 | như hàng 1 |
| `allow_unix_socket=[socket]`, `landlock_required=false` | 0 | cảnh báo như trên, `connect` OK |
| `rw_paths=[/dev/null]`, `landlock_required=false` | 0 | cảnh báo như trên, `open:EACCES` |
| bwrap-only + `--bind /dev/null /dev/null` | 0 | `open:EACCES` |

## 3. Lớp nào chặn phần nào

Cách quy lớp chặn (helper không có công tắc tắt riêng Landlock hay seccomp, và
task không cho sửa mã):

1. So cột bwrap-only với cột sandbox: bwrap-only cho phép mà sandbox từ chối thì
   lớp chặn nằm ở stage 2 (Landlock hoặc seccomp).
2. Trong stage 2, phân biệt bằng errno và danh sách syscall của seccomp: bộ lọc
   seccomp (`seccomp.rs`) chỉ chặn `socket` (domain/kiểu không phải TCP) và các syscall
   `ptrace`, `bpf`, `perf_event_open`, `kexec_load`, `init_module`, `finit_module`,
   `mount`, `umount2`, `pivot_root`, `unshare`, `setns`, `io_uring_*`, đều trả `EPERM`.
   Bộ lọc không đụng `open`, `unlink`, `rename`, `truncate`, `ioctl`, `connect`, `bind`,
   `kill`. Vì vậy `EACCES` ở các thao tác này là Landlock; `EPERM` ở `connect` abstract
   socket và `kill` là scope của Landlock (kernel trả `EPERM` cho hai scope này).
3. Riêng UDP: đo Landlock đơn thuần ở §1 cho thấy ruleset chỉ có bit TCP (như V6)
   không chặn UDP. Lời gọi bị chặn là `socket(AF_INET, SOCK_DGRAM)` (`socket:EPERM`).

Kết quả:

| Lớp | Chặn (đã đo) |
| --- | --- |
| bubblewrap | Đường dẫn không có trong profile (`ENOENT`); ghi/tạo/xóa/đổi tên/truncate trong `ro_paths` (`EROFS`, mount chỉ đọc); abstract socket của host khi `--unshare-net`; signal tới PID host (`ESRCH`, PID namespace); thiết bị bind vào qua profile (`--bind` mount `nodev`, `open:EACCES` ngay cả khi không có Landlock) |
| Landlock V6 | Đọc/ghi/tạo/xóa/đổi tên/truncate ngoài các đường dẫn được cấp nhưng vẫn thấy được trong sandbox (`/proc`, `/dev`); mở thiết bị của `bwrap --dev`; connect TCP tới cổng ngoài `tcp_connect` (cả khi `tcp_connect=[]`); **mọi** bind TCP (profile không có trường cấp bind); abstract UNIX socket và signal ra ngoài miền Landlock |
| seccomp | **UDP (chỉ seccomp chặn)**; ngoài ma trận còn raw socket, SCTP/MPTCP, domain khác `AF_UNIX`/`AF_INET*`, `io_uring` và các syscall nguy hiểm (test `seccomp_denies_non_tcp_sockets_and_dangerous_syscalls`) |
| Không lớp nào | `connect` tới UNIX socket có đường dẫn nằm trong `ro_paths`/`rw_paths` dù không có trong `allow_unix_socket` (L-2); `ioctl` trên fd đã mở trước khi vào sandbox (fd kế thừa) |

Giới hạn của cách quy lớp:

- `ro_paths`: cả hai cột đều `EROFS`. \[Inference\] VFS kiểm mount chỉ đọc trước khi
  gọi hook Landlock, nên không quan sát được Landlock ở đây; Landlock chỉ cấp
  `Execute | ReadFile | ReadDir` cho `ro_paths` (đọc mã), tức vẫn là lớp chặn thứ hai.
- Quyền `IOCTL_DEV` của Landlock **không đo riêng được** với helper hiện tại: không có
  thiết bị nào vừa mở được vừa được Landlock cấp. Thiết bị của `bwrap --dev` không
  được cấp (bị chặn từ lúc `open`); thiết bị đưa vào profile thì bị mount `nodev`
  (hàng bwrap-only + `--bind /dev/null` → `open:EACCES`).
- Không tách được Landlock và seccomp bằng một lần chạy có/không từng lớp vì helper
  không có công tắc; kết luận dựa trên errno, cột bwrap-only, phép đo Landlock đơn thuần
  ở §1 và việc đọc danh sách syscall trong `seccomp.rs`.

## 4. Thời gian khởi động thêm

Lệnh `true`, 50 lần mỗi dòng, đo thời gian tròn của `subprocess.run` (Python
`perf_counter_ns`) từ host của VM. Lần chạy cuối:

| Cách chạy | Trung vị | Nhỏ nhất | p90 | Lớn nhất |
| --- | --- | --- | --- | --- |
| `true` chạy thẳng | **0,30 ms** | 0,28 ms | 0,32 ms | 0,41 ms |
| bwrap-only, `--unshare-net` | 2,74 ms | 2,61 ms | 3,04 ms | 3,45 ms |
| `aios-sandbox-exec`, `tcp_connect=[]` (`--unshare-net`) | **3,98 ms** | 3,78 ms | 4,18 ms | 4,43 ms |
| `aios-sandbox-exec`, `tcp_connect` 2 cổng (`--share-net`) | 3,64 ms | 3,54 ms | 3,77 ms | 3,90 ms |

Thời gian thêm (trung vị) so với chạy thẳng: **+3,68 ms** (không mạng) và +3,34 ms
(có mạng). Trong đó bwrap chiếm 2,44 ms; phần còn lại (+1,24 ms so với bwrap-only)
là stage 2. \[Inference\] Stage 2 gồm một lần `execve` helper thêm, đọc lại profile,
tạo ruleset Landlock và biên dịch bộ lọc seccomp; bản build debug nên bản release có
thể nhanh hơn, chưa đo.

Số đo dao động giữa các lần chạy: lần chạy thứ hai (cùng ngày, cùng VM) cho trung vị
0,36 ms (thẳng), 4,89 ms (bwrap-only), 6,74 ms (sandbox không mạng), 6,07 ms (sandbox
có mạng). VM dùng chung CPU với máy Mac. Cả hai lần đều dưới 7 ms trung vị.

## 5. `landlock_required=true` và ABI giả lập thấp

Biến test `AIOS_SANDBOX_TEST_FORCE_ABI=<n>`, profile thư mục, `tcp_connect=[]`,
lệnh `true`:

| `landlock_required` | ABI ép | Mã thoát | stderr |
| --- | --- | --- | --- |
| true | (không đặt) | 0 | (trống) |
| true | 0, 1, 2, 3, 4, 5 | **70** | `aios-sandbox-exec: landlock is not fully enforced: refusing to run the command (fail closed)` |
| true | 6, 7, 9, 10 | 0 | (trống) |
| false | (không đặt) | 0 | (trống) |
| false | 5 | 0 | `aios-sandbox-exec: warning: landlock is only partially enforced` |

Fail closed đúng thiết kế (REQ-T-8-01): ABI < 6 thì lệnh không chạy, mã 70.

Giới hạn: biến test chỉ đổi **kết luận** sau khi ruleset V6 thật đã áp lên kernel
ABI 10 (`landlock.rs`, hàm `enforcement`), không giả lập kernel cũ thật.
\[Inference\] Trên kernel thật có ABI < 6, crate (mức tương thích mặc định
`BestEffort`) bỏ các quyền kernel không hỗ trợ và trả `PartiallyEnforced`, nên helper
cũng thoát 70; \[Unverified\] chưa đo trên kernel cũ (tech-stack §12 đề xuất
virtme-ng cho việc này). Ngoài biến test, lỗi L-1 cũng làm helper thoát 70 trên kernel
ABI 10 (§2, bảng cuối).

## 6. Đề xuất

1. **Đổi ABI V6 → V9 cho M1, cùng lúc sửa L-1.** Crate 0.4.7 đã có V9, kernel Fedora
   7.2 báo ABI 10 và nhận bit `RESOLVE_UNIX`. V9 thêm `AccessFs::ResolveUnix` vào
   `from_write` (không vào `from_read`), nên \[Inference\] socket trong `ro_paths` sẽ bị
   chặn còn `allow_unix_socket`/`rw_paths` vẫn được cấp (chưa đo, cần sửa mã helper): đóng lỗ L-2, làm `allow_unix_socket`
   có hiệu lực thật. Phải sửa L-1 trước, nếu không mọi `allow_unix_socket` trỏ tới file
   socket (như cấu hình DNS qua `/run/systemd/resolve/io.systemd.Resolve` trong
   sandbox.md) đều thoát 70. \[Inference\] V7 (cờ log audit) và V8 (`TSYNC`) không cần cho
   helper đơn luồng. Rủi ro: kernel ABI < 9 (< 7.1 theo tài liệu crate) sẽ cho
   `PartiallyEnforced` → tool `landlock_required` thoát 70; chấp nhận được vì ảnh sản
   phẩm theo kernel Fedora 7.2 và CI chạy S-14 trên kernel kế tiếp (tech-stack §2).
   Cần thêm test tích hợp: socket trong `ro_paths` bị từ chối, socket trong
   `allow_unix_socket` với `landlock_required=true` chạy được.
2. **Không chờ crate hỗ trợ ABI 10 để bỏ chặn UDP bằng seccomp.** Kernel đã chặn được
   UDP ở ABI 10 (đo ở §1), nhưng crate 0.4.7 chưa có quyền UDP, và kể cả khi có thì
   seccomp vẫn cần cho raw socket, SCTP/MPTCP, domain khác, `io_uring` — những thứ
   Landlock mạng không phủ (\[Inference\] theo header: quyền mạng của Landlock chỉ theo
   cổng TCP/UDP). Khi crate có ABI 10, có thể **nới** seccomp cho `SOCK_DGRAM` và giới
   hạn cổng UDP bằng Landlock (ví dụ DNS cổng 53) nếu M1 có tool cần; đó là tính năng
   mới, không phải điều kiện của M1.
3. Ghi nhận cho thiết kế profile (không phải lỗi): Landlock chặn **mọi** bind TCP vì
   profile không có trường cấp bind (tech-stack §8 nhắc "cổng TCP được bind/connect");
   `/proc` và `/dev/null` không dùng được trong sandbox.

## 7. Lỗi phát hiện

Không sửa mã `aios-sandbox-exec` (ngoài phạm vi task); cần leader quyết định.

**L-1 (cao): đường dẫn không phải thư mục trong profile làm helper fail closed.**
Mọi file hoặc socket trong `ro_paths`, `rw_paths`, `allow_unix_socket` làm Landlock
báo `PartiallyEnforced`: thoát 70 khi `landlock_required=true`, chỉ cảnh báo khi
`false` (đo ở §2, bảng cuối). Nguyên nhân (đọc mã): `landlock.rs` cấp
`AccessFs::from_read(abi)` (có `ReadDir`) và `AccessFs::from_all(abi)` (có các quyền
chỉ dành cho thư mục) cho mọi đường dẫn; với file, crate 0.4.7
(`PathBeneath::try_compat_inner` trong `src/fs.rs`) bỏ các quyền thư mục và trả
`CompatResult::Partial`, kéo trạng thái ruleset thành `PartiallyEnforced`. Test hiện
có không bắt được vì profile test chỉ dùng thư mục và `allow_unix_socket` rỗng
(`cargo test -p aios-sandbox-exec` trong VM: 9 passed). Hướng sửa: với đường dẫn
không phải thư mục, giao quyền với `AccessFs::from_file(abi)`; thêm test với socket
trong `allow_unix_socket` và `landlock_required=true`. Cách tránh tạm thời (đã đo):
khai báo thư mục chứa socket thay cho file socket, nhưng như vậy cấp cả quyền ghi cho
cả thư mục.

**L-2 (cao, giới hạn của V6): `allow_unix_socket` không được Landlock thực thi.**
`connect` tới UNIX socket có đường dẫn nằm trong `ro_paths` (không có trong
`allow_unix_socket`) thành công (§2). Landlock V6 không kiểm soát `connect` UNIX socket
có đường dẫn (`RESOLVE_UNIX` có từ ABI 9); chỉ có tầm nhìn của bwrap giới hạn socket nào
kết nối được. Đề xuất ở §6 mục 1.

**L-3 (thấp, ghi nhận): fd kế thừa không bị Landlock kiểm.** `ioctl` trên fd 0 (đã mở
ở host) thành công. Landlock kiểm quyền lúc `open`, nên fd mở trước khi vào sandbox
không bị kiểm. \[Inference\] `exec.rs` không đóng fd ≥ 3, nên mọi fd không có
`O_CLOEXEC` của tiến trình gọi đều vào được sandbox; chưa đo với fd ≥ 3. Broker nên chỉ
truyền stdin/stdout/stderr.

## 8. Chạy lại

```bash
dev/vm-run.sh 'bash dev/vm/spikes/s14/run.sh'
```

Script build `aios-sandbox-exec`, tạo thư mục `mktemp` dưới `/var/tmp` (biến
`S14_WORKDIR_BASE`), viết profile JSON và probe Python vào đó, chạy tiến trình nghe ở
host, in phiên bản, kết quả đo ABI, ma trận, bảng profile file/socket, bảng ABI giả lập
và thời gian (biến `S14_RUNS`, mặc định 50), rồi xóa thư mục tạm khi thoát.

## 9. Ngoài phạm vi

- Phần BPF LSM của S-14 (M2).
- Đo trên kernel cũ thật (ABI < 6) và trên kernel kế tiếp của Fedora.
- Sửa L-1, L-2, L-3 trong `aios-sandbox-exec`.

## 8. Trạng thái sau sửa (2026-10-21, task `sandbox-fix`)

- L-1 đã sửa: đường dẫn không phải thư mục chỉ nhận `AccessFs::from_file(ABI::V9)`; test `file_in_ro_paths_is_fully_enforced` (file trong `ro_paths`, `landlock_required=true`) thoát 0.
- L-2 đã sửa: helper yêu cầu `ABI::V9`, `allow_unix_socket` nhận đúng `RESOLVE_UNIX` và được `--ro-bind`. Test `only_allowed_unix_sockets_can_be_connected`: socket được cấp → connect được, socket trong `ro_paths` → `EACCES`. Kiểm đột biến: đổi lại `ABI::V6` thì test này FAIL.
- Hệ quả: kernel có ABI < 9 giờ bị fail closed khi `landlock_required=true` (ma trận ABI giả lập ở §5 đo với V6; ép ABI 6–8 nay cũng thoát 70 — \[Inference\] suy từ điều kiện `abi < REQUESTED_ABI` trong `landlock.rs`, chưa chạy lại `run.sh`).
- Fixture test chuyển từ `/tmp` sang `/var/tmp`, vì quyền ghi Landlock cấp cho `/tmp` cộng vào mọi rule bên dưới nó.
- L-3 (fd kế thừa) chưa sửa: Executor (task `executor`) phải đóng fd không cần trước khi gọi helper.
