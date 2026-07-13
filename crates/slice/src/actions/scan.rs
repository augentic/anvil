//! `touched_from_rendered`: classify rendered spec domains against the
//! baseline index for the synthesis persist tail.

use crate::slice::synthesis::baseline::{BaselineIndex, DomainKind};
use crate::slice::{SpecKind, TouchedSpec};

/// Classify rendered spec domains as `new` or `modified` from the baseline
/// index — pure, no filesystem I/O.
#[must_use]
pub fn touched_from_rendered(
    specs: &[crate::slice::synthesis::render::RenderedSpec], baseline_index: &BaselineIndex,
) -> Vec<TouchedSpec> {
    let mut entries: Vec<TouchedSpec> = specs
        .iter()
        .map(|spec| TouchedSpec {
            name: spec.domain.clone(),
            kind: if baseline_index.domain_kind(&spec.domain) == DomainKind::Modified {
                SpecKind::Modified
            } else {
                SpecKind::New
            },
        })
        .collect();
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    entries
}
