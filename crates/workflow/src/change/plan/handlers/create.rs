//! `specify plan create` — scaffold an empty `plan.yaml`.

use std::io::Write;
use std::path::PathBuf;

use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use serde::{Deserialize, Serialize};

use crate::change::plan::wire::{SourceAssign, parse_override_assigns, source_map};
use crate::change::{Lifecycle, authority_override, scaffold};
use crate::handler::{Anchor, Ctx, Render};
use crate::journal;

/// Wire input for `plan create`.
///
/// Carries the raw source surface on every transport: `sources` is
/// the [`SourceAssign`] list (`--source` repeats) and `intent` the
/// `--intent` sugar. The desugaring into the structured
/// `plan.yaml.sources` map — including the duplicate-key gate — runs
/// at the operation boundary. `--authority-override` rides as the raw interleaved
/// `<slice> <kind>=<key>` pair list and parses in `call` for the
/// same reason.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CreateInput {
    /// Kebab-case change name.
    pub name: String,
    /// Raw source bindings (the `--source` repeat list).
    #[serde(default)]
    pub sources: Vec<SourceAssign>,
    /// Operator intent literal — sugar for
    /// `--source intent=intent:value:<string>`.
    #[serde(default)]
    pub intent: Option<String>,
    /// Stamp `lifecycle: approved` atomically with create
    /// (auto-approve Gate-1 contract).
    #[serde(default)]
    pub auto_approve: bool,
    /// Interleaved `<slice> <kind>=<key>` authority-override pairs.
    #[serde(default)]
    pub authority_override: Vec<String>,
}

/// `specify plan create <name> [--source ...] [--auto-approve]`.
///
/// Scaffolds an empty `plan.yaml` (workflow §The Plan); slices are
/// authored later by the `plan author` reconcile kernel or `specify
/// plan add`.
///
/// When `auto_approve` is set (auto-approve Gate-1 contract), the plan
/// is constructed with `lifecycle: approved` *before* the single
/// atomic `plan.save` — there is never a transient `lifecycle:
/// pending` file on disk. The matching `plan.transition.approved`
/// journal event is appended in the same batched write as any
/// `plan.amend.authority-override` events the same invocation
/// produced; validation failures (kebab-case name, orphan source key)
/// refuse the create with or without the flag and leave the journal
/// untouched.
#[derive(Clone, Copy, Debug)]
pub struct Create;

impl<P: Anchor> Operation<P> for Create {
    type Error = crate::handler::Error;
    type Input = CreateInput;
    type Output = CreateBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let CreateInput {
            name,
            sources,
            intent,
            auto_approve,
            authority_override,
        } = input;
        let sources = source_map(sources, intent)?;
        let plan_path = cx.layout().plan_path();
        let mut plan = scaffold(&plan_path, &name, sources)?;

        let override_assigns = parse_override_assigns(&authority_override)?;
        // Route `--authority-override` through the shared mutation
        // helper used by `plan amend` so create and amend produce
        // byte-identical `plan.amend.authority-override` journal events
        // and share the unknown-slice gate. Empty `clears` / `clear_all`
        // slices keep the create path scoped to set-only semantics.
        let now = cx.now();
        let plan_name = plan.name.clone();
        let override_events =
            authority_override::mutate(&mut plan, &plan_name, &override_assigns, &[], &[], now)?;
        // Re-run the orphan-source gate after the override
        // pre-seeding: `Plan::init` ran no validation against the
        // override map (it didn't exist yet) and `validate_plan` only
        // checks JSON Schema. The orphan check is the only per-slice authority override
        // gate that fires on this code path.
        authority_override::reject_orphans(&plan)?;
        if auto_approve {
            plan.transition_lifecycle(Lifecycle::Approved)?;
        }
        plan.save(&plan_path)?;

        // Collect every journal event the invocation produced, then
        // hand the slice to `append_batch` so the post-save log write is
        // a single fsynced append. Either every event lands or none
        // does — `--auto-approve` and `--authority-override` compose
        // without a partial-state window in the journal.
        let mut events: Vec<journal::Event> = Vec::new();
        if auto_approve {
            // Typing `--auto-approve` *is* the operator's Gate-1 consent,
            // so the create path always records `actor: operator`.
            events.push(journal::Event::new(
                now,
                journal::EventKind::PlanTransitionApproved {
                    plan_name: plan_name.clone(),
                    actor: journal::Actor::Operator,
                },
            ));
        }
        events.extend(override_events);
        journal::append_batch(cx.layout(), &events)?;

        Ok(CreateBody {
            name,
            plan: plan_path,
            lifecycle: plan.lifecycle,
        })
    }
}

/// Success envelope for `plan create`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct CreateBody {
    pub name: String,
    pub plan: PathBuf,
    /// Final plan-level lifecycle persisted to disk — `pending` for
    /// the default create, `approved` when `--auto-approve` was set.
    /// Exposed in the JSON envelope so skill bodies and tests can
    /// branch on the on-disk state without re-reading `plan.yaml`.
    pub lifecycle: Lifecycle,
}

impl Render for CreateBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        match self.lifecycle {
            Lifecycle::Pending => {
                writeln!(w, "Initialised plan '{}' at {}.", self.name, self.plan.display())
            }
            Lifecycle::Approved => writeln!(
                w,
                "Initialised plan '{}' at {} and stamped lifecycle: approved.",
                self.name,
                self.plan.display()
            ),
        }
    }
}
