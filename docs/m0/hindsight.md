# hindsight — Spike S-4: Hindsight không torch, qua Model Gateway

| Thuộc tính | Giá trị |
| --- | --- |
| Người làm | worker |
| Phụ thuộc | ld-vm, gateway, llm |
| Chặn | ld-gate |
| Nguồn thiết kế | integration.md §6.2, §11.1; design.md §18.1, §18.4; roadmap.md §3 (S-4); review.md §5 (biến môi trường Hindsight) |

## Mục tiêu

Trả lời câu hỏi của S-4: Hindsight có chạy được trong VM với LLM, embedding và reranker đi qua Model Gateway, môi trường Python không có torch, và tốn bao nhiêu RAM. Kết quả dùng làm nền cho `aiosd-memory` ở M1.

## Phạm vi

Được sửa: `dev/vm/hindsight/` (script cài, unit systemd, file env mẫu), `docs/spikes/S-4-hindsight.md`.

## Thiết kế

- Postgres 18 + pgvector từ gói Fedora; một database và một role cho người dùng thử (`aios_u<uid>`), role không có quyền trên database khác (design §18.4).
- Cài `hindsight-api-slim` bằng `uv` vào venv riêng `/opt/aios/hindsight/`, **không** cài extra `local-ml`. Ghim phiên bản trong `uv.lock`.
- File env theo integration.md §6.2: LLM, embedding qua `http://127.0.0.1:8090/v1` với khóa dịch vụ `hindsight` (`cloud: false`). Reranker: chọn provider không cần torch; nếu Hindsight không có provider như vậy → thử tắt rerank; không tắt được → `NEED_ADVICE`.
- Unit `hindsight-api@.service` (instance theo uid), nghe `127.0.0.1:8888`, systemd sandboxing như integration.md §5.1 (dòng Hindsight).
- Tạo đủ 7 bank: `user-public`, `user-personal`, `user-secret`, `system`, `ext-public`, `ext-personal`, `ext-secret`.

## Kiểm thử và nghiệm thu

Script `dev/vm/hindsight/probe.py` (dùng `hindsight-client`):

| Bước | Mong đợi |
| --- | --- |
| `retain` 20 câu tiếng Việt vào `user-personal` | Thành công |
| `recall` "tôi làm việc lúc mấy giờ" | Có câu liên quan trong top 3 |
| Bank `ext-personal` tách khỏi `user-personal` | `recall` trên bank này không thấy câu của bank kia |
| Gateway log | Mọi lệnh gọi LLM, embedding của Hindsight đi route local |

```bash
limactl shell aios-dev -- bash -lc 'cd ~/build/AiOS && bash dev/vm/hindsight/install.sh && /opt/aios/hindsight/bin/python dev/vm/hindsight/probe.py'
limactl shell aios-dev -- bash -lc '/opt/aios/hindsight/bin/pip list 2>/dev/null | grep -iE "^(torch|sentence-transformers) " ; test $? -eq 1'
limactl shell aios-dev -- bash -lc 'systemctl show hindsight-api@$(id -u) -p MemoryCurrent'
```

`docs/spikes/S-4-hindsight.md` ghi: phiên bản, biến môi trường đã dùng, RSS khi rảnh và khi `recall`, dung lượng venv, các vấn đề gặp, kết luận cho D-18.

## Ngoài phạm vi

`aiosd-memory` (cổng nhãn và taint, M1), WeKnora (M4).
