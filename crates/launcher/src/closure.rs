//! Per-invocation component-closure derivation.
//!
//! The closure is the superset of adapters the command could dispatch,
//! scoped to the routed verb's reach ([`ClosureScope`], classified
//! per route in `transport::command::selectors::SCOPE_ROUTES`): the
//! engine guest always; the project's bound target
//! (`project.yaml.adapter`), workspace-slot targets (each
//! materialised slot's own `project.yaml.adapter` — the authoritative
//! input the guest re-derives topology from, not the derived
//! `topology.lock`), and the plan-bound sources
//! (`plan.yaml.sources.<key>`) each only when the verb can dispatch
//! that leg; and the selectors argv itself carries, always. Read-only
//! verbs therefore deploy the engine guest alone and never touch the
//! store or the registry. Inputs the guest would reject anyway (an
//! uninitialised project, an unparseable selector, a
//! foreign-namespace source binding) contribute nothing — the guest
//! renders its canonical diagnostic before any dispatch could occur,
//! so the closure-superset invariant holds without duplicating its
//! validation here.

use std::path::{Path, PathBuf};

use project::adapter::{AdapterSelector, Axis, FIRST_PARTY_NAMESPACE};
use project::config::{Layout, ProjectConfig};
use project::plan::Plan;
use project::registry::Registry;
use transport::command::selectors::{ClosureScope, CommandSelectors};

use crate::Engine;

/// The unresolved closure: the engine pin plus adapter requirements.
#[derive(Debug)]
pub struct Closure {
    /// The engine package pin (`specify:engine@<binary version>`).
    pub engine: AdapterSelector,
    /// Adapter requirements, in derivation order; duplicates are
    /// merged after resolution, when component paths are known.
    pub adapters: Vec<Requirement>,
}

/// One adapter the deployment must be able to dispatch.
#[derive(Debug)]
pub struct Requirement {
    pub axis: Axis,
    pub name: String,
    pub selector: AdapterSelector,
    /// Slot-local resolution root: a workspace-slot binding resolves
    /// against the slot's own tree (its component cache, its relative
    /// component paths), so hydration re-anchors at this directory.
    /// `None` resolves at the anchored project root.
    pub root: Option<PathBuf>,
}

impl Requirement {
    /// The deployment guest id — `<axis>:<name>`, the routed adapter
    /// id the engine names on every seam call.
    pub fn guest_id(&self) -> String {
        format!("{}:{}", self.axis.prefix(), self.name)
    }
}

/// The engine's own package identity in the global store.
pub const ENGINE_NAME: &str = "engine";

/// Derive the closure for one invocation, joining only the
/// state-derived legs the routed verb can dispatch
/// ([`CommandSelectors::scope`]); argv selectors always join. The
/// engine pin is the composition root's injected [`Engine`] — the
/// binary's own version, never this crate's.
pub fn compute(root: &Path, selectors: &CommandSelectors, engine: Engine) -> Closure {
    let ClosureScope {
        project_target,
        plan_sources,
        slot_targets: slots,
    } = selectors.scope;
    let mut adapters = Vec::new();

    if project_target
        && let Ok(config) = ProjectConfig::load(root)
        && let Some(adapter) = config.adapter
    {
        push(&mut adapters, Axis::Target, AdapterSelector::parse(&adapter).ok());
    }

    if slots {
        for (slot_dir, slot_target) in slot_targets(root) {
            push_rooted(&mut adapters, Axis::Target, Some(slot_target), Some(slot_dir));
        }
    }

    if plan_sources && let Ok(plan) = Plan::load(&Layout::new(root).plan_path()) {
        for binding in plan.sources.values() {
            push(&mut adapters, Axis::Source, Some(binding.selector()));
        }
    }

    for token in &selectors.targets {
        push(&mut adapters, Axis::Target, AdapterSelector::parse(token).ok());
    }
    for token in &selectors.sources {
        push(&mut adapters, Axis::Source, source_selector(token));
    }

    Closure {
        engine: engine_selector(engine.version),
        adapters,
    }
}

/// The engine package pin for the injected binary version.
fn engine_selector(version: &str) -> AdapterSelector {
    AdapterSelector::Package {
        namespace: FIRST_PARTY_NAMESPACE.to_string(),
        name: ENGINE_NAME.to_string(),
        version: semver::Version::parse(version)
            .expect("the composition root supplies its exact CARGO_PKG_VERSION"),
    }
}

fn push(adapters: &mut Vec<Requirement>, axis: Axis, selector: Option<AdapterSelector>) {
    push_rooted(adapters, axis, selector, None);
}

fn push_rooted(
    adapters: &mut Vec<Requirement>, axis: Axis, selector: Option<AdapterSelector>,
    root: Option<PathBuf>,
) {
    let Some(selector) = selector else {
        return;
    };
    let Ok(name) = selector.name() else {
        return;
    };
    adapters.push(Requirement {
        axis,
        name,
        selector,
        root,
    });
}

/// Interpret one projected source-axis token the way the plan grammar
/// does: a bare development name or a first-party exact pin. Anything
/// else (foreign namespaces, component paths) contributes nothing —
/// the guest rejects the binding with `plan-source-adapter-invalid`.
fn source_selector(token: &str) -> Option<AdapterSelector> {
    match AdapterSelector::parse(token).ok()? {
        bare @ AdapterSelector::Bare { .. } => Some(bare),
        package @ AdapterSelector::Package { .. } => match &package {
            AdapterSelector::Package { namespace, .. } if namespace == FIRST_PARTY_NAMESPACE => {
                Some(package)
            }
            _ => None,
        },
        AdapterSelector::Component { .. } => None,
    }
}

/// Workspace-slot targets from the authoritative inputs, mirroring the
/// guest's own topology walk (`registry::topology::cache_staleness`):
/// `registry.yaml` names the members; each materialised slot's own
/// `.specify/project.yaml` binds the target the guest would dispatch.
/// The derived `.specify/topology.lock` is never consulted — a stale
/// lock must not make the launcher enumerate an identity the guest no
/// longer resolves (or miss one it does).
///
/// A slot whose `project.yaml` is missing or unreadable contributes
/// nothing — parity with the guest's degrade to the
/// `workspace-slot-config-unreadable` finding. Together with the
/// hydration-time skip of slot bindings without a resolvable artifact,
/// the closure-superset invariant holds for every slot binding the
/// guest itself can resolve; only bindings the guest would also fail
/// on are excluded.
fn slot_targets(root: &Path) -> Vec<(PathBuf, AdapterSelector)> {
    let Ok(Some(registry)) = Registry::load(root) else {
        return Vec::new();
    };
    registry
        .projects
        .iter()
        .filter_map(|project| {
            let slot_dir = root.join("workspace").join(&project.name);
            let adapter = ProjectConfig::load(&slot_dir).ok()?.adapter?;
            let selector = AdapterSelector::parse(&adapter).ok()?;
            Some((slot_dir, selector))
        })
        .collect()
}
