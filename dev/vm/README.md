# VM aios-dev

Máy ảo Fedora 44 (Lima) để chạy spike và test cần Linux. Cấu hình: `dev/lima/aios-dev.yaml`.

| Việc | Lệnh |
| --- | --- |
| Tạo VM | `limactl start --name=aios-dev dev/lima/aios-dev.yaml` |
| Nâng kernel lên bản hiện hành của F44 (ảnh cloud có 6.19) | `limactl shell aios-dev -- sudo dnf -y upgrade "kernel*"` rồi `limactl stop aios-dev && limactl start aios-dev` |
| Cài công cụ (chạy lại được) | `limactl shell aios-dev -- bash /Users/quangthiep/Projects/AiOS/dev/vm/setup.sh` |
| Chạy lệnh trên bản sao của worktree hiện tại | `dev/vm-run.sh '<lệnh>'` |

User mặc định trong VM là `quangthiep.guest` (home `/home/quangthiep.guest`), có sudo không cần mật khẩu. Thư mục build của `dev/vm-run.sh`: `~/build/<tên worktree>`, target Cargo: `~/build/.target/<tên worktree>`.
