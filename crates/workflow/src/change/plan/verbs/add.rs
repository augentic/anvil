//! `specify plan add` — append one slice entry to an existing
//! `plan.yaml`. Authority-override seeding is delegated to the shared
//! domain helper so the journal events match `plan create` and `plan
//! amend` byte-for-byte.

use std::collections::BTreeMap;

use omnia_guest::api::{Context, Handler, Reply};
use serde::{Deserialize, Serialize};
use crate::change::{
    Entry, Plan, SliceAuthorityOverride, Status, emit_authority_override_seed_events, entry_mut,
};
use crate::config::with_state;
use crate::journal;
use crate::schema::validate_plan;

use super::args::{BindingArg, KindAssign, bindings_from_args, load_discovery};
use super::entry::{Action, EntryBody};
use super::{check_project, plan_ref};
use crate::verb::Anchor;
use crate::verb::Ctx;
use crate::verb::Out;

/// Wire input for `plan add`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AddInput {
    /// Kebab-case plan entry (slice) name for the new row under
    /// `plan.yaml.slices[]`.
    pub name: String,
    /// Ordering dependencies — change names in the plan.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Per-slice source bindings.
    #[serde(default)]
    pub sources: Vec<BindingArg>,
    /// Free-text scoping hint for the define step.
    #[serde(default)]
    pub description: Option<String>,
    /// Target registry project name.
    #[serde(default)]
    pub project: Option<String>,
    /// Baseline paths relevant to this change, relative to `.specify/`.
    #[serde(default)]
    pub context: Vec<String>,
    /// Per-slice authority-override assignments for the slice being
    /// added.
    #[serde(default)]
    pub authority_override: Vec<KindAssign>,
}

/// `specify plan add <name> [flags]` — append one `pending` entry.
#[derive(Debug)]
pub struct Add {
    input: AddInput,
}

impl<P: Anchor> Handler<P> for Add {
    type Error = crate::verb::Error;
    type Input = AddInput;
    type Output = Out<EntryBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let cx = Ctx::load(ctx.provider)?;
        let AddInput {
            name,
            depends_on,
            sources,
            description,
            project,
            context,
            authority_override,
        } = self.input;
        let name = name.as_str();

        if let Some(proj) = &project {
            check_project(&cx.project_dir, proj)?;
        }

        // When `discovery.md` exists, resolve `--sources <key>=<lead>` to the
        // canonical lead id before persisting. Absence of `discovery.md`
        // short-circuits to the verbatim path.
        let discovery = load_discovery(cx.layout())?;
        let sources = bindings_from_args(&sources, name, discovery.as_ref())?;
        let authority_override_map = SliceAuthorityOverride {
            by_kind: authority_override
                .iter()
                .map(|a| (a.kind, a.source.clone()))
                .collect::<BTreeMap<_, _>>(),
        };
        let entry = Entry {
            name: name.into(),
            project,
            status: Status::Pending,
            depends_on: depends_on.into_iter().map(Into::into).collect(),
            sources,
            context,
            description,
            divergence: None,
            disagreements: Vec::new(),
            authority_override: authority_override_map,
        };
        let plan_path = cx.layout().plan_path();
        let now = cx.now();
        let (body, override_events) =
            with_state::<Plan, _, _>(cx.layout(), "plan.yaml", move |plan| {
                plan.create(entry)?;
                validate_plan(plan)?;
                let plan_name = plan.name.clone();
                // Route the seeded overrides through the shared writer
                // (no clears on the add path) so all three handlers emit
                // identically-shaped, identically-sorted Set events.
                let created_entry = entry_mut(plan, &plan_name, name)?.clone();
                let events = emit_authority_override_seed_events(&plan_name, &created_entry, now);
                Ok((
                    EntryBody {
                        plan: plan_ref(plan, &plan_path),
                        action: Action::Create,
                        entry: created_entry,
                    },
                    events,
                ))
            })?;

        journal::append_batch(cx.layout(), &override_events)?;
        Ok(Reply::ok(Out(body)))
    }
}
