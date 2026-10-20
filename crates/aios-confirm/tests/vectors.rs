//! Confirm-token vectors (design.md §9.5, docs/m0/confirm.md). Written by the leader (task ld-secspec).
//! Workers MUST NOT edit this file or testdata/confirm-vectors/**.
//! Gated behind feature `spec-tests` until aios-confirm exposes the API: `cargo test -p aios-confirm --features spec-tests`.
#![cfg(feature = "spec-tests")]

use aios_confirm::{ConfirmClaims, PendingConfirm, Verifier, sign};
use aios_types::Risk;
use base64::Engine;
use ed25519_dalek::{SigningKey, VerifyingKey};
use serde_json::Value;
use std::path::PathBuf;

fn load() -> Value {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/confirm-vectors/vectors.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn pending(v: &Value) -> PendingConfirm {
    let steps = v["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            let risk: Risk = serde_json::from_value(s["risk"].clone()).unwrap();
            (
                s["tool"].as_str().unwrap().to_string(),
                s["args"].clone(),
                risk,
            )
        })
        .collect();
    PendingConfirm {
        request_id: v["request_id"].as_str().unwrap().into(),
        uid: v["uid"].as_u64().unwrap() as u32,
        principal: v["principal"].as_str().unwrap().into(),
        task_step: v["task_step"].as_str().unwrap().into(),
        steps,
        dry_run_hash: v["dry_run_hash"].as_str().map(String::from),
        policy_version: v["policy_version"].as_str().unwrap().into(),
        created_unix: v["created_unix"].as_u64().unwrap(),
    }
}

fn name_of<E: std::fmt::Debug>(e: &E) -> String {
    format!("{e:?}")
        .split(['(', ' ', '{'])
        .next()
        .unwrap()
        .to_string()
}

#[test]
fn vectors() {
    let data = load();
    let pk = base64::engine::general_purpose::STANDARD
        .decode(data["public_key_b64"].as_str().unwrap())
        .unwrap();
    let vk = VerifyingKey::from_bytes(&pk.try_into().unwrap()).unwrap();
    let sk = SigningKey::from_bytes(&(0u8..32).collect::<Vec<_>>().try_into().unwrap());
    assert_eq!(sk.verifying_key(), vk);

    for v in data["vectors"].as_array().unwrap() {
        let name = v["name"].as_str().unwrap();
        let expect = v["expect"].as_str().unwrap();
        let p = pending(&v["pending"]);
        let token = v["token"].as_str().unwrap();
        let now = v["now_unix"].as_u64().unwrap();
        let verifier = Verifier::new(vk);

        // signing side: deterministic Ed25519 over JCS(claims) must reproduce the valid tokens exactly
        if expect == "ok" {
            let claims: ConfirmClaims = serde_json::from_value(v["claims"].clone()).unwrap();
            assert_eq!(
                sign(&claims, &sk),
                token,
                "{name}: sign() must match vector token"
            );
        }

        let mut result = verifier
            .verify_and_consume(token, &p, now)
            .map_err(|e| name_of(&e));
        if v["kind"] == "replay" {
            assert!(result.is_ok(), "{name}: first use must succeed");
            result = verifier
                .verify_and_consume(token, &p, now)
                .map_err(|e| name_of(&e));
        }
        if let (Ok(vt), Some(steps)) = (result.as_mut(), v["consume"].as_array()) {
            for s in steps {
                if let Err(e) = verifier.consume_step(vt, s["tool"].as_str().unwrap(), &s["args"]) {
                    result = Err(name_of(&e));
                    break;
                }
            }
        }
        match (expect, &result) {
            ("ok", Ok(_)) => {}
            ("ok", Err(e)) => panic!("{name}: expected ok, got {e}"),
            (want, Ok(_)) => panic!("{name}: expected {want}, got ok"),
            (want, Err(e)) => assert_eq!(e, want, "{name}"),
        }
    }
}
