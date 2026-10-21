# dev/vm/llm — Mô hình local trong VM `aios-dev`

Cài `llama-server` cho một mô hình chat có tool calling và một mô hình embedding
trong VM `aios-dev` (Fedora 44), chạy dưới systemd **user** service có sandboxing.
Nguồn chân lý duy nhất cho file mô hình là [`models.lock`](../../../models.lock)
ở gốc repo (URL, SHA-256, kích thước, giấy phép).

| Thành phần | Cổng | Unit systemd user | Mô hình |
| --- | --- | --- | --- |
| Chat (tool calling) | `127.0.0.1:8081` | `aios-llm-chat.service` | `Qwen3-4B-Instruct-2507` — GGUF Q4_K_M, Apache-2.0 |
| Embedding | `127.0.0.1:8082` | `aios-llm-embed.service` | `multilingual-e5-large-instruct` — GGUF Q4_K_M, MIT |

Thiết kế chi tiết: [docs/m0/llm.md](../../../docs/m0/llm.md).
Đo nhanh: [docs/spikes/S-13-m0.md](../../../docs/spikes/S-13-m0.md).

## Cài đặt

```bash
# Từ worktree trên Mac: chép bản sao worktree vào VM rồi chạy
dev/vm-run.sh 'bash dev/vm/llm/install.sh'

# Hoặc trong VM, từ bản repo đã chia sẻ
limactl shell aios-dev -- bash -lc 'cd /Users/quangthiep/Projects/AiOS && bash dev/vm/llm/install.sh'
```

`install.sh` làm ba việc:

1. đọc `models.lock`;
2. tải từng file GGUF bằng `curl` vào
   `/var/lib/aios/models/<name>/<12 ký tự đầu của sha256>/` và **kiểm SHA-256**
   sau khi tải — sai thì xóa file và thoát lỗi; chạy lại thì file đã đúng hash
   không bị tải lại;
3. render `aios-llm-*.service.in` thành `~/.config/systemd/user/aios-llm-*.service`
   rồi `systemctl --user enable` + `restart`, và đợi `/health` trả về OK.

Biến môi trường để đổi đường dẫn/cổng: `AIOS_MODELS_LOCK`, `AIOS_MODEL_ROOT`,
`AIOS_LLM_CHAT_PORT`, `AIOS_LLM_EMBED_PORT`, `AIOS_LLM_CHAT_MEMORY_MAX`,
`AIOS_LLM_EMBED_MEMORY_MAX`.

## Kiểm tra

```bash
# tool calling (tiếng Việt)
curl -s 127.0.0.1:8081/v1/chat/completions -d @dev/vm/llm/tool-call-probe.json \
  | jq .choices[0].message.tool_calls

# embedding
curl -s 127.0.0.1:8082/v1/embeddings -d '{"input":"xin chào"}' \
  | jq '.data[0].embedding|length'

# RAM của từng service
systemctl --user show aios-llm-chat -p MemoryCurrent
systemctl --user show aios-llm-embed -p MemoryCurrent
```

## Lưu ý sandbox

* `PrivateNetwork=` không dùng được (server cần loopback để Model Gateway gọi),
  thay bằng `IPAddressDeny=any` + `IPAddressAllow=localhost`.
* **systemd chỉ thực thi tường lửa IP cho unit của system manager.** Unit user
  của `systemctl --user` chỉ ghi cảnh báo `not running as root` và **không** lọc
  IP. Muốn lọc IP thật thì phải nâng unit lên system manager (`sudo`).
* `ProtectSystem=strict` + `ReadOnlyPaths=<thư mục mô hình>`: server chỉ đọc,
  không ghi được ra hệ thống file.
