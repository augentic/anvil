//! Shared change-trial inputs from a checked-in `trial.env` definition.
//!
//! A wasm change example and a native trial parse the same file so the
//! two rungs cannot drift on their operator inputs.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context as _, Result, bail, ensure};

/// The operator inputs of one change trial.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrialInputs {
    /// Passed to `specify init --name`.
    pub project_name: String,
    /// Change name for `plan author` and `plan transition`.
    pub change: String,
    /// The `--source` binding (`<key>=<adapter>:<path>`).
    pub source: String,
    /// Operator intent bound as the `intent` source.
    pub intent: String,
}

impl TrialInputs {
    /// Load the shared definition at `path`.
    ///
    /// # Errors
    ///
    /// Returns a missing file and any parse failure from [`Self::parse`].
    pub fn load(path: &Path) -> Result<Self> {
        let body =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Self::parse(&body).with_context(|| format!("parsing {}", path.display()))
    }

    /// Parse the shell-sourceable `KEY="value"` body.
    ///
    /// # Errors
    ///
    /// Returns lines outside the documented subset, duplicate or unknown keys,
    /// missing keys, and values a shell `source` would interpret differently.
    pub fn parse(body: &str) -> Result<Self> {
        let mut values: BTreeMap<&str, String> = BTreeMap::new();
        for line in body.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, raw)) = line.split_once('=') else {
                bail!("expected `KEY=\"value\"`, got `{line}`");
            };
            let value = raw
                .strip_prefix('"')
                .and_then(|rest| rest.strip_suffix('"'))
                .with_context(|| format!("`{key}` value must be double-quoted"))?;
            ensure!(
                !value.contains(['"', '$', '`', '\\']),
                "`{key}` value contains a character the shell would expand"
            );
            ensure!(!value.trim().is_empty(), "`{key}` value is empty");
            ensure!(values.insert(key, value.to_owned()).is_none(), "duplicate key `{key}`");
        }

        let mut take =
            |key: &str| values.remove(key).with_context(|| format!("missing `{key}=\"…\"` line"));
        let inputs = Self {
            project_name: take("TRIAL_PROJECT_NAME")?,
            change: take("TRIAL_CHANGE")?,
            source: take("TRIAL_SOURCE")?,
            intent: take("TRIAL_INTENT")?,
        };
        ensure!(
            values.is_empty(),
            "unknown keys: {}",
            values.keys().copied().collect::<Vec<_>>().join(", ")
        );
        Ok(inputs)
    }
}
