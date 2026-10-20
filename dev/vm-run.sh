#!/usr/bin/env bash
# Chạy một lệnh trong VM aios-dev trên BẢN SAO của worktree hiện tại.
# Worktree của worker (../wt-<id>) nằm ngoài thư mục chia sẻ vào VM, nên script chép file qua tar.
# Dùng: dev/vm-run.sh '<lệnh>'      ví dụ: dev/vm-run.sh 'cargo test -p aios-sandbox-exec'
set -euo pipefail
[ $# -ge 1 ] || { echo "usage: dev/vm-run.sh '<command>'" >&2; exit 64; }
VM="${AIOS_VM:-aios-dev}"
NAME="$(basename "$PWD")"
DEST="build/$NAME"
git ls-files -co --exclude-standard -z | COPYFILE_DISABLE=1 tar --no-xattrs --no-mac-metadata --null -cf - -T - \
  | limactl shell "$VM" -- bash -c "rm -rf ~/$DEST && mkdir -p ~/$DEST && tar -xf - -C ~/$DEST"
exec limactl shell "$VM" -- bash -lc "cd ~/$DEST && { [ -f ~/.cargo/env ] && . ~/.cargo/env; true; } && export CARGO_TARGET_DIR=~/build/.target/$NAME && $*"
