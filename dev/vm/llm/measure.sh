#!/usr/bin/env bash
# Đo nhanh cho docs/spikes/S-13-m0.md:
#   - RAM hiện tại và đỉnh của hai service systemd user;
#   - độ trễ trung bình của N lần gọi chat (tool calling) và N lần gọi embedding.
#
# Dùng: dev/vm-run.sh 'bash dev/vm/llm/measure.sh'
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
CHAT_URL="${AIOS_LLM_CHAT_URL:-http://127.0.0.1:8081}"
EMBED_URL="${AIOS_LLM_EMBED_URL:-http://127.0.0.1:8082}"
ROUNDS="${AIOS_LLM_MEASURE_ROUNDS:-10}"
PROBE="$SCRIPT_DIR/tool-call-probe.json"

[ -f "$PROBE" ] || { echo "không tìm thấy probe: $PROBE" >&2; exit 1; }

# Thời gian một lần gọi (giây) theo định dạng của curl.
call_time() {
  local url="$1" payload="$2" out
  out="$(curl --silent --show-error --output /dev/null \
              --write-out '%{time_total}' \
              --header 'Content-Type: application/json' \
              --data-binary "$payload" "$url")"
  printf '%s\n' "$out"
}

stats() { # đọc danh sách số trên stdin, in "n avg min max first"
  awk '{v[NR]=$1; s+=$1; if (NR==1 || $1<mn) mn=$1; if (NR==1 || $1>mx) mx=$1}
       END {if (NR==0) {print "0 0 0 0 0"; exit}
            printf "%d %.3f %.3f %.3f %.3f\n", NR, s/NR, mn, mx, v[1]}'
}

echo "== RAM (systemctl --user show -p MemoryCurrent/MemoryPeak) =="
for unit in aios-llm-chat aios-llm-embed; do
  cur="$(systemctl --user show "$unit" -p MemoryCurrent --value)"
  peak="$(systemctl --user show "$unit" -p MemoryPeak --value)"
  printf '%s MemoryCurrent=%s (%.0f MiB) MemoryPeak=%s (%.0f MiB)\n' \
    "$unit" "$cur" "$(awk -v b="$cur" 'BEGIN{print b/1048576}')" \
    "$peak" "$(awk -v b="$peak" 'BEGIN{print b/1048576}')"
done

echo "== Chat: $ROUNDS lần gọi $CHAT_URL/v1/chat/completions =="
chat_times=()
for ((i = 1; i <= ROUNDS; i++)); do
  chat_times+=("$(call_time "$CHAT_URL/v1/chat/completions" "@$PROBE")")
done
printf '%s\n' "${chat_times[@]}" | stats | {
  read -r n avg mn mx first
  printf 'n=%s avg=%ss min=%ss max=%ss first_call=%ss\n' "$n" "$avg" "$mn" "$mx" "$first"
}

echo "== Embedding: $ROUNDS lần gọi $EMBED_URL/v1/embeddings =="
embed_times=()
for ((i = 1; i <= ROUNDS; i++)); do
  embed_times+=("$(call_time "$EMBED_URL/v1/embeddings" '{"input":"xin chào"}')")
done
printf '%s\n' "${embed_times[@]}" | stats | {
  read -r n avg mn mx first
  printf 'n=%s avg=%ss min=%ss max=%ss first_call=%ss\n' "$n" "$avg" "$mn" "$mx" "$first"
}

echo "== RAM sau khi đo =="
for unit in aios-llm-chat aios-llm-embed; do
  cur="$(systemctl --user show "$unit" -p MemoryCurrent --value)"
  printf '%s MemoryCurrent=%s (%.0f MiB)\n' \
    "$unit" "$cur" "$(awk -v b="$cur" 'BEGIN{print b/1048576}')"
done
