# AiOS — Nghiên cứu tech stack

_Cập nhật: 2026-10-18_

## 0. Phạm vi và phương pháp

> **Cập nhật 2026-10-18.** Một số lựa chọn dưới đây đã bị thay bởi các quyết định D-16 → D-23 (decisions.md) và [integration.md](integration.md): LangGraph → vòng lặp tự viết sau interface `agent-engine` (D-34; opencode từ M4); Slint → gpui-kit (D-17); SQLite + sqlite-vec cho bộ nhớ → Hindsight (D-18), sqlite-vec chỉ còn là phương án dự phòng. Các mục bị thay được đánh dấu "Đã thay" tại chỗ; phần còn lại vẫn có hiệu lực. Spike dùng mã S chung với integration.md §11.3.

Tab này chọn công nghệ cụ thể cho từng thành phần của thiết kế trong [design.md](design.md). Nguồn là tài liệu chính thức, kho mã nguồn và trang phát hành, tra cứu ngày 16/10/2026. Nhãn dùng giống design.md: \[Inference\], \[Unverified\], \[Speculation\], UNKNOWN.

Tiêu chí chọn:

1. Hợp với ngôn ngữ đã chọn: Rust cho core và UI, Python cho tầng nhận thức.
2. Chạy được local và offline.
3. Giấy phép phân phối được trong một OS (ưu tiên MIT, Apache-2.0, hoặc tương thích GPL).
4. Đủ trưởng thành: có dự án dùng thật, phát hành đều.
5. Hỗ trợ tiếng Việt khi liên quan (giọng nói, mô hình, bộ gõ).

Mỗi mục trình bày: khuyến nghị, phương án thay thế, lý do và rủi ro. Mọi lựa chọn ở đây là đề xuất; chủ dự án quyết định sau khi kiểm chứng (§13).

## 1. Tóm tắt: stack khuyến nghị

**Rust cho toàn bộ phần chạy dài hạn và có đặc quyền (core, sandbox, UI, phần cứng); Python chỉ cho tầng nhận thức và bộ nhớ; llama.cpp với Vulkan là runtime mô hình mặc định.** Cột "Độ chắc chắn" là đánh giá \[Inference\] dựa trên mức trưởng thành và mức đã kiểm chứng của từng lựa chọn.

| Tầng | Lựa chọn chính | Dự phòng | Độ chắc chắn | Mục |
| --- | --- | --- | --- | --- |
| Kernel | Kernel của Fedora (dòng stable, hiện 7.2) | Kernel LTS 6.18 tự build | Cao | §2, §2.1 |
| Cập nhật OS | bootc + Fedora bootc image (D-47) | mkosi + systemd-sysupdate | Cao | §2, §2.1 |
| eBPF | C (libbpf CO-RE) + Aya | libbpf-rs | Cao | §3 |
| Scheduler | scx\_lavd qua scx\_loader | cgroup v2 weight | Trung bình | §3 |
| Core runtime | Rust, tokio, zbus, tonic | — | Cao | §4 |
| Policy Engine | Cedar | OPA/Rego | Cao | §4 |
| MCP | rmcp | — | Cao | §4 |
| Điều phối System 2 | ~~LangGraph~~ Đã thay: vòng lặp tự viết sau interface `agent-engine` (D-34); opencode từ M4 | opencode | Trung bình | §5, design §4.4 |
| Runtime LLM | llama.cpp (Vulkan) | CUDA, ROCm, vLLM | Cao | §6 |
| NPU | Lemonade + FastFlowLM (AMD) | OpenVINO (Intel) | Thấp | §6 |
| Mô hình System 1 | Chọn bằng VMLU (ứng viên Qwen3.5-4B, Gemma 3 4B) | — | Thấp (chưa đo) | §6 |
| Bộ nhớ | ~~SQLite + sqlite-vec~~ Đã thay: Hindsight (D-18); embedding multilingual-e5-large-instruct qua Model Gateway | SQLite + sqlite-vec, LanceDB, bge-m3 | Trung bình | §7, integration §6 |
| Sandbox | cgroup systemd + bubblewrap + Landlock + seccomp + BPF LSM | — | Cao | §8 |
| STT | Zipformer-vi (sherpa-onnx) + Whisper lượt 2 | PhoWhisper | Trung bình | §9 |
| TTS | Piper giọng Việt + VieNeu-TTS | — | Trung bình (giấy phép) | §9 |
| Compositor | Smithay | wlroots | Trung bình | §10 |
| Shell UI | ~~Slint~~ Đã thay: gpui-kit (D-17) | Slint, iced | Trung bình | §10, integration §7 |
| Bộ gõ | fcitx5 | IBus | Thấp (chưa kiểm chứng) | §10 |
| Portal | Backend `aios` + xdg-desktop-portal-generic | xdg-desktop-portal-wlr | Trung bình | §10 |
| USB user space | nusb | rusb | Trung bình | §11 |
| Test kernel | virtme-ng | QEMU thủ công | Cao | §12 |

Thay đổi so với design.md:

- Một số code mẫu trong [design.md](design.md) dùng Python cho phần chạy dài hạn (pyudev, dbus-next). NÊN chuyển sang Rust: udev ở §11, D-Bus bằng zbus ở §4.
- sched\_ext có sẵn qua scx\_lavd và API D-Bus của scx\_loader (§3); theo REQ-D-3.1-02, AiOS chỉ bật khi đã có số đo chứng minh cgroup không đủ.
- Máy có NPU AMD: kernel 7.2 của Fedora đáp ứng yêu cầu kernel 7.0+ của Lemonade/FastFlowLM; không cần DKMS (§6).

## 2. Nền hệ điều hành, kernel và cập nhật

**Đã chọn (D-47): distro nền Fedora dạng ảnh bootc; kernel theo Fedora (dòng stable, hiện 7.2); cập nhật bằng bootc.** So sánh với Debian ở §2.1.

| Thành phần | Khuyến nghị | Thay thế | Ghi chú |
| --- | --- | --- | --- |
| Kernel sản phẩm | Kernel của Fedora (dòng stable, hiện 7.2) | Kernel LTS 6.18 tự build | Fedora đưa mọi bản phát hành lên cùng kernel stable mới (§2.1) |
| Kernel kiểm thử trước | Kernel của bản Fedora kế tiếp hoặc rawhide | — | CI chạy S-14 trước khi kernel mới vào ảnh sản phẩm |
| Cập nhật hệ thống | bootc (image mode) | mkosi + systemd-sysupdate \[Unverified\] | OS là một OCI image; cập nhật giao dịch, rollback |
| Distro nền | Fedora bootc base image (D-47) | Debian (§2.1) | Chọn sau khi so sánh với Debian |

**Vì sao cần distro nền.** AiOS là một distro riêng: có tên, ảnh hệ thống, kênh cập nhật và chính sách phát hành của riêng mình. "Distro nền" chỉ là nguồn cung cấp những gói AiOS không tự build: kernel đã cấu hình và build sẵn, glibc, systemd, Mesa và driver GPU, firmware, PipeWire, NetworkManager, chính sách SELinux, cùng hàng nghìn thư viện và các bản vá bảo mật phát hành liên tục. \[Inference\] Không dùng distro nền nghĩa là AiOS tự build và tự vá toàn bộ phần này (theo kiểu Linux From Scratch, Yocto, hoặc dựng trên Gentoo Portage như ChromeOS), cần một đội build và đội bảo mật riêng.

Thiết kế hiện tại đã phụ thuộc vào Fedora ở các điểm: chế độ ảnh bootc, fapolicyd tin cậy theo rpmdb, chính sách SELinux, cấu hình kernel-ark (BPF LSM, Landlock, IPE đã bật), và các gói có sẵn (firewalld, fapolicyd, ClamAV, AIDE, Suricata). Phần riêng của AiOS (`aiosd-*`, plugin, Model Gateway, desktop environment) không phụ thuộc distro; phụ thuộc distro gom ở tầng build ảnh, nên đổi distro sau này tốn công làm lại tầng ảnh và chính sách an ninh, không phải viết lại AiOS.

Chủ dự án giới hạn lựa chọn ở Debian và Fedora; so sánh chi tiết và kết luận ở §2.1.

**Vì sao không dùng kernel LTS.** Xem cuối §2.1.

**Vì sao chọn bootc.** bootc dùng OCI container image làm định dạng phân phối OS: image chứa cả kernel, cài lên máy, rồi cập nhật tại chỗ theo giao dịch ([FOSDEM 2025](https://fosdem.org/schedule/event/fosdem-2025-4513-bootable-containers-and-image-mode-transforming-linux-os-management-with-bootc)). Hệ thống chạy với `/usr` chỉ đọc, `/etc` và `/var` được giữ qua cập nhật ([contemper docs](https://pkg.go.dev/github.com/contemper-project/contemper)). Cách này khớp với yêu cầu image bất biến và rollback (§12.3 design.md). bootc viết chủ yếu bằng Rust.

Bố trí dữ liệu đề xuất:

- `/usr` (trong image): `aiosd`, `aios-shell`, eBPF object, design system.
- `/var/lib/aios/models`, `/var/lib/aios/adapters`: trọng số mô hình và LoRA. Nằm ngoài image để cập nhật độc lập và không làm image phình to.
- `/var/lib/aios/memory`: bộ nhớ người dùng.

```dockerfile
# Containerfile — [Unverified] tên base image cần kiểm tra theo bản Fedora hiện hành
FROM quay.io/fedora/fedora-bootc:latest   # bản phát hành PHẢI ghim digest (@sha256:...), không dùng :latest
RUN dnf -y install NetworkManager pipewire wireplumber bluez upower fcitx5 \
    && dnf clean all
COPY target/release/aiosd-core target/release/aios-model-gateway \
     target/release/aios-compositor target/release/aios-shell target/release/aios-trusted-ui /usr/bin/
COPY bpf/*.bpf.o /usr/lib/aios/bpf/
COPY systemd/ /usr/lib/systemd/system/
RUN systemctl enable aiosd-core.service aios-model-gateway.service   # tên unit theo integration.md §11.1
```

Phiên bản: Fedora có bootc 1.17.1 (updates-testing), ostree 2026.4, composefs 1.0.8 (mdapi, 19/10/2026). API của bootc vẫn có thể đổi giữa các bản; ghim phiên bản trong ảnh.

### 2.1 So sánh Debian và Fedora (D-47)

Chủ dự án giới hạn lựa chọn ở Debian và Fedora, và giao chọn bên phù hợp hơn. Số liệu kiểm ngày 19/10/2026 từ debian.org, qa.debian.org/madison, tracker.debian.org, salsa.debian.org (cấu hình kernel), fedoraproject.org, mdapi.fedoraproject.org, src.fedoraproject.org (cấu hình kernel) và quay.io. Phiên bản gói Fedora lấy bản mới nhất trên F44, có thể còn ở updates-testing.

| Tiêu chí của AiOS | Debian 13.7 "trixie" | Fedora 44 | Bên hơn |
| --- | --- | --- | --- |
| Ảnh bất biến (bootc) | Không có bootc chính thức; chỉ có repo không chính thức trên sid và ảnh cộng đồng. Có mkosi, `systemd-sysupdate`, ostree; composefs chỉ có ở forky/sid | Ảnh `quay.io/fedora/fedora-bootc` chính thức (tag 41–46), có chữ ký cosign; bootc, ostree, composefs trong kho chính thức | **Fedora** |
| Kernel | 6.12 LTS ở stable; 7.2 ở backports | 7.2, cả bản phát hành đều đi theo kernel stable mới | **Fedora** |
| sched\_ext | Tắt ở kernel stable 6.12; bật ở 7.2 backports | Bật | **Fedora** |
| Driver NPU (`amdxdna`, `ivpu`) | Kernel stable thiếu `amdxdna`; cần backports | Có cả hai | **Fedora** |
| BPF LSM, Landlock, IPE, fs-verity, fanotify, IMA | Có | Có | Ngang |
| `bpf_override_return` (Tetragon `Override`) | Không | Không | Ngang: cần kernel tự build trên cả hai |
| `BPF_JIT_ALWAYS_ON` | Không bật | Bật | **Fedora** |
| MAC | AppArmor | SELinux enforcing, chính sách tích hợp với bootc và podman | **Fedora** cho cách ly nhiều người dùng và dịch vụ (D-42) \[Inference\] |
| fapolicyd | 1.3.4 | 2.0.1 | **Fedora** |
| Gói an ninh khác | firewalld 2.3.1, ClamAV 1.4.3, Suricata 7.0.10 (8.0.7 ở backports), **có OpenSnitch** | firewalld 2.4.4, ClamAV 1.4.6, Suricata 8.0.7, không có OpenSnitch | Gần ngang |
| Tetragon, Falco, osquery, YARA-X | Không | Không | Ngang: lấy từ upstream |
| Userspace | systemd 257, Mesa 25.0, podman 5.4, Flatpak 1.16, Rust 1.85, Python 3.13, PostgreSQL 17 | systemd 259, Mesa 26.2, podman 5.8, Flatpak 1.18, Rust 1.99, Python 3.14, PostgreSQL 18 + pgvector | **Fedora** |
| llama.cpp | Chỉ ở forky/sid | Có | **Fedora** |
| Compositor cho M1 (D-35) | Không có niri, không có COSMIC | Có niri 26.04, COSMIC 1.0.9 và ảnh COSMIC Atomic | **Fedora** |
| Bộ gõ tiếng Việt | fcitx5 5.1.12, fcitx5-unikey, **có fcitx5-bamboo** | fcitx5 5.1.23, fcitx5-unikey, không có fcitx5-bamboo | Debian nhỉnh hơn |
| Vòng đời | 3 năm hỗ trợ đầy đủ, LTS tới 2030 | Khoảng 13 tháng mỗi bản; phát hành 6 tháng một lần | **Debian** |
| Quy trình bảo mật | Đội Security Team riêng, DSA có chữ ký, phối hợp embargo | Bản vá qua Bodhi, chờ karma hoặc 7–14 ngày ở testing | **Debian** |
| Build tái lập | Gói không tái lập được bị chặn khi vào testing (từ 5/2026), khoảng 96,9% tái lập | Mục tiêu ≥ 99% ở F43; chưa thấy số hiện tại \[Unverified\] | **Debian** |
| Tiền lệ | Vanilla OS, Endless OS (không dùng bootc) | Universal Blue: Bazzite, Bluefin, Aurora dựng trên ảnh Fedora Atomic | **Fedora** |
| Thương hiệu khi phân phối | Không dùng chữ "Debian" trong tên sản phẩm; được nói "dựa trên Debian"; cần tuyên bố không liên kết | Phải thay `fedora-logos`, `fedora-release`; được gọi là "Fedora Remix" hoặc dùng tên riêng | Ngang |

**Kết luận: chọn Fedora (ảnh bootc).** \[Inference\] AiOS là OS dạng ảnh, dùng nhiều eBPF và BPF LSM, cần SELinux cho nhiều người dùng, và dùng compositor có sẵn ở M1. Fedora đáp ứng các điểm này bằng thành phần chính thức. Với Debian, AiOS phải tự dựng và bảo trì bootc, composefs và bootupd (hoặc chuyển sang mkosi + A/B), dùng kernel backports để có sched\_ext và driver NPU, và tự đóng gói niri hoặc COSMIC.

Chi phí của việc chọn Fedora và cách giảm:

| Chi phí | Cách giảm |
| --- | --- |
| Nâng bản nền 6 tháng một lần; mỗi bản chỉ được hỗ trợ khoảng 13 tháng | CI build ảnh AiOS trên bản Fedora kế tiếp từ khi bản đó branch; chuyển ảnh sản phẩm trong cửa sổ khoảng 7 tháng hai bản cùng được hỗ trợ; bootc rollback nếu lỗi |
| Kernel đổi phiên bản mỗi 6–8 tuần \[Inference\] | BPF dùng CO-RE; CI chạy S-14 trên kernel hiện hành và kernel kế tiếp; ảnh chỉ được đẩy ra khi đạt; ghim digest ảnh để giữ kernel đã kiểm |
| Bản vá bảo mật chờ karma hoặc thời gian ở testing | AiOS CÓ THỂ lấy bản vá từ updates-testing cho gói ảnh hưởng an ninh, sau khi chạy bộ test của AiOS |
| Không có OpenSnitch và fcitx5-bamboo trong kho | AiOS tự đóng gói hai thành phần này khi build ảnh (OpenSnitch có RPM của upstream) |
| Phải đổi thương hiệu | Thay `fedora-logos`, `fedora-release` bằng gói của AiOS ngay ở M0 |
| Phụ thuộc Red Hat \[Unverified: chủ sở hữu thương hiệu\] | Phần riêng của AiOS không phụ thuộc distro (đã nêu ở trên); đổi nền sau này chỉ làm lại tầng ảnh |

**Kernel: theo Fedora, không dùng LTS.** Fedora đưa mọi bản phát hành lên cùng kernel stable mới (hiện 7.2). Dùng kernel LTS (6.18) trên Fedora nghĩa là AiOS tự build và tự vá kernel. \[Inference\] Lợi ích của LTS (API ổn định) đã được CO-RE và CI kernel giảm bớt, trong khi kernel mới cho thêm sched\_ext, `amdxdna` và Landlock ABI 10 (giới hạn được UDP). AiOS chỉ tự build kernel nếu sau này cần tính năng Fedora tắt, ví dụ `bpf_override_return`.

## 3. eBPF và tích hợp kernel

**Khuyến nghị: viết BPF program bằng C (libbpf CO-RE), nạp và quản lý bằng Aya trong `aiosd-core`. Không tự viết scheduler; dùng scx\_lavd và điều khiển qua D-Bus của scx\_loader.**

| Thành phần | Khuyến nghị | Thay thế | Ghi chú |
| --- | --- | --- | --- |
| Thư viện userspace | Aya (Rust) | libbpf-rs | Aya không phụ thuộc libbpf hay bcc, viết thuần Rust, hỗ trợ BTF và async với tokio ([Aya](https://github.com/lucab/aya)) |
| Ngôn ngữ BPF program | C + libbpf CO-RE | Rust với aya-ebpf | \[Inference\] C có nhiều ví dụ và tài liệu verifier nhất |
| BPF LSM | Aya `programs::Lsm` | libbpf-rs | Cần `CONFIG_BPF_LSM=y`, `CONFIG_DEBUG_INFO_BTF=y`, và `bpf` có trong danh sách LSM đang hoạt động ([docs.rs aya](https://docs.rs/crate/aya/latest/source/src/programs/lsm.rs)) |
| Scheduler | scx\_lavd qua scx\_loader | cgroup v2 `cpu.weight` | Có sẵn, không cần tự viết sched\_ext |

**scx\_lavd.** Đây là scheduler sched\_ext theo thuật toán LAVD (Latency-criticality Aware Virtual Deadline): đo mức nhạy độ trễ của từng task rồi dùng thông tin đó để chọn deadline và time slice. Phần BPF ra mọi quyết định; phần Rust nạp code và cung cấp thông tin như topology CPU. Động lực ban đầu là giảm giật khi chơi game ([scx\_lavd README](https://gitea.hillion.co.uk/JakeHillion/scx-upstream/src/branch/simulator.v3/scheds/rust/scx_lavd)).

**scx\_loader.** Dịch vụ này cho phép bắt đầu, dừng và đổi scheduler cùng chế độ qua D-Bus hệ thống `org.scx.Loader`, object `/org/scx/Loader`, method `SwitchScheduler(s, u)`. Ví dụ, đổi sang scx\_lavd ở chế độ 2 (tiết kiệm điện) ([sched-ext/scx-loader](https://github.com/sched-ext/scx-loader/blob/main/configs/org.scx.Loader.xml)).

\[Inference\] Vì đã có scheduler và API D-Bus, việc tích hợp sched\_ext rẻ: một rule Reflex đổi chế độ scx\_lavd theo ngữ cảnh (ví dụ `power = battery_low` → chế độ tiết kiệm điện). Theo REQ-D-3.1-02 (design §3.1), rule này chỉ bật khi đã có số đo chứng minh `cpu.weight` của cgroup không đủ.

```rust
// aiosd-core: nạp LSM program biên dịch từ C, rồi đổi chế độ scheduler
// [Unverified] API Aya và tên hook cần kiểm tra theo phiên bản dùng thực tế
use aya::{programs::Lsm, Btf, Ebpf};

let mut bpf = Ebpf::load_file("/usr/lib/aios/bpf/agent_guard.bpf.o")?;
let btf = Btf::from_sys_fs()?;
let guard: &mut Lsm = bpf.program_mut("aios_guard_open").unwrap().try_into()?;
guard.load("file_open", &btf)?;
guard.attach()?;

let conn = zbus::Connection::system().await?;
conn.call_method(Some("org.scx.Loader"), "/org/scx/Loader", Some("org.scx.Loader"),
                 "SwitchScheduler", &("scx_lavd", 2u32)).await?;   // 2 = tiết kiệm điện
```

Cấu hình kernel bắt buộc cho image AiOS:

- `CONFIG_BPF_LSM=y`, `CONFIG_DEBUG_INFO_BTF=y`, và `bpf` trong danh sách LSM: đặt bằng `CONFIG_LSM` khi build kernel (kernel Fedora đã có `lockdown,yama,integrity,selinux,bpf,landlock,ipe`) hoặc bằng `lsm=` trên cmdline (ghi đè `CONFIG_LSM`). Kiểm tra bằng `/sys/kernel/security/lsm`.
- `CONFIG_SECURITY_LANDLOCK=y` cho sandbox tool.
- Hỗ trợ sched\_ext: `CONFIG_SCHED_CLASS_EXT=y` (đã bật trong kernel 7.2 của Fedora).

## 4. aiosd core bằng Rust

**Khuyến nghị: tokio làm async runtime, zbus cho D-Bus, gRPC (tonic) giữa Rust và Python, Cedar làm Policy Engine, rmcp làm MCP client của Tool Broker.**

| Thành phần | Khuyến nghị | Thay thế | Ghi chú |
| --- | --- | --- | --- |
| Async runtime | tokio | — | Aya và rmcp đều chạy trên tokio |
| D-Bus | zbus | dbus-rs | \[Unverified\] zbus viết thuần Rust; gọi NetworkManager, UPower, logind, scx\_loader |
| IPC Rust ↔ Python | gRPC (tonic / grpcio) qua Unix socket | Cap'n Proto | \[Inference\] gRPC có codegen tốt cho cả 2 ngôn ngữ |
| Policy Engine | Cedar (`cedar-policy`) | OPA/Rego | Viết bằng Rust, nhúng trực tiếp |
| MCP | `rmcp` | — | SDK Rust chính thức của MCP |
| Schema | serde + schemars | — | Dùng chung cho tool schema và UI IR |

**Cedar.** Cedar là ngôn ngữ viết và thực thi chính sách phân quyền, thiết kế cho các mô hình RBAC và ABAC; ứng dụng gọi engine của Cedar để quyết định từng yêu cầu ([cedar-policy/cedar](https://github.com/cedar-policy/cedar)). Kho mã có crate `cedar-policy-symcc`, một trình biên dịch symbolic để kiểm chứng thuộc tính của bộ chính sách và trả về phản ví dụ cụ thể. AiOS dùng crate này để chứng minh các bất biến của chính sách thay vì chỉ kiểm thử (danh sách bất biến ở cuối mục).

**rmcp.** Đây là SDK Rust chính thức của MCP, có tính năng server, client, macro `#[tool]`, sinh JSON Schema qua schemars, và các transport stdio, child process, streamable HTTP ([docs.rs rmcp](https://docs.rs/crate/rmcp)). Tool Broker dùng transport child process để chạy mỗi MCP server trong sandbox riêng (§8).

**Allow, Confirm, Deny bằng hai tập chính sách.** Cedar chỉ trả Allow hoặc Deny. Cách cũ (đánh giá lại với `user_confirmed = true`) biến mọi Deny thành Confirm, vì một luật `permit ... when { context.user_confirmed }` áp cho mọi tool (review.md B-T5). AiOS dùng hai tập chính sách tách biệt, cùng nạp một tập `forbid`:

| Tập | Ý nghĩa của Allow | Nạp cùng |
| --- | --- | --- |
| `auto.cedar` | Tự chạy, không hỏi | `forbid.cedar` |
| `confirmable.cedar` | Được chạy nếu người dùng xác nhận trên bề mặt tin cậy | `forbid.cedar` |

Kết quả: Allow nếu `auto` cho phép; Confirm nếu chỉ `confirmable` cho phép; Deny nếu cả hai từ chối. Không có luật nào phụ thuộc `user_confirmed`; token xác nhận được kiểm ở Tool Broker (design §9.5).

Mọi trường trong `context` do Tool Broker tính, không lấy từ request của client:

| Trường | Cách tính |
| --- | --- |
| `autonomy` | `min(mức tự hành người dùng đặt, trần của kênh)`; trần kênh `remote` và `schedule` là 1, kênh `voice` là 1 cho tool ghi |
| `channel` | `local`, `voice`, `remote`, `schedule`, theo credential của principal |
| `principal.uid`, `principal.is_admin` | Từ `SO_PEERCRED` hoặc credential gắn UID; `is_admin` theo nhóm quản trị của hệ thống (design §18) |
| `tainted` | Taint của session (design §6.3), do Broker theo dõi |
| `risk`, `reversible`, `always_confirm` (thuộc `resource`) | Từ registry đã ký (design §15.2); `risk` là mức hiệu lực |

```cedar
// /usr/share/aios/policy/auto.cedar — tự chạy
// [Unverified] cần schema đầy đủ để validate
permit (principal is Agent, action == Action::"invoke", resource is Tool)
when { resource.risk == "low" && resource.reversible
       && context.autonomy >= 1 && !context.tainted && !resource.always_confirm
       && !resource.system_scope };

permit (principal is Agent, action == Action::"invoke", resource is Tool)
when { resource.risk == "medium" && resource.reversible
       && context.autonomy >= 2 && !context.tainted && !resource.always_confirm
       && !resource.system_scope };
```

```cedar
// /usr/share/aios/policy/confirmable.cedar — chạy được sau khi người dùng xác nhận
permit (principal is Agent, action == Action::"invoke", resource is Tool)
when { ["low", "medium"].contains(resource.risk) };

permit (principal is Agent, action == Action::"invoke", resource is Tool)
when { resource.risk == "high" && ["local", "voice"].contains(context.channel) };
```

```cedar
// /usr/share/aios/policy/forbid.cedar — nạp vào cả hai tập
forbid (principal is Agent, action == Action::"invoke", resource is Tool)
when { resource.risk == "high" && ["remote", "schedule"].contains(context.channel) };

forbid (principal is Agent, action == Action::"invoke", resource is Tool)
when { context.channel == "remote" && resource.name like "schedule.*" };   // kênh từ xa không tạo lịch

forbid (principal is Agent, action == Action::"invoke", resource is Tool)
when { resource.writes_trusted_core };

forbid (principal is Agent, action == Action::"invoke", resource is Tool)
when { resource.system_scope && !principal.is_admin };   // tool toàn hệ thống: chỉ người dùng quản trị (design §18.3)   // tool sửa chính sách, profile, registry, eval: chỉ qua cập nhật ảnh
```

```rust
// aiosd-core/src/policy.rs — chữ ký API khớp cedar-policy 4.14 (review.md §5)
use cedar_policy::{Authorizer, Context, Decision, Request};

pub enum ConfirmLevel { Dialog, DialogWithPin }
pub enum Verdict { Allow, Confirm(ConfirmLevel), Deny }

pub fn evaluate(&self, m: &Manifest, call: &ToolCall, p: &Principal, s: &SessionState)
    -> anyhow::Result<Verdict>
{
    // mọi trường context do Broker tính từ principal, session và registry (không từ client)
    let ctx = Context::from_json_value(self.context_json(m, call, p, s), None)?;
    let req = Request::new(p.uid(), self.invoke_uid(), m.tool_uid(), ctx, None)?;
    let auth = Authorizer::new();
    if auth.is_authorized(&req, &self.auto, &self.entities).decision() == Decision::Allow {
        return Ok(Verdict::Allow);
    }
    if auth.is_authorized(&req, &self.confirmable, &self.entities).decision() == Decision::Allow {
        return Ok(Verdict::Confirm(confirm_level(m, s)));
    }
    Ok(Verdict::Deny)
}

// ma trận design §9.3: high + không đảo ngược cần PIN; session tainted nâng high lên PIN
fn confirm_level(m: &Manifest, s: &SessionState) -> ConfirmLevel {
    match (m.risk, m.reversible, s.tainted) {
        (Risk::High, false, _) | (Risk::High, _, true) => ConfirmLevel::DialogWithPin,
        _ => ConfirmLevel::Dialog,
    }
}
```

Bất biến PHẢI được chứng minh bằng `cedar-policy-symcc` trong CI mỗi khi chính sách đổi (\[Unverified\] cách diễn đạt từng bất biến theo API symcc bản đang ghim):

1. `auto` không Allow tool `high` trong mọi context.
2. `auto` không Allow bất kỳ tool nào khi `context.tainted` là true, vì tool call từ ngữ cảnh chứa nội dung `external` phải nâng ít nhất một mức xác nhận (REQ-D-9.3-01).
3. Không tập nào Allow tool `high` khi `channel` là `remote` hoặc `schedule`.
4. Mọi request `auto` Allow thì `confirmable` cũng Allow (tập tự chạy nằm trong tập được phép).
5. Không tập nào Allow tool `system_scope` khi `principal.is_admin` là false.
6. `auto` không Allow tool `system_scope` (tool toàn hệ thống luôn hỏi xác nhận).

## 5. Tầng nhận thức và agent

> **Đã thay (D-34).** Vòng lặp System 2 là vòng lặp tự viết trong `aiosd-cognition`, sau interface `agent-engine` (design §4.4); từ M4 opencode là engine thay thế ([integration.md §4](integration.md)). Không dùng LangGraph. Hai quy tắc ở mục này vẫn có hiệu lực với mọi engine: idempotency key cho mọi lệnh gọi Tool Broker, và xác nhận phải là token ký gắn hành động (design §9.5). Phần LangGraph bên dưới giữ lại làm tham khảo cho phương án "vòng lặp tự viết".

**Tham khảo (đã thay bởi D-34): LangGraph cho vòng lặp System 2; System 1 gọi thẳng Model Gateway, không qua framework. Tầng Python không tự chạy tool: mọi tác động đi qua Tool Broker và Policy Engine bằng Rust.**

| Thành phần | Khuyến nghị | Thay thế | Ghi chú |
| --- | --- | --- | --- |
| Điều phối System 2 | LangGraph | Vòng lặp tự viết | Bản 1.0 ổn định từ tháng 10/2025, giấy phép MIT, cần Python 3.10+ ([Real Python](https://realpython.com/ref/ai-coding-tools/langgraph/)) |
| Lưu trạng thái task | Checkpointer SQLite của LangGraph | Postgres | \[Unverified\] tên gói checkpointer SQLite |
| System 1 | Gọi Model Gateway với đầu ra ràng buộc theo JSON Schema | — | \[Inference\] tránh overhead framework ở đường cần độ trễ thấp |
| Phân loại ý định | Classifier nhỏ xuất ONNX | Embedding + kNN | Chạy qua ONNX Runtime (§6) |

**Vì sao LangGraph.** LangGraph lưu trạng thái sau mỗi bước bằng checkpointer, nên một tác vụ dừng chờ người duyệt hoặc gặp sự cố có thể tiếp tục đúng chỗ đã dừng ([LangGraph docs](https://docs.langchain.com/oss/python/langgraph/durable-execution/index.html)). Hàm `interrupt()` dừng node và chờ đến khi được resume bằng `Command`. Hai cơ chế này khớp trực tiếp với thẻ `task` có trạng thái `waiting_confirm` (§7.9 design.md).

**Rủi ro cần xử lý.** Khi replay, LangGraph chạy lại các node sau checkpoint, kể cả lệnh gọi LLM và API; tác động phụ cần idempotency key hoặc bước kiểm tra kết quả ([rubinlake](https://rubinlake.com/es/technology-radar/ai-and-data-engineering/langgraph)). Engine nào cũng có thể chạy lại một bước (retry, khôi phục sau lỗi), vì vậy mọi lệnh gọi Tool Broker PHẢI mang idempotency key.

**Quy tắc an ninh.** Xác nhận của người dùng KHÔNG ĐƯỢC là một boolean do engine tự tạo (với LangGraph: giá trị resume sau `interrupt()`). Đó là token do `aios-trusted-ui` ký, gắn với đúng tool, hash args, nonce và hạn dùng; Tool Broker kiểm token và tự thực thi hành động (design §9.5). \[Inference\] Khi đó engine bị prompt injection cũng không tự duyệt hoặc tráo được hành động.

```python
# aiosd/cognition/system2.py — [Unverified] API theo LangGraph 1.x
from langgraph.graph import StateGraph, START, END
from langgraph.types import interrupt

def execute_step(state: TaskState) -> dict:
    step = state["plan"][state["cursor"]]
    key = f"{state['task_id']}:{state['cursor']}"           # idempotency key
    token = None
    if broker.preflight(step) == "confirm":                 # gRPC -> Policy Engine (Rust)
        token = interrupt({"task_id": state["task_id"], "step": step})  # UI: waiting_confirm
        if token is None:
            return {"status": "cancelled"}
    result = broker.invoke(step, idempotency_key=key, confirm_token=token)
    return {"results": state["results"] + [result], "cursor": state["cursor"] + 1}

g = StateGraph(TaskState)
g.add_node("plan", plan)
g.add_node("execute", execute_step)
g.add_node("verify", verify)
g.add_edge(START, "plan")
g.add_edge("plan", "execute")
g.add_edge("execute", "verify")
g.add_conditional_edges("verify", next_or_done,
                        {"next": "execute", "replan": "plan", "done": END})
app = g.compile(checkpointer=sqlite_checkpointer("/var/lib/aios/tasks.db"))
```

## 6. Model runtime, bộ tăng tốc và mô hình

**Khuyến nghị: llama.cpp (llama-server) với backend Vulkan làm runtime mặc định cho mọi hãng GPU; Lemonade + FastFlowLM cho NPU AMD; ONNX Runtime cho classifier, embedding và VAD. Chọn mô hình System 1 bằng benchmark VMLU.**

| Thành phần | Khuyến nghị | Thay thế | Ghi chú |
| --- | --- | --- | --- |
| Runtime LLM mặc định | llama.cpp, backend Vulkan | CUDA (NVIDIA), ROCm (AMD) | API OpenAI-compatible, định dạng GGUF |
| System 2 local trên GPU rời | vLLM | llama.cpp | \[Inference\] chỉ hợp với GPU nhiều VRAM |
| NPU AMD (XDNA 2) | Lemonade + FastFlowLM | — | Có trên Linux từ tháng 3/2026 |
| NPU Intel | OpenVINO | — | \[Unverified\] chưa tra cứu trong đợt này |
| Classifier, embedding, VAD | ONNX Runtime | candle (Rust) | — |

**Vì sao Vulkan làm mặc định.** Vulkan chạy trên NVIDIA, AMD, Intel Arc và iGPU mà không cần cài stack ML riêng của hãng; đổi lại chậm hơn backend gốc khoảng 10–20% theo một hướng dẫn cập nhật tháng 8/2026 ([bestllmfor](https://bestllmfor.com/guide/compiler-llama-cpp-vulkan/)). Trên iGPU dùng bộ nhớ chung của AMD, một phép đo tháng 8/2026 cho thấy Vulkan nhanh hơn ROCm ở mọi cấu hình thử ([llama.cpp #10879](https://github.com/ggml-org/llama.cpp/discussions/10879)). Một điểm cần lưu ý: có báo lỗi Vulkan trên iGPU Intel Arrow Lake ([llama.cpp #19327](https://github.com/ggml-org/llama.cpp/issues/19327)), nên AiOS PHẢI có đường dự phòng CPU.

**NPU AMD.** Từ tháng 3/2026, Lemonade 10.0 cùng FastFlowLM chạy được LLM và Whisper trên NPU Ryzen AI dưới Linux ([Phoronix](https://www.phoronix.com/news/AMD-Ryzen-AI-NPUs-Linux-LLMs)). Lemonade gom nhiều engine sau một API OpenAI-compatible, gồm LLM, Whisper và Kokoro TTS ([lilting.ch](https://lilting.ch/en/articles/amd-lemonade-local-ai-gpu-npu-server)). Ba ràng buộc:

- Lemonade/FastFlowLM yêu cầu kernel 7.0+ có `amdxdna`, hoặc gói `amdxdna-dkms` trên kernel cũ hơn ([Lemonade docs](https://lemonade-server.ai/flm_npu_linux.html)). Kernel 7.2 của Fedora (D-47) đáp ứng yêu cầu này và đã bật `amdxdna`.
- Kernel NPU của FastFlowLM là binary độc quyền ([lilting.ch](https://lilting.ch/en/articles/amd-lemonade-local-ai-gpu-npu-server)).
- NPU hiện hợp với mô hình nhỏ, tiêu thụ điện thấp. Ưu điểm thực tế là chạy Whisper trên NPU trong khi GPU lo LLM ([sleepingrobots](https://sleepingrobots.com/dreams/lemonade-server-npu-strix-halo/)).

\[Inference\] Phân công hợp lý trên máy có NPU: STT và System 1 trên NPU, System 2 local trên iGPU qua Vulkan.

**Chọn mô hình tiếng Việt.** VMLU (ACL 2025) là bộ benchmark tiếng Việt gồm 4 tập: kiến thức chung, đọc hiểu, suy luận và hội thoại ([ACL Anthology](https://aclanthology.org/2025.acl-long.563)). Ứng viên cho System 1 là các mô hình cỡ 4B đã có trong danh sách hỗ trợ của Lemonade, như Qwen3.5-4B (cả trên NPU) và Gemma 3 4B ([Phoronix](https://phoronix.com/news/lemonade-10.0.1)). Chất lượng tiếng Việt của từng mô hình: UNKNOWN đến khi chạy VMLU và bộ kịch bản vàng của AiOS.

```yaml
# /etc/aios/gateway.d/hw-profiles.yaml — Model Gateway chọn profile theo phần cứng phát hiện (§11)
# [Unverified] endpoint và tên mô hình của Lemonade cần kiểm tra theo bản cài thực tế
profiles:
  amd-ryzen-ai:                 # có driver amdxdna
    s1-local: { runtime: lemonade, endpoint: "http://127.0.0.1:8000", model: "<qwen3.5-4b-npu>" }   # chỉ Model Gateway gọi
    stt:      { runtime: lemonade, model: "<whisper-npu>" }   # NPU, nhường GPU cho LLM
    s2-local: { runtime: llama.cpp, backend: vulkan, endpoint: "http://127.0.0.1:8081/v1" }   # cổng theo integration §11.1
  nvidia-dgpu:
    s1-local: { runtime: llama.cpp, backend: cuda }
    s2-local: { runtime: vllm, endpoint: "http://127.0.0.1:8081/v1" }
  generic:
    s1-local: { runtime: llama.cpp, backend: vulkan, fallback: cpu }
    s2-local: null              # L2 dùng cloud; L2 với dữ liệu secret không có route (design §5.4)
```

## 7. Bộ nhớ: lưu trữ, vector, embedding

> **Đã thay (D-18).** Backend bộ nhớ chính là Hindsight với Postgres + pgvector ([integration.md §6](integration.md)). SQLite + sqlite-vec dưới đây là phương án dự phòng. Phần chọn mô hình embedding (multilingual-e5-large-instruct, đánh giá bằng VN-MTEB) vẫn có hiệu lực; embedding chạy qua Model Gateway.

**Khuyến nghị: một file SQLite cho mọi tầng bộ nhớ, sqlite-vec cho tìm kiếm vector, multilingual-e5-large-instruct làm mô hình embedding chạy qua ONNX Runtime. Đánh giá embedding bằng VN-MTEB.**

| Thành phần | Khuyến nghị | Thay thế | Ghi chú |
| --- | --- | --- | --- |
| Lưu trữ | SQLite | — | Một file, dễ snapshot và sao lưu |
| Vector search | sqlite-vec | LanceDB \[Unverified\] | Viết bằng C, không phụ thuộc, giấy phép MIT hoặc Apache |
| Embedding | multilingual-e5-large-instruct (560M) | bge-m3 (568M) | Chạy qua ONNX Runtime |
| Đồ thị quan hệ | Bảng cạnh trong cùng SQLite | Graph DB riêng | \[Inference\] giữ một file, tránh đồng bộ 2 kho |
| Bộ nhớ procedural | Thư mục git có phiên bản | — | \[Unverified\] chọn git2 hoặc gitoxide sau |

**sqlite-vec.** Đây là extension tìm kiếm vector cho SQLite, viết hoàn toàn bằng C và không phụ thuộc thư viện ngoài; tác giả viết nó để thay sqlite-vss ([i-programmer](https://www.i-programmer.info/news/84-database/17458-sqlite-gets-into-vector-search.html)). Bản mới nhất thấy được là 0.1.8, phát hành 30/3/2026 ([RubyGems](https://rubygems.org/gems/sqlite-vec?locale=ja)). Dự án vẫn ở 0.1.x, nên PHẢI ghim phiên bản và bọc sau một trait/interface để đổi được backend.

**Chọn embedding.** Trên benchmark SEA-BED cho các ngôn ngữ Đông Nam Á, cột tiếng Việt của một bảng kết quả cho multilingual-e5-large-instruct 66,92 điểm, bge-m3 64,57, còn bge-multilingual-gemma2 cỡ 9B đạt 67,80 ([SEA-BED, arXiv 2508.12243](https://arxiv.org/pdf/2508.12243)). \[Inference\] Mô hình 560M gần bằng mô hình 9B ở tiếng Việt, nên đủ cho máy cá nhân. Trước khi chốt, NÊN chạy VN-MTEB: 41 tập dữ liệu thuộc 6 nhóm tác vụ (truy hồi, xếp hạng lại, phân loại, phân cụm, phân loại cặp, độ tương đồng ngữ nghĩa) cho embedding tiếng Việt ([arXiv 2507.21500](https://arxiv.org/html/2507.21500v1)).

```python
# aiosd/memory/recall.py — [Unverified] cú pháp KNN theo sqlite-vec 0.1.x
import sqlite3
import sqlite_vec

db = sqlite3.connect("/var/lib/aios/memory/memory.db")
db.enable_load_extension(True)
sqlite_vec.load(db)
db.enable_load_extension(False)

RECALL_SQL = """
WITH knn AS (
  SELECT id, distance FROM memory_vec
  WHERE embedding MATCH ? AND k = ?
)
SELECT m.id, m.content, m.provenance, knn.distance
FROM knn JOIN memory_item m ON m.id = knn.id
WHERE m.deleted = 0
ORDER BY knn.distance
"""

def recall(query_vec: list[float], k: int = 8, allow=("stated", "observed")) -> list:
    rows = db.execute(RECALL_SQL, (sqlite_vec.serialize_float32(query_vec), k * 3)).fetchall()
    return [r for r in rows if r[2] in allow][:k]   # lọc provenance sau KNN (§6.3 design.md)
```

## 8. Sandbox và thực thi công cụ

**Khuyến nghị: mỗi lần gọi tool chạy qua 5 lớp: cgroup tạm thời (systemd), namespace (bubblewrap), Landlock, seccomp, và BPF LSM. Một helper nhỏ `aios-sandbox-exec` áp Landlock và seccomp rồi mới `exec` tool.**

| Lớp | Công cụ | Chặn được gì |
| --- | --- | --- |
| 1. cgroup tạm thời | systemd transient unit qua D-Bus (zbus) | Giới hạn CPU, RAM, IO; cung cấp cgroup id cho BPF LSM |
| 2. Namespace | bubblewrap | Cô lập mount, PID, mạng \[Unverified chi tiết cờ\] |
| 3. Landlock | crate `landlock` | Danh sách thư mục được đọc/ghi; cổng TCP được bind/connect; scope abstract UNIX socket và signal (ABI 6+) |
| 4. seccomp | crate `seccompiler` | Syscall không cần thiết; socket không phải TCP |
| 5. BPF LSM | `agent_guard` (§3, design §3.4) | Ghi, xóa, đổi tên, cắt file ngoài workspace, ptrace, exec, connect; theo cgroup tổ tiên `aios-agent.slice` |

**Landlock.** Landlock có từ Linux 5.13 và cho phép mọi process, kể cả process không đặc quyền, tự giới hạn quyền của mình; crate `landlock` là lớp trừu tượng an toàn cho các syscall này ([docs.rs landlock](https://docs.rs/landlock)). Tài liệu kernel liệt kê ABI tăng dần, với quy tắc mạng từ ABI v4 ([kernel.org](https://docs.kernel.org/userspace-api/landlock.html)). ABI theo kernel: 6.18 LTS = 7, 7.0 = 8, 7.1 = 9, 7.2 = 10, mainline 7.3-rc = 11. Trên kernel trước 7.2, quy tắc mạng chỉ giới hạn bind và connect TCP. Kernel 7.2 của Fedora (ABI 10, D-47) giới hạn được cả UDP bind/connect/send ([landlock.h v7.2](https://github.com/torvalds/linux/blob/v7.2/include/uapi/linux/landlock.h)), nhưng crate `landlock` 0.4.7 mới hỗ trợ tới `ABI::V9` và `AccessNet` chỉ có TCP ([docs.rs](https://docs.rs/landlock/0.4.7/landlock/enum.ABI.html)); seccomp vẫn cần cho đến khi crate hỗ trợ.

**Fail closed.** Crate `landlock` khuyên khai báo tính năng bắt buộc thay vì so số phiên bản ABI. Với tool rủi ro `medium` trở lên, nếu Landlock không được áp đầy đủ, `aios-sandbox-exec` PHẢI từ chối chạy.

**Vì sao cần helper riêng.** \[Inference\] Áp Landlock và seccomp trong một process nhỏ, đơn luồng, ngay trước `exec` đơn giản và an toàn hơn làm trong `pre_exec` của một daemon async nhiều luồng.

```rust
// aios-sandbox-exec/src/main.rs (rút gọn)
// [Unverified] tên kiểu và cờ theo phiên bản crate landlock đang dùng
use landlock::{Access, AccessFs, AccessNet, NetPort, PathBeneath, PathFd,
               Ruleset, RulesetAttr, RulesetCreatedAttr, RulesetStatus, ABI};

fn apply_landlock(m: &Manifest) -> anyhow::Result<()> {
    let abi = ABI::V9;   // V9: RESOLVE_UNIX + scope abstract UNIX socket và signal; kernel 7.2 của Fedora có ABI 10, crate 0.4.7 tới V9 (S-14)
    let mut rs = Ruleset::default()
        .handle_access(AccessFs::from_all(abi))?
        .handle_access(AccessNet::from_all(abi))?
        .create()?;
    for p in &m.sandbox.ro {
        rs = rs.add_rule(PathBeneath::new(PathFd::new(p)?, AccessFs::from_read(abi)))?;
    }
    for p in &m.sandbox.rw {
        rs = rs.add_rule(PathBeneath::new(PathFd::new(p)?, AccessFs::from_all(abi)))?;
    }
    for port in &m.sandbox.tcp_connect {
        rs = rs.add_rule(NetPort::new(*port, AccessNet::ConnectTcp))?;
    }
    let status = rs.restrict_self()?;
    if status.ruleset != RulesetStatus::FullyEnforced && m.risk >= Risk::Medium {
        anyhow::bail!("landlock not fully enforced: refuse {}", m.name);   // fail closed
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let m = Manifest::from_fd(3)?;     // Tool Broker truyền manifest đã kiểm chữ ký qua fd 3
    apply_landlock(&m)?;
    apply_seccomp(&m)?;                // seccompiler: chặn socket không phải TCP, syscall thừa
    Err(exec(&m.command, &m.args).into())
}
```

## 9. Giọng nói tiếng Việt

**Khuyến nghị: STT 2 lượt — Zipformer-vi (sherpa-onnx) streaming cho phản hồi tức thì, Whisper chạy lại đoạn có thuật ngữ tiếng Anh. TTS 2 tầng — Piper giọng Việt cho phản hồi nhanh, VieNeu-TTS cho đọc dài và câu trộn Anh–Việt.**

| Khâu | Khuyến nghị | Thay thế | Ghi chú |
| --- | --- | --- | --- |
| Thu âm, khử vọng | PipeWire + module echo-cancel | — | \[Unverified\] chưa tra cứu chi tiết module |
| Wake word | openWakeWord hoặc keyword spotting của sherpa-onnx | — | \[Unverified\]; cần huấn luyện từ đánh thức tiếng Việt |
| VAD | Silero VAD (ONNX) | — | phostt dùng Silero VAD cho streaming |
| STT lượt 1 (streaming) | sherpa-onnx Zipformer-vi-30M RNN-T | — | Apache 2.0, \~75 MB |
| STT lượt 2 (thuật ngữ, tên riêng) | Whisper large-v3-turbo (whisper.cpp, hoặc trên NPU qua Lemonade) | PhoWhisper | \[Unverified\] WER trên giọng người dùng |
| TTS nhanh | Piper (VITS), giọng tiếng Việt | — | ONNX, phiên âm bằng espeak-ng |
| TTS chất lượng cao | VieNeu-TTS-v2 | — | Hỗ trợ trộn Anh–Việt |

**Zipformer-vi.** Bộ trọng số Zipformer-vi-30M phát hành qua sherpa-onnx theo Apache 2.0, huấn luyện trên khoảng 70.000 giờ tiếng Việt, đạt WER khoảng 7,7% trên GigaSpeech2-vi (giọng sạch) và hỗ trợ nhận dạng từng phần theo thời gian thực ([phostt](https://pypi.org/project/phostt/)). Nhưng trên FLEURS tiếng Việt, cùng mô hình có WER 103%, vì tên nước ngoài, con số và thuật ngữ tiếng Anh bị phiên âm thành tiếng Việt ([phostt 0.4.0](https://docs.rs/crate/phostt/0.4.0/source/README.md)). Điều này không đạt yêu cầu V-3 của design.md (trộn thuật ngữ tiếng Anh). Vì vậy cần lượt 2.

\[Inference\] Cách ghép 2 lượt: Zipformer cho bản ghi tạm và phát hiện điểm dứt lời. Khi System 1 thấy bản ghi có dấu hiệu tên riêng, số, hoặc từ tiếng Anh bị phiên âm, `aios-voice` giải mã lại đoạn âm thanh đó bằng Whisper. Trên máy có NPU AMD, chạy Whisper trên NPU để không tranh GPU với LLM (§6).

**Piper.** Có bộ 14 giọng tiếng Việt fine-tune từ Piper (VITS medium) trên checkpoint `vi_VN/vais1000/medium`, xuất ONNX, chạy local trên CPU hoặc GPU, phiên âm bằng espeak-ng ([Hugging Face](https://huggingface.co/quangdung/Piper_checkpoint)).

**VieNeu-TTS-v2.** Mô hình 300M tham số dựa trên Qwen3, huấn luyện trên hơn 10.000 giờ dữ liệu song ngữ; hỗ trợ trộn Anh–Việt, 7 giọng gồm giọng Bắc và Nam, clone giọng từ 3–5 giây mẫu; chạy hoàn toàn trên CPU bằng GGUF Q4 và bộ giải mã ONNX, mỗi lần sinh mất khoảng 7 giây ([tts.ai](https://tts.ai/voices/vieneu/?lang=de)). \[Inference\] Độ trễ này quá cao cho câu trả lời đầu tiên, nhưng hợp để đọc nội dung dài.

**Giấy phép cần kiểm tra.** Bản Piper mới được nhắc với tên `piper1-gpl`, gợi ý giấy phép GPL ([Hugging Face](https://huggingface.co/quangdung/Piper_checkpoint)). Giấy phép của từng giọng, của bộ dữ liệu huấn luyện và của VieNeu-TTS: UNKNOWN. Tính năng clone giọng PHẢI tắt mặc định hoặc yêu cầu đồng ý rõ ràng.

```yaml
# /etc/aios/voice.yaml — [Unverified] tên mô hình và đường dẫn
voice:
  capture: { backend: pipewire, echo_cancel: true }
  wake_word: { engine: openwakeword, model: /var/lib/aios/voice/wake-vi.onnx }
  vad: { engine: silero, min_silence_ms: 500 }
  stt:
    pass1: { engine: sherpa-onnx, model: zipformer-vi-30m, streaming: true }
    pass2:
      engine: whisper
      model: large-v3-turbo
      device: auto            # npu > gpu > cpu
      trigger: [proper_noun, number, english_term, low_confidence]
  tts:
    fast: { engine: piper, voice: /var/lib/aios/voice/piper-vi-<giong>.onnx }
    long_form: { engine: vieneu-tts, min_chars: 400 }
```

## 10. UI: compositor, renderer, bộ gõ, portal

> **Đã thay một phần (D-17).** Shell hội thoại và bề mặt tin cậy dùng gpui-kit thay Slint ([integration.md §7](integration.md)). Smithay, fcitx5 và backend portal giữ nguyên.

**Khuyến nghị: compositor dựng bằng Smithay; shell hội thoại và bề mặt tin cậy bằng gpui-kit; fcitx5 cho bộ gõ tiếng Việt; một backend xdg-desktop-portal riêng của AiOS viết bằng Rust.**

| Thành phần | Khuyến nghị | Thay thế | Ghi chú |
| --- | --- | --- | --- |
| Compositor | Smithay | wlroots (C) | cosmic-comp và Niri đều dựng trên Smithay |
| Shell hội thoại | gpui-kit, renderer UI IR viết sẵn (D-17) | Slint + slint-interpreter, iced | Không nạp markup hay script lúc chạy |
| Bề mặt tin cậy | gpui-kit, tiến trình riêng `aios-trusted-ui` | — | Không nạp markup lúc chạy |
| First boot | `aios-compositor` + `aios-shell` ở chế độ first boot (grammar Reflex, không cần LLM) | TTY | GPUI chỉ là Wayland client nên cần compositor; \[Inference\] chế độ này dùng lại shell dự phòng |
| Recovery | TTY + `aiosctl` | — | Không phụ thuộc đồ họa |
| Bộ gõ | fcitx5 + engine tiếng Việt | IBus | Fedora có `fcitx5-unikey`; `fcitx5-bamboo` không có trong kho Fedora, AiOS tự đóng gói nếu cần (§2.1) |
| Portal | Backend riêng `aios` + xdg-desktop-portal-generic | xdg-desktop-portal-wlr | Ghép theo từng interface |

**Smithay và Slint.** Smithay cung cấp các khối dựng compositor Wayland bằng Rust ([Smithay](https://github.com/smithay/Smithay)); cosmic-comp của COSMIC dựng trên Smithay ([LinuxLinks](https://www.linuxlinks.com/cosmic-comp-wayland-compositor-cosmic/)). slint-interpreter nạp và biên dịch file .slint lúc chạy; tính năng accessibility bật mặc định; backend linuxkms vẽ toàn màn hình bằng KMS và libinput ([docs.rs slint-interpreter](https://docs.slint.dev/latest/docs/rust/slint_interpreter)). Một bản fork của iced quảng cáo cây trợ năng AccessKit đầy đủ như điểm khác với upstream ([toddy-iced](https://docs.rs/crate/toddy-iced/0.6.2)), nên iced là phương án 2.

**Bộ gõ tiếng Việt: điểm rủi ro cao nhất của tầng UI.** Trên Wayland, ứng dụng nói chuyện với compositor bằng giao thức text-input (v1, v2, v3), còn compositor nói chuyện với bộ gõ bằng giao thức input-method (v1, v2) ([fcitx5 #39](https://github.com/fcitx/fcitx5/issues/39)). GTK 3/4 hỗ trợ text-input-v3 sẵn; Qt có text-input-v3 từ 6.7 và nên dùng từ 6.8.2 ([fcitx wiki](https://fcitx-im.org/wiki/Using_Fcitx_5_on_Wayland)). Với sway, phía input-method-v2 chỉ được hỗ trợ một phần, cần thêm bản vá để hiện cửa sổ chọn từ ([fcitx wiki](https://fcitx-im.org/wiki/Translations:Using_Fcitx_5_on_Wayland/52/en)). \[Unverified\] Mức hỗ trợ text-input-v3 và input-method-v2 trong Smithay cần kiểm tra trực tiếp.

\[Inference\] Đề xuất 2 đường:

- **Thanh nhập hội thoại** (kênh nhập chính): `aios-shell` nói chuyện thẳng với fcitx5 qua D-Bus, giống im module của GTK/Qt; không phụ thuộc giao thức Wayland.
- **Ứng dụng ngoài**: compositor triển khai text-input-v3 và input-method-v2.

**Portal.** Frontend xdg-desktop-portal nhận yêu cầu từ ứng dụng rồi chuyển cho backend; backend khai báo trong `/usr/share/xdg-desktop-portal/portals/*.portal`, và frontend chọn backend theo file `<DE>-portals.conf` ứng với `XDG_CURRENT_DESKTOP`, có thể chỉ định riêng từng interface ([ArchWiki](https://wiki.archlinux.org/title/Xdg-desktop-portal)). xdg-desktop-portal-generic là backend Rust cho các compositor không có backend riêng ([docs.rs](https://docs.rs/xdg-desktop-portal-generic)). Lưu ý: xdg-desktop-portal-cosmic theo giấy phép GPL-3.0-only ([Arch package](https://archlinux.org/packages/extra/x86_64/xdg-desktop-portal-cosmic)); chỉ tham khảo, không chép mã nếu AiOS không theo GPL.

```ini
# /usr/share/xdg-desktop-portal/AiOS-portals.conf   (XDG_CURRENT_DESKTOP=AiOS)
[preferred]
default=aios
org.freedesktop.impl.portal.ScreenCast=generic
org.freedesktop.impl.portal.Screenshot=generic
```

```rust
// aios-portal/src/file_chooser.rs — hộp thoại chọn file hiện thành widget trong hội thoại
// [Unverified] chữ ký method theo spec impl.portal.FileChooser và API zbus hiện hành
struct FileChooser { conv: ConversationHandle }

#[zbus::interface(name = "org.freedesktop.impl.portal.FileChooser")]
impl FileChooser {
    async fn open_file(&self, _handle: OwnedObjectPath, app_id: String, _parent: String,
                       title: String, options: HashMap<String, OwnedValue>)
        -> (u32, HashMap<String, OwnedValue>) {
        match self.conv.pick_files(&app_id, &title, &options).await {
            Ok(uris) => (0, HashMap::from([("uris".into(), to_value(uris))])),
            Err(_) => (1, HashMap::new()),   // 1 = người dùng hủy
        }
    }
}
```

## 11. Khám phá phần cứng

**Khuyến nghị: chuyển phần nhận uevent và lấy vân tay thiết bị sang Rust trong `aiosd-core` (crate `udev`); dùng `nusb` cho driver user space; firmware qua fwupd.**

| Thành phần | Khuyến nghị | Thay thế | Ghi chú |
| --- | --- | --- | --- |
| Nhận uevent, đọc thuộc tính | crate `udev` / `tokio-udev` | pyudev (code mẫu trong [design.md](design.md)) | \[Unverified\] chưa tra cứu crate trong đợt này |
| Tra tên vendor/model | cơ sở dữ liệu hwdata của udev | — | Thuộc tính `ID_VENDOR_FROM_DATABASE` |
| Driver USB user space | `nusb` | rusb (libusb) | Thuần Rust, MIT hoặc Apache-2.0 |
| HID | `hidraw` qua udev | hidapi | — |
| Firmware | fwupd (D-Bus) | — | \[Unverified\] chi tiết API |
| Profile tăng tốc cho Model Gateway | Phát hiện driver `amdxdna`, GPU qua DRM | — | Nối với `hw-profiles.yaml` ở §6 |

**nusb.** Đây là thư viện thuần Rust truy cập thiết bị USB ở mức thấp, không phụ thuộc libusb hay thư viện C nào, ưu tiên async nhưng không bắt buộc async runtime; có API liệt kê và theo dõi thiết bị, đọc descriptor, mở interface và truyền dữ liệu control, bulk, interrupt ([nusb](https://github.com/kevinmehall/nusb/wiki)). Trên Linux, nusb dựa trên API usbfs của kernel, nên chỉ cần udev rule cấp quyền device node ([docs.rs nusb](https://docs.rs/nusb/=0.2.0-beta.2)). Thư viện được probe-rs và một số dự án khác dùng, nhưng tác giả vẫn mô tả là còn khá mới ([rust-digger](https://rust-digger.code-maven.com/crates/nusb)).

\[Inference\] nusb hợp với driver user space do System 2 sinh (§10.3 design.md): driver chạy trong sandbox §8, chỉ được cấp quyền tới đúng một device node qua udev rule tạm thời.

```rust
// aiosd-core/src/hw/usb.rs — theo dõi thiết bị USB cắm vào
// [Unverified] API hotplug theo phiên bản nusb 0.2.x
use futures_lite::StreamExt;
use nusb::hotplug::HotplugEvent;

pub async fn watch_usb(bus: EventBus) -> anyhow::Result<()> {
    let mut events = nusb::watch_devices()?;
    while let Some(ev) = events.next().await {
        if let HotplugEvent::Connected(info) = ev {
            bus.publish(Event::DeviceAdded(Fingerprint {
                subsystem: "usb".into(),
                vendor_id: info.vendor_id(),
                product_id: info.product_id(),
                class: info.class(),
                product: info.product_string().map(str::to_owned),
            }));
        }
    }
    Ok(())
}
```

## 12. Bảo mật, quan sát, kiểm thử

**Khuyến nghị: SELinux (mặc định của Fedora) giam `aiosd`; Secure Boot và chỉ nạp module đã ký; `tracing` + OpenTelemetry cho quan sát; virtme-ng để test BPF trên nhiều kernel; `cedar-policy-symcc` để kiểm chứng chính sách.**

| Mảng | Khuyến nghị | Ghi chú |
| --- | --- | --- |
| Giam daemon | SELinux policy riêng cho `aiosd-*` | \[Inference\] đi cùng distro nền Fedora (§2) |
| Thao tác đặc quyền | `aios-actuator` + polkit | Theo §11.1 design.md |
| Toàn vẹn boot | Secure Boot, chỉ module kernel đã ký | Khớp quy tắc "không tự nạp module" (§10 design.md) |
| Ký image và manifest tool | Ký OCI image của bootc; ký manifest tool bằng ed25519 | \[Unverified\] chọn cosign/sigstore sau |
| Khóa bí mật | Keyring (Secret Service) + TPM2 | \[Unverified\] chưa tra cứu triển khai cụ thể |
| Trace, metric | `tracing` (Rust), OpenTelemetry (Rust, Python), journald | \[Unverified\] phiên bản SDK |
| Test kernel và BPF | virtme-ng | Boot kernel mới trong VM rất nhanh |
| Test agent phá hủy | QEMU + snapshot | Theo §12.2 design.md |
| Kiểm chứng chính sách | `cedar-policy-symcc` | §4 |
| Eval | Bộ kịch bản vàng tiếng Việt, VMLU (§6), VN-MTEB (§7) | — |

**virtme-ng.** Công cụ này boot một kernel mới trong máy ảo QEMU/KVM rất nhanh: với kiểu máy microvm và tiến trình init viết bằng Rust, thời gian boot giảm xuống 1,2 giây; đầu vào và đầu ra được nối sẵn để chạy cùng một lệnh trên nhiều kernel liên tiếp ([LWN](https://lwn.net/Articles/950830)). Mutter đã dùng virtme-ng trong quy trình test ([LWN](https://lwn.net/Articles/951313)). \[Inference\] AiOS dùng nó để chạy test BPF LSM trên cả 6.18 LTS và nhánh 7.x (cần cho NPU AMD, §6) trong CI.

```yaml
# .github/workflows/bpf-lsm.yml — [Unverified] tên gói và cờ của vng
jobs:
  bpf-lsm:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        kernel: ["fedora-current", "fedora-next"]   # kernel của bản Fedora hiện hành và bản kế tiếp (rawhide)
    steps:
      - uses: actions/checkout@v4
      - run: sudo apt-get install -y qemu-system-x86 virtme-ng
      - run: ./ci/fetch-kernel.sh ${{ matrix.kernel }}   # kernel build sẵn với CONFIG_BPF_LSM=y
      - run: >
          vng --run ./kernels/${{ matrix.kernel }}/bzImage
          --append "lsm=lockdown,capability,landlock,yama,selinux,bpf"   # [Unverified] giữ đúng danh sách LSM của image sản phẩm
          -- cargo test -p aiosd-core --test lsm_guard
```

## 13. Rủi ro của stack và việc cần kiểm chứng sớm

**Đợt tra cứu này phát hiện 3 rủi ro chưa có trong design.md: xung đột phiên bản kernel khi dùng NPU AMD, STT tiếng Việt hỏng với thuật ngữ tiếng Anh, và bộ gõ tiếng Việt trên compositor tự xây.**

| Rủi ro | Bằng chứng | Tác động | Hướng xử lý |
| --- | --- | --- | --- |
| Kernel Fedora đổi phiên bản mỗi 6–8 tuần | §2.1 | API BPF hoặc hook LSM đổi, `agent_guard` hỏng | CO-RE; CI chạy S-14 trên kernel hiện hành và kế tiếp; ghim digest ảnh |
| STT tiếng Việt sai với tên riêng, số, từ tiếng Anh | Zipformer-vi: WER 103% trên FLEURS (§9) | Hỏng lệnh giọng nói có thuật ngữ | STT 2 lượt; đo trên giọng người dùng |
| Bộ gõ tiếng Việt trên Smithay chưa kiểm chứng | Hỗ trợ input-method-v2 ở compositor khác còn thiếu (§10) | Không gõ được tiếng Việt ở kênh nhập chính | Shell nói thẳng với fcitx5 qua D-Bus; spike sớm |
| Thư viện chưa 1.0 | sqlite-vec 0.1.x, nusb "còn mới", bootc (§2, §7, §11) | API đổi, phải sửa code | Ghim phiên bản, bọc sau interface |
| Thành phần độc quyền | Kernel NPU của FastFlowLM là binary (§6) | Khó kiểm toán, phụ thuộc nhà cung cấp | Giữ NPU là tùy chọn; luôn có đường Vulkan/CPU |
| Lỗi Vulkan trên iGPU Intel | Báo lỗi trên Arrow Lake (§6) | Model Gateway treo hoặc lỗi | Đường dự phòng CPU, health check |
| Giấy phép | Piper (`piper1-gpl`), VieNeu-TTS, xdg-desktop-portal-cosmic GPL-3.0-only; oh-my-openagent SUL-1.0 (không đóng gói, D-20) | Ràng buộc phân phối | Rà soát pháp lý trước khi đóng M0 (S-16) |
| Engine L2 chạy lại một bước (replay, retry) | §5 | Thao tác bị thực hiện 2 lần | Idempotency key cho mọi lệnh gọi tool |

### Các spike kiểm chứng

Mã spike dùng chung một dãy S với [integration.md §11.3](integration.md); bảng đầy đủ nằm ở đó.

| Mã | Spike | Ghi chú |
| --- | --- | --- |
| S-3 | **Bộ gõ.** Compositor Smithay tối thiểu + ô nhập gpui-kit + fcitx5 với kiểu gõ Telex. Đạt khi gõ được tiếng Việt có dấu trong thanh nhập hội thoại, trong một ứng dụng GTK4 và một ứng dụng Qt6 | Không chặn lộ trình; chủ dự án cung cấp cách hiện thực sau |
| S-12 | **STT.** Ghi khoảng 30 phút giọng người dùng mục tiêu có trộn thuật ngữ tiếng Anh. So WER của Zipformer-vi một lượt với Zipformer + Whisper lượt 2. Ngưỡng đạt: UNKNOWN, chủ dự án đặt | |
| S-13 | **Mô hình System 1.** Chạy VMLU và bộ kịch bản vàng trên Qwen3.5-4B và Gemma 3 4B qua llama.cpp Vulkan, trên phần cứng mục tiêu; ghi cả độ trễ | |
| S-14 | **Kernel.** Chạy `agent_guard` (mọi hook ở design §3.4) và `aios-sandbox-exec` (Landlock) trên kernel Fedora hiện hành và kernel kế tiếp bằng virtme-ng, với danh sách LSM giống image sản phẩm | |
| S-11 | **Renderer UI IR.** Đo thời gian render surface bằng gpui-kit theo số node. Đạt khi nằm trong ngân sách L1 của design §7.4 | Thay spike Slint interpreter |
| S-15 | **Portal.** Ứng dụng GTK4 và Qt6 mở hộp thoại chọn file qua backend `aios` | |
| S-16 | **Giấy phép.** Rà soát toàn bộ bảng §1 và các thành phần ở integration.md §1 | |
