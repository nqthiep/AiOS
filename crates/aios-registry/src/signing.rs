//! Ed25519 signing helpers for plugin manifests.
//!
//! The signature is computed over the **exact bytes** of `plugin.toml`; the
//! bytes are never normalized, re-encoded or line-ending translated before
//! signing or verifying (docs/m0/registry.md).
//!
//! * `plugin.toml.sig` holds the standard base64 encoding of the 64 raw
//!   signature bytes.
//! * A public key file (`*.pub`) holds the standard base64 encoding of the 32
//!   raw public key bytes.
//! * A secret key file holds the standard base64 encoding of the 32 raw seed
//!   bytes.

use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand_core::OsRng;

/// Length of a raw Ed25519 secret key (the seed) in bytes.
pub const SECRET_KEY_LENGTH: usize = 32;
/// Length of a raw Ed25519 public key in bytes.
pub const PUBLIC_KEY_LENGTH: usize = 32;
/// Length of a raw Ed25519 signature in bytes.
pub const SIGNATURE_LENGTH: usize = 64;

/// Decodes a standard base64 payload, ignoring whitespace.
///
/// Key and signature files are text files: a trailing newline or a wrapped
/// payload must not make a valid signature unreadable.
fn decode_base64(text: &str) -> Result<Vec<u8>, SigningError> {
    let compact: String = text.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    Ok(STANDARD.decode(compact)?)
}

/// Errors of the signing helpers.
#[derive(Debug, thiserror::Error)]
pub enum SigningError {
    /// The base64 payload could not be decoded.
    #[error("invalid base64: {0}")]
    Base64(#[from] base64::DecodeError),
    /// A key or signature had the wrong length.
    #[error("invalid length: expected {expected} bytes, got {actual}")]
    Length { expected: usize, actual: usize },
    /// The public key bytes are not a valid Ed25519 point.
    #[error("invalid ed25519 public key")]
    PublicKey,
    /// The signature bytes are not a valid Ed25519 signature.
    #[error("invalid ed25519 signature")]
    Signature,
    /// A file could not be read or written.
    #[error("i/o error on {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// A raw Ed25519 secret key (32 bytes).
#[derive(Clone)]
pub struct SecretKey([u8; SECRET_KEY_LENGTH]);

/// A raw Ed25519 public key (32 bytes).
#[derive(Clone)]
pub struct PublicKey([u8; PUBLIC_KEY_LENGTH]);

/// An Ed25519 key pair.
#[derive(Clone)]
pub struct KeyPair {
    secret: SecretKey,
    public: PublicKey,
}

impl std::fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print key material.
        f.write_str("SecretKey(<redacted>)")
    }
}

impl std::fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PublicKey({})", self.to_base64())
    }
}

impl std::fmt::Debug for KeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyPair")
            .field("secret", &self.secret)
            .field("public", &self.public)
            .finish()
    }
}

impl SecretKey {
    /// Builds a secret key from raw seed bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SigningError> {
        let array: [u8; SECRET_KEY_LENGTH] =
            bytes.try_into().map_err(|_| SigningError::Length {
                expected: SECRET_KEY_LENGTH,
                actual: bytes.len(),
            })?;
        Ok(Self(array))
    }

    /// Decodes a base64 encoded secret key.
    pub fn from_base64(text: &str) -> Result<Self, SigningError> {
        let bytes = decode_base64(text)?;
        Self::from_bytes(&bytes)
    }

    /// The raw seed bytes.
    pub fn as_bytes(&self) -> &[u8; SECRET_KEY_LENGTH] {
        &self.0
    }

    /// The base64 encoding of the raw seed bytes.
    pub fn to_base64(&self) -> String {
        STANDARD.encode(self.0)
    }

    /// The ed25519 signing key.
    pub fn signing_key(&self) -> SigningKey {
        SigningKey::from_bytes(&self.0)
    }

    /// Signs a message.
    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing_key().sign(message)
    }
}

impl PublicKey {
    /// Builds a public key from raw bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SigningError> {
        let array: [u8; PUBLIC_KEY_LENGTH] =
            bytes.try_into().map_err(|_| SigningError::Length {
                expected: PUBLIC_KEY_LENGTH,
                actual: bytes.len(),
            })?;
        VerifyingKey::from_bytes(&array).map_err(|_| SigningError::PublicKey)?;
        Ok(Self(array))
    }

    /// Decodes a base64 encoded public key.
    pub fn from_base64(text: &str) -> Result<Self, SigningError> {
        let bytes = decode_base64(text)?;
        Self::from_bytes(&bytes)
    }

    /// The raw public key bytes.
    pub fn as_bytes(&self) -> &[u8; PUBLIC_KEY_LENGTH] {
        &self.0
    }

    /// The base64 encoding of the raw public key bytes.
    pub fn to_base64(&self) -> String {
        STANDARD.encode(self.0)
    }

    /// The ed25519 verifying key.
    pub fn verifying_key(&self) -> VerifyingKey {
        VerifyingKey::from_bytes(&self.0).expect("public key was validated on construction")
    }

    /// Verifies a signature over `message`.
    ///
    /// `verify_strict` is used on purpose: it rejects small order points and
    /// non canonical signatures instead of accepting them.
    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        self.verifying_key()
            .verify_strict(message, signature)
            .is_ok()
    }
}

impl KeyPair {
    /// Generates a fresh key pair from the operating system CSPRNG.
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self {
            secret: SecretKey(signing_key.to_bytes()),
            public: PublicKey(signing_key.verifying_key().to_bytes()),
        }
    }

    /// The secret key of the pair.
    pub fn secret(&self) -> &SecretKey {
        &self.secret
    }

    /// The public key of the pair.
    pub fn public(&self) -> &PublicKey {
        &self.public
    }
}

/// Generates a fresh Ed25519 key pair.
pub fn generate_keypair() -> KeyPair {
    KeyPair::generate()
}

/// Signs `message` with `secret`.
pub fn sign_bytes(secret: &SecretKey, message: &[u8]) -> Signature {
    secret.sign(message)
}

/// Verifies `signature` over `message` with `public`.
pub fn verify_bytes(public: &PublicKey, message: &[u8], signature: &Signature) -> bool {
    public.verify(message, signature)
}

/// Decodes the base64 payload of a `.sig` file.
pub fn decode_signature(text: &str) -> Result<Signature, SigningError> {
    let bytes = decode_base64(text)?;
    if bytes.len() != SIGNATURE_LENGTH {
        return Err(SigningError::Length {
            expected: SIGNATURE_LENGTH,
            actual: bytes.len(),
        });
    }
    Signature::from_slice(&bytes).map_err(|_| SigningError::Signature)
}

/// The base64 payload of a `.sig` file.
pub fn encode_signature(signature: &Signature) -> String {
    STANDARD.encode(signature.to_bytes())
}

/// Signs the exact bytes of `manifest_path` and writes `<manifest_path>.sig`.
///
/// The signature file contains the standard base64 encoding of the 64 raw
/// signature bytes.
pub fn sign_file(manifest_path: &Path, secret: &SecretKey) -> Result<PathBuf, SigningError> {
    let bytes = fs::read(manifest_path).map_err(|source| SigningError::Io {
        path: manifest_path.to_path_buf(),
        source,
    })?;
    let signature = secret.sign(&bytes);
    let signature_path = signature_path_for(manifest_path);
    let payload = format!("{}\n", encode_signature(&signature));
    fs::write(&signature_path, payload).map_err(|source| SigningError::Io {
        path: signature_path.clone(),
        source,
    })?;
    Ok(signature_path)
}

/// The path of the signature file that belongs to a manifest.
pub fn signature_path_for(manifest_path: &Path) -> PathBuf {
    let name = manifest_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "plugin.toml".to_string());
    manifest_path.with_file_name(format!("{name}.sig"))
}

/// Writes a secret key to `path` and its public key to `path.pub`.
pub fn write_key_pair(path: &Path, pair: &KeyPair) -> Result<(PathBuf, PathBuf), SigningError> {
    let (secret_path, public_path) = key_pair_paths(path);
    fs::write(&secret_path, pair.secret().to_base64()).map_err(|source| SigningError::Io {
        path: secret_path.clone(),
        source,
    })?;
    fs::write(&public_path, pair.public().to_base64()).map_err(|source| SigningError::Io {
        path: public_path.clone(),
        source,
    })?;
    Ok((secret_path, public_path))
}

/// Reads a secret key from a base64 encoded file.
pub fn read_secret_key(path: &Path) -> Result<SecretKey, SigningError> {
    let text = fs::read_to_string(path).map_err(|source| SigningError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    SecretKey::from_base64(&text)
}

/// The paths a key pair is written to.
///
/// The secret key always goes to `path`; the public key goes to `path.pub`.
/// When `path` already ends in `.pub` the roles are swapped: the public key
/// goes to `path` and the secret key to `path.key`.
pub fn key_pair_paths(path: &Path) -> (PathBuf, PathBuf) {
    if path.extension().is_some_and(|ext| ext == "pub") {
        let secret = path.with_extension("key");
        (secret, path.to_path_buf())
    } else {
        let public = path.with_extension("pub");
        (path.to_path_buf(), public)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_pair_roundtrip() {
        let pair = generate_keypair();
        assert_eq!(
            SecretKey::from_base64(&pair.secret().to_base64())
                .unwrap()
                .as_bytes(),
            pair.secret().as_bytes()
        );
        assert_eq!(
            PublicKey::from_base64(&pair.public().to_base64())
                .unwrap()
                .as_bytes(),
            pair.public().as_bytes()
        );
    }

    #[test]
    fn signature_roundtrip() {
        let pair = generate_keypair();
        let signature = sign_bytes(pair.secret(), b"hello");
        assert!(verify_bytes(pair.public(), b"hello", &signature));
        assert!(!verify_bytes(pair.public(), b"hellp", &signature));
        let encoded = encode_signature(&signature);
        assert_eq!(
            decode_signature(&encoded).unwrap().to_bytes(),
            signature.to_bytes()
        );
    }

    #[test]
    fn wrong_length_is_rejected() {
        assert!(matches!(
            decode_signature("AAAA"),
            Err(SigningError::Length { .. })
        ));
        assert!(matches!(
            PublicKey::from_bytes(&[0u8; 31]),
            Err(SigningError::Length { .. })
        ));
    }

    #[test]
    fn key_pair_paths_swap_for_pub_files() {
        let (secret, public) = key_pair_paths(Path::new("/tmp/keys/aios.pub"));
        assert_eq!(secret, PathBuf::from("/tmp/keys/aios.key"));
        assert_eq!(public, PathBuf::from("/tmp/keys/aios.pub"));

        let (secret, public) = key_pair_paths(Path::new("/tmp/keys/dev.key"));
        assert_eq!(secret, PathBuf::from("/tmp/keys/dev.key"));
        assert_eq!(public, PathBuf::from("/tmp/keys/dev.pub"));
    }
}
