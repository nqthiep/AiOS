# ld-gate — Tích hợp và cổng nghiệm thu M0

| Thuộc tính | Giá trị |
| --- | --- |
| Người làm | leader |
| Phụ thuộc | executor, cognition, hindsight, s14 |
| Chặn | — |
| Nguồn thiết kế | roadmap.md §3 (cổng M0), §2.8 (fail-safe), §12.7 của design.md (S-9) |

## Mục tiêu

Chạy toàn bộ M0 trong VM và xác nhận cổng: `aiosctl ask` chạy hết một yêu cầu dùng cả 3 tool; tắt các dịch vụ `aiosd-*` thì OS vẫn dùng như Linux thường; ghi baseline tài nguyên S-9.

## Phạm vi

Leader sửa: `dev/vm/gate/` (script cổng), `docs/spikes/S-9-baseline-m0.md`, cập nhật `docs/roadmap.md` và `docs/decisions.md` theo kết quả.

## Thiết kế

Kịch bản cổng (chạy trong VM, user mặc định của VM, autonomy 1):

| # | Bước | Mong đợi |
| --- | --- | --- |
| 1 | `aiosctl status` | Mọi thành phần sẵn sàng; registry có 4 tool |
| 2 | `aiosctl ask "trạng thái máy"` | L0, `sys.info`, không gọi mô hình |
| 3 | `aiosctl ask "tìm file kế hoạch rồi ghi một ghi chú tóm tắt kết quả tên là tom-tat"` | L2; gọi `fs.search` (low) chạy ngay; gọi `fs.write_note` (medium) → chờ xác nhận; `aiosctl confirm` ký bằng bộ ký dev → ghi file |
| 4 | Kiểm session | Nhãn tối thiểu `personal`, taint `tainted` (vì `fs.search` trả `external`); nếu đã cấu hình cloud, Gateway log không có lượt nào vượt trần riêng tư của người dùng |
| 5 | `aiosctl ask "hoàn tác ghi chú <undo_id>"` (undo_id in ra ở bước 3; M0 chưa có bộ nhớ hội thoại) | `fs.undo_note` chạy ngay (session mới, sạch, tool low); file trở về trạng thái trước bước 3 |
| 6 | `aiosctl audit verify` | ok, có đủ dòng của bước 2–5 |
| 7 | `aiosctl freeze`; gọi lại bước 2 | Tool không chạy; `unfreeze` → chạy lại được |
| 8 | Dừng `aiosd-core`, Gateway, mô hình | Shell, mạng, `dnf`, `systemctl` của VM vẫn dùng bình thường; `aiosctl status` báo không sẵn sàng, không treo |
| 9 | S-9 | RSS từng dịch vụ khi rảnh và trong bước 3; ghi báo cáo |

Cổng đạt khi bước 1–8 đạt. Bước 9 chỉ ghi số đo, chưa so với ngưỡng 8 GB (máy ảo arm64 không phải phần cứng mục tiêu D-53).

## Kiểm thử và nghiệm thu

```bash
dev/vm-run.sh 'bash dev/vm/gate/run.sh'
```

## Ngoài phạm vi

Ngưỡng M1 (D-52), app chat, trusted UI thật.
