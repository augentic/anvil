//! Input-side argument helpers shared by `plan create`, `plan add`,
//! and `plan amend`.
//!
//! Each helper turns the wire-shaped payload into the domain type the
//! handler hands to [`crate::change::Plan`]; the handlers
//! themselves stay free of parsing chatter.

use std::collections::BTreeMap;
use std::str::FromStr;

use artifacts::discovery::{Discovery, DiscoveryResolveError};
use artifacts::evidence::ClaimKind;
use error::{Error, Result};
use serde::{Deserialize, Serialize};

use crate::change::{Divergence, SliceSourceBinding, SourceBinding};
use crate::config::Layout;

/// One top-level plan source binding as it crosses the wire.
///
/// Carries the key from `plan.yaml.sources.<key>` plus the adapter and
/// its path- or value-binding — the raw `plan create` / `plan author`
/// sources shape on both transports; [`source_map`] desugars the list
/// into the structured `plan.yaml.sources` map inside `from_input`.
///
/// Argv grammar (locked), carried by the [`FromStr`] impl so clap
/// parses `--source` values directly into this type:
///
/// - `--source <key>=<adapter>:<path>` — path-bound binding. The
///   adapter is the substring up to the first `:` after `=`; the
///   path is everything after that first `:` (URLs containing
///   `:` such as `git@github.com:org/foo.git` round-trip cleanly).
/// - `--source <key>=<adapter>:value:<literal>` — value-bound
///   binding. The `value:` sentinel after the adapter switches the
///   parser to literal mode; the literal payload is everything
///   after the second `:` and may contain anything (newlines,
///   colons, equals signs).
///
/// Materialises as [`SourceBinding`] under the structured
/// `{ adapter, path?, value? }` wire form. Every binding carries an
/// explicit adapter name; there is no bare-string
/// `--source <key>=<path>` form.
///
/// The [`FromStr`] impl returns a `String` error on malformed input
/// so clap surfaces a standard usage diagnostic (exit code 2).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SourceAssign {
    /// Source key (left of `=`).
    pub key: String,
    /// Kebab-case source-adapter name (parsed out of the `<adapter>:…`
    /// prefix after `=`).
    pub adapter: String,
    /// Mutually exclusive with `value`. `Some(path)` for the
    /// `<adapter>:<path>` form.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Mutually exclusive with `path`. `Some(literal)` for the
    /// `<adapter>:value:<literal>` form.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

impl SourceAssign {
    /// The desugared `plan create --intent <string>` binding —
    /// byte-identical to parsing `intent=intent:value:<string>`.
    #[must_use]
    pub fn intent(value: String) -> Self {
        Self {
            key: "intent".to_string(),
            adapter: "intent".to_string(),
            path: None,
            value: Some(value),
        }
    }
}

impl FromStr for SourceAssign {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (key, rest) = s.split_once('=').ok_or_else(|| {
            format!(
                "--source must be <key>=<adapter>:<path> or <key>=<adapter>:value:<literal>, got \
                 `{s}`"
            )
        })?;
        if key.is_empty() {
            return Err(format!("--source key must be non-empty, got `{s}`"));
        }
        let (adapter, body) = rest.split_once(':').ok_or_else(|| {
            format!(
                "--source value must be <adapter>:<path> or <adapter>:value:<literal>, got \
                 `{rest}` for key `{key}`"
            )
        })?;
        if adapter.is_empty() {
            return Err(format!("--source adapter must be non-empty, got `{s}`"));
        }
        if body.is_empty() {
            return Err(format!(
                "--source binding (path or `value:<literal>`) must be non-empty, got `{s}`"
            ));
        }
        let (path, value) = if let Some(literal) = body.strip_prefix("value:") {
            if literal.is_empty() {
                return Err(format!(
                    "--source value-literal must be non-empty after `value:`, got `{s}`"
                ));
            }
            (None, Some(literal.to_string()))
        } else {
            (Some(body.to_string()), None)
        };
        Ok(Self {
            key: key.to_string(),
            adapter: adapter.to_string(),
            path,
            value,
        })
    }
}

/// Desugar the `plan create` / `plan author` raw source surface into
/// the structured binding map [`crate::change::Plan::init`] expects.
///
/// Runs inside `from_input` so both transports share the duplicate-key
/// gate and the `--intent` sugar.
///
/// `intent` appends the value-bound intent binding before the
/// duplicate-key gate, so an explicit `--source intent=...` in the
/// same invocation trips `plan-source-duplicate-key` — the same
/// refusal two conflicting `--source intent=...` occurrences get.
///
/// # Errors
///
/// `Error::Diag` with the stable `plan-source-duplicate-key`
/// discriminant on a duplicate source key.
pub fn source_map(
    mut sources: Vec<SourceAssign>, intent: Option<String>,
) -> Result<BTreeMap<String, SourceBinding>> {
    if let Some(value) = intent {
        sources.push(SourceAssign::intent(value));
    }
    let mut map: BTreeMap<String, SourceBinding> = BTreeMap::new();
    for SourceAssign {
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

/// One per-slice source binding as it crosses the wire: the key from
/// `plan.yaml.sources.<key>` plus an optional lead id. `lead: None` is
/// the bare-string shorthand (`{ key, lead: <slice.name> }`).
///
/// Argv wire forms (workflow §`Slice.sources`), carried by the
/// [`FromStr`] impl so clap parses `--sources` / `--add-source`
/// values directly into this type:
///
/// - `<key>=<lead>` — structured binding; both sides are non-empty
///   kebab identifiers.
/// - `<key>` — bare-string shorthand; sugar for
///   `{ key: <key>, lead: <slice.name> }`.
///
/// Malformed inputs (empty key, empty lead, dangling `=`, more than
/// one `=`) produce a `FromStr` error that clap surfaces as a
/// standard usage diagnostic (exit code 2).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BindingArg {
    /// Source key.
    pub key: String,
    /// Lead id from `discovery.md`; `None` for the bare shorthand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lead: Option<String>,
}

impl FromStr for BindingArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err("--sources value must be non-empty".to_string());
        }
        if let Some((k, v)) = s.split_once('=') {
            if v.contains('=') {
                return Err(format!(
                    "--sources value `{s}` must be <key>=<lead> with at most one `=`"
                ));
            }
            if k.is_empty() || v.is_empty() {
                return Err(format!("--sources key and lead must both be non-empty, got `{s}`"));
            }
            Ok(Self {
                key: k.to_string(),
                lead: Some(v.to_string()),
            })
        } else {
            Ok(Self {
                key: s.to_string(),
                lead: None,
            })
        }
    }
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
