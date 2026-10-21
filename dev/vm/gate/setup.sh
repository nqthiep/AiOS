#!/usr/bin/env bash
# Dựng toàn bộ M0 trong VM aios-dev cho cổng nghiệm thu (docs/m0/ld-gate.md).
# Chạy lại được. Cần: dev/vm/setup.sh, dev/vm/llm/install.sh, dev/vm/hindsight/install.sh đã chạy.
# Dùng: dev/vm-run.sh 'bash dev/vm/gate/setup.sh'
#
# Bố cục (mọi trạng thái của cổng nằm dưới $GATE, chủ là user của VM):
#   /opt/aios/bin/                     binary release
#   $GATE/plugins/core/aios.{sys,fs}   plugin đã ký (root, chỉ đọc) + venv dựng sẵn
#   $GATE/trust/core/gate.pub          khóa công khai ký plugin (bản chép của $GATE/sign.pub); khóa bí mật $GATE/sign.key (0600)
#   $GATE/policy/                      bản chép policy/ của repo
#   $GATE/audit/audit.jsonl            audit log
#   $GATE/workspaces/<uid>/            workspace của user
#   /etc/aios/dev-trusted-ui.{key,pub} khóa dev của bộ ký xác nhận (key: aios-trusted, 0400)
#   ~/.config/systemd/user/aiosd-core.service
#   $GATE/aiosctl.env                  biến AIOS_* cho aiosctl
set -euo pipefail
cd "$(dirname "$0")/../../.."
REPO="$PWD"
U="$(id -un)"
UID_="$(id -u)"
BIN=/opt/aios/bin
GATE=/var/lib/aios/gate
ADMIN_SOCKET=/run/aios/gateway-admin.sock
log() { echo "[gate-setup] $*"; }
die() { echo "[gate-setup] ERROR: $*" >&2; exit 1; }
wait_until() { local n=$1; shift; for _ in $(seq "$n"); do "$@" && return 0; sleep 1; done; return 1; }

# 0. Gói cần cho kịch bản cổng: `script` cấp pty cho bộ ký xác nhận (aios-dev-confirm cần TTY).
command -v script >/dev/null || sudo dnf -y -q install util-linux-script

# 1. Binary release.
log "build release"
cargo build --release -q -p aiosd-core -p aios-model-gateway -p aios-sandbox-exec \
  -p aios-audit -p aios-confirm -p aios-registry
T="${CARGO_TARGET_DIR:-target}/release"
sudo install -d -m 0755 "$BIN"
for b in aiosd-core aios-model-gateway aios-sandbox-exec aios-audit aios-dev-confirm aios-plugin-sign; do
  sudo install -m 0755 "$T/$b" "$BIN/$b"
done

# 2. Thư mục trạng thái.
sudo install -d -o "$U" -g "$U" -m 0755 "$GATE"
install -d -m 0755 "$GATE/plugins/vendor" "$GATE/plugins/local" "$GATE/trust/core" "$GATE/policy"
install -d -m 0700 "$GATE/audit" "$GATE/workspaces"

# 3. Khóa dev của bộ ký xác nhận: 32 byte seed thô (aios-dev-confirm), khóa công khai base64.
if ! id aios-trusted >/dev/null 2>&1; then
  sudo useradd --system --no-create-home --shell /usr/sbin/nologin aios-trusted
fi
if ! sudo test -s /etc/aios/dev-trusted-ui.key; then
  log "sinh khóa dev trusted UI"
  sudo install -d -m 0755 /etc/aios
  sudo sh -c 'umask 077; head -c 32 /dev/urandom > /etc/aios/dev-trusted-ui.key'
  sudo chown aios-trusted:aios-trusted /etc/aios/dev-trusted-ui.key
  sudo chmod 0400 /etc/aios/dev-trusted-ui.key
fi
sudo cat /etc/aios/dev-trusted-ui.key | uv run -q --no-project --with cryptography python -I -c '
import base64, sys
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives.serialization import Encoding, PublicFormat
k = Ed25519PrivateKey.from_private_bytes(sys.stdin.buffer.read())
print(base64.b64encode(k.public_key().public_bytes(Encoding.Raw, PublicFormat.Raw)).decode())
' | sudo tee /etc/aios/dev-trusted-ui.pub >/dev/null
sudo chmod 0644 /etc/aios/dev-trusted-ui.pub

# 4. Plugin: chép, dựng venv, ký, rồi khóa quyền ghi.
[ -s "$GATE/sign.key" ] || "$BIN/aios-plugin-sign" --gen-key "$GATE/sign.key" >/dev/null
chmod 0600 "$GATE/sign.key"
cp "$GATE/sign.pub" "$GATE/trust/core/gate.pub"
sudo rm -rf "$GATE/plugins/core"
install -d -m 0755 "$GATE/plugins/core"
for p in sys fs; do
  dst="$GATE/plugins/core/aios.$p"
  install -d -m 0755 "$dst"
  cp -r "$REPO/plugins/tools/$p/"{plugin.toml,pyproject.toml,uv.lock,server.py,src} "$dst/"
  UV_PYTHON_DOWNLOADS=never uv sync -q --directory "$dst" --frozen --no-dev --python /usr/bin/python3
  "$BIN/aios-plugin-sign" --key "$GATE/sign.key" "$dst/plugin.toml" >/dev/null
done
sudo chown -R root:root "$GATE/plugins/core"
cp "$REPO"/policy/{aios.cedarschema,auto.cedar,confirmable.cedar,forbid.cedar} "$GATE/policy/"

# 5. Model Gateway: cấu hình của cổng, khởi động lại, cấp lại khóa dịch vụ cho Hindsight
#    (khóa ảo chỉ nằm trong bộ nhớ của Gateway, docs/spikes/S-4-hindsight.md).
sudo install -m 0644 "$REPO/dev/vm/gate/gateway.yaml" /etc/aios/gateway.yaml
systemctl --user restart aios-model-gateway
wait_until 30 test -S "$ADMIN_SOCKET" || die "gateway: không thấy $ADMIN_SOCKET"
# Gateway has no /health; any HTTP answer (401 without a key) means it is up.
wait_until 30 curl -s -o /dev/null http://127.0.0.1:8090/v1/models || die "gateway: không trả lời"
if [ -f "/etc/aios/hindsight/$UID_.env" ]; then
  key="$(curl -sf --unix-socket "$ADMIN_SOCKET" -H 'content-type: application/json' \
    -d "{\"uid\":$UID_,\"client\":\"hindsight\"}" http://gateway/v1/sessions | jq -er '.key')" \
    || die "gateway: không cấp được khóa cho hindsight"
  env_file="/etc/aios/hindsight/$UID_.env"
  sudo sed -i -e "s|^HINDSIGHT_API_LLM_API_KEY=.*|HINDSIGHT_API_LLM_API_KEY=$key|" \
    -e "s|^HINDSIGHT_API_EMBEDDINGS_OPENAI_API_KEY=.*|HINDSIGHT_API_EMBEDDINGS_OPENAI_API_KEY=$key|" \
    "$env_file"
  sudo chown "$U:$U" "$env_file"
  sudo chmod 0600 "$env_file"
  sudo systemctl restart "hindsight-api@$UID_"
  log "hindsight: đã cấp lại khóa Gateway"
fi

# 6. aiosd-core (unit user: Executor dùng `systemd-run --user`).
cat > "$GATE/autonomy.yaml" <<'YAML'
autonomy:
  level: 1
YAML
cat > "$GATE/core.yaml" <<YAML
registry_core_dir: $GATE/plugins/core
registry_vendor_dir: $GATE/plugins/vendor
registry_local_dir: $GATE/plugins/local
trust_dir: $GATE/trust
policy_dir: $GATE/policy
audit_file: $GATE/audit/audit.jsonl
trusted_ui_pubkey: /etc/aios/dev-trusted-ui.pub
autonomy_file: $GATE/autonomy.yaml
gateway_admin_socket: $ADMIN_SOCKET
admin_socket: /run/aios/core.sock
workspace_root: $GATE/workspaces
YAML
install -d "$HOME/.config/systemd/user"
cat > "$HOME/.config/systemd/user/aiosd-core.service" <<UNIT
[Unit]
Description=AiOS core daemon (M0 gate, dev VM)
After=aios-model-gateway.service

[Service]
ExecStart=$BIN/aiosd-core --config $GATE/core.yaml
Environment=AIOS_SANDBOX_EXEC=$BIN/aios-sandbox-exec
Environment=RUST_LOG=info
Restart=on-failure
RestartSec=2s

[Install]
WantedBy=default.target
UNIT
systemctl --user daemon-reload
systemctl --user enable -q aiosd-core
systemctl --user restart aiosd-core
wait_until 30 test -S /run/aios/core.sock || die "aiosd-core: không thấy /run/aios/core.sock"

# 7. aiosctl.
uv tool install -q --force --reinstall --python /usr/bin/python3 "$REPO/python/aiosd-cognition"
cat > "$GATE/aiosctl.env" <<ENV
export PATH="\$HOME/.local/bin:\$PATH"
export AIOS_AUDIT_FILE=$GATE/audit/audit.jsonl
export AIOS_AUDIT_CMD=$BIN/aios-audit
export AIOS_CONFIRM_CMD="sudo -u aios-trusted $BIN/aios-dev-confirm --key /etc/aios/dev-trusted-ui.key"
ENV

# 8. Workspace mẫu cho bước 3 của cổng.
WS="$GATE/workspaces/$UID_"
install -d -m 0700 "$WS"
cat > "$WS/ke-hoach-quy-4.md" <<'MD'
# Kế hoạch quý 4

- Hoàn thành cổng M0 của AiOS trước 31/10.
- Đo baseline tài nguyên S-9 trong VM.
- Bắt đầu M1: app chat và trusted UI.
MD
log "xong: aiosd-core $(systemctl --user is-active aiosd-core), gateway $(systemctl --user is-active aios-model-gateway)"
