# AiOS

Hệ điều hành tự hành dựa trên Linux, tích hợp mô hình ngôn ngữ lớn (LLM). AiOS phản hồi qua 3 cấp (code logic, System 1, System 2), tự học, giao tiếp bằng giọng nói, dùng công cụ, tự khám phá phần cứng mới, và lấy giao diện chatbot làm màn hình chính thay cho desktop environment truyền thống.

> **Trạng thái:** giai đoạn thiết kế, chưa có mã nguồn. \[Inference\] Toàn bộ tài liệu là thiết kế đề xuất, chưa được kiểm chứng bằng triển khai; số liệu độ trễ, tài nguyên và ngưỡng là ước lượng.

## Tài liệu

| File | Nội dung |
| --- | --- |
| [docs/design.md](docs/design.md) | Thiết kế chi tiết: kiến trúc, tích hợp kernel, 3 cấp phản hồi, Model Gateway, bộ nhớ và tự học, UI sinh, giọng nói, công cụ, phần cứng, bảo mật, vận hành, lộ trình, rủi ro |
| [docs/overview.md](docs/overview.md) | Tổng quan một trang, bản đồ tài liệu, thuật ngữ |
| [docs/roadmap.md](docs/roadmap.md) | Phạm vi MVP (M1), các mốc M0–M5 + R, cổng nghiệm thu, ngân sách độ phức tạp |
| [docs/integration.md](docs/integration.md) | Kiến trúc tích hợp các thành phần mã nguồn mở (linux, WeKnora, Hindsight, opencode, rea, ArtCraft, OpenShell, gpui-kit, browser-use, open-code-review, Harness², bộ công cụ an ninh; deepseek-harness và hermes-agent làm mẫu tham khảo) |
| [docs/tech-stack.md](docs/tech-stack.md) | Nghiên cứu tech stack cho từng tầng, kèm nguồn tham khảo và các spike kiểm chứng |
| [docs/requirements.md](docs/requirements.md) | Danh sách yêu cầu (PHẢI / KHÔNG ĐƯỢC / NÊN / CÓ THỂ) trích từ design.md, integration.md, roadmap.md, tech-stack.md bằng `scripts/extract_requirements.py` |
| [docs/decisions.md](docs/decisions.md) | Nhật ký quyết định thiết kế và trạng thái từng quyết định |
| [docs/feasibility.md](docs/feasibility.md) | Đánh giá khả thi và khuyến nghị giảm rủi ro |
| [docs/review.md](docs/review.md) | Review và phản biện toàn bộ tài liệu (2026-10-18): chiến lược, bảo mật, nhất quán, kiểm chứng kỹ thuật |

Đọc theo thứ tự: overview.md → decisions.md → roadmap.md → design.md → integration.md → tech-stack.md → requirements.md.

## Kiến trúc tóm tắt

- **Kernel** (Linux LTS): chạy eBPF cho phản xạ và hàng rào an toàn (BPF LSM), phát sự kiện. LLM không chạy trong kernel.
- **aiosd** (user space): `aiosd-core` bằng Rust (Reflex, Policy Engine, Tool Broker), plugin host; `aiosd-cognition` và `aiosd-memory` bằng Python.
- **Model Gateway**: định tuyến giữa AI local và cloud AI theo nhãn riêng tư, mạng và ngân sách cloud riêng của từng người dùng.
- **Nhiều người dùng**: mỗi người dùng độc lập (bộ nhớ, kỹ năng, khóa và ngân sách cloud riêng) (design.md §18).
- **Desktop environment tự viết**: `aios-compositor` (Smithay, từ M3) + `aios-shell` và `aios-trusted-ui` (gpui-kit); M1–M2 chạy app chat trên compositor có sẵn (niri hoặc cosmic-comp). Hội thoại là màn hình chính, UI sinh render từ UI IR (integration.md §7).
- **System 2**: vòng lặp tự viết, chạy trong worker `aiosd-l2` do `aiosd-cognition` khởi chạy (MVP); opencode thêm vào từ M4 làm coding agent và engine thay thế. Mọi tool đi qua Tool Broker.
- **Hệ plugin**: mọi tính năng ngoài lõi tin cậy là plugin `wasm`, `process` hoặc `mcp` (design.md §15).
- **An ninh**: phòng thủ nhiều lớp chống mã độc, sâu máy tính, tấn công mạng; AI phân loại và điều tra cảnh báo nhưng chỉ đề xuất (design.md §16). Mã do agent sinh qua cổng review open-code-review (integration.md §13).

Sơ đồ chi tiết ở design.md §2.

## Quy ước nhãn

| Nhãn | Ý nghĩa |
| --- | --- |
| \[Inference\] | Suy luận kỹ thuật, chưa đo trên AiOS |
| \[Speculation\] | Hướng nghiên cứu, chưa có bằng chứng khả thi |
| \[Unverified\] | Cần kiểm tra lại trên phiên bản thực tế |
| UNKNOWN | Thiếu dữ liệu đầu vào từ chủ dự án |

## Giấy phép

AiOS là dự án mã nguồn mở theo giấy phép Apache-2.0 (D-48). Riêng mã BPF chạy trong kernel dùng giấy phép kép GPL-2.0 hoặc BSD, vì kernel chỉ cho chương trình BPF khai giấy phép tương thích GPL gọi các helper GPL-only. Lưu ý các ràng buộc giấy phép của thành phần bên thứ ba ở tech-stack.md §13; oh-my-openagent không được đóng gói vì giấy phép SUL-1.0 không phải mã nguồn mở (D-20).
