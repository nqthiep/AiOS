# S-16 — Giấy phép phụ thuộc (Rust, Python, RPM trong ảnh)

_Ngày rà soát: 2026-10-21 · Task: `s16` (thiết kế: [docs/m0/s16.md](../m0/s16.md)) · Nguồn quyết định: D-48 (AiOS theo Apache-2.0)_

> **Đây là bảng kê do công cụ sinh ra, không phải tư vấn pháp lý.** Mọi giấy phép trong
> file này lấy nguyên văn từ output của `cargo deny`, `cargo metadata`, `pip-licenses`
> và `rpm -qa`. Nhận định không lấy từ output công cụ được gắn \[Inference\].

## 1. Kết luận

| Nhóm | Số lượng | Kết quả | Kiểm tra tự động |
| --- | --- | --- | --- |
| Crate Rust (workspace, mọi target) | 337 crate trong đồ thị của `cargo deny` | Đạt; 2 ngoại lệ theo crate, có lý do trong `deny.toml` | `cargo deny check licenses` (chặn) |
| Gói Python (3 uv project, gồm nhóm `dev`) | 35–36 gói mỗi project | Đạt; 2 ngoại lệ theo gói (`cffi`, `typing_extensions`) | `make licenses` (chặn) |
| Gói RPM trong ảnh `aios:m0` | 711 gói | Chỉ ghi nhận, không chặn (thiết kế s16) | Không |

- Không có phụ thuộc Rust hay Python nào chỉ có giấy phép GPL, AGPL, SSPL, BUSL, BSL-only hay CDLA. Mọi crate có lựa chọn không nằm trong danh sách cho phép (LGPL, BSL-1.0, Unlicense, BSD-1-Clause, LLVM-exception) đều là biểu thức `OR` có ít nhất một lựa chọn được phép (§2.3).
- Ảnh có nhiều gói GPL/LGPL của Fedora (kernel, glibc, systemd, bash, coreutils, grub2…). \[Inference\] Các gói này là chương trình riêng, phân phối dạng nhị phân kèm nguồn ở kho Fedora, AiOS không liên kết tĩnh với chúng, nên không ràng buộc giấy phép Apache-2.0 của mã AiOS; khi phát hành ảnh vẫn phải giữ nghĩa vụ cung cấp mã nguồn tương ứng của các gói GPL.
- Ba thành phần ở [tech-stack.md §13](../tech-stack.md) (Piper `piper1-gpl`, VieNeu-TTS, xdg-desktop-portal-cosmic) **không có** trong ảnh M0 (`rpm -qa` không có tên `piper`, `cosmic`, `vieneu`). Rủi ro này chuyển sang lúc các thành phần đó vào ảnh (M1 trở đi).

## 2. Rust

### 2.1 Công cụ và cấu hình

| Mục | Giá trị |
| --- | --- |
| Công cụ | `cargo-deny 0.20.2` (`cargo install --locked cargo-deny`) |
| Cấu hình | [`deny.toml`](../../deny.toml): `[graph] all-features = true`, không lọc `targets` (kiểm cả crate chỉ-Linux như `landlock`, `seccompiler`) |
| Danh sách cho phép | `Apache-2.0`, `MIT`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Unicode-3.0`, `Zlib`, `MPL-2.0` (đúng thiết kế s16) |
| `cargo metadata` | 378 package trong `Cargo.lock`; `cargo deny list` liệt kê 337 crate trong đồ thị thực |

### 2.2 Số crate theo giấy phép (`cargo deny list -l license`)

Một crate có biểu thức nhiều giấy phép được đếm ở mọi giấy phép trong biểu thức, nên tổng lớn hơn 337.

| Giấy phép | Số crate | Ghi chú |
| --- | --- | --- |
| MIT | 290 | |
| Apache-2.0 | 236 | gồm 9 crate `aios-*`/`aiosd-core` của workspace |
| Unicode-3.0 | 20 | họ `icu_*`, `zerovec`, `unicode-ident`… |
| BSD-3-Clause | 6 | `curve25519-dalek`, `ed25519-dalek`, `encoding_rs`, `matchit`, `seccompiler`, `subtle` |
| ISC | 6 | `ring`, `rustls`, `rustls-webpki`, `hyper-rustls`, `rustls-native-certs`, `untrusted` |
| Unlicense | 5 | đều là `Unlicense OR MIT` |
| Apache-2.0 WITH LLVM-exception | 4 | `ar_archive_writer` (ngoại lệ), `wasi`, `wasip2`, `wit-bindgen` |
| Zlib | 2 | `foldhash`, `tinyvec` |
| LGPL-2.1-or-later | 2 | `r-efi` 5.3.0 và 6.0.0, đều là biểu thức `OR` |
| BSD-2-Clause | 1 | `zerocopy` |
| BSD-1-Clause | 1 | `fiat-crypto` |
| BSL-1.0 | 1 | `ryu` |
| MIT-0 | 1 | `borrow-or-share` (ngoại lệ) |
| MPL-2.0 | 0 | được phép nhưng chưa crate nào dùng (`unused-allowed-license = "allow"`) |

### 2.3 Crate cần chú ý

Biểu thức lấy từ trường `license` của `cargo metadata`.

| Crate | Giấy phép | Kéo vào bởi | Xử lý |
| --- | --- | --- | --- |
| `ar_archive_writer` 0.5.3 | `Apache-2.0 WITH LLVM-exception` | (build) `psm` → `stacker` → `cedar-policy-core` → `aios-policy` | Ngoại lệ theo crate trong `deny.toml`. \[Inference\] LLVM-exception chỉ thêm quyền cho Apache-2.0, và crate chỉ dùng lúc build |
| `borrow-or-share` 0.2.4 | `MIT-0` | `fluent-uri` → `referencing` → `jsonschema` → `aios-registry`, `aios-broker` | Ngoại lệ theo crate. \[Inference\] MIT-0 là MIT bỏ yêu cầu giữ thông báo bản quyền, nên không hẹp hơn MIT |
| `r-efi` 5.3.0, 6.0.0 | `MIT OR Apache-2.0 OR LGPL-2.1-or-later` | `getrandom` → `rand`, `uuid` → `aios-broker` | Không cần ngoại lệ: chọn MIT hoặc Apache-2.0 |
| `ryu` 1.0.23 | `Apache-2.0 OR BSL-1.0` | `serde_urlencoded`, `serde_yaml_ng` | Chọn Apache-2.0 |
| `fiat-crypto` 0.2.9 | `MIT OR Apache-2.0 OR BSD-1-Clause` | `curve25519-dalek` | Chọn MIT hoặc Apache-2.0 |
| `wasi`, `wasip2`, `wit-bindgen` | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | `getrandom` (chỉ target wasm) | Chọn Apache-2.0 hoặc MIT |
| `aho-corasick`, `memchr`, `walkdir`, `same-file`, `winapi-util` | `Unlicense OR MIT` | `regex`, `regex-automata`, `walkdir`… | Chọn MIT |
| `ring` 0.17.14 | `Apache-2.0 AND ISC` | `rustls` | Cả hai đều được phép; không cần ngoại lệ |
| `encoding_rs` 0.8.42 | `(Apache-2.0 OR MIT) AND BSD-3-Clause` | `reqwest` | Đều được phép |
| `unicode-ident` 1.0.27 | `(MIT OR Apache-2.0) AND Unicode-3.0` | `proc-macro2` | Đều được phép |

Không có crate nào dùng giấy phép OpenSSL, GPL, AGPL, SSPL, BUSL hay CDLA.

## 3. Python

### 3.1 Công cụ và phạm vi

| Mục | Giá trị |
| --- | --- |
| Công cụ | `pip-licenses 5.5.5` qua `uvx`, `--from=mixed` (ưu tiên `License-Expression`, rồi classifier), `--with-system`, `--python <project>/.venv/bin/python` (không lẫn gói của chính `pip-licenses`) |
| uv | `uv 0.12.17` |
| Project | Mọi `pyproject.toml` do git theo dõi: `python/aiosd-cognition`, `plugins/tools/sys`, `plugins/tools/fs`. Repo hiện không có `pyproject.toml` nào trong `testdata/` hay `dev/vm/hindsight` |
| Môi trường | `uv sync --frozen` (theo `uv.lock`, gồm nhóm `dev`: `pytest`, `ruff`) |
| Danh sách cho phép | SPDX id của `deny.toml`, chuỗi classifier `MIT License`, và hai biểu thức `Apache-2.0 OR BSD-2-Clause`, `Apache-2.0 OR BSD-3-Clause` (mỗi lựa chọn đều được phép). `pip-licenses --allow-only` so khớp nguyên chuỗi, không hiểu biểu thức SPDX, nên phải liệt kê từng biểu thức |

### 3.2 Gói theo giấy phép (hợp của 3 project)

Ba project có cùng tập gói, chỉ khác gói của chính project và `PyYAML` (chỉ `aiosd-cognition`).

| Giấy phép (nguyên văn từ `pip-licenses`) | Gói |
| --- | --- |
| `Apache-2.0` | `aiosd-cognition`, `aios-sys`, `aios-fs`, `opentelemetry-api`, `python-multipart` |
| `Apache-2.0 OR BSD-3-Clause` | `cryptography` |
| `Apache-2.0 OR BSD-2-Clause` | `packaging` |
| `MIT` | `PyJWT`, `annotated-types`, `anyio`, `attrs`, `iniconfig`, `jsonschema`, `jsonschema-specifications`, `pydantic`, `pydantic_core`, `pytest`, `referencing`, `rpds-py`, `ruff`, `truststore`, `typing-inspection` |
| `MIT License` | `PyYAML`, `h11`, `mcp`, `mcp-types`, `pluggy` |
| `BSD-3-Clause` | `click`, `httpcore2`, `httpx2`, `idna`, `pycparser`, `sse-starlette`, `starlette`, `uvicorn` |
| `BSD-2-Clause` | `Pygments` |
| `MIT-0` | `cffi` — **ngoại lệ** |
| `PSF-2.0` | `typing_extensions` — **ngoại lệ** |

### 3.3 Ngoại lệ

| Gói | Giấy phép | Lý do |
| --- | --- | --- |
| `cffi` 2.1.1 | `MIT-0` | Cùng lý do với `borrow-or-share` ở §2.3. Kéo vào bởi `cryptography` → `pyjwt` → `mcp[crypto]` |
| `typing_extensions` 4.16.0 | `PSF-2.0` | \[Inference\] Giấy phép dễ dãi của Python Software Foundation, không copyleft; tương thích Apache-2.0. PSF-2.0 không có trong danh sách của docs/m0/README.md §5 nên ghi là ngoại lệ theo gói, không thêm vào danh sách chung |

`make licenses` bỏ qua hai gói này bằng `--ignore-packages`, nên nếu bản mới của chúng đổi giấy phép thì kiểm tra không thấy. \[Inference\] Rủi ro thấp; cần xem lại khi nâng phiên bản trong `uv.lock`.

### 3.4 Gói Python trong ảnh

Ảnh cài wheel `aiosd_cognition` bằng `uv pip install --system --no-deps` (image/Containerfile). Trong `aios:m0`: `import mcp` báo `ModuleNotFoundError`, `python3-pyyaml-6.0.3` có từ RPM (giấy phép `MIT`). Nghĩa là các gói ở §3.2 hiện chưa vào ảnh; khi được thêm (RPM hoặc wheel), giấy phép đã rà ở trên vẫn áp dụng. Thiếu `mcp` trong ảnh nằm ngoài phạm vi s16; ghi lại để task `image` xem.

## 4. Gói RPM trong ảnh `aios:m0`

### 4.1 Cách lấy

```bash
docker run --rm aios:m0 rpm -qa --qf '%{NAME} %{LICENSE}\n' | sort
```

Ảnh: `sha256:52dcff19b3c2…`, tạo lúc 2026-10-20T23:58:32Z, arm64, `os-release` AiOS 0.0.1 (m0), `kernel-core-7.2.11`. Không có gói RPM nào tên `aios*`: binary `aiosd-core`, `aios-model-gateway`, `aios-sandbox-exec` và wheel `aiosd_cognition` được chép vào ảnh, không qua RPM, nên giấy phép của chúng là phần Rust/Python ở §2, §3.

### 4.2 Tóm tắt (đếm bằng `grep` trên trường `LICENSE`)

| Nhóm | Số gói | Cách đếm |
| --- | --- | --- |
| Tổng | 711 | |
| Có nhắc GPL hoặc LGPL ở bất kỳ vị trí nào | 455 | `grep -c GPL` |
| Có GPL (không tính LGPL) | 296 | `grep -cE '(^\|[ (])GPL-'` |
| Chỉ có LGPL, không có GPL | 159 | |
| Có GPL-3 (không tính LGPL-3) | 101 | `grep -cE '(^\|[ (])GPL-3'` |
| Có AGPL | 0 | |
| Không nhắc GPL/LGPL | 256 | |
| Có MPL | 27 | |
| Firmware `LicenseRef-Fedora-Firmware` | 16 | |
| Giấy phép kiểu cũ `LicenseRef-Callaway-*` | 6 | |

Một biểu thức như `(FTL OR GPL-2.0-or-later)` cũng được đếm vào nhóm GPL, nên các số GPL là cận trên.

### 4.3 Gói cần chú ý

| Nhóm | Gói (giấy phép nguyên văn hoặc rút gọn) |
| --- | --- |
| GPL, lõi hệ thống | `kernel`/`kernel-core`/`kernel-modules*` (`GPL-2.0-only WITH Linux-syscall-note` và nhiều thành phần khác), `bash` (`GPL-3.0-or-later`), `coreutils` (`GPL-3.0-or-later AND GFDL-1.3-no-invariants-or-later AND LGPL-…`), `grub2-*` (`GPL-3.0-or-later`), `selinux-policy*` (`GPL-2.0-or-later`), `crun` (`GPL-2.0-only`), `firewalld` (`GPL-2.0-or-later`), `util-linux`, `tar`, `sed`, `findutils`, `nano`, `sssd-*` |
| LGPL, thư viện | `glibc*` (`LGPL-2.1-or-later AND …`), `systemd*` (`LGPL-2.1-or-later AND MIT AND GPL-2.0-or-later`), `ostree` (`LGPL-2.0-or-later`), `bubblewrap`, `flatpak`, `NetworkManager-libnm` (`LGPL-2.1-or-later`), `libgcc`/`libstdc++` (`GPL-3.0-or-later WITH GCC-exception-3.1` và nhiều thành phần khác) |
| MPL | `nss*`, `nspr`, `cairo`, `librsvg2`, `podman`, `skopeo`, `uv`, `python-pip-wheel`, `bind-libs` … (27 gói) |
| Firmware không phải mã nguồn mở | 16 gói `LicenseRef-Fedora-Firmware`: `linux-firmware`, `amd-gpu-firmware`, `amd-ucode-firmware`, `intel-gpu-firmware`, `intel-npu-firmware`, `nvidia-gpu-firmware`, `realtek-firmware`, `brcmfmac-firmware`… \[Inference\] Firmware phân phối lại được theo điều khoản của Fedora nhưng không có mã nguồn; liên quan D-48 ("thành phần có giấy phép không phải mã nguồn mở không được đóng gói") — cần chủ dự án quyết định có giữ firmware trong ảnh phát hành hay không |
| Giấy phép đáng xem thêm | `clevis-pin-tpm2` (có `EUPL-1.2` trong biểu thức `AND`), `uv` (có `CDLA-Permissive-2.0`, `(LGPL-3.0-or-later OR MIT)`), `podman-sequoia`/`rpm-sequoia` (có `LGPL-2.0-or-later`, `BSL-1.0`), `rav1e-libs`/`svt-av1-libs` (có `AOMPL-1.0` dạng `LicenseRef`), `libbsd`/`libmd` (`Beerware`) |
| Gói lạ trong ảnh | `qemu-user-static` và 17 gói `qemu-user-static-*` (18 gói) (GPL/LGPL). \[Inference\] Có thể đến từ ảnh nền hoặc phụ thuộc kéo theo; không cần cho runtime AiOS, nên xem lại ở task `image` |
| Gói thành phần AiOS dùng | `llama-cpp` (`MIT AND Apache-2.0 AND LicenseRef-Fedora-Public-Domain`), `postgresql`/`pgvector` (`PostgreSQL`), `python3` (`Python-2.0.1`), `bootc` (`Apache-2.0 AND …`), `uv` (xem trên) |

Không gói nào có trường `LICENSE` rỗng.

## 5. Kiểm tra tự động

| Lệnh | Làm gì | Khi vi phạm |
| --- | --- | --- |
| `cargo deny check licenses` | Đối chiếu mọi crate với `deny.toml` | `error[rejected]: failed to satisfy license requirements`, thoát khác 0 |
| `make licenses` | Chạy lệnh trên, rồi với mỗi uv project: `uv sync --frozen`, `pip-licenses --allow-only=…` | `license … not in allow-only licenses was found for package …`, thoát khác 0 |

Đã thử nhánh lỗi:

- `deny.toml` chỉ có danh sách cho phép, chưa có `exceptions`: `cargo deny check licenses` báo `rejected` cho `ar_archive_writer` và `borrow-or-share`, kết thúc `licenses FAILED`.
- `make licenses PY_LICENSE_EXCEPT=cffi` (bỏ ngoại lệ `typing_extensions`): `license PSF-2.0 not in allow-only licenses was found for package typing_extensions:4.16.0`, `make: *** [licenses] Error 1`.

Thêm crate hay gói mới có giấy phép ngoài danh sách sẽ làm hai lệnh trên thất bại; muốn cho qua phải thêm ngoại lệ theo crate/gói kèm lý do, không mở rộng danh sách chung.

## 6. Việc còn lại

| Việc | Ai | Ghi chú |
| --- | --- | --- |
| Quyết định giữ hay bỏ 16 gói firmware `LicenseRef-Fedora-Firmware` trong ảnh phát hành | Chủ dự án | Liên quan D-48 |
| Xem lại `qemu-user-static*` và thiếu `mcp` trong ảnh | Task `image` | Ngoài phạm vi s16 |
| Rà Piper, VieNeu-TTS, xdg-desktop-portal-cosmic khi chúng vào ảnh | Task thêm các thành phần đó | tech-stack.md §13 |
| Chạy `rpm -qa` lại và so với bảng này mỗi lần đổi ảnh nền | Task `image` | Chưa tự động |
