# AiOS — Bàn giao công việc

_Cập nhật: 2026-10-20. Người bàn giao: Claude (phiên thiết kế với chủ dự án nqthiep)._

**Tài liệu này dành cho agent tiếp nhận dự án. Đọc hết file này trước khi làm bất cứ việc gì. Dự án đang ở cuối giai đoạn thiết kế: bộ tài liệu thiết kế đã đủ và được chủ dự án chốt phần lớn; mốc M0 (nền móng) đã được chia thành 19 task có thiết kế chi tiết; chưa có dòng mã nào và chưa giao task nào.**

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
| Thiết kế | Đủ cho M0–M1. 40 quyết định đã chốt; 6 đề xuất chờ kết quả spike (D-13, D-14, D-19, D-27, D-30, D-44); 2 thay thế (D-12, D-23) |
| Mã nguồn | Chưa có |
| Kế hoạch M0 | 21 task (19 task thực hiện + 2 vỏ nhóm), thiết kế chi tiết ở [docs/m0/](docs/m0/README.md) |
| Giao việc | Chưa giao task nào; chủ dự án đã yêu cầu chia task để giao worker, **chưa xác nhận bắt đầu giao** |
| Repo | github.com/nqthiep/AiOS (public), nhánh `main` |
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

1. **Hỏi chủ dự án xác nhận bắt đầu M0** (nếu chưa có). Việc giao worker OpenCode tốn quota OpenCode Go; theo quy tắc của chủ dự án, phải hỏi trước (§6).
2. **ld-setup** (leader): dựng Cargo workspace, crate `aios-types`, project Python, `Makefile`, `dev/vm-run.sh`, `AGENTS.md`. Trên Mac, Rust chưa có toolchain mặc định: chạy `rustup default stable` trước. Thiết kế: [docs/m0/ld-setup.md](docs/m0/ld-setup.md).
3. **ld-vm** (leader): `dev/vm/setup.sh` cài công cụ vào VM. Thiết kế: [docs/m0/ld-vm.md](docs/m0/ld-vm.md).
4. **ld-secspec** (leader, không giao worker vì nhạy cảm an ninh): chính sách Cedar, test bất biến duyệt 3.072 tổ hợp, test vector token. Thiết kế: [docs/m0/ld-secspec.md](docs/m0/ld-secspec.md).
5. Giao worker đợt 2 song song: audit, registry, sandbox, gateway, tools3, llm, image; rồi các đợt sau theo bảng (§5).
6. **ld-gate** (leader): chạy cổng M0 trong VM, ghi baseline S-9.

## 5. Bảng task M0

Bảng task nằm ở [.tasks/board.json](.tasks/board.json), được commit cùng repo. Xem bảng:

```bash
~/.claude/skills/opencode-lead/oc-board.py show
```

Khi điều phối, commit `.tasks/board.json` sau mỗi lần bảng đổi để agent khác tiếp nhận được. File khóa `.tasks/.board.lock` không được commit.

Bản chụp:

| Đợt | ID | Ưu tiên | Việc | Quan hệ | Giao cho | Trạng thái |
|---|---|---|---|---|---|---|
| 1 | m0 | P0 | M0 · Nền móng (vỏ gom nhóm) | - | leader | 0/13 con xong |
| 1 | ld-setup | P0 | └ Leader: dựng workspace Rust + Python, quy ước, make check, AGENTS.md cho worker | →ld-secspec,policy,confirm,audit,registry,tools3,gateway,image,sandbox | leader | sẵn sàng |
| 1 | ld-vm | P0 | └ Leader: cài toolchain vào VM aios-dev (Rust, clang, libbpf, bpftool, uv, Postgres+pgvector, llama.cpp) | →llm,hindsight,sandbox | leader | sẵn sàng |
| 1 | core | P1 | └ aiosd-core (vỏ gom nhóm) | - | leader | 0/7 con xong |
| 2 | audit | P1 | └ └ aios-audit: audit log JSONL append-only có hash chain + lệnh verify | ←ld-setup →broker | worker | bị chặn (đợi ld-setup) |
| 2 | registry | P1 | └ └ aios-registry: đọc plugin.toml, kiểm chữ ký .sig, mức rủi ro hiệu lực = max(khai báo, suy ra) | ←ld-setup →broker | worker | bị chặn (đợi ld-setup) |
| 2 | sandbox | P1 | └ └ aios-sandbox-exec: bubblewrap + Landlock ABI V6 fail-closed + seccomp chặn UDP/raw; test trong VM | ←ld-setup,ld-vm →executor,s14 ~s14 | worker | bị chặn (đợi ld-setup,ld-vm) |
| 3 | policy | P0 | └ └ aios-policy: nạp auto/confirmable/forbid Cedar, tính context, trả Allow/Confirm/Deny; qua test bất biến | ←ld-setup,ld-secspec →broker | worker | bị chặn (đợi ld-setup,ld-secspec) |
| 3 | confirm | P0 | └ └ aios-confirm: ConfirmClaims, chuẩn hóa NFC+JCS, ký/kiểm Ed25519, nonce một lần, hạn, token theo lô | ←ld-setup,ld-secspec →broker | worker | bị chặn (đợi ld-setup,ld-secspec) |
| 4 | broker | P1 | └ └ aios-broker: MCP endpoint aios-tools, gọi policy/confirm/audit/registry, idempotency key, nhãn session | ←policy,confirm,audit,registry →cognition,executor | worker | bị chặn (đợi policy,confirm,audit,registry) |
| 5 | executor | P1 | └ └ Broker executor: chạy tool process/mcp qua aios-sandbox-exec, cgroup tạm dưới aios-agent.slice | ←broker,sandbox →ld-gate | worker | bị chặn (đợi broker,sandbox) |
| 2 | ld-secspec | P0 | └ Leader: viết trước test bất biến Cedar (6 bất biến) và test vector token xác nhận | ←ld-setup →policy,confirm | leader | bị chặn (đợi ld-setup) |
| 2 | gateway | P0 | └ aios-model-gateway: /v1/chat/completions + /v1/embeddings, khóa ảo, nhãn session, secret không cloud, ngân sách theo người dùng | ←ld-setup →hindsight,cognition | worker | bị chặn (đợi ld-setup) |
| 2 | tools3 | P1 | └ 3 tool cho cổng M0: sys.info, fs.search (low), fs.write_note (medium, có undo) | ←ld-setup →cognition | worker | bị chặn (đợi ld-setup) |
| 2 | llm | P1 | └ llama.cpp trong VM: unit systemd, tải mô hình nhỏ có tool calling + mô hình embedding, ghim sha256 vào models.lock | ←ld-vm →hindsight | worker | bị chặn (đợi ld-vm) |
| 2 | image | P2 | └ Containerfile ảnh AiOS: Fedora bootc ghim digest, đổi thương hiệu, unit systemd; build + bootc container lint | ←ld-setup →s16 | worker | bị chặn (đợi ld-setup) |
| 3 | hindsight | P2 | └ Spike S-4: Postgres+pgvector+hindsight-api-slim trong VM, LLM/embedding/reranker qua Gateway, không torch, đo RSS | ←ld-vm,gateway,llm →ld-gate | worker | bị chặn (đợi ld-vm,gateway,llm) |
| 3 | s14 | P2 | └ Spike S-14 (phần Landlock): aios-sandbox-exec trên kernel 7.2 của VM, báo cáo ABI và hành vi | ←sandbox →ld-gate | worker | bị chặn (đợi sandbox) |
| 5 | cognition | P1 | └ aiosd-cognition tối thiểu + aiosctl: ask (grammar Reflex + vòng L2), status, audit verify, freeze | ←broker,gateway,tools3 →s16,ld-gate | worker | bị chặn (đợi broker,gateway,tools3) |
| 6 | ld-gate | P0 | └ Leader: tích hợp, chạy cổng M0 trong VM (aiosctl ask + 3 tool, tắt aiosd vẫn chạy), đo S-9 baseline | ←executor,cognition,hindsight,s14 | leader | bị chặn (đợi executor,cognition,hindsight,s14) |
| 6 | s16 | P2 | └ Spike S-16: bảng giấy phép mọi phụ thuộc (cargo-deny, pip-licenses) so với Apache-2.0 | ←cognition,image | worker | bị chặn (đợi cognition,image) |

Ba spike của M0 chưa có trong bảng vì thiếu môi trường: S-17 (cần phiên đồ họa trên niri), S-24 (cần hệ bootc đã boot), S-13 đầy đủ (cần benchmark tiếng Việt và khóa API cloud).

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

**Mac** (máy của chủ dự án): Homebrew, Docker Desktop, `uv`, Python 3.14, `rustup` (chưa đặt toolchain mặc định), `opencode` v2.0.32, `gh` đã đăng nhập tài khoản `nqthiep`.

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
- **Mã yêu cầu REQ-... đổi số** khi tài liệu gốc thêm hoặc bớt câu; khi trích dẫn, kèm mục thiết kế gốc.

## 9. Việc chờ chủ dự án

| Việc | Ghi chú |
| --- | --- |
| Xác nhận bắt đầu giao worker M0 | Xem §4 bước 1 |
| Khóa API cloud và ngân sách thử nghiệm | Cho S-1, S-13, S-18, S-25; chủ dự án tự nhập |
| Laptop x86_64 | Cho spike phần cứng |
| Cách hiện thực bộ gõ tiếng Việt | Chủ dự án sẽ cung cấp sau (D-51) |
| niri hay cosmic-comp | Quyết sau spike S-17 |
