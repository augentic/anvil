//! `specify plan *` grammar plus the clap-to-`Input` conversions the
//! dispatch matches use to feed the `workflow::change::plan::handlers` handlers.

pub mod cli;

use std::collections::BTreeMap;

use error::{Error, Result};
use workflow::change::SourceBinding;

use crate::cli::{AuthorityOverrideKindAssign, SliceSourceArg, SourceArg};

impl From<SliceSourceArg> for workflow::change::plan::handlers::BindingArg {
    fn from(arg: SliceSourceArg) -> Self {
        Self {
            key: arg.key,
            lead: arg.lead,
        }
    }
}

impl From<AuthorityOverrideKindAssign> for workflow::change::plan::handlers::KindAssign {
    fn from(assign: AuthorityOverrideKindAssign) -> Self {
        Self {
            kind: assign.kind,
            source: assign.source,
        }
    }
}

/// Convert repeated `--sources` / `--add-source` values into the wire
/// [`workflow::change::plan::handlers::BindingArg`] list.
#[must_use]
pub fn bindings(args: Vec<SliceSourceArg>) -> Vec<workflow::change::plan::handlers::BindingArg> {
    args.into_iter().map(Into::into).collect()
}

/// Convert repeated `--authority-override <kind>=<source>` values into
/// the wire [`workflow::change::plan::handlers::KindAssign`] list.
#[must_use]
pub fn assigns(
    args: Vec<AuthorityOverrideKindAssign>,
) -> Vec<workflow::change::plan::handlers::KindAssign> {
    args.into_iter().map(Into::into).collect()
}

/// Desugar the `plan create` / `plan author` source surface into the
/// structured binding map [`workflow::change::Plan::init`] expects.
///
/// `--intent <string>` appends the value-bound intent binding before
/// the duplicate-key gate, so an explicit `--source intent=...` in
/// the same invocation trips `plan-source-duplicate-key` — the same
/// refusal two conflicting `--source intent=...` occurrences get.
///
/// # Errors
///
/// `Error::Diag` with the stable `plan-source-duplicate-key`
/// discriminant on a duplicate source key.
pub fn source_map(
    mut sources: Vec<SourceArg>, intent: Option<String>,
) -> Result<BTreeMap<String, SourceBinding>> {
    if let Some(value) = intent {
        sources.push(SourceArg::intent(value));
    }
    let mut map: BTreeMap<String, SourceBinding> = BTreeMap::new();
    for SourceArg {
        key,
        adapter,
        path,
        value,
    } in sources
    {
        if map.contains_key(&key) {
            return Err(Error::Diag {
                code: "plan-source-duplicate-key",
                detail: format!("duplicate key `{key}` in --source arguments"),
            });
        }
        map.insert(
            key,
            SourceBinding {
                adapter,
                version: None,
                path,
                value,
            },
        );
    }
    Ok(map)
}
