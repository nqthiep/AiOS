//! One-time nonce registry (design.md §9.5).
//!
//! M0 keeps nonces in memory only; persisting them across restarts is M1
//! work (docs/m0/confirm.md, "Ngoài phạm vi").

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use crate::ConfirmError;

/// In-memory nonce registry. A nonce is remembered until its token expires,
/// then forgotten; a nonce seen twice while still valid is a replay.
///
/// Expired entries are pruned on every check, which keeps the store
/// self-cleaning without a background task and is enough for M0 volumes.
#[derive(Debug, Default)]
pub struct NonceStore {
    /// nonce -> exp_unix of the token that consumed it.
    seen: Mutex<HashMap<String, u64>>,
}

impl NonceStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reject `nonce` if some unexpired token already used it; otherwise
    /// record it until `exp_unix`. Only called after every other check has
    /// passed, so the nonce is consumed exactly when the token is accepted.
    pub(crate) fn check_and_consume(
        &self,
        nonce: &str,
        exp_unix: u64,
        now_unix: u64,
    ) -> Result<(), ConfirmError> {
        // The payload is a plain map, always structurally consistent, so
        // recovering from a poisoned lock carries no risk.
        let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        seen.retain(|_, exp| *exp > now_unix);
        if seen.contains_key(nonce) {
            return Err(ConfirmError::NonceReused);
        }
        seen.insert(nonce.to_owned(), exp_unix);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonce_usable_once_until_expiry() {
        let store = NonceStore::new();
        assert!(store.check_and_consume("n1", 100, 50).is_ok());
        assert_eq!(
            store.check_and_consume("n1", 100, 50).unwrap_err(),
            ConfirmError::NonceReused
        );
        // A different nonce is fine.
        assert!(store.check_and_consume("n2", 100, 50).is_ok());
        // Once the first token's expiry has passed, its nonce is forgotten.
        assert!(store.check_and_consume("n1", 200, 101).is_ok());
    }
}
