//! aios-sandbox-exec — xem docs/m0/sandbox.md

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("aios-sandbox-exec: linux only");
    std::process::exit(64);
}

#[cfg(target_os = "linux")]
fn main() {
    eprintln!("aios-sandbox-exec: chưa hiện thực (M0, task sandbox)");
    std::process::exit(2);
}
