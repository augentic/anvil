//! Clap derive surface for the `specify plan *` verbs. The umbrella
//! `cli.rs` re-exports [`PlanAction`].

use std::path::PathBuf;

use clap::{ArgAction, Args, Subcommand};

use crate::runtime::cli::{AuthorityOverrideKindAssign, SliceSourceArg, SourceArg};

/// Plan-authoring verbs (`specify plan *`).
#[derive(Subcommand)]
pub enum PlanAction {
    /// Scaffold an empty `plan.yaml` at the repo root. Refuses to
    /// overwrite an existing plan.
    Create {
        /// Kebab-case change name
        name: String,
        /// Named source binding, repeatable. Wire grammar:
        /// `--source <key>=<adapter>:<path>` for path-bound bindings,
        /// or `--source <key>=<adapter>:value:<literal>` for
        /// value-bound bindings (used by `intent`). Recorded in the
        /// plan's `sources:` map as the structured
        /// `{ adapter, path?, value? }` shape per workflow §Source.
        #[arg(long = "source")]
        sources: Vec<SourceArg>,
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
        auto_approve: bool,
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
        authority_override: Vec<String>,
    },
    /// Validate plan.yaml (structure + plan/change consistency).
    ///
    /// Includes the three health diagnostics — `cycle-in-depends-on`,
    /// `orphan-source`, and `stale-workspace-clone` — alongside
    /// the base shape rules.
    Validate,
    /// Return the active in-progress entry, or transition the next eligible
    /// `Pending` entry to `InProgress` and return it. `plan next` is the
    /// only writer of per-entry `in-progress` (workflow §CLI surface).
    Next,
    /// Read-only projection of the plan's execution state into a
    /// deterministic `next-action` — `refine|build|merge <slice>`,
    /// `stop <reason>`, or `drained`.
    ///
    /// Projects `plan.yaml` entries, the candidate slice's
    /// `metadata.yaml` lifecycle (slot-aware in workspace mode), and
    /// the journal tail. Stop reasons (`plan-not-approved`,
    /// `refine-failed`, `build-failed`, `merge-conflict`,
    /// `slice-dropped`, `merge-incomplete`, `stuck`) are classified
    /// from `slice.synthesize.failed` / `slice.build.failed` /
    /// `slice.merge.failed` journal events scoped to the active
    /// entry's claim window. Writes nothing — `plan next` stays the
    /// only writer of per-entry `in-progress`.
    Status,
    /// Add a new plan entry (status: pending)
    Add(AddArgs),
    /// Edit non-status fields on an existing plan entry.
    ///
    /// Three orthogonal flag families operate on `sources`:
    ///
    /// - `--sources <binding>` (with `num_args = 0..`) replaces the
    ///   slice's `sources` array wholesale.
    /// - `--add-source <binding>` (repeatable) adds a single binding.
    /// - `--remove-source <key>` (repeatable) removes a binding by
    ///   key; fails with `plan-binding-not-found` when no binding
    ///   matches.
    ///
    /// `--add-source` and `--remove-source` apply after `--sources`,
    /// so wholesale replacement plus targeted edits can be combined
    /// in a single invocation when needed.
    Amend(AmendArgs),
    /// Reconcile surveyed leads into `plan.yaml.slices[]`.
    ///
    /// Exactly one mode is required — the parser rejects passing both:
    ///
    /// - `--dry-run` reads the surveyed `discovery.md` lead inventory and
    ///   the resolved project topology (`registry.yaml` for a workspace, or
    ///   the sole project synthesised from `project.yaml`) and emits the
    ///   `kind: request` envelope for the agent to group, recreating the
    ///   plan scratch lane (`.specify/scratch/plan/`) empty for the
    ///   response envelope. Writes no plan state. Aborts with
    ///   `plan-reconcile-empty-catalog` when `discovery.md` carries no
    ///   leads.
    /// - `--from <response.json>` is the only writer. On every invocation
    ///   it re-reads `discovery.md`, rebuilds the lead catalog (never
    ///   trusting a prior dry-run snapshot), validates the agent's
    ///   grouping response, and replaces `plan.yaml.slices[]` wholesale —
    ///   in the agent's response order. When a bound project declares
    ///   non-empty `project.yaml.platforms`, a deterministic bootstrap
    ///   post-pass runs via in-process shell detect (`vectis_missing_platforms`
    ///   / `specify-vectis-shell-detect`; Vectis-bound projects only) and
    ///   inserts bootstrap slices for declared-but-absent shells before
    ///   emitting the single `plan.reconcile.completed` event.
    ///
    /// Passing neither mode fails with `plan-propose-mode-required`
    /// (exit 2).
    Propose(ProposeArgs),
    /// Remove a pending plan entry while the plan is still replaceable
    /// (`lifecycle: pending` and every entry `pending`). Gate 1 curation
    /// only — defers a lead without re-surveying `discovery.md`.
    Remove {
        /// Kebab-case entry name to remove
        name: String,
    },
    /// Apply a validated status transition.
    ///
    /// Two transition shapes share this verb (workflow §CLI surface):
    ///
    /// - **Plan-level Gate 1 stamp** — `<name>` is the plan name and
    ///   `<target>` is `approved`. Operator-only — `/spec:plan` MUST
    ///   NOT call this verb; skill bodies stop at `pending` and print
    ///   the literal `specify plan transition <name> approved`
    ///   command in their closing hint for the operator to run.
    /// - **Per-entry close** — `<name>` is a plan-entry name and
    ///   `<target>` is `done`. The `/spec:merge` skill is the
    ///   canonical caller.
    ///
    /// Per-entry `pending` is written only by `plan add` / `plan amend`;
    /// per-entry `in-progress` is written only by `plan next`. v1 has
    /// no per-entry `blocked`, `failed`, or `skipped` state — build
    /// failures and merge conflicts leave the active entry `in-progress`.
    Transition {
        /// Plan name (for plan-level `approved`) or kebab-case entry
        /// name (for per-entry `done` / `--undo`).
        name: String,
        /// Transition target — `approved` (plan-level) or `done`
        /// (per-entry). Omit when `--undo` is set.
        #[arg(required_unless_present = "undo")]
        target: Option<String>,
        /// Walk one rung backwards on per-entry status. Legal rungs:
        /// `done → in-progress`, `in-progress → pending`. The flag
        /// refuses to skip rungs — undoing a `done` entry to
        /// `pending` MUST run twice so the journal records each step
        /// independently. Fires one `plan.transition.undone` event
        /// per call. Plan-level `approved` cannot be undone; un-stamp
        /// by editing `plan.yaml` directly (out of scope for v1).
        #[arg(long = "undo", action = ArgAction::SetTrue, conflicts_with = "target")]
        undo: bool,
        /// Who is driving this invocation — `operator` (default) or
        /// `agent`. Recorded on the `plan.transition.approved`
        /// journal event so eval probes can grade
        /// `gate-1-not-auto-stamped` mechanically; self-reported
        /// evidence, not an enforcement gate. Ignored on per-entry
        /// and `--undo` transitions.
        #[arg(long = "actor", value_name = "ACTOR", default_value = "operator")]
        actor: String,
    },
    /// Archive the current plan to `.specify/archive/plans/<name>-<YYYYMMDD>.yaml`
    Archive {
        /// Archive even when the plan has pending or in-progress entries.
        /// Without --force, these non-terminal statuses block the archive.
        #[arg(long)]
        force: bool,
    },
    /// Acquire the exclusive plan lock, run `<cmd>` under it, and
    /// release on the child's exit (the `flock(1)` command form).
    ///
    /// The OS advisory lock on `<plan-root>/.specify/plan.lock` is held
    /// for the spawned child's lifetime; the plan-state-writing verbs
    /// (`plan next`, per-entry `plan transition`, `slice merge run`)
    /// run as children and pass their own lock probe. A second driver
    /// that finds the lock held fails fast with `plan-lock-busy`
    /// (exit 2). Re-entrant invocations — a breakout under a parent
    /// `/spec:execute` — skip re-acquisition when `SPECIFY_PLAN_LOCK_HELD=1`
    /// is already set, which the wrapper exports for the child. The
    /// child's exit code is passed through unchanged. Pass any global
    /// flags (e.g. `--plan-dir`) before the `--`.
    Lock {
        /// Command (and its arguments) to run under the plan lock.
        /// Everything after `--` is forwarded verbatim to the child.
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },
}

/// Flag surface for `specify plan propose`. The two flags are mutually
/// exclusive (`--from` `conflicts_with` `--dry-run`); the handler
/// rejects passing neither with `plan-propose-mode-required`.
#[derive(Args)]
pub struct ProposeArgs {
    /// Emit the reconciliation request envelope (flat lead catalog + project topology) for the agent. Writes no plan state; resets .specify/scratch/plan/.
    #[arg(long = "dry-run", action = ArgAction::SetTrue)]
    pub dry_run: bool,
    /// Apply the agent's grouping response, validate it, and replace plan.yaml.slices[]. The only writer.
    #[arg(long = "from", value_name = "RESPONSE_JSON", conflicts_with = "dry_run")]
    pub from: Option<PathBuf>,
}

/// Flag surface for `specify plan add`. Grouped into one struct so the
/// handler threads a single owned value instead of a positional list.
#[derive(Args)]
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
    pub sources: Vec<SliceSourceArg>,
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
    /// and the kind is checked against the closed [`ClaimKind`]
    /// enum at parse time. Repeatable; later occurrences win on
    /// the same `(kind)` key. Orphan source keys are caught by
    /// `specify slice validate`. One
    /// `plan.amend.authority-override` event fires per resolved
    /// entry.
    #[arg(long = "authority-override", action = ArgAction::Append)]
    pub authority_override: Vec<AuthorityOverrideKindAssign>,
}

/// Flag surface for `specify plan amend`. Grouped into one struct so the
/// handler threads a single owned value instead of a positional list.
#[derive(Args)]
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
    pub sources: Option<Vec<SliceSourceArg>>,
    /// Add a single per-slice source binding (repeatable). Each
    /// value is `<key>=<lead>` or the bare `<key>`
    /// shorthand per workflow §`Slice.sources`.
    #[arg(long = "add-source", action = ArgAction::Append)]
    pub add_source: Vec<SliceSourceArg>,
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
    /// closed [`ClaimKind`] enum at parse time; orphan source
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
