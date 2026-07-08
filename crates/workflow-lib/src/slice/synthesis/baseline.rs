//! Baseline index for synthesis ID assignment and delta rendering.
//!
//! Built from the bound project's `.specify/specs/` tree at synthesize
//! time. Each domain with an on-disk `spec.md` is [`DomainKind::Modified`];
//! domains absent from the baseline are [`DomainKind::New`].

use std::collections::BTreeMap;
use std::path::Path;

use specify_error::Error;
use specify_model::spec::provenance::parse_spec_md;

use crate::slice::SpecKind;

/// Whether a domain exists in the project baseline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainKind {
    /// No baseline `specs/<domain>/spec.md` — flat requirement blocks.
    New,
    /// Baseline spec exists — delta sections (`## ADDED` / `## MODIFIED`).
    Modified,
}

/// Per-domain baseline facts used by the projection kernel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainBaseline {
    /// `modified` when a baseline `spec.md` exists.
    pub kind: DomainKind,
    /// Highest `REQ-NNN` numeric suffix in the baseline (0 when empty).
    pub max_req_num: u32,
    /// Baseline `REQ` id → requirement title.
    pub ids: BTreeMap<String, String>,
}

/// Index of baseline domains for baseline-aware synthesis.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BaselineIndex {
    domains: BTreeMap<String, DomainBaseline>,
}

impl BaselineIndex {
    /// Scan `specs_dir` for domains with a baseline `spec.md`.
    ///
    /// # Errors
    ///
    /// Propagates filesystem errors from reading the specs tree.
    pub fn build(specs_dir: &Path) -> Result<Self, Error> {
        let mut domains = BTreeMap::new();
        if !specs_dir.is_dir() {
            return Ok(Self { domains });
        }
        for entry in std::fs::read_dir(specs_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let Some(domain) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            let spec_path = entry.path().join("spec.md");
            if !spec_path.is_file() {
                continue;
            }
            let text = std::fs::read_to_string(&spec_path)?;
            domains.insert(domain, domain_baseline_from_spec(&text));
        }
        Ok(Self { domains })
    }

    /// Classify a domain slug. Absent domains are [`DomainKind::New`].
    #[must_use]
    pub fn domain_kind(&self, domain: &str) -> DomainKind {
        self.domains.get(domain).map_or(DomainKind::New, |baseline| baseline.kind)
    }

    /// `true` when `id` is an existing baseline requirement in `domain`.
    #[must_use]
    pub fn is_baseline_req(&self, domain: &str, id: &str) -> bool {
        self.domains.get(domain).is_some_and(|baseline| baseline.ids.contains_key(id))
    }

    /// Iterate every indexed domain in slug order.
    pub fn domains(&self) -> impl Iterator<Item = (&str, &DomainBaseline)> {
        self.domains.iter().map(|(domain, baseline)| (domain.as_str(), baseline))
    }

    /// Map a [`SpecKind`] from [`crate::slice::actions::scan_touched`] to
    /// [`DomainKind`].
    #[must_use]
    pub const fn kind_from_spec_kind(kind: SpecKind) -> DomainKind {
        match kind {
            SpecKind::New => DomainKind::New,
            SpecKind::Modified => DomainKind::Modified,
        }
    }
}

fn domain_baseline_from_spec(text: &str) -> DomainBaseline {
    let parsed = parse_spec_md(text);
    let mut ids = BTreeMap::new();
    let mut max_req_num = 0_u32;
    for req in parsed.requirements {
        if req.id.is_empty() {
            continue;
        }
        ids.insert(req.id.clone(), req.name);
        if let Some(num) = req_num(&req.id) {
            max_req_num = max_req_num.max(num);
        }
    }
    DomainBaseline {
        kind: DomainKind::Modified,
        max_req_num,
        ids,
    }
}

fn req_num(id: &str) -> Option<u32> {
    id.strip_prefix("REQ-")?.parse().ok()
}
