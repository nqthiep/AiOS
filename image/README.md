# image — ảnh hệ thống AiOS (bootc)

Ảnh bootc của AiOS dựng từ ảnh nền Fedora 44 (`quay.io/fedora/fedora-bootc:44`)
đã ghim digest, đã đổi thương hiệu, có binary Rust, gói Python và các unit
systemd của M0. Thiết kế: [docs/m0/image.md](../docs/m0/image.md).

## Dựng ảnh

```bash
make image
# tương đương:
docker build --platform linux/arm64 -f image/Containerfile -t aios:m0 .
```

Trên máy x86 thì đổi `--platform linux/arm64` thành `linux/amd64`: digest
nền ghim theo index multi-arch nên cùng một `Containerfile` dựng được cả hai.

## Cấu trúc

| Đường dẫn | Vai trò |
| --- | --- |
| `Containerfile` | Ba tầng: `builder` (Rust + `uv`), `assets` (gom `policy/`, `plugins/`), `runtime` (ảnh cuối) |
| `Containerfile.dockerignore` | Cắt context build (`.git/`, `target/`, `.venv/`) |
| `base.lock` | Digest ảnh nền (index + amd64 + arm64) và ngày tra |
| `os-release` | Nội dung `/usr/lib/os-release` của AiOS |
| `systemd/` | `aiosd-core.service`, `aios-model-gateway.service`, `aios-killswitch.service` |
| `systemd-user/` | `aios-agent.slice` (slice theo người dùng) |
| `tmpfiles.d/` | `aios.conf` cho `/var/lib/aios/...`; `var-paths.conf` cho các thư mục `/var` do gói runtime tạo |

## Ghi chú

- **Nền ghim digest.** `Containerfile` dùng digest của index multi-arch; digest
  từng kiến trúc nằm trong `base.lock` (tra bằng
  `docker buildx imagetools inspect quay.io/fedora/fedora-bootc:44`).
- **Thương hiệu.** `fedora-release` và `fedora-logos` (nếu có) bị gỡ, thay bằng
  `generic-release` và `generic-logos`; `/usr/lib/os-release` được ghi đè bằng
  `image/os-release` (`ID=aios`, `ID_LIKE=fedora`, `VERSION_ID=0.0.1-m0`).
- **Tường lửa.** Zone mặc định của firewalld là `drop` (không mở dịch vụ nào),
  `sshd.service` bị mask.
- **Không có trong ảnh.** Mô hình, khóa ký, khóa API và file `.env` không được
  đưa vào ảnh; `policy/` và `plugins/` được chép có điều kiện nên ảnh vẫn build
  được khi hai thư mục đó chưa tồn tại.

## Nghiệm thu

```bash
docker build --platform linux/arm64 -f image/Containerfile -t aios:m0 .
docker run --rm aios:m0 bootc container lint
docker run --rm aios:m0 cat /usr/lib/os-release | grep -q '^ID=aios$'
docker run --rm aios:m0 rpm -q fedora-logos ; test $? -ne 0
docker run --rm aios:m0 systemctl is-enabled aiosd-core firewalld
```
