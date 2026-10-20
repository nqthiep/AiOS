# gateway — aios-model-gateway

| Thuộc tính | Giá trị |
| --- | --- |
| Người làm | worker |
| Phụ thuộc | ld-setup |
| Chặn | hindsight, cognition |
| Nguồn thiết kế | design.md §5 (đặc biệt §5.2 nhãn theo session, §5.4), §18.2 (ngân sách riêng từng người dùng); integration.md §3.1; REQ-D-5.2-01..05, REQ-I-3.1-01 |

## Mục tiêu

Binary Rust `aios-model-gateway`: điểm duy nhất gọi mô hình. API OpenAI-compatible cho client; chọn route local hoặc cloud theo nhãn của session và trần của khóa ảo; không bao giờ gửi `secret` lên cloud; giữ ngân sách riêng của từng người dùng.

## Phạm vi

Được sửa: `crates/aios-model-gateway/**`.

## Thiết kế

### API cho client (`127.0.0.1:8090`)

| Endpoint | Ghi chú |
| --- | --- |
| `POST /v1/chat/completions` | Body OpenAI; trường `model` là tên logic: `s1`, `s2`, `coder`. M0 chỉ hỗ trợ `stream: false`; `stream: true` → 400 `stream_not_supported` |
| `POST /v1/embeddings` | Luôn local (REQ-D-5.2-04) |
| `GET /v1/models` | Danh sách tên logic |

- Xác thực: `Authorization: Bearer <khóa ảo>`. Khóa không biết → 401.
- Header tùy chọn `X-AIOS-Privacy: public|personal|secret`: client chỉ được **nâng** nhãn (REQ-D-5.2-03).
- Lỗi khi không có route: HTTP 409, `{"error":{"code":"no_route_privacy","message":...}}`.

### Chọn route (design §5.4)

```text
privacy = max(nhãn session (hoặc nhãn khóa dịch vụ), nhãn client khai nếu có)
nếu khóa không biết session → privacy = secret (REQ-D-5.2-02)
ứng viên = routes khớp model logic
loại cloud nếu: privacy == secret (kiểm lần 2 trong code, độc lập cấu hình)
              hoặc privacy > min(max_cloud_privacy của client, max_cloud_privacy của người dùng)
              hoặc mất mạng, hoặc ngân sách người dùng hết, hoặc người dùng chưa cấu hình cloud
thử lần lượt; lỗi thì sang ứng viên kế; hết ứng viên → 409 no_route_privacy hoặc 503 upstream_unavailable
```

### Cấu hình

- `/etc/aios/gateway.yaml` (design §5.3): `listen`, `models` (local: `endpoint` llama-server; cloud: `kind: openai_compatible`, `base_url`), `routes`, `clients` (integration §3.1: `max_cloud_privacy` hoặc `cloud: false`, `key_scope`).
- `/var/lib/aios/users/<uid>/gateway.yaml` (design §18.2): provider, `api_key_ref`, `budget.usd_per_day`, `usd_per_month`, `warn_at`, `max_cloud_privacy`. Không có file → cloud tắt cho người dùng đó. M0: `api_key_ref` dạng `file:/var/lib/aios/users/<uid>/cloud-key` (quyền `0600`, chủ là user dịch vụ của Gateway); `systemd-creds` để M1.
- Bảng giá token theo model cloud trong `gateway.yaml` (`price_per_mtok_in`, `price_per_mtok_out`); chi phí = token × giá, cộng vào `/var/lib/aios/users/<uid>/usage.json` theo ngày (UTC).

### Quản trị (`/run/aios/gateway-admin.sock`)

Chỉ UID cấu hình là `aiosd-core` gọi được (`SO_PEERCRED`); UID khác → 403.

| Endpoint | Hành vi |
| --- | --- |
| `POST /v1/sessions` `{"uid","session_id","client"}` | Cấp khóa ảo `aios-sk-<32 byte base64url>` gắn uid, session, client; nhãn ban đầu `public`, taint `clean` |
| `POST /v1/sessions/{id}/raise` `{"label","taint"}` | Nâng nhãn và taint; yêu cầu hạ → bỏ qua phần hạ, trả 200 kèm nhãn hiện tại |
| `DELETE /v1/sessions/{id}` | Thu hồi khóa |
| `POST /v1/revoke-all` | Thu hồi mọi khóa session (kill switch, design §11.3) |

Trạng thái session giữ trong bộ nhớ (M0); Gateway khởi động lại thì mọi khóa session mất hiệu lực (fail closed).

### Khác

- Không log nội dung prompt; log metric: model, route, token, độ trễ, uid.
- Hàng đợi local theo uid, mỗi uid tối đa 1 request đồng thời tới mỗi model local (design §18.4); không chia sẻ prompt cache giữa người dùng: gửi `cache_prompt: false` tới llama-server khi request kế tiếp khác uid với request trước trên cùng slot \[Unverified\] tên trường theo bản llama.cpp trong F44.

## Kiểm thử và nghiệm thu

Test với upstream giả (một local, một "cloud") trong test:

| Test | Mong đợi |
| --- | --- |
| Session nhãn `public`, người dùng có cloud | Đi cloud |
| Sau `raise` lên `secret` | Đi local; cloud giả không nhận request nào |
| Cấu hình route sai cho phép `secret` lên cloud | Code vẫn chặn (kiểm lần 2) |
| Client khai `X-AIOS-Privacy: public` khi session là `personal` | Vẫn coi là `personal` |
| Khóa không gắn session (khóa dịch vụ `cloud:false`) | Chỉ local |
| Ngân sách đã dùng hết | Local; nếu không có local cho model đó → 409 |
| Người dùng A và B cùng gọi | Chi phí ghi đúng file của từng người |
| `revoke-all` | Mọi khóa session → 401 |
| `/v1/embeddings` với session `public` | Vẫn local |

```bash
cargo test -p aios-model-gateway
cargo clippy -p aios-model-gateway -- -D warnings
```

## Ngoài phạm vi

Streaming (M1), redaction định danh trước khi gửi cloud (M1), `systemd-creds` (M1), Lemonade/NPU.
