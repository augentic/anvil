//! Clap argument types for the `specify plan *` routes.

use clap::{ArgAction, Args};
use workflow::change::plan::handlers::{BindingArg, KindAssign, SourceAssign};

/// Argv mirror of `plan create`'s wire input
/// (`workflow::change::plan::handlers::CreateInput`).
#[derive(Debug, Args)]
pub struct CreateArgs {
    /// Kebab-case change name
    pub name: String,
    /// Named source binding, repeatable. Wire grammar:
    /// `--source <key>=<adapter>:<path>` for path-bound bindings,
    /// or `--source <key>=<adapter>:value:<literal>` for
    /// value-bound bindings (used by `intent`). Recorded in the
    /// plan's `sources:` map as the structured
    /// `{ adapter, path?, value? }` shape per workflow §Source.
    #[arg(long = "source")]
    pub sources: Vec<SourceAssign>,
    /// Operator intent as a literal string — pure sugar for
    /// `--source intent=intent:value:<string>` (the N=1 entry
    /// point without hand-writing the binding grammar).
    /// Combining it with an explicit `--source intent=...`
    /// binding fails on the duplicate-key gate
    /// (`plan-source-duplicate-key`), the same refusal two
    /// conflicting `--source intent=...` occurrences get.
    #[arg(long = "intent", value_name = "STRING")]
    pub intent: Option<String>,
    /// Stamp `lifecycle: approved` atomically with create
    /// (auto-approve Gate-1 contract). Typing this flag *is* the operator's
    /// Gate-1 consent — the CLI runs the same validation it
    /// runs on the post-create path, refuses the create on
    /// failure regardless of the flag, and on success writes a
    /// single atomic `plan.yaml` carrying `lifecycle: approved`
    /// plus the matching `plan.transition.approved` journal
    /// event. Valid on any plan shape (empty scaffold,
    /// single-slice, multi-slice).
    #[arg(long = "auto-approve", action = ArgAction::SetTrue)]
    pub auto_approve: bool,
    /// Pre-seed a per-slice `authority-override` entry on a
    /// named slice (per-slice authority override). Each occurrence takes two
    /// positional values: the slice name and a
    /// `<claim-kind>=<source>` assignment. Repeatable; later
    /// occurrences override earlier ones on the same
    /// `(slice, kind)` tuple. The slice MUST already exist in
    /// the plan being created (unknown names short-circuit with
    /// `plan-authority-override-unknown-slice`); the source key
    /// is validated at `specify slice validate` time via the
    /// orphan-key check. One
    /// `plan.amend.authority-override` journal event fires per
    /// resolved entry in the same batched append as
    /// `--auto-approve`.
    #[arg(
        long = "authority-override",
        value_names = ["SLICE", "KIND=KEY"],
        num_args = 2,
        action = ArgAction::Append,
    )]
    pub authority_override: Vec<String>,
}

/// Argv mirror of `plan validate`'s wire input (no fields).
#[derive(Clone, Copy, Debug, Args)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "clap's `Args` derive requires a braced struct"
)]
pub struct ValidateArgs {}

/// Argv mirror of `plan next`'s wire input (no fields).
#[derive(Clone, Copy, Debug, Args)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "clap's `Args` derive requires a braced struct"
)]
pub struct NextArgs {}

/// Argv mirror of `plan status`' wire input (no fields).
#[derive(Clone, Copy, Debug, Args)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "clap's `Args` derive requires a braced struct"
)]
pub struct StatusArgs {}

/// Argv mirror of `plan execute`'s wire input (no fields).
#[derive(Clone, Copy, Debug, Args)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "clap's `Args` derive requires a braced struct"
)]
pub struct ExecuteArgs {}

/// Argv mirror of `plan remove`'s wire input
/// (`workflow::change::plan::handlers::RemoveInput`).
#[derive(Debug, Args)]
pub struct RemoveArgs {
    /// Kebab-case entry name to remove
    pub name: String,
}

/// Argv mirror of `plan transition`'s wire input
/// (`workflow::change::plan::handlers::TransitionInput`).
#[derive(Debug, Args)]
pub struct TransitionArgs {
    /// Plan name (for plan-level `approved`) or kebab-case entry
    /// name (for per-entry `done` / `--undo`).
    pub name: String,
    /// Transition target — `approved` (plan-level) or `done`
    /// (per-entry). Omit when `--undo` is set.
    #[arg(required_unless_present = "undo")]
    pub target: Option<String>,
    /// Walk one rung backwards on per-entry status. Legal rungs:
    /// `done → in-progress`, `in-progress → pending`. The flag
    /// refuses to skip rungs — undoing a `done` entry to
    /// `pending` MUST run twice so the journal records each step
    /// independently. Fires one `plan.transition.undone` event
    /// per call. Plan-level `approved` cannot be undone; un-stamp
    /// by editing `plan.yaml` directly (out of scope for v1).
    #[arg(long = "undo", action = ArgAction::SetTrue, conflicts_with = "target")]
    pub undo: bool,
    /// Who is driving this invocation — `operator` (default) or
    /// `agent`. Recorded on the `plan.transition.approved`
    /// journal event so eval probes can grade
    /// `gate-1-not-auto-stamped` mechanically; self-reported
    /// evidence, not an enforcement gate. Ignored on per-entry
    /// and `--undo` transitions.
    #[arg(long = "actor", value_name = "ACTOR", default_value = "operator")]
    pub actor: String,
}

/// Argv mirror of `plan author`'s wire input
/// (`workflow::orchestrate::handlers::AuthorInput`).
#[derive(Debug, Args)]
pub struct AuthorArgs {
    /// Kebab-case change name
    pub name: String,
    /// Named source binding, repeatable — the `plan create`
    /// grammar verbatim: `--source <key>=<adapter>:<path>` or
    /// `--source <key>=<adapter>:value:<literal>`.
    #[arg(long = "source")]
    pub sources: Vec<SourceAssign>,
    /// Operator intent as a literal string — pure sugar for
    /// `--source intent=intent:value:<string>`, exactly as on
    /// `plan create`.
    #[arg(long = "intent", value_name = "STRING")]
    pub intent: Option<String>,
}

/// Argv mirror of `plan archive`'s wire input
/// (`workflow::change::plan::handlers::ArchiveInput`).
#[derive(Clone, Copy, Debug, Args)]
pub struct ArchiveArgs {
    /// Archive even when the plan has pending or in-progress entries.
    /// Without --force, these non-terminal statuses block the archive.
    #[arg(long)]
    pub force: bool,
}

/// Argv mirror of `plan add`'s wire input
/// (`workflow::change::plan::handlers::AddInput`).
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
    #[arg(long = "sources", action = ArgAction::Append)]
    pub sources: Vec<BindingArg>,
    /// Free-text scoping hint for the define step
    #[arg(long)]
    pub description: Option<String>,
    /// Target registry project name
    #[arg(long)]
    pub project: Option<String>,
    /// Baseline paths relevant to this change, relative to `.specify/` (repeatable)
    #[arg(long)]
    pub context: Vec<String>,
    /// Set a per-slice `authority-override` entry on the slice
    /// being added (per-slice authority override). Wire form is
    /// `<claim-kind>=<source>`; both sides are kebab-case
    /// and the kind is checked against the closed
    /// [`ClaimKind`](artifacts::evidence::ClaimKind)
    /// enum at parse time. Repeatable; later occurrences win on
    /// the same `(kind)` key. Orphan source keys are caught by
    /// `specify slice validate`. One
    /// `plan.amend.authority-override` event fires per resolved
    /// entry.
    #[arg(long = "authority-override", action = ArgAction::Append)]
    pub authority_override: Vec<KindAssign>,
}

/// Argv mirror of `plan amend`'s wire input
/// (`workflow::change::plan::handlers::AmendInput`).
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
    #[arg(long = "sources", num_args = 0.., value_delimiter = ',')]
    pub sources: Option<Vec<BindingArg>>,
    /// Add a single per-slice source binding (repeatable). Each
    /// value is `<key>=<lead>` or the bare `<key>`
    /// shorthand per workflow §`Slice.sources`.
    #[arg(long = "add-source", action = ArgAction::Append)]
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
    /// `specify plan amend <plan> <slice> --divergence likely`
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
    /// keys are caught by `specify slice validate`.
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
