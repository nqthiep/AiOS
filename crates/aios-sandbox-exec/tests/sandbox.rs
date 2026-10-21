#![cfg(target_os = "linux")]

//! Integration tests of `aios-sandbox-exec` (docs/m0/sandbox.md).
//!
//! Each test runs the real binary: stage 1 builds the bubblewrap sandbox,
//! stage 2 applies Landlock + seccomp and runs the child command.

use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

/// Path to the freshly built binary.
const HELPER: &str = env!("CARGO_BIN_EXE_aios-sandbox-exec");

/// Counter giving each test its own temporary directory.
static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Test sandbox: temporary directories, JSON profile, helper runner.
struct Fixture {
    root: PathBuf,
    ro: PathBuf,
    rw: PathBuf,
    profile: PathBuf,
}

impl Fixture {
    /// Creates a fixture with the default profile: no network, Landlock required.
    fn new(name: &str) -> Self {
        // Not under /tmp: the sandbox grants Landlock write on /tmp, which
        // would add to every rule below it (S-14).
        let root = PathBuf::from("/var/tmp").join(format!(
            "aios-sandbox-exec-{}-{name}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("ro")).expect("create the read-only directory");
        std::fs::create_dir_all(root.join("rw")).expect("create the read-write directory");
        let fixture = Fixture {
            ro: root.join("ro"),
            rw: root.join("rw"),
            profile: root.join("profile.json"),
            root,
        };
        fixture.write_profile(&[], true);
        fixture
    }

    /// Rewrites the fixture's JSON profile.
    fn write_profile(&self, tcp_connect: &[u16], landlock_required: bool) {
        let ports: Vec<String> = tcp_connect.iter().map(u16::to_string).collect();
        let json = format!(
            concat!(
                "{{\n",
                "  \"ro_paths\": [{ro}],\n",
                "  \"rw_paths\": [{rw}],\n",
                "  \"tcp_connect\": [{ports}],\n",
                "  \"allow_unix_socket\": [],\n",
                "  \"memory_max_mb\": 512,\n",
                "  \"landlock_required\": {required}\n",
                "}}\n"
            ),
            ro = json_string(&self.ro.to_string_lossy()),
            rw = json_string(&self.rw.to_string_lossy()),
            ports = ports.join(", "),
            required = landlock_required,
        );
        std::fs::write(&self.profile, json).expect("write the profile");
    }

    /// Runs the helper with the given child command.
    fn run(&self, args: &[&str]) -> Output {
        self.run_with_env(&[], args)
    }

    /// Runs the helper with extra environment variables.
    fn run_with_env(&self, env: &[(&str, &str)], args: &[&str]) -> Output {
        let mut command = Command::new(HELPER);
        command.arg("--profile").arg(&self.profile).arg("--");
        command.args(args);
        for (key, value) in env {
            command.env(key, value);
        }
        command.output().expect("run aios-sandbox-exec")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Encodes a string as a JSON string literal.
fn json_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// stderr of a run.
fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

/// A file in `ro_paths` can be read.
#[test]
fn read_in_ro_path_is_allowed() {
    let fixture = Fixture::new("read-ro");
    let file = fixture.ro.join("note.txt");
    std::fs::write(&file, "top-secret").expect("write the file");

    let output = fixture.run(&["sh", "-c", &format!("cat {}", file.display())]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "top-secret");
}

/// Writing into `ro_paths` is denied.
#[test]
fn write_in_ro_path_is_denied() {
    let fixture = Fixture::new("write-ro");
    let file = fixture.ro.join("new.txt");

    let output = fixture.run(&["sh", "-c", &format!("echo hacked > {}", file.display())]);

    assert!(!output.status.success(), "the write must be denied");
    assert!(!file.exists(), "the file must not be created");
}

/// Writing into `rw_paths` is allowed.
#[test]
fn write_in_rw_path_is_allowed() {
    let fixture = Fixture::new("write-rw");
    let file = fixture.rw.join("out.txt");

    let output = fixture.run(&["sh", "-c", &format!("echo hello > {}", file.display())]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(
        std::fs::read_to_string(&file).expect("read the file"),
        "hello\n"
    );
}

/// `$HOME/.ssh` is not in the profile, so it is not visible.
#[test]
fn home_ssh_is_not_visible() {
    let fixture = Fixture::new("home-ssh");
    let home = fixture.root.join("home");
    std::fs::create_dir_all(home.join(".ssh")).expect("create the home directory");
    std::fs::write(home.join(".ssh/id_rsa"), "PRIVATE KEY").expect("write the key");

    let output = fixture.run_with_env(
        &[("HOME", home.to_str().expect("home path"))],
        &["sh", "-c", "cat \"$HOME/.ssh/id_rsa\""],
    );

    assert!(!output.status.success(), "the file must not be readable");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("PRIVATE KEY"));
}

/// Connecting to a port outside `tcp_connect` is denied; the allowed port works.
#[test]
fn tcp_connect_outside_the_profile_is_denied() {
    let fixture = Fixture::new("tcp-deny");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind the listener");
    listener
        .set_nonblocking(true)
        .expect("set the listener non-blocking");
    let port = listener.local_addr().expect("local address").port();
    let curl = |fixture: &Fixture| {
        fixture.run(&[
            "curl",
            "-sS",
            "--max-time",
            "1",
            &format!("http://127.0.0.1:{port}/"),
        ])
    };

    // The profile allows only a port other than the listening one.
    let other_port = if port == 9 { 10 } else { 9 };
    fixture.write_profile(&[other_port], true);
    let output = curl(&fixture);
    assert!(!output.status.success(), "the connection must be denied");
    assert!(
        listener.accept().is_err(),
        "no connection may reach the listener"
    );

    // Control: with the port allowed, the same command reaches the listener
    // (so the denial above is the sandbox, not a missing curl or network).
    fixture.write_profile(&[port], true);
    let _ = curl(&fixture);
    assert!(
        listener.accept().is_ok(),
        "the allowed port must be reachable"
    );
}

/// Creating a UDP socket fails with `EPERM`.
#[test]
fn udp_socket_is_denied() {
    let fixture = Fixture::new("udp-deny");

    let output = fixture.run(&[
        "python3",
        "-c",
        "import socket; socket.socket(socket.AF_INET, socket.SOCK_DGRAM)",
    ]);

    assert!(!output.status.success(), "the socket must be denied");
    assert!(
        stderr(&output).contains("not permitted"),
        "stderr: {}",
        stderr(&output)
    );
}

/// `landlock_required` on an emulated kernel with a too-old ABI exits 70.
#[test]
fn landlock_required_with_old_abi_exits_70() {
    let fixture = Fixture::new("old-abi");
    let marker = fixture.rw.join("ran.txt");

    let output = fixture.run_with_env(
        &[("AIOS_SANDBOX_TEST_FORCE_ABI", "5")],
        &["sh", "-c", &format!("echo ran > {}", marker.display())],
    );

    assert_eq!(
        output.status.code(),
        Some(70),
        "stderr: {}",
        stderr(&output)
    );
    assert!(!marker.exists(), "the command must not run");
}

/// A child exiting 3 makes the helper exit 3.
#[test]
fn child_exit_code_is_preserved() {
    let fixture = Fixture::new("exit-code");

    let output = fixture.run(&["sh", "-c", "exit 3"]);

    assert_eq!(output.status.code(), Some(3), "stderr: {}", stderr(&output));
}

/// seccomp denies the other non-TCP sockets and the listed syscalls with
/// `EPERM`, while plain TCP and UNIX sockets still work.
#[test]
fn seccomp_denies_non_tcp_sockets_and_dangerous_syscalls() {
    let fixture = Fixture::new("seccomp");
    let script = r#"
import ctypes, errno, os, socket
def io_uring_setup():
    libc = ctypes.CDLL(None, use_errno=True)
    params = ctypes.create_string_buffer(120)
    fd = libc.syscall(425, 1, params)  # __NR_io_uring_setup on every arch
    if fd < 0:
        raise OSError(ctypes.get_errno(), "io_uring_setup")
    os.close(fd)
def denied(f):
    try:
        f()
    except OSError as e:
        return e.errno == errno.EPERM
    return False
checks = {
    "raw": lambda: socket.socket(socket.AF_INET, socket.SOCK_RAW, socket.IPPROTO_ICMP),
    "udp6": lambda: socket.socket(socket.AF_INET6, socket.SOCK_DGRAM | socket.SOCK_CLOEXEC),
    "sctp": lambda: socket.socket(socket.AF_INET, socket.SOCK_STREAM, 132),
    "netlink": lambda: socket.socket(socket.AF_NETLINK, socket.SOCK_RAW, 0),
    "unshare": lambda: os.unshare(os.CLONE_NEWUSER),
    "io_uring": io_uring_setup,
}
bad = [name for name, f in checks.items() if not denied(f)]
socket.socket(socket.AF_INET, socket.SOCK_STREAM).close()
socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM).close()
print("not denied:", bad) if bad else print("ok")
"#;

    let output = fixture.run(&["python3", "-c", script]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "ok\n");
}

/// Writes a profile with explicit `ro_paths` and `allow_unix_socket` (Landlock required).
fn write_paths_profile(
    fixture: &Fixture,
    ro_paths: &[&std::path::Path],
    unix: &[&std::path::Path],
) {
    let list = |paths: &[&std::path::Path]| {
        paths
            .iter()
            .map(|p| json_string(&p.to_string_lossy()))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let json = format!(
        "{{\"ro_paths\": [{}], \"rw_paths\": [], \"tcp_connect\": [], \"allow_unix_socket\": [{}], \
         \"memory_max_mb\": 512, \"landlock_required\": true}}",
        list(ro_paths),
        list(unix)
    );
    std::fs::write(&fixture.profile, json).expect("write the profile");
}

/// A single file in `ro_paths` is readable and does not fail closed (S-14 L-1).
#[test]
fn file_in_ro_paths_is_fully_enforced() {
    let fixture = Fixture::new("ro-file");
    let file = fixture.ro.join("one.txt");
    std::fs::write(&file, "one").expect("write the file");
    write_paths_profile(&fixture, &[&file], &[]);

    let output = fixture.run(&["cat", &file.to_string_lossy()]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "one");
}

/// Only sockets in `allow_unix_socket` can be connected (S-14 L-1, L-2).
#[test]
fn only_allowed_unix_sockets_can_be_connected() {
    use std::os::unix::net::UnixListener;

    let fixture = Fixture::new("unix");
    let allowed = fixture.rw.join("allowed.sock");
    let other = fixture.ro.join("other.sock");
    let _allowed = UnixListener::bind(&allowed).expect("bind the allowed socket");
    let _other = UnixListener::bind(&other).expect("bind the other socket");
    write_paths_profile(&fixture, &[&fixture.ro], &[&allowed]);

    let script = "import socket, sys\n\
                  for p in sys.argv[1:]:\n    \
                  s = socket.socket(socket.AF_UNIX)\n    \
                  try:\n        s.connect(p); print('ok')\n    \
                  except OSError as e:\n        print(e.errno)\n";
    let output = fixture.run(&[
        "python3",
        "-c",
        script,
        &allowed.to_string_lossy(),
        &other.to_string_lossy(),
    ]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "ok\n13\n" // 13 = EACCES
    );
}
