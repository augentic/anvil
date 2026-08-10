//! Clap argument types for the `emery plan *` routes, including the
//! locked argv grammars for `--source` ([`source_assign`]) and
//! `--sources` / `--add-source` ([`binding_arg`]).

use ::change::plan::wire::{BindingArg, KindAssign, SourceAssign};
use artifacts::evidence::ClaimKind;
use clap::{ArgAction, Args};

/// Parse the locked `--source` argv grammar into a [`SourceAssign`]:
/// `<key>=<adapter>:<path>` (path-bound) or
/// `<key>=<adapter>:value:<literal>` (value-bound).
///
/// Only the first `:` after `=` splits adapter from binding, so URLs
/// like `git@github.com:org/foo.git` round-trip; after the `value:`
/// sentinel the literal may contain anything. Malformed input returns
/// a `String` error clap surfaces as a usage diagnostic (exit 2).
fn source_assign(s: &str) -> Result<SourceAssign, String> {
    let (key, rest) = s.split_once('=').ok_or_else(|| {
        format!(
            "--source must be <key>=<adapter>:<path> or <key>=<adapter>:value:<literal>, got `{s}`"
        )
    })?;
    if key.is_empty() {
        return Err(format!("--source key must be non-empty, got `{s}`"));
    }
    let (adapter, body) = rest.split_once(':').ok_or_else(|| {
        format!(
            "--source value must be <adapter>:<path> or <adapter>:value:<literal>, got `{rest}` \
             for key `{key}`"
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
    Ok(SourceAssign {
        key: key.to_string(),
        adapter: adapter.to_string(),
        path,
        value,
    })
}

/// Parse the `plan add --source` / `plan amend --sources` /
/// `--add-source` argv forms into a [`BindingArg`]:
/// `<key>=<lead>` (structured) or bare `<key>` (shorthand for
/// `{ key: <key>, lead: <slice.name> }`).
///
/// Malformed inputs produce a `String` error that clap surfaces as a
/// usage diagnostic (exit 2). The messages stay flag-neutral — clap
/// prefixes the offending flag itself.
fn binding_arg(s: &str) -> Result<BindingArg, String> {
    if s.is_empty() {
        return Err("source binding must be non-empty".to_string());
    }
    if let Some((k, v)) = s.split_once('=') {
        if v.contains('=') {
            return Err(format!("source binding `{s}` must be <key>=<lead> with at most one `=`"));
        }
        if k.is_empty() || v.is_empty() {
            return Err(format!("source binding key and lead must both be non-empty, got `{s}`"));
        }
        Ok(BindingArg {
            key: k.to_string(),
            lead: Some(v.to_string()),
        })
    } else {
        Ok(BindingArg {
            key: s.to_string(),
            lead: None,
        })
    }
}

/// Arguments for `plan validate`.
#[derive(Clone, Copy, Debug, Args)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "clap's `Args` derive requires a braced struct"
)]
pub struct ValidateArgs {}

/// Arguments for `plan status`.
#[derive(Clone, Copy, Debug, Args)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "clap's `Args` derive requires a braced struct"
)]
pub struct StatusArgs {}

/// Arguments for `plan gaps`.
#[derive(Clone, Copy, Debug, Args)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "clap's `Args` derive requires a braced struct"
)]
pub struct GapsArgs {}

/// Parse `--waive <slice>/<req>` into a [`change::orchestrate::WaiveSelector`].
fn waive_selector(raw: &str) -> Result<::change::orchestrate::WaiveSelector, String> {
    let (slice, req) =
        raw.split_once('/').ok_or_else(|| format!("--waive must be <slice>/<req>, got `{raw}`"))?;
    if slice.is_empty() || req.is_empty() {
        return Err(format!("--waive must be <slice>/<req> with non-empty parts, got `{raw}`"));
    }
    Ok(::change::orchestrate::WaiveSelector {
        slice: slice.to_string(),
        req: req.to_string(),
    })
}

/// Arguments for `plan execute`.
#[derive(Clone, Debug, Args)]
pub struct ExecuteArgs {
    /// Waive one open `[unknown]` requirement (`<slice>/<req>`). Repeatable.
    /// Requires `--reason`. Conflicts are never waiveable.
    #[arg(
        long = "waive",
        action = ArgAction::Append,
        value_name = "SLICE/REQ",
        value_parser = waive_selector,
        requires = "reason"
    )]
    pub waive: Vec<::change::orchestrate::WaiveSelector>,
    /// Operator reason applied to every `--waive` on this invocation.
    /// Requires at least one `--waive`.
    #[arg(long, value_name = "REASON", requires = "waive")]
    pub reason: Option<String>,
}

/// Arguments for `plan remove`.
#[derive(Debug, Args)]
pub struct RemoveArgs {
    /// Kebab-case entry name to remove
    pub name: String,
}

/// Arguments for `plan drop`.
#[derive(Debug, Args)]
pub struct DropArgs {
    /// Kebab-case plan entry (slice) name to drop.
    pub name: String,
    /// Free-text reason; surfaced in `metadata.yaml.drop_reason` and
    /// the archive path.
    #[arg(long)]
    pub reason: Option<String>,
}

/// Parse `<slice>/<req>` into a
/// [`::change::plan::handlers::DeferSelector`].
fn defer_selector(raw: &str) -> Result<::change::plan::handlers::DeferSelector, String> {
    let (slice, req) = raw
        .split_once('/')
        .ok_or_else(|| format!("selector must be <slice>/<req>, got `{raw}`"))?;
    if slice.is_empty() || req.is_empty() {
        return Err(format!("selector must be <slice>/<req> with non-empty parts, got `{raw}`"));
    }
    Ok(::change::plan::handlers::DeferSelector {
        slice: slice.to_string(),
        req: req.to_string(),
    })
}

/// Arguments for `plan defer`.
#[derive(Debug, Args)]
pub struct DeferArgs {
    /// Gap requirement selectors (`<slice>/<req>`), one or more.
    #[arg(required = true, value_name = "SLICE/REQ", value_parser = defer_selector)]
    pub selectors: Vec<::change::plan::handlers::DeferSelector>,
    /// Reason recorded on every appended fact. Required to defer;
    /// optional with `--retract`.
    #[arg(long, value_name = "REASON")]
    pub reason: Option<String>,
    /// Retract live deferrals (append `gap.deferral-retracted`)
    /// instead of deferring.
    #[arg(long)]
    pub retract: bool,
}

/// Arguments for `plan author`.
#[derive(Debug, Args)]
pub struct AuthorArgs {
    /// Kebab-case change name
    pub name: String,
    /// Named source binding, repeatable:
    /// `--source <key>=<adapter>:<path>` or
    /// `--source <key>=<adapter>:value:<literal>`.
    ///
    /// `<key>` (kebab-case, your label) becomes the slot name in
    /// `plan.yaml.sources`; `<adapter>` is the source adapter name
    /// (e.g. `typescript`). Only the first `:` splits adapter from
    /// binding, so URLs like `git@github.com:org/repo.git` pass
    /// through unchanged. Example: `--source legacy=typescript:./legacy`
    #[arg(long = "source", value_parser = source_assign)]
    pub sources: Vec<SourceAssign>,
    /// Operator intent as a literal string — pure sugar for
    /// `--source intent=intent:value:<string>`.
    #[arg(long = "intent", value_name = "STRING")]
    pub intent: Option<String>,
    /// Replace an existing plan unconditionally, whatever its entry
    /// statuses. Without --force an existing `plan.yaml` refuses
    /// with `plan-already-exists`.
    #[arg(long)]
    pub force: bool,
}

/// Arguments for `plan archive`.
#[derive(Clone, Copy, Debug, Args)]
pub struct ArchiveArgs {
    /// Archive even when the plan has pending or in-progress entries.
    /// Without --force, these non-terminal statuses block the archive.
    #[arg(long)]
    pub force: bool,
}

/// Arguments for `plan add`.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// Kebab-case plan entry (slice) name for the new row under `plan.yaml.slices[]`.
    pub name: String,
    /// Ordering dependencies (repeatable or comma-separated). Every
    /// value is another plan-entry name.
    #[arg(long = "depends-on", action = ArgAction::Append, value_delimiter = ',')]
    pub depends_on: Vec<String>,
    /// Per-slice source binding (repeatable; one binding per
    /// occurrence, matching `plan author --source`). Wire form is
    /// `<key>=<lead>`; bare `<key>` is accepted as
    /// shorthand for `{ key: <key>, lead: <slice.name> }`
    /// per workflow §`Slice.sources`.
    #[arg(long = "source", action = ArgAction::Append, value_parser = binding_arg)]
    pub sources: Vec<BindingArg>,
    /// Free-text scoping hint for the define step
    #[arg(long)]
    pub description: Option<String>,
    /// Baseline paths relevant to this change, relative to `.emery/` (repeatable)
    #[arg(long)]
    pub context: Vec<String>,
    /// Set a per-slice `authority-override` entry on the slice being
    /// added. Wire form is `<claim-kind>=<source>` (kebab-case); the
    /// kind is checked against the closed
    /// [`ClaimKind`](artifacts::evidence::ClaimKind) enum at parse
    /// time. Repeatable; later occurrences win on the same kind.
    /// Orphan source keys are caught by `emery slice validate`; one
    /// `plan.amend.authority-override` event fires per resolved entry.
    #[arg(long = "authority-override", action = ArgAction::Append)]
    pub authority_override: Vec<KindAssign>,
}

/// Arguments for `plan amend`.
#[derive(Debug, Args)]
pub struct AmendArgs {
    /// Kebab-case plan entry (slice) name — the row under `plan.yaml.slices[]`
    /// being edited. There is one active plan file; this is not the plan name.
    pub name: String,
    /// Replace depends-on. Pass `--depends-on` (with no value) to clear the
    /// field; omit the flag to leave it unchanged. Repeat or comma-separate
    /// to supply multiple values.
    #[arg(long = "depends-on", num_args = 0.., value_delimiter = ',')]
    pub depends_on: Option<Vec<String>>,
    /// Replace per-slice source bindings wholesale. Each value
    /// is `<key>=<lead>` (or bare `<key>` shorthand).
    /// Pass `--sources` (no value) to clear; omit to leave
    /// unchanged.
    #[arg(long = "sources", num_args = 0.., value_delimiter = ',', value_parser = binding_arg)]
    pub sources: Option<Vec<BindingArg>>,
    /// Add a single per-slice source binding (repeatable). Each
    /// value is `<key>=<lead>` or the bare `<key>`
    /// shorthand per workflow §`Slice.sources`.
    #[arg(long = "add-source", action = ArgAction::Append, value_parser = binding_arg)]
    pub add_source: Vec<BindingArg>,
    /// Remove a per-slice source binding by key (repeatable).
    /// Fails with `plan-binding-not-found` when no such binding
    /// exists on the slice.
    #[arg(long = "remove-source", action = ArgAction::Append)]
    pub remove_source: Vec<String>,
    /// Set the entry's `divergence` field (workflow §Plan-time
    /// reconciliation; divergence and writer-ownership contract).
    /// Accepts `likely`, `accepted`, or `rejected` — the CLI is the
    /// single writer of this field across every value of the closed
    /// enum, so use `emery plan amend <entry> --divergence likely`
    /// (or `--divergence accepted|rejected`) instead of editing
    /// `plan.yaml` by hand. `none` (absent) is the implicit
    /// default; omit this flag to leave the field unchanged.
    #[arg(long = "divergence")]
    pub divergence: Option<String>,
    /// Replace description. Pass `--description ""` to clear; omit the flag
    /// to leave it unchanged.
    #[arg(long)]
    pub description: Option<String>,
    /// Replace context paths. Pass `--context` (with no value) to clear; omit the
    /// flag to leave it unchanged.
    #[arg(long, num_args = 0.., value_delimiter = ',')]
    pub context: Option<Vec<String>>,
    /// Set a per-slice `authority-override` entry on the entry being
    /// amended — the same `<claim-kind>=<source>` grammar as `plan add
    /// --authority-override`; the kind is checked against the closed
    /// [`ClaimKind`](artifacts::evidence::ClaimKind) enum at parse
    /// time. Repeatable; later occurrences win on the same kind. If
    /// the same kind also appears in `--clear-authority-override`, the
    /// clear wins (clears apply after sets). Orphan source keys are
    /// caught by `emery slice validate`.
    #[arg(long = "authority-override", action = ArgAction::Append)]
    pub authority_override: Vec<KindAssign>,
    /// Remove a single claim kind from the amended entry's
    /// `authority-override` map (closed enum, kebab-case).
    /// Repeatable; no-op when the entry was already absent. Applied
    /// after `--authority-override` sets so a same-invocation set +
    /// clear pair resolves to the cleared state.
    #[arg(long = "clear-authority-override", value_name = "KIND", action = ArgAction::Append)]
    pub clear_authority_override: Vec<ClaimKind>,
    /// Wipe the amended entry's entire `authority-override` map.
    /// Applied last, after `--authority-override` sets and
    /// `--clear-authority-override` clears. One
    /// `plan.amend.authority-override` event with `action: clear`
    /// fires per kind that was actually present in the map before the
    /// wipe (no events when the map was already empty).
    #[arg(long = "clear-authority-overrides", action = ArgAction::SetTrue)]
    pub clear_authority_overrides: bool,
    /// Set the entry's `allow-composition-replace` field: authorise a
    /// whole-document (`screens:`) slice composition to overwrite a
    /// non-empty baseline when the execute loop merges this slice.
    /// Reserved for intentional full-baseline rewrites; routine
    /// per-screen edits flow through `delta:` and never need it. Pass
    /// `true` or `false`; omit the flag to leave it unchanged.
    #[arg(long = "allow-composition-replace", value_name = "BOOL")]
    pub allow_composition_replace: Option<bool>,
}
