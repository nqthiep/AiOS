//! Virtual keys and session state (docs/m0/gateway.md "Quản trị").
//!
//! The gateway issues a virtual key `aios-sk-<32 bytes base64url>` per session or
//! service. Clients call the API with it and never see the real cloud key.
//! State is in memory only: a gateway restart invalidates every key (fail closed).

use std::collections::HashMap;
use std::sync::RwLock;

use aios_types::{Privacy, SessionState, Taint};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore;

use crate::config::KeyScope;

/// Virtual key prefix.
pub const KEY_PREFIX: &str = "aios-sk-";
/// Random bytes in a virtual key.
pub const KEY_BYTES: usize = 32;

/// Data bound to a virtual key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyInfo {
    /// Owning UID (from the admin API, never from a client request body).
    pub uid: u32,
    /// Client name, looked up in `clients`.
    pub client: String,
    /// Bound session; `None` for service keys.
    pub session_id: Option<String>,
    /// Key scope.
    pub scope: KeyScope,
}

/// Generate a new virtual key: `aios-sk-<32 bytes base64url>`.
pub fn generate_key() -> String {
    let mut bytes = [0u8; KEY_BYTES];
    rand::rng().fill_bytes(&mut bytes);
    format!("{KEY_PREFIX}{}", URL_SAFE_NO_PAD.encode(bytes))
}

#[derive(Debug, Default)]
struct Inner {
    /// Virtual key -> info.
    keys: HashMap<String, KeyInfo>,
    /// Session -> state (label, taint).
    sessions: HashMap<String, SessionState>,
}

/// Virtual key and session registry.
#[derive(Debug, Default)]
pub struct SessionRegistry {
    inner: RwLock<Inner>,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Issue a new virtual key for a session.
    ///
    /// Returns `None` when the session already exists for another uid. An
    /// existing session keeps its label (labels only rise).
    pub fn issue_session_key(
        &self,
        uid: u32,
        session_id: &str,
        client: &str,
        scope: KeyScope,
    ) -> Option<(String, KeyInfo)> {
        let key = generate_key();
        let info = KeyInfo {
            uid,
            client: client.to_string(),
            session_id: Some(session_id.to_string()),
            scope,
        };
        let mut guard = self.write();
        let state = guard
            .sessions
            .entry(session_id.to_string())
            .or_insert_with(|| SessionState::new(session_id, uid));
        if state.uid != uid {
            return None;
        }
        guard.keys.insert(key.clone(), info.clone());
        Some((key, info))
    }

    /// Issue a service key (no session): its label is always `secret`.
    pub fn issue_service_key(&self, uid: u32, client: &str) -> (String, KeyInfo) {
        let key = generate_key();
        let info = KeyInfo {
            uid,
            client: client.to_string(),
            session_id: None,
            scope: KeyScope::Service,
        };
        self.write().keys.insert(key.clone(), info.clone());
        (key, info)
    }

    /// Look up a virtual key.
    pub fn lookup(&self, key: &str) -> Option<KeyInfo> {
        self.read().keys.get(key).cloned()
    }

    /// Session label, if the session exists.
    pub fn session_label(&self, session_id: &str) -> Option<Privacy> {
        self.read().sessions.get(session_id).map(|s| s.label)
    }

    /// Raise session label and taint; any lowering part is ignored.
    /// Returns the current label/taint afterwards.
    pub fn raise(
        &self,
        session_id: &str,
        label: Privacy,
        taint: Taint,
    ) -> Option<(Privacy, Taint)> {
        let mut guard = self.write();
        let state = guard.sessions.get_mut(session_id)?;
        state.raise(label, taint);
        Some((state.label, state.taint))
    }

    /// Revoke every key of a session. Returns the number of revoked keys.
    pub fn revoke_session(&self, session_id: &str) -> usize {
        let mut guard = self.write();
        guard.sessions.remove(session_id);
        let before = guard.keys.len();
        guard
            .keys
            .retain(|_, info| info.session_id.as_deref() != Some(session_id));
        before - guard.keys.len()
    }

    /// Revoke every session key (kill switch). Service keys are kept.
    pub fn revoke_all_session_keys(&self) -> usize {
        let mut guard = self.write();
        let before = guard.keys.len();
        guard.keys.retain(|_, info| info.session_id.is_none());
        guard.sessions.clear();
        before - guard.keys.len()
    }

    fn read(&self) -> std::sync::RwLockReadGuard<'_, Inner> {
        self.inner.read().unwrap_or_else(|e| e.into_inner())
    }

    fn write(&self) -> std::sync::RwLockWriteGuard<'_, Inner> {
        self.inner.write().unwrap_or_else(|e| e.into_inner())
    }
}
