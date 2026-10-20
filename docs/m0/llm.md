# llm — Mô hình local trong VM

| Thuộc tính | Giá trị |
| --- | --- |
| Người làm | worker |
| Phụ thuộc | ld-vm |
| Chặn | hindsight |
| Nguồn thiết kế | design.md §5, §11.5 (ghim hash mô hình, sandbox runtime), §12.7; tech-stack.md §6; REQ-D-11.5-01, REQ-D-11.5-04 |

## Mục tiêu

Trong VM `aios-dev`, chạy `llama-server` cho một mô hình chat có tool calling (cổng 8081) và một mô hình embedding (cổng 8082) dưới systemd có sandboxing, với file mô hình được ghim SHA-256 trong `models.lock`.

## Phạm vi

Được sửa: `models.lock`, `dev/vm/llm/` (script tải, unit systemd), `docs/spikes/S-13-m0.md` (ghi chú đo nhanh).

## Thiết kế

- **Chọn mô hình chat.** Tiêu chí: GGUF lượng tử hóa khoảng 4 bit, ≤ 3 GB, giấy phép Apache-2.0 hoặc MIT, hỗ trợ tool calling qua template chat của `llama-server --jinja`, có tiếng Việt. Ứng viên theo tech-stack §6: họ Qwen cỡ 4B. Worker chọn đúng một file cụ thể, ghi URL, giấy phép, SHA-256. Chọn mô hình ngoài tiêu chí → `NEED_ADVICE`.
- **Chọn mô hình embedding.** multilingual-e5-large-instruct dạng GGUF nếu có; không có thì `NEED_ADVICE` kèm phương án.
- **`models.lock`** (TOML): mỗi mô hình có `name`, `role` (`s1`/`s2`/`embedding`), `url`, `sha256`, `size`, `license`. Script tải kiểm SHA-256 sau khi tải; sai → xóa file, thoát lỗi.
- **Unit systemd** `aios-llm-chat.service`, `aios-llm-embed.service` (user service trong VM M0): `ExecStart=llama-server --host 127.0.0.1 --port 8081 -m <file> --jinja -c 8192` (embedding: `--embeddings --port 8082`); `PrivateNetwork=` không dùng được vì cần loopback, thay bằng `IPAddressDeny=any` + `IPAddressAllow=localhost`; `ProtectSystem=strict`, `ReadOnlyPaths=` thư mục mô hình, `NoNewPrivileges=yes`, `MemoryMax=4G` (chat), `1.5G` (embedding).
- Thư mục mô hình trong VM: `/var/lib/aios/models/<name>/<sha256-ngắn>/`.

## Kiểm thử và nghiệm thu

```bash
limactl shell aios-dev -- bash -lc 'cd ~/build/AiOS && bash dev/vm/llm/install.sh'
limactl shell aios-dev -- bash -lc 'curl -s 127.0.0.1:8081/v1/chat/completions -d @dev/vm/llm/tool-call-probe.json | jq .choices[0].message.tool_calls'
limactl shell aios-dev -- bash -lc 'curl -s 127.0.0.1:8082/v1/embeddings -d "{\"input\":\"xin chào\"}" | jq ".data[0].embedding|length"'
limactl shell aios-dev -- bash -lc 'systemctl --user show aios-llm-chat -p MemoryCurrent'
```

Đạt khi: tool call trả về đúng tên tool và args JSON hợp lệ cho câu hỏi mẫu tiếng Việt; embedding trả vector (độ dài theo mô hình); hash khớp `models.lock`; `docs/spikes/S-13-m0.md` ghi RAM và độ trễ trung bình 10 lần gọi.

## Ngoài phạm vi

Benchmark tiếng Việt đầy đủ VMLU (S-13 đầy đủ, chờ), mô hình cloud.
