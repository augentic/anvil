//! Wave-binding phase of `plan author`: import a reviewed handoff,
//! ingest locators, write `discovery.yaml`, and stop until decomposition.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use artifacts::leads::{Lead, Leads};
use error::Error;
use project::adapter::catalog::{self, Hint, Pin, Row};
use project::adapter::{Inventory, Resolver};
use project::binding::{Location, Origin, Policy};
use project::config::{Layout, ProjectConfig};
use project::definition::{self, Home, INTENT, Reviewed, Scope};
use project::handler::ExecutionPaths;
use project::plan::{
    DISCOVERY_VERSION, DefinitionIdentity, Discovery, Plan, ReviewIdentity, SourceBinding,
    TargetBinding, retain_leads,
};
use project::profile::Profiles;
use project::seam::{self, Ingest};
use project::snapshot::SnapshotId;

/// Binding-phase result: topology written, decomposition still pending.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorOutcome {
    /// Change name.
    pub plan: String,
    /// Canonical `discovery.yaml` digest.
    pub discovery_digest: SnapshotId,
    /// Canonical `leads.md` digest.
    pub leads_digest: SnapshotId,
    /// Bound target ids.
    pub targets: Vec<String>,
    /// Bound source keys.
    pub sources: Vec<String>,
    /// The typed stop until decomposition lands.
    pub pending: String,
    /// Operator hint.
    pub hint: String,
}

/// Bind a reviewed handoff into `discovery.yaml` and a skeleton `plan.yaml`.
///
/// # Errors
///
/// Definition resolve, ingest, catalog, overwrite, and validation failures.
#[tracing::instrument(name = "plan.author", skip_all, fields(plan = %name))]
pub async fn author<P: Resolver + Inventory + Profiles + Ingest>(
    provider: &P, paths: &ExecutionPaths, name: &str, from: &Path, wave: &str, force: bool,
) -> Result<AuthorOutcome, Error> {
    project::name::validate_name(name)?;
    let from = resolve_from(paths, from);
    let reviewed = definition::resolve(&from, wave)?;
    let layout = paths.layout();
    let plan_path = layout.plan_path();
    let existing = load_existing(&plan_path)?;
    refuse_overwrite(existing.as_ref(), &reviewed, force, &plan_path)?;

    std::fs::create_dir_all(layout.change_root())?;
    copy_imports(&from, &layout, &reviewed)?;

    let catalog = provider.inventory();
    let mut intern = BTreeMap::new();
    let mut targets = bind_targets(provider, &reviewed, existing.as_ref(), &mut intern).await?;
    let (source_rows, source_pins) =
        bind_sources(provider, catalog, paths, &reviewed, existing.as_ref(), &mut intern).await?;
    let prior = prior_keys(existing.as_ref());
    let keys = catalog::assign(&source_rows, &prior)?;
    let scopes = &reviewed.handoff.wave.evidence_scopes;
    let mut sources = BTreeMap::new();
    let mut imported = Vec::with_capacity(keys.len());
    for ((key, row), scope) in keys.into_iter().zip(source_rows).zip(scopes) {
        let binding = match row.origin {
            Origin::Value(value) => SourceBinding::intent(row.pin, value),
            Origin::Location(_) => {
                let (locator, cid) = source_pins
                    .get(&catalog::identity(&row))
                    .cloned()
                    .ok_or_else(|| Error::Diag {
                        code: "source-pin-missing",
                        detail: format!("source `{key}` has no cid pin after ingest"),
                    })?;
                SourceBinding::located(row.pin, locator, cid)
            }
        };
        binding.validate(&key)?;
        imported.push(import_lead(&key, scope));
        sources.insert(key, binding);
    }

    let definition = identity(&reviewed);
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

    Ok(AuthorOutcome {
        plan: name.to_string(),
        discovery_digest,
        leads_digest,
        targets: plan.targets.keys().cloned().collect(),
        sources: plan.sources.keys().cloned().collect(),
        pending: "decomposition".into(),
        hint: "decomposition pending; later authoring phases land with the decomposition step"
            .into(),
    })
}

/// One catalog row from a handoff evidence scope. Synopsis is the
/// inline value for `intent`, otherwise the lead id — the handoff
/// carries no survey-grade headline.
fn import_lead(key: &str, scope: &Scope) -> Lead {
    let synopsis = scope
        .value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(scope.lead.as_str())
        .to_string();
    Lead::new(scope.lead.clone(), key, synopsis)
}

fn resolve_from(paths: &ExecutionPaths, from: &Path) -> PathBuf {
    if from.is_absolute() {
        return from.to_path_buf();
    }
    if paths.is_detached() {
        paths.change_root().join(from)
    } else {
        paths.project_root().join(from)
    }
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
    let home = Home::new(from);
    let handoff_src = home.handoff_path(&reviewed.digest);
    let handoff_dest = layout.import_handoff_path(&reviewed.digest);
    std::fs::create_dir_all(handoff_dest.parent().unwrap_or(&handoff_dest))?;
    std::fs::copy(&handoff_src, &handoff_dest).map_err(|source| Error::Filesystem {
        op: "copy",
        path: handoff_src.clone(),
        source,
    })?;
    let review_dest = layout.import_review_path(&reviewed.event_digest);
    std::fs::create_dir_all(review_dest.parent().unwrap_or(&review_dest))?;
    let events_dir = home.events_dir();
    let line = review_line(&events_dir, reviewed)?;
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
            let Ok(event) = serde_json::from_str::<project::journal::Event>(line) else {
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

fn identity(reviewed: &Reviewed) -> DefinitionIdentity {
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
    ingest: &P, reviewed: &Reviewed, existing: Option<&Plan>,
    intern: &mut BTreeMap<String, SnapshotId>,
) -> Result<BTreeMap<String, TargetBinding>, Error> {
    let mut targets = BTreeMap::new();
    for target in &reviewed.handoff.wave.targets {
        let recorded = intern.get(&target.locator).cloned().or_else(|| {
            existing.and_then(|plan| plan.targets.get(&target.id).map(|row| row.cid.clone()))
        });
        let prior = existing
            .and_then(|plan| plan.targets.get(&target.id).and_then(|row| git_sha(&row.locator)));
        let fetched = ingest
            .fetch(target.locator.clone(), recorded, prior)
            .await
            .map_err(|err| seam::seam_failure("fetch", "ingest", &err))?;
        intern.insert(target.locator.clone(), fetched.cid.clone());
        intern.insert(fetched.locator.clone(), fetched.cid.clone());
        let pin = Pin::parse(&target.adapter)?;
        validate_target_tree(&fetched.root, &pin)?;
        targets.insert(target.id.clone(), TargetBinding::new(pin, fetched.locator, fetched.cid));
    }
    Ok(targets)
}

async fn bind_sources<P: Ingest + Resolver>(
    provider: &P, catalog: &catalog::Catalog, paths: &ExecutionPaths, reviewed: &Reviewed,
    existing: Option<&Plan>, intern: &mut BTreeMap<String, SnapshotId>,
) -> Result<(Vec<Row>, BTreeMap<String, (String, SnapshotId)>), Error> {
    let mut rows = Vec::new();
    let mut pins = BTreeMap::new();
    let policy = Policy::standard();
    let mut meter = project::binding::Meter::new();
    for scope in &reviewed.handoff.wave.evidence_scopes {
        if scope.source == INTENT || scope.value.is_some() {
            let value = scope.value.clone().ok_or_else(|| Error::Diag {
                code: "source-intent-locator",
                detail: "adapter `intent` is inline `value` only; a locator is refused".into(),
            })?;
            if scope.locator.as_ref().is_some_and(|locator| !locator.is_empty()) {
                return Err(Error::Diag {
                    code: "source-intent-locator",
                    detail: "adapter `intent` is inline `value` only; a locator is refused".into(),
                });
            }
            let pin = intent_pin(scope, catalog)?;
            rows.push(Row {
                origin: Origin::Value(value),
                pin,
            });
            continue;
        }
        let locator =
            scope.locator.as_deref().filter(|locator| !locator.is_empty()).ok_or_else(|| {
                Error::Diag {
                    code: "source-locator-missing",
                    detail: format!(
                        "evidence scope `{}` is location-backed and needs a locator",
                        scope.source
                    ),
                }
            })?;
        let recorded =
            intern.get(locator).cloned().or_else(|| scope.source_cid.clone()).or_else(|| {
                existing.and_then(|plan| {
                    plan.sources.values().find_map(|row| {
                        (row.locator.as_deref() == Some(locator)).then(|| row.cid.clone()).flatten()
                    })
                })
            });
        let fetched = provider
            .fetch(locator.to_string(), recorded, None)
            .await
            .map_err(|err| seam::seam_failure("fetch", "ingest", &err))?;
        intern.insert(locator.to_string(), fetched.cid.clone());
        intern.insert(fetched.locator.clone(), fetched.cid.clone());
        let origin = Origin::Location(Location::parse(&fetched.locator, None)?);
        let pin = select_source(
            provider,
            catalog,
            paths,
            scope,
            Path::new(&fetched.root),
            &origin,
            Some((&mut meter, &policy)),
        )?;
        let row = Row { origin, pin };
        let id = catalog::identity(&row);
        pins.insert(id, (fetched.locator, fetched.cid));
        rows.push(row);
    }
    Ok((rows, pins))
}

fn intent_pin(scope: &Scope, catalog: &catalog::Catalog) -> Result<Pin, Error> {
    scope.adapter.as_deref().map(str::trim).filter(|adapter| !adapter.is_empty()).map_or_else(
        || {
            catalog
                .sources
                .iter()
                .find(|source| source.pin.name == INTENT)
                .map(|source| source.pin.clone())
                .ok_or_else(|| Error::Diag {
                    code: "adapter-catalog-invalid",
                    detail: "catalog has no `intent` source".into(),
                })
        },
        Pin::parse,
    )
}

fn select_source<R: Resolver>(
    resolver: &R, catalog: &catalog::Catalog, paths: &ExecutionPaths, scope: &Scope, root: &Path,
    origin: &Origin, budget: Option<(&mut project::binding::Meter, &Policy)>,
) -> Result<Pin, Error> {
    let adapter = scope.adapter.as_deref().map(str::trim).filter(|adapter| !adapter.is_empty());
    let hint = adapter.map_or(Hint::Open(root), Hint::Pin);
    let pin = catalog::select_metered(catalog, hint, origin, budget)?;
    if adapter.is_none() {
        let bound = resolver.resolve_source(&pin.selector(), paths)?;
        return Ok(catalog::overlay(pin, bound.manifest.version));
    }
    Ok(pin)
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

fn prior_keys(existing: Option<&Plan>) -> BTreeMap<String, String> {
    let Some(existing) = existing else {
        return BTreeMap::new();
    };
    existing
        .sources
        .iter()
        .map(|(key, binding)| {
            let origin = binding_origin(binding);
            let row = Row {
                origin,
                pin: binding.adapter.clone(),
            };
            (catalog::identity(&row), key.clone())
        })
        .collect()
}

fn binding_origin(binding: &SourceBinding) -> Origin {
    if let Some(value) = &binding.value {
        return Origin::Value(value.clone());
    }
    binding
        .locator
        .as_deref()
        .and_then(|locator| Location::parse(locator, None).ok())
        .map_or_else(|| Origin::Value(String::new()), Origin::Location)
}

fn git_sha(locator: &str) -> Option<String> {
    let parsed = Location::parse(locator, None).ok()?;
    match parsed.locator {
        project::binding::Locator::Git { revision, .. }
            if project::binding::Locator::is_sha(&revision) =>
        {
            Some(revision)
        }
        _ => None,
    }
}
