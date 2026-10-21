//! Route selection (docs/m0/gateway.md "Chọn route", design.md §5.4).
//!
//! ```text
//! privacy = max(session label (or service key label), client-declared label if any)
//! key without a known session -> privacy = secret
//! candidates = routes matching the logical model
//! drop cloud if: privacy == secret (second check in code, independent of config)
//!             or privacy > min(client max_cloud_privacy, user max_cloud_privacy)
//!             or network offline, or user budget exhausted, or user has no cloud config
//! try in order; on error go to the next; none left -> 409 no_route_privacy or 503 upstream_unavailable
//! ```

use aios_types::Privacy;

use crate::config::{ClientConfig, GatewayConfig, KeyScope, ModelConfig, UserConfig};
use crate::error::GatewayError;

/// Context of a request authenticated with a virtual key.
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// UID owning the virtual key.
    pub uid: u32,
    /// Client name (looked up in `clients`).
    pub client: String,
    /// Key scope.
    pub scope: KeyScope,
    /// Session bound to the key (`None` for service keys).
    pub session_id: Option<String>,
}

/// Conditions under which a user may use cloud routes.
#[derive(Debug, Clone, Default)]
pub struct CloudGate {
    /// The user has cloud configured (usable key, budget left).
    pub configured: bool,
    /// Privacy ceiling: `None` = no cloud.
    pub ceiling: Option<Privacy>,
    /// Provider chosen by the user.
    pub provider: Option<String>,
}

/// A route candidate (one concrete upstream).
#[derive(Debug, Clone)]
pub struct Candidate {
    /// Upstream name in `models`.
    pub name: String,
    /// Whether this upstream is cloud.
    pub is_cloud: bool,
    /// Upstream config.
    pub model: ModelConfig,
}

/// Route plan for one request.
#[derive(Debug, Clone)]
pub struct RoutePlan {
    /// Effective privacy label.
    pub privacy: Privacy,
    /// Candidates in try order.
    pub candidates: Vec<Candidate>,
    /// Why each candidate was dropped (for logs; contains no secrets).
    pub excluded: Vec<String>,
}

/// Effective privacy label of a request.
///
/// - Service key: always `secret` (mixed-label data).
/// - Session key: the session label (`public` when the session is created).
/// - Key whose session is unknown: `secret` (unlabeled = strictest).
/// - A client-declared label can only raise, never lower.
pub fn effective_privacy(
    scope: KeyScope,
    session_label: Option<Privacy>,
    declared: Option<Privacy>,
) -> Privacy {
    let base = match scope {
        KeyScope::Service => Privacy::Secret,
        KeyScope::Session | KeyScope::Plugin => session_label.unwrap_or(Privacy::Secret),
    };
    match declared {
        Some(declared) => base.max(declared),
        None => base,
    }
}

/// Effective cloud ceiling = min(client ceiling, user ceiling).
///
/// `None` means no cloud: the client has `cloud: false`, the client is not
/// configured, or the user has no cloud config.
pub fn cloud_ceiling(client: Option<&ClientConfig>, user: Option<&UserConfig>) -> Option<Privacy> {
    let client_ceiling = client.and_then(ClientConfig::cloud_ceiling)?;
    let user_ceiling = user
        .and_then(|u| u.cloud.as_ref())
        .and_then(|c| c.max_cloud_privacy)?;
    Some(client_ceiling.min(user_ceiling))
}

/// Select route candidates for a request.
///
/// `is_reachable` returns true when a cloud provider is reachable (offline
/// drops cloud routes).
///
/// Returns `Err(409 no_route_privacy)` when no candidate is left.
pub fn select_candidates(
    config: &GatewayConfig,
    logical_model: &str,
    privacy: Privacy,
    gate: &CloudGate,
    is_reachable: impl Fn(&str) -> bool,
) -> Result<RoutePlan, GatewayError> {
    let Some(route) = config.route_for(logical_model) else {
        return Err(GatewayError::model_not_found(format!(
            "unknown logical model '{logical_model}'"
        )));
    };

    let mut candidates = Vec::new();
    let mut excluded = Vec::new();

    for name in &route.use_models {
        let Some(model) = config.models.get(name) else {
            excluded.push(format!("{name}: not defined in models"));
            continue;
        };
        if !model.kind.is_cloud() {
            candidates.push(Candidate {
                name: name.clone(),
                is_cloud: false,
                model: model.clone(),
            });
            continue;
        }
        // Second check, in code: `secret` NEVER goes to cloud, whatever the
        // route, client or user configuration says (design.md §5.4).
        if privacy == Privacy::Secret {
            excluded.push(format!("{name}: secret data never goes to cloud"));
            continue;
        }
        if !gate.configured {
            excluded.push(format!("{name}: cloud not configured for this user"));
            continue;
        }
        let Some(ceiling) = gate.ceiling else {
            excluded.push(format!("{name}: no cloud privacy ceiling"));
            continue;
        };
        if privacy > ceiling {
            excluded.push(format!("{name}: privacy above cloud ceiling"));
            continue;
        }
        if let (Some(provider), Some(user_provider)) =
            (model.provider.as_deref(), gate.provider.as_deref())
            && provider != user_provider
        {
            excluded.push(format!("{name}: provider mismatch"));
            continue;
        }
        if !is_reachable(model.base_url().unwrap_or_default()) {
            excluded.push(format!("{name}: network offline"));
            continue;
        }
        candidates.push(Candidate {
            name: name.clone(),
            is_cloud: true,
            model: model.clone(),
        });
    }

    if candidates.is_empty() {
        return Err(GatewayError::no_route(format!(
            "no route for model '{logical_model}' with privacy '{privacy:?}'{}",
            if excluded.is_empty() {
                String::new()
            } else {
                format!(" ({})", excluded.join("; "))
            }
        )));
    }

    Ok(RoutePlan {
        privacy,
        candidates,
        excluded,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> GatewayConfig {
        serde_yaml_ng::from_str(
            r#"
models:
  cloud: { kind: openai_compatible, base_url: "http://cloud/v1" }
  local: { kind: local, endpoint: "http://local/v1" }
routes:
  - { model: s2, use: [cloud, local] }
"#,
        )
        .expect("parse config")
    }

    fn names(plan: &RoutePlan) -> Vec<&str> {
        plan.candidates.iter().map(|c| c.name.as_str()).collect()
    }

    /// Configuration fully allows `secret` on cloud; the code check alone must block it.
    #[test]
    fn secret_is_blocked_in_code_even_if_config_allows() {
        let gate = CloudGate {
            configured: true,
            ceiling: Some(Privacy::Secret),
            provider: None,
        };
        let plan = select_candidates(&config(), "s2", Privacy::Secret, &gate, |_| true)
            .expect("local route");
        assert_eq!(names(&plan), ["local"]);
        let plan =
            select_candidates(&config(), "s2", Privacy::Personal, &gate, |_| true).expect("routes");
        assert_eq!(names(&plan), ["cloud", "local"]);
    }

    #[test]
    fn ceiling_offline_and_unconfigured_drop_cloud() {
        let gate = CloudGate {
            configured: true,
            ceiling: Some(Privacy::Public),
            provider: None,
        };
        let plan = select_candidates(&config(), "s2", Privacy::Personal, &gate, |_| true)
            .expect("local route");
        assert_eq!(names(&plan), ["local"]);
        let plan = select_candidates(&config(), "s2", Privacy::Public, &gate, |_| false)
            .expect("local route");
        assert_eq!(names(&plan), ["local"]);
        let gate = CloudGate {
            configured: false,
            ..gate
        };
        let plan = select_candidates(&config(), "s2", Privacy::Public, &gate, |_| true)
            .expect("local route");
        assert_eq!(names(&plan), ["local"]);
    }

    #[test]
    fn declared_label_only_raises_and_unknown_session_is_secret() {
        use KeyScope::*;
        assert_eq!(
            effective_privacy(Session, Some(Privacy::Personal), Some(Privacy::Public)),
            Privacy::Personal
        );
        assert_eq!(
            effective_privacy(Session, Some(Privacy::Public), Some(Privacy::Secret)),
            Privacy::Secret
        );
        assert_eq!(effective_privacy(Session, None, None), Privacy::Secret);
        assert_eq!(
            effective_privacy(Service, None, Some(Privacy::Public)),
            Privacy::Secret
        );
    }
}
