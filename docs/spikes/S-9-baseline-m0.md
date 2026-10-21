# S-9 (M0) — Cổng M0 và baseline tài nguyên trong VM `aios-dev`

_Ngày đo: 2026-10-21 · Task: `ld-gate` (thiết kế: [docs/m0/ld-gate.md](../m0/ld-gate.md)) · Script: `dev/vm/gate/setup.sh`, `dev/vm/gate/run.sh`_

> Đây là baseline đo trong máy ảo arm64, **không phải** phần cứng mục tiêu D-53 (x86_64, 16 GB, iGPU Vulkan). Số liệu chỉ để so sánh giữa các lần đo trong cùng VM, chưa so với ngưỡng 8 GB của M1.

## 1. Môi trường

| Mục | Giá trị |
| --- | --- |
| VM | Lima `aios-dev`, Fedora 44 aarch64, kernel `7.2.11-200.fc44.aarch64`, 4 vCPU, 7 906 MiB RAM, swap 7 905 MiB |
| Mô hình chat (S1, cũng dùng cho route `s2`) | `Qwen3-4B-Instruct-2507` Q4_K_M, `llama-server` chỉ CPU (S-13-m0) |
| Mô hình embedding | `multilingual-e5-large-instruct` Q4_K_M |
| Cloud | Không cấu hình; Gateway chỉ có route local (`dev/vm/gate/gateway.yaml`) |
| Autonomy | 1 (`$GATE/autonomy.yaml`) |
| Dịch vụ | `aiosd-core`, `aios-model-gateway`, `aios-llm-chat`, `aios-llm-embed`: unit systemd user; `hindsight-api@501`, `postgresql`: unit system |

`aiosd-core` chạy dưới dạng unit user vì Executor gọi `systemd-run --user` (xem §5).

## 2. Kết quả cổng (lần chạy 3, đạt)

Lệnh: `dev/vm-run.sh 'bash dev/vm/gate/setup.sh && bash dev/vm/gate/run.sh'` → `CỔNG M0: ĐẠT`, `rc=0`.

| Bước | Kết quả | Chi tiết đo được |
| --- | --- | --- |
| 1 status | đạt | core + gateway sẵn sàng, registry 4 tool, models `s1,s2` |
| 2 L0 | đạt | `trạng thái máy` → L0, `sys.info` ok, 0 token mô hình |
| 3 L2 + xác nhận | đạt | `fs.search {"pattern":"*kế*hoạch*"}` (low, chạy ngay) → `fs.write_note {"name":"tom-tat",…}` (medium) chờ xác nhận → `aiosctl confirm` ký bằng bộ ký dev → ghi `notes/tom-tat.md`; 116 s; 2 665 token |
| 4 nhãn session | đạt | Gateway trả `personal/tainted` (do `fs.search` trả `external`); 0 lượt cloud |
| 5 hoàn tác | đạt | `hoàn tác ghi chú <undo_id>` → `fs.undo_note` ok; `notes/tom-tat.md` bị xóa (trạng thái trước bước 3) |
| 6 audit verify | đạt | `ok 43 dòng`; dòng mới có đủ 4 tool ok và bản ghi `confirm` |
| 7 freeze | đạt | khi đóng băng: `sys.info` lỗi `execution_failed` (hết thời gian); sau `unfreeze`: `sys.info` ok |
| 8 fail-safe | đạt | dừng `aiosd-core`, Gateway, 2 mô hình, Hindsight → shell, mạng (`curl https://fedoraproject.org/`), `dnf`, `systemctl` vẫn dùng được (`running`); `aiosctl status` trả trong 258 ms, báo không sẵn sàng |

**Lưu ý về chất lượng nội dung bước 3.** `fs.search` chỉ trả tên file, không trả nội dung, nên "tóm tắt kết quả" mà mô hình ghi vào ghi chú có câu không có trong file mẫu (mô hình tự thêm "hoàn thiện hệ thống quản lý dự án"). Cổng M0 chỉ kiểm luồng tool, xác nhận và nhãn, không kiểm độ đúng nội dung; độ đúng thuộc bộ kịch bản vàng của S-13 đầy đủ.

### Các lần chạy trước (không đạt) và nguyên nhân

| Lần | Bước hỏng | Nguyên nhân | Đã sửa |
| --- | --- | --- | --- |
| 1 | 3–7 | `llama-server` trả HTTP 400: không dịch được `\p{L}` trong `pattern` của schema `fs.write_note` sang grammar; VM thiếu `script` nên `unfreeze` không chạy | `aiosctl` bỏ từ khóa `pattern` (chuỗi regex) khỏi schema gửi mô hình, Broker vẫn kiểm schema đầy đủ; `setup.sh` cài `util-linux-script` |
| 2 | 3, 5, 6 | Mô hình tìm `khoan*` (sai từ khóa, chỉ khớp đầu tên), không thấy file rồi dừng để hỏi lại người dùng | Prompt L2 yêu cầu làm hết các bước; mô tả `fs.search` hướng dẫn đặt `*` giữa các từ |

Lỗi tích hợp tìm thấy và sửa trước lần chạy 1 (khi dựng cổng): Broker thiếu `resultType` trong kết quả MCP (client Python 2026-07-28 từ chối); Executor không truyền `AIOS_WORKSPACE`; đường dẫn cgroup của `aiosctl freeze` thiếu `aios.slice/`; luật L0 `tìm file …` bắt cả câu ghép; `sys.info` cần quyền đọc workspace trong sandbox.

\[Inference\] Bước 3 phụ thuộc vào lựa chọn của mô hình 4B, nên một lần đạt chưa chứng minh bước này ổn định. Kết quả lần chạy lặp ghi ở §4.

## 3. Baseline tài nguyên (lần chạy 3)

Lấy mẫu mỗi giây trong bước 3; RSS từ `/proc/<pid>/status`, `MemoryCurrent` từ systemd. Đơn vị MiB.

| Dịch vụ | RSS rảnh | MemoryCurrent rảnh | RSS đỉnh bước 3 | MemoryCurrent đỉnh bước 3 |
| --- | --- | --- | --- | --- |
| `aiosd-core` | 27 | 34 | 39 | 44 |
| `aios-model-gateway` | 15 | 18 | 15 | 19 |
| `aios-llm-chat` | 3 621 | 3 627 | 3 738 | 3 746 |
| `aios-llm-embed` | 528 | 531 | 532 | 535 |
| `hindsight-api@501` | 196 | 163 | 401 | 378 |
| `postgresql` (tiến trình chính) | 29 | 47 | 29 | 76 |
| **Tổng RSS** | **4 416** | | **4 754** | |

- RSS của `aios-llm-chat` gồm khoảng 2,33 GiB trang file mô hình mmap (S-13-m0 §5). Ở lần đo này `MemoryCurrent` xấp xỉ RSS, khác với S-13-m0 (lúc đó trang cache bị tính vào cgroup gốc): \[Inference\] do VM vừa boot lại, trang mô hình được nạp lần đầu trong cgroup của unit. Trần `MemoryMax=4G` của unit chat chỉ còn dư khoảng 350 MiB lúc đỉnh.
- `free -m` sau cổng: used 2 042 MiB, buff/cache 4 166 MiB, available 5 863 MiB, swap dùng 0.
- Tool chạy trong scope tạm dưới `aios-agent.slice` và thoát ngay sau mỗi lần gọi; bộ lấy mẫu không ghi RSS của chúng.
- `aiosd-core` + Gateway dưới 60 MiB cả khi bận; phần lớn RAM là mô hình local.

## 4. Lần chạy lặp (lần 4, đạt)

Chạy ngay sau lần 3, không chạy lại `setup.sh`: `CỔNG M0: ĐẠT`, `rc=0`. Bước 3 đạt sau 82 s, 2 722 token, cùng chuỗi `fs.search {"pattern":"*kế*hoạch*"}` → `fs.write_note`; nội dung ghi chú lại có câu mô hình tự thêm (§2). Bước 8: `aiosctl status` trả trong 281 ms.

| Dịch vụ | RSS rảnh | MemoryCurrent rảnh | RSS đỉnh bước 3 | MemoryCurrent đỉnh bước 3 |
| --- | --- | --- | --- | --- |
| `aiosd-core` | 25 | 13 | 37 | 23 |
| `aios-model-gateway` | 15 | 9 | 15 | 10 |
| `aios-llm-chat` | 3 622 | 1 243 | 3 737 | 1 359 |
| `aios-llm-embed` | 529 | 138 | 529 | 138 |
| `hindsight-api@501` | — | — | 387 | 348 |
| `postgresql` | 29 | 58 | 29 | 58 |

- RSS gần như bằng lần 3; `MemoryCurrent` thấp hơn nhiều. \[Inference\] Nguyên nhân có thể là bước 8 của lần 3 đã khởi động lại các dịch vụ, trang file mô hình nằm sẵn trong page cache nên không bị tính vào cgroup của unit (giống S-13-m0 §5). Vì vậy `MemoryCurrent` không dùng được làm số RAM thật của mô hình mmap; baseline nên đọc theo RSS.
- Lúc lấy mẫu rảnh, `hindsight-api@501` không có PID chính. \[Inference\] Dịch vụ vừa được bước 8 của lần 3 khởi động lại và chưa chạy xong.

Hai lần liên tiếp (3 và 4) đạt cả 8 bước; lần 2 hỏng ở bước 3 với prompt cũ. \[Inference\] Chưa đủ số lần chạy để ước lượng tỉ lệ đạt của bước 3 với mô hình 4B.

## 5. Vấn đề còn mở (chuyển sang M1)

1. `aiosd-core` phải chạy dưới dạng unit user, vì Executor dùng `systemd-run --user`; ảnh `image/` đang khai `aiosd-core.service` là dịch vụ hệ thống. Cần chốt mô hình chạy theo từng người dùng (design §18) trước khi boot ảnh.
2. Kill switch: `aiosctl freeze` đóng băng được slice nhưng không thu hồi khóa ảo (không có `/v1/revoke-all` trên `aiosd-core`); `aios-killswitch.service` trong ảnh đóng băng nhầm cấp (system thay vì user slice).
3. Khóa ảo của Gateway chỉ nằm trong bộ nhớ: khởi động lại Gateway thì Hindsight nhận 401 cho tới khi `setup.sh` cấp lại khóa.
4. Bộ ký dev ghi `PendingConfirm` ra file 0644 (thư mục 0711) trong lúc ký; M1 thay bằng `aios-trusted-ui`.
5. Độ đúng nội dung của L2 (bước 3) chưa được đo; cần bộ kịch bản vàng S-13 trên phần cứng mục tiêu.
6. Spike S-17 (`ext-session-lock-v1`) và S-24 (`systemd-sysext` trên ảnh bootc) có trong danh sách spike M0 của roadmap nhưng chưa có trong kế hoạch task M0 và **chưa làm**.
