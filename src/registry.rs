use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

/// Service registry loaded from `registry.toml`.
#[derive(Debug, Deserialize)]
pub struct Registry {
    #[serde(rename = "services")]
    pub services: Vec<Service>,
}

/// A single service entry in the registry.
#[derive(Debug, Clone, Deserialize)]
pub struct Service {
    pub id: String,
    pub repo: String,
    pub project_dir: String,
    #[serde(rename = "crate")]
    pub crate_name: String,
    pub domain: String,
    pub capabilities: Vec<String>,
}

impl Registry {
    /// Load registry from a TOML file.
    pub fn load(path: &Path) -> Result<Self> {
        let content =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&content).with_context(|| format!("parsing {}", path.display()))
    }

    /// Look up a service by its ID.
    pub fn find_by_id(&self, id: &str) -> Option<&Service> {
        self.services.iter().find(|s| s.id == id)
    }

    /// Return all services in the given domain.
    pub fn find_by_domain(&self, domain: &str) -> Vec<&Service> {
        self.services.iter().filter(|s| s.domain == domain).collect()
    }

    /// Return all services that expose the given capability.
    pub fn find_by_capability(&self, cap: &str) -> Vec<&Service> {
        self.services.iter().filter(|s| s.capabilities.iter().any(|c| c == cap)).collect()
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_registry() -> Registry {
        let toml_str = r#"
[[services]]
id = "r9k-connector"
repo = "git@github.com:wasm-replatform/train.git"
project_dir = "."
crate = "r9k-connector"
domain = "train"
capabilities = ["r9k-xml-ingest"]

[[services]]
id = "r9k-adapter"
repo = "git@github.com:wasm-replatform/train.git"
project_dir = "."
crate = "r9k-adapter"
domain = "train"
capabilities = ["r9k-xml-to-smartrak-gtfs"]

[[services]]
id = "api"
repo = "git@github.com:wasm-replatform/traffic.git"
project_dir = "."
crate = "api"
domain = "traffic"
capabilities = ["flows-api", "incidents-api"]
"#;
        toml::from_str(toml_str).unwrap()
    }

    #[test]
    fn find_by_id() {
        let reg = sample_registry();
        assert_eq!(reg.find_by_id("r9k-connector").unwrap().crate_name, "r9k-connector");
        assert!(reg.find_by_id("nonexistent").is_none());
    }

    #[test]
    fn find_by_domain() {
        let reg = sample_registry();
        assert_eq!(reg.find_by_domain("train").len(), 2);
        assert_eq!(reg.find_by_domain("traffic").len(), 1);
    }

    #[test]
    fn find_by_capability() {
        let reg = sample_registry();
        assert_eq!(reg.find_by_capability("r9k-xml-ingest").len(), 1);
        assert_eq!(reg.find_by_capability("flows-api").len(), 1);
        assert!(reg.find_by_capability("nonexistent").is_empty());
    }

    #[test]
    fn malformed_registry_toml_gives_error() {
        let bad_toml = "not valid toml [[[";
        let result: Result<Registry, _> = toml::from_str(bad_toml);
        assert!(result.is_err());
    }

    #[test]
    fn registry_missing_required_fields() {
        let toml_str = r#"
[[services]]
id = "a"
"#;
        let result: Result<Registry, _> = toml::from_str(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn load_nonexistent_file() {
        let result = Registry::load(std::path::Path::new("/nonexistent/registry.toml"));
        assert!(result.is_err());
    }
}
