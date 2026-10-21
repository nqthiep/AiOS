//! `aios-audit` — append-only audit log with a hash chain.
//!
//! Spec: `docs/m0/audit.md` (design.md §9.3, §11.4, §18.5).
//!
//! Every line of the log is one JSON object terminated by `\n`:
//!
//! ```json
//! {"seq":42,"ts":"2026-10-20T06:30:14.123Z","uid":1000,"kind":"tool_call",
//!  "data":{"tool":"fs.search"},"prev":"<64 hex>","hash":"<64 hex>"}
//! ```
//!
//! The chain is defined by
//! `hash = SHA-256(prev_hash_bytes || JCS(record without the "hash" field))`,
//! where `JCS` is the JSON Canonicalization Scheme (RFC 8785) and `prev` of the
//! first line is 64 zero characters. Every hash is written as 64 lowercase hex
//! characters.

pub mod jcs;
pub mod redact;

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub use jcs::{JcsError, canonicalize};
pub use redact::redact_secret;

/// `prev` of the first line of a chain: 64 zero characters.
pub const GENESIS_PREV: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Length in bytes of a SHA-256 digest.
const HASH_LEN: usize = 32;

/// How many bytes at the end of the log are read to find the last line.
const TAIL_WINDOW: u64 = 64 * 1024;

/// The closed set of auditable events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditKind {
    ToolCall,
    Confirm,
    PluginLoad,
    PluginUnload,
    CapabilityChange,
    SessionCreate,
    Freeze,
    Unfreeze,
}

impl AuditKind {
    /// The name used on the wire.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ToolCall => "tool_call",
            Self::Confirm => "confirm",
            Self::PluginLoad => "plugin_load",
            Self::PluginUnload => "plugin_unload",
            Self::CapabilityChange => "capability_change",
            Self::SessionCreate => "session_create",
            Self::Freeze => "freeze",
            Self::Unfreeze => "unfreeze",
        }
    }
}

impl std::fmt::Display for AuditKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Everything that can go wrong while writing or reading an audit log.
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("i/o error on {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid audit log: {reason}")]
    InvalidLog { reason: String },
    #[error("cannot serialize the audit record: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("cannot canonicalize the audit record: {0}")]
    Jcs(#[from] JcsError),
    #[error("cannot redact '{path}': {reason}")]
    Redaction { path: String, reason: String },
    #[error("the audit log lock is poisoned")]
    Poisoned,
}

impl AuditError {
    fn io(path: &Path, source: std::io::Error) -> Self {
        Self::Io {
            path: path.display().to_string(),
            source,
        }
    }
}

/// Options of an [`AuditLog`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditOptions {
    /// Call `fsync` after every append. Enabled by default.
    pub fsync: bool,
}

impl Default for AuditOptions {
    fn default() -> Self {
        Self { fsync: true }
    }
}

/// One line of the log, in the order required by `docs/m0/audit.md`.
///
/// The `hash` member is left out while the hash of the line is computed.
#[derive(Debug, Serialize)]
struct Record<'a> {
    seq: u64,
    ts: &'a str,
    uid: u32,
    kind: AuditKind,
    data: &'a Value,
    prev: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    hash: Option<&'a str>,
}

/// An append-only audit log.
///
/// The log is safe to share between tasks: every append is serialized by an
/// internal lock and written with a single write on a file opened with
/// `O_APPEND`, so lines never interleave. Only one process may write a given
/// log (the Broker).
pub struct AuditLog {
    path: PathBuf,
    options: AuditOptions,
    state: Mutex<State>,
}

struct State {
    file: File,
    next_seq: u64,
    last_hash: [u8; HASH_LEN],
}

impl std::fmt::Debug for AuditLog {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AuditLog")
            .field("path", &self.path)
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

impl AuditLog {
    /// Open (and create when missing) the audit log at `path`.
    ///
    /// The last line of an existing log is read and checked so that the next
    /// append continues the chain. A file created here gets the mode `0600`.
    pub fn open(path: &Path) -> Result<Self, AuditError> {
        Self::open_with(path, AuditOptions::default())
    }

    /// Open the audit log at `path` with explicit options.
    pub fn open_with(path: &Path, options: AuditOptions) -> Result<Self, AuditError> {
        let existed = path.exists();
        let mut open_options = OpenOptions::new();
        open_options.write(true).append(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            open_options.mode(0o600);
        }
        let file = open_options
            .open(path)
            .map_err(|error| AuditError::io(path, error))?;
        if !existed {
            set_private_permissions(path)?;
        }
        let (next_seq, last_hash) = read_tail_state(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            options,
            state: Mutex::new(State {
                file,
                next_seq,
                last_hash,
            }),
        })
    }

    /// The path of the log file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append one event and return its sequence number.
    ///
    /// The sequence numbers start at 0 and are contiguous. The whole line is
    /// written with a single write and flushed to the disk when
    /// [`AuditOptions::fsync`] is enabled.
    pub fn append(&self, uid: u32, kind: AuditKind, data: Value) -> Result<u64, AuditError> {
        let mut state = self.state.lock().map_err(|_| AuditError::Poisoned)?;
        let seq = state.next_seq;
        let ts = timestamp();
        let prev = hex_encode(&state.last_hash);

        let record = Record {
            seq,
            ts: &ts,
            uid,
            kind,
            data: &data,
            prev: &prev,
            hash: None,
        };
        let payload = serde_json::to_value(&record)?;
        let hash = compute_hash(&prev, &payload)?;
        let hash_hex = hex_encode(&hash);

        let record = Record {
            seq,
            ts: &ts,
            uid,
            kind,
            data: &data,
            prev: &prev,
            hash: Some(&hash_hex),
        };
        let mut line = serde_json::to_string(&record)?;
        line.push('\n');
        state
            .file
            .write_all(line.as_bytes())
            .map_err(|error| AuditError::io(&self.path, error))?;
        if self.options.fsync {
            state
                .file
                .sync_data()
                .map_err(|error| AuditError::io(&self.path, error))?;
        }
        state.next_seq = seq + 1;
        state.last_hash = hash;
        Ok(seq)
    }
}

/// A broken line of an audit log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokenLine {
    /// The sequence number at which the chain broke.
    pub seq: u64,
    /// The 1-based line number of the broken line in the file.
    pub line_no: u64,
    /// Why the line is broken.
    pub reason: String,
}

/// The result of [`verify`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyReport {
    /// The number of lines that were read.
    pub lines: u64,
    /// The first broken line, if the chain is broken.
    pub broken: Option<BrokenLine>,
}

impl VerifyReport {
    /// True when the whole chain is intact.
    pub fn is_ok(&self) -> bool {
        self.broken.is_none()
    }
}

/// Verify the hash chain of the audit log at `path`.
///
/// The report names the first broken line; an unreadable file is an error.
pub fn verify(path: &Path) -> Result<VerifyReport, AuditError> {
    let file = File::open(path).map_err(|error| AuditError::io(path, error))?;
    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();
    let mut expected_seq = 0u64;
    let mut last_hash = [0u8; HASH_LEN];
    let mut lines = 0u64;
    let mut line_no = 0u64;

    loop {
        buffer.clear();
        let read = reader
            .read_until(b'\n', &mut buffer)
            .map_err(|error| AuditError::io(path, error))?;
        if read == 0 {
            break;
        }
        line_no += 1;
        lines += 1;
        let terminated = buffer.last() == Some(&b'\n');
        let line = if terminated {
            &buffer[..read - 1]
        } else {
            &buffer[..read]
        };
        match check_line(line, terminated, expected_seq, &last_hash) {
            Ok(hash) => {
                expected_seq += 1;
                last_hash = hash;
            }
            Err(reason) => {
                return Ok(VerifyReport {
                    lines,
                    broken: Some(BrokenLine {
                        seq: expected_seq,
                        line_no,
                        reason,
                    }),
                });
            }
        }
    }

    Ok(VerifyReport {
        lines,
        broken: None,
    })
}

/// Check one line against the chain and return its hash.
fn check_line(
    line: &[u8],
    terminated: bool,
    expected_seq: u64,
    last_hash: &[u8; HASH_LEN],
) -> Result<[u8; HASH_LEN], String> {
    if !terminated {
        return Err("the line is not terminated by a newline".to_string());
    }
    if line.is_empty() {
        return Err("the line is empty".to_string());
    }
    let mut value: Value =
        serde_json::from_slice(line).map_err(|error| format!("invalid json: {error}"))?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| "the line is not a json object".to_string())?;

    let seq = object
        .get("seq")
        .and_then(Value::as_u64)
        .ok_or_else(|| "the line has no seq".to_string())?;
    if seq != expected_seq {
        return Err(format!("unexpected seq {seq}, expected {expected_seq}"));
    }
    if !object.contains_key("ts") {
        return Err("the line has no timestamp".to_string());
    }
    if !object.get("uid").is_some_and(Value::is_u64) {
        return Err("the line has no uid".to_string());
    }
    let kind = object
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| "the line has no kind".to_string())?;
    serde_json::from_value::<AuditKind>(Value::String(kind.to_string()))
        .map_err(|_| format!("unknown kind '{kind}'"))?;
    if !object.contains_key("data") {
        return Err("the line has no data".to_string());
    }
    let prev = object
        .get("prev")
        .and_then(Value::as_str)
        .ok_or_else(|| "the line has no prev hash".to_string())?
        .to_string();
    let stored = object
        .get("hash")
        .and_then(Value::as_str)
        .ok_or_else(|| "the line has no hash".to_string())?
        .to_string();

    object.remove("hash");
    let hash = compute_hash(&prev, &value).map_err(|error| error.to_string())?;
    if !hash_eq(&hash, &stored) {
        return Err("the hash does not match the content of the line".to_string());
    }
    let prev_bytes = hex_decode(&prev).ok_or_else(|| "the prev hash is not hex".to_string())?;
    if prev_bytes.as_slice() != last_hash.as_slice() {
        return Err("the prev hash does not match the previous line".to_string());
    }
    Ok(hash)
}

/// Read the state of the chain from the end of the log.
///
/// Returns the sequence number of the next line and the hash of the last one.
fn read_tail_state(path: &Path) -> Result<(u64, [u8; HASH_LEN]), AuditError> {
    let len = std::fs::metadata(path)
        .map_err(|error| AuditError::io(path, error))?
        .len();
    if len == 0 {
        return Ok((0, [0u8; HASH_LEN]));
    }

    let mut window = TAIL_WINDOW.min(len);
    let (buffer, at_file_start) = loop {
        let buffer = read_range(path, len - window, window)?;
        if buffer.last() != Some(&b'\n') {
            return Err(AuditError::InvalidLog {
                reason: "the last line is not terminated by a newline".to_string(),
            });
        }
        let newlines = buffer.iter().filter(|byte| **byte == b'\n').count();
        if newlines >= 2 || len - window == 0 {
            break (buffer, len - window == 0);
        }
        window = (window * 2).min(len);
    };

    let (last_line, previous_line) = split_last_lines(&buffer, at_file_start);
    let last_line = last_line.ok_or_else(|| AuditError::InvalidLog {
        reason: "the audit log has no line".to_string(),
    })?;
    let value: Value =
        serde_json::from_slice(last_line).map_err(|error| AuditError::InvalidLog {
            reason: format!("the last line is not valid json: {error}"),
        })?;

    let seq = value
        .get("seq")
        .and_then(Value::as_u64)
        .ok_or_else(|| AuditError::InvalidLog {
            reason: "the last line has no seq".to_string(),
        })?;
    let prev = value
        .get("prev")
        .and_then(Value::as_str)
        .ok_or_else(|| AuditError::InvalidLog {
            reason: "the last line has no prev hash".to_string(),
        })?
        .to_string();
    let stored = value
        .get("hash")
        .and_then(Value::as_str)
        .ok_or_else(|| AuditError::InvalidLog {
            reason: "the last line has no hash".to_string(),
        })?
        .to_string();

    let mut payload = value;
    if let Some(object) = payload.as_object_mut() {
        object.remove("hash");
    }
    let hash = compute_hash(&prev, &payload)?;
    if !hash_eq(&hash, &stored) {
        return Err(AuditError::InvalidLog {
            reason: "the hash of the last line does not match its content".to_string(),
        });
    }

    if let Some(previous_line) = previous_line {
        let previous: Value =
            serde_json::from_slice(previous_line).map_err(|error| AuditError::InvalidLog {
                reason: format!("the previous line is not valid json: {error}"),
            })?;
        let previous_hash = previous
            .get("hash")
            .and_then(Value::as_str)
            .ok_or_else(|| AuditError::InvalidLog {
                reason: "the previous line has no hash".to_string(),
            })?;
        if !previous_hash.eq_ignore_ascii_case(&prev) {
            return Err(AuditError::InvalidLog {
                reason: "the last line does not link to the previous line".to_string(),
            });
        }
    }

    Ok((seq + 1, hash))
}

/// Split a buffer into the last complete line and the one before it.
fn split_last_lines(buffer: &[u8], at_file_start: bool) -> (Option<&[u8]>, Option<&[u8]>) {
    let ends: Vec<usize> = buffer
        .iter()
        .enumerate()
        .filter(|(_, byte)| **byte == b'\n')
        .map(|(index, _)| index)
        .collect();
    let Some(&last_end) = ends.last() else {
        return (None, None);
    };
    let last_start = if ends.len() >= 2 {
        ends[ends.len() - 2] + 1
    } else {
        0
    };
    let last = &buffer[last_start..last_end];

    let previous = if ends.len() >= 2 {
        let previous_end = ends[ends.len() - 2];
        let previous_start = if ends.len() >= 3 {
            ends[ends.len() - 3] + 1
        } else {
            0
        };
        // The first line of the buffer is only complete when the buffer starts
        // at a line boundary.
        if previous_start == 0 && !at_file_start && buffer[0] != b'\n' {
            None
        } else {
            Some(&buffer[previous_start..previous_end])
        }
    } else {
        None
    };

    (Some(last), previous)
}

/// Read `len` bytes of `path` starting at `offset`.
fn read_range(path: &Path, offset: u64, len: u64) -> Result<Vec<u8>, AuditError> {
    let mut file = File::open(path).map_err(|error| AuditError::io(path, error))?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|error| AuditError::io(path, error))?;
    let mut buffer = vec![0u8; len as usize];
    file.read_exact(&mut buffer)
        .map_err(|error| AuditError::io(path, error))?;
    Ok(buffer)
}

/// Compute the hash of a record: `SHA-256(prev_hash_bytes || JCS(record))`.
fn compute_hash(prev_hex: &str, payload: &Value) -> Result<[u8; HASH_LEN], AuditError> {
    let prev = hex_decode(prev_hex).ok_or_else(|| AuditError::InvalidLog {
        reason: format!("'{prev_hex}' is not a hex hash"),
    })?;
    let canonical = canonicalize(payload)?;
    let mut hasher = Sha256::new();
    hasher.update(&prev);
    hasher.update(canonical.as_bytes());
    let digest = hasher.finalize();
    let mut hash = [0u8; HASH_LEN];
    hash.copy_from_slice(&digest);
    Ok(hash)
}

/// The current time, RFC 3339 with millisecond precision.
fn timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Lowercase hex of a digest.
fn hex_encode(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(2 * bytes.len());
    for byte in bytes {
        out.push(DIGITS[usize::from(byte >> 4)] as char);
        out.push(DIGITS[usize::from(byte & 0x0f)] as char);
    }
    out
}

/// Decode a hex string; `None` when the text is not valid hex.
fn hex_decode(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(text.len() / 2);
    for pair in text.as_bytes().chunks(2) {
        out.push((hex_value(pair[0])? << 4) | hex_value(pair[1])?);
    }
    Some(out)
}

/// The value of one hex digit.
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Compare a digest with its hex form.
fn hash_eq(hash: &[u8; HASH_LEN], hex: &str) -> bool {
    match hex_decode(hex) {
        Some(decoded) => decoded.as_slice() == hash.as_slice(),
        None => false,
    }
}

/// Restrict a file to the owner (`0600`).
#[cfg(unix)]
fn set_private_permissions(path: &Path) -> Result<(), AuditError> {
    use std::fs::Permissions;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, Permissions::from_mode(0o600))
        .map_err(|error| AuditError::io(path, error))
}

/// Restrict a file to the owner (`0600`).
#[cfg(not(unix))]
fn set_private_permissions(_path: &Path) -> Result<(), AuditError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn genesis_prev_is_sixty_four_zeros() {
        assert_eq!(GENESIS_PREV.len(), 64);
        assert!(GENESIS_PREV.bytes().all(|byte| byte == b'0'));
    }

    #[test]
    fn hex_round_trip() {
        let bytes = [0x00u8, 0x0f, 0xa5, 0xff];
        assert_eq!(hex_encode(&bytes), "000fa5ff");
        assert_eq!(hex_decode("000fa5ff").unwrap(), bytes);
        assert!(hex_decode("0g").is_none());
        assert!(hex_decode("000").is_none());
    }

    #[test]
    fn hash_depends_on_the_previous_hash() {
        let payload = json!({"seq": 0, "ts": "t", "uid": 1, "kind": "freeze", "data": {}});
        let first = compute_hash(GENESIS_PREV, &payload).unwrap();
        let second = compute_hash(&hex_encode(&first), &payload).unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn timestamp_is_rfc3339_with_milliseconds() {
        let ts = timestamp();
        assert!(ts.ends_with('Z'));
        assert_eq!(ts.len(), 24);
        assert_eq!(&ts[10..11], "T");
        assert_eq!(&ts[19..20], ".");
    }
}
