//! CLI-side argument-parsing helpers shared by `plan create`,
//! `plan add`, and `plan amend`. Each helper turns the clap-shaped
//! string payload into the domain type the handler will hand to
//! [`specify_workflow::change::Plan`]; the handlers themselves stay
//! free of `FromStr` chatter and `--flag` plumbing.

use std::collections::BTreeMap;
use std::str::FromStr;

use specify_error::{Error, Result};
use specify_model::discovery::{Discovery, DiscoveryResolveError};
use specify_model::evidence::ClaimKind;
use specify_workflow::change::{Divergence, SliceSourceBinding, SourceBinding};
use specify_workflow::config::Layout;

use crate::runtime::cli::{AuthorityOverrideKindAssign, SliceSourceArg, SourceArg};

/// Validate `--source <key>=<adapter>:<binding>` arguments and
/// collapse them into the structured [`SourceBinding`] map
/// `Plan::init` expects. Refuses duplicate keys with the stable
/// `plan-source-duplicate-key` diagnostic.
pub fn build_source_map(sources: Vec<SourceArg>) -> Result<BTreeMap<String, SourceBinding>> {
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

/// Materialise CLI `--sources` / `--add-source` arguments into the
/// on-disk [`SliceSourceBinding`] shape, preferring the bare-string
/// shorthand when the lead id equals the slice's name
/// (workflow §`Slice.sources`).
///
/// When `discovery` is `Some(_)`, the operator-supplied lead value
/// must match a canonical `lead` id in `discovery.md`. Unknown tokens
/// surface as `Error::validation_failed` (exit 2) with the discriminant
/// `discovery-lead-unknown`. With `discovery` `None` (no `discovery.md`
/// on disk) the supplied value is used verbatim.
pub fn bindings_from_args(
    args: Vec<SliceSourceArg>, slice_name: &str, discovery: Option<&Discovery>,
) -> Result<Vec<SliceSourceBinding>> {
    args.into_iter().map(|a| binding_from_arg(a, slice_name, discovery)).collect()
}

fn binding_from_arg(
    arg: SliceSourceArg, slice_name: &str, discovery: Option<&Discovery>,
) -> Result<SliceSourceBinding> {
    let lead = match arg.lead {
        None => None,
        Some(value) => Some(resolve_lead_token(&value, discovery)?),
    };
    Ok(match lead {
        None => SliceSourceBinding::bare(arg.key),
        Some(lead) if lead == slice_name => SliceSourceBinding::bare(arg.key),
        Some(lead) => SliceSourceBinding::structured(arg.key, lead),
    })
}

/// Rewrite a `--sources <key>=<value>` lead token to the canonical
/// `lead` id discovered in `discovery.md`.
///
/// When `discovery` is `None` (no `discovery.md` on disk), the token
/// round-trips unchanged.
fn resolve_lead_token(token: &str, discovery: Option<&Discovery>) -> Result<String> {
    let Some(discovery) = discovery else {
        return Ok(token.to_string());
    };
    match discovery.resolve_lead(token) {
        Ok(lead) => Ok(lead.lead.clone()),
        Err(DiscoveryResolveError::Unknown { token }) => Err(Error::validation_failed(
            "discovery-lead-unknown",
            "--sources <key>=<value> must resolve to a lead in discovery.md",
            format!(
                "no lead in discovery.md has an id matching `{token}`; inspect discovery.md \
                 directly to review the inventory"
            ),
        )),
    }
}

/// Best-effort load of `<project_dir>/discovery.md`. Returns
/// `Ok(None)` when the file is absent so the legacy plan-create
/// path (with no `discovery.md`) keeps working; propagates parse /
/// I/O errors otherwise.
pub fn load_discovery(layout: Layout<'_>) -> Result<Option<Discovery>> {
    let path = layout.discovery_path();
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(Discovery::load(&path)?))
}

/// Parse the `--divergence` flag value. `likely` / `accepted` /
/// `rejected` are wire-legal — divergence and writer-ownership contract widens the operator
/// surface so the CLI is the single writer of every variant
/// reachable on disk. The implicit default (absent on disk) has
/// no flag spelling; any other token — including `none` — falls
/// through to the catch-all and is rejected with the same
/// actionable hint.
pub fn parse_divergence(raw: &str) -> Result<Divergence> {
    match raw {
        "likely" => Ok(Divergence::Likely),
        "accepted" => Ok(Divergence::Accepted),
        "rejected" => Ok(Divergence::Rejected),
        other => Err(Error::Argument {
            flag: "--divergence",
            detail: format!(
                "`{other}` is not a valid --divergence value; expected `likely`, `accepted`, or \
                 `rejected`"
            ),
        }),
    }
}

/// Chunk a clap `num_args = 2` flag payload (`Vec<String>` of
/// interleaved `<slice>` and `<value>` values) into typed
/// `(slice, T)` pairs. The value half is parsed via `T`'s `FromStr`
/// impl, so the closed enum (`ClaimKind`) and the composite assign
/// (`AuthorityOverrideKindAssign`) share one implementation.
pub fn parse_slice_pair_args<T>(raw: &[String], flag: &'static str) -> Result<Vec<(String, T)>>
where
    T: FromStr<Err = String>,
{
    let mut out = Vec::with_capacity(raw.len() / 2);
    for chunk in raw.chunks_exact(2) {
        let slice = chunk[0].clone();
        if slice.is_empty() {
            return Err(Error::Argument {
                flag,
                detail: format!("{flag} <slice> must be non-empty"),
            });
        }
        let value: T =
            chunk[1].parse().map_err(|detail: String| Error::Argument { flag, detail })?;
        out.push((slice, value));
    }
    Ok(out)
}

/// Parse `--authority-override <slice> <kind>=<source>` repeats
/// into the typed `(slice, kind, source)` tuple
/// [`specify_workflow::change::mutate_authority_overrides`] expects.
pub fn parse_override_assigns(raw: &[String]) -> Result<Vec<(String, ClaimKind, String)>> {
    Ok(parse_slice_pair_args::<AuthorityOverrideKindAssign>(raw, "--authority-override")?
        .into_iter()
        .map(|(slice, a)| (slice, a.kind, a.source))
        .collect())
}
