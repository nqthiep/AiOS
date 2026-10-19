# AiOS — Đánh giá khả thi

_Cập nhật: 2026-10-18_

**\[Inference\] Toàn bộ đánh giá là suy luận kỹ thuật từ kiến thức hiện có; chưa đối chiếu với benchmark mô hình hoặc phần cứng mới nhất.**

**Kết luận: nền kỹ thuật khả thi. Rủi ro chính nằm ở chất lượng UI sinh, độ tin cậy của hành động tự động, và tài nguyên phần cứng, không nằm ở tích hợp kernel.**

## Khả thi cao

Công nghệ đã có; phần việc chính là tích hợp.

- Tích hợp kernel bằng eBPF, BPF LSM, cgroup v2, Landlock (design.md §3).
- Model Gateway local/cloud: llama.cpp và vLLM có API OpenAI-compatible (design.md §5).
- Hệ thống tool: MCP, bubblewrap, Landlock, polkit (design.md §9).
- Bộ nhớ episodic và semantic (design.md §6.1).
- Khám phá thiết bị thuộc các subsystem chuẩn (design.md §10.2).

## Khả thi trung bình

Làm được, nhưng chất lượng cần nhiều vòng eval.

- Router 3 cấp: kiến trúc đơn giản, nhưng hiệu chỉnh độ tin cậy System 1 và đo novelty cần dữ liệu thật.
- Giọng nói tiếng Việt: linh kiện đã có sẵn; chất lượng STT/TTS tiếng Việt, độ trễ trên CPU và khử vọng trên mic laptop chưa chắc chắn.
- Màn hình chính dạng chat kèm widget: gần với các chatbot agent hiện nay.
- Tác vụ System 2 nhiều bước: tỷ lệ thành công giảm khi số bước tăng.
- Chưng cất trajectory System 2 thành script.
- Chống prompt injection: policy và sandbox chỉ giảm thiểu rủi ro; hiện chưa có lời giải triệt để.

## Khả thi thấp

Hướng nghiên cứu.

- UI hệ điều hành sinh hoàn toàn ở chất lượng sản phẩm: cần compositor, renderer và design system tự xây, đồng thời phải ổn định, nhanh và dễ dùng.
- Tự biên dịch rule Reflex và huấn luyện LoRA trên máy người dùng mà không làm hành vi xấu đi.
- Driver user space do LLM sinh cho thiết bị lạ.
- Agent tự sửa mã trong vùng mở (D-14 trong decisions.md): đã có thiết kế ở integration.md §10, chưa có phương pháp kiểm chứng chất lượng thay đổi ngoài bộ eval.

## Ràng buộc phần cứng (ước lượng)

- System 1 (1–4B tham số, q4), STT, TTS và embedding chạy thường trú cần vài GB RAM; máy 16 GB RAM là mức tối thiểu hợp lý.
- System 2 local cỡ lớn cần GPU nhiều VRAM; trên phần cứng phổ thông, System 2 thực tế sẽ chạy trên cloud.

## Khuyến nghị giảm rủi ro

1. Xây `aiosd` và app chat thành MVP (mốc M1 trong [roadmap.md](roadmap.md)), chạy trên compositor có sẵn (niri hoặc cosmic-comp, D-35); compositor tự viết đến M3.
2. Đo sớm 3 chỉ số quyết định: tỷ lệ thành công tác vụ System 2, tỷ lệ hoàn tác hành động tự động, độ trễ giọng nói tiếng Việt.
3. Hoãn Reflex tự biên dịch, LoRA và driver tự sinh cho đến khi có số đo.

## Rủi ro bổ sung từ nghiên cứu tech stack

Chi tiết ở tech-stack.md §13:

- Máy có NPU AMD cần driver `amdxdna` trong kernel 7.x; kernel của Fedora (D-47, hiện 7.2) đã có. Đổi lại, kernel Fedora đổi phiên bản mỗi vài tuần nên cần CI kiểm BPF trên từng kernel mới.
- STT tiếng Việt (Zipformer-vi) nhận sai tên riêng, số và thuật ngữ tiếng Anh; cần STT 2 lượt.
- Bộ gõ tiếng Việt trên compositor tự xây (Smithay) chưa được kiểm chứng. Chủ dự án đánh giá rủi ro thấp và sẽ cung cấp cách hiện thực sau.

Mức khả thi tổng thể còn phụ thuộc phần cứng mục tiêu (roadmap.md §5) và năng lực của coding agent hiện thực dự án (D-50, roadmap.md §6).
