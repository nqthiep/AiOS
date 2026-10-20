//! Minimal RFC 8785 (JSON Canonicalization Scheme) serializer.
//!
//! Only the JSON subset used by confirm tokens is supported: objects,
//! arrays, strings, booleans, null, and integers within the IEEE-754 safe
//! range ±2^53. Floats and integers outside the safe range are rejected
//! with [`ConfirmError::BadFormat`]: they cannot be serialized
//! deterministically without full ECMAScript number formatting, which the
//! token format does not use.
//!
//! Key ordering follows RFC 8785 §3.2.3: lexicographic ordering of the
//! UTF-16 code units of the key strings (not code-point or UTF-8 order).
//! String quoting follows ECMAScript `JSON.stringify`: two-character
//! escapes where they exist, `\u00xx` (lowercase hex) for the remaining
//! control characters, and no escaping of non-ASCII characters.
//!
//! Reference implementation (read-only, for cross-checking):
//! `testdata/confirm-vectors/gen.py`.

use std::fmt::Write as _;

use serde_json::Value;

use crate::ConfirmError;

/// 2^53: largest integer exactly representable as an IEEE-754 double.
const SAFE_INTEGER: u64 = 9_007_199_254_740_992;

/// Serialize `value` to its canonical RFC 8785 form.
pub(crate) fn serialize(value: &Value) -> Result<String, ConfirmError> {
    let mut out = String::new();
    write_value(&mut out, value)?;
    Ok(out)
}

fn write_value(out: &mut String, value: &Value) -> Result<(), ConfirmError> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(n) => write_number(out, n)?,
        Value::String(s) => write_string(out, s),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(out, item)?;
            }
            out.push(']');
        }
        Value::Object(map) => {
            // RFC 8785 §3.2.3: sort members by the UTF-16 code units of
            // their keys, which differs from code-point order exactly for
            // characters at and above the surrogate range.
            let mut entries: Vec<(&str, &Value)> =
                map.iter().map(|(k, v)| (k.as_str(), v)).collect();
            entries.sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
            out.push('{');
            for (i, (key, val)) in entries.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(out, key);
                out.push(':');
                write_value(out, val)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

/// Integers within ±2^53 are written in plain decimal; everything else is
/// rejected (see module docs).
fn write_number(out: &mut String, n: &serde_json::Number) -> Result<(), ConfirmError> {
    if let Some(u) = n.as_u64()
        && u <= SAFE_INTEGER
    {
        out.push_str(&u.to_string());
        return Ok(());
    } else if let Some(i) = n.as_i64()
        && i >= -(SAFE_INTEGER as i64)
    {
        out.push_str(&i.to_string());
        return Ok(());
    }
    Err(ConfirmError::BadFormat)
}

/// ECMAScript `JSON.stringify`-compatible quoting. Rust strings are valid
/// UTF-8, so lone surrogates cannot occur and need no special handling.
fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sorts_keys_like_the_reference_generator() {
        // Self-check example from testdata/confirm-vectors/gen.py.
        assert_eq!(
            serialize(&json!({"b": 1, "a": [true, null, "x"]})).unwrap(),
            "{\"a\":[true,null,\"x\"],\"b\":1}"
        );
    }

    #[test]
    fn sorts_keys_by_utf16_code_units() {
        // UTF-16 order: U+20AC (€) < U+D83D.. (surrogate pair) < U+FF61 (｡),
        // while code-point order is € < ｡ < 😀. The middle element only
        // lands there under UTF-16 code-unit ordering (RFC 8785 §3.2.3).
        let v = json!({
            "｡": 1,  // U+FF61
            "😀": 2, // U+1F600
            "€": 3,  // U+20AC
        });
        assert_eq!(serialize(&v).unwrap(), "{\"€\":3,\"😀\":2,\"｡\":1}");
    }

    #[test]
    fn quotes_strings_like_json_stringify() {
        let v = json!("a\u{001f}b\"c\\d\ne\u{000c} g\rh\ti");
        assert_eq!(
            serialize(&v).unwrap(),
            "\"a\\u001fb\\\"c\\\\d\\ne\\f g\\rh\\ti\""
        );
        // Non-ASCII characters are never escaped.
        assert_eq!(
            serialize(&json!("Tiếng Việt ｡ 😀")).unwrap(),
            "\"Tiếng Việt ｡ 😀\""
        );
    }

    #[test]
    fn rejects_floats_and_integers_outside_the_safe_range() {
        assert_eq!(serialize(&json!(1.5)).unwrap_err(), ConfirmError::BadFormat);
        assert_eq!(
            serialize(&json!(u64::MAX)).unwrap_err(),
            ConfirmError::BadFormat
        );
        assert_eq!(
            serialize(&json!(-(SAFE_INTEGER as i64) - 1)).unwrap_err(),
            ConfirmError::BadFormat
        );
        assert_eq!(serialize(&json!(0)).unwrap(), "0");
        assert_eq!(serialize(&json!(SAFE_INTEGER)).unwrap(), "9007199254740992");
        assert_eq!(
            serialize(&json!(-(SAFE_INTEGER as i64))).unwrap(),
            "-9007199254740992"
        );
    }

    #[test]
    fn serializes_nested_structures() {
        let v = json!({"fields": ["ram", "cpu"], "depth": 2, "opts": {"z": true, "a": null}});
        assert_eq!(
            serialize(&v).unwrap(),
            "{\"depth\":2,\"fields\":[\"ram\",\"cpu\"],\"opts\":{\"a\":null,\"z\":true}}"
        );
    }
}
