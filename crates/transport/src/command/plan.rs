//! Clap argument types for the `emery plan *` routes, including the
//! locked argv grammar for `--sources` / `--add-source` ([`binding_arg`]).

use std::path::PathBuf;

use ::change::plan::wire::{BindingArg, KindAssign};
use artifacts::evidence::ClaimKind;
use clap::{ArgAction, Args};

use super::change_dir::ChangeDir;

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
#[derive(Clone, Debug, Args)]
pub struct ValidateArgs {
    #[command(flatten)]
    pub change_dir: ChangeDir,
}

/// Arguments for `plan status`.
#[derive(Clone, Debug, Args)]
pub struct StatusArgs {
    #[command(flatten)]
    pub change_dir: ChangeDir,
}

/// Arguments for `plan gaps`.
#[derive(Clone, Debug, Args)]
pub struct GapsArgs {
    #[command(flatten)]
    pub change_dir: ChangeDir,
}

/// Arguments for `plan refine`.
#[derive(Clone, Debug, Args)]
pub struct RefineArgs {
    /// Refine only this slice (repeatable). Selected slices bring in
    /// the stale-or-missing predecessor closure needed to keep the
    /// selected work coherent; fresh siblings are skipped.
    #[arg(long = "slice", action = ArgAction::Append, value_name = "NAME")]
    pub slice: Vec<String>,
    #[command(flatten)]
    pub change_dir: ChangeDir,
}

/// Arguments for `plan execute`.
#[derive(Clone, Debug, Args)]
pub struct ExecuteArgs {
    #[command(flatten)]
    pub change_dir: ChangeDir,
}

/// Arguments for `plan remove`.
#[derive(Debug, Args)]
pub struct RemoveArgs {
    /// Kebab-case entry name to remove
    pub name: String,
    #[command(flatten)]
    pub change_dir: ChangeDir,
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
    #[command(flatten)]
    pub change_dir: ChangeDir,
}

/// Arguments for `plan author`.
#[derive(Debug, Args)]
pub struct AuthorArgs {
    /// Kebab-case change name
    pub name: String,
    /// Reviewed definition home. Requires `--wave`. Relative values
    /// join the product root in-place (`.emery/system/` for a colocated
    /// degenerate definition) or the change home when detached.
    #[arg(long = "from", value_name = "DIR", required = true)]
    pub from: PathBuf,
    /// Wave id inside the definition named by `--from`.
    #[arg(long = "wave", value_name = "ID", required = true)]
    pub wave: String,
    /// Replace an existing plan unconditionally, whatever its entry
    /// statuses. Without --force an existing `plan.yaml` refuses
    /// with `plan-already-exists`. Rebind requires the same reviewed
    /// handoff; a changed wave needs a new handoff and review fact.
    #[arg(long)]
    pub force: bool,
    #[command(flatten)]
    pub change_dir: ChangeDir,
}

/// Arguments for `plan archive`.
#[derive(Clone, Debug, Args)]
pub struct ArchiveArgs {
    /// Archive even when the plan has pending or in-progress entries.
    /// Without --force, these non-terminal statuses block the archive.
    #[arg(long)]
    pub force: bool,
    #[command(flatten)]
    pub change_dir: ChangeDir,
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
    /// occurrence). Wire form is
    /// `<key>=<lead>`; bare `<key>` is accepted as
    /// shorthand for `{ key: <key>, lead: <slice.name> }`
    /// per workflow §`Slice.sources`.
    #[arg(long = "source", action = ArgAction::Append, value_parser = binding_arg)]
    pub sources: Vec<BindingArg>,
    /// Required `plan.yaml.targets` key this slice binds to.
    #[arg(long)]
    pub target: String,
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
    #[command(flatten)]
    pub change_dir: ChangeDir,
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
    #[command(flatten)]
    pub change_dir: ChangeDir,
}
