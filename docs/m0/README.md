# M0 — Thiết kế chi tiết các task

_Cập nhật: 2026-10-20_

**Thư mục này chứa thiết kế chi tiết cho từng task của mốc M0 ([roadmap.md §3](../roadmap.md)). File này chốt quy ước chung và giao diện giữa các task; mỗi file `<id>.md` mô tả một task. Worker đọc file này trước, rồi đọc file của task mình. Khi một file task mâu thuẫn với file này, file này thắng.**

Bảng task (thứ tự, phụ thuộc, người làm) nằm ở `.tasks/board.json` của leader. Mã yêu cầu `REQ-...` lấy từ [requirements.md](../requirements.md) ngày 2026-10-20; mã có thể đổi khi tài liệu gốc đổi, nên mỗi chỗ trích đều kèm mục thiết kế gốc.

## 1. Danh sách task

| ID | Tài liệu | Người làm | Phụ thuộc |
| --- | --- | --- | --- |
| ld-setup | [ld-setup.md](ld-setup.md) | leader | — |
| ld-vm | [ld-vm.md](ld-vm.md) | leader | — |
| ld-secspec | [ld-secspec.md](ld-secspec.md) | leader | ld-setup |
| audit | [audit.md](audit.md) | worker | ld-setup |
| registry | [registry.md](registry.md) | worker | ld-setup |
| sandbox | [sandbox.md](sandbox.md) | worker | ld-setup, ld-vm |
| policy | [policy.md](policy.md) | worker | ld-setup, ld-secspec |
| confirm | [confirm.md](confirm.md) | worker | ld-setup, ld-secspec |
| gateway | [gateway.md](gateway.md) | worker | ld-setup |
| tools3 | [tools3.md](tools3.md) | worker | ld-setup |
| llm | [llm.md](llm.md) | worker | ld-vm |
| image | [image.md](image.md) | worker | ld-setup |
| broker | [broker.md](broker.md) | worker | policy, confirm, audit, registry |
| executor | [executor.md](executor.md) | worker | broker, sandbox |
| hindsight | [hindsight.md](hindsight.md) | worker | ld-vm, gateway, llm |
| s14 | [s14.md](s14.md) | worker | sandbox |
| cognition | [cognition.md](cognition.md) | worker | broker, gateway, tools3 |
| s16 | [s16.md](s16.md) | worker | cognition, image |
| ld-gate | [ld-gate.md](ld-gate.md) | leader | executor, cognition, hindsight, s14 |

## 2. Cấu trúc repo (do ld-setup tạo)

```text
AiOS/
├── Cargo.toml                    # Cargo workspace
├── rust-toolchain.toml           # ghim phiên bản Rust stable
├── Makefile                      # make check, make vm-test
├── AGENTS.md                     # quy tắc cho coding agent
├── crates/
│   ├── aios-types/               # kiểu dùng chung (§3) — ld-setup
│   ├── aios-policy/              # policy
│   ├── aios-confirm/             # confirm
│   ├── aios-audit/               # audit
│   ├── aios-registry/            # registry
│   ├── aios-broker/              # broker, executor
│   ├── aios-sandbox-exec/        # sandbox (binary, chỉ build trên Linux)
│   ├── aiosd-core/               # binary ghép broker + admin socket — broker
│   └── aios-model-gateway/       # gateway (binary)
├── policy/                       # *.cedar, aios.cedarschema — ld-secspec viết bản đầu
├── testdata/
│   ├── policy-invariants/        # ld-secspec
│   └── confirm-vectors/          # ld-secspec
├── python/
│   └── aiosd-cognition/          # cognition (uv project, gồm lệnh aiosctl)
├── plugins/
│   └── tools/                    # tools3: sys/, fs/ (MCP server Python + plugin.toml)
├── image/                        # image: Containerfile, systemd units
├── dev/
│   ├── lima/aios-dev.yaml        # đã có
│   ├── vm-run.sh                 # ld-setup: chạy lệnh trong VM trên bản sao của worktree
│   └── vm/                       # ld-vm, llm, hindsight: script cài đặt trong VM
├── models.lock                   # llm
└── docs/
    ├── m0/                       # thư mục này
    └── spikes/                   # báo cáo spike: S-4, S-14, S-16, S-9
```

Mỗi task chỉ sửa thư mục ghi trong mục "Phạm vi" của task. Sửa `Cargo.toml` gốc chỉ để thêm crate của chính task vào `members` (ld-setup đã khai sẵn mọi member, nên thường không cần).

## 3. Kiểu dữ liệu dùng chung: crate `aios-types`

ld-setup viết crate này; các task khác dùng, không định nghĩa lại. Đổi kiểu ở đây là việc của leader.

```rust
// crates/aios-types/src/lib.rs (tóm tắt; bản thật có serde Serialize/Deserialize, Debug, Clone, PartialEq, Eq)

#[serde(rename_all = "lowercase")]
pub enum Privacy { Public, Personal, Secret }          // thứ tự: Public < Personal < Secret
#[serde(rename_all = "lowercase")]
pub enum Risk { Low, Medium, High }                    // thứ tự: Low < Medium < High
#[serde(rename_all = "lowercase")]
pub enum Taint { Clean, Tainted }
#[serde(rename_all = "lowercase")]
pub enum Channel { Local, Voice, Remote, Schedule }
#[serde(rename_all = "lowercase")]
pub enum Provenance { Stated, Observed, Inferred, External }

pub struct Principal {
    pub uid: u32,                  // từ SO_PEERCRED hoặc credential, không từ body (design §18.1)
    pub is_admin: bool,
    pub channel: Channel,
    pub session_id: Option<String>,// UUIDv7 dạng chuỗi
}

pub struct ToolSpec {              // do aios-registry tạo từ manifest đã ký
    pub name: String,              // "fs.search"
    pub plugin_id: String,         // "aios.fs"
    pub description: String,
    pub input_schema: serde_json::Value,
    pub risk: Risk,                // mức HIỆU LỰC (design §15.2), không phải mức khai báo
    pub reversible: bool,
    pub rollback_tool: Option<String>,
    pub always_confirm: bool,
    pub system_scope: bool,
    pub writes_trusted_core: bool,
    pub result_privacy: Privacy,   // nhãn gán cho kết quả tool (mặc định Personal)
    pub result_provenance: Provenance, // mặc định Observed; tool đọc nội dung ngoài: External
    pub entry: Vec<String>,        // lệnh chạy MCP server của plugin
    pub sandbox: SandboxProfile,
    pub timeout_ms: u64,           // mặc định 30000
}

pub struct SandboxProfile {        // aios-sandbox-exec đọc dạng JSON
    pub ro_paths: Vec<String>,
    pub rw_paths: Vec<String>,
    pub tcp_connect: Vec<u16>,     // rỗng = không mạng
    pub allow_unix_socket: Vec<String>,
    pub memory_max_mb: u32,        // mặc định 512
    pub landlock_required: bool,   // true cho mọi tool risk >= Medium (REQ-T-8-01, tech-stack §8)
}

pub struct SessionState {
    pub session_id: String,
    pub uid: u32,
    pub label: Privacy,            // nhãn theo session, chỉ tăng (design §5.2)
    pub taint: Taint,              // taint theo session, chỉ tăng (design §6.3)
}

pub enum ConfirmLevel { Dialog, DialogWithPin }
pub enum Verdict { Allow, Confirm(ConfirmLevel), Deny(String) }

pub struct ToolCall {
    pub tool: String,
    pub args: serde_json::Value,
    pub idempotency_key: String,   // "<task_id>:<cursor>" hoặc UUID; bắt buộc (REQ-T-5-01)
}

pub struct ToolOutput {
    pub content: serde_json::Value,// kết quả MCP (content[])
    pub is_error: bool,
    pub privacy: Privacy,          // = ToolSpec.result_privacy, có thể bị tool nâng, không hạ
    pub provenance: Provenance,
}
```

Hàm tiện ích trong `aios-types`: `Privacy::max(a, b)`, `Risk::max(a, b)`, `Taint::or(a, b)`.

## 4. Đường dẫn, cổng và socket trong M0

Theo [integration.md §11.1](../integration.md). M0 chỉ dùng các mục dưới đây.

| Thành phần | Địa chỉ | Ghi chú M0 |
| --- | --- | --- |
| Model Gateway (API) | `127.0.0.1:8090` | HTTP OpenAI-compatible |
| Model Gateway (quản trị) | `/run/aios/gateway-admin.sock` | Chỉ `aiosd-core` gọi; cấp khóa session, nâng nhãn |
| Mô hình local (llama-server) | `127.0.0.1:8081` (chat), `127.0.0.1:8082` (embedding) | Chỉ Gateway gọi |
| Tool Broker MCP `aios-tools` | `127.0.0.1:8091/mcp` | MCP streamable HTTP, Bearer = credential session |
| `aiosd-core` quản trị | `/run/aios/core.sock` | Unix socket, `SO_PEERCRED`; tạo session, nhận token xác nhận |
| Hindsight API | `127.0.0.1:8888` | spike S-4 |
| Audit log | `/var/lib/aios/audit/audit.jsonl` | |
| Registry | `/usr/share/aios/plugins/`, `/var/lib/aios/plugins/{vendor,local}/` | Trong test: thư mục tạm |
| Trust store | `/usr/share/aios/trust/` (`*.pub`, `revoked.json`) | |
| Khóa ký của trusted UI (M0: bộ ký dev) | `/etc/aios/dev-trusted-ui.key` | Chỉ M0; M1 thay bằng `aios-trusted-ui` |

Trong test, mọi đường dẫn và cổng PHẢI đọc được từ cấu hình hoặc biến môi trường, không viết cứng trong logic.

## 5. Quy ước mã

| Mục | Quy ước |
| --- | --- |
| Rust | Edition 2024, stable ghim trong `rust-toolchain.toml`; `cargo fmt`; `cargo clippy -- -D warnings`; lỗi dùng `thiserror` trong thư viện, `anyhow` trong binary |
| Async | `tokio` |
| Serde | JSON qua `serde_json`; TOML qua `toml`; YAML qua `serde_yaml_ng` \[Unverified tên crate, ld-setup chốt\] |
| Python | 3.14 (Fedora 44), `uv`, `ruff`, `pytest`; type hint bắt buộc ở hàm công khai |
| Log | Rust: `tracing` dạng JSON; Python: `logging` dạng JSON; không log nội dung nhãn `secret` |
| Unsafe | Cấm `unsafe` trừ `aios-sandbox-exec` (syscall); mỗi khối `unsafe` có comment lý do |
| Phụ thuộc | Chỉ thêm crate/gói có giấy phép tương thích Apache-2.0 (MIT, Apache-2.0, BSD, ISC, Unicode, Zlib, MPL-2.0); thêm phụ thuộc mới ghi vào mục "Quyết định tự đưa ra" khi báo DONE |
| Ngôn ngữ | Comment và thông báo lỗi bằng tiếng Anh trong mã; tài liệu bằng tiếng Việt |

## 6. Kiểm tra

| Lệnh | Chạy ở | Nội dung |
| --- | --- | --- |
| `make check` | Mac | `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace` (crate chỉ-Linux bị bỏ qua trên Mac), `uv run --directory python/aiosd-cognition pytest`, `ruff check`, `python3 -I scripts/extract_requirements.py --check` |
| `dev/vm-run.sh <lệnh>` | VM `aios-dev` | Chép bản hiện tại của worktree vào `~/build/<tên>` trong VM (không dùng thư mục chia sẻ, vì worktree của worker nằm ngoài thư mục chia sẻ), rồi chạy lệnh ở đó |
| `make vm-test` | VM | `dev/vm-run.sh 'cargo test --workspace'` |

Một task chỉ được báo DONE khi mọi lệnh trong mục "Nghiệm thu" của task đó chạy đạt, kèm output.

## 7. Giao thức báo cáo của worker

- Gặp chỗ thiết kế không rõ hoặc mâu thuẫn: dừng, trả lời bắt đầu bằng `NEED_ADVICE`, nêu câu hỏi và các phương án. Không tự đoán ở chỗ liên quan an ninh (policy, token, nhãn, sandbox, registry).
- Xong: trả lời bắt đầu bằng `DONE`, kèm: danh sách file đã sửa, output của lệnh nghiệm thu, và mục "Quyết định tự đưa ra" (mỗi lựa chọn không có trong tài liệu).
- Không sửa file ngoài phạm vi; không sửa `docs/`, `policy/`, `testdata/` trừ khi task cho phép.
