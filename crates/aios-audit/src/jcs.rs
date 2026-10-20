//! JSON Canonicalization Scheme (RFC 8785), restricted to the JSON subset the
//! audit log needs.
//!
//! Scope of this implementation (docs/m0/audit.md):
//!
//! * object members are sorted by their **UTF-16 code units**,
//! * no whitespace is emitted,
//! * strings are escaped exactly as RFC 8785 §3.2.2.2 requires,
//! * integers are printed in plain decimal notation,
//! * floating point numbers are **rejected** (the audit log never stores any,
//!   and RFC 8785 number serialization is out of scope here).

use std::cmp::Ordering;

use serde_json::{Number, Value};

/// Errors raised while canonicalizing a JSON value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum JcsError {
    /// A JSON number that is not an integer was found.
    #[error("unsupported number: {0} (only integers are supported)")]
    UnsupportedNumber(String),
    /// The value tree contains something that cannot be canonicalized.
    #[error("unsupported value: {0}")]
    UnsupportedValue(String),
}

/// Canonicalize `value` according to RFC 8785.
///
/// The returned string is the UTF-8 serialization of the canonical form and
/// contains no whitespace.
pub fn canonicalize(value: &Value) -> Result<String, JcsError> {
    let mut out = String::new();
    write_value(value, &mut out)?;
    Ok(out)
}

fn write_value(value: &Value, out: &mut String) -> Result<(), JcsError> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(number) => write_number(number, out)?,
        Value::String(string) => write_string(string, out),
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_value(item, out)?;
            }
            out.push(']');
        }
        Value::Object(members) => {
            let mut keys: Vec<&String> = members.keys().collect();
            keys.sort_by(|left, right| compare_utf16(left, right));
            out.push('{');
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_string(key, out);
                out.push(':');
                let member = members
                    .get(*key)
                    .ok_or_else(|| JcsError::UnsupportedValue(format!("missing member {key}")))?;
                write_value(member, out)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

/// Compare two strings by their UTF-16 code units (RFC 8785 §3.2.3).
fn compare_utf16(left: &str, right: &str) -> Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

fn write_number(number: &Number, out: &mut String) -> Result<(), JcsError> {
    if let Some(value) = number.as_i64() {
        out.push_str(&value.to_string());
        return Ok(());
    }
    if let Some(value) = number.as_u64() {
        out.push_str(&value.to_string());
        return Ok(());
    }
    Err(JcsError::UnsupportedNumber(number.to_string()))
}

/// Serialize a string exactly as RFC 8785 §3.2.2.2 requires.
fn write_string(value: &str, out: &mut String) {
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{09}' => out.push_str("\\t"),
            '\u{0a}' => out.push_str("\\n"),
            '\u{0c}' => out.push_str("\\f"),
            '\u{0d}' => out.push_str("\\r"),
            control if (control as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", control as u32));
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sorts_members_by_utf16_code_units() {
        // Test data of RFC 8785 §3.2.3.
        let value = json!({
            "\u{20ac}": "Euro Sign",
            "\r": "Carriage Return",
            "\u{fb33}": "Hebrew Letter Dalet With Dagesh",
            "1": "One",
            "\u{1f600}": "Emoji: Grinning Face",
            "\u{0080}": "Control",
            "\u{00f6}": "Latin Small Letter O With Diaeresis"
        });
        assert_eq!(
            canonicalize(&value).unwrap(),
            concat!(
                "{\"\\r\":\"Carriage Return\",",
                "\"1\":\"One\",",
                "\"\u{0080}\":\"Control\",",
                "\"\u{00f6}\":\"Latin Small Letter O With Diaeresis\",",
                "\"\u{20ac}\":\"Euro Sign\",",
                "\"\u{1f600}\":\"Emoji: Grinning Face\",",
                "\"\u{fb33}\":\"Hebrew Letter Dalet With Dagesh\"}"
            )
        );
    }

    #[test]
    fn keeps_array_order_and_drops_whitespace() {
        let value = json!({"b": [3, 1, 2], "a": {"c": true, "d": null}});
        assert_eq!(
            canonicalize(&value).unwrap(),
            "{\"a\":{\"c\":true,\"d\":null},\"b\":[3,1,2]}"
        );
    }

    #[test]
    fn escapes_strings_like_rfc8785() {
        let value = json!("\u{20ac}$\u{000f}\nA'\u{0042}\u{0022}\u{005c}\\\"");
        assert_eq!(
            canonicalize(&value).unwrap(),
            concat!(
                "\"", // opening quote
                "€",  // euro sign: emitted as UTF-8, never escaped
                "$", "\\u000f", // control character
                "\\n", "A'B", "\\\"", // escaped double quote
                "\\\\", // escaped backslash
                "\\\\", // escaped backslash
                "\\\"", // escaped double quote
                "\""    // closing quote
            )
        );
    }

    #[test]
    fn prints_integers_in_decimal() {
        assert_eq!(canonicalize(&json!(0)).unwrap(), "0");
        assert_eq!(canonicalize(&json!(-42)).unwrap(), "-42");
        assert_eq!(
            canonicalize(&json!(9007199254740991u64)).unwrap(),
            "9007199254740991"
        );
    }

    #[test]
    fn rejects_floating_point_numbers() {
        assert!(matches!(
            canonicalize(&json!(1.5)),
            Err(JcsError::UnsupportedNumber(_))
        ));
        assert!(matches!(
            canonicalize(&json!(1.0)),
            Err(JcsError::UnsupportedNumber(_))
        ));
    }
}
