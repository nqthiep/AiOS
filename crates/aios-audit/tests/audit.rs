//! Tests required by `docs/m0/audit.md`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use aios_audit::{AuditKind, AuditLog, verify};
use serde_json::json;

/// A temporary directory removed when the test ends.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!("aios-audit-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create the temporary directory");
        Self { path }
    }

    fn file(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Write `count` lines of a tool call.
fn write_lines(path: &Path, count: u64) {
    let log = AuditLog::open(path).expect("open the audit log");
    for index in 0..count {
        log.append(1000, AuditKind::ToolCall, json!({ "i": index }))
            .expect("append");
    }
}

/// Read the lines of a log.
fn read_lines(path: &Path) -> Vec<String> {
    let content = fs::read_to_string(path).expect("read the audit log");
    content.lines().map(str::to_string).collect()
}

/// Write the lines back to a log.
fn write_lines_back(path: &Path, lines: &[String]) {
    let mut content = lines.join("\n");
    content.push('\n');
    fs::write(path, content).expect("write the audit log");
}

/// A log of a thousand lines is intact.
#[test]
fn a_thousand_lines_verify_ok() {
    let directory = TempDir::new("thousand");
    let path = directory.file("audit.jsonl");
    write_lines(&path, 1000);

    let report = verify(&path).expect("verify the audit log");
    assert!(report.is_ok(), "{:?}", report.broken);
    assert_eq!(report.lines, 1000);
}

/// A single modified byte breaks the chain at its own sequence number.
#[test]
fn a_modified_byte_is_detected() {
    let directory = TempDir::new("modified");
    let path = directory.file("audit.jsonl");
    write_lines(&path, 100);

    let mut lines = read_lines(&path);
    let middle = 50usize;
    let original = lines[middle].clone();
    let tampered = original.replacen("\"i\":50", "\"i\":51", 1);
    assert_ne!(original, tampered, "the byte must have been modified");
    lines[middle] = tampered;
    write_lines_back(&path, &lines);

    let report = verify(&path).expect("verify the audit log");
    let broken = report.broken.expect("the chain must be broken");
    assert_eq!(broken.seq, middle as u64);
    assert_eq!(broken.line_no, middle as u64 + 1);
}

/// A deleted line breaks the chain at the sequence number of the missing line.
#[test]
fn a_deleted_line_is_detected() {
    let directory = TempDir::new("deleted");
    let path = directory.file("audit.jsonl");
    write_lines(&path, 100);

    let mut lines = read_lines(&path);
    let middle = 50usize;
    lines.remove(middle);
    write_lines_back(&path, &lines);

    let report = verify(&path).expect("verify the audit log");
    let broken = report.broken.expect("the chain must be broken");
    assert_eq!(broken.seq, middle as u64);
}

/// Two swapped lines break the chain.
#[test]
fn two_swapped_lines_are_detected() {
    let directory = TempDir::new("swapped");
    let path = directory.file("audit.jsonl");
    write_lines(&path, 100);

    let mut lines = read_lines(&path);
    lines.swap(50, 51);
    write_lines_back(&path, &lines);

    let report = verify(&path).expect("verify the audit log");
    let broken = report.broken.expect("the chain must be broken");
    assert_eq!(broken.seq, 50);
}

/// Reopening a log continues the chain.
#[test]
fn reopening_continues_the_chain() {
    let directory = TempDir::new("reopen");
    let path = directory.file("audit.jsonl");

    {
        let log = AuditLog::open(&path).expect("open the audit log");
        assert_eq!(
            log.append(1000, AuditKind::SessionCreate, json!({}))
                .expect("append"),
            0
        );
        assert_eq!(
            log.append(1000, AuditKind::ToolCall, json!({ "tool": "fs.search" }))
                .expect("append"),
            1
        );
    }

    let log = AuditLog::open(&path).expect("reopen the audit log");
    assert_eq!(
        log.append(1000, AuditKind::Freeze, json!({}))
            .expect("append"),
        2
    );
    assert_eq!(
        log.append(1000, AuditKind::Unfreeze, json!({}))
            .expect("append"),
        3
    );

    let report = verify(&path).expect("verify the audit log");
    assert!(report.is_ok(), "{:?}", report.broken);
    assert_eq!(report.lines, 4);
}

/// Reopening works whatever the number of lines already in the log.
#[test]
fn reopening_after_any_line_count() {
    for count in 1..=5u64 {
        let directory = TempDir::new(&format!("reopen-{count}"));
        let path = directory.file("audit.jsonl");
        for seq in 0..count {
            let log = AuditLog::open(&path).expect("reopen the audit log");
            assert_eq!(
                log.append(1000, AuditKind::ToolCall, json!({ "i": seq }))
                    .expect("append"),
                seq
            );
        }
        let report = verify(&path).expect("verify the audit log");
        assert!(report.is_ok(), "{count} lines: {:?}", report.broken);
        assert_eq!(report.lines, count);
    }
}

/// Eight tasks writing at the same time never interleave their lines.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn concurrent_appends_do_not_interleave() {
    let directory = TempDir::new("concurrent");
    let path = directory.file("audit.jsonl");
    let log = Arc::new(AuditLog::open(&path).expect("open the audit log"));

    let mut handles = Vec::new();
    for task in 0..8u32 {
        let log = Arc::clone(&log);
        handles.push(tokio::task::spawn_blocking(move || {
            for index in 0..50u64 {
                log.append(
                    1000 + task,
                    AuditKind::ToolCall,
                    json!({ "task": task, "i": index }),
                )
                .expect("append");
            }
        }));
    }
    for handle in handles {
        handle.await.expect("the task must not panic");
    }

    let report = verify(&path).expect("verify the audit log");
    assert!(report.is_ok(), "{:?}", report.broken);
    assert_eq!(report.lines, 400);

    for (index, line) in read_lines(&path).iter().enumerate() {
        let value: serde_json::Value =
            serde_json::from_str(line).expect("every line is a json object");
        assert_eq!(value["seq"].as_u64(), Some(index as u64));
        assert!(value["data"]["task"].is_u64(), "the line is mixed up");
        assert!(value["data"]["i"].is_u64(), "the line is mixed up");
    }
}

/// A log created by the crate is only readable by its owner.
#[test]
fn a_new_log_is_private() {
    let directory = TempDir::new("private");
    let path = directory.file("audit.jsonl");
    let _log = AuditLog::open(&path).expect("open the audit log");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&path).expect("stat").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
