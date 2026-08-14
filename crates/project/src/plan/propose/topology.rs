//! Plan-topology resolution: `plan.yaml.targets` keys become the
//! request envelope's `projects[]`.

use super::wire::ProjectRef;
use crate::identity;
use crate::plan::Plan;

/// One [`ProjectRef`] per `plan.yaml.targets` key, named by the
/// handoff target id and carrying that row's adapter pin.
///
/// Surface, recent, and decisions stay empty — those projections
/// read a product tree, and the stored topology is the target map.
#[must_use]
pub fn resolve_topology(plan: &Plan) -> Vec<ProjectRef> {
    plan.targets
        .iter()
        .map(|(id, row)| ProjectRef {
            name: id.clone(),
            target: identity::target_ref(&row.adapter.name, Some(&row.adapter.version)),
            description: None,
            surface: Vec::new(),
            recent: Vec::new(),
            decisions: Vec::new(),
            decisions_more: None,
            platforms: Vec::new(),
        })
        .collect()
}
