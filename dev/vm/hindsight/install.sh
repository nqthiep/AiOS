#!/usr/bin/env bash
# Cài Hindsight (spike S-4, docs/m0/hindsight.md) trong VM aios-dev (Fedora 44).
#
#   1. Postgres 18 + pgvector: initdb nếu chưa có, một role + một database
#      `aios_u<uid>` (peer auth qua Unix socket, không mật khẩu), extension
#      vector, pg_trgm, btree_gin tạo sẵn bằng superuser.
#   2. Model Gateway: build `aios-model-gateway` (release), cài vào /opt/aios/bin,
#      config dev/vm/hindsight/gateway.yaml -> /etc/aios/gateway.yaml, chạy
#      bằng unit systemd user `aios-model-gateway`, admin socket
#      /run/aios/gateway-admin.sock (admin uid = user đang chạy script).
#   3. Cấp khóa dịch vụ cho client `hindsight` (cloud: false) qua admin socket.
#      Khóa chỉ nằm trong bộ nhớ Gateway: restart Gateway -> chạy lại script.
#   4. venv /opt/aios/hindsight bằng `uv sync --frozen` (uv.lock), không extra
#      local-ml (không torch). Thoát lỗi nếu venv có torch/sentence-transformers.
#   5. Env /etc/aios/hindsight/<uid>.env (chủ <uid>, 0600) từ hindsight.env.example,
#      unit system hindsight-api@<uid>.service, đợi API sẵn sàng.
#
# Dùng (trong VM, từ gốc repo): bash dev/vm/hindsight/install.sh
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/../../.." && pwd)"

log() { printf '[hindsight] %s\n' "$*" >&2; }
die() { printf '[hindsight] ERROR: %s\n' "$*" >&2; exit 1; }

USER_UID="$(id -u)"
USER_NAME="$(id -un)"
USER_GROUP="$(id -gn)"
DB_ROLE="aios_u${USER_UID}"
DB_NAME="aios_u${USER_UID}"
PGDATA="${AIOS_PGDATA:-/var/lib/pgsql/data}"

GATEWAY_PORT="${AIOS_GATEWAY_PORT:-8090}"
GATEWAY_BIN="${AIOS_GATEWAY_BIN:-/opt/aios/bin/aios-model-gateway}"
GATEWAY_CONFIG="${AIOS_GATEWAY_CONFIG:-/etc/aios/gateway.yaml}"
GATEWAY_USERS_DIR="${AIOS_GATEWAY_USERS_DIR:-/var/lib/aios/users}"
ADMIN_SOCKET="${AIOS_GATEWAY_ADMIN_SOCKET:-/run/aios/gateway-admin.sock}"
UNIT_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"

HINDSIGHT_PORT="${AIOS_HINDSIGHT_PORT:-8888}"
VENV="/opt/aios/hindsight"
ENV_DIR="/etc/aios/hindsight"
ENV_FILE="$ENV_DIR/$USER_UID.env"
UNIT="hindsight-api@$USER_UID.service"

wait_until() { # <tries> <command...>
  local tries="$1" i
  shift
  for ((i = 0; i < tries; i++)); do
    if "$@" >/dev/null 2>&1; then return 0; fi
    sleep 1
  done
  return 1
}

psql_admin() { sudo -u postgres PGOPTIONS="-c client_min_messages=warning" psql -v ON_ERROR_STOP=1 -qtA "$@"; }

setup_postgres() {
  rpm -q postgresql-contrib >/dev/null 2>&1 ||
    sudo dnf install -y -q postgresql-contrib >/dev/null # pg_trgm, btree_gin
  if ! sudo test -f "$PGDATA/PG_VERSION"; then
    log "initdb $PGDATA"
    sudo postgresql-setup --initdb
  fi

  # Peer auth over the Unix socket: OS user -> role aios_u<uid>, this database only.
  local map="aios_u${USER_UID}" hba="$PGDATA/pg_hba.conf" ident="$PGDATA/pg_ident.conf"
  local hba_line="local $DB_NAME $DB_ROLE peer map=$map"
  if ! sudo grep -qxF "$hba_line" "$hba"; then
    sudo sed -i "1i $hba_line" "$hba"
  fi
  local ident_line="$map $USER_NAME $DB_ROLE"
  sudo grep -qxF "$ident_line" "$ident" || echo "$ident_line" | sudo tee -a "$ident" >/dev/null

  sudo systemctl enable --now postgresql.service >/dev/null
  sudo systemctl reload postgresql.service
  wait_until 30 sudo -u postgres psql -qtAc 'select 1' || die "postgresql không sẵn sàng"

  if [ "$(psql_admin -c "select 1 from pg_roles where rolname='$DB_ROLE'")" != 1 ]; then
    psql_admin -c "create role $DB_ROLE login nosuperuser nocreatedb nocreaterole noinherit"
  fi
  if [ "$(psql_admin -c "select 1 from pg_database where datname='$DB_NAME'")" != 1 ]; then
    psql_admin -c "create database $DB_NAME owner $DB_ROLE"
  fi
  psql_admin -c "revoke all on database $DB_NAME from public"
  # pgvector is not a trusted extension: create it as superuser.
  psql_admin -d "$DB_NAME" -c "create extension if not exists vector; \
    create extension if not exists pg_trgm; create extension if not exists btree_gin"
  log "postgres: database $DB_NAME, role $DB_ROLE (peer, map $map)"
}

install_gateway() {
  [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
  log "build aios-model-gateway (release)"
  (cd "$REPO_ROOT" && cargo build -q -p aios-model-gateway --release)
  local target="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
  sudo install -D -m 0755 "$target/release/aios-model-gateway" "$GATEWAY_BIN"
  install -D -m 0644 "$SCRIPT_DIR/gateway.yaml" "$GATEWAY_CONFIG"
  mkdir -p "$UNIT_DIR"
  sed -e "s|@BIN@|$GATEWAY_BIN|g" -e "s|@CONFIG@|$GATEWAY_CONFIG|g" \
    -e "s|@USERS_DIR@|$GATEWAY_USERS_DIR|g" -e "s|@ADMIN_SOCKET@|$ADMIN_SOCKET|g" \
    -e "s|@ADMIN_SOCKET_DIR@|$(dirname -- "$ADMIN_SOCKET")|g" -e "s|@ADMIN_UID@|$USER_UID|g" \
    "$SCRIPT_DIR/aios-model-gateway.service.in" >"$UNIT_DIR/aios-model-gateway.service"
  systemctl --user daemon-reload
  systemctl --user enable -q aios-model-gateway.service
  systemctl --user restart aios-model-gateway.service
  wait_until 30 test -S "$ADMIN_SOCKET" || die "gateway: không thấy admin socket $ADMIN_SOCKET"
  wait_until 30 curl -s -o /dev/null "http://127.0.0.1:$GATEWAY_PORT/v1/models" ||
    die "gateway: không nghe 127.0.0.1:$GATEWAY_PORT"
  log "gateway: 127.0.0.1:$GATEWAY_PORT, admin $ADMIN_SOCKET"
}

# Service key for client `hindsight`; printed on stdout only, never logged.
issue_gateway_key() {
  curl -sf --unix-socket "$ADMIN_SOCKET" -H 'content-type: application/json' \
    -d "{\"uid\":$USER_UID,\"client\":\"hindsight\"}" http://gateway/v1/sessions |
    jq -er '.key'
}

install_venv() {
  log "uv sync -> $VENV (uv.lock, không extra local-ml)"
  sudo install -d -m 0755 /opt/aios
  sudo env UV_PROJECT_ENVIRONMENT="$VENV" uv sync -q --frozen --no-cache --compile-bytecode \
    --python /usr/bin/python3.14 --project "$SCRIPT_DIR"
  local banned
  banned="$(uv pip list --python "$VENV/bin/python" --format freeze 2>/dev/null |
    grep -iE '^(torch|sentence-transformers|transformers|flashrank)==' || true)"
  [ -z "$banned" ] || die "venv có gói bị cấm: $banned"
  log "venv: $(sudo du -sh "$VENV" | cut -f1), $("$VENV/bin/python" -c \
    'import importlib.metadata as m; print("hindsight-api-slim", m.version("hindsight-api-slim"))')"
}

write_env() {
  local gateway_key="$1" api_key tmp
  api_key="$(openssl rand -hex 32)"
  tmp="$(mktemp)"
  chmod 0600 "$tmp"
  sed -e "s|@HINDSIGHT_PORT@|$HINDSIGHT_PORT|g" -e "s|@GATEWAY_PORT@|$GATEWAY_PORT|g" \
    -e "s|@DB_ROLE@|$DB_ROLE|g" -e "s|@DB_NAME@|$DB_NAME|g" \
    -e "s|@GATEWAY_KEY@|$gateway_key|g" -e "s|@API_KEY@|$api_key|g" \
    "$SCRIPT_DIR/hindsight.env.example" >"$tmp"
  sudo install -d -m 0755 -o root -g root "$ENV_DIR"
  sudo install -m 0600 -o "$USER_NAME" -g "$USER_GROUP" "$tmp" "$ENV_FILE"
  rm -f -- "$tmp"
  log "env: $ENV_FILE (0600)"
}

start_hindsight() {
  sudo install -m 0644 "$SCRIPT_DIR/hindsight-api@.service" /etc/systemd/system/hindsight-api@.service
  sudo systemctl daemon-reload
  sudo systemctl enable -q "$UNIT"
  sudo systemctl restart "$UNIT"
  if ! wait_until 180 curl -sf "http://127.0.0.1:$HINDSIGHT_PORT/health"; then
    sudo journalctl -u "$UNIT" --no-pager -n 40 >&2 || true
    die "$UNIT: không sẵn sàng tại 127.0.0.1:$HINDSIGHT_PORT"
  fi
  log "$UNIT: sẵn sàng tại http://127.0.0.1:$HINDSIGHT_PORT"
}

main() {
  command -v uv >/dev/null || die "thiếu uv (dev/vm/setup.sh)"
  setup_postgres
  install_gateway
  local key
  key="$(issue_gateway_key)" || die "gateway: không cấp được khóa dịch vụ cho client hindsight"
  install_venv
  write_env "$key"
  start_hindsight
}

main "$@"
