# image — Containerfile ảnh AiOS

| Thuộc tính | Giá trị |
| --- | --- |
| Người làm | worker |
| Phụ thuộc | ld-setup |
| Chặn | s16 |
| Nguồn thiết kế | tech-stack.md §2, §2.1 (Fedora bootc, đổi thương hiệu); design.md §9.6, §11.5; D-47 |

## Mục tiêu

`image/Containerfile` dựng được ảnh bootc của AiOS từ Fedora 44, ghim digest, đã đổi thương hiệu, có các binary và unit systemd của M0, và qua `bootc container lint`.

## Phạm vi

Được sửa: `image/**`, mục tiêu `image` trong `Makefile` (chỉ thêm dòng).

## Thiết kế

- **Nền:** `FROM quay.io/fedora/fedora-bootc:44@sha256:<digest>`. Worker tra digest hiện tại của tag `44` cho cả `linux/amd64` và `linux/arm64`, ghi vào `image/base.lock` kèm ngày tra. Không dùng `:latest`.
- **Build nhiều tầng:** tầng `builder` (cùng nền, cài `rust`, `cargo`, `uv`) build workspace ở chế độ release; tầng cuối chỉ chép binary và gói Python đã build.
- **Đổi thương hiệu:** thay `fedora-logos`, `fedora-release` bằng `generic-logos`, `generic-release` của Fedora \[Unverified\] tên gói trong F44; đặt `/usr/lib/os-release` với `NAME="AiOS"`, `ID=aios`, `ID_LIKE=fedora`, `VERSION_ID=0.0.1-m0`.
- **Gói:** `bubblewrap`, `firewalld`, `postgresql-server`, `postgresql18-pgvector`, `llama-cpp`, `python3`, `uv`, `flatpak`.
- **Unit:** `aiosd-core.service`, `aios-model-gateway.service` (hệ thống), `aios-agent.slice` (user), `aios-killswitch.service`; `systemctl enable` cho 2 dịch vụ đầu và `firewalld`.
- **firewalld:** zone mặc định `drop` cho mạng vào; không mở dịch vụ nào (design §16.1 lớp 1). SSH tắt.
- **Thư mục:** `/usr/share/aios/{plugins,trust,policy}` (chép `policy/*.cedar`, `plugins/tools/*` đã ký), `tmpfiles.d` cho `/var/lib/aios/...`.
- **Không có trong ảnh:** mô hình (tải theo `models.lock` khi cài), khóa ký dev, khóa API.

## Kiểm thử và nghiệm thu

```bash
docker build --platform linux/arm64 -f image/Containerfile -t aios:m0 .      # trên Mac
docker run --rm aios:m0 bootc container lint
docker run --rm aios:m0 cat /usr/lib/os-release | grep -q '^ID=aios$'
docker run --rm aios:m0 rpm -q fedora-logos ; test $? -ne 0                  # đã gỡ thương hiệu Fedora
docker run --rm aios:m0 systemctl is-enabled aiosd-core firewalld
```

## Ngoài phạm vi

Boot ảnh trên máy thật hoặc VM (`bootc install`, S-24), ký ảnh bằng cosign (M1), bản amd64 build trên máy x86.
