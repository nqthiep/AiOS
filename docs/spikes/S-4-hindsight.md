# S-4 — Hindsight không torch, gọi mô hình qua Model Gateway

_Ngày đo: 2026-10-21 · Task: `hindsight` (thiết kế: [docs/m0/hindsight.md](../m0/hindsight.md))_

> **Kết quả ngắn.** Hindsight 1.0.0 (`hindsight-api-slim`) chạy được trong VM `aios-dev`:
> không có torch, LLM và embedding đi qua Model Gateway (route local), reranker tắt bằng
> provider `rrf`, Postgres 18 + pgvector của Fedora. Probe đạt cả 4 bước. RAM của
> Hindsight khoảng **400–426 MiB**. Điểm yếu: `retain` chậm (**26–53 s/câu** khi prompt
> cache còn, **7,6 phút** khi cache nguội) vì Qwen3-4B chạy CPU; venv nặng **1,2 GB**.

## 1. Môi trường

| Mục | Giá trị |
| --- | --- |
| VM | Lima `aios-dev`, Fedora 44 aarch64, 4 vCPU, 8 GB RAM, swap 7,7 GB |
| Mô hình | Theo [S-13-m0](S-13-m0.md): chat `Qwen3-4B-Instruct-2507` Q4_K_M trên `127.0.0.1:8081`, embedding `multilingual-e5-large-instruct` Q4_K_M (1024 chiều) trên `127.0.0.1:8082`; `llama-server` chỉ backend CPU |
| Máy chủ | Mac 10 lõi, chạy chung với các worker khác; load average 9–14 trong lúc đo (nhiều `rustc`) |
| Thư mục chạy | `~/build/wt-hindsight` (bản chép của worktree qua `dev/vm-run.sh`), **không phải** `~/build/AiOS` như lệnh nghiệm thu trong file task |

## 2. Thành phần đã cài (`dev/vm/hindsight/install.sh`)

| Thành phần | Cách chạy | Ghi chú |
| --- | --- | --- |
| Postgres 18.6 + pgvector 0.8.0 | `postgresql.service` (system) | `install.sh` cài thêm `postgresql-contrib` (cần `pg_trgm`, `btree_gin`), `postgresql-setup --initdb` nếu chưa có |
| Database + role | `aios_u501` / `aios_u501` | Peer auth qua socket `/run/postgresql`, `pg_ident` map `quangthiep → aios_u501`, chỉ cho database `aios_u501`. Không có mật khẩu. `vector`, `pg_trgm`, `btree_gin` tạo sẵn bằng `postgres` (pgvector không phải extension "trusted") |
| Model Gateway | unit **user** `aios-model-gateway.service` | Binary build từ `crates/aios-model-gateway` (release) → `/opt/aios/bin/`; config `dev/vm/hindsight/gateway.yaml` → `/etc/aios/gateway.yaml`; admin socket `/run/aios/gateway-admin.sock`, `--admin-uid 501` |
| Khóa Gateway | client `hindsight`, `cloud: false`, `key_scope: service` | Cấp qua `POST /v1/sessions` trên admin socket mỗi lần chạy `install.sh`; khóa dịch vụ luôn mang nhãn `secret` |
| venv | `/opt/aios/hindsight`, chủ `root` | `uv sync --frozen --no-cache --compile-bytecode` theo `dev/vm/hindsight/uv.lock`; không extra `local-ml`/`local-onnx`; `install.sh` thoát lỗi nếu thấy `torch`, `sentence-transformers`, `transformers`, `flashrank` |
| Env | `/etc/aios/hindsight/501.env`, chủ `501`, `0600` | Sinh từ `hindsight.env.example`; chứa khóa Gateway và khóa API Hindsight (sinh ngẫu nhiên mỗi lần cài), không nằm trong repo |
| Hindsight | unit **system** `hindsight-api@501.service`, `User=%i`, `Slice=aios-svc.slice`, nghe `127.0.0.1:8888` | System unit để `IPAddressDeny=` có hiệu lực (unit user bỏ qua, xem S-13-m0 §5.1) |

## 3. Phiên bản

| Gói | Phiên bản | Giấy phép |
| --- | --- | --- |
| `hindsight-api-slim` | 1.0.0 | MIT |
| `hindsight-client` | 1.0.0 | MIT |
| Python | 3.14.3 (`/usr/bin/python3.14`) | |
| uv | 0.12.24 | |
| `litellm` / `openai` / `asyncpg` / `fastapi` / `uvicorn` | 1.106.1 / 2.54.0 / 0.32.0 / 0.156.4 / 0.54.0 | MIT / Apache-2.0 / Apache-2.0 / MIT / BSD-3-Clause |
| `onnxruntime` (kéo theo `markitdown` → `magika`) | 1.31.0 | MIT |
| `psycopg2-binary` | 2.9.13 | **LGPL** (xem §7.6) |

`uv lock` giải ra 201 gói; venv có 193 gói (kể cả `pip`, thêm vào để lệnh nghiệm thu
`bin/pip list` chạy thật, không qua một cách vô nghĩa vì thiếu `pip`).

## 4. Biến môi trường đã dùng

Tên biến đối chiếu với `hindsight_api/config.py` của bản 1.0.0 đã cài.

```ini
HINDSIGHT_API_HOST=127.0.0.1
HINDSIGHT_API_PORT=8888
HINDSIGHT_API_DATABASE_URL=postgresql://aios_u501@/aios_u501?host=/run/postgresql
HINDSIGHT_API_LLM_PROVIDER=openai
HINDSIGHT_API_LLM_BASE_URL=http://127.0.0.1:8090/v1
HINDSIGHT_API_LLM_API_KEY=<khóa dịch vụ Gateway>
HINDSIGHT_API_LLM_MODEL=s1
HINDSIGHT_API_LLM_MAX_CONCURRENT=1
HINDSIGHT_API_LLM_TIMEOUT=300
HINDSIGHT_API_EMBEDDINGS_PROVIDER=openai
HINDSIGHT_API_EMBEDDINGS_OPENAI_BASE_URL=http://127.0.0.1:8090/v1
HINDSIGHT_API_EMBEDDINGS_OPENAI_API_KEY=<khóa dịch vụ Gateway>
HINDSIGHT_API_EMBEDDINGS_OPENAI_MODEL=embed-local
HINDSIGHT_API_RERANKER_PROVIDER=rrf
HINDSIGHT_API_TEXT_SEARCH_EXTENSION_NATIVE_LANGUAGE=simple
HINDSIGHT_API_LLM_OUTPUT_LANGUAGE=Vietnamese
HINDSIGHT_API_TENANT_EXTENSION=hindsight_api.extensions.builtin.tenant:ApiKeyTenantExtension
HINDSIGHT_API_TENANT_API_KEY=<khóa API ngẫu nhiên>
HINDSIGHT_API_OTEL_TRACES_ENABLED=false
LITELLM_LOCAL_MODEL_COST_MAP=True
HF_HUB_OFFLINE=1
DO_NOT_TRACK=1
```

Khác biệt so với [integration.md §6.2](../integration.md) (đề nghị leader sửa tài liệu):

| integration.md §6.2 | Thực tế 1.0.0 |
| --- | --- |
| `HINDSIGHT_API_LLM_BASE_URL` gắn \[Unverified\] | Đúng tên (`ENV_LLM_BASE_URL`) |
| Không ghi khóa và tên model | Cần thêm `HINDSIGHT_API_LLM_API_KEY`, `HINDSIGHT_API_LLM_MODEL`, `HINDSIGHT_API_EMBEDDINGS_OPENAI_API_KEY`, `HINDSIGHT_API_EMBEDDINGS_OPENAI_MODEL` (mặc định `text-embedding-3-small`) |
| `HINDSIGHT_API_RERANKER_PROVIDER=<provider không cần torch>` | Giá trị `rrf`: "RRF passthrough, neural reranking disabled". Mặc định là `local` (cần torch). `flashrank` cần onnx từ extra `local-ml`, không có trong bản slim |
| Không ghi database | `HINDSIGHT_API_DATABASE_URL` mặc định là `pg0` (Postgres nhúng, extra `embedded-db`); phải đặt URL Postgres hệ thống |
| Không ghi xác thực API | Mặc định API `:8888` **không có xác thực**; bật bằng `ApiKeyTenantExtension` (xem §7.5) |
| Một bộ biến LLM | Có thêm bộ riêng theo thao tác: `HINDSIGHT_API_{RETAIN,REFLECT,CONSOLIDATION,MENTAL_MODEL_REFRESH}_LLM_*`; không đặt thì dùng `HINDSIGHT_API_LLM_*` |

Hai biến còn lại là lựa chọn của spike: `LLM_MAX_CONCURRENT=1` (mặc định 32; Gateway chỉ
cho một request đồng thời mỗi uid mỗi mô hình local) và `TEXT_SEARCH_..._LANGUAGE=simple`
(mặc định `english`, stemming tiếng Anh không hợp tiếng Việt).

## 5. Nghiệm thu

Thư mục chạy là `~/build/wt-hindsight` (bản chép worktree), thay cho `~/build/AiOS` trong file task.

### 5.1 `install.sh` + `probe.py`

```bash
dev/vm-run.sh 'bash dev/vm/hindsight/install.sh && sleep 30 && /opt/aios/hindsight/bin/python dev/vm/hindsight/probe.py'
```

Lần chạy cuối (lần cài thứ ba; dòng `retain up-01` … `up-19` lược bớt):

```text
[hindsight] postgres: database aios_u501, role aios_u501 (peer, map aios_u501)
[hindsight] build aios-model-gateway (release)
[hindsight] gateway: 127.0.0.1:8090, admin /run/aios/gateway-admin.sock
[hindsight] uv sync -> /opt/aios/hindsight (uv.lock, không extra local-ml)
[hindsight] venv: 1.2G, hindsight-api-slim 1.0.0
[hindsight] env: /etc/aios/hindsight/501.env (0600)
[hindsight] hindsight-api@501.service: sẵn sàng tại http://127.0.0.1:8888
[probe] hindsight-api@501.service pid=65715 idle: VmRSS=409764 kB VmSwap=0 kB MemoryCurrent=420 MiB
[probe] banks: ['ext-personal', 'ext-public', 'ext-secret', 'system', 'user-personal', 'user-public', 'user-secret']
[probe] retain up-00 ok=True 456.0s
[probe] retain 20 sentences: 1180.2s total, 59.0s/sentence; after retain: VmRSS=416076 kB VmSwap=0 kB MemoryCurrent=423 MiB
[probe] recall 'tôi làm việc lúc mấy giờ' top3:
  1. [up-00] User works from 8:00 AM to 5:00 PM, Monday to Friday | Involving: user
  2. [up-02] User lives in Cầu Giấy district, Hanoi | Involving: user
  3. [up-19] User dislikes weekend afternoon meetings | Involving: user
[probe] recall latency: avg=0.10s min=0.10s max=0.13s first=0.13s; VmRSS peak during recall=418504 kB; after recall: VmRSS=418516 kB VmSwap=0 kB MemoryCurrent=425 MiB
[probe] ext-personal recall: 2 results, documents=['ep-00', 'ep-01']
[probe] gateway: 23 chat (routes ['s1-local'], 79460 in / 2491 out tokens), 34 embedding (routes ['embed-local']), 0 upstream errors, 0 non-local
[probe] end: VmRSS=418520 kB VmSwap=0 kB MemoryCurrent=426 MiB
[probe] PASS
rc=0
```

| Bước (docs/m0/hindsight.md) | Kết quả |
| --- | --- |
| `retain` 20 câu tiếng Việt vào `user-personal` | Đạt: 20/20 `success=True` |
| `recall` "tôi làm việc lúc mấy giờ" | Đạt: câu giờ làm việc (`up-00`) đứng **hạng 1**. Probe kiểm theo `document_id`, không theo chữ, vì Hindsight lưu fact do LLM viết lại (§7.3) |
| `ext-personal` tách khỏi `user-personal` | Đạt: recall trên `ext-personal` chỉ trả tài liệu `ep-*`; recall ngược trên `user-personal` không trả tài liệu `ep-*` |
| Gateway log | Đạt: 23 lệnh chat đều route `s1-local` (`kind=local`), 34 lệnh embedding đều `embed-local`; 0 lệnh không local |
| Đủ 7 bank | Đạt |

Hai lần chạy trước không đạt, ghi lại vì có giá trị đo:

- Lần 1: client hết thời gian chờ (900 s) ở câu thứ ba. Nguyên nhân: tác vụ hợp nhất nền
  (observation consolidation) chiếm mô hình và xóa prompt cache của `retain` (§7.2).
  Từ lần 2, probe tắt `enable_observations` trên hai bank thử.
- Lần 2: mọi bước đạt trừ kiểm tra top 3, vì probe so khớp chữ tiếng Việt trong khi fact
  được viết bằng tiếng Anh. Top 1 lúc đó cũng là câu giờ làm việc. Đã sửa probe sang so `document_id`.

### 5.2 Không có torch

```bash
limactl shell aios-dev -- bash -lc '/opt/aios/hindsight/bin/pip list 2>/dev/null | grep -iE "^(torch|sentence-transformers) " ; test $? -eq 1'; echo "rc=$?"
```

```text
rc=0
```

`bin/pip list` liệt kê 193 gói (`pip 26.2.2` có trong venv), nên kết quả này là kiểm thật,
không phải `grep` trên đầu vào rỗng.

### 5.3 RAM của unit

```bash
limactl shell aios-dev -- bash -lc 'systemctl show hindsight-api@$(id -u) -p MemoryCurrent'
```

```text
MemoryCurrent=445779968
```

## 6. Số đo

### 6.1 RAM

| Lúc đo | `VmRSS` tiến trình | `MemoryCurrent` unit |
| --- | --- | --- |
| Rảnh, sau khởi động (lần 3) | 409 764 kB (400 MiB) | 420 MiB |
| Rảnh, sau khởi động (lần 1, lần 2) | 411 464 kB / 404 128 kB | 541 MiB / 425 MiB |
| Sau 20 lần `retain` | 416 076 kB | 423 MiB |
| Trong 10 lần `recall` (đỉnh, lấy mẫu 0,1 s) | 418 504 kB (409 MiB) | 425 MiB sau đó |
| Cuối probe / lệnh nghiệm thu 5.3 | 418 520 kB | 426 MiB / 445 779 968 B (425 MiB) |

Thêm cho bức tranh RAM chung: Postgres `MemoryCurrent` 91 463 680 B (87 MiB), Gateway
19 959 808 B (19 MiB), database `aios_u501` 12 MB sau probe.

Ở lần 2, RSS của Hindsight tụt từ 404 MB xuống 160 MB sau `retain` vì VM đẩy trang ra swap
(VM dùng 1,1 GB swap; `VmSwap` của Hindsight 228 532 kB). Probe lần 3 vì vậy in thêm `VmSwap`;
lần 3 `VmSwap=0` suốt quá trình. \[Inference\] Áp lực bộ nhớ đến từ `llama-server` chat
(RSS ~3,6 GiB, S-13-m0 §4) trong VM 8 GB.

### 6.2 Dung lượng

| Mục | Kích thước |
| --- | --- |
| venv `/opt/aios/hindsight` (có bytecode) | 1,2 GB |
| — `claude_agent_sdk` (trong đó `_bundled/claude` 277 MB) | 279 MB |
| — `litellm` | 161 MB |
| — `pandas` | 76 MB |
| — `onnxruntime` | 59 MB |
| — `numpy` + `numpy.libs` | 68 MB |
| Cache uv sau cài | 0 (`--no-cache`) |

### 6.3 Độ trễ

| Thao tác | Số đo (lần 3) |
| --- | --- |
| Khởi động unit đến `Application startup complete` (migration đã chạy) | 4,8 s |
| `retain` câu đầu (prompt cache nguội) | 456,0 s |
| `retain` 19 câu sau | 26,6–52,5 s, trung bình 38,1 s |
| `retain` 20 câu | 1 180,2 s |
| `recall` (10 lần, budget `mid`) | trung bình 0,10 s, 0,10–0,13 s |
| Token LLM cho 23 `retain` (20 + 3 câu `ext-personal`) | 79 460 vào / 2 491 ra (~3 450 token prompt mỗi lần) |

Số lệnh chat trong log Gateway (23) bằng đúng số lần `retain`, nên 12 lần `recall` của probe
không gọi LLM, chỉ gọi embedding.

Đo trực tiếp `llama-server` chat với prompt 2 413 token, `cache_prompt: false`:
prompt 347,7 s (**6,94 token/s**), sinh 4,9 token/s. Prompt hệ thống của bước trích fact
dài ~3 450 token, nên mỗi lần prompt cache bị xóa thì một `retain` mất 5–10 phút.

## 7. Vấn đề gặp và rủi ro

1. **Mô hình local quá chậm cho `retain` trên VM này.** Lần chạy 2, request đầu vượt
   `request_timeout_s: 300` của Gateway (log: `upstream call failed`, `latency_ms: 300014`),
   Hindsight thử lại và thành công. Log `llama-server` lần 1 cho thấy cùng hiện tượng: task
   bị hủy ở `n_past = 2049`, task thử lại chỉ còn xử lý 1 409/3 458 token prompt. `gateway.yaml`
   của spike đặt `request_timeout_s: 300` (mặc định 120). \[Inference\] Trên phần cứng
   mục tiêu (x86_64 + iGPU Vulkan, S-13) prompt sẽ nhanh hơn nhiều; con số ở đây chỉ là
   cận dưới cho VM CPU dùng chung.
2. **Hợp nhất nền tranh mô hình với `retain`.** Lần 1, một lệnh LLM của consolidation mất
   1 949 s (gồm thời gian chờ slot ở Gateway), lệnh sau 210 s cho 4 286 token vào. Prompt
   khác với prompt `retain`, nên `llama-server` (1 slot) xóa cache và `retain` kế tiếp lại
   nguội. Probe tắt observations trên `user-personal`, `ext-personal`; 5 bank còn lại để
   mặc định (bật). M1 cần: chạy consolidation khi máy rảnh, hoặc slot/mô hình riêng.
3. **Fact bị viết bằng tiếng Anh.** Với Qwen3-4B, fact trích từ câu tiếng Việt ra tiếng Anh
   ở cả hai cấu hình: không đặt `HINDSIGHT_API_LLM_OUTPUT_LANGUAGE` (prompt mặc định yêu cầu
   giữ ngôn ngữ nguồn, lần 2) và đặt `=Vietnamese` (lần 3). Có một lỗi dịch nghĩa:
   "Tôi không thích họp vào buổi chiều thứ Sáu" thành "dislikes weekend afternoon meetings".
   Recall vẫn đúng vì embedding e5 đa ngôn ngữ. \[Unverified\] Mô hình lớn hơn có tuân theo
   chỉ thị ngôn ngữ hay không; chưa thử.
4. **Khóa Gateway chỉ nằm trong bộ nhớ.** Restart `aios-model-gateway` làm khóa dịch vụ
   của Hindsight mất hiệu lực (401) cho tới khi chạy lại `install.sh`. M1: `aiosd-core`
   cấp lại khóa khi Gateway khởi động, hoặc Gateway giữ khóa dịch vụ bền.
5. **API Hindsight mặc định không xác thực.** Mọi user cục bộ đều gọi được `127.0.0.1:8888`
   và đọc mọi bank, kể cả `user-secret`. Spike bật `ApiKeyTenantExtension` (khóa trong file
   env `0600` của chủ instance). Endpoint MCP `/mcp` cũng bật mặc định và cũng đòi khóa này.
   M1: chỉ `aiosd-memory` giữ khóa; cân nhắc tắt MCP (`HINDSIGHT_API_MCP_ENABLED=false`).
6. **Phụ thuộc nặng và giấy phép.** `hindsight-api-slim` bắt buộc kéo SDK của nhiều nhà
   cung cấp (`claude-agent-sdk`, `github-copilot-sdk`, `google-genai`, `boto3`, `cohere`,
   `litellm` …) dù AiOS chỉ dùng provider `openai`. `claude-agent-sdk` (MIT) mang theo binary
   `_bundled/claude` 277 MB; \[Unverified\] giấy phép của binary này chưa kiểm. `psycopg2-binary`
   là **LGPL**, ngoài danh sách giấy phép của AGENTS.md; đây là phụ thuộc bắt buộc của
   Hindsight, không phải mã AiOS liên kết tĩnh. Cần leader quyết. `onnxruntime` có mặt vì
   `markitdown[...]` → `magika`, không phải reranker.
7. **Sandbox.** Unit system có tường lửa IP thật: `bpftool cgroup show` thấy
   `sd_fw_ingress`, `sd_fw_egress` gắn vào cgroup unit. Kiểm bằng unit tạm cùng thuộc tính
   (`User=501`, `Slice=aios-svc.slice`, `IPAddressDeny=any`, `IPAddressAllow=localhost`):
   `curl https://pypi.org` → `Connection timed out` (rc 28); `curl 127.0.0.1:8888/health` → 200.
   Tiến trình chạy uid 501, `NoNewPrivs: 1`, `CapEff: 0`. `sudo systemd-run --pipe` vẫn lỗi
   `Connection reset by peer` như S-13; chạy không `--pipe` thì được.
8. **Postgres.** Role `aios_u501` không đăng nhập được database khác (`psql ... /postgres` →
   `Peer authentication failed`); `datacl` của `aios_u501` chỉ còn `{aios_u501=CTc/aios_u501}`.
   \[Inference\] Database `postgres` vẫn có quyền CONNECT mặc định cho PUBLIC; điều đang chặn
   là `pg_hba.conf` (dòng peer map chỉ khớp `aios_u501`). M1 nên `REVOKE CONNECT ... FROM PUBLIC`
   trên mọi database hệ thống.
9. **venv chủ root, unit đọc env của chủ instance.** Hindsight không sửa được mã của chính nó
   (`ProtectSystem=strict`, venv chủ root). File env do user sở hữu: user tự đổi được biến
   môi trường của instance của mình (kể cả `*_EXTENSION` nạp mã Python), nhưng mã đó chạy với
   quyền của chính user nên \[Inference\] không nâng quyền.

## 8. Kết luận cho D-18

D-18 ("Hindsight thay bảng bộ nhớ tự viết", đã chốt, có từ M1 theo D-36) **giữ nguyên**.
S-4 xác nhận các tiền đề kỹ thuật:

- Chạy được không torch: `hindsight-api-slim` 1.0.0 + `RERANKER_PROVIDER=rrf` + embedding
  qua Gateway; venv không có torch, sentence-transformers, transformers.
- Mọi lệnh LLM và embedding đi qua Gateway bằng khóa dịch vụ `cloud: false`, route local;
  tách bank theo nhãn và taint hoạt động (recall không thấy chéo bank).
- RAM ~0,4 GiB cho Hindsight + ~0,09 GiB cho Postgres, nằm trong ước tính 0,6–1,8 GB của
  review.md cho cả cụm Postgres + Hindsight + WeKnora + OpenShell.

Điều kiện cho M1 (đề xuất, chờ leader):

1. `retain` phải **không đồng bộ** (`retain_async`) và consolidation chạy khi máy rảnh;
   với mô hình local trên CPU, `retain` đồng bộ (26–456 s) không dùng được trong vòng hội thoại.
2. Gateway giữ hoặc cấp lại khóa dịch vụ khi khởi động (§7.4).
3. Bật xác thực API Hindsight, chỉ `aiosd-memory` giữ khóa (§7.5).
4. Quyết định về `psycopg2-binary` (LGPL) và binary trong `claude-agent-sdk` (§7.6); cân nhắc
   đóng gói venv gọn hơn (bỏ SDK provider không dùng) nếu 1,2 GB là quá lớn cho ảnh hệ thống.
5. Đo lại tốc độ `retain` và ngôn ngữ của fact trên phần cứng mục tiêu với mô hình S-13 chính thức.
