#!/usr/bin/env bash
# Cổng nghiệm thu M0 trong VM aios-dev (docs/m0/ld-gate.md). Chạy sau dev/vm/gate/setup.sh.
# Dùng: dev/vm-run.sh 'bash dev/vm/gate/setup.sh && bash dev/vm/gate/run.sh'
# In bảng kết quả từng bước; thoát 0 khi bước 1–8 đạt. Số đo S-9 ghi vào $OUT/s9.tsv.
set -uo pipefail
GATE=/var/lib/aios/gate
WS="$GATE/workspaces/$(id -u)"
OUT="${GATE_OUT:-$HOME/gate-out}"
ADMIN_SOCKET=/run/aios/gateway-admin.sock
# shellcheck disable=SC1091
source "$GATE/aiosctl.env"
rm -rf "$OUT"; mkdir -p "$OUT"
declare -a RESULTS=()
FAILED=0
pass() { RESULTS+=("| $1 | đạt | $2 |"); echo "[gate] $1: đạt — $2"; }
fail() { RESULTS+=("| $1 | KHÔNG ĐẠT | $2 |"); echo "[gate] $1: KHÔNG ĐẠT — $2"; FAILED=1; }
jqf() { python3 -c "import json,sys; d=json.load(open(sys.argv[1])); print(eval(sys.argv[2]))" "$@"; }

SERVICES_USER=(aiosd-core aios-model-gateway aios-llm-chat aios-llm-embed)
SERVICES_SYSTEM=("hindsight-api@$(id -u)" postgresql)
# RSS (kB) và MemoryCurrent (byte) của từng dịch vụ, một dòng mỗi dịch vụ.
snapshot() {
  local tag=$1 s pid
  for s in "${SERVICES_USER[@]}"; do
    pid=$(systemctl --user show -p MainPID --value "$s")
    printf '%s\t%s\t%s\t%s\n' "$tag" "$s" "$(awk '/VmRSS/{print $2}' "/proc/$pid/status" 2>/dev/null || echo -)" \
      "$(systemctl --user show -p MemoryCurrent --value "$s")"
  done
  for s in "${SERVICES_SYSTEM[@]}"; do
    pid=$(systemctl show -p MainPID --value "$s")
    printf '%s\t%s\t%s\t%s\n' "$tag" "$s" "$(sudo awk '/VmRSS/{print $2}' "/proc/$pid/status" 2>/dev/null || echo -)" \
      "$(systemctl show -p MemoryCurrent --value "$s")"
  done
}
# Lấy mẫu mỗi giây cho tới khi file $OUT/stop-sampler xuất hiện.
sampler() { while [ ! -e "$OUT/stop-sampler" ]; do snapshot busy >> "$OUT/s9-busy.tsv"; sleep 1; done; }

ask_json() { local name=$1; shift; timeout "${ASK_TIMEOUT:-1200}" aiosctl ask --json "$@" > "$OUT/$name.json" 2> "$OUT/$name.err"; }
audit_lines() { wc -l < "$AIOS_AUDIT_FILE"; }

# --- S-9 lúc rảnh ---------------------------------------------------------
snapshot idle > "$OUT/s9-idle.tsv"

# --- 1. status --------------------------------------------------------------
aiosctl status --json > "$OUT/1.json" 2>&1
if jqf "$OUT/1.json" 'd["core"]["ready"] and d["gateway"]["ready"] and d["core"]["tools"] == 4' | grep -qx True; then
  pass "1 status" "core + gateway sẵn sàng, registry 4 tool, models $(jqf "$OUT/1.json" '",".join(d["gateway"]["models"])')"
else fail "1 status" "$(head -c 300 "$OUT/1.json")"; fi
AUDIT_START=$(audit_lines)

# --- 2. L0 sys.info -----------------------------------------------------------
ask_json 2 "trạng thái máy"
if [ "$(jqf "$OUT/2.json" 'd["route"]+" "+",".join(s["tool"]+":"+s.get("status","") for s in d["steps"])+" "+str(d["tokens"])')" = "L0 sys.info:ok 0" ]; then
  pass "2 L0" "route L0, sys.info ok, 0 token mô hình"
else fail "2 L0" "$(head -c 300 "$OUT/2.json") $(tail -c 300 "$OUT/2.err")"; fi

# --- 3. L2: fs.search rồi fs.write_note (cần xác nhận) -----------------------
rm -f "$WS/notes/tom-tat.md"
rm -f "$OUT/stop-sampler"; sampler & SAMPLER=$!
T0=$(date +%s)
ask_json 3 "tìm file kế hoạch rồi ghi một ghi chú tóm tắt kết quả tên là tom-tat" & ASK=$!
CONFIRMED=no
while kill -0 "$ASK" 2>/dev/null; do
  if aiosctl confirm --list 2>/dev/null | grep -q '^1\.'; then
    aiosctl confirm --list > "$OUT/3-pending.txt"
    # aios-dev-confirm chỉ chạy trên TTY: `script` cấp một pty, "y" là câu trả lời của người dùng.
    echo y | script -qec "aiosctl confirm" /dev/null > "$OUT/3-confirm.txt" 2>&1 && CONFIRMED=yes
    break
  fi
  sleep 2
done
wait "$ASK"; ASK_RC=$?
T3=$(( $(date +%s) - T0 ))
touch "$OUT/stop-sampler"; wait "$SAMPLER" 2>/dev/null
TOOLS3=$(jqf "$OUT/3.json" '",".join(s["tool"]+":"+s.get("status","?") for s in d["steps"])' 2>/dev/null)
UNDO_ID=$(jqf "$OUT/3.json" '[json.loads(s["result"])["undo_id"] for s in d["steps"] if s["tool"]=="fs.write_note" and s.get("status")=="ok"][-1]' 2>/dev/null)
if [ "$ASK_RC" = 0 ] && [ "$CONFIRMED" = yes ] && [ -s "$WS/notes/tom-tat.md" ] && [ -n "$UNDO_ID" ] \
   && [[ "$TOOLS3" == *fs.search:ok* ]] && [ "$(jqf "$OUT/3.json" 'd["route"]')" = L2 ]; then
  pass "3 L2 + xác nhận" "route L2, $TOOLS3; xác nhận bằng bộ ký dev; ghi notes/tom-tat.md; ${T3}s; undo_id $UNDO_ID"
else fail "3 L2 + xác nhận" "rc=$ASK_RC confirmed=$CONFIRMED tools=$TOOLS3 err=$(tail -c 300 "$OUT/3.err")"; fi

# --- 4. nhãn session ----------------------------------------------------------
SESSION=$(jqf "$OUT/3.json" 'd["session"]' 2>/dev/null)
# Hạ nhãn bị bỏ qua và Gateway trả nhãn hiện tại (raise đơn điệu).
curl -s --unix-socket "$ADMIN_SOCKET" -H 'content-type: application/json' -d '{}' \
  "http://gateway/v1/sessions/$SESSION/raise" > "$OUT/4.json"
LABEL=$(jqf "$OUT/4.json" 'd["label"]+"/"+d["taint"]' 2>/dev/null)
NONLOCAL=$(journalctl --user -u aios-model-gateway --since "@$T0" -o cat | grep -c '"kind":"cloud"')
case "$LABEL" in
  personal/tainted|secret/tainted) pass "4 nhãn session" "Gateway: $LABEL; $NONLOCAL lượt cloud (chưa cấu hình cloud)" ;;
  *) fail "4 nhãn session" "Gateway trả '$LABEL' cho session $SESSION ($(head -c 200 "$OUT/4.json"))" ;;
esac

# --- 5. hoàn tác ----------------------------------------------------------------
ask_json 5 "hoàn tác ghi chú $UNDO_ID"
TOOLS5=$(jqf "$OUT/5.json" '",".join(s["tool"]+":"+s.get("status","?") for s in d["steps"])' 2>/dev/null)
if [[ "$TOOLS5" == *fs.undo_note:ok* ]] && [ ! -e "$WS/notes/tom-tat.md" ]; then
  pass "5 hoàn tác" "$TOOLS5; notes/tom-tat.md đã bị xóa (trạng thái trước bước 3)"
else fail "5 hoàn tác" "tools=$TOOLS5 file_còn=$([ -e "$WS/notes/tom-tat.md" ] && echo có || echo không) err=$(tail -c 300 "$OUT/5.err")"; fi

# --- 6. audit verify --------------------------------------------------------------
aiosctl audit verify > "$OUT/6.txt" 2>&1; V=$?
tail -n +"$((AUDIT_START + 1))" "$AIOS_AUDIT_FILE" > "$OUT/6-lines.jsonl"
HAVE=$(python3 - "$OUT/6-lines.jsonl" <<'PY'
import json, sys
tools = set()
kinds = set()
for line in open(sys.argv[1]):
    r = json.loads(line)
    kinds.add(r.get("kind"))
    if r.get("kind") == "tool_call" and r["data"].get("result") == "ok":
        tools.add(r["data"]["tool"])
print(",".join(sorted(tools)), "confirm" in kinds)
PY
)
if [ "$V" = 0 ] && [ "$HAVE" = "fs.search,fs.undo_note,fs.write_note,sys.info True" ]; then
  pass "6 audit verify" "$(tr '\n' ' ' < "$OUT/6.txt"); dòng mới có đủ 4 tool ok + confirm"
else fail "6 audit verify" "rc=$V có=[$HAVE] $(tr '\n' ' ' < "$OUT/6.txt" | head -c 200)"; fi

# --- 7. freeze / unfreeze --------------------------------------------------------
aiosctl freeze > "$OUT/7-freeze.txt" 2>&1
ASK_TIMEOUT=120 ask_json 7a "trạng thái máy"; R7A=$?
STATUS7A=$(jqf "$OUT/7a.json" '",".join(s["tool"]+":"+s.get("status","?")+":"+s.get("error","") for s in d["steps"])' 2>/dev/null)
echo y | script -qec "aiosctl unfreeze" /dev/null > "$OUT/7-unfreeze.txt" 2>&1
ask_json 7b "trạng thái máy"
STATUS7B=$(jqf "$OUT/7b.json" '",".join(s["tool"]+":"+s.get("status","?") for s in d["steps"])' 2>/dev/null)
if [ "$R7A" != 0 ] && [[ "$STATUS7A" != *sys.info:ok* ]] && [ "$STATUS7B" = "sys.info:ok" ]; then
  pass "7 freeze" "khi đóng băng: rc=$R7A [$STATUS7A]; sau unfreeze: $STATUS7B"
else fail "7 freeze" "frozen rc=$R7A [$STATUS7A]; unfrozen [$STATUS7B]"; fi

# --- 8. tắt AiOS, Linux vẫn dùng được ----------------------------------------------
systemctl --user stop "${SERVICES_USER[@]}"
sudo systemctl stop "hindsight-api@$(id -u)"
CHECKS=()
bash -c 'echo ok' >/dev/null && CHECKS+=(shell)
curl -sf -o /dev/null --max-time 20 https://fedoraproject.org/ && CHECKS+=(mạng)
timeout 120 dnf -q repoquery --installed bash >/dev/null 2>&1 && CHECKS+=(dnf)
systemctl --user list-units --no-pager >/dev/null && systemctl is-system-running >/dev/null; CHECKS+=("systemctl:$(systemctl is-system-running)")
S8_START=$(date +%s%N)
timeout 30 aiosctl status > "$OUT/8.txt" 2>&1; R8=$?
S8_MS=$(( ($(date +%s%N) - S8_START) / 1000000 ))
systemctl --user start aios-llm-chat aios-llm-embed aios-model-gateway aiosd-core
sudo systemctl start "hindsight-api@$(id -u)"
if [ "${#CHECKS[@]}" -ge 4 ] && [ "$R8" = 0 ] && grep -q "aiosd-core: unavailable" "$OUT/8.txt" && grep -q "gateway: unavailable" "$OUT/8.txt"; then
  pass "8 fail-safe" "${CHECKS[*]}; aiosctl status trả trong ${S8_MS} ms: không sẵn sàng"
else fail "8 fail-safe" "checks=${CHECKS[*]} status_rc=$R8 $(tr '\n' ' ' < "$OUT/8.txt" | head -c 200)"; fi

# --- 9. S-9 ------------------------------------------------------------------------
python3 - "$OUT" <<'PY' > "$OUT/s9.tsv"
import collections, sys
out = sys.argv[1]
idle = {}
for line in open(f"{out}/s9-idle.tsv"):
    _, s, rss, mem = line.rstrip("\n").split("\t")
    idle[s] = (rss, mem)
peak = collections.defaultdict(lambda: [0, 0])
try:
    for line in open(f"{out}/s9-busy.tsv"):
        _, s, rss, mem = line.rstrip("\n").split("\t")
        p = peak[s]
        p[0] = max(p[0], int(rss) if rss.isdigit() else 0)
        p[1] = max(p[1], int(mem) if mem.isdigit() else 0)
except FileNotFoundError:
    pass
print("service\tidle_rss_mib\tidle_memcur_mib\tpeak_rss_mib_step3\tpeak_memcur_mib_step3")
mib = lambda kb: f"{int(kb) / 1024:.0f}" if str(kb).isdigit() else "-"
mb = lambda b: f"{int(b) / 1048576:.0f}" if str(b).isdigit() else "-"
for s, (rss, mem) in idle.items():
    p = peak[s]
    print(f"{s}\t{mib(rss)}\t{mb(mem)}\t{mib(p[0])}\t{mb(p[1])}")
PY
column -t -s $'\t' "$OUT/s9.tsv"

echo
echo "| Bước | Kết quả | Chi tiết |"
echo "| --- | --- | --- |"
printf '%s\n' "${RESULTS[@]}"
[ "$FAILED" = 0 ] && echo "[gate] CỔNG M0: ĐẠT" || echo "[gate] CỔNG M0: KHÔNG ĐẠT"
exit "$FAILED"
