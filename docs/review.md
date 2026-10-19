# AiOS — Review và phản biện bộ tài liệu thiết kế

_Ngày review: 2026-10-18_

## 0. Phạm vi và phương pháp

Review toàn bộ README.md, design.md, integration.md, tech-stack.md, decisions.md, feasibility.md, requirements.md theo 4 hướng chạy song song:

| Hướng | Câu hỏi |
| --- | --- |
| A. Chiến lược và khả thi | Phạm vi, độ phức tạp, tài nguyên, lộ trình có đưa giá trị sớm không |
| B. Bảo mật | Ranh giới tin cậy, prompt injection, rò nhãn riêng tư, sandbox, chuỗi cung ứng |
| C. Nhất quán | Mâu thuẫn giữa các tài liệu, quyết định cũ, tham chiếu hỏng |
| D. Kiểm chứng kỹ thuật | Đối chiếu khẳng định kỹ thuật với mã nguồn và tài liệu chính thức |

Hướng D kiểm tra bằng mã nguồn kernel tại tag v6.18/v7.0/v7.1/v7.2/v7.3-rc7, docs.rs, crates.io, PyPI và tài liệu chính thức của opencode, OpenShell, Hindsight. Các hướng A–C chỉ dựa trên văn bản tài liệu; chưa có mã AiOS để kiểm chứng. Nhận định không trích trực tiếp từ tài liệu hoặc nguồn được gắn \[Inference\].

Mức độ: **Cao** = phá vỡ một nguyên tắc cốt lõi hoặc chặn lộ trình; **Trung bình** = cần sửa trước khi viết mã phần liên quan; **Thấp** = sửa khi tiện.

## 1. Tóm tắt

**Nền tảng thiết kế đúng hướng: LLM ở user space, mô hình bị coi là không tin cậy, dữ liệu không nhãn xử lý như `secret`, UI sinh qua IR có validator. Nhưng bộ tài liệu có 3 nhóm vấn đề lớn:**

1. **Phạm vi quá rộng so với một MVP.** \[Inference\] Thiết kế hiện tại tương đương 4 sản phẩm (desktop environment, agent runtime, hệ plugin, nền tảng ML trên thiết bị), với khoảng 10 runtime, khoảng 12 daemon và 4 lớp chính sách. Lộ trình không cho người dùng một sản phẩm dùng được trước P3.
2. **Ranh giới tin cậy có lỗ hổng cụ thể.** Token xác nhận không gắn với hành động; file trong "vùng mở" quyết định quyền; nhãn riêng tư chỉ áp theo client, không theo dữ liệu; BPF LSM không bao phủ sandbox OpenShell. Bốn điểm này phá trực tiếp tuyên bố "ba lớp chính sách độc lập" ở integration.md §3.3.
3. **Tài liệu chưa đồng bộ sau các quyết định D-16 → D-23.** tech-stack.md vẫn khuyến nghị LangGraph và Slint; design.md §2, §4, §6 vẫn mô tả kiến trúc cũ; có 2 định dạng UI IR và 2 định dạng manifest.

Kiểm chứng kỹ thuật (hướng D): phần lớn khẳng định đúng (phiên bản kernel, hook `file_open`, API Cedar, Aya, rmcp, endpoint opencode, client Hindsight). 8 khẳng định sai một phần, trong đó quan trọng nhất là Landlock mạng (UDP đã có từ kernel 7.2) và chính sách OpenShell mẫu sẽ bị từ chối khi nạp.

### 1.1 Mười vấn đề ưu tiên

| # | Vấn đề | Mức | Mục |
| --- | --- | --- | --- |
| 1 | Token xác nhận không gắn với tool + args cụ thể (TOCTOU, replay, tráo args) | Cao | §3 B-C1 |
| 2 | Vùng mở D-14 chứa `risk_overrides`, `trust`, `metadata.aios.risk`: agent tự hạ mức rủi ro | Cao | §3 B-C2 |
| 3 | Nhãn riêng tư áp theo khóa ảo của client, không theo dữ liệu trong session; dữ liệu `secret` lên cloud được | Cao | §3 B-C4 |
| 4 | BPF LSM chỉ so khớp đúng cgroup id do Broker ghi; sandbox OpenShell của opencode không được bảo vệ ở lớp kernel | Cao | §3 B-T1 |
| 5 | Lộ trình: UI và `aios-trusted-ui` đến P3, nhưng cổng P1 cần luồng xác nhận | Cao | §4 C-6, §2 A-1 |
| 6 | Ma trận §9.3 cho `low` + không đảo ngược "Tự chạy", trái REQ-D-4.4-01 | Cao | §4 C-4 |
| 7 | tech-stack.md và design.md §2/§4/§6 mâu thuẫn với D-16 → D-23 | Cao | §4 C-1, C-2 |
| 8 | Ngân sách RAM: \[Inference\] 6–9 GB thường trú trên máy 16 GB | Cao | §2 A-5 |
| 9 | bootc bất biến xung đột với cài Node/JDK/npm lúc chạy | Cao | §2 A-7 |
| 10 | Provenance bị "rửa" qua `reflect`/hợp nhất của Hindsight, đầu độc dần tới skill và Reflex | Cao | §3 B-C3 |

## 2. Phản biện chiến lược và khả thi (hướng A)

**A-1. \[Cao\] Lộ trình không đưa giá trị sớm.**
- Cổng P0 là "tắt aiosd thì OS vẫn chạy" (design.md §13): không có giá trị cho người dùng.
- P1 gom Router, System 1, System 2, sandbox, BPF LSM; nhưng `aios-shell` đến P3, trong khi hội thoại là màn hình chính (D-10).
- Quy mô đội: UNKNOWN (design.md §13, feasibility.md).
- **Khuyến nghị:** định nghĩa MVP rõ ràng, gồm: ảnh hệ thống, app chat tối thiểu, `aiosd-core` (Tool Broker + chính sách + audit + `aios-sandbox-exec`), một vòng lặp agent gọi Model Gateway, khoảng 20 tool hệ thống, bộ nhớ SQLite, hộp thoại xác nhận. Cổng MVP: tỷ lệ thành công trên bộ kịch bản vàng và tỷ lệ hoàn tác.

**A-2. \[Cao\] Độ phức tạp vượt nhu cầu v1.** Runtime: Rust, Python, Bun, Go, Node ≥22/≥24, JDK, wasmtime, Podman, microVM, Chromium (integration.md §11.1). Đề xuất cắt hoặc hoãn:

| Hiện tại | Đề xuất cho v1 | Lý do |
| --- | --- | --- |
| Hindsight + WeKnora + Postgres | SQLite + sqlite-vec như design §6.4; hoãn WeKnora | Hindsight 1.0.0 mới phát hành 13/10/2026; WeKnora chưa có nhu cầu đo được |
| OpenShell + `aios-sandbox-exec` | Một sandbox: systemd-run + bwrap + Landlock; microVM chỉ khi có rea | Hai cơ chế sandbox, hai bộ chính sách |
| `aiosd-cognition` (Python) + opencode (Bun) | Một tiến trình agent (xem A-6) | Hai runtime cho một việc |
| 3 runtime plugin + WIT + 4 cấp tin cậy + 2 phiên bản ABI | Chỉ plugin `mcp` qua stdio trong sandbox | Chưa có plugin bên thứ ba thì chưa cần ABI ổn định |
| `aios-gateway`, ArtCraft, browser-use, rea | Ra khỏi lộ trình lõi | Không thuộc MVP |

**A-3. \[Cao\] Tự viết compositor ngay từ v1.** Chủ dự án đã chốt tự viết DE (D-16). Phản biện không đề nghị đổi quyết định này, mà đề nghị **chia giai đoạn**: \[Inference\] v1 chạy app chat như Wayland client trên compositor có sẵn (niri, cosmic-comp hoặc sway), bề mặt tin cậy thử dùng `ext-session-lock-v1` (cần spike); `aios-compositor` tự viết vào sau MVP. Lý do: gpui-kit phụ thuộc `gpui-pre =0.3.9` và fork `lb-wry`; tự viết compositor + shell + trusted-ui + portal là khối việc lớn nhất của lộ trình.

**A-4. \[Cao\] Router 3 cấp chưa định nghĩa chặt.**
- `risk_of(req)` gọi trên yêu cầu thô, trước khi biết tool (design.md §4.3), nhưng rủi ro phụ thuộc tool.
- Khi khởi động lạnh, bộ nhớ rỗng nên mọi yêu cầu có novelty cao và lên thẳng System 2. Novelty đo độ mới của dữ liệu, không đo năng lực System 1.
- τ cần classifier tiếng Việt đã hiệu chỉnh, chưa có dữ liệu gán nhãn.
- Timeout S1 0,8 s rồi mới gọi S2: leo thang cộng dồn độ trễ.
- **Khuyến nghị:** v1 dùng 2 cấp: grammar tất định, và LLM gọi tool trong tập ý định đóng có schema (hợp lệ thì chạy, không hợp lệ thì leo thang). Bỏ novelty khỏi đường quyết định. Đo precision/recall của leo thang trên tập gán nhãn.

**A-5. \[Cao\] Ngân sách tài nguyên trên laptop 16 GB.** \[Inference\] Ước lượng, chưa đo:

| Thường trú | RAM ước lượng |
| --- | --- |
| System 1 4B q4 + KV cache | 3–3,5 GB |
| Embedding e5-large 560M | 0,6–1,1 GB |
| Whisper lượt 2 | 0,6–1,6 GB |
| Zipformer, Piper, VieNeu, wake word, VAD | 0,5–0,7 GB |
| Postgres, Hindsight (không torch), WeKnora, OpenShell + Podman | 0,6–1,8 GB |
| `aiosd-*`, compositor, shell, trusted-ui | 0,6–0,8 GB |
| **Tổng khi rảnh** | **khoảng 6–9 GB** |

- Thêm khi có task L2: opencode 0,2–0,4 GB; Chromium cho browser-use 0,5–1,5 GB.
- Độ trễ: VAD `min_silence_ms: 500` (tech-stack.md §9) đã chiếm một nửa mục tiêu V-4 (< 1 s). \[Inference\] L1 với mô hình 4B trên iGPU Vulkan cộng recall và sinh JSON tool call có thể mất 1–3 s, không phải 50–500 ms như design §4.
- **Khuyến nghị:** chạy S-9 ngay ở P0; giọng nói v1 là bấm-để-nói, một STT, một TTS; nạp mô hình theo nhu cầu.

**A-6. \[Cao\] opencode làm engine System 2.** Agent `aios` tắt toàn bộ tool built-in và chỉ dùng MCP (integration.md §4). Khi đó opencode chỉ còn là vòng lặp LLM + MCP client + SSE, nhưng kéo theo Bun, sandbox Podman, hook `aios-guard`, và một upstream phát hành nhanh (v1.19.x; nhánh `dev` đã có API plugin V2). \[Inference\] Một vòng plan/act/verify tự viết khoảng vài trăm dòng, chạy cùng tiến trình với router. **Phản biện:** giữ opencode cho lập trình (agent `build`), cân nhắc tự viết vòng lặp cho agent `aios`. Đây là điều chỉnh D-23, cần chủ dự án quyết định.

**A-7. \[Cao\] bootc bất biến và cài đặt lúc chạy.** `/usr` chỉ đọc (tech-stack.md §2). Nhưng tài liệu định cài Node, JDK theo nhu cầu (integration.md §11.1), `pkg.install` dùng package manager của distro (design §7.10, §9.2), opencode cần npm/bun. **Khuyến nghị:** ứng dụng qua Flatpak; công cụ phát triển trong toolbox/distrobox; runtime của thành phần lõi đưa vào lúc build image.

**A-8. \[Trung bình\] UI sinh hoàn toàn và "không có ứng dụng Cài đặt".**
- 6 chiều ngữ cảnh (design §7.2) cho 6×4×4×3×3×3 = 2.592 signature. \[Inference\] Cache L0 hiếm khi trúng lúc đầu.
- \[Inference\] Không có Settings thì người dùng không duyệt được máy làm được gì; họ phải biết hỏi gì.
- Tài liệu không nói cách giữ focus bàn phím và trình đọc màn hình khi surface được sinh lại.
- **Khuyến nghị:** v1 dùng shell chat bố cục cố định; khoảng 10 nhóm cài đặt có widget cố định, tìm được qua ô tìm kiếm (cũng là shell dự phòng); chỉ sinh nội dung bên trong widget. Hoãn Context Engine, cache theo signature, patch, ngân sách thay đổi.

**A-9. \[Trung bình\] Vòng tự học khó đo.** Cổng thăng cấp Reflex (20 lần, ≥ 98%) không có oracle định nghĩa "thành công": Verifier là LLM, "không hoàn tác" là tín hiệu yếu. Shadow mode chỉ so Reflex với S1/S2. \[Inference\] LoRA trên laptop với bộ eval nhỏ có nguy cơ quên kiến thức cũ. **Khuyến nghị:** bỏ biên dịch Reflex tự động và LoRA khỏi lộ trình chính; thay bằng "lưu thành lệnh nhanh" do người dùng chủ động; đo bằng số lần dùng lại và tỷ lệ hoàn tác.

**A-10. \[Trung bình\] Driver do LLM sinh là bài toán nghiên cứu.** Subsystem chuẩn đã có PipeWire, CUPS, BlueZ; "sinh adapter" thực chất là điền template. Sinh driver cần rea, Ghidra, JDK, microVM và phần cứng thật trong vòng lặp; người dùng không thẩm định được diff driver. **Khuyến nghị:** v1 dùng bảng ánh xạ tĩnh; design §10.3 và integration §9 chuyển sang mục nghiên cứu.

**A-11. \[Trung bình\] Phần còn thiếu.**

| Vấn đề | Hiện trạng | Đề xuất |
| --- | --- | --- |
| Offline | Mục tiêu "chạy hoàn toàn offline", nhưng profile `generic` có `s2-local: null` | Ghi rõ những gì làm được khi offline |
| Nhiều người dùng | Câu hỏi mở; bank Hindsight chia theo nhãn, không theo người dùng; Postgres, Gateway là dịch vụ chung | Chốt một người dùng cho v1, hoặc chia theo UID |
| Backup, migration | Chỉ có snapshot | Định dạng export, migration schema, sao lưu ra ngoài máy |
| i18n | Grammar Reflex, kịch bản vàng, giọng nói chỉ có tiếng Việt | Tách tài nguyên ngôn ngữ |
| Test agent phi tất định | Kịch bản vàng chạy một lần | Chạy N lần, báo pass@k, ghim seed/temperature, replay trace |
| Quan sát | OpenTelemetry nhưng không nói lưu ở đâu | Trace cục bộ, xoay vòng có giới hạn |
| Giấy phép dự án | UNKNOWN; kèm SUL-1.0, GPL (Piper), binary độc quyền (FastFlowLM) | Chọn trước P1 |

**A-12. \[Thấp\] Phần kernel làm quá sớm.** Ví dụ Reflex như "tắt tiếng" không cần BPF. v1 đủ với Landlock + seccomp + cgroup; event bus eBPF và BPF LSM có thể lùi sau MVP.

## 3. Phản biện bảo mật (hướng B)

### Mức Cao

**B-C1. Token xác nhận không gắn với hành động.** (tech-stack.md §4–§5; integration.md §3.2, §4.2)
- Cedar chỉ kiểm `context.user_confirmed && confirm_source == "trusted_ui"`; tài liệu không định nghĩa token ký nội dung gì.
- Trong `aios-guard`, hook gọi `/confirm` rồi trả quyền cho opencode; opencode bị chiếm có thể đổi `output.args` sau khi người dùng duyệt.
- Token đi qua tầng không tin cậy có thể bị dùng lại cho bước sau.
- **Sửa:** token = chữ ký trên `hash(principal, session, task_step, tool, args đã chuẩn hóa, hash kết quả dry-run, phiên bản policy)` + nonce + hạn ≤ 60 s; Broker lưu nonce, dùng một lần, so với đúng lệnh mà Broker tự thực thi; `user_confirmed` chỉ do Broker đặt. Tool built-in mức `medium` trở lên của opencode PHẢI chạy qua Broker, không chạy trong opencode.

**B-C2. Vùng mở D-14 chứa các file quyết định quyền.** (integration.md §6.3, §7.4, §10; design §15.2, §15.7)
- Manifest ứng dụng (vùng mở) chứa `risk_overrides`, `trust`, `sandbox`, `entry`. Agent bị prompt injection đổi `"file.delete" = "low"` hoặc `trust = "vendor"`, rồi đưa một diff dài cho người dùng duyệt.
- `metadata.aios.risk` trong SKILL.md do agent tự khai nhưng "Policy Engine dùng".
- "CI cục bộ chạy eval" không nói chạy ở đâu; nếu chạy trong cùng sandbox thì kết quả eval giả mạo được.
- **Sửa:** risk, trust, capability không bao giờ lấy từ file vùng mở; Policy dùng `max(mức khai báo, mức suy ra từ capability)` hoặc registry đã ký; `risk_overrides` chỉ nhận từ gói vendor có chữ ký; eval chạy trong sandbox sạch từ image khóa, kết quả do `aiosd-core` ký; trusted UI hiển thị riêng phần thay đổi quyền.

**B-C3. Rửa provenance qua bộ nhớ.** (design §6.3, §6.5; integration.md §6.2, §6.3, §7.4)
- `gate.py` đánh dấu external chỉ bằng tiền tố văn bản. \[Inference\] Khi Hindsight `reflect` hoặc hợp nhất, LLM sinh sự thật mới và metadata provenance khó được truyền tiếp: nội dung external thành inferred hoặc observed.
- Mental model điều khiển UI Planner và nút Action.
- Nhãn `observed` có thể bị ép: một trang web hoặc cron do injection tạo khiến cùng hành vi lặp 20 lần, đạt `MIN_RUNS`.
- Skill sinh từ ngữ cảnh external vào kho sau khi duyệt, nhưng không giữ dấu nhiễm.
- **Sửa:** taint gắn theo bản ghi và đơn điệu (đầu ra mang `max(taint đầu vào)`); đầu ra reflect/hợp nhất mặc định `inferred` + tainted; duyệt không xóa taint; `observed` chỉ tính từ sự kiện nhập thật có serial của compositor.

**B-C4. Nhãn riêng tư chỉ áp theo client.** (integration.md §3.1; design §5.2)
- opencode có `max_cloud_privacy: personal`. Agent `aios` gọi `recall` vào bank `user-secret` hoặc đọc file mật qua `aios-tools`, rồi lượt LLM kế tiếp gửi lên cloud. API OpenAI-compatible không có trường nhãn; Gateway chỉ biết khóa ảo.
- Đường rửa khác: reflect từ bank secret sang public; embedding gọi cloud; `recall()` (tech-stack §7) chỉ lọc provenance, không lọc privacy.
- REQ-D-5.2-02 coi dữ liệu không nhãn là `secret`; vì request của Hindsight, WeKnora, opencode không mang nhãn, bank `user-public` "được phép cloud" thực tế không dùng được.
- **Sửa:** Broker ghi "session S đã nhận dữ liệu nhãn X"; Gateway (lõi tin cậy) tra nhãn theo session qua khóa ảo cấp cho từng session và ép route local; cấm ghi chéo giữa bank trừ khi người dùng hạ nhãn qua trusted UI; embedding của bank personal/secret luôn local.

**B-C5. Kênh từ xa và lịch là đường rò dữ liệu.** (integration.md §8)
- Gửi kết quả về Telegram/Discord là gửi dữ liệu cho bên thứ ba, ngoài phạm vi §5.2.
- Injection có thể tạo job lịch với kênh nhận do kẻ tấn công kiểm soát.
- Chiếm tài khoản chat là chạy được mọi tool `low`/`medium` và task L2.
- **Sửa:** kênh remote có autonomy ≤ 1, mặc định chỉ đọc; lọc nhãn đầu ra (personal/secret không ra kênh bên thứ ba); đích của lịch chỉ là kênh đã ghép đôi; yêu cầu chờ duyệt ghi nguồn, thời điểm và hết hạn.

### Mức Trung bình

**B-T1. BPF LSM phủ hẹp hơn tuyên bố.** (design §3.4; integration.md §3.3)
- Chỉ hook `file_open` với cờ ghi; không chặn unlink, rename, truncate, setattr, connect, ptrace, bpf, exec. integration.md §3.3 ghi "Từ chối syscall ngoài phạm vi" là quá mức.
- `workspace_dev` so theo superblock: workspace cùng filesystem với /home thì toàn bộ /home ghi được.
- Đã kiểm tra lại: `agent_guard` tra `bpf_get_current_cgroup_id()` trong map `agent_cgroups` do Broker ghi cho từng lần gọi tool (design.md §3.3–§3.4). Sandbox OpenShell/Podman của opencode không do Broker tạo, nên rơi vào nhánh "không phải agent" và được cho qua. Spike S-5 sẽ không đạt ở lớp kernel.
- \[Inference\] Link BPF không được pin thì chương trình LSM bị gỡ khi `aiosd-core` chết.
- **Sửa:** kiểm theo cgroup tổ tiên (`aios-agent.slice`, mặc định từ chối); thêm hook `inode_unlink`, `inode_rename`, `path_truncate`, `socket_connect`, `ptrace_access_check`, `bprm_check_security`; pin link trên bpffs.

**B-T2. Agent `build` của opencode là thực thi mã tùy ý.**
- npm/bun với `GET /**` vẫn chạy script postinstall.
- \[Inference\] opencode đọc `opencode.json` và `.opencode/` cấp dự án: agent tự ghi được cấu hình gỡ hook.
- Broker nhận diện client bằng trường `client` trong body HTTP không xác thực.
- **Sửa:** mount cấu hình chỉ đọc, tắt nạp cấu hình cấp dự án; npm mirror nội bộ với `--ignore-scripts`; nhận diện client bằng Unix socket + `SO_PEERCRED` hoặc mTLS theo session.

**B-T3. Trusted UI bị giả mạo và gây mệt mỏi duyệt.**
- Ứng dụng toàn màn hình hoặc Chromium vẽ giả được dấu hiệu thị giác.
- Dòng "AI giải thích" trong bề mặt tin cậy là kênh injection.
- Args dài, homoglyph, ký tự bidi bị hiển thị cắt.
- Autonomy 0 + nâng mức vì external + task nhiều bước sinh hàng chục hộp thoại.
- UI IR: label và action độc lập; nút "Hủy" có thể mang action `fs.delete`.
- **Sửa:** secure attention key hoặc compositor làm mờ toàn màn hình kèm bí mật do người dùng chọn; bỏ "AI giải thích" với hành động `high`; chuẩn hóa và escape args; duyệt theo lô cho một kế hoạch đã băm; renderer luôn hiện tên tool thật cạnh label; bấm trong UI sinh không tính là xác nhận.

**B-T4. Đường injection không mang nhãn external.** Body thông báo `Notify` của mọi ứng dụng vào hội thoại và episodic với nhãn personal; chuỗi USB descriptor (`product_string`) do thiết bị BadUSB đặt tùy ý và tự kích hoạt task L2; cache `vendor:product → Reflex` giả mạo được bằng ID trùng. **Sửa:** đánh dấu cả hai nguồn là external; task L2 cho thiết bị lạ cần người dùng đồng ý.

**B-T5. Cedar biến Deny thành Confirm.** Đánh giá 2 lần với luật `permit ... user_confirmed` áp cho mọi tool, nên mọi Deny thành Confirm. Policy không mã hóa yêu cầu PIN, không phân biệt principal (remote, plugin, voice); `origin_external` có thể do caller tự khai. **Sửa:** thêm `forbid` tường minh; đưa `confirm_level`, `channel` vào policy; mọi trường context do Broker tính; dùng `cedar-policy-symcc` chứng minh bất biến.

**B-T6. Chuỗi cung ứng.** Image nền `fedora-bootc:latest` không ghim digest; 54+ hook oh-my-openagent chạy trong opencode; \[Inference\] opencode cài provider npm lúc chạy và tự cập nhật; trọng số mô hình và LoRA trong /var không ký (parser GGUF là bề mặt tấn công); chưa có thu hồi khóa vendor; plugin `generated` không ký. **Sửa:** ghim digest và lockfile, vendor sẵn trong image; ký và verify bằng sigstore; manifest hash mô hình trong image; tắt auto-update; danh sách thu hồi khóa.

**B-T7. Kill switch phụ thuộc `aiosd-core`.** Không rõ freeze có dừng opencode, sandbox OpenShell, lịch, `aios-gateway`, plugin `process`. **Sửa:** compositor (tin cậy) ghi trực tiếp `cgroup.freeze`/`cgroup.kill` lên `aios-agent.slice` qua một unit systemd độc lập; freeze thu hồi khóa ảo ở Gateway.

### Mức Thấp

- **Giọng nói:** hành động `medium` qua giọng nói tự chạy, nên âm thanh từ TV có thể kích hoạt; hệ thống có clone giọng 3–5 giây (VieNeu); "PIN" ở V-5 phải nhập trên trusted UI, không nói ra. Giới hạn autonomy kênh voice như kênh remote.
- **browser-use:** \[Inference\] Chromium trong Podman rootless thường cần `--no-sandbox`. Chặn DNS rebinding, IPv6, `host.aios.internal`; dùng profile riêng không có cookie người dùng.
- **Chia sẻ màn hình:** chỉ phát hiện qua portal; ứng dụng chụp trực tiếp qua PipeWire không bị phát hiện.
- **Plugin:** `register-reflex-rule` từ plugin `community`/`generated` phải qua cùng cổng duyệt như design §6.5.

## 4. Nhất quán giữa các tài liệu (hướng C)

### Mức Cao

| # | Mâu thuẫn | Bên A | Bên B | Cách sửa |
| --- | --- | --- | --- | --- |
| C-1 | tech-stack vẫn chọn LangGraph, Slint, SQLite + sqlite-vec cho bộ nhớ; first boot dùng "Slint backend linuxkms" | tech-stack §1, §5, §7, §10, §13 | D-17, D-18, D-23; integration §2.1 | Ghi "đã thay bởi D-xx"; chọn toolkit first boot; trích lại requirements (REQ-T-5-01, REQ-T-5-02, REQ-T-7-01 đang bắt buộc) |
| C-2 | design mô tả 5 tầng, `aios-shell` = compositor + renderer, L2 trong `aiosd-cognition` Python, bộ nhớ `memory_item` | design §2, §4, §6.1, §6.4, §7.10 | integration §2, §4, §6, §7 | Cập nhật design §2, §4, §4.4, §6.1, §6.4 |
| C-3 | D-16 "Đã chốt" đã gồm gpui-kit, nhưng D-17 (gpui-kit) "Đề xuất"; design §14 đánh dấu [x]; trạng thái "Thay thế" của D-12 không có trong chú giải | decisions.md | design §14 | Tách toolkit khỏi D-16 hoặc chốt D-17; thêm "Thay thế" vào chú giải |
| C-4 | "Mỗi bước có tác động không đảo ngược PHẢI có xác nhận" | design §4.4 (REQ-D-4.4-01) | design §9.3: `low` + không đảo ngược = "Tự chạy, thông báo" | Đổi ô ma trận thành "Hỏi xác nhận" hoặc thu hẹp REQ |
| C-5 | "OpenShell và BPF LSM chặn mọi đường vòng"; plugin `mcp` chạy trong OpenShell | integration §3, design §15.3 | `hindsight-api.service`, `weknora.service` chạy như systemd service, không có trong §5.1 | Thêm vào §5.1 hoặc nêu cơ chế thay thế |
| C-6 | Cổng P1 "không có tool high nào chạy thiếu xác nhận" | design §13 | `aios-trusted-ui` đến P3 | Đưa hộp thoại xác nhận tối thiểu vào P1 |

### Mức Trung bình

- **C-7. Hai định dạng UI IR.** design §7.3 dùng `kind`, `source`; design §15.5 dùng `type`, `$bind`, `row`, `badge` nhưng nói "đi qua cùng validator"; integration §7.2 dùng `Column`, `Progress` không có trong §7.3. Đã kiểm tra lại: §7.3 dùng `kind`. Viết lại §15.5 theo IR v1.
- **C-8. Cổng, socket, scheme secret của Model Gateway lệch nhau:** design §5.3 (`/etc/aios/gateway.yaml`, `unix:///run/aios/llm-s1.sock`, `127.0.0.1:8001`, `secret://`), integration §3.1 (`gateway.d/clients.yaml`, `8081`, `8090`, `aios-secret://`), tech-stack §6 (`8000`, `llm-s2.sock`). Tên mô hình logic `s1`/`s2` khác `system2`/`coder`. `max_cloud_privacy: none` không phải nhãn hợp lệ. Thêm một bảng cổng/socket duy nhất.
- **C-9. Giao thức tới Tool Broker:** design §2 ghi gRPC/Cap'n Proto qua Unix socket; integration §4.2 gọi REST `/preflight` cổng 8091, §5.2 khai cổng 8091 là `protocol: mcp`; `aios-broker-bridge` không có trong `binaries` của profile.
- **C-10. Hai mô hình sandbox:** design §9, tech-stack §8 sandbox theo từng lần gọi tool; integration §5 dùng sandbox OpenShell chạy lâu. Hai định dạng manifest tool: YAML (design §9.2) và `plugin.toml` (design §15.2).
- **C-11. "Lõi tin cậy" và "desktop environment" bị trôi nghĩa:** integration §0 liệt kê 3 thành phần, §7.1 thêm `aios-shell`, §10 rộng hơn, design §15.1 thêm Model Gateway, Cedar, profile OpenShell. D-11 "Không có desktop environment" so với D-16 "Desktop environment tự viết". Định nghĩa một lần ở design §15.1; đổi D-11 thành "không dùng DE bên thứ ba".
- **C-12. D-14 cũ:** vẫn ghi "plugin dsh"; tiêu đề "thay đổi chính `aiosd`" trái REQ-D-15.1-01; feasibility.md vẫn ghi "chưa được thiết kế"; integration §9 cho agent `build` sinh driver libusb nhưng design §15.7 chỉ cho plugin `generated` chạy `wasm`.
- **C-13. Nhãn cho LLM call của thành phần bên thứ ba:** xem B-C4.

### Mức Thấp

- **C-14.** Spike: tech-stack §13 đánh số 1–7, integration dùng S-1 đến S-11; S-3 trùng spike 1, S-11 thay spike 5; spike STT, mô hình System 1, portal, giấy phép chưa có mã S; D-13 vẫn ghi "Chờ 7 spike".
- **C-15.** REQ-D-3.1-02 chỉ cho dùng sched_ext khi có số đo, nhưng tech-stack §3 đề xuất dùng sớm; design §3.5 dùng libbpf-rs, tech-stack chọn Aya.
- **C-16.** Câu hỏi mở design §14 đã được trả lời ngầm: ngân sách cloud (`budget_usd_day` ở integration §3.1), distro (S-5 ghi Fedora bootc).
- **C-17.** Tham chiếu hỏng trong integration.md:
  - §11.2 nhắc "phương án A" mà không định nghĩa.
  - §9 dẫn REQ-D-10.3-01, nhưng đó là câu "CÓ THỂ".
  - §3.1 dẫn "(§5.2)" cho việc tiêm credential, nhưng §5.2 không có nội dung này.
  - §5.2 dẫn REQ-T-8-01, vốn thuộc `aios-sandbox-exec`.
  - Manifest wordcraft thiếu trường `api` bắt buộc và có trường ngoài lược đồ design §15.2.
  - Tên unit lệch: `aiosd.service` (tech-stack) so với `aiosd-core.service` (integration).
  - Sơ đồ §2 có cạnh `router --> hs`, trái quy tắc "agent không ghi thẳng vào Hindsight" ở §6.2.
- **C-18.** requirements.md phản ánh nguyên các mâu thuẫn trên; sửa ở tài liệu gốc rồi chạy lại script.

## 5. Kiểm chứng kỹ thuật (hướng D)

### Sai hoặc đúng một phần

| # | Khẳng định | Vị trí | Kết luận | Sửa |
| --- | --- | --- | --- | --- |
| D-1 | Landlock mạng chỉ giới hạn TCP bind/connect | tech-stack §8 | Một phần | ABI theo tag: 6.18 = 7, 7.0 = 8, 7.1 = 9, 7.2 = 10 (thêm UDP bind/connect/send), 7.3-rc7 = 11 (thêm `RESTRICT_SELF_NO_NEW_PRIVS`). Crate `landlock` 0.4.7 mới đến `ABI::V9`, `AccessNet` chỉ có TCP. Trên 6.18 vẫn cần seccomp chặn UDP. Dùng `ABI::V6` trở lên cho scope abstract UNIX socket và signal ([landlock.h v7.2](https://github.com/torvalds/linux/blob/v7.2/include/uapi/linux/landlock.h), [docs.rs](https://docs.rs/landlock/0.4.7/landlock/enum.ABI.html)) |
| D-2 | "Landlock ABI 11" | integration §1 | Một phần | ABI 11 chỉ ở mainline 7.3-rc; stable 7.2 là 10; LTS 6.18 là 7 |
| D-3 | Phải bật `bpf` trong `lsm=` lúc boot | tech-stack §3 | Một phần | `CONFIG_LSM` mặc định của v6.18 đã kết thúc bằng `bpf`; đặt khi build image, kiểm bằng `/sys/kernel/security/lsm`. Lệnh test `lsm=landlock,bpf` tắt SELinux, lockdown, yama nên khác production |
| D-4 | 6.18 LTS chưa có amdxdna đủ mới | tech-stack §6, §13; integration §9 | Một phần | 6.18 có amdxdna, ivpu, rocket in-tree; `ethosu` từ 7.0. Lemonade/FastFlowLM yêu cầu 7.0+ hoặc `amdxdna-dkms` ([lemonade](https://lemonade-server.ai/flm_npu_linux.html)) |
| D-5 | Chính sách OpenShell mẫu | integration §5.2 | Một phần | Khóa đúng schema v0.1.4, nhưng: endpoint `protocol: rest` cần `access` hoặc `rules` (cổng 8090 thiếu, chính sách bị từ chối); `protocol: mcp` cần `rules` hoặc `mcp.allow_all_known_mcp_methods: true`; `enforcement` mặc định là `audit` (rule npm chỉ ghi log) nên phải đặt `enforce`; gói npm có scope cần `allow_encoded_slash: true`; `openshell profile lint` dành cho provider profile, prover chạy bằng `openshell-prover` ([schema.mdx](https://github.com/NVIDIA/OpenShell/blob/main/docs/how-it-works/policies/schema.mdx)) |
| D-6 | Hook `aios-guard` của opencode | integration §4.2 | Đúng, có lưu ý | Chữ ký khớp `@opencode-ai/plugin` v1.19.3. Docs không nói rõ `throw` chặn tool (\[Inference\] có chặn, cần test). Nhánh `dev` có API plugin V2. Đường dẫn tuyệt đối trong `plugin` được loader xử lý nhưng không có trong docs; an toàn hơn là đặt vào thư mục plugins |
| D-7 | Hindsight không cần torch khi dùng embedding qua Gateway | integration §11.2, S-4 | Đúng, thiếu bước | Đặt `HINDSIGHT_API_EMBEDDINGS_PROVIDER=openai` + `..._OPENAI_BASE_URL`; reranker mặc định cũng `local`, phải đặt `HINDSIGHT_API_RERANKER_PROVIDER`, nếu không vẫn cần torch |
| D-8 | opencode đọc `SKILL.md` (đang ghi \[Unverified\]) | integration §6.3 | Đúng | Đọc `.opencode/skills`, `.claude/skills`, `.agents/skills`; kho riêng khai bằng `skills.paths`. Có thể bỏ nhãn và thêm khóa vào §4.1 ([docs](https://opencode.ai/docs/skills/)) |

### Đúng

- Phiên bản kernel: mainline 7.3-rc7, stable 7.2.11, LTS 6.18.57; 6.18 và 6.12 hết hỗ trợ tháng 12/2028 ([releases.json](https://www.kernel.org/releases.json)).
- Hook `file_open`: chữ ký giống ở 6.18 và 7.2; `FMODE_WRITE` = 0x2; `file_open` không bị tắt cho BPF; mẫu trả `-EPERM` khớp `Documentation/bpf/prog_lsm.rst`.
- sched_ext: `CONFIG_SCHED_CLASS_EXT`; scx_loader `SwitchScheduler`, object `/org/scx/Loader`; repo đã tách thành [sched-ext/scx-loader](https://github.com/sched-ext/scx-loader).
- Cedar 4.14.0: chữ ký `Context::from_json_value`, `Request::new`, `Authorizer::is_authorized` khớp; `cedar-policy-symcc` 0.8.0 tồn tại.
- Aya 0.14 `Lsm::load` + `attach`; rmcp 3.5.1 có stdio, child process, streamable HTTP.
- opencode: `serve` mặc định 127.0.0.1:4096; `POST /session`, `/session/:id/prompt_async` (204), `/session/:id/abort`, `GET /event`; quyền `edit`/`bash`/`webfetch`; provider `@ai-sdk/openai-compatible`; MCP `type: local`.
- Hindsight client 1.0.0: `retain(bank_id, content, metadata)` đúng.
- sqlite-vec ổn định 0.1.9 (chưa 1.0); LangGraph 1.2.16 có `interrupt()` và `Command(resume=...)`.
- GPUI hiện thực client `zwp_text_input_v3` trên Wayland; không có mã compositor; Smithay có module `text_input` và `input_method`. Issue mở: zed #65302 (preedit trên Wayland), #54959 (fcitx5 trên X11).

### Không kiểm chứng được

- Cấu hình kernel của Fedora (`CONFIG_LSM`, amdxdna): nguồn bị chặn truy cập tự động.
- Bảng kernel tối thiểu ở design §3.1 ngoài sched_ext (6.12) và Landlock (5.13); fanotify.

## 6. Điểm mạnh nên giữ

- LLM chạy ở user space; mô hình bị coi là không tin cậy; kernel chỉ nhận chính sách đã kiểm.
- Dữ liệu không nhãn xử lý như `secret`; chặn cloud ở cả cấu hình và code.
- UI sinh qua IR có validator; shell không chạy code của mô hình (không QuickJS, không webview).
- Thừa nhận thẳng hook trong tiến trình không tin cậy chỉ là lớp tiện lợi; bộ eval nằm trong vùng khóa.
- OpenShell tiêm credential, client chỉ giữ placeholder.
- Landlock `hard_requirement`, fail closed.
- Speaker ID không dùng để cấp quyền.
- Plugin `community` chỉ chạy `wasm`.
- Audit log có hash chain; Cedar có công cụ kiểm chứng hình thức.
- Mọi khẳng định chưa đo đều gắn nhãn; lộ trình có cổng nghiệm thu.

## 7. Đề xuất thứ tự sửa

| Đợt | Việc | Cần chủ dự án quyết định |
| --- | --- | --- |
| 1 | Sửa bảo mật cấu trúc: B-C1 (token gắn hành động), B-C2 (quyền không lấy từ vùng mở), B-C4 (nhãn theo session), B-T1 (BPF theo cgroup tổ tiên + thêm hook) | Không |
| 2 | Đồng bộ tài liệu: C-1 → C-6, C-7 → C-12; sửa 8 khẳng định ở §5; chạy lại `scripts/extract_requirements.py` | Không |
| 3 | Định nghĩa MVP và viết lại lộ trình (A-1, A-2, C-6) | Có: phạm vi MVP |
| 4 | Quyết định lại các lựa chọn bị phản biện | Có: A-3 (compositor theo giai đoạn), A-4 (router 2 cấp cho v1), A-6 (vòng lặp agent tự viết thay opencode cho agent `aios`), A-7 (Flatpak/toolbox), A-8 (UI sinh tối giản cho v1), A-9 (hoãn Reflex tự động và LoRA), A-10 (driver sinh chuyển sang nghiên cứu) |
| 5 | Bổ sung phần thiếu A-11: giấy phép dự án, một hay nhiều người dùng, backup, test agent phi tất định | Có: giấy phép, số người dùng |

## 8. Trạng thái xử lý

_Cập nhật: 2026-10-18, sau khi sửa đợt 1 và đợt 2._

| Phát hiện | Trạng thái | Sửa ở đâu |
| --- | --- | --- |
| B-C1 Token xác nhận | Đã sửa | design §9.4, §9.5 (claims ký, nonce một lần, Broker giữ args và tự thực thi); integration §3.2, §4.2 (tool built-in không nhận xác nhận); tech-stack §5; D-26 |
| B-C2 Quyền từ vùng mở | Đã sửa | design §9.3, §15.2 (registry đã ký, mức hiệu lực = max, `risk_overrides` chỉ vendor/core, `trust` do host gán); integration §6.3, §7.4, §10 (eval trong sandbox sạch, ký; diff quyền tách riêng); D-25 |
| B-C4 Nhãn theo client | Đã sửa | design §5.2 (nhãn theo session, khóa ảo theo session, embedding local, cấm chép chéo bank), §5.4, §9.4; integration §3.1, §6.2 (Hindsight, WeKnora không cloud); D-24 |
| B-T1 BPF LSM | Đã sửa | design §3.4 (cgroup tổ tiên, thêm hook, pin, workspace là filesystem riêng), §9 (cả 2 đường thực thi dưới `aios-agent.slice`); integration §3.3, §5.1, S-5; D-27 |
| B-T2 Agent `build` | Đã sửa | Cấu hình và hook mount chỉ đọc; Broker nhận diện client bằng credential (integration §3.2, §4.1, §5.2); proxy gói nội bộ không chạy install script, chặn phiên bản mới, kiểm lockfile, cảnh báo typosquat (integration §5.3); D-46 |
| B-T6 Chuỗi cung ứng | Đã sửa | design §11.5: ghim và kiểm chữ ký/hash cho ảnh, extension, ảnh OCI, plugin, mô hình, LoRA, rule; `revoked.json`; tắt tự cập nhật; sandbox runtime mô hình; SBOM và quét CVE; D-45 |
| B-C3 Rửa provenance | Đã sửa | design §6.3 (taint đơn điệu), §6.5 (cổng thăng cấp chặn tainted, chỉ đếm lần chạy do người dùng khởi phát); integration §6.2 (bank `ext-*`, taint theo session), §6.3; D-28 |
| B-C5 Kênh từ xa và lịch | Đã sửa | integration §8 (tự hành ≤ 1, lọc nhãn đầu ra, yêu cầu chờ duyệt có hạn, lịch là tool cần xác nhận, đích chỉ là kênh đã ghép đôi); design §11.1, §11.2; tech-stack §4 (`forbid`); D-29 |
| B-T3 Giả mạo bề mặt tin cậy, mệt mỏi duyệt | Đã sửa | design §7.7 (làm mờ + khóa nhập, bí mật cá nhân, phím gọi an toàn, bỏ văn bản mô hình cho `high`, chuẩn hóa args, duyệt theo lô, giới hạn tần suất, tên tool thật cạnh nhãn nút), §9.5 (`steps` cho token theo lô); D-30 |
| B-T4 Injection không nhãn | Đã sửa | design §6.3, §7.10 (thông báo là `external`), §10.1, §10.4 (chuỗi thiết bị là `external`, thiết bị lạ không tự kích hoạt L2, cache chỉ chọn adapter chỉ đọc); D-31 |
| B-T5 Cedar | Đã sửa | tech-stack §4 (hai tập `auto`/`confirmable` + `forbid`, context do Broker tính, 4 bất biến symcc); D-32 |
| B-T7 Kill switch | Đã sửa | design §11.3 (`aios-killswitch.service`, freeze slice, thu hồi khóa và token, dừng lịch và kênh từ xa); D-33 |
| Giọng nói (mức Thấp) | Đã sửa | Trần tự hành kênh `voice` là 1 cho tool ghi (design §11.2); PIN chỉ nhập trên bề mặt tin cậy, không đọc thành lời (design §8.2 V-5); clone giọng tắt mặc định (REQ-T-9-01) |
| browser-use, chia sẻ màn hình, plugin `register-reflex-rule` (mức Thấp) | Đã sửa | integration §5.1 (hồ sơ riêng, chặn dải mạng riêng IPv4/IPv6, chống DNS rebinding, MicroVM khi tắt sandbox Chromium); design §7.2 (compositor phát hiện mọi client chụp từ M3, giới hạn ở M1–M2 ghi rõ); design §15.3 (rule từ plugin qua cổng §6.5) |
| C-1 → C-12, C-14, C-15, C-17 | Đã sửa | design §2, §4, §4.4, §6.1, §6.4, §7.8, §7.10, §9, §13, §14, §15; integration §0–§3, §5, §7, §9–§11; tech-stack §0–§3, §5–§8, §10, §12, §13; decisions D-11, D-13, D-14, D-16, chú giải "Thay thế"; feasibility |
| C-13 | Đã sửa qua D-24 | |
| C-16 | Đã xử lý | Ngân sách cloud: do từng người dùng tự đặt (D-43, design §18.2). Distro: giải thích ở tech-stack §2, chờ chủ dự án chấp nhận Fedora bootc (roadmap §5) |
| C-18 | Đã sửa | Chạy lại `scripts/extract_requirements.py` |
| D-1 → D-8 | Đã sửa | tech-stack §1, §3, §6, §8, §12, §13; integration §1, §4.1, §4.2, §5.2, §6.2, §6.3, §9 |
| A-1 Lộ trình, A-2 Độ phức tạp | Đã xử lý | roadmap.md: MVP (M1) có giá trị cho người dùng, mốc M0–M5 + R, ngân sách độ phức tạp, cổng nghiệm thu; D-38 |
| A-3 Compositor theo giai đoạn | Đã xử lý | D-35: compositor có sẵn ở M1–M2, `aios-compositor` ở M3 |
| A-4 Router | Xử lý cho M1 | roadmap.md §2.3: router 2 cấp, bỏ novelty. design §4 vẫn mô tả router 3 cấp đầy đủ cho các mốc sau |
| A-5 Tài nguyên | Đã thiết kế, chưa đo | design §12.7: ngân sách 8 GB cho M1, giới hạn cgroup từng thành phần, xử lý áp lực bộ nhớ theo PSI, quy trình đo S-9 có tính nhiều người dùng. Số đo thật chỉ có sau khi chạy S-9 |
| A-6 Engine L2 | Đã xử lý | D-34: vòng lặp tự viết; opencode từ M4 |
| A-7 bootc và cài đặt lúc chạy | Đã sửa | design §9.6: 5 đường cài (ảnh hệ thống, Flatpak, ảnh OCI ký cho sandbox, system extension ký, toolbox); `pkg.*` không gọi `dnf`; D-44; spike S-24 |
| A-8 UI sinh | Xử lý cho M1 | roadmap.md §2.4: bố cục cố định, chỉ sinh nội dung widget |
| A-9 Tự học, A-10 Driver sinh | Đã xử lý | Dời sang M5 và R |
| A-11 Phần còn thiếu | Đã xử lý | design §12.4 (kiểm thử agent phi tất định: chạy lặp, pass^k, Wilson), §12.5 (trace cục bộ), §12.6 (sao lưu mã hóa, xuất JSONL, migration theo `schema_version`), §17 (bản địa hóa). Giấy phép dự án và số người dùng vẫn là câu hỏi mở (roadmap.md §5) |
| A-12 Kernel sớm | Đã xử lý | BPF LSM dời sang M2 |
| Rà nhất quán lần cuối (2026-10-19) | Đã sửa | 10 mục mức Cao/Trung bình và các mục mức Thấp: worker `aiosd-l2` dưới `aios-agent.slice` để kill switch và BPF LSM áp được cho L2; khóa ảo cho `aiosd-l2` và `aiosd-core-security`; phạm vi BPF LSM chỉ `aios-agent.slice`; runtime `mcp` theo mốc; taint theo trường của sự kiện an ninh; mốc của vùng mở (R); D-13, D-14, D-17, D-19; bảng cổng thêm Lemonade; hành vi khi không có route cho dữ liệu `secret`; README và roadmap |
| Nhiều người dùng (yêu cầu mới 2026-10-19) | Đã thiết kế | design §18; BPF LSM theo map `agent_slices` (§3.4); Cedar `is_admin`, `system_scope` (tech-stack §4); Hindsight mỗi người dùng một instance (integration §6.2, §11.1); D-42, D-43 |
