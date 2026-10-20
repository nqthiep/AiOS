//! aios-confirm — confirmation tokens bound to concrete actions.
//!
//! Spec: docs/m0/confirm.md; design.md §9.5, §7.7. Two sides share one
//! format:
//!
//! * the trusted UI (in M0, the `aios-dev-confirm` binary) turns the
//!   broker's [`PendingConfirm`] into [`ConfirmClaims`] and produces a
//!   token with [`sign`];
//! * the Tool Broker verifies the token against its own pending copy with
//!   [`Verifier::verify_and_consume`] and consumes batch tokens step by
//!   step with [`Verifier::consume_step`].
//!
//! Token format: `base64url(JCS(claims)) + "." + base64url(sig)`, no
//! padding. Args are canonicalized before hashing: every string (object
//! keys included) is Unicode NFC-normalized, the JSON is serialized with
//! the RFC 8785 subset of [`jcs`], then SHA-256-hashed into `args_hash`.

use std::time::{SystemTime, UNIX_EPOCH};

use aios_types::{Principal, Risk};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq as _;
use thiserror::Error;
use uuid::Uuid;

mod jcs;
mod nonce;

pub use nonce::NonceStore;

/// Maximum accepted token length in bytes (16 KiB). Longer tokens are
/// rejected before anything is decoded.
pub const MAX_TOKEN_BYTES: usize = 16 * 1024;
/// Maximum validity of a single-step token, in seconds from creation.
pub const MAX_STEP_SECONDS: u64 = 60;
/// Maximum validity of a batch token, in seconds from creation (M0).
pub const MAX_BATCH_SECONDS: u64 = 3600;

/// Token format version produced by this crate.
const CLAIMS_VERSION: u8 = 1;
/// Nonce length in bytes (carried base64url-encoded in the claims).
const NONCE_BYTES: usize = 16;

/// Failure modes of building, signing, verifying and consuming confirm
/// tokens. The variant names are part of the spec test vectors
/// (`testdata/confirm-vectors/`), so they must not change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ConfirmError {
    /// The token is not `base64url(JCS(claims)) + "." + base64url(sig)`,
    /// or the claims inside are structurally invalid.
    #[error("malformed token or claims")]
    BadFormat,
    /// The Ed25519 signature over the token body does not verify.
    #[error("bad signature")]
    BadSignature,
    /// The token is past its expiry, or claims a validity window longer
    /// than allowed for its step count.
    #[error("token expired")]
    Expired,
    /// The token's nonce was already used by an unexpired token.
    #[error("nonce already used")]
    NonceReused,
    /// The token is for a different pending request (request id, task step
    /// or dry-run hash).
    #[error("request mismatch")]
    RequestMismatch,
    /// The principal string differs from the pending request.
    #[error("principal mismatch")]
    PrincipalMismatch,
    /// The uid differs from the pending request.
    #[error("uid mismatch")]
    UidMismatch,
    /// A step's tool differs from the pending request, or a consumed call
    /// is not any remaining step of the plan.
    #[error("tool mismatch")]
    ToolMismatch,
    /// A step's args hash differs from the pending request.
    #[error("args mismatch")]
    ArgsMismatch,
    /// A batch token was consumed out of order, or is already exhausted.
    #[error("step out of order")]
    StepOutOfOrder,
    /// The pending request has more than one step and one of them is
    /// `high` risk (REQ-D-9.5-03).
    #[error("high-risk step in a batch confirmation")]
    HighInBatch,
    /// The policy version differs from the pending request.
    #[error("policy version mismatch")]
    PolicyVersionMismatch,
    /// The token is longer than [`MAX_TOKEN_BYTES`].
    #[error("token too long")]
    TooLong,
}

/// One step of a confirmed plan: the tool plus the hash of its canonical
/// args. `args_hash` is 64 lowercase hex characters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedStep {
    pub tool: String,
    pub args_hash: String,
}

/// What the trusted UI signs (design.md §9.5). One `steps` element confirms
/// a single step; several confirm a batch (§7.7), which must not contain a
/// `high`-risk step (REQ-D-9.5-03).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmClaims {
    /// Format version; currently 1.
    pub v: u8,
    /// UUIDv7, issued by the broker together with the pending request.
    pub request_id: String,
    /// `uid:<uid>/session:<id>`, taken from the credential, never from a
    /// request body (design.md §18.1).
    pub principal: String,
    pub uid: u32,
    /// `<task_id>:<cursor>` of the first step.
    pub task_step: String,
    pub steps: Vec<SignedStep>,
    /// Hash of the dry-run result shown to the user, if any.
    pub dry_run_hash: Option<String>,
    pub policy_version: String,
    /// 16 random bytes, base64url without padding; usable once.
    pub nonce: String,
    /// Expiry, Unix seconds: at most `created_unix + 60` for one step,
    /// `created_unix + 3600` for a batch (M0).
    pub exp_unix: u64,
}

/// The broker's copy of a confirmation request (design.md §9.5). The
/// dialog must show THIS copy, never one sent back by the client
/// (REQ-D-9.5-01). Serializes like the `pending` field of the spec
/// vectors: `steps` is an array of `{tool, args, risk}` objects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingConfirm {
    pub request_id: String,
    pub uid: u32,
    pub principal: String,
    pub task_step: String,
    #[serde(with = "steps_serde")]
    pub steps: Vec<(String, Value, Risk)>,
    pub dry_run_hash: Option<String>,
    pub policy_version: String,
    pub created_unix: u64,
}

/// Serde adapter for `Vec<(String, Value, Risk)>` as
/// `[{ "tool": .., "args": .., "risk": .. }]`.
mod steps_serde {
    use super::*;

    #[derive(Serialize, Deserialize)]
    struct StepJson {
        tool: String,
        args: Value,
        risk: Risk,
    }

    pub(super) fn serialize<S>(
        steps: &[(String, Value, Risk)],
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let as_json: Vec<StepJson> = steps
            .iter()
            .map(|(tool, args, risk)| StepJson {
                tool: tool.clone(),
                args: args.clone(),
                risk: *risk,
            })
            .collect();
        as_json.serialize(serializer)
    }

    pub(super) fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Vec<(String, Value, Risk)>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let as_json = Vec::<StepJson>::deserialize(deserializer)?;
        Ok(as_json
            .into_iter()
            .map(|StepJson { tool, args, risk }| (tool, args, risk))
            .collect())
    }
}

/// Recursively Unicode NFC-normalize every string in `args`, object keys
/// included (design.md §7.7: normalize before display and hashing).
///
/// If two distinct keys normalize to the same string, the later entry in
/// iteration order wins, mirroring a JSON object rebuild.
pub fn normalize(args: &Value) -> Value {
    use unicode_normalization::UnicodeNormalization as _;
    match args {
        Value::String(s) => Value::String(s.nfc().collect()),
        Value::Array(items) => Value::Array(items.iter().map(normalize).collect()),
        Value::Object(map) => Value::Object({
            let mut out = serde_json::Map::new();
            for (key, val) in map {
                out.insert(key.nfc().collect(), normalize(val));
            }
            out
        }),
        other => other.clone(),
    }
}

/// `SHA-256(JCS(normalize(args)))`, 64 lowercase hex characters.
/// Fails with [`ConfirmError::BadFormat`] if the args contain floats or
/// integers outside ±2^53 (see [`jcs`]).
pub fn args_hash(args: &Value) -> Result<String, ConfirmError> {
    let canonical = jcs::serialize(&normalize(args))?;
    Ok(to_hex(&Sha256::digest(canonical.as_bytes())))
}

/// Sign `claims` into `base64url(JCS(claims)) + "." + base64url(sig)`,
/// base64url without padding. Trusted-UI side.
///
/// Panics only if `claims` cannot be canonicalized (an integer outside
/// ±2^53); the claim types make that a programming error, not a runtime
/// condition.
pub fn sign(claims: &ConfirmClaims, key: &SigningKey) -> String {
    let body = jcs::serialize(&serde_json::to_value(claims).expect("claims serialize to JSON"))
        .expect("claims integers are within the JCS safe range");
    let sig = key.sign(body.as_bytes());
    format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(body.as_bytes()),
        URL_SAFE_NO_PAD.encode(sig.to_bytes())
    )
}

/// Open a confirmation request for `principal` (broker side). The returned
/// [`PendingConfirm`] is the copy the broker must keep and show
/// (REQ-D-9.5-01); its args are NFC-normalized here so that what is hashed,
/// displayed and executed is one canonical copy.
///
/// Fails with [`ConfirmError::HighInBatch`] if `steps` has more than one
/// entry and any of them is `high` risk (REQ-D-9.5-03).
pub fn pending_for(
    principal: &Principal,
    task_step: &str,
    steps: Vec<(String, Value, Risk)>,
    policy_version: &str,
) -> Result<PendingConfirm, ConfirmError> {
    if steps.len() > 1 && steps.iter().any(|(_, _, risk)| *risk == Risk::High) {
        return Err(ConfirmError::HighInBatch);
    }
    let session = principal.session_id.as_deref().unwrap_or("");
    Ok(PendingConfirm {
        request_id: Uuid::now_v7().to_string(),
        uid: principal.uid,
        principal: format!("uid:{}/session:{session}", principal.uid),
        task_step: task_step.to_owned(),
        steps: steps
            .into_iter()
            .map(|(tool, args, risk)| (tool, normalize(&args), risk))
            .collect(),
        dry_run_hash: None,
        policy_version: policy_version.to_owned(),
        created_unix: unix_now(),
    })
}

fn unix_now() -> u64 {
    // A clock before the epoch is not a real condition; treat it as 0
    // rather than panicking inside a library.
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// Decode 64 lowercase hex chars into 32 bytes; `None` if malformed.
fn hash_from_hex(s: &str) -> Option<[u8; 32]> {
    let b = s.as_bytes();
    if b.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = hex_val(b[2 * i])? << 4 | hex_val(b[2 * i + 1])?;
    }
    Some(out)
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    }
}

/// Constant-time comparison of two 64-lowercase-hex hashes
/// (docs/m0/confirm.md: hash comparisons use `subtle`).
fn hashes_equal(a_hex: &str, b_hex: &str) -> bool {
    match (hash_from_hex(a_hex), hash_from_hex(b_hex)) {
        (Some(a), Some(b)) => bool::from(a.ct_eq(&b)),
        (None, _) | (_, None) => false,
    }
}

/// Token parts after base64url decoding: raw body bytes, raw signature
/// and the parsed claims. The signature covers `body` exactly as
/// received, so canonicality is enforced by the signer, not re-checked
/// here.
struct ParsedToken {
    body: Vec<u8>,
    sig: [u8; 64],
    claims: ConfirmClaims,
}

fn parse_token(token: &str) -> Result<ParsedToken, ConfirmError> {
    let mut parts = token.split('.');
    let body_b64 = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or(ConfirmError::BadFormat)?;
    let sig_b64 = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or(ConfirmError::BadFormat)?;
    if parts.next().is_some() {
        return Err(ConfirmError::BadFormat);
    }
    let body = URL_SAFE_NO_PAD
        .decode(body_b64)
        .map_err(|_| ConfirmError::BadFormat)?;
    let sig_bytes = URL_SAFE_NO_PAD
        .decode(sig_b64)
        .map_err(|_| ConfirmError::BadFormat)?;
    let sig: [u8; 64] = sig_bytes.try_into().map_err(|_| ConfirmError::BadFormat)?;
    let claims: ConfirmClaims =
        serde_json::from_slice(&body).map_err(|_| ConfirmError::BadFormat)?;
    if claims.v != CLAIMS_VERSION {
        return Err(ConfirmError::BadFormat);
    }
    let nonce_raw = URL_SAFE_NO_PAD
        .decode(&claims.nonce)
        .map_err(|_| ConfirmError::BadFormat)?;
    if nonce_raw.len() != NONCE_BYTES {
        return Err(ConfirmError::BadFormat);
    }
    for step in &claims.steps {
        if hash_from_hex(&step.args_hash).is_none() {
            return Err(ConfirmError::BadFormat);
        }
    }
    Ok(ParsedToken { body, sig, claims })
}

/// A token that passed [`Verifier::verify_and_consume`]. Batch tokens are
/// consumed step by step via [`Verifier::consume_step`].
#[derive(Debug, Clone)]
pub struct VerifiedToken {
    claims: ConfirmClaims,
    next_step: usize,
}

impl VerifiedToken {
    /// The verified claims.
    pub fn claims(&self) -> &ConfirmClaims {
        &self.claims
    }

    /// 0-based index of the next step to consume.
    pub fn next_step(&self) -> usize {
        self.next_step
    }

    /// How many steps of the plan are still unconsumed.
    pub fn steps_remaining(&self) -> usize {
        self.claims.steps.len() - self.next_step
    }
}

/// Broker-side verifier: checks a token against the broker's pending copy
/// and consumes it. Holds the trusted UI's public key and a
/// [`NonceStore`].
pub struct Verifier {
    key: VerifyingKey,
    nonces: NonceStore,
}

impl Verifier {
    pub fn new(key: VerifyingKey) -> Self {
        Self {
            key,
            nonces: NonceStore::new(),
        }
    }

    /// Verify `token` against the broker's `pending` copy at `now_unix`
    /// and, on success, consume its nonce (first use only).
    ///
    /// Check order (fixed by docs/m0/confirm.md): length → format →
    /// signature → high-in-batch → request identity (request id, task
    /// step, principal, uid, policy version, dry-run hash) → expiry →
    /// per-step tool and args hash → nonce.
    pub fn verify_and_consume(
        &self,
        token: &str,
        pending: &PendingConfirm,
        now_unix: u64,
    ) -> Result<VerifiedToken, ConfirmError> {
        if token.len() > MAX_TOKEN_BYTES {
            return Err(ConfirmError::TooLong);
        }
        let parsed = parse_token(token)?;
        self.key
            .verify_strict(&parsed.body, &Signature::from_bytes(&parsed.sig))
            .map_err(|_| ConfirmError::BadSignature)?;

        if pending.steps.len() > 1 && pending.steps.iter().any(|(_, _, r)| *r == Risk::High) {
            return Err(ConfirmError::HighInBatch);
        }

        // Request identity: everything the pending request and the claims
        // must agree on before the action itself is compared.
        if parsed.claims.request_id != pending.request_id {
            return Err(ConfirmError::RequestMismatch);
        }
        if parsed.claims.task_step != pending.task_step {
            return Err(ConfirmError::RequestMismatch);
        }
        if parsed.claims.principal != pending.principal {
            return Err(ConfirmError::PrincipalMismatch);
        }
        if parsed.claims.uid != pending.uid {
            return Err(ConfirmError::UidMismatch);
        }
        if parsed.claims.policy_version != pending.policy_version {
            return Err(ConfirmError::PolicyVersionMismatch);
        }
        if parsed.claims.dry_run_hash != pending.dry_run_hash {
            return Err(ConfirmError::RequestMismatch);
        }

        // Expiry: the token may not outlive its allowed window, and the
        // current time may not be past its expiry.
        let window = if pending.steps.len() <= 1 {
            MAX_STEP_SECONDS
        } else {
            MAX_BATCH_SECONDS
        };
        if parsed.claims.exp_unix > pending.created_unix.saturating_add(window) {
            return Err(ConfirmError::Expired);
        }
        if now_unix > parsed.claims.exp_unix {
            return Err(ConfirmError::Expired);
        }

        // Per-step comparison against the broker's own args.
        if parsed.claims.steps.len() != pending.steps.len() {
            return Err(ConfirmError::ArgsMismatch);
        }
        for (signed, pending_step) in parsed.claims.steps.iter().zip(&pending.steps) {
            if signed.tool != pending_step.0 {
                return Err(ConfirmError::ToolMismatch);
            }
            let want = args_hash(&pending_step.1)?;
            if !hashes_equal(&signed.args_hash, &want) {
                return Err(ConfirmError::ArgsMismatch);
            }
        }

        // Last check, only on full success: burn the nonce.
        self.nonces
            .check_and_consume(&parsed.claims.nonce, parsed.claims.exp_unix, now_unix)?;

        Ok(VerifiedToken {
            claims: parsed.claims,
            next_step: 0,
        })
    }

    /// Consume the next step of a (batch) token: `tool` and `args` must
    /// match the next element of the signed plan, canonicalized exactly
    /// like [`args_hash`] does (design.md §9.5: batch tokens are used
    /// sequentially; a wrong step invalidates the whole token).
    ///
    /// * right position, different content → [`ConfirmError::ToolMismatch`]
    ///   / [`ConfirmError::ArgsMismatch`]
    /// * a later step of the plan presented now →
    ///   [`ConfirmError::StepOutOfOrder`]
    /// * a call that is not any remaining step →
    ///   [`ConfirmError::ToolMismatch`]
    /// * the plan is already exhausted →
    ///   [`ConfirmError::StepOutOfOrder`]
    pub fn consume_step(
        &self,
        vt: &mut VerifiedToken,
        tool: &str,
        args: &Value,
    ) -> Result<(), ConfirmError> {
        let want_hash = args_hash(args)?;
        let Some(next) = vt.claims.steps.get(vt.next_step) else {
            return Err(ConfirmError::StepOutOfOrder);
        };
        if next.tool == tool {
            if hashes_equal(&next.args_hash, &want_hash) {
                vt.next_step += 1;
                return Ok(());
            }
            return Err(ConfirmError::ArgsMismatch);
        }
        // Wrong tool at the next position: a complete later step of the
        // plan is out of order; anything else is not part of the plan.
        let is_later_step = vt.claims.steps[vt.next_step + 1..]
            .iter()
            .any(|s| s.tool == tool && hashes_equal(&s.args_hash, &want_hash));
        if is_later_step {
            Err(ConfirmError::StepOutOfOrder)
        } else {
            Err(ConfirmError::ToolMismatch)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aios_types::Channel;
    use serde_json::json;
    use unicode_normalization::UnicodeNormalization as _;

    /// Nonce-shaped strings (base64url of 16 bytes, varied to keep tests
    /// independent). Canonical no-pad encoding requires the final
    /// character's low 4 bits to be zero, hence the trailing 'A'.
    fn nonce(c: char) -> String {
        let mut s: String = std::iter::repeat_n(c, 21).collect();
        s.push('A');
        s
    }

    fn dev_key() -> SigningKey {
        SigningKey::from_bytes(&[0x5a; 32])
    }

    fn pending_one(args: Value) -> PendingConfirm {
        PendingConfirm {
            request_id: "01920000-0000-7000-8000-0000000000f1".into(),
            uid: 1000,
            principal: "uid:1000/session:01920000-0000-7000-8000-000000000001".into(),
            task_step: "t-1:0".into(),
            steps: vec![("fs.write_note".into(), args, Risk::Medium)],
            dry_run_hash: None,
            policy_version: "a".repeat(64),
            created_unix: 1_800_000_000,
        }
    }

    fn pending_batch() -> PendingConfirm {
        PendingConfirm {
            request_id: "01920000-0000-7000-8000-0000000000f2".into(),
            uid: 1000,
            principal: "uid:1000/session:01920000-0000-7000-8000-000000000001".into(),
            task_step: "t-1:0".into(),
            steps: vec![
                ("fs.search".into(), json!({"q": "hợp đồng"}), Risk::Low),
                (
                    "fs.write_note".into(),
                    json!({"name": "tom-tat", "text": "ok"}),
                    Risk::Medium,
                ),
            ],
            dry_run_hash: None,
            policy_version: "a".repeat(64),
            created_unix: 1_800_000_000,
        }
    }

    fn claims_for(p: &PendingConfirm, nonce: &str, exp_unix: u64) -> ConfirmClaims {
        let steps = p
            .steps
            .iter()
            .map(|(t, a, _)| SignedStep {
                tool: t.clone(),
                args_hash: args_hash(a).unwrap(),
            })
            .collect();
        ConfirmClaims {
            v: 1,
            request_id: p.request_id.clone(),
            principal: p.principal.clone(),
            uid: p.uid,
            task_step: p.task_step.clone(),
            steps,
            dry_run_hash: p.dry_run_hash.clone(),
            policy_version: p.policy_version.clone(),
            nonce: nonce.into(),
            exp_unix,
        }
    }

    /// Unit test required by docs/m0/confirm.md: a valid token used a
    /// second time fails with `NonceReused`.
    #[test]
    fn valid_token_second_use_is_nonce_reused() {
        let p = pending_one(json!({"name": "ke-hoach", "text": "Họp lúc 9 giờ"}));
        let exp = p.created_unix + MAX_STEP_SECONDS;
        let claims = claims_for(&p, &nonce('A'), exp);
        let token = sign(&claims, &dev_key());
        // Ed25519 signing is deterministic: same claims, same token.
        assert_eq!(token, sign(&claims, &dev_key()));

        let verifier = Verifier::new(dev_key().verifying_key());
        let now = p.created_unix + 5;
        assert!(verifier.verify_and_consume(&token, &p, now).is_ok());
        assert_eq!(
            verifier.verify_and_consume(&token, &p, now).unwrap_err(),
            ConfirmError::NonceReused
        );
    }

    /// Unit test required by docs/m0/confirm.md: changing one character of
    /// the args after signing fails with `ArgsMismatch`.
    #[test]
    fn one_changed_args_char_after_signing_is_args_mismatch() {
        let signed = pending_one(json!({"name": "ke-hoach", "text": "Họp lúc 9 giờ"}));
        let claims = claims_for(&signed, &nonce('B'), signed.created_unix + MAX_STEP_SECONDS);
        let token = sign(&claims, &dev_key());

        let mut other = signed.clone();
        other.steps[0].1["text"] = json!("Họp lúc 10 giờ");
        assert_eq!(
            Verifier::new(dev_key().verifying_key())
                .verify_and_consume(&token, &other, signed.created_unix + 5)
                .unwrap_err(),
            ConfirmError::ArgsMismatch
        );
    }

    /// Unit test required by docs/m0/confirm.md: NFD and NFC spellings of
    /// the same args produce the same `args_hash`.
    #[test]
    fn nfd_and_nfc_args_hash_equal() {
        let nfd = "Tiếng Việt".nfd().collect::<String>();
        assert_ne!(nfd.as_str(), "Tiếng Việt"); // the NFD form really differs
        let a = args_hash(&json!({"name": "ghi-chú", "text": nfd})).unwrap();
        let b = args_hash(&json!({"name": "ghi-chú", "text": "Tiếng Việt"})).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn token_valid_up_to_and_including_exp_unix() {
        let p = pending_one(json!({"a": 1}));
        let exp = p.created_unix + MAX_STEP_SECONDS;
        let claims = claims_for(&p, &nonce('C'), exp);
        let token = sign(&claims, &dev_key());
        let verifier = Verifier::new(dev_key().verifying_key());
        assert!(verifier.verify_and_consume(&token, &p, exp).is_ok());
        // Expired is checked before the nonce, so the replay reads as
        // Expired, not as NonceReused.
        assert_eq!(
            verifier
                .verify_and_consume(&token, &p, exp + 1)
                .unwrap_err(),
            ConfirmError::Expired
        );
    }

    #[test]
    fn validity_window_cannot_be_stretched() {
        let single = pending_one(json!({"a": 1}));
        let too_late = claims_for(
            &single,
            &nonce('D'),
            single.created_unix + MAX_STEP_SECONDS + 1,
        );
        let token = sign(&too_late, &dev_key());
        assert_eq!(
            Verifier::new(dev_key().verifying_key())
                .verify_and_consume(&token, &single, single.created_unix)
                .unwrap_err(),
            ConfirmError::Expired
        );

        let batch = pending_batch();
        let max_window = claims_for(&batch, &nonce('E'), batch.created_unix + MAX_BATCH_SECONDS);
        let token = sign(&max_window, &dev_key());
        assert!(
            Verifier::new(dev_key().verifying_key())
                .verify_and_consume(&token, &batch, batch.created_unix + 5)
                .is_ok()
        );
        let stretched = claims_for(
            &batch,
            &nonce('F'),
            batch.created_unix + MAX_BATCH_SECONDS + 1,
        );
        let token = sign(&stretched, &dev_key());
        assert_eq!(
            Verifier::new(dev_key().verifying_key())
                .verify_and_consume(&token, &batch, batch.created_unix + 5)
                .unwrap_err(),
            ConfirmError::Expired
        );
    }

    #[test]
    fn batch_consumes_steps_in_order_only() {
        let p = pending_batch();
        let claims = claims_for(&p, &nonce('G'), p.created_unix + MAX_BATCH_SECONDS);
        let token = sign(&claims, &dev_key());
        let verifier = Verifier::new(dev_key().verifying_key());
        let mut vt = verifier
            .verify_and_consume(&token, &p, p.created_unix + 5)
            .unwrap();
        assert_eq!(vt.next_step(), 0);

        // Presenting step 2 first is out of order (it IS a step of the plan).
        assert_eq!(
            verifier
                .consume_step(
                    &mut vt,
                    "fs.write_note",
                    &json!({"name": "tom-tat", "text": "ok"})
                )
                .unwrap_err(),
            ConfirmError::StepOutOfOrder
        );

        // In order works.
        verifier
            .consume_step(&mut vt, "fs.search", &json!({"q": "hợp đồng"}))
            .unwrap();
        verifier
            .consume_step(
                &mut vt,
                "fs.write_note",
                &json!({"name": "tom-tat", "text": "ok"}),
            )
            .unwrap();
        assert_eq!(vt.steps_remaining(), 0);

        // The plan is exhausted.
        assert_eq!(
            verifier
                .consume_step(&mut vt, "fs.search", &json!({"q": "hợp đồng"}))
                .unwrap_err(),
            ConfirmError::StepOutOfOrder
        );
    }

    #[test]
    fn consume_step_content_mismatches() {
        let p = pending_batch();
        let claims = claims_for(&p, &nonce('H'), p.created_unix + MAX_BATCH_SECONDS);
        let token = sign(&claims, &dev_key());
        let verifier = Verifier::new(dev_key().verifying_key());
        let mut vt = verifier
            .verify_and_consume(&token, &p, p.created_unix + 5)
            .unwrap();

        // Right tool, wrong args.
        assert_eq!(
            verifier
                .consume_step(&mut vt, "fs.search", &json!({"q": "khac"}))
                .unwrap_err(),
            ConfirmError::ArgsMismatch
        );
        // A tool that is not any remaining step of the plan.
        assert_eq!(
            verifier
                .consume_step(&mut vt, "fs.undo_note", &json!({"undo_id": "u1"}))
                .unwrap_err(),
            ConfirmError::ToolMismatch
        );
        // State is unchanged after failures.
        assert_eq!(vt.next_step(), 0);
        verifier
            .consume_step(&mut vt, "fs.search", &json!({"q": "hợp đồng"}))
            .unwrap();
    }

    #[test]
    fn high_in_batch_rejected_on_both_sides() {
        let principal = Principal {
            uid: 1000,
            is_admin: false,
            channel: Channel::Local,
            session_id: Some("01920000-0000-7000-8000-000000000001".into()),
        };
        let steps = vec![
            ("fs.search".into(), json!({"q": "x"}), Risk::Low),
            (
                "pkg.app_install_system".into(),
                json!({"app_id": "org.x.Y"}),
                Risk::High,
            ),
        ];
        assert_eq!(
            pending_for(&principal, "t-1:0", steps.clone(), "pv").unwrap_err(),
            ConfirmError::HighInBatch
        );

        // Even a perfectly signed batch token is rejected against a
        // pending copy that contains a high step.
        let mut p = pending_batch();
        p.steps[1] = (
            "pkg.app_install_system".into(),
            json!({"app_id": "org.x.Y"}),
            Risk::High,
        );
        let claims = claims_for(&p, &nonce('I'), p.created_unix + MAX_BATCH_SECONDS);
        let token = sign(&claims, &dev_key());
        assert_eq!(
            Verifier::new(dev_key().verifying_key())
                .verify_and_consume(&token, &p, p.created_unix + 5)
                .unwrap_err(),
            ConfirmError::HighInBatch
        );

        // A single high step is fine (it is confirmed on its own).
        let mut single = pending_one(json!({"app_id": "org.videolan.VLC"}));
        single.steps[0] = (
            "pkg.app_install_system".into(),
            json!({"app_id": "org.videolan.VLC"}),
            Risk::High,
        );
        let claims = claims_for(&single, &nonce('J'), single.created_unix + MAX_STEP_SECONDS);
        let token = sign(&claims, &dev_key());
        assert!(
            Verifier::new(dev_key().verifying_key())
                .verify_and_consume(&token, &single, single.created_unix + 5)
                .is_ok()
        );
    }

    #[test]
    fn pending_for_formats_principal_and_normalizes_args() {
        let principal = Principal {
            uid: 1000,
            is_admin: false,
            channel: Channel::Local,
            session_id: Some("01920000-0000-7000-8000-000000000001".into()),
        };
        let nfd = "Tiếng Việt".nfd().collect::<String>();
        let p = pending_for(
            &principal,
            "t-1:0",
            vec![("fs.write_note".into(), json!({"text": nfd}), Risk::Medium)],
            "pv1",
        )
        .unwrap();
        assert_eq!(
            p.principal,
            "uid:1000/session:01920000-0000-7000-8000-000000000001"
        );
        assert_eq!(p.uid, 1000);
        assert_eq!(p.task_step, "t-1:0");
        assert_eq!(p.policy_version, "pv1");
        assert_eq!(p.dry_run_hash, None);
        // The stored copy is NFC: what is hashed, shown and executed.
        assert_eq!(p.steps[0].1, json!({"text": "Tiếng Việt"}));
        assert_eq!(p.request_id.len(), 36); // UUIDv7 string form
    }

    #[test]
    fn token_length_boundaries() {
        let p = pending_one(json!({"a": 1}));
        let verifier = Verifier::new(dev_key().verifying_key());
        let claims = claims_for(&p, &nonce('K'), p.created_unix + MAX_STEP_SECONDS);
        let token = sign(&claims, &dev_key());
        assert!(token.len() <= MAX_TOKEN_BYTES);

        // Exactly at the limit is not TooLong; it fails later as BadFormat.
        let at_limit = "x".repeat(MAX_TOKEN_BYTES);
        assert_eq!(
            verifier
                .verify_and_consume(&at_limit, &p, p.created_unix)
                .unwrap_err(),
            ConfirmError::BadFormat
        );
        let over_limit = "x".repeat(MAX_TOKEN_BYTES + 1);
        assert_eq!(
            verifier
                .verify_and_consume(&over_limit, &p, p.created_unix)
                .unwrap_err(),
            ConfirmError::TooLong
        );
    }

    #[test]
    fn malformed_tokens_fail_as_bad_format() {
        let p = pending_one(json!({"a": 1}));
        let verifier = Verifier::new(dev_key().verifying_key());
        let now = p.created_unix + 5;
        for bad in ["", "not-a-token", "a.b.c", ".x", "x.", "####.####"] {
            assert_eq!(
                verifier.verify_and_consume(bad, &p, now).unwrap_err(),
                ConfirmError::BadFormat,
                "case {bad:?}"
            );
        }
        // Valid base64url body that is not JSON claims.
        let body = URL_SAFE_NO_PAD.encode(b"{}");
        assert_eq!(
            verifier
                .verify_and_consume(&format!("{body}.{body}"), &p, now)
                .unwrap_err(),
            ConfirmError::BadFormat
        );
    }

    #[test]
    fn pending_confirm_json_matches_vector_shape() {
        // `steps` serialize as {tool, args, risk} objects, exactly like
        // the "pending" field of testdata/confirm-vectors/vectors.json.
        let p = pending_one(json!({"name": "ke-hoach", "text": "Họp lúc 9 giờ"}));
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["steps"][0]["tool"], "fs.write_note");
        assert_eq!(v["steps"][0]["risk"], "medium");
        assert_eq!(v["steps"][0]["args"]["name"], "ke-hoach");
        assert_eq!(v["dry_run_hash"], serde_json::Value::Null);
        let back: PendingConfirm = serde_json::from_value(v).unwrap();
        assert_eq!(back, p);
    }
}
