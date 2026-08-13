//! Wire DTOs and payload parsers shared by `plan add` and `plan amend`
//! on both transports. The clap-only argv grammars (`--sources` /
//! `--add-source` string forms) live in the transport crate.

use std::str::FromStr;

use artifacts::evidence::ClaimKind;
use artifacts::leads::{Leads, LeadsResolveError};
use error::{Error, Result};
use project::config::Layout;
use project::plan::{Divergence, SliceSourceBinding};
use serde::{Deserialize, Serialize};

/// One per-slice source binding as it crosses the wire: the key from
/// `plan.yaml.sources.<key>` plus an optional lead id. `lead: None` is
/// the bare-string shorthand (`{ key, lead: <slice.name> }`).
///
/// The `<key>=<lead>` / bare-`<key>` argv forms (workflow
/// §`Slice.sources`) are transport-owned; the command grammar parses
/// them into this type.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BindingArg {
    /// Source key.
    pub key: String,
    /// Lead id from `leads.md`; `None` for the bare shorthand.
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
/// When `leads` is `Some(_)`, the supplied lead value must match a
/// canonical `lead` id in `leads.md`. With `leads` `None` (no
/// `leads.md` on disk) the supplied value is used verbatim.
///
/// # Errors
///
/// Unknown lead tokens surface as `Error::validation_failed` (exit 2)
/// with the discriminant `leads-lead-unknown`.
pub(crate) fn bindings_from_args(
    args: &[BindingArg], slice_name: &str, leads: Option<&Leads>,
) -> Result<Vec<SliceSourceBinding>> {
    args.iter().map(|a| binding_from_arg(a, slice_name, leads)).collect()
}

fn binding_from_arg(
    arg: &BindingArg, slice_name: &str, leads: Option<&Leads>,
) -> Result<SliceSourceBinding> {
    let lead = match &arg.lead {
        None => None,
        Some(value) => Some(resolve_lead_token(value, leads)?),
    };
    Ok(match lead {
        None => SliceSourceBinding::bare(arg.key.clone()),
        Some(lead) if lead == slice_name => SliceSourceBinding::bare(arg.key.clone()),
        Some(lead) => SliceSourceBinding::structured(arg.key.clone(), lead),
    })
}

/// Rewrite a `<key>=<lead>` binding's lead token to the canonical
/// `lead` id in `leads.md`.
///
/// When `leads` is `None` (no `leads.md` on disk), the token
/// round-trips unchanged.
fn resolve_lead_token(token: &str, leads: Option<&Leads>) -> Result<String> {
    let Some(leads) = leads else {
        return Ok(token.to_string());
    };
    match leads.resolve_lead(token) {
        Ok(lead) => Ok(lead.lead.clone()),
        Err(LeadsResolveError::Unknown { token }) => Err(Error::validation_failed(
            "leads-lead-unknown",
            "source bindings (`<key>=<lead>`) must resolve to a lead in leads.md",
            format!(
                "no lead in leads.md has an id matching `{token}`; inspect leads.md \
                 directly to review the inventory"
            ),
        )),
    }
}

/// Best-effort load of `<change>/leads.md`. Returns `Ok(None)` when
/// the file is absent so plan scaffolding works without a catalog.
///
/// # Errors
///
/// Propagates `leads.md` parse and I/O failures.
pub(crate) fn load_leads(layout: Layout<'_>) -> Result<Option<Leads>> {
    let path = layout.leads_path();
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(Leads::load(&path)?))
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
pub(crate) fn parse_divergence(raw: &str) -> Result<Divergence> {
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
