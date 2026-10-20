//! The effective risk of a tool (docs/m0/registry.md, design.md §15.2).
//!
//! The effective risk of a tool is
//! `max(risk declared by the tool, risk inferred from the capabilities of the
//! plugin)`. A manifest may only raise a risk, never lower one:
//!
//! * `risk_overrides` are honoured for `core` and `vendor` plugins only, and
//!   an override that is lower than the inferred risk is ignored;
//! * a capability that matches none of the known patterns fails closed to
//!   `high`.

use std::collections::{BTreeMap, BTreeSet};

use aios_types::Risk;

use crate::manifest::Manifest;
use crate::trust::TrustLevel;

/// The minimum risk implied by a single capability string.
///
/// | Capability | Minimum risk |
/// | --- | --- |
/// | `fs.read:<path>` | `low` |
/// | `fs.write:$WORKSPACE[/...]` | `low` |
/// | `fs.write:<path outside the workspace>` | `medium` |
/// | `net:<name>` | `medium` |
/// | `dbus:<name>` | `medium` |
/// | `priv.escalate:<helper>` | `high` |
/// | `fs.delete:<path>` | `high` |
/// | `send:<channel>` | `high` |
/// | anything else | `high` (fail closed) |
pub fn capability_risk(capability: &str) -> Risk {
    let Some((domain, argument)) = capability.split_once(':') else {
        return Risk::High;
    };
    if argument.is_empty() {
        return Risk::High;
    }
    match domain {
        "fs.read" => Risk::Low,
        "fs.write" => {
            if is_workspace_path(argument) {
                Risk::Low
            } else {
                Risk::Medium
            }
        }
        // Deleting a file is never covered by a lower rule.
        "fs.delete" => Risk::High,
        "net" => Risk::Medium,
        "dbus" => Risk::Medium,
        "priv.escalate" => Risk::High,
        "send" => Risk::High,
        _ => Risk::High,
    }
}

/// Whether a path stays inside the workspace of the user.
fn is_workspace_path(path: &str) -> bool {
    path == "$WORKSPACE" || path.starts_with("$WORKSPACE/")
}

/// The risk inferred from the capabilities of a plugin.
pub fn inferred_risk(capabilities: &[String]) -> Risk {
    capabilities
        .iter()
        .map(|capability| capability_risk(capability))
        .max()
        .unwrap_or(Risk::Low)
}

/// The result of applying the `risk_overrides` of a manifest.
#[derive(Debug, Clone)]
pub struct RiskResolution {
    /// The risk inferred from the capabilities of the plugin.
    pub inferred: Risk,
    /// The risk raised for a single tool by name.
    pub tool_overrides: BTreeMap<String, Risk>,
}

impl Default for RiskResolution {
    fn default() -> Self {
        Self {
            inferred: Risk::Low,
            tool_overrides: BTreeMap::new(),
        }
    }
}

impl RiskResolution {
    /// The effective risk of a tool with the given declared risk.
    pub fn effective_risk(&self, tool_name: &str, declared: Risk) -> Risk {
        let mut effective = declared.max(self.inferred);
        if let Some(&raised) = self.tool_overrides.get(tool_name) {
            if raised > effective {
                effective = raised;
            } else {
                tracing::warn!(
                    tool = tool_name,
                    declared = ?raised,
                    effective = ?effective,
                    "the risk override of a tool is not higher than its effective risk and is ignored"
                );
            }
        }
        effective
    }
}

/// Applies the `risk_overrides` of a manifest to the capabilities of a plugin.
///
/// `risk_overrides` is only honoured for `core` and `vendor` plugins. A key of
/// the map is either a capability of the plugin (the inferred risk of that
/// capability is raised) or the name of a tool of the plugin (the effective
/// risk of that tool is raised).
pub fn resolve(manifest: &Manifest, trust: TrustLevel) -> RiskResolution {
    let allowed = matches!(trust, TrustLevel::Core | TrustLevel::Vendor);
    if !allowed && !manifest.risk_overrides.is_empty() {
        tracing::warn!(
            plugin = %manifest.id,
            trust = %trust,
            "risk_overrides are ignored for a plugin that is not core or vendor"
        );
    }

    let mut resolution = RiskResolution::default();
    let mut used: BTreeSet<&str> = BTreeSet::new();

    for capability in &manifest.capabilities {
        let base = capability_risk(capability);
        let mut effective = base;
        if allowed {
            match manifest.risk_overrides.get(capability) {
                Some(&raised) if raised > base => {
                    effective = raised;
                    used.insert(capability.as_str());
                }
                Some(&lowered) => {
                    tracing::warn!(
                        plugin = %manifest.id,
                        capability = %capability,
                        declared = ?lowered,
                        inferred = ?base,
                        "the risk override of a capability is not higher than its inferred risk and is ignored"
                    );
                    used.insert(capability.as_str());
                }
                None => {}
            }
        }
        resolution.inferred = resolution.inferred.max(effective);
    }

    if allowed {
        let tool_names: BTreeSet<&str> = manifest
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect();
        for (key, risk) in &manifest.risk_overrides {
            if tool_names.contains(key.as_str()) {
                resolution.tool_overrides.insert(key.clone(), *risk);
                used.insert(key.as_str());
            }
        }
        for key in manifest.risk_overrides.keys() {
            if !used.contains(key.as_str()) {
                tracing::warn!(
                    plugin = %manifest.id,
                    key = %key,
                    "a risk override matches neither a capability nor a tool of the plugin and is ignored"
                );
            }
        }
    }

    resolution
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_capabilities() {
        assert_eq!(capability_risk("fs.read:/proc"), Risk::Low);
        assert_eq!(capability_risk("fs.read:$WORKSPACE"), Risk::Low);
        assert_eq!(capability_risk("fs.write:$WORKSPACE"), Risk::Low);
        assert_eq!(capability_risk("fs.write:$WORKSPACE/notes"), Risk::Low);
        assert_eq!(capability_risk("fs.write:/etc/hosts"), Risk::Medium);
        assert_eq!(capability_risk("net:flathub"), Risk::Medium);
        assert_eq!(
            capability_risk("dbus:org.freedesktop.NetworkManager"),
            Risk::Medium
        );
        assert_eq!(capability_risk("priv.escalate:aios-actuator"), Risk::High);
        assert_eq!(capability_risk("fs.delete:/etc/hosts"), Risk::High);
        assert_eq!(capability_risk("send:email"), Risk::High);
    }

    #[test]
    fn unknown_capabilities_fail_closed() {
        assert_eq!(capability_risk("quantum.break"), Risk::High);
        assert_eq!(capability_risk("fs.write"), Risk::High);
        assert_eq!(capability_risk("fs.write:"), Risk::High);
        assert_eq!(capability_risk("net:"), Risk::High);
        assert_eq!(capability_risk(""), Risk::High);
    }

    #[test]
    fn workspace_paths_are_exact() {
        assert!(is_workspace_path("$WORKSPACE"));
        assert!(is_workspace_path("$WORKSPACE/notes/a.md"));
        assert!(!is_workspace_path("$WORKSPACE2/a.md"));
        assert!(!is_workspace_path("notes/a.md"));
        assert!(!is_workspace_path("/home/user/notes"));
    }
}
