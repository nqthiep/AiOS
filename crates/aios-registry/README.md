# aios-registry

Registry manifest đã ký của AiOS. Đặc tả: [docs/m0/registry.md](../../docs/m0/registry.md),
[docs/design.md](../../docs/design.md) §9.2, §15.2, §15.7.

Crate đọc `plugin.toml` từ ba thư mục nạp, kiểm chữ ký Ed25519 trên **đúng các byte**
của manifest, gán cấp tin cậy theo nơi nạp, tính mức rủi ro hiệu lực và trả
`ToolSpec` (crate `aios-types`) cho Tool Broker.

## Nơi nạp và cấp tin cậy

| Thư mục | Cấp tin cậy | Khóa công khai |
| --- | --- | --- |
| `core_dir` (`/usr/share/aios/plugins`) | `core` | `<trust_dir>/core/*.pub` |
| `vendor_dir` (`/var/lib/aios/plugins/vendor`) | `vendor` | `<trust_dir>/vendors/*.pub` |
| `local_dir` (`/var/lib/aios/plugins/local`) | `community` | `<trust_dir>/local.pub` |

Trường `trust` trong manifest bị bỏ qua: cấp tin cậy lấy từ nơi nạp.

## Kho tin cậy

```text
<trust_dir>/
├── core/*.pub        # base64 của 32 byte khóa công khai
├── vendors/*.pub     # base64 của 32 byte khóa công khai
├── local.pub         # base64 của 32 byte khóa công khai
└── revoked.json      # {"keys": ["<base64 pubkey>"], "plugins": [{"id": "...", "version": "..."}]}
```

`revoked.json` hỏng thì không plugin nào được nạp (fail closed). Một file `*.pub`
hỏng chỉ bị bỏ qua, kèm cảnh báo.

## Manifest `plugin.toml`

```toml
id = "aios.fs"                     # bắt buộc, namespace của tool = phần cuối của id
version = "1.0.0"                  # bắt buộc
api = "aios:plugin@0.1"            # bắt buộc, chỉ hỗ trợ giá trị này
provides = ["tool"]                # bắt buộc
runtime = "mcp"                    # bắt buộc: wasm | process | mcp
entry = ["/usr/bin/env", "python3", "-I", "server.py"]   # bắt buộc, chuỗi hoặc mảng
trust = "core"                     # có thể có, luôn bị bỏ qua
capabilities = ["fs.read:$WORKSPACE", "fs.write:$WORKSPACE"]  # bắt buộc, có thể rỗng
inject = []
config_schema = "config.schema.json"
desktop_entry = "aios-fs.desktop"

[sandbox]                          # tùy chọn, mặc định theo runtime
ro_paths = ["$WORKSPACE"]          # $WORKSPACE giữ nguyên dạng chuỗi
rw_paths = ["$WORKSPACE/notes"]
tcp_connect = []                   # rỗng = không mạng
allow_unix_socket = []
memory_max_mb = 512
landlock_required = true

timeout_ms = 30000                 # tùy chọn, mặc định 30000
result_privacy = "personal"        # tùy chọn, mặc định personal
result_provenance = "observed"     # tùy chọn, mặc định observed

[risk_overrides]                   # chỉ core/vendor; chỉ được nâng, không được hạ
"net:flathub" = "high"

[[tools]]
name = "fs.search"                 # phải nằm trong namespace của plugin ("fs.")
description = "Tìm file trong workspace"
risk = "low"                       # bắt buộc
reversible = true
rollback_tool = "fs.undo_note"
supports_dry_run = false
runs_via = "aios-actuator"
always_confirm = false             # chỉ bật được, không ai hạ được
system_scope = false               # chỉ bật được, không ai hạ được
writes_trusted_core = false
result_privacy = "personal"        # ghi đè mặc định của plugin
result_provenance = "external"     # ghi đè mặc định của plugin
input_schema = '{ "type": "object" }'   # JSON Schema, chuỗi JSON hoặc bảng inline
```

Trường lạ trong manifest (kể cả trong `[sandbox]`) → plugin bị từ chối.

## Mức rủi ro hiệu lực

`rủi ro hiệu lực của tool = max(rủi ro khai báo, rủi ro suy ra từ capability của plugin)`

| Capability | Mức tối thiểu |
| --- | --- |
| `fs.read:<path>` | `low` |
| `fs.write:$WORKSPACE[/...]` | `low` |
| `fs.write:<path ngoài workspace>` | `medium` |
| `net:<tên>`, `dbus:<tên>` | `medium` |
| `priv.escalate:<helper>`, `fs.delete:<path>`, `send:<kênh>` | `high` |
| chuỗi không khớp mẫu nào | `high` (fail closed) |

`risk_overrides` chỉ áp dụng cho `core` và `vendor`; khoá của map là tên capability
(để nâng mức suy ra của capability đó) hoặc tên tool (để nâng mức hiệu lực của tool đó).
Mọi override thấp hơn mức đang có bị bỏ qua kèm cảnh báo.

## Công cụ ký (dev)

```bash
# sinh cặp khóa: khóa bí mật ở <file>, khóa công khai ở <file>.pub
cargo run -p aios-registry --bin aios-plugin-sign -- --gen-key /tmp/dev.key

# ký manifest: ghi <plugin.toml>.sig
cargo run -p aios-registry --bin aios-plugin-sign -- --key /tmp/dev.key plugins/tools/fs/plugin.toml
```

## Xem registry đã nạp

```bash
cargo run -p aios-registry --example load -- --core <dir> [--vendor <dir>] [--local <dir>] --trust <dir>
```

Lệnh in danh sách plugin, danh sách tool (kèm mức rủi ro hiệu lực) và danh sách lỗi.
Các đường dẫn cũng lấy được từ `AIOS_REGISTRY_CORE_DIR`, `AIOS_REGISTRY_VENDOR_DIR`,
`AIOS_REGISTRY_LOCAL_DIR`, `AIOS_REGISTRY_TRUST_DIR`.
