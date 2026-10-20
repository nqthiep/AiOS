"""Sinh test vector cho token xác nhận (design.md §9.5, docs/m0/confirm.md). Leader viết (ld-secspec).

Chạy:  uv run --no-project --with cryptography python -I testdata/confirm-vectors/gen.py [--check]
--check: sinh lại và so với vectors.json đã commit (thoát 1 nếu khác).
"""

import base64
import copy
import hashlib
import json
import sys
import unicodedata
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives.serialization import Encoding, PublicFormat

HERE = Path(__file__).resolve().parent
SEED = bytes(range(32))  # khóa cố định, chỉ dùng cho test


def jcs(v) -> str:
    """RFC 8785 cho tập con dùng ở đây: object, array, string, int, bool, null."""
    if v is None:
        return "null"
    if v is True:
        return "true"
    if v is False:
        return "false"
    if isinstance(v, int):
        if abs(v) > 2**53:
            raise ValueError("integer out of safe range")
        return str(v)
    if isinstance(v, str):
        return json.dumps(v, ensure_ascii=False)
    if isinstance(v, list):
        return "[" + ",".join(jcs(x) for x in v) + "]"
    if isinstance(v, dict):
        items = sorted(v.items(), key=lambda kv: kv[0].encode("utf-16-be"))
        return "{" + ",".join(json.dumps(k, ensure_ascii=False) + ":" + jcs(x) for k, x in items) + "}"
    raise TypeError(f"unsupported: {type(v)}")


def nfc(v):
    if isinstance(v, str):
        return unicodedata.normalize("NFC", v)
    if isinstance(v, list):
        return [nfc(x) for x in v]
    if isinstance(v, dict):
        return {nfc(k): nfc(x) for k, x in v.items()}
    return v


def args_hash(args) -> str:
    return hashlib.sha256(jcs(nfc(args)).encode()).hexdigest()


def b64u(b: bytes) -> str:
    return base64.urlsafe_b64encode(b).rstrip(b"=").decode()


KEY = Ed25519PrivateKey.from_private_bytes(SEED)
PUB = KEY.public_key().public_bytes(Encoding.Raw, PublicFormat.Raw)


def sign(claims: dict) -> str:
    body = jcs(claims).encode()
    return b64u(body) + "." + b64u(KEY.sign(body))


PRINCIPAL = "uid:1000/session:01920000-0000-7000-8000-000000000001"
POLICY = "a" * 64
NOW = 1_800_000_000


def make(name, steps, *, dry=None, uid=1000, request="01920000-0000-7000-8000-0000000000aa"):
    """steps: list of (tool, args, risk). Trả (claims, pending) khớp nhau."""
    pending = {
        "request_id": request,
        "uid": uid,
        "principal": PRINCIPAL,
        "task_step": "t-1:0",
        "steps": [{"tool": t, "args": a, "risk": r} for t, a, r in steps],
        "dry_run_hash": dry,
        "policy_version": POLICY,
        "created_unix": NOW,
    }
    claims = {
        "v": 1,
        "request_id": request,
        "principal": PRINCIPAL,
        "uid": uid,
        "task_step": "t-1:0",
        "steps": [{"tool": t, "args_hash": args_hash(a)} for t, a, _ in steps],
        "dry_run_hash": dry,
        "policy_version": POLICY,
        "nonce": b64u(hashlib.sha256(name.encode()).digest()[:16]),
        "exp_unix": NOW + (60 if len(steps) == 1 else 3600),
    }
    return claims, pending


def vec(name, claims, pending, *, kind="single", now=NOW + 5, expect="ok", token=None, consume=None):
    return {
        "name": name,
        "kind": kind,
        "claims": claims,
        "token": token if token is not None else sign(claims),
        "pending": pending,
        "now_unix": now,
        "consume": consume,
        "expect": expect,
    }


def build() -> dict:
    out = []
    one = [("fs.write_note", {"name": "ke-hoach", "text": "Họp lúc 9 giờ"}, "medium")]
    c, p = make("single", one)
    out.append(vec("single-ok", c, p))

    batch = [
        ("fs.search", {"q": "hợp đồng"}, "low"),
        ("fs.write_note", {"name": "tom-tat", "text": "ok"}, "medium"),
        ("fs.undo_note", {"undo_id": "u1"}, "low"),
    ]
    c, p = make("batch", batch)
    out.append(vec("batch-ok", c, p, kind="batch", consume=[{"tool": t, "args": a} for t, a, _ in batch]))

    # args ở dạng NFD phía pending và phía claims (hash phải bằng nhau sau chuẩn hóa NFC)
    nfd = [("fs.write_note", {"name": unicodedata.normalize("NFD", "ghi-chú"), "text": unicodedata.normalize("NFD", "Tiếng Việt")}, "medium")]
    c, p = make("nfd", nfd)
    out.append(vec("nfd-args-ok", c, p))

    nested = [("sys.info", {"fields": ["ram", "cpu"], "depth": 2, "opts": {"z": True, "a": None}}, "low")]
    c, p = make("nested", nested)
    out.append(vec("nested-args-ok", c, p))

    c, p = make("dry", [("pkg.app_install_system", {"app_id": "org.videolan.VLC"}, "high")], dry="b" * 64)
    out.append(vec("high-dry-run-ok", c, p))

    # --- không hợp lệ ---
    c, p = make("badsig", one)
    t = sign(c)
    body, sig = t.split(".")
    bad = bytearray(base64.urlsafe_b64decode(sig + "=="))
    bad[0] ^= 1
    out.append(vec("bad-signature", c, p, token=body + "." + b64u(bytes(bad)), expect="BadSignature"))

    c, p = make("expired", one)
    out.append(vec("expired", c, p, now=NOW + 61, expect="Expired"))

    c, p = make("replay", one)
    out.append(vec("nonce-reused", c, p, kind="replay", expect="NonceReused"))

    c, p = make("argsmis", one)
    p2 = copy.deepcopy(p)
    p2["steps"][0]["args"]["text"] = "Họp lúc 10 giờ"
    out.append(vec("args-mismatch", c, p2, expect="ArgsMismatch"))

    c, p = make("toolmis", one)
    p2 = copy.deepcopy(p)
    p2["steps"][0]["tool"] = "fs.undo_note"
    out.append(vec("tool-mismatch", c, p2, expect="ToolMismatch"))

    c, p = make("princ", one)
    p2 = copy.deepcopy(p)
    p2["principal"] = "uid:1000/session:01920000-0000-7000-8000-000000000002"
    out.append(vec("principal-mismatch", c, p2, expect="PrincipalMismatch"))

    c, p = make("uid", one)
    p2 = copy.deepcopy(p)
    p2["uid"] = 1001
    out.append(vec("uid-mismatch", c, p2, expect="UidMismatch"))

    hb = [("fs.search", {"q": "x"}, "low"), ("pkg.app_install_system", {"app_id": "org.x.Y"}, "high")]
    c, p = make("highbatch", hb)
    out.append(vec("high-in-batch", c, p, kind="batch", expect="HighInBatch"))

    c, p = make("order", batch)
    out.append(vec("step-out-of-order", c, p, kind="batch",
                   consume=[{"tool": batch[1][0], "args": batch[1][1]}], expect="StepOutOfOrder"))

    c, p = make("fmt", one)
    out.append(vec("bad-format", c, p, token="not-a-token", expect="BadFormat"))

    c, p = make("pv", one)
    p2 = copy.deepcopy(p)
    p2["policy_version"] = "c" * 64
    out.append(vec("policy-version-mismatch", c, p2, expect="PolicyVersionMismatch"))

    c, p = make("long", one)
    out.append(vec("too-long", c, p, token="a" * 16385, expect="TooLong"))

    return {
        "comment": "Sinh bởi gen.py; KHÔNG sửa tay. Khóa Ed25519 seed = bytes 0..31.",
        "public_key_b64": base64.b64encode(PUB).decode(),
        "vectors": out,
    }


def main() -> int:
    data = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    target = HERE / "vectors.json"
    if "--check" in sys.argv:
        same = target.exists() and target.read_text(encoding="utf-8") == data
        print("vectors.json khớp" if same else "vectors.json khác bản sinh lại")
        return 0 if same else 1
    target.write_text(data, encoding="utf-8")
    print(f"{len(json.loads(data)['vectors'])} vector → {target}")
    return 0


if __name__ == "__main__":
    # tự kiểm JCS với ví dụ của RFC 8785 về thứ tự khóa
    assert jcs({"b": 1, "a": [True, None, "x"]}) == '{"a":[true,null,"x"],"b":1}'
    assert args_hash({"t": unicodedata.normalize("NFD", "é")}) == args_hash({"t": "é"})
    raise SystemExit(main())
