# AiOS — Bàn giao công việc

_Cập nhật: 2026-10-21. Người bàn giao: Claude (leader M0, phiên với chủ dự án nqthiep)._

**Tài liệu này dành cho agent tiếp nhận dự án. Đọc hết file này trước khi làm bất cứ việc gì. Mốc M0 (nền móng) đã hiện thực xong và cổng nghiệm thu M0 đạt trong VM `aios-dev` (docs/spikes/S-9-baseline-m0.md). Còn 2 spike của M0 chờ môi trường (S-17, S-24). Việc tiếp theo là M1.**

## 1. Dự án là gì

AiOS là hệ điều hành mã nguồn mở (Apache-2.0) dựa trên Fedora (ảnh bootc), lấy hội thoại làm màn hình chính, phản hồi qua 3 cấp (code tất định, mô hình nhỏ local, mô hình lớn), tự học, tự bảo vệ, nhiều người dùng, chạy được với AI local lẫn cloud. Nguyên tắc gốc: **mô hình AI luôn bị coi là không tin cậy; quyền hành động nằm ở lõi tất định (Tool Broker, Policy Engine Cedar, token xác nhận ký, sandbox, BPF LSM).**

Đọc theo thứ tự:

| # | File | Đọc để biết |
| --- | --- | --- |
| 1 | [docs/overview.md](docs/overview.md) | Tổng quan một trang, bản đồ tài liệu, thuật ngữ |
| 2 | [docs/decisions.md](docs/decisions.md) | 53 quyết định và trạng thái (đã chốt, đề xuất, thay thế) |
| 3 | [docs/roadmap.md](docs/roadmap.md) | MVP (M1), mốc M0–M5 + R, cổng nghiệm thu, spike S-1 → S-25, quy tắc triển khai bằng coding agent (§6) |
| 4 | [docs/m0/README.md](docs/m0/README.md) | Quy ước chung và giao diện giữa các task M0 |
| 5 | [docs/design.md](docs/design.md) | Thiết kế chi tiết từng tầng (§1–§18) |
| 6 | [docs/integration.md](docs/integration.md) | Cách ghép các dự án mã nguồn mở, bảng cổng và socket (§11.1) |
| 7 | [docs/tech-stack.md](docs/tech-stack.md) | Thư viện và công cụ cụ thể; so sánh Debian/Fedora (§2.1) |
| 8 | [docs/review.md](docs/review.md) | Phản biện toàn bộ thiết kế và trạng thái xử lý (§8) |
| 9 | [docs/requirements.md](docs/requirements.md) | 192 yêu cầu PHẢI/KHÔNG ĐƯỢC/NÊN/CÓ THỂ, trích tự động |

## 2. Trạng thái hiện tại

| Mục | Trạng thái |
| --- | --- |
| Thiết kế | Đủ cho M0–M1. Quyết định ở decisions.md (D-01 → D-53) |
| Mã nguồn | M0 xong trên `main` local: 9 crate Rust (`aios-types`, `aios-audit`, `aios-registry`, `aios-policy`, `aios-confirm`, `aios-broker`, `aiosd-core`, `aios-model-gateway`, `aios-sandbox-exec`), `python/aiosd-cognition` (`aiosctl`), 2 plugin tool (`plugins/tools/{sys,fs}`), chính sách Cedar `policy/`, ảnh `image/` |
| Cổng M0 | Đạt 8/8 bước, 2 lần liên tiếp, bằng `dev/vm/gate/{setup,run}.sh` trong VM |
| Spike có báo cáo | `docs/spikes/`: S-4 (Hindsight), S-9 (baseline M0), S-13 (đo nhanh mô hình local), S-14 (Landlock), S-16 (giấy phép) |
| Chưa làm | S-17, S-24 (chờ môi trường), S-13 đầy đủ (chờ phần cứng mục tiêu và khóa cloud); ảnh `aios:m0` chưa boot |
| Kiểm tra | `make check` (fmt, clippy, test Rust gồm test bất biến Cedar và vector token, ruff + pytest, requirements); `make licenses` |
| Repo | github.com/nqthiep/AiOS (public), nhánh `main`. **Các commit M0 chưa được push** (chỉ push khi chủ dự án yêu cầu) |
| Môi trường | Mac (Apple M5, 32 GB) + VM `aios-dev` (Fedora 44, kernel 7.2.11) |

## 3. Quyết định then chốt cần nhớ (chi tiết ở decisions.md)

| Mã | Nội dung |
| --- | --- |
| D-01 | LLM chạy ở user space; kernel chỉ chạy eBPF và nhận chính sách |
| D-16, D-17, D-35 | Desktop environment tự viết (Smithay + gpui-kit), nhưng M1–M2 chạy app chat trên compositor có sẵn (niri hoặc cosmic-comp) |
| D-34 | Engine System 2 của MVP là vòng lặp tự viết; opencode chỉ vào từ M4 |
| D-36 | Hindsight làm bộ nhớ ngay từ M1 |
| D-37, D-43 | L2 dùng cloud cho dữ liệu public/personal; cloud do từng người dùng tự cấu hình, ngân sách riêng, mặc định 0 |
| D-38 | Lộ trình M0–M5 + R (đã chốt) |
| D-42 | Nhiều người dùng, độc lập nhau |
| D-47 | Fedora bootc; kernel theo Fedora (hiện 7.2), không dùng LTS |
| D-48 | Mã nguồn mở Apache-2.0; mã BPF giấy phép kép GPL-2.0/BSD |
| D-50 | Dự án được hiện thực bằng coding agent; chủ dự án duyệt |
| D-51 | Bộ gõ tiếng Việt tạm bỏ qua ở M1 |
| D-52, D-53 | Ngưỡng M1: pass@1 ≥ 80%, hoàn tác ≤ 5%, L1 p95 ≤ 1,5 s, RAM khi rảnh ≤ 8 GB; phần cứng mục tiêu: laptop x86_64, 16 GB, iGPU Vulkan |
| D-21 → D-33, D-41, D-45, D-46 | Nguyên tắc an ninh đã chốt: 2 cửa bắt buộc, plugin, nhãn theo session, quyền từ registry ký, token gắn hành động, taint, kênh từ xa, Cedar 2 tập, kill switch độc lập, AI an ninh chỉ đề xuất, chuỗi cung ứng, proxy gói |

## 4. Việc tiếp theo, theo thứ tự

1. **Hỏi chủ dự án có push các commit M0 lên remote không** (repo public).
2. **Các việc còn hở từ M0, nên làm đầu M1** (chi tiết ở docs/spikes/S-9-baseline-m0.md §5 và cột ghi chú của bảng task):
   - `aiosd-core` chạy dưới dạng unit user vì Executor dùng `systemd-run --user`, trong khi ảnh khai nó là dịch vụ hệ thống: chốt mô hình chạy theo từng người dùng (design §18).
   - Kill switch: `aiosctl freeze` không thu hồi khóa ảo; `aios-killswitch.service` đóng băng nhầm cấp.
   - Kênh (`channel`) do client tự khai khi mở session với Broker.
   - Tường lửa IP của systemd không có hiệu lực với unit user (S-13-m0): mô hình local chạy unit user vẫn ra mạng được.
   - Khóa ảo của Gateway chỉ nằm trong bộ nhớ.
   - seccomp là danh sách chặn; chưa xét ABI x32 trên x86_64.
   - `psycopg2-binary` (LGPL) của Hindsight; 16 gói firmware không nguồn mở trong ảnh (chủ dự án quyết, D-48).
3. **S-17, S-24** khi có môi trường (bảng task: `s17`, `s24`).
4. **M1** theo roadmap.md §2.

## 5. Bảng task M0

Bảng task nằm ở [.tasks/board.json](.tasks/board.json), được commit cùng repo. Xem bảng:

```bash
~/.claude/skills/opencode-lead/oc-board.py show
```

Khi điều phối, commit `.tasks/board.json` sau mỗi lần bảng đổi để agent khác tiếp nhận được. File khóa `.tasks/.board.lock` không được commit.

Bản chụp (2026-10-21):

**Nhiệm vụ:** Hiện thực mốc M0 của AiOS (roadmap.md §3): aiosd-core khung, Model Gateway, Hindsight, aiosctl; cổng: aiosctl ask chạy hết một yêu cầu với 3 tool
| Đợt | ID | Ưu tiên | Việc | Quan hệ | Giao cho | Trạng thái |
|---|---|---|---|---|---|---|
| 1 | m0 | P0 | M0 · Nền móng (vỏ gom nhóm) | - | leader | 13/16 con xong |
| 1 | ld-setup | P0 | └ Leader: dựng workspace Rust + Python, quy ước, make check, AGENTS.md cho worker | →ld-secspec,policy,confirm,audit,registry,tools3,gateway,image,sandbox | leader | đạt |
| 1 | ld-vm | P0 | └ Leader: cài toolchain vào VM aios-dev (Rust, clang, libbpf, bpftool, uv, Postgres+pgvector, llama.cpp) | →llm,hindsight,sandbox | leader | đạt |
| 1 | core | P1 | └ aiosd-core (vỏ gom nhóm) | - | leader | 9/9 con xong |
| 1 | audit-fix | P0 | └ └ Sửa split_last_lines của aios-audit (log 2–3 dòng không mở lại được) | - | leader | đạt — phát hiện bởi cognition; test reopening_after_any_line_count |
| 2 | audit | P1 | └ └ aios-audit: audit log JSONL append-only có hash chain + lệnh verify | ←ld-setup✓ →broker | worker | đạt |
| 2 | registry | P1 | └ └ aios-registry: đọc plugin.toml, kiểm chữ ký .sig, mức rủi ro hiệu lực = max(khai báo, suy ra) | ←ld-setup✓ →broker | worker | đạt |
| 2 | sandbox | P1 | └ └ aios-sandbox-exec: bubblewrap + Landlock ABI V6 fail-closed + seccomp chặn UDP/raw; test trong VM | ←ld-setup✓,ld-vm✓ →executor,s14 ~s14 | leader | đạt — seccomp là denylist: x32 ABI trên x86_64 chưa xét; Landlock chưa cấp /dev/null, /proc, /etc/pki |
| 3 | policy | P0 | └ └ aios-policy: nạp auto/confirmable/forbid Cedar, tính context, trả Allow/Confirm/Deny; qua test bất biến | ←ld-setup✓,ld-secspec✓ →broker | leader | đạt |
| 3 | confirm | P0 | └ └ aios-confirm: ConfirmClaims, chuẩn hóa NFC+JCS, ký/kiểm Ed25519, nonce một lần, hạn, token theo lô | ←ld-setup✓,ld-secspec✓ →broker | worker@glm-5.3 | đạt |
| 4 | sandbox-fix | P0 | └ └ Sửa L-1/L-2 của S-14: quyền Landlock cho file, nâng ABI V9 (RESOLVE_UNIX) | ←s14✓ →executor | leader | đạt — L-3 (fd kế thừa) giao cho executor: đóng fd trước khi gọi helper |
| 4 | broker | P1 | └ └ aios-broker: MCP endpoint aios-tools, gọi policy/confirm/audit/registry, idempotency key, nhãn session | ←policy✓,confirm✓,audit✓,registry✓ →cognition,executor | leader | đạt — channel tự khai trong /v1/session (M1 cần xác minh); quiet_hours bỏ qua; rmcp 3.6.0 |
| 5 | executor | P1 | └ └ Broker executor: chạy tool process/mcp qua aios-sandbox-exec, cgroup tạm dưới aios-agent.slice | ←broker✓,sandbox✓,sandbox-fix✓ →ld-gate | leader | đạt — L-3: từ chối chạy khi có fd không CLOEXEC; systemd-run --user chưa hợp với aiosd-core là system service (M1) |
| 1 | s17 | P1 | └ Spike S-17: aios-trusted-ui qua ext-session-lock-v1 trên niri và cosmic-comp | - | leader | sẵn sàng — chờ môi trường: phiên đồ họa |
| 1 | image-deps | P2 | └ Ảnh: cài phụ thuộc Python của aiosd-cognition theo uv.lock (đang --no-deps), bỏ qemu-user-static | - | leader | đạt — qemu-user-static có sẵn trong ảnh nền fedora-bootc: để M1 quyết gỡ |
| 1 | s24 | P2 | └ Spike S-24: systemd-sysext trên ảnh bootc + composefs | - | leader | sẵn sàng — chờ môi trường: boot ảnh aios:m0 |
| 2 | ld-secspec | P0 | └ Leader: viết trước test bất biến Cedar (6 bất biến) và test vector token xác nhận | ←ld-setup✓ →policy,confirm | leader | đạt |
| 2 | gateway | P0 | └ aios-model-gateway: /v1/chat/completions + /v1/embeddings, khóa ảo, nhãn session, secret không cloud, ngân sách theo người dùng | ←ld-setup✓ →hindsight,cognition | leader | đạt — budget check+charge không nguyên tử (ponytail); cache_prompt chưa kiểm với llama.cpp F44 |
| 2 | tools3 | P1 | └ 3 tool cho cổng M0: sys.info, fs.search (low), fs.write_note (medium, có undo) | ←ld-setup✓ →cognition | worker | đạt |
| 2 | llm | P1 | └ llama.cpp trong VM: unit systemd, tải mô hình nhỏ có tool calling + mô hình embedding, ghim sha256 vào models.lock | ←ld-vm✓ →hindsight | leader | đạt — IPAddressDeny không hiệu lực ở unit user: chấp nhận cho M0, chuyển sang system unit ở M1 |
| 2 | image | P2 | └ Containerfile ảnh AiOS: Fedora bootc ghim digest, đổi thương hiệu, unit systemd; build + bootc container lint | ←ld-setup✓ →s16 | leader | đạt — killswitch freeze aios-agent.slice ở system manager, nhưng slice là user unit: sửa ở M1 (per-user) |
| 3 | hindsight | P2 | └ Spike S-4: Postgres+pgvector+hindsight-api-slim trong VM, LLM/embedding/reranker qua Gateway, không torch, đo RSS | ←ld-vm✓,gateway✓,llm✓ →ld-gate | leader | đạt — probe đạt (sub-agent, ~20 phút; leader chỉ kiểm lại service, no-torch, MemoryCurrent); psycopg2-binary LGPL; khóa Gateway mất khi restart; gateway.yaml dev ghi đè /etc/aios |
| 3 | s14 | P2 | └ Spike S-14 (phần Landlock): aios-sandbox-exec trên kernel 7.2 của VM, báo cáo ABI và hành vi | ←sandbox✓ →ld-gate,sandbox-fix | leader | đạt — phát hiện L-1 (file trong profile → fail closed), L-2 (allow_unix_socket không thực thi dưới V6): task sandbox-fix |
| 5 | cognition | P1 | └ aiosd-cognition tối thiểu + aiosctl: ask (grammar Reflex + vòng L2), status, audit verify, freeze | ←broker✓,gateway✓,tools3✓ →s16,ld-gate | leader | đạt — freeze chưa revoke-all (aiosd-core không có endpoint; thuộc killswitch); file pending 0644 trong thư mục 0711 lúc ký |
| 6 | ld-gate | P0 | └ Leader: tích hợp, chạy cổng M0 trong VM (aiosctl ask + 3 tool, tắt aiosd vẫn chạy), đo S-9 baseline | ←executor✓,cognition✓,hindsight✓,s14✓ | leader | đạt — cổng đạt 8/8, lần chạy 3 và 4 (docs/spikes/S-9-baseline-m0.md) |
| 6 | s16 | P2 | └ Spike S-16: bảng giấy phép mọi phụ thuộc (cargo-deny, pip-licenses) so với Apache-2.0 | ←cognition✓,image✓ | leader | đạt — 16 gói firmware không nguồn mở trong ảnh: chủ dự án quyết; ảnh cài wheel --no-deps nên thiếu mcp; qemu-user-static trong ảnh |

Tổng 26 · đạt 22 · đang làm 0 · sẵn sàng 2 · bị chặn 0 · huỷ 0
Quan hệ: ←phụ thuộc (✓ đã xong) · →chặn · ~liên quan · └ task con. Đợt = thứ tự thực hiện, cùng đợt chạy song song được.

M0 được làm theo skill `opencode-lead`: worker OpenCode làm đợt đầu (audit, registry, tools3, confirm); khi quota OpenCode Go hết, chủ dự án chọn để leader (Claude) làm nốt, dùng sub-agent Claude trong worktree `../wt-<id>`. Mọi task đều được leader nghiệm thu bằng cách tự chạy lại lệnh nghiệm thu trước khi merge.

## 6. Quy tắc làm việc với chủ dự án

| Quy tắc | Chi tiết |
| --- | --- |
| Ngôn ngữ | Trả lời và viết tài liệu bằng tiếng Việt; mã, comment, thông báo lỗi trong mã bằng tiếng Anh |
| Nhãn | Không trình bày suy luận như sự thật. Gắn `[Inference]`, `[Speculation]`, `[Unverified]` ở đầu câu khi chưa kiểm chứng; dùng `UNKNOWN` khi thiếu dữ liệu từ chủ dự án. Sai thì ghi "Correction: ..." |
| Tài liệu gốc trước | Sửa yêu cầu ở tài liệu gốc (design, integration, roadmap, tech-stack), rồi chạy lại `python3 -I scripts/extract_requirements.py`; `--check` phải đạt trước khi commit |
| Quyết định | Quyết định mới ghi vào decisions.md với mã D-xx, trạng thái và ngày; chỉ ghi "Đã chốt" khi chủ dự án đồng ý rõ ràng |
| Giao worker OpenCode | Hỏi chủ dự án trước khi giao; một lần đồng ý áp dụng cho cả kế hoạch trong bảng. Không giao việc thiết kế hay việc nhạy cảm an ninh (policy, token, nhãn) mà không có test do leader viết trước. Quy trình: skill `opencode-lead` (`~/.claude/skills/opencode-lead/`) |
| Thư viện | Tra tài liệu thư viện mới nhất (Context7 hoặc trang chính thức) trước khi dùng API; ghi phiên bản đã kiểm |
| Git | Chỉ commit khi được yêu cầu hoặc khi đã được giao quyền cho kế hoạch; không force push; không push nhánh task của worker |
| Bí mật | Không nhập hay lưu khóa API, mật khẩu thay chủ dự án; chủ dự án sẽ tự cung cấp khóa cloud sau |

## 7. Môi trường

**Mac** (máy của chủ dự án): Homebrew, Docker Desktop, `uv`, Python 3.14, Rust 1.99.1 (`rust-toolchain.toml`), `cargo-deny` 0.20.2, `opencode` v2.0.32 (bỏ cờ `--dir`; `oc-task.sh` đã được sửa để chạy từ trong worktree), `gh` đã đăng nhập tài khoản `nqthiep`.

**VM `aios-dev`** (Lima 2.2.1, cấu hình [dev/lima/aios-dev.yaml](dev/lima/aios-dev.yaml)):

| Mục | Giá trị |
| --- | --- |
| Hệ điều hành | Fedora 44 aarch64, kernel 7.2.11-200.fc44 |
| An ninh | SELinux Enforcing; LSM `lockdown,capability,yama,selinux,bpf,landlock,ipe,ima,evm`; Landlock ABI 10 |
| Tài nguyên | 4 CPU, 8 GB RAM, ổ 20 GB |
| Chia sẻ | Chỉ `/Users/quangthiep/Projects/AiOS` (ghi được); home của Mac không được chia sẻ |

```bash
limactl shell aios-dev
```

```bash
limactl start --name=aios-dev dev/lima/aios-dev.yaml
```

## 8. Điểm dễ vấp

- **Worktree của worker nằm ngoài thư mục chia sẻ vào VM** (`../wt-<id>`). Test cần Linux phải chạy qua `dev/vm-run.sh` (chép bản hiện tại vào VM), không qua đường dẫn chia sẻ. Không mở rộng phần chia sẻ: chủ dự án chỉ cho chia sẻ thư mục AiOS.
- **VM là arm64**, phần cứng mục tiêu là x86_64: spike NPU, Vulkan trên iGPU, compositor trên GPU thật phải chờ laptop x86_64.
- **Kernel ảnh cloud Fedora 44 là 6.19**; đã nâng lên 7.2.11. Tạo lại VM thì nhớ `sudo dnf upgrade "kernel*"` rồi khởi động lại.
- **Lima cho user mặc định sudo không cần mật khẩu**: bộ ký xác nhận dev của M0 chỉ tách quyền, không chứng minh người dùng có mặt (docs/m0/confirm.md).
- **Ổ đĩa Mac** từng chỉ còn khoảng 16 GB trống (lần đo sau còn 39 GB); kiểm `df -h /` trước khi tải mô hình hay build ảnh.
- **Repo public**: mọi thứ push lên đều công khai. Không commit khóa, file `.env`, khóa ký dev.
- **Tài liệu cũ không thuộc AiOS**: `docs/design/thiet-ke-cong-ty-ai-native.md` và `docs/critique/phan-bien-rev12.md` là tài liệu CompanyOS có sẵn trên remote, được giữ nguyên; không sửa, không trích yêu cầu từ đó.
- **Dựng lại M0 trong VM**: `dev/vm-run.sh 'bash dev/vm/gate/setup.sh && bash dev/vm/gate/run.sh'`. `setup.sh` khởi động lại Gateway và cấp lại khóa cho Hindsight (khóa ảo chỉ nằm trong bộ nhớ). Trạng thái cổng ở `/var/lib/aios/gate/`, output ở `~/gate-out/` trong VM.
- **Mô hình 4B trên CPU chậm**: một lượt L2 của cổng mất 80–120 s; `retain` của Hindsight mất 26–59 s mỗi câu.
- **Thư mục tạm dưới `/tmp` trong sandbox**: Landlock cấp quyền ghi `/tmp`, nên test hay fixture đặt dưới `/tmp` sẽ thấy quyền rộng hơn thực tế; dùng `/var/tmp`.
- **llama.cpp đổi schema tool thành grammar**: regex `\p{...}` trong `pattern` làm `llama-server` trả HTTP 400; `aiosctl` bỏ `pattern` (chuỗi) khỏi schema gửi mô hình.
- **Mã yêu cầu REQ-... đổi số** khi tài liệu gốc thêm hoặc bớt câu; khi trích dẫn, kèm mục thiết kế gốc.

## 9. Việc chờ chủ dự án

| Việc | Ghi chú |
| --- | --- |
| Push các commit M0 lên remote | Repo public; chưa push |
| Firmware không nguồn mở trong ảnh | 16 gói `LicenseRef-Fedora-Firmware` (docs/spikes/S-16-licenses.md); giữ hay bỏ trong ảnh phát hành (D-48) |
| Khóa API cloud và ngân sách thử nghiệm | Cho S-1, S-13, S-18, S-25; chủ dự án tự nhập |
| Laptop x86_64 | Cho spike phần cứng và S-13 đầy đủ |
| Cách hiện thực bộ gõ tiếng Việt | Chủ dự án sẽ cung cấp sau (D-51) |
| niri hay cosmic-comp | Quyết sau spike S-17 |
