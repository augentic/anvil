//! Wasmtime-backed WASI Preview 2 runner boundary.

use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};

use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Cache, CacheConfig, Config, Engine, Store};
use wasmtime_wasi::p2::bindings::sync::Command;
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;
use wasmtime_wasi::{DirPerms, FilePerms, I32Exit, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

use crate::error::ExtensionError;
use crate::manifest::ExtensionScope;
use crate::permissions::{canonicalise_under, deny_lifecycle_write, substitute};
use crate::resolver::ResolvedExtension;

/// Host-side context for running a resolved tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunContext {
    /// Project root used for `$PROJECT_DIR` and permission-root checks.
    pub project_dir: PathBuf,
    /// Canonical or canonicalisable capability root for plugin-scope tools.
    pub capability_dir: Option<PathBuf>,
    /// Arguments forwarded after `argv[0]`, which is always the tool name.
    pub args: Vec<String>,
}

impl RunContext {
    /// Construct a run context with inherited stdio.
    #[must_use]
    pub fn new(project_dir: impl Into<PathBuf>, args: Vec<String>) -> Self {
        Self {
            project_dir: project_dir.into(),
            capability_dir: None,
            args,
        }
    }

    /// Attach a capability root for plugin-scope tools.
    #[must_use]
    pub fn with_capability_dir(mut self, capability_dir: impl Into<PathBuf>) -> Self {
        self.capability_dir = Some(capability_dir.into());
        self
    }
}

/// Wasmtime-backed synchronous WASI Preview 2 runner.
pub struct WasiRunner {
    engine: Engine,
}

impl std::fmt::Debug for WasiRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `wasmtime::Engine` is not `Debug`; expose the wrapper shape only.
        f.debug_struct("WasiRunner").finish_non_exhaustive()
    }
}

impl WasiRunner {
    /// Construct a reusable Wasmtime engine for WASI Preview 2 components.
    ///
    /// # Errors
    ///
    /// Returns a runtime error when the engine configuration cannot be built.
    pub fn new() -> Result<Self, ExtensionError> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        if let Some(cache) = compilation_cache() {
            config.cache(Some(cache));
        }
        let engine = Engine::new(&config).map_err(|err| ExtensionError::Diag {
            code: "tool-runtime",
            detail: format!("failed to create Wasmtime engine: {err}"),
        })?;
        Ok(Self { engine })
    }

    /// Run a resolved WASI tool and return the guest's process-style exit code.
    ///
    /// # Errors
    ///
    /// Returns permission errors before instantiation, or runtime errors when
    /// Wasmtime cannot compile, link, instantiate, or execute the component.
    pub fn run(
        &self, resolved: &ResolvedExtension, ctx: &RunContext,
    ) -> Result<i32, ExtensionError> {
        self.invoke(resolved, ctx, Stdio::Inherit).map(|outcome| outcome.exit_code)
    }

    /// Run a resolved WASI tool with stdout / stderr captured in memory.
    ///
    /// Mirrors [`Self::run`] but redirects the guest's stdout and stderr
    /// into capped [`MemoryOutputPipe`] buffers so the host can examine
    /// the output without printing to the inherited terminal. Used by
    /// `specify lint`'s `kind: tool` evaluator to fold a tool's
    /// `DiagnosticReport` envelope into the scan output (`kind: tool` evaluator contract).
    ///
    /// # Errors
    ///
    /// Same failure modes as [`Self::run`]; the captured buffers are
    /// dropped on error.
    pub fn run_captured(
        &self, resolved: &ResolvedExtension, ctx: &RunContext,
    ) -> Result<CapturedOutput, ExtensionError> {
        let stdout = MemoryOutputPipe::new(CAPTURE_BUFFER_BYTES);
        let stderr = MemoryOutputPipe::new(CAPTURE_BUFFER_BYTES);
        let outcome =
            self.invoke(resolved, ctx, Stdio::Captured(stdout.clone(), stderr.clone()))?;
        Ok(CapturedOutput {
            stdout: stdout.contents().to_vec(),
            stderr: stderr.contents().to_vec(),
            exit_code: outcome.exit_code,
        })
    }

    fn invoke(
        &self, resolved: &ResolvedExtension, ctx: &RunContext, stdio: Stdio,
    ) -> Result<Outcome, ExtensionError> {
        let project_dir = canonical_project_dir(&ctx.project_dir)?;
        let capability_dir =
            canonical_capability_dir(&resolved.scope, ctx.capability_dir.as_deref())?;
        let preopens = prepare_preopens(resolved, &project_dir, capability_dir.as_deref())?;
        let wasi = build_wasi_ctx(
            resolved,
            ctx,
            &project_dir,
            capability_dir.as_deref(),
            &preopens,
            stdio,
        )?;

        let component =
            Component::from_file(&self.engine, &resolved.bytes_path).map_err(|err| {
                ExtensionError::Diag {
                    code: "tool-runtime",
                    detail: format!("failed to compile component: {err}"),
                }
            })?;
        let mut linker = Linker::<WasiState>::new(&self.engine);
        let mut link_options = wasmtime_wasi::p2::bindings::sync::LinkOptions::default();
        link_options.cli_exit_with_code(true);
        wasmtime_wasi::p2::add_to_linker_with_options_sync(&mut linker, &link_options).map_err(
            |err| ExtensionError::Diag {
                code: "tool-runtime",
                detail: format!("failed to link WASI Preview 2: {err}"),
            },
        )?;
        let mut store = Store::new(
            &self.engine,
            WasiState {
                ctx: wasi,
                table: ResourceTable::new(),
            },
        );
        let command = Command::instantiate(&mut store, &component, &linker).map_err(|err| {
            ExtensionError::Diag {
                code: "tool-runtime",
                detail: format!("failed to instantiate command: {err}"),
            }
        })?;

        let exit_code = match command.wasi_cli_run().call_run(&mut store) {
            Ok(Ok(())) => 0,
            Ok(Err(())) => 1,
            Err(err) => map_guest_error(&err)?,
        };
        Ok(Outcome { exit_code })
    }
}

/// Captured stdout / stderr bytes plus the guest exit code from a
/// [`WasiRunner::run_captured`] invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedOutput {
    /// Verbatim stdout bytes captured from the guest.
    pub stdout: Vec<u8>,
    /// Verbatim stderr bytes captured from the guest.
    pub stderr: Vec<u8>,
    /// Process-style exit code returned by the guest.
    pub exit_code: i32,
}

const CAPTURE_BUFFER_BYTES: usize = 1024 * 1024;

#[derive(Debug)]
enum Stdio {
    Inherit,
    Captured(MemoryOutputPipe, MemoryOutputPipe),
}

#[derive(Debug, Clone, Copy)]
struct Outcome {
    exit_code: i32,
}

struct WasiState {
    ctx: WasiCtx,
    table: ResourceTable,
}

impl WasiView for WasiState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Preopen {
    host_path: PathBuf,
    guest_path: String,
    writable: bool,
}

/// Best-effort Cranelift compilation cache so repeated runs of the same
/// component bytes skip the JIT.
///
/// `SPECIFY_WASMTIME_CACHE` overrides the cache directory (absolute
/// path required; any other value disables caching). Otherwise the
/// cache lives at `<tool cache root>/wasmtime/`, beside the per-scope
/// tool directories, so it follows the `SPECIFY_EXTENSIONS_CACHE` → XDG →
/// HOME precedence rather than wasmtime's default `~/.cache/wasmtime`.
/// Failure to resolve or initialise the cache disables it — the engine
/// still runs, it just recompiles.
fn compilation_cache() -> Option<Cache> {
    let dir = match env::var_os("SPECIFY_WASMTIME_CACHE") {
        Some(value) => {
            let path = PathBuf::from(value);
            path.is_absolute().then_some(path)?
        }
        None => crate::cache::root().ok()?.join("wasmtime"),
    };
    let mut cache_config = CacheConfig::new();
    cache_config.with_directory(dir);
    Cache::new(cache_config).ok()
}

fn canonical_project_dir(project_dir: &Path) -> Result<PathBuf, ExtensionError> {
    project_dir.canonicalize().map_err(|err| {
        ExtensionError::permission_denied(
            project_dir,
            format!("PROJECT_DIR must exist and be canonicalisable: {err}"),
        )
    })
}

fn canonical_capability_dir(
    scope: &ExtensionScope, ctx_capability_dir: Option<&Path>,
) -> Result<Option<PathBuf>, ExtensionError> {
    match scope {
        ExtensionScope::Project { .. } => Ok(None),
        ExtensionScope::Plugin { capability_dir, .. } => {
            let path = ctx_capability_dir.unwrap_or(capability_dir);
            let canonical = path.canonicalize().map_err(|err| {
                ExtensionError::permission_denied(
                    path,
                    format!("CAPABILITY_DIR must exist and be canonicalisable: {err}"),
                )
            })?;
            Ok(Some(canonical))
        }
    }
}

fn prepare_preopens(
    resolved: &ResolvedExtension, project_dir: &Path, capability_dir: Option<&Path>,
) -> Result<Vec<Preopen>, ExtensionError> {
    let mut roots = vec![project_dir];
    if let Some(capability_dir) = capability_dir {
        roots.push(capability_dir);
    }

    let mut permissions = BTreeMap::<PathBuf, bool>::new();
    for template in &resolved.tool.permissions.read {
        let expanded = substitute(template, project_dir, capability_dir)?;
        let canonical = canonicalise_under(Path::new(&expanded), &roots)?;
        permissions.entry(canonical).or_insert(false);
    }
    for template in &resolved.tool.permissions.write {
        let expanded = substitute(template, project_dir, capability_dir)?;
        let canonical = canonicalise_under(Path::new(&expanded), &roots)?;
        deny_lifecycle_write(&canonical, project_dir)?;
        permissions.insert(canonical, true);
    }

    permissions
        .into_iter()
        .map(|(host_path, writable)| {
            let guest_path = guest_path(&host_path)?;
            Ok(Preopen {
                host_path,
                guest_path,
                writable,
            })
        })
        .collect()
}

fn build_wasi_ctx(
    resolved: &ResolvedExtension, ctx: &RunContext, project_dir: &Path,
    capability_dir: Option<&Path>, preopens: &[Preopen], stdio: Stdio,
) -> Result<WasiCtx, ExtensionError> {
    let mut builder = WasiCtxBuilder::new();
    builder
        .allow_blocking_current_thread(true)
        .allow_tcp(false)
        .allow_udp(false)
        .allow_ip_name_lookup(false);

    match stdio {
        Stdio::Inherit => {
            builder.inherit_stdio();
        }
        Stdio::Captured(stdout, stderr) => {
            builder.stdout(stdout);
            builder.stderr(stderr);
        }
    }

    let mut argv = Vec::with_capacity(ctx.args.len() + 1);
    argv.push(resolved.tool.name.clone());
    argv.extend(ctx.args.iter().cloned());
    builder.args(&argv);

    builder.env(
        "PROJECT_DIR",
        project_dir.to_str().ok_or_else(|| {
            ExtensionError::invalid_permission(
                "PROJECT_DIR",
                "PROJECT_DIR contains non-UTF-8 bytes and cannot be exposed to WASI",
            )
        })?,
    );
    if let Some(capability_dir) = capability_dir {
        builder.env(
            "CAPABILITY_DIR",
            capability_dir.to_str().ok_or_else(|| {
                ExtensionError::invalid_permission(
                    "CAPABILITY_DIR",
                    "CAPABILITY_DIR contains non-UTF-8 bytes and cannot be exposed to WASI",
                )
            })?,
        );
    }

    for preopen in preopens {
        let (dir_perms, file_perms) = if preopen.writable {
            (DirPerms::READ | DirPerms::MUTATE, FilePerms::READ | FilePerms::WRITE)
        } else {
            (DirPerms::READ, FilePerms::READ)
        };
        builder
            .preopened_dir(&preopen.host_path, &preopen.guest_path, dir_perms, file_perms)
            .map_err(|err| {
                ExtensionError::permission_denied(
                    &preopen.host_path,
                    format!("failed to preopen directory for WASI: {err}"),
                )
            })?;
    }

    Ok(builder.build())
}

fn guest_path(path: &Path) -> Result<String, ExtensionError> {
    path.to_str().map(ToOwned::to_owned).ok_or_else(|| {
        ExtensionError::permission_denied(
            path,
            "preopen path contains non-UTF-8 bytes and cannot be exposed to WASI",
        )
    })
}

fn map_guest_error(err: &wasmtime::Error) -> Result<i32, ExtensionError> {
    if let Some(exit) = err.downcast_ref::<I32Exit>() {
        return Ok(exit.0.clamp(0, 255));
    }
    Err(ExtensionError::Diag {
        code: "tool-runtime",
        detail: format!("guest trapped or failed at runtime: {err}"),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;
    use crate::manifest::{Extension, ExtensionPermissions, ExtensionScope, ExtensionSource};

    fn tool_with_permissions(read: Vec<String>, write: Vec<String>) -> Extension {
        Extension {
            name: "probe".to_string(),
            version: "1.0.0".to_string(),
            source: ExtensionSource::LocalPath(PathBuf::from("/tmp/probe.wasm")),
            sha256: None,
            permissions: ExtensionPermissions { read, write },
        }
    }

    const fn resolved(
        scope: ExtensionScope, tool: Extension, bytes_path: PathBuf,
    ) -> ResolvedExtension {
        ResolvedExtension {
            bytes_path,
            scope,
            tool,
        }
    }

    #[test]
    fn preopens_promote_write_over_read() {
        let tmp = tempdir().expect("tempdir");
        let project = tmp.path().join("project");
        let output = project.join("output");
        fs::create_dir_all(&output).expect("output");
        let project = project.canonicalize().expect("project");

        let resolved = resolved(
            ExtensionScope::Project {
                project_name: "demo".to_string(),
            },
            tool_with_permissions(
                vec!["$PROJECT_DIR/output".to_string()],
                vec!["$PROJECT_DIR/output".to_string()],
            ),
            tmp.path().join("missing.wasm"),
        );

        let preopens = prepare_preopens(&resolved, &project, None).expect("preopens");
        assert_eq!(preopens.len(), 1);
        assert!(preopens[0].writable);
        assert_eq!(preopens[0].host_path, output.canonicalize().expect("canonical output"));
    }

    #[test]
    fn run_rejects_cap_dir_in_project_scope() {
        let tmp = tempdir().expect("tempdir");
        let project = tmp.path().join("project");
        fs::create_dir_all(&project).expect("project");
        let resolved = resolved(
            ExtensionScope::Project {
                project_name: "demo".to_string(),
            },
            tool_with_permissions(vec!["$CAPABILITY_DIR/templates".to_string()], Vec::new()),
            tmp.path().join("missing.wasm"),
        );

        let runner = WasiRunner::new().expect("runner");
        let err = runner
            .run(&resolved, &RunContext::new(&project, Vec::new()))
            .expect_err("permission preparation must fail before component load");
        assert!(matches!(err, ExtensionError::InvalidPermission { .. }), "{err}");
    }

    #[test]
    fn run_rejects_lifecycle_write() {
        let tmp = tempdir().expect("tempdir");
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join(".specify")).expect("specify");
        let resolved = resolved(
            ExtensionScope::Project {
                project_name: "demo".to_string(),
            },
            tool_with_permissions(Vec::new(), vec!["$PROJECT_DIR/.specify".to_string()]),
            tmp.path().join("missing.wasm"),
        );

        let runner = WasiRunner::new().expect("runner");
        let err = runner
            .run(&resolved, &RunContext::new(&project, Vec::new()))
            .expect_err("lifecycle permission must fail before component load");
        assert!(matches!(err, ExtensionError::PermissionDenied { .. }), "{err}");
        assert!(err.to_string().contains("tool.lifecycle-state-write-denied"), "{err}");
    }

    // `map_guest_error` is the boundary between a WASI guest's process
    // exit and the host's typed error. The clamp to `0..=255` and the
    // "trap is not an exit" branch are both easy to get subtly wrong
    // (e.g. saturating vs wrapping, or treating a trap as exit 0).
    #[test]
    fn guest_exit_code_clamps() {
        let in_range: wasmtime::Error = I32Exit(42).into();
        assert_eq!(map_guest_error(&in_range).expect("exit is not an error"), 42);

        let over: wasmtime::Error = I32Exit(300).into();
        assert_eq!(map_guest_error(&over).expect("over clamps"), 255);

        let negative: wasmtime::Error = I32Exit(-5).into();
        assert_eq!(map_guest_error(&negative).expect("negative clamps"), 0);

        let trap = wasmtime::Error::msg("guest trapped");
        let err = map_guest_error(&trap).expect_err("a non-exit trap is an error");
        assert!(
            matches!(
                err,
                ExtensionError::Diag {
                    code: "tool-runtime",
                    ..
                }
            ),
            "{err}"
        );
    }

    // `prepare_preopens` folds read/write templates through a `BTreeMap`,
    // so duplicate read entries must collapse to one preopen and the
    // surviving set must come out in sorted host-path order regardless of
    // declaration order — the determinism the WASI sandbox relies on.
    #[test]
    fn preopens_dedup_and_sort() {
        let tmp = tempdir().expect("tempdir");
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("beta")).expect("beta");
        fs::create_dir_all(project.join("alpha")).expect("alpha");
        let project = project.canonicalize().expect("project");

        let resolved = resolved(
            ExtensionScope::Project {
                project_name: "demo".to_string(),
            },
            tool_with_permissions(
                vec![
                    "$PROJECT_DIR/beta".to_string(),
                    "$PROJECT_DIR/alpha".to_string(),
                    "$PROJECT_DIR/alpha".to_string(),
                ],
                Vec::new(),
            ),
            tmp.path().join("missing.wasm"),
        );

        let preopens = prepare_preopens(&resolved, &project, None).expect("preopens");
        let host_paths: Vec<PathBuf> = preopens.iter().map(|p| p.host_path.clone()).collect();
        assert_eq!(
            host_paths,
            vec![
                project.join("alpha").canonicalize().expect("alpha canonical"),
                project.join("beta").canonicalize().expect("beta canonical"),
            ]
        );
        assert!(preopens.iter().all(|p| !p.writable), "read templates stay read-only");
    }

    #[test]
    fn invalid_component_bytes_error() {
        let tmp = tempdir().expect("tempdir");
        let project = tmp.path().join("project");
        fs::create_dir_all(&project).expect("project");
        let wasm = tmp.path().join("not-a-component.wasm");
        fs::write(&wasm, b"not wasm").expect("write wasm");
        let resolved = resolved(
            ExtensionScope::Project {
                project_name: "demo".to_string(),
            },
            tool_with_permissions(Vec::new(), Vec::new()),
            wasm,
        );

        let runner = WasiRunner::new().expect("runner");
        let err = runner
            .run(&resolved, &RunContext::new(&project, Vec::new()))
            .expect_err("invalid component must fail");
        assert!(
            matches!(
                err,
                ExtensionError::Diag {
                    code: "tool-runtime",
                    ..
                }
            ),
            "{err}"
        );
    }
}
