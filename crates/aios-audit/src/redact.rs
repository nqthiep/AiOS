//! Secret redaction for audit payloads (docs/m0/audit.md).
//!
//! The audit log must never contain a value labelled `secret`. Callers pass
//! the paths of the secret values they are about to log and this module
//! replaces them with a non-reversible marker of the form
//! `<secret:sha256:<hex>>`, where the digest is the SHA-256 of the secret
//! value (the UTF-8 bytes for strings, the canonical JCS form otherwise).
//!
//! Paths are JSON Pointers (RFC 6901): `/` separates the members, `~1` is a
//! literal `/` and `~0` a literal `~` inside a member name. The empty path
//! addresses the whole document. A member named `*` is a wildcard that
//! matches every element of an array or every member of an object.

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::AuditError;
use crate::jcs;

/// One step of a redaction path.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    /// A member of an object, or an index of an array when it is a number.
    Member(String),
    /// Every element of an array or every member of an object.
    Wildcard,
}

/// Replace every value addressed by `paths` with a `<secret:sha256:...>` marker.
///
/// The document is modified in place. A path that does not address anything is
/// an error: the caller must not silently skip a secret.
pub fn redact_secret(value: &mut Value, paths: &[&str]) -> Result<(), AuditError> {
    for path in paths {
        let segments = parse_pointer(path)?;
        apply(value, &segments).map_err(|reason| AuditError::Redaction {
            path: (*path).to_string(),
            reason,
        })?;
    }
    Ok(())
}

/// Parse a JSON Pointer (RFC 6901) into redaction segments.
fn parse_pointer(path: &str) -> Result<Vec<Segment>, AuditError> {
    if path.is_empty() {
        return Ok(Vec::new());
    }
    if !path.starts_with('/') {
        return Err(AuditError::Redaction {
            path: path.to_string(),
            reason: "a JSON Pointer must be empty or start with '/'".to_string(),
        });
    }
    let mut segments = Vec::new();
    for raw in path[1..].split('/') {
        let name = unescape(raw).map_err(|reason| AuditError::Redaction {
            path: path.to_string(),
            reason,
        })?;
        if name == "*" {
            segments.push(Segment::Wildcard);
        } else {
            segments.push(Segment::Member(name));
        }
    }
    Ok(segments)
}

/// Undo the JSON Pointer escaping of `~1` and `~0`.
fn unescape(raw: &str) -> Result<String, String> {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(character) = chars.next() {
        if character != '~' {
            out.push(character);
            continue;
        }
        match chars.next() {
            Some('0') => out.push('~'),
            Some('1') => out.push('/'),
            Some(other) => {
                return Err(format!("invalid escape '~{other}' in a JSON Pointer"));
            }
            None => return Err("a JSON Pointer must not end with '~'".to_string()),
        }
    }
    Ok(out)
}

/// Apply the remaining path to `node`, replacing the addressed value.
fn apply(node: &mut Value, segments: &[Segment]) -> Result<(), String> {
    let Some((head, rest)) = segments.split_first() else {
        *node = marker(node);
        return Ok(());
    };
    match node {
        Value::Object(members) => match head {
            Segment::Member(name) => {
                let member = members
                    .get_mut(name)
                    .ok_or_else(|| format!("no member '{name}'"))?;
                apply(member, rest)
            }
            Segment::Wildcard => {
                if members.is_empty() {
                    return Err("the wildcard does not match any member".to_string());
                }
                for member in members.values_mut() {
                    apply(member, rest)?;
                }
                Ok(())
            }
        },
        Value::Array(items) => match head {
            Segment::Member(name) => {
                let index = name
                    .parse::<usize>()
                    .map_err(|_| format!("'{name}' is not an array index"))?;
                let item = items
                    .get_mut(index)
                    .ok_or_else(|| format!("index {index} is out of range"))?;
                apply(item, rest)
            }
            Segment::Wildcard => {
                if items.is_empty() {
                    return Err("the wildcard does not match any element".to_string());
                }
                for item in items.iter_mut() {
                    apply(item, rest)?;
                }
                Ok(())
            }
        },
        other => Err(format!("cannot address a member of {}", kind_of(other))),
    }
}

/// Name of the JSON type of `value`, for error messages.
fn kind_of(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

/// Build the `<secret:sha256:...>` marker of `value`.
fn marker(value: &Value) -> Value {
    let digest = match value {
        Value::String(text) => sha256(text.as_bytes()),
        other => {
            let canonical = jcs::canonicalize(other).unwrap_or_else(|_| other.to_string());
            sha256(canonical.as_bytes())
        }
    };
    Value::String(format!("<secret:sha256:{digest}>"))
}

/// Lowercase hex SHA-256 of `bytes`.
fn sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(2 * digest.len());
    for byte in digest.iter() {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn replaces_string_and_object_members() {
        let mut value = json!({
            "tool": "fs.search",
            "args": {"q": "hop dong", "token": "s3cret"},
            "level": 2
        });
        redact_secret(&mut value, &["/args/token"]).unwrap();
        assert_eq!(
            value,
            json!({
                "tool": "fs.search",
                "args": {
                    "q": "hop dong",
                    "token": format!("<secret:sha256:{}>", sha256(b"s3cret"))
                },
                "level": 2
            })
        );
    }

    #[test]
    fn wildcard_matches_every_element() {
        let mut value = json!({"args": [{"token": "a"}, {"token": "b"}]});
        redact_secret(&mut value, &["/args/*/token"]).unwrap();
        assert_eq!(
            value,
            json!({
                "args": [
                    {"token": format!("<secret:sha256:{}>", sha256(b"a"))},
                    {"token": format!("<secret:sha256:{}>", sha256(b"b"))}
                ]
            })
        );
    }

    #[test]
    fn missing_path_is_an_error() {
        let mut value = json!({"args": {}});
        assert!(redact_secret(&mut value, &["/args/token"]).is_err());
    }

    #[test]
    fn addresses_numeric_members_and_array_indexes() {
        let mut value = json!({"args": {"0": "zero", "list": ["a", "b"]}});
        redact_secret(&mut value, &["/args/0", "/args/list/1"]).unwrap();
        assert_eq!(
            value,
            json!({
                "args": {
                    "0": format!("<secret:sha256:{}>", sha256(b"zero")),
                    "list": ["a", format!("<secret:sha256:{}>", sha256(b"b"))]
                }
            })
        );
    }

    #[test]
    fn invalid_pointer_is_an_error() {
        let mut value = json!({});
        assert!(redact_secret(&mut value, &["args/token"]).is_err());
    }
}
