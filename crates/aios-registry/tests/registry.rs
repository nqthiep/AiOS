//! The mandatory tests of the registry (docs/m0/registry.md).

use std::fs;
use std::path::{Path, PathBuf};

use aios_registry::{KeyPair, generate_keypair, sign_file};
use aios_registry::{Registry, RegistryConfig, TrustLevel};
use aios_types::{Privacy, Provenance, Risk};
use base64::Engine as _;
use tempfile::TempDir;

/// A registry tree with a core, a vendor and a local directory.
struct Fixture {
    _root: TempDir,
    core: PathBuf,
    vendor: PathBuf,
    local: PathBuf,
    trust: PathBuf,
    key: KeyPair,
}

impl Fixture {
    fn new() -> Self {
        let root = TempDir::new().expect("temporary directory");
        let core = root.path().join("core");
        let vendor = root.path().join("vendor");
        let local = root.path().join("local");
        let trust = root.path().join("trust");
        for dir in [
            &core,
            &vendor,
            &local,
            &trust.join("core"),
            &trust.join("vendors"),
        ] {
            fs::create_dir_all(dir).expect("create the directory");
        }
        Self {
            _root: root,
            core,
            vendor,
            local,
            trust,
            key: generate_keypair(),
        }
    }

    /// Writes a plugin directory with a signed manifest.
    fn write_plugin(&self, directory: &Path, name: &str, manifest: &str) -> PathBuf {
        let plugin_dir = directory.join(name);
        fs::create_dir_all(&plugin_dir).expect("create the plugin directory");
        let manifest_path = plugin_dir.join("plugin.toml");
        fs::write(&manifest_path, manifest).expect("write the manifest");
        sign_file(&manifest_path, self.key.secret()).expect("sign the manifest");
        plugin_dir
    }

    /// Writes a plugin into the core directory.
    fn write_core_plugin(&self, name: &str, manifest: &str) -> PathBuf {
        self.write_plugin(&self.core, name, manifest)
    }

    /// Writes a plugin into the vendor directory.
    fn write_vendor_plugin(&self, name: &str, manifest: &str) -> PathBuf {
        self.write_plugin(&self.vendor, name, manifest)
    }

    /// Writes a plugin into the local (community) directory.
    fn write_local_plugin(&self, name: &str, manifest: &str) -> PathBuf {
        self.write_plugin(&self.local, name, manifest)
    }

    /// Trusts the key of the fixture for the core level.
    fn trust_core_key(&self) {
        let dir = self.trust.join("core");
        fs::create_dir_all(&dir).expect("create the trust directory");
        fs::write(dir.join("aios.pub"), self.key.public().to_base64())
            .expect("write the public key");
    }

    /// Trusts the key of the fixture for the vendor level.
    fn trust_vendor_key(&self) {
        let dir = self.trust.join("vendors");
        fs::create_dir_all(&dir).expect("create the trust directory");
        fs::write(dir.join("vendor.pub"), self.key.public().to_base64())
            .expect("write the public key");
    }

    /// Trusts the key of the fixture for the community level.
    fn trust_local_key(&self) {
        fs::write(self.trust.join("local.pub"), self.key.public().to_base64())
            .expect("write the public key");
    }

    /// Writes the revocation list.
    fn revoke(&self, keys: &[&str], plugins: &str) {
        let keys = keys
            .iter()
            .map(|key| format!("\"{key}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let list = format!("{{\"keys\": [{keys}], \"plugins\": [{plugins}]}}");
        fs::write(self.trust.join("revoked.json"), list).expect("write the revocation list");
    }

    /// Loads the registry.
    fn load(&self) -> (Registry, aios_registry::LoadReport) {
        Registry::load(&RegistryConfig {
            core_dir: self.core.clone(),
            vendor_dir: self.vendor.clone(),
            local_dir: self.local.clone(),
            trust_dir: self.trust.clone(),
        })
    }
}

/// Builds a manifest with the given capabilities and tools.
fn manifest(id: &str, capabilities: &[&str], tools: &str) -> String {
    let capabilities = capabilities
        .iter()
        .map(|capability| format!("\"{capability}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        r#"id = "{id}"
version = "1.0.0"
api = "aios:plugin@0.1"
provides = ["tool"]
runtime = "mcp"
entry = ["/usr/bin/env", "python3", "-I", "server.py"]
capabilities = [{capabilities}]

[sandbox]
ro_paths = ["$WORKSPACE"]
rw_paths = ["$WORKSPACE/notes"]
landlock_required = true

{tools}"#
    )
}

/// Builds a tool block.
fn tool(name: &str, risk: &str) -> String {
    format!(
        r#"[[tools]]
name = "{name}"
description = "a test tool"
risk = "{risk}"
reversible = true
input_schema = {{ type = "object" }}
"#
    )
}

#[test]
fn a_valid_plugin_is_loaded() {
    let fixture = Fixture::new();
    fixture.trust_core_key();
    fixture.write_core_plugin(
        "fs",
        &manifest(
            "aios.fs",
            &["fs.read:$WORKSPACE", "fs.write:$WORKSPACE"],
            &tool("fs.search", "low"),
        ),
    );

    let (registry, report) = fixture.load();

    assert!(report.rejected.is_empty(), "{:?}", report.rejected);
    assert_eq!(report.loaded, vec!["aios.fs".to_string()]);

    let spec = registry.tool("fs.search").expect("the tool is registered");
    assert_eq!(spec.plugin_id, "aios.fs");
    assert_eq!(spec.risk, Risk::Low);
    assert!(spec.reversible);
    assert!(!spec.always_confirm);
    assert!(!spec.system_scope);
    assert_eq!(spec.result_privacy, Privacy::Personal);
    assert_eq!(spec.result_provenance, Provenance::Observed);
    assert_eq!(spec.timeout_ms, 30_000);
    assert_eq!(spec.entry.len(), 4);
    assert_eq!(spec.sandbox.ro_paths, vec!["$WORKSPACE".to_string()]);
    assert!(spec.sandbox.landlock_required);

    let plugin = registry
        .plugin("aios.fs")
        .expect("the plugin is registered");
    assert_eq!(plugin.trust, TrustLevel::Core);
    assert_eq!(plugin.tools, vec!["fs.search".to_string()]);
}

#[test]
fn a_manifest_with_one_changed_byte_is_rejected() {
    let fixture = Fixture::new();
    fixture.trust_core_key();
    let plugin_dir = fixture.write_core_plugin(
        "fs",
        &manifest(
            "aios.fs",
            &["fs.read:$WORKSPACE"],
            &tool("fs.search", "low"),
        ),
    );

    let manifest_path = plugin_dir.join("plugin.toml");
    let text = fs::read_to_string(&manifest_path).expect("read the manifest");
    let tampered = text.replace("1.0.0", "1.0.1");
    assert_ne!(text, tampered, "the manifest was changed");
    fs::write(&manifest_path, tampered).expect("write the tampered manifest");

    let (registry, report) = fixture.load();

    assert!(registry.tool("fs.search").is_none());
    assert_eq!(report.rejected.len(), 1);
    assert!(
        report.rejected[0].1.contains("signature"),
        "unexpected reason: {:?}",
        report.rejected[0].1
    );
}

#[test]
fn a_revoked_key_is_rejected() {
    let fixture = Fixture::new();
    fixture.trust_core_key();
    fixture.write_core_plugin(
        "fs",
        &manifest(
            "aios.fs",
            &["fs.read:$WORKSPACE"],
            &tool("fs.search", "low"),
        ),
    );
    fixture.revoke(&[&fixture.key.public().to_base64()], "");

    let (registry, report) = fixture.load();

    assert!(registry.tool("fs.search").is_none());
    assert_eq!(report.rejected.len(), 1);
    assert!(
        report.rejected[0].1.contains("revoked"),
        "unexpected reason: {:?}",
        report.rejected[0].1
    );
}

#[test]
fn a_community_plugin_cannot_lower_a_risk_with_risk_overrides() {
    let fixture = Fixture::new();
    fixture.trust_local_key();
    let manifest = manifest(
        "aios.fs",
        &["net:flathub"],
        &format!(
            "{}\n[risk_overrides]\n\"net:flathub\" = \"low\"\n\"fs.search\" = \"low\"\n",
            tool("fs.search", "low")
        ),
    );
    fixture.write_local_plugin("fs", &manifest);

    let (registry, report) = fixture.load();

    assert!(report.rejected.is_empty(), "{:?}", report.rejected);
    let spec = registry.tool("fs.search").expect("the tool is registered");
    assert_eq!(
        spec.risk,
        Risk::Medium,
        "the risk override of a community plugin must not lower the risk"
    );
}

#[test]
fn an_unknown_capability_raises_the_risk_to_high() {
    let fixture = Fixture::new();
    fixture.trust_core_key();
    fixture.write_core_plugin(
        "fs",
        &manifest("aios.fs", &["quantum.break"], &tool("fs.search", "low")),
    );

    let (registry, report) = fixture.load();

    assert!(report.rejected.is_empty(), "{:?}", report.rejected);
    let spec = registry.tool("fs.search").expect("the tool is registered");
    assert_eq!(spec.risk, Risk::High);
}

#[test]
fn a_write_outside_of_the_workspace_raises_the_risk_to_medium() {
    let fixture = Fixture::new();
    fixture.trust_core_key();
    fixture.write_core_plugin(
        "fs",
        &manifest(
            "aios.fs",
            &["fs.write:/etc/hosts"],
            &tool("fs.write_note", "low"),
        ),
    );

    let (registry, report) = fixture.load();

    assert!(report.rejected.is_empty(), "{:?}", report.rejected);
    let spec = registry
        .tool("fs.write_note")
        .expect("the tool is registered");
    assert_eq!(spec.risk, Risk::Medium);
}

#[test]
fn a_manifest_with_an_unknown_field_is_rejected() {
    let fixture = Fixture::new();
    fixture.trust_core_key();
    let manifest = format!(
        "{}\nnot_a_known_field = true\n",
        manifest(
            "aios.fs",
            &["fs.read:$WORKSPACE"],
            &tool("fs.search", "low")
        )
    );
    fixture.write_core_plugin("fs", &manifest);

    let (registry, report) = fixture.load();

    assert!(registry.tool("fs.search").is_none());
    assert_eq!(report.rejected.len(), 1);
    assert!(
        report.rejected[0].1.contains("not_a_known_field"),
        "unexpected reason: {:?}",
        report.rejected[0].1
    );
}

#[test]
fn a_tool_outside_of_the_namespace_of_its_plugin_is_dropped() {
    let fixture = Fixture::new();
    fixture.trust_core_key();
    let tools = format!("{}{}", tool("fs.search", "low"), tool("sys.info", "low"));
    fixture.write_core_plugin("fs", &manifest("aios.fs", &["fs.read:$WORKSPACE"], &tools));

    let (registry, report) = fixture.load();

    assert!(report.rejected.is_empty(), "{:?}", report.rejected);
    assert!(registry.tool("fs.search").is_some());
    assert!(
        registry.tool("sys.info").is_none(),
        "a tool outside of the namespace of the plugin has to be dropped"
    );
    let plugin = registry
        .plugin("aios.fs")
        .expect("the plugin is registered");
    assert_eq!(plugin.tools, vec!["fs.search".to_string()]);
}

#[test]
fn a_plugin_id_in_two_directories_keeps_the_core_plugin() {
    let fixture = Fixture::new();
    fixture.trust_core_key();
    fixture.trust_local_key();
    fixture.write_core_plugin(
        "fs-core",
        &manifest(
            "aios.fs",
            &["fs.read:$WORKSPACE"],
            &tool("fs.search", "low"),
        ),
    );
    let local_manifest = manifest("aios.fs", &["net:flathub"], &tool("fs.search", "high"))
        .replace("version = \"1.0.0\"", "version = \"9.9.9\"");
    fixture.write_local_plugin("fs-local", &local_manifest);

    let (registry, report) = fixture.load();

    assert!(report.rejected.is_empty(), "{:?}", report.rejected);
    let plugin = registry
        .plugin("aios.fs")
        .expect("the plugin is registered");
    assert_eq!(plugin.trust, TrustLevel::Core);
    assert_eq!(plugin.version, "1.0.0");
    let spec = registry.tool("fs.search").expect("the tool is registered");
    assert_eq!(spec.risk, Risk::Low);
}

#[test]
fn a_plugin_without_a_signature_is_rejected() {
    let fixture = Fixture::new();
    fixture.trust_core_key();
    let plugin_dir = fixture.core.join("fs");
    fs::create_dir_all(&plugin_dir).expect("create the plugin directory");
    fs::write(
        plugin_dir.join("plugin.toml"),
        manifest(
            "aios.fs",
            &["fs.read:$WORKSPACE"],
            &tool("fs.search", "low"),
        ),
    )
    .expect("write the manifest");

    let (registry, report) = fixture.load();

    assert!(registry.tool("fs.search").is_none());
    assert_eq!(report.rejected.len(), 1);
    assert!(
        report.rejected[0].1.contains("signature"),
        "unexpected reason: {:?}",
        report.rejected[0].1
    );
}

#[test]
fn a_plugin_without_a_trusted_key_is_rejected() {
    let fixture = Fixture::new();
    fixture.write_core_plugin(
        "fs",
        &manifest(
            "aios.fs",
            &["fs.read:$WORKSPACE"],
            &tool("fs.search", "low"),
        ),
    );

    let (registry, report) = fixture.load();

    assert!(registry.tool("fs.search").is_none());
    assert_eq!(report.rejected.len(), 1);
    assert!(
        report.rejected[0].1.contains("public key"),
        "unexpected reason: {:?}",
        report.rejected[0].1
    );
}

#[test]
fn a_revoked_plugin_is_rejected() {
    let fixture = Fixture::new();
    fixture.trust_core_key();
    fixture.write_core_plugin(
        "fs",
        &manifest(
            "aios.fs",
            &["fs.read:$WORKSPACE"],
            &tool("fs.search", "low"),
        ),
    );
    fixture.revoke(&[], "{\"id\": \"aios.fs\", \"version\": \"1.0.0\"}");

    let (registry, report) = fixture.load();

    assert!(registry.tool("fs.search").is_none());
    assert_eq!(report.rejected.len(), 1);
    assert!(
        report.rejected[0].1.contains("revoked"),
        "unexpected reason: {:?}",
        report.rejected[0].1
    );
}

#[test]
fn a_plugin_with_an_unsupported_api_is_rejected() {
    let fixture = Fixture::new();
    fixture.trust_core_key();
    let manifest = manifest(
        "aios.fs",
        &["fs.read:$WORKSPACE"],
        &tool("fs.search", "low"),
    )
    .replace("api = \"aios:plugin@0.1\"", "api = \"aios:plugin@0.2\"");
    fixture.write_core_plugin("fs", &manifest);

    let (registry, report) = fixture.load();

    assert!(registry.tool("fs.search").is_none());
    assert_eq!(report.rejected.len(), 1);
    assert!(
        report.rejected[0].1.contains("api"),
        "unexpected reason: {:?}",
        report.rejected[0].1
    );
}

#[test]
fn a_tool_with_an_invalid_input_schema_is_rejected() {
    let fixture = Fixture::new();
    fixture.trust_core_key();
    let tools = r#"[[tools]]
name = "fs.search"
description = "a test tool"
risk = "low"
input_schema = { type = "not_a_type" }
"#;
    fixture.write_core_plugin("fs", &manifest("aios.fs", &["fs.read:$WORKSPACE"], tools));

    let (registry, report) = fixture.load();

    assert!(registry.tool("fs.search").is_none());
    assert_eq!(report.rejected.len(), 1);
    assert!(
        report.rejected[0].1.contains("JSON Schema"),
        "unexpected reason: {:?}",
        report.rejected[0].1
    );
}

#[test]
fn a_community_plugin_keeps_always_confirm_and_system_scope() {
    let fixture = Fixture::new();
    fixture.trust_local_key();
    let tools = r#"[[tools]]
name = "fs.write_note"
description = "a test tool"
risk = "medium"
always_confirm = true
system_scope = true
"#;
    fixture.write_local_plugin("fs", &manifest("aios.fs", &["fs.write:$WORKSPACE"], tools));

    let (registry, report) = fixture.load();

    assert!(report.rejected.is_empty(), "{:?}", report.rejected);
    let spec = registry
        .tool("fs.write_note")
        .expect("the tool is registered");
    assert!(spec.always_confirm);
    assert!(spec.system_scope);
}

#[test]
fn a_vendor_plugin_may_raise_a_risk_with_risk_overrides() {
    let fixture = Fixture::new();
    fixture.trust_vendor_key();
    let manifest = manifest(
        "aios.vendor",
        &["net:flathub"],
        &format!(
            "{}\n[risk_overrides]\n\"net:flathub\" = \"high\"\n",
            tool("vendor.install", "low")
        ),
    );
    fixture.write_vendor_plugin("vendor", &manifest);

    let (registry, report) = fixture.load();

    assert!(report.rejected.is_empty(), "{:?}", report.rejected);
    let spec = registry
        .tool("vendor.install")
        .expect("the tool is registered");
    assert_eq!(spec.risk, Risk::High);
}

#[test]
fn the_signature_covers_the_exact_bytes_of_the_manifest() {
    let fixture = Fixture::new();
    fixture.trust_core_key();
    let plugin_dir = fixture.write_core_plugin(
        "fs",
        &manifest(
            "aios.fs",
            &["fs.read:$WORKSPACE"],
            &tool("fs.search", "low"),
        ),
    );

    let manifest_path = plugin_dir.join("plugin.toml");
    let bytes = fs::read(&manifest_path).expect("read the manifest");
    let signature_text =
        fs::read_to_string(plugin_dir.join("plugin.toml.sig")).expect("read the signature");
    let signature_bytes = base64::engine::general_purpose::STANDARD
        .decode(signature_text.trim())
        .expect("the signature is base64");
    assert_eq!(signature_bytes.len(), 64);

    let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(fixture.key.public().as_bytes())
        .expect("a valid public key");
    let signature =
        ed25519_dalek::Signature::from_slice(&signature_bytes).expect("a valid signature");
    verifying_key
        .verify_strict(&bytes, &signature)
        .expect("the signature covers the exact bytes of the manifest");
}

#[test]
fn an_input_schema_with_an_internal_reference_is_accepted() {
    let fixture = Fixture::new();
    fixture.trust_core_key();
    let tools = r##"[[tools]]
name = "fs.search"
description = "a test tool"
risk = "low"
input_schema = '{ "type": "object", "properties": { "name": { "$ref": "#/$defs/name" } }, "$defs": { "name": { "type": "string" } } }'
"##;
    fixture.write_core_plugin("fs", &manifest("aios.fs", &["fs.read:$WORKSPACE"], tools));

    let (registry, report) = fixture.load();

    assert!(report.rejected.is_empty(), "{:?}", report.rejected);
    assert!(registry.tool("fs.search").is_some());
}

#[test]
fn an_input_schema_with_an_external_reference_is_rejected() {
    let fixture = Fixture::new();
    fixture.trust_core_key();
    let tools = r##"[[tools]]
name = "fs.search"
description = "a test tool"
risk = "low"
input_schema = '{ "$ref": "https://example.invalid/schema.json" }'
"##;
    fixture.write_core_plugin("fs", &manifest("aios.fs", &["fs.read:$WORKSPACE"], tools));

    let (registry, report) = fixture.load();

    assert!(registry.tool("fs.search").is_none());
    assert_eq!(report.rejected.len(), 1);
    assert!(
        report.rejected[0].1.contains("JSON Schema"),
        "unexpected reason: {:?}",
        report.rejected[0].1
    );
}

#[test]
fn a_broken_revocation_list_stops_every_plugin_from_being_loaded() {
    let fixture = Fixture::new();
    fixture.trust_core_key();
    fixture.write_core_plugin(
        "fs",
        &manifest(
            "aios.fs",
            &["fs.read:$WORKSPACE"],
            &tool("fs.search", "low"),
        ),
    );
    fs::write(fixture.trust.join("revoked.json"), "{ not json").expect("write the list");

    let (registry, report) = fixture.load();

    assert!(registry.tool("fs.search").is_none());
    assert_eq!(report.rejected.len(), 1);
    assert!(
        report.rejected[0].1.contains("trust store"),
        "unexpected reason: {:?}",
        report.rejected[0].1
    );
}
