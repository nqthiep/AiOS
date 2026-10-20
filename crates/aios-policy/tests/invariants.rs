//! Policy invariants (tech-stack.md §4, design.md §9.3, §18.3). Written by the leader (task ld-secspec).
//! Workers MUST NOT edit this file. Exhaustive over the finite input space (3,072 combinations),
//! plus the concrete cases in testdata/policy-invariants/cases.json.
//! Gated behind feature `spec-tests` until aios-policy exposes the API: `cargo test -p aios-policy --features spec-tests`.
#![cfg(feature = "spec-tests")]

use aios_policy::{PolicyEngine, PolicyInput};
use aios_types::{
    Channel, ConfirmLevel, Principal, Privacy, Provenance, Risk, SandboxProfile, ToolSpec, Verdict,
};
use serde_json::Value;
use std::path::PathBuf;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn engine() -> PolicyEngine {
    PolicyEngine::load(&repo().join("policy")).expect("policy/ must load")
}

#[allow(clippy::too_many_arguments)]
fn tool(
    name: &str,
    risk: Risk,
    reversible: bool,
    always: bool,
    system: bool,
    wtc: bool,
) -> ToolSpec {
    ToolSpec {
        name: name.into(),
        plugin_id: "aios.test".into(),
        description: String::new(),
        input_schema: serde_json::json!({"type": "object"}),
        risk,
        reversible,
        rollback_tool: None,
        always_confirm: always,
        system_scope: system,
        writes_trusted_core: wtc,
        result_privacy: Privacy::Personal,
        result_provenance: Provenance::Observed,
        entry: vec!["true".into()],
        sandbox: SandboxProfile {
            ro_paths: vec![],
            rw_paths: vec![],
            tcp_connect: vec![],
            allow_unix_socket: vec![],
            memory_max_mb: 512,
            landlock_required: true,
        },
        timeout_ms: 30_000,
    }
}

fn principal(channel: Channel, is_admin: bool) -> Principal {
    Principal {
        uid: 1000,
        is_admin,
        channel,
        session_id: Some("s".into()),
    }
}

const RISKS: [Risk; 3] = [Risk::Low, Risk::Medium, Risk::High];
const CHANNELS: [Channel; 4] = [
    Channel::Local,
    Channel::Voice,
    Channel::Remote,
    Channel::Schedule,
];
const B: [bool; 2] = [false, true];

#[test]
fn exhaustive_invariants() {
    let e = engine();
    let mut n = 0;
    for risk in RISKS {
        for reversible in B {
            for always in B {
                for system in B {
                    for wtc in B {
                        for autonomy in 0u8..=3 {
                            for tainted in B {
                                for channel in CHANNELS {
                                    for admin in B {
                                        n += 1;
                                        let t =
                                            tool("x.tool", risk, reversible, always, system, wtc);
                                        let p = principal(channel, admin);
                                        let input = PolicyInput {
                                            principal: &p,
                                            tool: &t,
                                            user_autonomy: autonomy,
                                            tainted,
                                        };
                                        let v = e.evaluate(&input);
                                        let sets = e.decide_sets(&input);
                                        let ctx = format!(
                                            "{risk:?} rev={reversible} always={always} sys={system} wtc={wtc} aut={autonomy} taint={tainted} {channel:?} admin={admin} -> {v:?}"
                                        );
                                        // 1. auto never allows high
                                        if risk == Risk::High {
                                            assert!(!sets.auto, "inv1 {ctx}");
                                        }
                                        // 2. auto never allows when tainted (REQ: external context raises confirmation)
                                        if tainted {
                                            assert!(!sets.auto, "inv2 {ctx}");
                                        }
                                        // 3. high from remote/schedule is denied by both sets
                                        if risk == Risk::High
                                            && matches!(
                                                channel,
                                                Channel::Remote | Channel::Schedule
                                            )
                                        {
                                            assert!(matches!(v, Verdict::Deny(_)), "inv3 {ctx}");
                                        }
                                        // 4. auto allow implies confirmable allow
                                        if sets.auto {
                                            assert!(sets.confirmable, "inv4 {ctx}");
                                        }
                                        // 5. system_scope without admin is denied
                                        if system && !admin {
                                            assert!(matches!(v, Verdict::Deny(_)), "inv5 {ctx}");
                                        }
                                        // 6. system_scope is never auto-allowed
                                        if system {
                                            assert!(v != Verdict::Allow, "inv6 {ctx}");
                                        }
                                        // extra: trusted-core writers denied; irreversible / always_confirm never auto
                                        if wtc {
                                            assert!(matches!(v, Verdict::Deny(_)), "wtc {ctx}");
                                        }
                                        if !reversible || always {
                                            assert!(v != Verdict::Allow, "irrev/always {ctx}");
                                        }
                                        // verdict agrees with the two sets
                                        let expect_kind = if sets.auto {
                                            "allow"
                                        } else if sets.confirmable {
                                            "confirm"
                                        } else {
                                            "deny"
                                        };
                                        let kind = match v {
                                            Verdict::Allow => "allow",
                                            Verdict::Confirm(_) => "confirm",
                                            Verdict::Deny(_) => "deny",
                                        };
                                        assert_eq!(kind, expect_kind, "sets {ctx}");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(n, 3072);
}

#[test]
fn concrete_cases() {
    let e = engine();
    let raw =
        std::fs::read_to_string(repo().join("testdata/policy-invariants/cases.json")).unwrap();
    let cases: Vec<Value> = serde_json::from_str(&raw).unwrap();
    assert!(cases.len() >= 20);
    for c in cases {
        let risk: Risk = serde_json::from_value(c["risk"].clone()).unwrap();
        let channel: Channel = serde_json::from_value(c["channel"].clone()).unwrap();
        let t = tool(
            c["tool"].as_str().unwrap_or("x.tool"),
            risk,
            c["reversible"].as_bool().unwrap(),
            c["always_confirm"].as_bool().unwrap_or(false),
            c["system_scope"].as_bool().unwrap_or(false),
            c["writes_trusted_core"].as_bool().unwrap_or(false),
        );
        let p = principal(channel, c["is_admin"].as_bool().unwrap_or(false));
        let input = PolicyInput {
            principal: &p,
            tool: &t,
            user_autonomy: c["user_autonomy"].as_u64().unwrap() as u8,
            tainted: c["tainted"].as_bool().unwrap_or(false),
        };
        let got = e.evaluate(&input);
        let want = c["expect"].as_str().unwrap();
        let ok = matches!(
            (want, &got),
            ("allow", Verdict::Allow)
                | ("confirm", Verdict::Confirm(ConfirmLevel::Dialog))
                | ("confirm_pin", Verdict::Confirm(ConfirmLevel::DialogWithPin))
                | ("deny", Verdict::Deny(_))
        );
        assert!(ok, "case {}: want {want}, got {got:?}", c["name"]);
    }
}
