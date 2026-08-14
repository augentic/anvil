//! `plan author`: bind a reviewed handoff, focus delivery scopes,
//! decompose, and publish `decomposition.yaml` + `plan.yaml`.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use artifacts::leads::{Lead, Leads};
use error::Error;
use jiff::Timestamp;
use omnia_guest::Model;
use project::adapter::catalog::{self, INTENT, Pin};
use project::adapter::{Inventory, Resolver};
use project::binding::{Location, Locator};
use project::config::{Layout, ProjectConfig};
use project::handler::ExecutionPaths;
use project::journal::{self, Event, EventKind, JournalRoot, read_union_at};
use project::plan::{
    DISCOVERY_VERSION, DefinitionIdentity, Discovery, GateProse, Plan, ReviewIdentity,
    SourceBinding, TargetBinding, build_request, resolve_from, resolve_topology,
    retain_decomposition, retain_leads,
};
use project::profile::Profiles;
use project::seam::{self, Ingest, Source, Workspaces};
use project::snapshot::SnapshotId;
use system::handoff::{self, EvidenceScopeRef, Handoff};

use super::decompose;
use crate::judgment::propose::{self, GateContext};

/// Completed authoring: bound topology, decomposition, and leaf plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorOutcome {
    /// Change name.
    pub plan: String,
    /// Canonical `discovery.yaml` digest.
    pub discovery_digest: SnapshotId,
    /// Canonical `leads.md` digest.
    pub leads_digest: SnapshotId,
    /// Canonical `decomposition.yaml` digest.
    pub decomposition_digest: SnapshotId,
    /// Bound target ids.
    pub targets: Vec<String>,
    /// Bound source keys.
    pub sources: Vec<String>,
    /// Projected slice names, in tree order.
    pub slices: Vec<String>,
}

/// Current reviewed handoff plus its `system.wave.reviewed` envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Reviewed {
    handoff: Handoff,
    digest: SnapshotId,
    review: Event,
    event_digest: SnapshotId,
}

/// Bind a reviewed handoff, decompose it, and publish the leaf plan.
///
/// # Errors
///
/// Definition resolve, ingest, catalog, overwrite, judgment, and
/// validation failures.
#[tracing::instrument(name = "plan.author", skip_all, fields(plan = %name))]
pub async fn author<P>(
    provider: &P, paths: &ExecutionPaths, now: Timestamp, name: &str, from: &Path, wave: &str,
    force: bool,
) -> Result<AuthorOutcome, Error>
where
    P: Resolver + Inventory + Profiles + Ingest + Model + Source + Workspaces,
{
    project::name::validate_name(name)?;
    let from = resolve_from(paths, from);
    let reviewed = load_reviewed(&from, wave)?;
    let layout = paths.layout();
    let plan_path = layout.plan_path();
    let existing = load_existing(&plan_path)?;
    refuse_overwrite(existing.as_ref(), &reviewed, force, &plan_path)?;

    std::fs::create_dir_all(layout.change_root())?;
    copy_imports(&from, &layout, &reviewed)?;

    let catalog = provider.inventory();
    let mut intern = BTreeMap::new();
    let mut targets =
        bind_targets(provider, catalog, &reviewed, existing.as_ref(), &mut intern).await?;
    let sources =
        bind_sources(provider, catalog, &reviewed, existing.as_ref(), &mut intern).await?;
    let imported = reviewed
        .handoff
        .wave
        .evidence_scopes
        .iter()
        .map(|scope| import_lead(&scope.source, scope, &sources))
        .collect();

    let definition = identity(&reviewed, &from);
    let discovery = Discovery {
        version: DISCOVERY_VERSION,
        definition: definition.clone(),
        targets: targets.clone(),
        sources: sources.clone(),
    };
    discovery.validate()?;
    let discovery_digest = discovery.digest()?;
    std::fs::create_dir_all(layout.imports_dir())?;
    discovery.save(&layout.discovery_yaml_path())?;

    let catalog = Leads::from_leads(imported);
    catalog.write_atomic(&layout.leads_path())?;
    let leads_digest = retain_leads(layout)?;

    stamp_profiles(&mut targets, provider.profiles())?;

    let mut plan = Plan::named(name);
    plan.discovery_digest = Some(discovery_digest.clone());
    plan.leads_digest = Some(leads_digest.clone());
    plan.definition = Some(definition);
    plan.targets = targets;
    plan.sources = sources;
    plan.save(&plan_path)?;

    finish(provider, paths, layout, now, name, discovery_digest, leads_digest).await
}

/// Fail closed when the live definition home no longer projects the
/// bound handoff. Lives in `change` so `project` does not depend on
/// `system`.
///
/// # Errors
///
/// `plan-definition-stale` when the home is gone or its current
/// handoff is not the bound digest.
pub fn current_definition(paths: &ExecutionPaths, plan: &Plan) -> Result<(), Error> {
    let Some(def) = &plan.definition else {
        return Ok(());
    };
    let Some(root) = definition_root(paths, def) else {
        return Ok(());
    };
    if !root.is_dir() {
        return Err(stale(format!("definition home `{}` is gone", root.display())));
    }
    let layout = system::Layout::new(&root);
    let current = system::review::current_handoff(&layout, &def.wave_id).map_err(|err| {
        stale(format!(
            "definition home current handoff is not the bound `{bound}`: {err}",
            bound = def.handoff_digest
        ))
    })?;
    if current.digest != def.handoff_digest {
        return Err(stale(format!(
            "definition home current handoff `{}` is not the bound `{bound}`",
            current.digest,
            bound = def.handoff_digest
        )));
    }
    Ok(())
}

fn definition_root(paths: &ExecutionPaths, def: &DefinitionIdentity) -> Option<PathBuf> {
    if let Some(from) = &def.from {
        return Some(resolve_from(paths, Path::new(from)));
    }
    if paths.is_detached() {
        return None;
    }
    let colocated = paths.project_root().join(".emery/system");
    colocated.is_dir().then_some(colocated)
}

const fn stale(detail: String) -> Error {
    Error::Diag {
        code: "plan-definition-stale",
        detail,
    }
}

fn load_reviewed(from: &Path, wave: &str) -> Result<Reviewed, Error> {
    let layout = system::Layout::new(from);
    let projected = system::review::current_handoff(&layout, wave)?;
    let verified = handoff::load(&layout.handoff_path(projected.digest.digest()))?;
    let review = matching_review(&layout, wave, &verified.digest)?;
    let event_digest = review.digest()?;
    Ok(Reviewed {
        handoff: verified.handoff,
        digest: verified.digest,
        review,
        event_digest,
    })
}

fn matching_review(
    layout: &system::Layout<'_>, wave: &str, digest: &SnapshotId,
) -> Result<Event, Error> {
    read_union_at(&JournalRoot::new(layout.events_dir()))?
        .into_iter()
        .find(|event| {
            matches!(
                &event.kind,
                EventKind::SystemWaveReviewed { wave: reviewed, handoff_digest }
                    if reviewed == wave && handoff_digest == digest
            )
        })
        .ok_or_else(|| Error::Diag {
            code: "definition-review-missing",
            detail: format!("no system.wave.reviewed fact names handoff `{digest}`"),
        })
}

async fn finish<P>(
    provider: &P, paths: &ExecutionPaths, layout: Layout<'_>, now: Timestamp, name: &str,
    discovery_digest: SnapshotId, leads_digest: SnapshotId,
) -> Result<AuthorOutcome, Error>
where
    P: Model + Profiles + Resolver + Source + Workspaces,
{
    let leads = Leads::load(&layout.leads_path())?;
    let mut plan = Plan::load(&layout.plan_path())?;

    let decomposed = decompose::decompose(provider, paths, now, &plan, &leads).await?;
    let entries = project::plan::decomposition::slices(&decomposed.tree)?;
    let slice_names: Vec<String> = entries.iter().map(|entry| entry.name.to_string()).collect();
    decomposed.tree.save(&layout.decomposition_path())?;
    let decomposition_digest = retain_decomposition(layout)?;
    plan.decomposition_digest = Some(decomposition_digest.clone());
    plan.leads_digest = Some(decomposed.tree.leads_digest.clone());
    plan.entries = entries;
    plan.save(&layout.plan_path())?;

    let topology = resolve_topology(&plan);
    let request = build_request(&leads, &topology)?;
    let gate = propose::reconcile(
        provider,
        &request,
        Some(GateContext {
            plan: plan.name.as_str(),
            sources: &plan.sources,
        }),
        |answer| {
            let mut trial = plan.clone();
            trial.propose_from(answer.clone(), &leads, &topology, &HashMap::new()).map(|_| ())
        },
    )
    .await?;
    let debt = slice::debt::from_targets(provider, layout, &plan, now).await.unwrap_or_default();
    write_change(layout, &plan, &leads, gate.gate.as_ref(), &decomposed.caveats, &debt)?;
    journal::append_one(
        layout,
        &Event::new(
            now,
            EventKind::PlanReconcileCompleted {
                plan_name: plan.name.clone(),
                slice_count: slice_names.len(),
                slice_names: slice_names.iter().map(|name| name.as_str().into()).collect(),
            },
        ),
    )?;

    Ok(AuthorOutcome {
        plan: name.to_string(),
        discovery_digest,
        leads_digest,
        decomposition_digest,
        targets: plan.targets.keys().cloned().collect(),
        sources: plan.sources.keys().cloned().collect(),
        slices: slice_names,
    })
}

fn write_change(
    layout: Layout<'_>, plan: &Plan, leads: &Leads, gate: Option<&GateProse>, caveats: &[String],
    debt: &[slice::debt::DebtRow],
) -> Result<(), Error> {
    let mut body = format!("# Change — {}\n\n", plan.name);
    body.push_str(&orientation(plan, leads));
    if let Some(gate) = gate {
        body.push('\n');
        body.push_str(gate.change.trim());
        body.push('\n');
    }
    if !caveats.is_empty() {
        body.push_str("\n## Uncertain boundaries\n\n");
        for caveat in caveats {
            let _ = writeln!(body, "- {caveat}");
        }
    }
    if let Some(section) = slice::debt::markdown(debt) {
        body.push('\n');
        body.push_str(&section);
        if !body.ends_with('\n') {
            body.push('\n');
        }
    }
    let path = layout.change_brief_path();
    std::fs::write(&path, body).map_err(|source| Error::Filesystem {
        op: "write",
        path,
        source,
    })
}

fn orientation(plan: &Plan, leads: &Leads) -> String {
    let mut out = format!(
        "## Summary\n\nSources: {}. Leads: {}.\n\n## Source inventory\n\n\
         | Key | Adapter | Binding |\n| --- | --- | --- |\n",
        plan.sources.len(),
        leads.leads().len()
    );
    for (key, binding) in &plan.sources {
        let bound = match (&binding.locator, &binding.value) {
            (Some(locator), _) => format!("locator `{locator}`"),
            (None, Some(value)) => format!("value \"{value}\""),
            (None, None) => "unbound".into(),
        };
        let _ = writeln!(out, "| {key} | {} | {bound} |", binding.adapter);
    }
    out
}

/// One catalog row from a handoff surface lead. Synopsis is the bound
/// intent value when present, otherwise the lead id.
fn import_lead(
    key: &str, scope: &EvidenceScopeRef, sources: &BTreeMap<String, SourceBinding>,
) -> Lead {
    let synopsis = sources
        .get(key)
        .and_then(|binding| binding.value.clone())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| scope.lead.clone());
    Lead::new(scope.lead.clone(), key, synopsis)
}

fn load_existing(plan_path: &Path) -> Result<Option<Plan>, Error> {
    if !plan_path.exists() {
        return Ok(None);
    }
    Ok(Some(Plan::load(plan_path)?))
}

fn refuse_overwrite(
    existing: Option<&Plan>, reviewed: &Reviewed, force: bool, plan_path: &Path,
) -> Result<(), Error> {
    let Some(existing) = existing else {
        return Ok(());
    };
    if !force {
        return Err(Error::Diag {
            code: "plan-already-exists",
            detail: format!(
                "refusing to overwrite existing plan at {}; pass --force to replace it",
                plan_path.display()
            ),
        });
    }
    if let Some(definition) = &existing.definition
        && definition.handoff_digest != reviewed.digest
    {
        return Err(Error::Diag {
            code: "plan-author-handoff-changed",
            detail: " --force rebinds the same reviewed handoff; a changed wave needs a new \
                     handoff and review fact"
                .trim()
                .into(),
        });
    }
    Ok(())
}

fn copy_imports(from: &Path, layout: &Layout<'_>, reviewed: &Reviewed) -> Result<(), Error> {
    let home = system::Layout::new(from);
    let handoff_src = home.handoff_path(reviewed.digest.digest());
    let handoff_dest = layout.import_handoff_path(&reviewed.digest);
    std::fs::create_dir_all(handoff_dest.parent().unwrap_or(&handoff_dest))?;
    std::fs::copy(&handoff_src, &handoff_dest).map_err(|source| Error::Filesystem {
        op: "copy",
        path: handoff_src.clone(),
        source,
    })?;
    let review_dest = layout.import_review_path(&reviewed.event_digest);
    std::fs::create_dir_all(review_dest.parent().unwrap_or(&review_dest))?;
    let line = review_line(&home.events_dir(), reviewed)?;
    std::fs::write(&review_dest, line).map_err(|source| Error::Filesystem {
        op: "write",
        path: review_dest,
        source,
    })?;
    Ok(())
}

fn review_line(events_dir: &Path, reviewed: &Reviewed) -> Result<Vec<u8>, Error> {
    let want = reviewed.event_digest.clone();
    let entries = std::fs::read_dir(events_dir).map_err(|source| Error::Filesystem {
        op: "readdir",
        path: events_dir.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let path = entry
            .map_err(|source| Error::Filesystem {
                op: "readdir",
                path: events_dir.to_path_buf(),
                source,
            })?
            .path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
            continue;
        }
        let text = std::fs::read_to_string(&path).map_err(|source| Error::Filesystem {
            op: "read",
            path: path.clone(),
            source,
        })?;
        for line in text.lines() {
            let Ok(event) = serde_json::from_str::<Event>(line) else {
                continue;
            };
            if event.digest().ok().as_ref() == Some(&want) {
                let mut bytes = line.as_bytes().to_vec();
                bytes.push(b'\n');
                return Ok(bytes);
            }
        }
    }
    let mut bytes = serde_json::to_vec(&reviewed.review).map_err(|err| Error::Diag {
        code: "journal-event-serialise-failed",
        detail: format!("failed to serialise journal event: {err}"),
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn identity(reviewed: &Reviewed, from: &Path) -> DefinitionIdentity {
    DefinitionIdentity {
        system: reviewed.handoff.definition.clone(),
        handoff_digest: reviewed.digest.clone(),
        review: ReviewIdentity {
            writer: reviewed.review.writer.clone(),
            sequence: reviewed.review.sequence,
            event_digest: reviewed.event_digest.clone(),
        },
        system_model_digest: reviewed.handoff.system_model_digest.clone(),
        migration_plan_digest: reviewed.handoff.migration_plan_digest.clone(),
        wave_id: reviewed.handoff.wave.id.clone(),
        from: Some(from.display().to_string()),
    }
}

fn stamp_profiles(
    targets: &mut BTreeMap<String, TargetBinding>, table: &project::profile::Table,
) -> Result<(), Error> {
    let bound = table.resolve()?.reference()?;
    for row in targets.values_mut() {
        row.model_capability_profile = Some(bound.clone());
    }
    Ok(())
}

async fn bind_targets<P: Ingest>(
    ingest: &P, catalog: &catalog::Catalog, reviewed: &Reviewed, existing: Option<&Plan>,
    intern: &mut BTreeMap<String, SnapshotId>,
) -> Result<BTreeMap<String, TargetBinding>, Error> {
    let mut targets = BTreeMap::new();
    for target in &reviewed.handoff.wave.targets {
        let locator = target.locator.clone();
        let recorded = intern.get(&locator).cloned().or_else(|| {
            existing.and_then(|plan| plan.targets.get(&target.id).map(|row| row.cid.clone()))
        });
        let prior = existing
            .and_then(|plan| plan.targets.get(&target.id).and_then(|row| git_sha(&row.locator)));
        let fetched = ingest
            .fetch(locator.clone(), recorded, prior)
            .await
            .map_err(|err| seam::seam_failure("fetch", "ingest", &err))?;
        intern.insert(locator, fetched.cid.clone());
        intern.insert(fetched.locator.clone(), fetched.cid.clone());
        let pin = catalog::fill(catalog, &target.adapter)?;
        validate_target_tree(&fetched.root, &pin)?;
        targets.insert(target.id.clone(), TargetBinding::new(pin, fetched.locator, fetched.cid));
    }
    Ok(targets)
}

async fn bind_sources<P: Ingest>(
    provider: &P, catalog: &catalog::Catalog, reviewed: &Reviewed, existing: Option<&Plan>,
    intern: &mut BTreeMap<String, SnapshotId>,
) -> Result<BTreeMap<String, SourceBinding>, Error> {
    let mut sources = BTreeMap::new();
    for scope in &reviewed.handoff.wave.evidence_scopes {
        if sources.contains_key(&scope.source) {
            continue;
        }
        let pin = catalog::fill(catalog, &scope.adapter)?;
        let locator = scope.location.clone();
        let recorded = intern.get(&locator).cloned().or_else(|| {
            existing.and_then(|plan| {
                plan.sources.values().find_map(|row| {
                    (row.locator.as_deref() == Some(locator.as_str()))
                        .then(|| row.cid.clone())
                        .flatten()
                })
            })
        });
        let fetched = provider
            .fetch(locator.clone(), recorded, None)
            .await
            .map_err(|err| seam::seam_failure("fetch", "ingest", &err))?;
        intern.insert(locator, fetched.cid.clone());
        intern.insert(fetched.locator.clone(), fetched.cid.clone());
        let binding = if pin.name == INTENT || scope.source == INTENT {
            let value = read_intent_value(Path::new(&fetched.root))?;
            SourceBinding::intent(pin, value)
        } else {
            SourceBinding::located(pin, fetched.locator, fetched.cid)
        };
        binding.validate(&scope.source)?;
        sources.insert(scope.source.clone(), binding);
    }
    Ok(sources)
}

fn read_intent_value(root: &Path) -> Result<String, Error> {
    let path = if root.is_file() {
        root.to_path_buf()
    } else {
        let mut files = Vec::new();
        let entries = std::fs::read_dir(root).map_err(|source| Error::Filesystem {
            op: "readdir",
            path: root.to_path_buf(),
            source,
        })?;
        for entry in entries {
            let path = entry
                .map_err(|source| Error::Filesystem {
                    op: "readdir",
                    path: root.to_path_buf(),
                    source,
                })?
                .path();
            if path.is_file() {
                files.push(path);
            }
        }
        match files.len() {
            1 => files.remove(0),
            0 => {
                return Err(Error::Diag {
                    code: "source-intent-locator",
                    detail: format!(
                        "intent location `{}` has no file to read as value",
                        root.display()
                    ),
                });
            }
            n => {
                return Err(Error::Diag {
                    code: "source-intent-locator",
                    detail: format!(
                        "intent location `{}` has {n} files; bind needs a single file",
                        root.display()
                    ),
                });
            }
        }
    };
    let value = std::fs::read_to_string(&path).map_err(|source| Error::Filesystem {
        op: "read",
        path: path.clone(),
        source,
    })?;
    if value.trim().is_empty() {
        return Err(Error::Diag {
            code: "source-intent-locator",
            detail: format!("intent location `{}` is empty", path.display()),
        });
    }
    Ok(value)
}

fn validate_target_tree(root: &str, pin: &Pin) -> Result<(), Error> {
    let root = Path::new(root);
    let config = match ProjectConfig::load(root) {
        Ok(config) => config,
        Err(Error::NotInitialized) => {
            return Err(Error::Diag {
                code: "target-project-missing",
                detail: format!("target tree `{}` has no `.emery/project.yaml`", root.display()),
            });
        }
        Err(err) => return Err(err),
    };
    if config.name.is_empty() {
        return Err(Error::Diag {
            code: "target-project-identity",
            detail: format!("target tree `{}` project.yaml omits `name`", root.display()),
        });
    }
    let Some(adapter) = config.adapter.as_deref() else {
        return Err(Error::Diag {
            code: "target-project-adapter",
            detail: format!(
                "target tree `{}` project.yaml omits a target-axis adapter",
                root.display()
            ),
        });
    };
    let declared = project::adapter::AdapterSelector::parse(adapter)?;
    let declared_name = match &declared {
        project::adapter::AdapterSelector::Bare { name }
        | project::adapter::AdapterSelector::Package { name, .. } => name.as_str(),
        project::adapter::AdapterSelector::Component { .. } => {
            return Err(Error::Diag {
                code: "target-project-adapter",
                detail: format!(
                    "target tree `{}` project.yaml adapter is a local component",
                    root.display()
                ),
            });
        }
    };
    if declared_name != pin.name {
        return Err(Error::Diag {
            code: "target-project-adapter",
            detail: format!(
                "target tree `{}` adapter `{declared_name}` does not match pin `{pin}`",
                root.display()
            ),
        });
    }
    Ok(())
}

fn git_sha(locator: &str) -> Option<String> {
    let parsed = Location::parse(locator, None).ok()?;
    match parsed.locator {
        Locator::Git { revision, .. } if Locator::is_sha(&revision) => Some(revision),
        _ => None,
    }
}
