#!/usr/bin/env bash
# Cài mô hình local của AiOS trong VM aios-dev (Fedora 44).
#
#   1. Đọc `models.lock` (nguồn chân lý duy nhất: URL, SHA-256, kích thước).
#   2. Tải file GGUF từ Hugging Face bằng curl vào
#      /var/lib/aios/models/<name>/<12 ký tự đầu của sha256>/.
#      Sau khi tải, kiểm SHA-256: sai -> xóa file và thoát lỗi.
#      Chạy lại: file đã có và đúng hash thì không tải lại.
#   3. Render unit systemd user (từ *.service.in) rồi bật:
#      aios-llm-chat  (chat, 127.0.0.1:8081)
#      aios-llm-embed (embedding, 127.0.0.1:8082)
#
# Dùng: bash dev/vm/llm/install.sh
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

# Log ra stderr để stdout chỉ mang dữ liệu (đường dẫn file) cho phần gọi.
log() { printf '[llm] %s\n' "$*" >&2; }
err() { printf '[llm] ERROR: %s\n' "$*" >&2; }
die() { err "$*"; exit 1; }

# models.lock nằm ở gốc repo; tìm bằng cách đi lên từ thư mục script.
repo_root() {
  local dir="$SCRIPT_DIR"
  while :; do
    if [ -f "$dir/models.lock" ]; then printf '%s\n' "$dir"; return 0; fi
    [ "$dir" = "/" ] && return 1
    dir="$(dirname -- "$dir")"
  done
}
REPO_ROOT="$(repo_root)" || die "không tìm thấy models.lock từ $SCRIPT_DIR trở lên"

LOCK_FILE="${AIOS_MODELS_LOCK:-$REPO_ROOT/models.lock}"
MODEL_ROOT="${AIOS_MODEL_ROOT:-/var/lib/aios/models}"
UNIT_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
CHAT_PORT="${AIOS_LLM_CHAT_PORT:-8081}"
EMBED_PORT="${AIOS_LLM_EMBED_PORT:-8082}"
CHAT_MEMORY_MAX="${AIOS_LLM_CHAT_MEMORY_MAX:-4G}"
EMBED_MEMORY_MAX="${AIOS_LLM_EMBED_MEMORY_MAX:-1.5G}"
CHAT_SERVICE="aios-llm-chat"
EMBED_SERVICE="aios-llm-embed"

[ -f "$LOCK_FILE" ] || die "không tìm thấy models.lock: $LOCK_FILE"

LLAMA_SERVER="$(command -v llama-server || true)"
[ -n "$LLAMA_SERVER" ] || die "không tìm thấy llama-server trong PATH (cài gói llama-cpp trong VM)"

# Đọc models.lock -> TSV: name, role, url, sha256, size, license, filename
read_lock() {
  python3 - "$LOCK_FILE" <<'PY'
import sys
import tomllib
from urllib.parse import urlparse

REQUIRED = ("name", "role", "url", "sha256", "size", "license")

with open(sys.argv[1], "rb") as fh:
    lock = tomllib.load(fh)

models = lock.get("model", [])
if not models:
    sys.exit("models.lock: không có mô hình nào ([[model]])")

for model in models:
    for key in REQUIRED:
        if key not in model:
            sys.exit(f"models.lock: mô hình {model.get('name', '?')} thiếu trường '{key}'")
    if model["role"] not in ("s1", "s2", "embedding"):
        sys.exit(f"models.lock: role không hợp lệ '{model['role']}'")
    filename = urlparse(model["url"]).path.rsplit("/", 1)[-1]
    if not filename:
        sys.exit(f"models.lock: URL không có tên file: {model['url']}")
    print("\t".join([
        model["name"], model["role"], model["url"],
        model["sha256"].lower(), str(int(model["size"])), model["license"], filename,
    ]))
PY
}

sha256_of() { sha256sum -- "$1" | awk '{print $1}'; }

# Thư mục đích của một mô hình: <root>/<name>/<12 ký tự đầu của sha256>
model_dir() { printf '%s/%s/%s' "$MODEL_ROOT" "$1" "${2:0:12}"; }

# Tải và kiểm tra một mô hình; kết quả đường dẫn nằm trong DOWNLOADED_PATH.
DOWNLOADED_PATH=""
download_model() {
  local name="$1" url="$2" sha="$3" size="$4" filename="$5"
  local dest_dir dest tmp actual actual_size

  dest_dir="$(model_dir "$name" "$sha")"
  dest="$dest_dir/$filename"
  mkdir -p "$dest_dir"

  if [ -f "$dest" ]; then
    actual="$(sha256_of "$dest")"
    if [ "$actual" = "$sha" ]; then
      log "$name: đã có tại $dest (SHA-256 khớp) — không tải lại"
      DOWNLOADED_PATH="$dest"
      return 0
    fi
    log "$name: SHA-256 cũ không khớp ($actual) — tải lại"
    rm -f -- "$dest"
  fi

  tmp="$dest.part"
  rm -f -- "$tmp"
  log "$name: tải $url"
  if ! curl --fail --location --silent --show-error \
       --retry 3 --retry-delay 2 --connect-timeout 30 \
       --output "$tmp" "$url"; then
    rm -f -- "$tmp"
    die "$name: tải thất bại ($url)"
  fi

  actual_size="$(stat -c %s -- "$tmp")"
  actual="$(sha256_of "$tmp")"
  if [ "$actual" != "$sha" ] || [ "$actual_size" != "$size" ]; then
    rm -f -- "$tmp"
    die "$name: kiểm tra file thất bại (size $actual_size/$size, sha256 $actual/$sha) — đã xóa file"
  fi

  mv -f -- "$tmp" "$dest"
  log "$name: đã tải và kiểm SHA-256 OK ($dest)"
  DOWNLOADED_PATH="$dest"
}

render_unit() {
  local template="$1" out="$2" model_file="$3" port="$4" memory_max="$5"
  sed -e "s|@LLAMA_SERVER@|$LLAMA_SERVER|g" \
      -e "s|@MODEL_FILE@|$model_file|g" \
      -e "s|@MODEL_DIR@|$(dirname -- "$model_file")|g" \
      -e "s|@PORT@|$port|g" \
      -e "s|@CHAT_PORT@|$port|g" \
      -e "s|@EMBED_PORT@|$port|g" \
      -e "s|@MEMORY_MAX@|$memory_max|g" \
      "$template" >"$out"
  log "đã ghi unit $out"
}

wait_health() {
  local url="$1" name="$2" tries="${3:-180}" i
  for ((i = 0; i < tries; i++)); do
    if curl --fail --silent --max-time 2 "$url" >/dev/null 2>&1; then
      log "$name: sẵn sàng tại $url"
      return 0
    fi
    sleep 1
  done
  return 1
}

show_diagnostics() {
  local unit="$1"
  systemctl --user --no-pager --full status "$unit" >&2 || true
  journalctl --user -u "$unit" --no-pager -n 40 >&2 || true
}

main() {
  local lock_data name role url sha size license filename
  local chat_model="" chat_file="" embed_model="" embed_file=""

  lock_data="$(read_lock)" || die "đọc models.lock thất bại: $LOCK_FILE"

  while IFS=$'\t' read -r name role url sha size license filename; do
    [ -n "$name" ] || continue
    download_model "$name" "$url" "$sha" "$size" "$filename"
    case "$role" in
      s1|s2)
        [ -z "$chat_file" ] || die "models.lock: có nhiều mô hình chat (role s1/s2), chỉ nhận một"
        chat_model="$name"; chat_file="$DOWNLOADED_PATH"
        ;;
      embedding)
        [ -z "$embed_file" ] || die "models.lock: có nhiều mô hình embedding, chỉ nhận một"
        embed_model="$name"; embed_file="$DOWNLOADED_PATH"
        ;;
    esac
  done <<<"$lock_data"

  [ -n "$chat_file" ] || die "models.lock: không có mô hình chat (role s1 hoặc s2)"
  [ -n "$embed_file" ] || die "models.lock: không có mô hình embedding (role embedding)"

  mkdir -p "$UNIT_DIR"
  render_unit "$SCRIPT_DIR/$CHAT_SERVICE.service.in" "$UNIT_DIR/$CHAT_SERVICE.service" \
    "$chat_file" "$CHAT_PORT" "$CHAT_MEMORY_MAX"
  render_unit "$SCRIPT_DIR/$EMBED_SERVICE.service.in" "$UNIT_DIR/$EMBED_SERVICE.service" \
    "$embed_file" "$EMBED_PORT" "$EMBED_MEMORY_MAX"

  log "bật unit systemd user: $CHAT_SERVICE, $EMBED_SERVICE"
  systemctl --user daemon-reload
  systemctl --user enable "$CHAT_SERVICE.service" "$EMBED_SERVICE.service" ||
    die "không enable được unit systemd user"
  systemctl --user restart "$CHAT_SERVICE.service" || {
    show_diagnostics "$CHAT_SERVICE.service"
    die "$CHAT_SERVICE: không khởi động được"
  }
  systemctl --user restart "$EMBED_SERVICE.service" || {
    show_diagnostics "$EMBED_SERVICE.service"
    die "$EMBED_SERVICE: không khởi động được"
  }

  if ! wait_health "http://127.0.0.1:$CHAT_PORT/health" "$CHAT_SERVICE"; then
    show_diagnostics "$CHAT_SERVICE.service"
    die "$CHAT_SERVICE: không sẵn sàng trong thời hạn"
  fi
  if ! wait_health "http://127.0.0.1:$EMBED_PORT/health" "$EMBED_SERVICE"; then
    show_diagnostics "$EMBED_SERVICE.service"
    die "$EMBED_SERVICE: không sẵn sàng trong thời hạn"
  fi

  log "cài đặt xong:"
  log "  chat      $chat_model -> http://127.0.0.1:$CHAT_PORT/v1 ($chat_file)"
  log "  embedding $embed_model -> http://127.0.0.1:$EMBED_PORT/v1 ($embed_file)"
}

main "$@"
