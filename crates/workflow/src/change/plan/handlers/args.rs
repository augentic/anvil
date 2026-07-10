//! Input-side argument helpers shared by `plan create`, `plan add`,
//! and `plan amend`.
//!
//! Each helper turns the wire-shaped payload into the domain type the
//! handler hands to [`crate::change::Plan`]; the handlers
//! themselves stay free of parsing chatter.

use std::str::FromStr;

use artifacts::discovery::{Discovery, DiscoveryResolveError};
use artifacts::evidence::ClaimKind;
use error::{Error, Result};
use serde::{Deserialize, Serialize};

use crate::change::{Divergence, SliceSourceBinding};
use crate::config::Layout;

/// One per-slice source binding as it crosses the wire: the key from
/// `plan.yaml.sources.<key>` plus an optional lead id. `lead: None` is
/// the bare-string shorthand (`{ key, lead: <slice.name> }`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BindingArg {
    /// Source key.
    pub key: String,
    /// Lead id from `discovery.md`; `None` for the bare shorthand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lead: Option<String>,
}

/// One `<claim-kind>=<source>` authority-override assignment where the
/// slice context is implicit (the `plan add` shape).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct KindAssign {
    /// Claim kind (closed enum).
    pub kind: ClaimKind,
    /// Source key the kind resolves to.
    pub source: String,
}

impl FromStr for KindAssign {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (raw_kind, source) = s
            .split_once('=')
            .ok_or_else(|| format!("--authority-override must be <kind>=<source>, got `{s}`"))?;
        if raw_kind.is_empty() || source.is_empty() {
            return Err(format!(
                "--authority-override kind and source must both be non-empty, got `{s}`"
            ));
        }
        if source.contains('=') {
            return Err(format!(
                "--authority-override value `{s}` must contain exactly one `=` separator between \
                 kind and source"
            ));
        }
        let kind: ClaimKind = raw_kind.parse()?;
        Ok(Self {
            kind,
            source: source.to_string(),
        })
    }
}

/// Materialise wire [`BindingArg`]s into the on-disk
/// [`SliceSourceBinding`] shape, preferring the bare-string shorthand
/// when the lead id equals the slice's name (workflow
/// §`Slice.sources`).
///
/// When `discovery` is `Some(_)`, the supplied lead value must match a
/// canonical `lead` id in `discovery.md`. With `discovery` `None` (no
/// `discovery.md` on disk) the supplied value is used verbatim.
///
/// # Errors
///
/// Unknown lead tokens surface as `Error::validation_failed` (exit 2)
/// with the discriminant `discovery-lead-unknown`.
pub fn bindings_from_args(
    args: &[BindingArg], slice_name: &str, discovery: Option<&Discovery>,
) -> Result<Vec<SliceSourceBinding>> {
    args.iter().map(|a| binding_from_arg(a, slice_name, discovery)).collect()
}

fn binding_from_arg(
    arg: &BindingArg, slice_name: &str, discovery: Option<&Discovery>,
) -> Result<SliceSourceBinding> {
    let lead = match &arg.lead {
        None => None,
        Some(value) => Some(resolve_lead_token(value, discovery)?),
    };
    Ok(match lead {
        None => SliceSourceBinding::bare(arg.key.clone()),
        Some(lead) if lead == slice_name => SliceSourceBinding::bare(arg.key.clone()),
        Some(lead) => SliceSourceBinding::structured(arg.key.clone(), lead),
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
/// `Ok(None)` when the file is absent so `plan create` works
/// without a `discovery.md`.
///
/// # Errors
///
/// Propagates `discovery.md` parse and I/O failures.
pub fn load_discovery(layout: Layout<'_>) -> Result<Option<Discovery>> {
    let path = layout.discovery_path();
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(Discovery::load(&path)?))
}

/// Parse the `--divergence` flag value.
///
/// `likely` / `accepted` / `rejected` are wire-legal — the divergence
/// and writer-ownership contract widens the operator surface so the
/// CLI is the single writer of every variant reachable on disk. The
/// implicit default (absent on disk) has no flag spelling; any other
/// token — including `none` — falls through to the catch-all and is
/// rejected with the same actionable hint.
///
/// # Errors
///
/// `Error::Argument` on any token outside the wire-legal set.
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

/// Chunk an interleaved pair payload into typed `(slice, T)` pairs.
///
/// The payload is a `Vec<String>` of `<slice>` and `<value>` values
/// (the clap `num_args = 2` shape). The value half is parsed via `T`'s
/// `FromStr` impl, so the closed enum (`ClaimKind`) and the composite
/// assign ([`KindAssign`]) share one implementation.
///
/// # Errors
///
/// `Error::Argument` on an empty slice name or an unparseable value
/// half.
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
/// [`crate::change::mutate_authority_overrides`] expects.
///
/// # Errors
///
/// `Error::Argument` on a malformed pair (see
/// [`parse_slice_pair_args`]).
pub fn parse_override_assigns(raw: &[String]) -> Result<Vec<(String, ClaimKind, String)>> {
    Ok(parse_slice_pair_args::<KindAssign>(raw, "--authority-override")?
        .into_iter()
        .map(|(slice, a)| (slice, a.kind, a.source))
        .collect())
}
