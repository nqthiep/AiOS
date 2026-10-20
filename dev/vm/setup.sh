#!/usr/bin/env bash
# Cài công cụ cho VM aios-dev (Fedora 44). Chạy lại được nhiều lần.
# Dùng: bash dev/vm/setup.sh   (chạy trong VM, user có sudo)
set -euo pipefail

PKGS=(
  gcc clang llvm make pkgconf-pkg-config git rustup
  libbpf-devel bpftool "kernel-devel-$(uname -r)"
  bubblewrap libseccomp-devel
  python3 uv jq
  postgresql-server pgvector
  llama-cpp
  sysstat
)
sudo dnf -y -q install --skip-unavailable "${PKGS[@]}"

# Rust: toolchain theo rust-toolchain.toml của repo (rustup tự đọc khi build)
if ! command -v cargo >/dev/null 2>&1 && [ ! -x "$HOME/.cargo/bin/cargo" ]; then
  rustup-init -y -q --default-toolchain stable --profile minimal -c rustfmt -c clippy
fi

# Thư mục dùng chung của AiOS (M0: chủ sở hữu là user hiện tại)
U="$(id -un)"
sudo install -d -o "$U" -g "$U" -m 0755 \
  /var/lib/aios /var/lib/aios/audit /var/lib/aios/plugins /var/lib/aios/plugins/vendor \
  /var/lib/aios/plugins/local /var/lib/aios/models /var/lib/aios/users \
  /usr/share/aios /usr/share/aios/plugins /usr/share/aios/trust /etc/aios
printf 'd /run/aios 0755 %s %s -\n' "$U" "$U" | sudo tee /etc/tmpfiles.d/aios.conf >/dev/null
sudo systemd-tmpfiles --create /etc/tmpfiles.d/aios.conf

# Slice cho mọi tiến trình agent của người dùng (design §3.4, §18.4)
mkdir -p "$HOME/.config/systemd/user"
cat > "$HOME/.config/systemd/user/aios-agent.slice" <<'UNIT'
[Unit]
Description=AiOS agent processes (tool calls, L2 workers)

[Slice]
MemoryHigh=3G
MemoryMax=4G
CPUWeight=50
UNIT
systemctl --user daemon-reload

echo "setup ok: $(uname -r)"
