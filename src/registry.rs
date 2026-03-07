use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Registry {
    #[serde(rename = "services")]
    pub services: Vec<Service>,
}

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
    pub fn load(path: &Path) -> Result<Self> {
        let content =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&content).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn find_by_id(&self, id: &str) -> Option<&Service> {
        self.services.iter().find(|s| s.id == id)
    }

    pub fn find_by_domain(&self, domain: &str) -> Vec<&Service> {
        self.services.iter().filter(|s| s.domain == domain).collect()
    }

    pub fn find_by_capability(&self, cap: &str) -> Vec<&Service> {
        self.services.iter().filter(|s| s.capabilities.iter().any(|c| c == cap)).collect()
    }

    pub fn print_all(&self) {
        println!("{:<24} {:<12} {:<24} REPO", "ID", "DOMAIN", "CRATE");
        println!("{}", "-".repeat(80));
        for s in &self.services {
            println!("{:<24} {:<12} {:<24} {}", s.id, s.domain, s.crate_name, s.repo);
        }
    }

    pub fn print_by_domain(&self, domain: &str) {
        let matches = self.find_by_domain(domain);
        if matches.is_empty() {
            println!("no services in domain '{domain}'");
            return;
        }
        println!("services in domain '{domain}':");
        for s in matches {
            println!(
                "  {:<24} crate={:<20} caps=[{}]",
                s.id,
                s.crate_name,
                s.capabilities.join(", ")
            );
        }
    }

    pub fn print_by_capability(&self, cap: &str) {
        let matches = self.find_by_capability(cap);
        if matches.is_empty() {
            println!("no services with capability '{cap}'");
            return;
        }
        println!("services with capability '{cap}':");
        for s in matches {
            println!("  {:<24} domain={:<12} crate={}", s.id, s.domain, s.crate_name);
        }
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
}
