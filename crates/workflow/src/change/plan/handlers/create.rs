//! `specify plan create` — scaffold an empty `plan.yaml`. Composes the
//! shared argument parsers in [`super::args`] with the domain
//! authority-override engine in
//! [`crate::change::mutate_authority_overrides`] so the handler
//! stays declarative.

use std::collections::BTreeMap;
use std::io::Write;

use error::{Error, is_kebab};
use omnia_guest::api::{Context, Handler, Reply};
use serde::{Deserialize, Serialize};

use super::args::parse_override_assigns;
use crate::change::{
    Lifecycle, Plan, SourceBinding, mutate_authority_overrides, reject_orphan_overrides,
};
use crate::handler::{Anchor, Ctx, Out, Render};
use crate::journal;

/// Wire input for `plan create`.
///
/// `--source` / `--intent` desugar to the structured `sources` map at
/// the CLI boundary (the map shape is the `plan.yaml.sources` wire
/// form, so HTTP callers pass it directly); `--authority-override`
/// rides as the raw interleaved `<slice> <kind>=<key>` pair list and
/// parses here, keeping the diagnostic identical on every transport.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CreateInput {
    /// Kebab-case change name.
    pub name: String,
    /// Structured source bindings (`plan.yaml.sources` shape).
    #[serde(default)]
    pub sources: BTreeMap<String, SourceBinding>,
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
#[derive(Debug)]
pub struct Create {
    input: CreateInput,
}

impl<P: Anchor> Handler<P> for Create {
    type Error = crate::handler::Error;
    type Input = CreateInput;
    type Output = Out<CreateBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let cx = Ctx::load(ctx.provider)?;
        let CreateInput {
            name,
            sources,
            auto_approve,
            authority_override,
        } = self.input;

        if !is_kebab(&name) {
            return Err(Error::Diag {
                code: "change-name-not-kebab",
                detail: format!(
                    "change: name `{name}` must be kebab-case \
                     (lowercase ascii, digits, single hyphens; no leading/trailing/doubled \
                     hyphens)"
                ),
            }
            .into());
        }
        let plan_path = cx.layout().plan_path();
        if plan_path.exists() {
            return Err(Error::Diag {
                code: "already-exists",
                detail: format!("refusing to overwrite existing plan at {}", plan_path.display()),
            }
            .into());
        }

        let override_assigns = parse_override_assigns(&authority_override)?;

        let mut plan = Plan::init(&name, sources)?;
        // Route `--authority-override` through the shared mutation
        // helper used by `plan amend` so create and amend produce
        // byte-identical `plan.amend.authority-override` journal events
        // and share the unknown-slice gate. Empty `clears` / `clear_all`
        // slices keep the create path scoped to set-only semantics.
        let now = cx.now();
        let plan_name = plan.name.clone();
        let override_events =
            mutate_authority_overrides(&mut plan, &plan_name, &override_assigns, &[], &[], now)?;
        // Re-run the orphan-source gate after the override
        // pre-seeding: `Plan::init` ran no validation against the
        // override map (it didn't exist yet) and `validate_plan` only
        // checks JSON Schema. The orphan check is the only per-slice authority override
        // gate that fires on this code path.
        reject_orphan_overrides(&plan)?;
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

        Ok(Reply::ok(Out(CreateBody {
            name,
            plan: plan_path.display().to_string(),
            lifecycle: plan.lifecycle,
        })))
    }
}

/// Success envelope for `plan create`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct CreateBody {
    /// Change name.
    pub name: String,
    /// Display path of the created plan file.
    pub plan: String,
    /// Final plan-level lifecycle persisted to disk — `pending` for
    /// the default create, `approved` when `--auto-approve` was set.
    /// Exposed in the JSON envelope so skill bodies and tests can
    /// branch on the on-disk state without re-reading `plan.yaml`.
    pub lifecycle: Lifecycle,
}

impl Render for CreateBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        match self.lifecycle {
            Lifecycle::Pending => writeln!(w, "Initialised plan '{}' at {}.", self.name, self.plan),
            Lifecycle::Approved => writeln!(
                w,
                "Initialised plan '{}' at {} and stamped lifecycle: approved.",
                self.name, self.plan
            ),
        }
    }
}
