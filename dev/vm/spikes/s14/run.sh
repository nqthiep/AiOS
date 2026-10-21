#!/usr/bin/env bash
# Spike S-14 (Landlock part): measure the real behaviour of aios-sandbox-exec
# on the running kernel and print the result matrix (docs/m0/s14.md).
#
# Run inside the Linux VM: dev/vm-run.sh 'bash dev/vm/spikes/s14/run.sh'
# Re-runnable: everything lives in a fresh mktemp directory removed on exit.
# Env: S14_RUNS (timing runs, default 50), S14_WORKDIR_BASE (default /var/tmp;
# kept outside /tmp so that the profile paths are not under the /tmp grant).
set -euo pipefail

RUNS="${S14_RUNS:-50}"
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
TARGET="${CARGO_TARGET_DIR:-$REPO/target}"

echo "== build"
(cd "$REPO" && cargo build -q -p aios-sandbox-exec)
HELPER="$TARGET/debug/aios-sandbox-exec"
[ -x "$HELPER" ] || { echo "missing $HELPER" >&2; exit 1; }

D="$(mktemp -d "${S14_WORKDIR_BASE:-/var/tmp}/aios-s14.XXXXXX")"
LISTENER_PID=""
cleanup() {
  [ -n "$LISTENER_PID" ] && kill "$LISTENER_PID" 2>/dev/null || true
  rm -rf "$D"
}
trap cleanup EXIT
mkdir -p "$D/ro" "$D/rw" "$D/sock" "$D/hidden" "$D/py"
echo "ro-data" >"$D/ro/file.txt"
echo "hidden" >"$D/hidden/secret.txt"

# ---------------------------------------------------------------- python code
cat >"$D/py/probe.py" <<'PY'
"""One probe: prints OK or the errno name (prefixed by the failing step)."""
import errno, fcntl, os, signal, socket, sys, termios, time


def step(label, fn):
    try:
        return fn()
    except OSError as e:
        e.s14_step = label
        raise


def run(op, a):
    if op == "read":
        with open(a[0], "rb") as f:
            f.read()
    elif op == "write":
        fd = os.open(a[0], os.O_WRONLY)
        os.write(fd, b"x")
        os.close(fd)
    elif op == "create":
        os.close(os.open(a[0], os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600))
    elif op == "delete":
        os.unlink(a[0])
    elif op == "rename":
        os.rename(a[0], a[1])
    elif op == "truncate":
        os.truncate(a[0], 0)
    elif op == "ioctl":
        if a[0].startswith("fd:"):
            fd = int(a[0][3:])
        else:
            fd = step("open", lambda: os.open(a[0], os.O_RDONLY))
        try:
            fcntl.ioctl(fd, termios.TCGETS, bytes(64))
        except OSError as e:
            if e.errno == errno.ENOTTY:
                return "OK (ioctl ENOTTY from driver)"
            e.s14_step = "ioctl"
            raise
    elif op == "tcp_connect":
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(3)
        s.connect(("127.0.0.1", int(a[0])))
    elif op == "tcp_bind":
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.bind(("127.0.0.1", int(a[0])))
    elif op == "udp_send":
        s = step("socket", lambda: socket.socket(socket.AF_INET, socket.SOCK_DGRAM))
        step("sendto", lambda: s.sendto(b"x", ("127.0.0.1", int(a[0]))))
    elif op == "udp_bind":
        s = step("socket", lambda: socket.socket(socket.AF_INET, socket.SOCK_DGRAM))
        step("bind", lambda: s.bind(("127.0.0.1", int(a[0]))))
    elif op == "abstract_connect":
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        s.connect("\0" + a[0])
    elif op == "abstract_self":
        srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        step("bind", lambda: srv.bind("\0" + a[0]))
        srv.listen(1)
        c = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        step("connect", lambda: c.connect("\0" + a[0]))
    elif op == "unix_connect":
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        s.connect(a[0])
    elif op == "kill":
        os.kill(int(a[0]), 0)
    elif op == "kill_child":
        pid = os.fork()
        if pid == 0:
            time.sleep(5)
            os._exit(0)
        os.kill(pid, signal.SIGTERM)
        os.waitpid(pid, 0)
    else:
        return "BAD_OP"
    return "OK"


try:
    print(run(sys.argv[1], sys.argv[2:]))
except socket.timeout:
    print("TIMEOUT")
except OSError as e:
    name = errno.errorcode.get(e.errno, str(e.errno))
    st = getattr(e, "s14_step", None)
    print(f"{st}:{name}" if st else name)
PY

cat >"$D/py/listeners.py" <<'PY'
"""Host-side listeners: TCP on two ports, abstract + pathname UNIX sockets."""
import socket, sys, time

out, abstract, unix_in, unix_out = sys.argv[1:5]
socks = []


def tcp():
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    s.listen(16)
    socks.append(s)
    return s.getsockname()[1]


def free_port():
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    port = s.getsockname()[1]
    s.close()
    return port


ports = [tcp(), tcp(), free_port(), free_port()]
for name in ("\0" + abstract, unix_in, unix_out):
    u = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    u.bind(name)
    u.listen(16)
    socks.append(u)
with open(out, "w") as f:
    f.write(" ".join(map(str, ports)) + "\n")
while True:
    time.sleep(3600)
PY

cat >"$D/py/landlock.py" <<'PY'
"""Kernel Landlock ABI, accepted access bits, and a Landlock-only UDP check."""
import ctypes, errno, os, socket, sys

libc = ctypes.CDLL(None, use_errno=True)
libc.syscall.restype = ctypes.c_long
CREATE, ADD_RULE, RESTRICT = 444, 445, 446  # same numbers on aarch64 and x86_64
VERSION, ERRATA = 1, 2
RULE_NET_PORT = 2
PR_SET_NO_NEW_PRIVS = 38
BIND_TCP, CONNECT_TCP, BIND_UDP, CONNECT_SEND_UDP = 1, 2, 4, 8


class RulesetAttr(ctypes.Structure):
    _fields_ = [("fs", ctypes.c_uint64), ("net", ctypes.c_uint64), ("scoped", ctypes.c_uint64)]


class NetPortAttr(ctypes.Structure):
    _fields_ = [("allowed", ctypes.c_uint64), ("port", ctypes.c_uint64)]


def create(fs=0, net=0, scoped=0):
    attr = RulesetAttr(fs, net, scoped)
    fd = libc.syscall(CREATE, ctypes.byref(attr), ctypes.c_size_t(ctypes.sizeof(attr)), 0)
    return fd, (ctypes.get_errno() if fd < 0 else 0)


def accepted(field, nbits):
    ok = []
    for bit in range(nbits):
        fd, _ = create(**{field: 1 << bit})
        if fd >= 0:
            os.close(fd)
            ok.append(bit)
    return ok


def udp(op, port):
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        if op == "send":
            s.sendto(b"x", ("127.0.0.1", port))
        else:
            s.bind(("127.0.0.1", port))
        return "OK"
    except OSError as e:
        return errno.errorcode.get(e.errno, str(e.errno))


def restricted(net, rules, checks):
    """Fork, apply a net-only Landlock ruleset, run the UDP checks."""
    r, w = os.pipe()
    pid = os.fork()
    if pid == 0:
        os.close(r)
        fd, err = create(net=net)
        res = []
        if fd < 0:
            res = [f"create:{errno.errorcode.get(err, err)}"]
        else:
            for allowed, port in rules:
                a = NetPortAttr(allowed, port)
                if libc.syscall(ADD_RULE, fd, RULE_NET_PORT, ctypes.byref(a), 0) != 0:
                    res.append(f"add_rule:{errno.errorcode.get(ctypes.get_errno())}")
            libc.prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0)
            if libc.syscall(RESTRICT, fd, 0) != 0:
                res.append(f"restrict:{errno.errorcode.get(ctypes.get_errno())}")
            res += [f"{label}={udp(op, port)}" for label, op, port in checks]
        os.write(w, (" ".join(res)).encode())
        os._exit(0)
    os.close(w)
    data = os.read(r, 4096).decode()
    os.waitpid(pid, 0)
    return data


abi = libc.syscall(CREATE, None, ctypes.c_size_t(0), VERSION)
errata = libc.syscall(CREATE, None, ctypes.c_size_t(0), ERRATA)
print(f"kernel_abi={abi}")
print(f"kernel_errata=0x{errata:x}" if errata >= 0 else f"kernel_errata=err:{ctypes.get_errno()}")
print(f"accepted_fs_bits={accepted('fs', 24)}")
print(f"accepted_net_bits={accepted('net', 8)}")
print(f"accepted_scope_bits={accepted('scoped', 8)}")

p_in, p_out, p_bind_in, p_bind_out = map(int, sys.argv[1:5])
checks = [
    ("send_in", "send", p_in),
    ("send_out", "send", p_out),
    ("bind_in", "bind", p_bind_in),
    ("bind_out", "bind", p_bind_out),
]
print("landlock_only_v6_net(tcp bits only): " + restricted(BIND_TCP | CONNECT_TCP, [], checks))
print(
    "landlock_only_v10_net(tcp+udp bits, allow send_udp to p_in, bind_udp to 0 and p_bind_in): "
    + restricted(
        BIND_TCP | CONNECT_TCP | BIND_UDP | CONNECT_SEND_UDP,
        [(CONNECT_SEND_UDP, p_in), (BIND_UDP, 0), (BIND_UDP, p_bind_in)],
        checks,
    )
)
PY

cat >"$D/py/timing.py" <<'PY'
"""Wall time of N runs of a command; prints median/min/p90/max in ms."""
import statistics, subprocess, sys, time

n, label, cmd = int(sys.argv[1]), sys.argv[2], sys.argv[3:]
samples = []
for _ in range(n):
    t = time.perf_counter_ns()
    rc = subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode
    samples.append((time.perf_counter_ns() - t) / 1e6)
    if rc != 0:
        sys.exit(f"{label}: rc={rc}")
s = sorted(samples)
print(f"{label:<34} median={statistics.median(s):8.2f}  min={s[0]:8.2f}  "
      f"p90={s[int(0.9 * (n - 1))]:8.2f}  max={s[-1]:8.2f}  (ms, n={n})")
PY

# ------------------------------------------------------------- host listeners
ABSTRACT="aios-s14-$$"
python3 -I "$D/py/listeners.py" "$D/ports" "$ABSTRACT" "$D/sock/in.sock" "$D/ro/out.sock" &
LISTENER_PID=$!
for _ in $(seq 50); do [ -s "$D/ports" ] && break; sleep 0.1; done
read -r P_IN P_OUT P_BIND P_FREE <"$D/ports"

# -------------------------------------------------------------------- profiles
# Profile paths are directories: a file or socket path makes the helper exit
# 70 (see the "file/socket entries" section below), so the matrix grants the
# UNIX socket through its directory.
profile() { # <file> <ro json> <rw json> <tcp_connect json> <allow_unix_socket json> <required>
  cat >"$1" <<EOF
{
  "ro_paths": [$2],
  "rw_paths": [$3],
  "tcp_connect": [$4],
  "allow_unix_socket": [$5],
  "memory_max_mb": 512,
  "landlock_required": $6
}
EOF
}
profile "$D/net.json" "\"$D/ro\"" "\"$D/rw\"" "$P_IN, $P_BIND" "\"$D/sock\"" true
profile "$D/nonet.json" "\"$D/ro\"" "\"$D/rw\"" "" "\"$D/sock\"" true
profile "$D/optional.json" "\"$D/ro\"" "\"$D/rw\"" "" "\"$D/sock\"" false

# Same bubblewrap flags as crates/aios-sandbox-exec/src/linux/bwrap.rs, without
# stage 2 (no Landlock, no seccomp): isolates what bubblewrap alone blocks.
bwrap_args() { # <net|nonet>: sets the BW array
  local netflag="--unshare-net"
  [ "$1" = net ] && netflag="--share-net"
  BW=(bwrap --unshare-all "$netflag" --unshare-user --disable-userns --die-with-parent
    --new-session --cap-drop ALL --ro-bind /usr /usr --symlink usr/bin /bin
    --symlink usr/lib /lib --symlink usr/lib64 /lib64 --symlink usr/sbin /sbin
    --proc /proc --dev /dev --tmpfs /tmp)
  [ -e /etc/ssl ] && BW+=(--ro-bind /etc/ssl /etc/ssl)
  BW+=(--ro-bind "$D/ro" "$D/ro" --bind "$D/rw" "$D/rw"
    --bind "$D/sock" "$D/sock" --)
}
bwrap_only() { # <net|nonet> -- cmd...
  bwrap_args "$1"
  shift 2
  "${BW[@]}" "$@"
}

reset_fixtures() {
  rm -rf "$D/rw" && mkdir -p "$D/rw"
  for f in write del ren trunc ioctl; do echo "rw-data" >"$D/rw/$f.txt"; done
}

PROBE="$(cat "$D/py/probe.py")"
probe() { # <variant> <net|nonet> <op> args...
  local variant="$1" net="$2" out rc=0
  shift 2
  reset_fixtures
  if [ "$variant" = bwrap ]; then
    out="$(bwrap_only "$net" -- python3 -I -c "$PROBE" "$@" 2>&1 </dev/null)" || rc=$?
  else
    out="$("$HELPER" --profile "$D/$net.json" -- python3 -I -c "$PROBE" "$@" 2>&1 </dev/null)" || rc=$?
  fi
  out="$(printf '%s' "$out" | tail -n 1)"
  [ "$rc" -eq 0 ] || out="rc=$rc ${out}"
  printf '%s' "$out"
}

row() { # <right> <scope> <target label> <net|nonet> <op> args...
  local right="$1" scope="$2" label="$3" net="$4"
  shift 4
  local b s
  b="$(probe bwrap "$net" "$@")"
  s="$(probe sandbox "$net" "$@")"
  printf '| %-16s | %-5s | %-44s | %-26s | %-26s |\n' "$right" "$scope" "$label" "$b" "$s"
}

# ---------------------------------------------------------------------- report
echo "== versions"
echo "kernel: $(uname -r) ($(uname -m))"
echo "os: $(. /etc/os-release && echo "$PRETTY_NAME")"
echo "lsm: $(cat /sys/kernel/security/lsm 2>/dev/null || echo unknown)"
echo "bwrap: $(bwrap --version)"
echo "python: $(python3 --version)"
echo "rustc: $(rustc --version)"
for c in landlock seccompiler; do
  echo "crate $c: $(grep -A1 "^name = \"$c\"$" "$REPO/Cargo.lock" | sed -n 's/^version = "\(.*\)"/\1/p')"
done
echo "helper requested ABI: $(grep -o 'REQUESTED_ABI: ABI = ABI::V[0-9]*' "$REPO/crates/aios-sandbox-exec/src/linux/landlock.rs")"
LL_SRC="$(ls -d "${CARGO_HOME:-$HOME/.cargo}"/registry/src/*/landlock-"$(grep -A1 '^name = "landlock"$' "$REPO/Cargo.lock" | sed -n 's/^version = "\(.*\)"/\1/p')" 2>/dev/null | head -n 1)"
if [ -n "$LL_SRC" ]; then
  echo "crate landlock max ABI variant: $(grep -o '^    V[0-9]* = [0-9]*' "$LL_SRC/src/compat.rs" | tail -n 1 | tr -s ' ')"
fi

echo
echo "== landlock kernel probe (no bwrap, no seccomp)"
python3 -I "$D/py/landlock.py" "$P_IN" "$P_OUT" "$P_BIND" "$P_FREE"

echo
echo "== matrix (ports: in=$P_IN out=$P_OUT bind_in=$P_BIND bind_out=$P_FREE; host listener pid=$LISTENER_PID)"
echo "columns: bwrap-only = same bwrap flags, no Landlock/seccomp; sandbox = aios-sandbox-exec (landlock_required=true)"
printf '| %-16s | %-5s | %-44s | %-26s | %-26s |\n' "right" "scope" "target" "bwrap-only" "sandbox"
printf '| %-16s | %-5s | %-44s | %-26s | %-26s |\n' "---" "---" "---" "---" "---"
row read in "ro_paths/file.txt" net read "$D/ro/file.txt"
row read out "/proc/self/status (visible, not granted)" net read /proc/self/status
row read out "host dir not in profile" net read "$D/hidden/secret.txt"
row write in "rw_paths/write.txt" net write "$D/rw/write.txt"
row write out "/proc/self/comm (visible, writable)" net write /proc/self/comm
row write out "ro_paths/file.txt" net write "$D/ro/file.txt"
row create in "rw_paths/new.txt" net create "$D/rw/new.txt"
row create out "/dev/s14-new (bwrap tmpfs)" net create /dev/s14-new
row create out "ro_paths/new.txt" net create "$D/ro/new.txt"
row delete in "rw_paths/del.txt" net delete "$D/rw/del.txt"
row delete out "/dev/stdin (bwrap symlink)" net delete /dev/stdin
row delete out "ro_paths/file.txt" net delete "$D/ro/file.txt"
row rename in "rw_paths/ren.txt -> ren2.txt" net rename "$D/rw/ren.txt" "$D/rw/ren2.txt"
row rename out "/dev/stdout -> /dev/stdout2" net rename /dev/stdout /dev/stdout2
row rename out "ro_paths/file.txt -> file2.txt" net rename "$D/ro/file.txt" "$D/ro/file2.txt"
row truncate in "rw_paths/trunc.txt" net truncate "$D/rw/trunc.txt"
row truncate out "/proc/self/comm" net truncate /proc/self/comm
row truncate out "ro_paths/file.txt" net truncate "$D/ro/file.txt"
row ioctl in "TCGETS on rw_paths regular file" net ioctl "$D/rw/ioctl.txt"
row ioctl out "TCGETS on /dev/null (bwrap --dev)" net ioctl /dev/null
row ioctl out "TCGETS on fd 0 = host /dev/null (inherited)" net ioctl fd:0
row tcp_connect in "127.0.0.1:in (tcp_connect)" net tcp_connect "$P_IN"
row tcp_connect out "127.0.0.1:out (not listed)" net tcp_connect "$P_OUT"
row tcp_connect out "127.0.0.1:in, profile tcp_connect=[]" nonet tcp_connect "$P_IN"
row tcp_bind in "127.0.0.1:bind_in (in tcp_connect)" net tcp_bind "$P_BIND"
row tcp_bind out "127.0.0.1:bind_out (not listed)" net tcp_bind "$P_FREE"
row udp_send in "127.0.0.1:in (port in tcp_connect)" net udp_send "$P_IN"
row udp_send out "127.0.0.1:out" net udp_send "$P_OUT"
row udp_bind in "127.0.0.1:bind_in" net udp_bind "$P_BIND"
row udp_bind out "127.0.0.1:bind_out" net udp_bind "$P_FREE"
row abstract_unix in "listener created inside the sandbox" net abstract_self "aios-s14-self-$$"
row abstract_unix out "host listener (--share-net)" net abstract_connect "$ABSTRACT"
row abstract_unix out "host listener (--unshare-net)" nonet abstract_connect "$ABSTRACT"
row unix_path in "allow_unix_socket dir sock/ -> in.sock" net unix_connect "$D/sock/in.sock"
row unix_path out "socket inside ro_paths (not allowed)" net unix_connect "$D/ro/out.sock"
row signal in "fork child inside, SIGTERM" net kill_child
row signal out "pid 1 = bwrap init (outside domain)" net kill 1
row signal out "host pid $LISTENER_PID (outside pid ns)" net kill "$LISTENER_PID"

echo
echo "== file/socket entries in the profile (sandbox only)"
entry_row() { # <label> <ro json> <rw json> <allow_unix_socket json> <required> <op> args...
  local label="$1" rc=0 out
  profile "$D/entry.json" "$2" "$3" "" "$4" "$5"
  shift 5
  out="$("$HELPER" --profile "$D/entry.json" -- python3 -I -c "$PROBE" "$@" 2>&1 </dev/null)" || rc=$?
  printf '| %-46s | rc=%-3s | %s\n' "$label" "$rc" "$(printf '%s' "$out" | tr '\n' ' ')"
}
entry_row "ro_paths=[file], required=true" "\"$D/ro/file.txt\"" "" "" true read "$D/ro/file.txt"
entry_row "ro_paths=[file], required=false" "\"$D/ro/file.txt\"" "" "" false read "$D/ro/file.txt"
entry_row "rw_paths=[file], required=true" "" "\"$D/ro/file.txt\"" "" true read "$D/ro/file.txt"
entry_row "allow_unix_socket=[socket], required=true" "" "" "\"$D/sock/in.sock\"" true unix_connect "$D/sock/in.sock"
entry_row "allow_unix_socket=[socket], required=false" "" "" "\"$D/sock/in.sock\"" false unix_connect "$D/sock/in.sock"
entry_row "rw_paths=[/dev/null], required=false" "" "\"/dev/null\"" "" false ioctl /dev/null
# Same bind without stage 2: does bubblewrap alone allow the device?
bwrap_args nonet
rc=0
out="$("${BW[@]:0:${#BW[@]}-1}" --bind /dev/null /dev/null -- python3 -I -c "$PROBE" ioctl /dev/null 2>&1 </dev/null)" || rc=$?
printf '| %-46s | rc=%-3s | %s\n' "bwrap-only + --bind /dev/null /dev/null" "$rc" "$out"

echo
echo "== landlock_required + forced ABI (AIOS_SANDBOX_TEST_FORCE_ABI), command: true"
force_row() { # <profile> <abi or ->
  local rc=0 err
  if [ "$2" = - ]; then
    err="$("$HELPER" --profile "$D/$1.json" -- true 2>&1)" || rc=$?
  else
    err="$(AIOS_SANDBOX_TEST_FORCE_ABI="$2" "$HELPER" --profile "$D/$1.json" -- true 2>&1)" || rc=$?
  fi
  printf '| %-8s | %-9s | rc=%-3s | %s\n' "$1" "$2" "$rc" "${err:-<no stderr>}"
}
for abi in - 0 1 2 3 4 5 6 7 9 10; do force_row nonet "$abi"; done
for abi in - 5; do force_row optional "$abi"; done

echo
echo "== startup overhead (command: true)"
python3 -I "$D/py/timing.py" "$RUNS" "true (direct)" true
bwrap_args nonet
python3 -I "$D/py/timing.py" "$RUNS" "bwrap-only, --unshare-net" "${BW[@]}" true
python3 -I "$D/py/timing.py" "$RUNS" "aios-sandbox-exec, tcp_connect=[]" "$HELPER" --profile "$D/nonet.json" -- true
python3 -I "$D/py/timing.py" "$RUNS" "aios-sandbox-exec, tcp_connect=[2]" "$HELPER" --profile "$D/net.json" -- true
