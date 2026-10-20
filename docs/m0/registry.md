# registry — Registry manifest đã ký

| Thuộc tính | Giá trị |
| --- | --- |
| Người làm | worker |
| Phụ thuộc | ld-setup |
| Chặn | broker |
| Nguồn thiết kế | design.md §9.2, §9.6, §11.5, §15.2, §15.7; REQ-D-9.3-04, REQ-D-15.7-01, REQ-D-11.5-02 |

## Mục tiêu

Crate `aios-registry` đọc `plugin.toml` từ các thư mục registry, kiểm chữ ký, gán cấp tin cậy theo nơi nạp, tính mức rủi ro hiệu lực, và trả danh sách `ToolSpec` (aios-types) cho Broker. Mọi thông tin quyền mà Policy Engine dùng chỉ đến từ crate này.

## Phạm vi

Được sửa: `crates/aios-registry/**`.

## Thiết kế

**Nơi nạp và cấp tin cậy** (trường `trust` trong manifest bị bỏ qua, design §15.2):

| Thư mục | Cấp tin cậy | Chữ ký |
| --- | --- | --- |
| `<core_dir>` (`/usr/share/aios/plugins`) | `core` | Bắt buộc, khóa trong `<trust_dir>/core/*.pub` |
| `<vendor_dir>` (`/var/lib/aios/plugins/vendor`) | `vendor` | Bắt buộc, khóa trong `<trust_dir>/vendors/*.pub` |
| `<local_dir>` (`/var/lib/aios/plugins/local`) | `community` (M0 chưa có `generated`) | Bắt buộc, khóa cục bộ `<trust_dir>/local.pub` |

Mỗi plugin là một thư mục chứa `plugin.toml` và `plugin.toml.sig`. Chữ ký: Ed25519 trên **đúng các byte** của `plugin.toml`, file `.sig` chứa base64 chuẩn (64 byte sau giải mã). Plugin thiếu chữ ký, chữ ký sai, hoặc khóa nằm trong `revoked.json` thì bị bỏ qua và trả về trong danh sách lỗi (không làm hỏng cả registry).

**`revoked.json`:** `{"keys": ["<base64 pubkey>"], "plugins": [{"id": "...", "version": "..."}]}`.

**Lược đồ `plugin.toml`** theo design §15.2 và §9.2, thêm 2 trường của README.md §3: `result_privacy` (mặc định `personal`), `result_provenance` (mặc định `observed`). Trường không biết → lỗi (deny unknown fields). `api` phải là `aios:plugin@0.1`.

**Mức rủi ro hiệu lực** = `max(tools[].risk khai báo, mức suy ra từ capabilities của plugin)`:

| Capability (dạng chuỗi) | Mức tối thiểu |
| --- | --- |
| `fs.read:<path>` | low |
| `fs.write:$WORKSPACE[/...]` | low |
| `fs.write:<path ngoài workspace>`, `net:<tên>`, `dbus:<tên>` (ghi) | medium |
| `priv.escalate:<helper>`, `fs.delete:<path ngoài workspace>`, `send:<kênh>` | high |
| Chuỗi không khớp mẫu nào | **high** (fail closed) |

- `risk_overrides` chỉ áp dụng khi cấp tin cậy là `core` hoặc `vendor`; và chỉ được **nâng** mức (override thấp hơn mức suy ra bị bỏ qua, ghi cảnh báo).
- `always_confirm`, `system_scope` chỉ được bật, không ai hạ được: plugin `community` khai `system_scope = true` vẫn giữ true.
- Tên tool PHẢI bắt đầu bằng phần cuối của `id` plugin cộng dấu chấm (ví dụ plugin `aios.fs` → tool `fs.*`); sai thì bỏ tool đó.
- `input_schema` phải là JSON Schema hợp lệ (kiểm bằng crate `jsonschema`).
- Plugin trùng `id` ở nhiều thư mục: cấp tin cậy cao hơn thắng (core > vendor > community).

**Giao diện:**

```rust
pub struct RegistryConfig { pub core_dir: PathBuf, pub vendor_dir: PathBuf, pub local_dir: PathBuf, pub trust_dir: PathBuf }
pub struct Registry { /* map tool name -> ToolSpec, plugin id -> PluginInfo */ }
pub struct LoadReport { pub loaded: Vec<String>, pub rejected: Vec<(PathBuf, String)> }
impl Registry {
    pub fn load(cfg: &RegistryConfig) -> (Registry, LoadReport);
    pub fn tool(&self, name: &str) -> Option<&ToolSpec>;
    pub fn tools(&self) -> impl Iterator<Item = &ToolSpec>;
}
```

**Công cụ ký cho dev:** binary `aios-plugin-sign --key <file> <plugin.toml>` sinh `.sig` (dùng cho test và cho tools3). Khóa dev sinh bằng `aios-plugin-sign --gen-key <file>`.

## Kiểm thử và nghiệm thu

Test bắt buộc: plugin hợp lệ nạp được; sửa 1 byte manifest → bị từ chối; khóa bị thu hồi → bị từ chối; `community` có `risk_overrides` hạ mức → bị bỏ qua; capability lạ → `high`; `fs.write` ngoài workspace + khai `low` → `medium`; trường lạ → từ chối; tên tool sai namespace → bỏ tool; cùng id ở core và local → lấy core.

```bash
cargo test -p aios-registry
cargo clippy -p aios-registry -- -D warnings
```

## Ngoài phạm vi

Nạp và gỡ lúc chạy, theo dõi thư mục (M1); plugin `wasm` (M3).
