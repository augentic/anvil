//! Clap argument types for the `emery plan *` routes, including the
//! locked argv grammars for `--source` ([`source_assign`]) and
//! `--sources` / `--add-source` ([`binding_arg`]). The parsed values
//! land directly in the workflow wire DTOs, with each `*Args` type
//! mirroring its command's wire input.

use ::change::plan::wire::{BindingArg, KindAssign, SourceAssign};
use clap::{ArgAction, Args};

/// Parse the locked `--source` argv grammar into a [`SourceAssign`]:
///
/// - `<key>=<adapter>:<path>` — path-bound binding. The adapter is the
///   substring up to the first `:` after `=`; the path is everything
///   after that first `:` (URLs containing `:` such as
///   `git@github.com:org/foo.git` round-trip cleanly).
/// - `<key>=<adapter>:value:<literal>` — value-bound binding. The
///   `value:` sentinel after the adapter switches the parser to
///   literal mode; the literal payload is everything after the second
///   `:` and may contain anything (newlines, colons, equals signs).
///
/// Returns a `String` error on malformed input so clap surfaces a
/// standard usage diagnostic (exit code 2).
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

/// Parse the `--sources` / `--add-source` argv forms (workflow
/// §`Slice.sources`) into a [`BindingArg`]:
///
/// - `<key>=<lead>` — structured binding; both sides are non-empty.
/// - `<key>` — bare-string shorthand; sugar for
///   `{ key: <key>, lead: <slice.name> }`.
///
/// Malformed inputs (empty key, empty lead, dangling `=`, more than
/// one `=`) produce a `String` error that clap surfaces as a standard
/// usage diagnostic (exit code 2).
fn binding_arg(s: &str) -> Result<BindingArg, String> {
    if s.is_empty() {
        return Err("--sources value must be non-empty".to_string());
    }
    if let Some((k, v)) = s.split_once('=') {
        if v.contains('=') {
            return Err(format!("--sources value `{s}` must be <key>=<lead> with at most one `=`"));
        }
        if k.is_empty() || v.is_empty() {
            return Err(format!("--sources key and lead must both be non-empty, got `{s}`"));
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

/// Arguments for `plan next`.
#[derive(Clone, Copy, Debug, Args)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "clap's `Args` derive requires a braced struct"
)]
pub struct NextArgs {}

/// Arguments for `plan status`.
#[derive(Clone, Copy, Debug, Args)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "clap's `Args` derive requires a braced struct"
)]
pub struct StatusArgs {}

/// Arguments for `plan execute`.
#[derive(Clone, Debug, Args)]
pub struct ExecuteArgs {
    /// Who is driving this invocation — `operator` (default) or
    /// `agent`. Recorded on the `plan.transition.approved` journal
    /// event when this run stamps Gate 1 (a `pending` plan approves
    /// on first execute); self-reported evidence for eval probes,
    /// not an enforcement gate.
    #[arg(long = "actor", value_name = "ACTOR", default_value = "operator")]
    pub actor: String,
}

/// Arguments for `plan remove`.
#[derive(Debug, Args)]
pub struct RemoveArgs {
    /// Kebab-case entry name to remove
    pub name: String,
}

/// Arguments for `plan transition`.
#[derive(Debug, Args)]
pub struct TransitionArgs {
    /// Kebab-case plan-entry name.
    pub name: String,
    /// Walk one rung backwards on per-entry status — the only
    /// transition mode. Legal rungs: `done → in-progress`,
    /// `in-progress → pending`. The flag refuses to skip rungs —
    /// undoing a `done` entry to `pending` MUST run twice so the
    /// journal records each step independently. Fires one
    /// `plan.transition.undone` event per call.
    #[arg(long = "undo", action = ArgAction::SetTrue, required = true)]
    pub undo: bool,
}

/// Arguments for `plan author`.
#[derive(Debug, Args)]
pub struct AuthorArgs {
    /// Kebab-case change name
    pub name: String,
    /// Named source binding, repeatable:
    /// `--source <key>=<adapter>:<path>` or
    /// `--source <key>=<adapter>:value:<literal>`.
    #[arg(long = "source", value_parser = source_assign)]
    pub sources: Vec<SourceAssign>,
    /// Operator intent as a literal string — pure sugar for
    /// `--source intent=intent:value:<string>`.
    #[arg(long = "intent", value_name = "STRING")]
    pub intent: Option<String>,
    /// Replace an existing replaceable plan (`lifecycle: pending`
    /// and every entry `pending`). Without --force an existing
    /// `plan.yaml` refuses with `already-exists`; an approved or
    /// in-flight plan refuses with `plan-author-not-replaceable`
    /// (archive first).
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
    /// Ordering dependencies (repeatable). Every value is a change name in the plan.
    /// Pass `--depends-on` (with no value) to clear the field; omit the flag to
    /// leave it unchanged.
    #[arg(long = "depends-on", action = ArgAction::Append)]
    pub depends_on: Vec<String>,
    /// Per-slice source binding (repeatable). Wire form is
    /// `<key>=<lead>`; bare `<key>` is accepted as
    /// shorthand for `{ key: <key>, lead: <slice.name> }`
    /// per workflow §`Slice.sources`.
    #[arg(long = "sources", action = ArgAction::Append, value_parser = binding_arg)]
    pub sources: Vec<BindingArg>,
    /// Free-text scoping hint for the define step
    #[arg(long)]
    pub description: Option<String>,
    /// Target registry project name
    #[arg(long)]
    pub project: Option<String>,
    /// Baseline paths relevant to this change, relative to `.emery/` (repeatable)
    #[arg(long)]
    pub context: Vec<String>,
    /// Set a per-slice `authority-override` entry on the slice
    /// being added (per-slice authority override). Wire form is
    /// `<claim-kind>=<source>`; both sides are kebab-case
    /// and the kind is checked against the closed
    /// [`ClaimKind`](artifacts::evidence::ClaimKind)
    /// enum at parse time. Repeatable; later occurrences win on
    /// the same `(kind)` key. Orphan source keys are caught by
    /// `emery slice validate`. One
    /// `plan.amend.authority-override` event fires per resolved
    /// entry.
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
    /// Set the slice's `divergence` field (workflow §Plan-time
    /// reconciliation; divergence and writer-ownership contract). Accepts `likely`, `accepted`, or
    /// `rejected` — the CLI is the single writer of this field
    /// across every value of the closed enum, so use
    /// `emery plan amend <plan> <slice> --divergence likely`
    /// (or `--divergence accepted|rejected`) instead of editing
    /// `plan.yaml` by hand. `none` (absent) is the implicit
    /// default; omit this flag to leave the field unchanged.
    #[arg(long = "divergence")]
    pub divergence: Option<String>,
    /// Replace description. Pass `--description ""` to clear; omit the flag
    /// to leave it unchanged.
    #[arg(long)]
    pub description: Option<String>,
    /// Replace project. Pass `--project ""` to clear; omit the flag to leave it unchanged.
    #[arg(long)]
    pub project: Option<String>,
    /// Replace context paths. Pass `--context` (with no value) to clear; omit the
    /// flag to leave it unchanged.
    #[arg(long, num_args = 0.., value_delimiter = ',')]
    pub context: Option<Vec<String>>,
    /// Set a per-slice `authority-override` entry (per-slice authority override).
    /// Two positional values per occurrence: the slice name and
    /// a `<claim-kind>=<source>` assignment. Repeatable;
    /// later occurrences override earlier ones on the same
    /// `(slice, kind)` tuple. If the same `(slice, kind)` also
    /// appears in `--clear-authority-override`, the clear
    /// wins (clears apply after sets). Validated against the
    /// closed [`ClaimKind`](artifacts::evidence::ClaimKind) enum
    /// at parse time; orphan source
    /// keys are caught by `emery slice validate`.
    #[arg(
        long = "authority-override",
        value_names = ["SLICE", "KIND=KEY"],
        num_args = 2,
        action = ArgAction::Append,
    )]
    pub authority_override: Vec<String>,
    /// Remove a single `(slice, kind)` entry from the
    /// per-slice `authority-override` map (per-slice authority override). Two
    /// positional values per occurrence: the slice name and
    /// the claim kind (closed enum, kebab-case). Repeatable;
    /// no-op when the entry was already absent. Applied after
    /// `--authority-override` sets so a same-invocation set +
    /// clear pair resolves to the cleared state.
    #[arg(
        long = "clear-authority-override",
        value_names = ["SLICE", "KIND"],
        num_args = 2,
        action = ArgAction::Append,
    )]
    pub clear_authority_override: Vec<String>,
    /// Wipe the entire per-slice `authority-override` map on
    /// the named slice (per-slice authority override). Repeatable for multiple
    /// slices. Applied last, after `--authority-override` sets
    /// and `--clear-authority-override` clears. One
    /// `plan.amend.authority-override` event with `action: clear`
    /// fires per kind that was actually present in the map
    /// before the wipe (no events when the map was already
    /// empty).
    #[arg(
        long = "clear-authority-overrides",
        value_name = "SLICE",
        num_args = 1,
        action = ArgAction::Append,
    )]
    pub clear_authority_overrides: Vec<String>,
}
