//! aios-policy — the Cedar-backed Policy Engine. Spec: docs/m0/policy.md
//! (sources: tech-stack.md §4, design.md §9.3, §11.2, §18.3).
//!
//! Two Cedar policy sets share one `forbid` set: an Allow from the `auto`
//! set runs without asking; an Allow from only the `confirmable` set needs a
//! confirmation token issued on a trusted surface; everything else is Deny.
//! Every context field is computed here from broker-owned data — no field,
//! and in particular no confirmation state, is ever taken from a client
//! request (design.md §9.5).

// PolicyError wraps Cedar parse errors (large). Only PolicyEngine::load returns it, once at startup,
// so the size of the Err variant does not matter on any hot path.
#![allow(clippy::result_large_err)]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use aios_types::{Channel, ConfirmLevel, Principal, Risk, ToolSpec, Verdict};
use cedar_policy::{
    Authorizer, Context, Decision, Effect, Entities, Entity, EntityId, EntityTypeName, EntityUid,
    PolicySet, Request, Response, RestrictedExpression, Schema, ValidationMode, Validator,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// The four files of a policy directory, in filename order. `forbid.cedar` is
/// loaded into BOTH policy sets; the others define the schema and the two
/// permit sets (tech-stack.md §4). The filename order is also the input order
/// for the version hash.
const POLICY_FILES: [&str; 4] = [
    "aios.cedarschema",
    "auto.cedar",
    "confirmable.cedar",
    "forbid.cedar",
];

/// Deny reason when no permit policy applied at all (implicit Cedar deny).
const NO_PERMIT: &str = "no-permit";

/// Deny reason used only if Cedar rejects a request we built ourselves, which
/// would mean the request shape drifted from the schema validated at load
/// time. Unreachable for well-typed inputs; kept so the engine fails closed
/// instead of panicking.
const INTERNAL_DENY: &str = "internal";

/// Errors from [`PolicyEngine::load`]. The engine refuses to start on any
/// read, parse, or validation failure — there is no pass-through mode
/// (fail closed, docs/m0/policy.md).
#[derive(Debug, Error)]
pub enum PolicyError {
    /// A policy file could not be read.
    #[error("failed to read policy file `{path}`: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// The Cedar schema could not be parsed.
    #[error("failed to parse Cedar schema: {0}")]
    SchemaParse(#[from] cedar_policy::CedarSchemaError),
    /// A Cedar policy set could not be parsed.
    #[error("failed to parse {set} policy set: {source}")]
    PolicyParse {
        set: &'static str,
        source: cedar_policy::ParseErrors,
    },
    /// A Cedar entity reference reserved by this engine could not be parsed.
    #[error("failed to parse Cedar entity reference `{reference}`: {source}")]
    EntityRefParse {
        reference: &'static str,
        source: cedar_policy::ParseErrors,
    },
    /// A policy set does not conform to the schema.
    #[error("{set} policy set failed schema validation: {}", errors.join("; "))]
    Validation {
        set: &'static str,
        errors: Vec<String>,
    },
}

/// One decision request. Every field is broker-owned; a client never
/// supplies confirmation state or policy context (design.md §9.5).
#[derive(Debug, Clone, Copy)]
pub struct PolicyInput<'a> {
    /// Principal of the call (uid from SO_PEERCRED or a uid-bound
    /// credential, never from a request body, design.md §18.1).
    pub principal: &'a Principal,
    /// Tool being invoked, from the signed registry (risk is the effective
    /// level, design.md §15.2).
    pub tool: &'a ToolSpec,
    /// Autonomy level the user configured, 0..=3 (design.md §11.2).
    pub user_autonomy: u8,
    /// Whether the session is tainted (design.md §6.3).
    pub tainted: bool,
}

/// Per-set outcome for one request: which of the two policy sets allows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetDecisions {
    /// The `auto` set (`auto.cedar` + `forbid.cedar`) allows the request.
    pub auto: bool,
    /// The `confirmable` set (`confirmable.cedar` + `forbid.cedar`) allows
    /// the request.
    pub confirmable: bool,
}

/// Cedar-backed policy engine: two permit sets plus one shared forbid set,
/// all validated against one schema at load time.
#[derive(Debug)]
pub struct PolicyEngine {
    /// `auto.cedar` + `forbid.cedar`: an Allow runs without asking.
    auto: PolicySet,
    /// `confirmable.cedar` + `forbid.cedar`: an Allow needs a confirmation
    /// token (design.md §9.5).
    confirmable: PolicySet,
    /// Schema the policy sets were validated against and every request is
    /// built with.
    schema: Schema,
    /// Entity type names used in request building, parsed once at load so
    /// that evaluation stays infallible.
    agent_type: EntityTypeName,
    /// Tool entity type name, parsed once at load.
    tool_type: EntityTypeName,
    /// The `Action::"invoke"` entity uid, parsed once at load.
    invoke: EntityUid,
    /// SHA-256 hex over the four policy file contents, bound into
    /// confirmation tokens (design.md §9.5).
    version: String,
}

impl PolicyEngine {
    /// Load a policy directory: `auto.cedar` + `forbid.cedar` as the auto
    /// set, `confirmable.cedar` + `forbid.cedar` as the confirmable set, both
    /// validated against `aios.cedarschema`. Any read, parse, or validation
    /// failure is an error — the broker must not run without valid policies.
    pub fn load(dir: &Path) -> Result<Self, PolicyError> {
        // Read the four files in filename order, hashing the contents as we
        // go: the version is SHA-256 over the concatenation of all four.
        let mut texts = Vec::with_capacity(POLICY_FILES.len());
        let mut hasher = Sha256::new();
        for name in POLICY_FILES {
            let path = dir.join(name);
            let text = std::fs::read_to_string(&path)
                .map_err(|source| PolicyError::Io { path, source })?;
            hasher.update(text.as_bytes());
            texts.push(text);
        }
        let version: String = hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let [schema_text, auto_text, confirmable_text, forbid_text] = texts
            .try_into()
            .expect("POLICY_FILES contains exactly four names");

        let schema: Schema = schema_text.parse()?;
        let validator = Validator::new(schema.clone());
        let auto = parse_set("auto", &auto_text, &forbid_text, &validator)?;
        let confirmable = parse_set("confirmable", &confirmable_text, &forbid_text, &validator)?;

        Ok(Self {
            auto,
            confirmable,
            schema,
            agent_type: EntityTypeName::from_str("Agent").map_err(|source| {
                PolicyError::EntityRefParse {
                    reference: "Agent",
                    source,
                }
            })?,
            tool_type: EntityTypeName::from_str("Tool").map_err(|source| {
                PolicyError::EntityRefParse {
                    reference: "Tool",
                    source,
                }
            })?,
            invoke: r#"Action::"invoke""#
                .parse()
                .map_err(|source| PolicyError::EntityRefParse {
                    reference: r#"Action::"invoke""#,
                    source,
                })?,
            version,
        })
    }

    /// Full verdict for one request: [`Verdict::Allow`] when the auto set
    /// allows it; [`Verdict::Confirm`] with the required level when only the
    /// confirmable set allows it; [`Verdict::Deny`] otherwise, with the `@id`
    /// of a forbid policy that determined the decision, or [`NO_PERMIT`]
    /// when no permit applied at all.
    pub fn evaluate(&self, input: &PolicyInput<'_>) -> Verdict {
        let Some((auto, confirmable)) = self.authorize(input) else {
            // Fail closed: if a request we built is rejected, neither set
            // allows it.
            return Verdict::Deny(INTERNAL_DENY.to_string());
        };
        if matches!(auto.decision(), Decision::Allow) {
            Verdict::Allow
        } else if matches!(confirmable.decision(), Decision::Allow) {
            Verdict::Confirm(confirm_level(input.tool, input.tainted))
        } else {
            Verdict::Deny(self.deny_reason(&confirmable))
        }
    }

    /// Per-set outcomes for one request (used by the invariants spec). A
    /// `false` for both sets means Deny; `true` only for `confirmable` means
    /// Confirm.
    pub fn decide_sets(&self, input: &PolicyInput<'_>) -> SetDecisions {
        let Some((auto, confirmable)) = self.authorize(input) else {
            // Fail closed: if a request we built is rejected, neither set
            // allows it.
            return SetDecisions {
                auto: false,
                confirmable: false,
            };
        };
        SetDecisions {
            auto: matches!(auto.decision(), Decision::Allow),
            confirmable: matches!(confirmable.decision(), Decision::Allow),
        }
    }

    /// SHA-256 hex over the four policy file contents in filename order.
    /// Bound into confirmation tokens so a token cannot be replayed against
    /// different policies (design.md §9.5).
    pub fn policy_version(&self) -> String {
        self.version.clone()
    }

    /// Build the Cedar request and entities for one input, then run both
    /// policy sets. Returns `None` when Cedar rejects something we built
    /// ourselves. Unreachable for well-typed inputs: the entities and the
    /// context exactly mirror the schema that the policy sets were validated
    /// against at load time. Callers must treat `None` as deny-by-both (fail
    /// closed).
    fn authorize(&self, input: &PolicyInput<'_>) -> Option<(Response, Response)> {
        let principal_uid = EntityUid::from_type_name_and_id(
            self.agent_type.clone(),
            EntityId::new(input.principal.uid.to_string()),
        );
        let tool_uid = EntityUid::from_type_name_and_id(
            self.tool_type.clone(),
            EntityId::new(input.tool.name.clone()),
        );

        // Principal entity: Agent::<uid> with uid (Long) and is_admin (Bool).
        let principal_attrs = HashMap::from([
            (
                "uid".to_string(),
                RestrictedExpression::new_long(i64::from(input.principal.uid)),
            ),
            (
                "is_admin".to_string(),
                RestrictedExpression::new_bool(input.principal.is_admin),
            ),
        ]);
        let principal = Entity::new(principal_uid.clone(), principal_attrs, HashSet::new()).ok()?;

        // Tool entity: Tool::<name> with the attributes the policies read.
        let tool_attrs = HashMap::from([
            (
                "name".to_string(),
                RestrictedExpression::new_string(input.tool.name.clone()),
            ),
            (
                "risk".to_string(),
                RestrictedExpression::new_string(risk_str(input.tool.risk).to_string()),
            ),
            (
                "reversible".to_string(),
                RestrictedExpression::new_bool(input.tool.reversible),
            ),
            (
                "always_confirm".to_string(),
                RestrictedExpression::new_bool(input.tool.always_confirm),
            ),
            (
                "system_scope".to_string(),
                RestrictedExpression::new_bool(input.tool.system_scope),
            ),
            (
                "writes_trusted_core".to_string(),
                RestrictedExpression::new_bool(input.tool.writes_trusted_core),
            ),
        ]);
        let tool = Entity::new(tool_uid.clone(), tool_attrs, HashSet::new()).ok()?;

        // The schema argument validates both entities against the schema the
        // policy sets were validated with.
        let entities = Entities::from_entities([principal, tool], Some(&self.schema)).ok()?;

        // Every context field is computed here, never taken from the client
        // (design.md §9.5, tech-stack.md §4).
        let context = Context::from_json_value(
            json!({
                "autonomy": effective_autonomy(
                    input.user_autonomy,
                    input.principal.channel,
                    input.tool.writes(),
                ),
                "channel": channel_str(input.principal.channel),
                "tainted": input.tainted,
            }),
            None,
        )
        .ok()?;
        let request = Request::new(
            principal_uid,
            self.invoke.clone(),
            tool_uid,
            context,
            Some(&self.schema),
        )
        .ok()?;

        let authorizer = Authorizer::new();
        Some((
            authorizer.is_authorized(&request, &self.auto, &entities),
            authorizer.is_authorized(&request, &self.confirmable, &entities),
        ))
    }

    /// Deny reason from the confirmable set's response. On a Deny, Cedar's
    /// diagnostics carry the ids of the forbid policies that determined the
    /// decision (an empty set when no permit applied at all). Map each to its
    /// `@id` annotation and pick the smallest, so the reason is deterministic
    /// when several forbid policies apply at once.
    fn deny_reason(&self, confirmable: &Response) -> String {
        confirmable
            .diagnostics()
            .reason()
            // On a Deny the reasons are satisfied forbid policies; the
            // lookup and effect check are a guard against semantic drift.
            .filter_map(|id| self.confirmable.policy(id))
            .filter(|policy| policy.effect() == Effect::Forbid)
            .filter_map(|policy| policy.annotation("id"))
            .min()
            .unwrap_or(NO_PERMIT)
            .to_string()
    }
}

/// Parse one permit file plus `forbid.cedar` into a policy set validated
/// against the schema.
fn parse_set(
    set: &'static str,
    permits: &str,
    forbid: &str,
    validator: &Validator,
) -> Result<PolicySet, PolicyError> {
    let text = format!("{permits}\n{forbid}\n");
    let policy_set: PolicySet = text
        .parse()
        .map_err(|source| PolicyError::PolicyParse { set, source })?;
    let result = validator.validate(&policy_set, ValidationMode::default());
    let errors: Vec<String> = result.validation_errors().map(|e| e.to_string()).collect();
    if errors.is_empty() {
        Ok(policy_set)
    } else {
        Err(PolicyError::Validation { set, errors })
    }
}

/// Effective autonomy for one call: `min(user level, channel cap)`
/// (design.md §11.2). Remote and schedule channels cap at 1; voice caps at 1
/// for tools that write (risk >= Medium or irreversible, `ToolSpec::writes`);
/// local is uncapped.
pub fn effective_autonomy(user: u8, channel: Channel, tool_writes: bool) -> u8 {
    let cap = match channel {
        Channel::Remote | Channel::Schedule => 1,
        Channel::Voice if tool_writes => 1,
        Channel::Voice | Channel::Local => 3,
    };
    user.min(cap)
}

/// Confirmation level for a [`Verdict::Confirm`] (tech-stack.md §4): high
/// risk needs a PIN when the tool is irreversible or the session is tainted;
/// everything else is a plain dialog.
pub fn confirm_level(tool: &ToolSpec, tainted: bool) -> ConfirmLevel {
    if tool.risk == Risk::High && (!tool.reversible || tainted) {
        ConfirmLevel::DialogWithPin
    } else {
        ConfirmLevel::Dialog
    }
}

/// Lowercase schema string for a channel.
fn channel_str(channel: Channel) -> &'static str {
    match channel {
        Channel::Local => "local",
        Channel::Voice => "voice",
        Channel::Remote => "remote",
        Channel::Schedule => "schedule",
    }
}

/// Lowercase schema string for the effective risk level.
fn risk_str(risk: Risk) -> &'static str {
    match risk {
        Risk::Low => "low",
        Risk::Medium => "medium",
        Risk::High => "high",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aios_types::{Privacy, Provenance, SandboxProfile};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn repo() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn engine() -> PolicyEngine {
        PolicyEngine::load(&repo().join("policy")).expect("policy/ must load")
    }

    fn repo_policy(name: &str) -> String {
        std::fs::read_to_string(repo().join("policy").join(name)).unwrap()
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

    fn input<'a>(
        principal: &'a Principal,
        tool: &'a ToolSpec,
        user_autonomy: u8,
        tainted: bool,
    ) -> PolicyInput<'a> {
        PolicyInput {
            principal,
            tool,
            user_autonomy,
            tainted,
        }
    }

    /// Write the four policy files into a fresh temp directory and load it.
    fn load_temp(
        schema: &str,
        auto: &str,
        confirmable: &str,
        forbid: &str,
    ) -> Result<PolicyEngine, PolicyError> {
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "aios-policy-tests-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        for (name, text) in [
            ("aios.cedarschema", schema),
            ("auto.cedar", auto),
            ("confirmable.cedar", confirmable),
            ("forbid.cedar", forbid),
        ] {
            std::fs::write(dir.join(name), text).unwrap();
        }
        let result = PolicyEngine::load(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        result
    }

    #[test]
    fn effective_autonomy_follows_channel_caps() {
        for user in 0u8..=3 {
            // Remote and schedule always cap at 1.
            assert_eq!(
                effective_autonomy(user, Channel::Remote, false),
                user.min(1)
            );
            assert_eq!(effective_autonomy(user, Channel::Remote, true), user.min(1));
            assert_eq!(
                effective_autonomy(user, Channel::Schedule, false),
                user.min(1)
            );
            assert_eq!(
                effective_autonomy(user, Channel::Schedule, true),
                user.min(1)
            );
            // Local is uncapped.
            assert_eq!(effective_autonomy(user, Channel::Local, false), user);
            assert_eq!(effective_autonomy(user, Channel::Local, true), user);
            // Voice caps at 1 only for tools that write.
            assert_eq!(effective_autonomy(user, Channel::Voice, true), user.min(1));
            assert_eq!(effective_autonomy(user, Channel::Voice, false), user);
        }
    }

    #[test]
    fn confirm_level_needs_pin_only_for_high() {
        for (risk, reversible, tainted, want) in [
            (Risk::High, false, false, ConfirmLevel::DialogWithPin),
            (Risk::High, false, true, ConfirmLevel::DialogWithPin),
            (Risk::High, true, true, ConfirmLevel::DialogWithPin),
            (Risk::High, true, false, ConfirmLevel::Dialog),
            (Risk::Medium, false, true, ConfirmLevel::Dialog),
            (Risk::Medium, true, false, ConfirmLevel::Dialog),
            (Risk::Low, false, true, ConfirmLevel::Dialog),
            (Risk::Low, true, false, ConfirmLevel::Dialog),
        ] {
            let t = tool("x.tool", risk, reversible, false, false, false);
            assert_eq!(confirm_level(&t, tainted), want);
        }
    }

    #[test]
    fn policy_version_is_sha256_of_four_files_in_filename_order() {
        let mut hasher = Sha256::new();
        for name in POLICY_FILES {
            hasher.update(repo_policy(name).as_bytes());
        }
        let expected: String = hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let e = engine();
        let version = e.policy_version();
        assert_eq!(version, expected);
        assert_eq!(version.len(), 64);
        assert!(
            version
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        );
        // Loading the same directory again yields the same version.
        assert_eq!(engine().policy_version(), version);
    }

    #[test]
    fn policy_version_tracks_file_contents() {
        let a = load_temp(
            &repo_policy("aios.cedarschema"),
            &repo_policy("auto.cedar"),
            &repo_policy("confirmable.cedar"),
            &repo_policy("forbid.cedar"),
        )
        .unwrap();
        // A comment that changes no decision still changes the version.
        let b = load_temp(
            &repo_policy("aios.cedarschema"),
            &repo_policy("auto.cedar"),
            &repo_policy("confirmable.cedar"),
            &format!("{}\n// comment\n", repo_policy("forbid.cedar")),
        )
        .unwrap();
        assert_ne!(a.policy_version(), b.policy_version());
    }

    #[test]
    fn evaluate_core_cases() {
        let e = engine();
        let local = principal(Channel::Local, false);
        let remote = principal(Channel::Remote, false);

        // Low, reversible, clean, local: auto at autonomy >= 1.
        let low = tool("x.tool", Risk::Low, true, false, false, false);
        assert_eq!(e.evaluate(&input(&local, &low, 3, false)), Verdict::Allow);
        assert_eq!(e.evaluate(&input(&local, &low, 1, false)), Verdict::Allow);
        assert_eq!(
            e.evaluate(&input(&local, &low, 0, false)),
            Verdict::Confirm(ConfirmLevel::Dialog)
        );
        // Tainted sessions never auto-run; low still confirms as a dialog.
        assert_eq!(
            e.evaluate(&input(&local, &low, 3, true)),
            Verdict::Confirm(ConfirmLevel::Dialog)
        );

        // Medium reversible needs autonomy >= 2 on local, and is capped to 1
        // (confirm) on remote and on writing voice.
        let medium = tool("x.tool", Risk::Medium, true, false, false, false);
        assert_eq!(
            e.evaluate(&input(&local, &medium, 2, false)),
            Verdict::Allow
        );
        assert_eq!(
            e.evaluate(&input(&local, &medium, 1, false)),
            Verdict::Confirm(ConfirmLevel::Dialog)
        );
        assert_eq!(
            e.evaluate(&input(&remote, &medium, 3, false)),
            Verdict::Confirm(ConfirmLevel::Dialog)
        );

        // High is never auto; local/voice confirm, remote/schedule deny.
        let high = tool("x.tool", Risk::High, true, false, false, false);
        assert_eq!(
            e.evaluate(&input(&local, &high, 3, false)),
            Verdict::Confirm(ConfirmLevel::Dialog)
        );
        assert!(matches!(
            e.evaluate(&input(&remote, &high, 3, false)),
            Verdict::Deny(_)
        ));

        // High + irreversible, or high + tainted: dialog with PIN.
        let high_irr = tool("x.tool", Risk::High, false, false, false, false);
        assert_eq!(
            e.evaluate(&input(&local, &high_irr, 3, false)),
            Verdict::Confirm(ConfirmLevel::DialogWithPin)
        );
        assert_eq!(
            e.evaluate(&input(&local, &high, 3, true)),
            Verdict::Confirm(ConfirmLevel::DialogWithPin)
        );
    }

    #[test]
    fn deny_reasons() {
        let e = engine();
        let local = principal(Channel::Local, false);
        let admin = principal(Channel::Local, true);
        let remote = principal(Channel::Remote, false);

        // With the current policy sets every combination is covered by a permit or a
        // forbid, so "no-permit" cannot be produced; autonomy 0 on a low tool confirms.
        let low = tool("x.tool", Risk::Low, true, false, false, false);
        assert_eq!(
            e.evaluate(&input(&local, &low, 0, false)),
            Verdict::Confirm(ConfirmLevel::Dialog)
        );
        // Trusted-core writers are forbidden even for admins.
        let wtc = tool("x.tool", Risk::Low, true, false, false, true);
        assert_eq!(
            e.evaluate(&input(&admin, &wtc, 3, false)),
            Verdict::Deny("forbid-trusted-core".into())
        );
        // System scope without admin; with admin it confirms instead.
        let sys = tool("x.tool", Risk::Low, true, false, true, false);
        assert_eq!(
            e.evaluate(&input(&local, &sys, 3, false)),
            Verdict::Deny("forbid-system-scope-non-admin".into())
        );
        assert_eq!(
            e.evaluate(&input(&admin, &sys, 3, false)),
            Verdict::Confirm(ConfirmLevel::Dialog)
        );
        // High over remote/schedule is denied by both sets.
        let high = tool("x.tool", Risk::High, true, false, false, false);
        assert_eq!(
            e.evaluate(&input(&remote, &high, 3, false)),
            Verdict::Deny("forbid-high-remote-schedule".into())
        );
        // Remote channel may not call schedule tools.
        let schedule = tool("schedule.create", Risk::Medium, true, false, false, false);
        assert_eq!(
            e.evaluate(&input(&remote, &schedule, 3, false)),
            Verdict::Deny("forbid-remote-schedule-tools".into())
        );
        // Several forbids at once: the smallest @id wins, deterministically.
        let all = tool("schedule.create", Risk::High, true, false, true, true);
        assert_eq!(
            e.evaluate(&input(&remote, &all, 3, false)),
            Verdict::Deny("forbid-high-remote-schedule".into())
        );
    }

    #[test]
    fn decide_sets_agree_with_verdicts() {
        let e = engine();
        for (channel, risk, reversible, always, system, wtc, autonomy, tainted, admin) in [
            (
                Channel::Local,
                Risk::Low,
                true,
                false,
                false,
                false,
                3u8,
                false,
                false,
            ),
            (
                Channel::Local,
                Risk::Low,
                true,
                false,
                false,
                false,
                0u8,
                false,
                false,
            ),
            (
                Channel::Local,
                Risk::Low,
                true,
                false,
                false,
                false,
                3u8,
                true,
                false,
            ),
            (
                Channel::Local,
                Risk::Medium,
                true,
                false,
                false,
                false,
                3u8,
                false,
                false,
            ),
            (
                Channel::Local,
                Risk::Medium,
                true,
                true,
                false,
                false,
                3u8,
                false,
                false,
            ),
            (
                Channel::Local,
                Risk::High,
                true,
                false,
                false,
                false,
                3u8,
                false,
                false,
            ),
            (
                Channel::Local,
                Risk::High,
                false,
                false,
                false,
                false,
                3u8,
                false,
                false,
            ),
            (
                Channel::Voice,
                Risk::Low,
                true,
                false,
                false,
                false,
                3u8,
                false,
                false,
            ),
            (
                Channel::Voice,
                Risk::Medium,
                true,
                false,
                false,
                false,
                3u8,
                false,
                false,
            ),
            (
                Channel::Remote,
                Risk::High,
                true,
                false,
                false,
                false,
                3u8,
                false,
                false,
            ),
            (
                Channel::Schedule,
                Risk::Low,
                true,
                false,
                false,
                false,
                3u8,
                false,
                false,
            ),
            (
                Channel::Local,
                Risk::Low,
                true,
                false,
                true,
                false,
                3u8,
                false,
                false,
            ),
            (
                Channel::Local,
                Risk::Low,
                true,
                false,
                true,
                false,
                3u8,
                false,
                true,
            ),
            (
                Channel::Local,
                Risk::Low,
                true,
                false,
                false,
                true,
                3u8,
                false,
                true,
            ),
        ] {
            let t = tool("x.tool", risk, reversible, always, system, wtc);
            let p = principal(channel, admin);
            let i = input(&p, &t, autonomy, tainted);
            let sets = e.decide_sets(&i);
            let ctx = format!(
                "{channel:?} {risk:?} rev={reversible} always={always} sys={system} wtc={wtc} aut={autonomy} tainted={tainted} admin={admin}"
            );
            // Auto allows imply confirmable allows (invariant 4).
            if sets.auto {
                assert!(sets.confirmable, "inv4 {ctx}");
            }
            match e.evaluate(&i) {
                Verdict::Allow => {
                    assert!(sets.auto && sets.confirmable, "sets {ctx}");
                }
                Verdict::Confirm(_) => {
                    assert!(!sets.auto && sets.confirmable, "sets {ctx}");
                }
                Verdict::Deny(reason) => {
                    assert!(!sets.auto && !sets.confirmable, "sets {ctx}");
                    assert!(!reason.is_empty(), "reason {ctx}");
                }
            }
        }
    }

    #[test]
    fn load_fails_on_missing_directory() {
        let err = PolicyEngine::load(Path::new("/nonexistent-aios-policy-dir")).unwrap_err();
        assert!(matches!(err, PolicyError::Io { .. }), "{err:?}");
    }

    #[test]
    fn load_fails_on_missing_policy_file() {
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "aios-policy-tests-missing-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["aios.cedarschema", "auto.cedar", "confirmable.cedar"] {
            std::fs::write(dir.join(name), repo_policy(name)).unwrap();
        }
        let err = PolicyEngine::load(&dir).unwrap_err();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(matches!(err, PolicyError::Io { .. }), "{err:?}");
    }

    #[test]
    fn load_fails_on_policy_syntax_error() {
        let broken_auto =
            "permit (principal is Agent, action == Action::\"invoke\", resource is Tool) when {";
        let err = load_temp(
            &repo_policy("aios.cedarschema"),
            broken_auto,
            &repo_policy("confirmable.cedar"),
            &repo_policy("forbid.cedar"),
        )
        .unwrap_err();
        assert!(
            matches!(err, PolicyError::PolicyParse { set: "auto", .. }),
            "{err:?}"
        );
    }

    #[test]
    fn load_fails_on_schema_violation() {
        // Type error: the schema declares risk as String, not Long.
        let invalid_auto = concat!(
            "permit (principal is Agent, action == Action::\"invoke\", resource is Tool)\n",
            "when { resource.risk == 3 };\n",
        );
        let err = load_temp(
            &repo_policy("aios.cedarschema"),
            invalid_auto,
            &repo_policy("confirmable.cedar"),
            &repo_policy("forbid.cedar"),
        )
        .unwrap_err();
        assert!(
            matches!(err, PolicyError::Validation { set: "auto", .. }),
            "{err:?}"
        );
    }

    #[test]
    fn load_fails_on_schema_syntax_error() {
        let broken_schema = "entity Agent = { uid: Long,, };";
        let err = load_temp(
            broken_schema,
            &repo_policy("auto.cedar"),
            &repo_policy("confirmable.cedar"),
            &repo_policy("forbid.cedar"),
        )
        .unwrap_err();
        assert!(matches!(err, PolicyError::SchemaParse(_)), "{err:?}");
    }
}
