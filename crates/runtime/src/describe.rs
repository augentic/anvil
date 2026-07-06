//! Host-side `describe` dispatch (RFC-64).
//!
//! An adapter component's metadata lives in its own deterministic
//! `describe` export. The resolver (in `specify-workflow`, wasmtime-free)
//! obtains it through a registered runner backed by this module: a
//! minimal, backend-free wasmtime instantiation — instance-per-call —
//! that invokes `describe` on the component's axis interface and decodes
//! the returned `manifest` record dynamically.
//!
//! Deliberately *not* an omnia deployment: `DeploymentBuilder::build`
//! initialises process-global telemetry, which may only happen once per
//! process and belongs to the composed guest run (`drive`). `describe`
//! is effect-free by contract (no model call, no filesystem access, no
//! I/O), so *every* import — WASI included — is stubbed as a trap that
//! can never fire on a conforming adapter. Trapping WASI outright
//! (rather than linking a real empty-context WASI) also sidesteps the
//! mixed-version imports a wasip2 guest carries (the std adapter's
//! `wasi:*@0.2.0` beside wit-bindgen's newer `wasi:*@0.2.x`), which the
//! linker's semver aliasing cannot host side-by-side without
//! shadowing.

use std::path::Path;

use anyhow::{Result, bail};
use omnia::RuntimeOptions;
use omnia::wasmtime::component::{Component, Linker, Val};
use omnia::wasmtime::error::Context as _;
use omnia::wasmtime::{Config, Engine, Store};

/// Which axis interface to invoke `describe` on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescribeAxis {
    /// `augentic:specify/source@0.1.0`.
    Source,
    /// `augentic:specify/target@0.1.0`.
    Target,
}

impl DescribeAxis {
    /// The fully-qualified instance name this axis exports.
    #[must_use]
    pub const fn interface(self) -> &'static str {
        match self {
            Self::Source => "augentic:specify/source@0.1.0",
            Self::Target => "augentic:specify/target@0.1.0",
        }
    }

    /// The opposite axis's instance name — probed to distinguish an
    /// axis mismatch from a non-adapter component.
    const fn other_interface(self) -> &'static str {
        match self {
            Self::Source => Self::Target.interface(),
            Self::Target => Self::Source.interface(),
        }
    }
}

/// The decoded `describe` answer, string-typed so this crate carries no
/// workflow vocabulary. The caller (the `specify` binary's registered
/// describe runner) projects it onto the typed workflow shapes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DescribeValue {
    /// `specify-floor` — the optional host-CLI compatibility floor.
    pub specify_floor: Option<String>,
    /// Target-declared build inputs (`path`, `required`); empty for
    /// source adapters.
    pub inputs: Vec<DescribeInput>,
    /// Target platforms capability; absent for source adapters and
    /// platform-agnostic targets.
    pub platforms: Option<DescribePlatforms>,
}

/// One `build-input` record from a target's `describe` answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescribeInput {
    /// Slice-tree-relative path of the input.
    pub path: String,
    /// Whether the build must abort when the path is absent.
    pub required: bool,
}

/// The `platforms-capability` record from a target's `describe` answer,
/// platform tokens carried as their kebab-case WIT enum names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescribePlatforms {
    /// Whether projects must declare a platform set.
    pub required: bool,
    /// Platform tokens this target accepts.
    pub allowed: Vec<String>,
    /// Default platform set for greenfield scaffolding.
    pub default: Vec<String>,
}

/// Typed failure split so the caller can map an axis mismatch onto its
/// own diagnostic code without string-matching.
#[derive(Debug)]
pub enum DescribeFailure {
    /// The component is a valid adapter but exports the *other* axis
    /// interface.
    AxisMismatch {
        /// The interface the caller expected.
        expected: &'static str,
        /// The axis interface the component actually exports.
        found: &'static str,
    },
    /// Any other failure: load, instantiation, call, or decode.
    Other(anyhow::Error),
}

impl std::fmt::Display for DescribeFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AxisMismatch { expected, found } => {
                write!(f, "component exports `{found}`, not the expected `{expected}`")
            }
            Self::Other(err) => write!(f, "{err:#}"),
        }
    }
}

/// Sniff which axis interface a component file exports, without
/// compiling it.
///
/// One wasmparser pass over the top-level component export names.
/// Returns `None` when the file exports neither axis (not an adapter)
/// or, defensively, both.
///
/// Used by the guest-leg discovery scan, where an unbound component in
/// the project component cache carries no binding axis to trust.
///
/// # Errors
///
/// Returns an error when the file cannot be read or is not a
/// WebAssembly component.
pub fn sniff_axis(component: &Path) -> Result<Option<DescribeAxis>> {
    let bytes = std::fs::read(component)
        .with_context(|| format!("reading component {}", component.display()))?;
    let mut source = false;
    let mut target = false;
    for payload in wasmparser::Parser::new(0).parse_all(&bytes) {
        let payload =
            payload.with_context(|| format!("parsing component {}", component.display()))?;
        let wasmparser::Payload::ComponentExportSection(reader) = payload else {
            continue;
        };
        for export in reader {
            let export =
                export.with_context(|| format!("parsing component {}", component.display()))?;
            match export.name.name {
                name if name == DescribeAxis::Source.interface() => source = true,
                name if name == DescribeAxis::Target.interface() => target = true,
                _ => {}
            }
        }
    }
    Ok(match (source, target) {
        (true, false) => Some(DescribeAxis::Source),
        (false, true) => Some(DescribeAxis::Target),
        _ => None,
    })
}

/// Instantiate the adapter component at `component` and invoke
/// `describe(adapter_id)` on the `axis` interface.
///
/// Instance-per-call: engine, linker, store, and instance are built
/// fresh and dropped on return; the caller caches the answer against the
/// component digest.
///
/// # Errors
///
/// [`DescribeFailure::AxisMismatch`] when the component exports the
/// other axis; [`DescribeFailure::Other`] for load, instantiation,
/// call, or decode failures.
pub fn describe_adapter(
    component: &Path, axis: DescribeAxis, adapter_id: &str,
) -> Result<DescribeValue, DescribeFailure> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .map_err(|err| DescribeFailure::Other(err.into()))?;
    runtime.block_on(dispatch(component, axis, adapter_id))
}

async fn dispatch(
    component_path: &Path, axis: DescribeAxis, adapter_id: &str,
) -> Result<DescribeValue, DescribeFailure> {
    let options = RuntimeOptions::load().map_err(DescribeFailure::Other)?;
    let config = Config::from(&options);
    let engine = Engine::new(&config).map_err(|err| DescribeFailure::Other(err.into()))?;
    let component = Component::from_file(&engine, component_path)
        .map_err(|err| DescribeFailure::Other(err.into()))?;

    check_axis(&engine, &component, axis)?;

    call_describe(&engine, &options, &component, axis, adapter_id)
        .await
        .map_err(DescribeFailure::Other)
}

/// Verify the component exports the requested axis interface before
/// instantiating, so a wrong-axis binding fails as a typed mismatch
/// rather than a generic missing-export error mid-call.
fn check_axis(
    engine: &Engine, component: &Component, axis: DescribeAxis,
) -> Result<(), DescribeFailure> {
    let exports: Vec<String> =
        component.component_type().exports(engine).map(|(name, _)| name.to_owned()).collect();
    if exports.iter().any(|name| name == axis.interface()) {
        return Ok(());
    }
    if exports.iter().any(|name| name == axis.other_interface()) {
        return Err(DescribeFailure::AxisMismatch {
            expected: axis.interface(),
            found: axis.other_interface(),
        });
    }
    Err(DescribeFailure::Other(anyhow::anyhow!(
        "component exports no `augentic:specify` axis interface (found: {})",
        exports.join(", "),
    )))
}

async fn call_describe(
    engine: &Engine, options: &RuntimeOptions, component: &Component, axis: DescribeAxis,
    adapter_id: &str,
) -> Result<DescribeValue> {
    let mut linker: Linker<()> = Linker::new(engine);
    // `describe` is effect-free by contract: every import — WASI and
    // non-WASI (`omnia:model`, `wasi:http`, …) alike — is stubbed as a
    // trap that a conforming adapter can never reach from `describe`.
    // Shadowing is enabled because a wasip2 guest imports the same WASI
    // interface at two versions (std's 0.2.0 and wit-bindgen's newer
    // 0.2.x), which collide on the linker's semver alias key.
    linker.allow_shadowing(true);
    linker
        .define_unknown_imports_as_traps(component)
        .context("stubbing imports for the describe instance")?;

    let mut store = Store::new(engine, ());
    store.set_epoch_deadline(1);
    store.epoch_deadline_async_yield_and_update(1);
    if options.max_fuel > 0 {
        let _unused = store.set_fuel(options.max_fuel);
    }

    let instance = linker
        .instantiate_async(&mut store, component)
        .await
        .context("instantiating the adapter component")?;

    let interface_idx = instance
        .get_export_index(&mut store, None, axis.interface())
        .with_context(|| format!("component exports no `{}` instance", axis.interface()))?;
    let func_idx = instance
        .get_export_index(&mut store, Some(&interface_idx), "describe")
        .with_context(|| format!("`{}` exports no `describe`", axis.interface()))?;
    let func = instance
        .get_func(&mut store, func_idx)
        .with_context(|| format!("`{}/describe` is not a function", axis.interface()))?;

    let args = vec![Val::String(adapter_id.to_string())];
    let mut results = vec![Val::Bool(false)];
    func.call_async(&mut store, &args, &mut results).await.context("calling `describe`")?;

    decode_manifest(&results[0])
}

/// Decode the WIT `manifest` record (either axis) from its dynamic
/// [`Val`] representation.
fn decode_manifest(value: &Val) -> Result<DescribeValue> {
    let Val::Record(fields) = value else {
        bail!("`describe` returned a non-record value");
    };
    let mut answer = DescribeValue::default();
    for (name, field) in fields {
        match name.as_str() {
            "specify-floor" => answer.specify_floor = decode_option_string(field)?,
            "inputs" => answer.inputs = decode_inputs(field)?,
            "platforms" => answer.platforms = decode_platforms(field)?,
            other => bail!("`describe` answer carries an unknown field `{other}`"),
        }
    }
    Ok(answer)
}

fn decode_option_string(value: &Val) -> Result<Option<String>> {
    match value {
        Val::Option(None) => Ok(None),
        Val::Option(Some(inner)) => match inner.as_ref() {
            Val::String(text) => Ok(Some(text.clone())),
            _ => bail!("expected an optional string"),
        },
        _ => bail!("expected an option value"),
    }
}

fn decode_inputs(value: &Val) -> Result<Vec<DescribeInput>> {
    let Val::List(items) = value else {
        bail!("`inputs` is not a list");
    };
    items
        .iter()
        .map(|item| {
            let Val::Record(fields) = item else {
                bail!("`inputs[]` entry is not a record");
            };
            let mut path = None;
            let mut required = None;
            for (name, field) in fields {
                match (name.as_str(), field) {
                    ("path", Val::String(text)) => path = Some(text.clone()),
                    ("required", Val::Bool(flag)) => required = Some(*flag),
                    _ => bail!("unexpected `inputs[]` field `{name}`"),
                }
            }
            Ok(DescribeInput {
                path: path.context("`inputs[]` entry is missing `path`")?,
                required: required.context("`inputs[]` entry is missing `required`")?,
            })
        })
        .collect()
}

fn decode_platforms(value: &Val) -> Result<Option<DescribePlatforms>> {
    let inner = match value {
        Val::Option(None) => return Ok(None),
        Val::Option(Some(inner)) => inner.as_ref(),
        _ => bail!("`platforms` is not an option"),
    };
    let Val::Record(fields) = inner else {
        bail!("`platforms` is not a record");
    };
    let mut required = None;
    let mut allowed = None;
    let mut default = None;
    for (name, field) in fields {
        match (name.as_str(), field) {
            ("required", Val::Bool(flag)) => required = Some(*flag),
            ("allowed", list) => allowed = Some(decode_platform_list(list)?),
            ("default", list) => default = Some(decode_platform_list(list)?),
            _ => bail!("unexpected `platforms` field `{name}`"),
        }
    }
    Ok(Some(DescribePlatforms {
        required: required.context("`platforms` is missing `required`")?,
        allowed: allowed.context("`platforms` is missing `allowed`")?,
        default: default.context("`platforms` is missing `default`")?,
    }))
}

fn decode_platform_list(value: &Val) -> Result<Vec<String>> {
    let Val::List(items) = value else {
        bail!("platform set is not a list");
    };
    items
        .iter()
        .map(|item| match item {
            Val::Enum(name) => Ok(name.clone()),
            _ => bail!("platform entry is not an enum value"),
        })
        .collect()
}
