//! Composed-runtime coverage for the widened guest surface (RFC-65
//! move 1): every project-scoped pure verb runs in the workflow guest
//! against a mounted project tree — `registry {validate,add,remove}`,
//! `archive prune`, `rules export`, `completions`, and init's scaffold
//! leg (`init --scaffold-only`).
//!
//! Guest stdout is inherited in-process, so the in-process runs assert
//! exit codes plus the filesystem effects on the `"."` mount and the
//! derived-cache mount; the completions run goes through the replay
//! binary so its stdout can be compared byte-for-byte against the
//! native grammar's script (the shared clap tree is the equivalence).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use omnia::{DeploymentBuilder, ExitStatus, Mode};
use omnia_testkit::{TempManifest, temp_manifest};
use tempfile::TempDir;

use crate::common::{self, CacheGuard, ECHO_WASM, Quiet, StubBundle, WORKFLOW_WASM, scoped_cache};

/// A throw-away project tree the composed deployment mounts at `"."`,
/// with the hermetic per-project cache pinned beneath the tempdir —
/// the cache is mounted alongside `"."` exactly as the generated
/// deployment manifest mounts it.
struct Project {
    _tmp: TempDir,
    _cache: CacheGuard,
    root: PathBuf,
}

impl Project {
    /// A bare directory — nothing scaffolded (the scaffold-leg input).
    fn bare() -> Self {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path().to_path_buf();
        let cache = scoped_cache(&root);
        Self {
            _tmp: tmp,
            _cache: cache,
            root,
        }
    }

    /// An initialised project (`.specify/project.yaml` present).
    fn initialised() -> Self {
        let project = Self::bare();
        fs::create_dir_all(project.root.join(".specify")).expect("mkdir .specify");
        fs::write(
            project.root.join(".specify/project.yaml"),
            "name: demo\nadapter: demo\nrules: {}\n",
        )
        .expect("write project.yaml");
        project
    }

    fn cache_dir(&self) -> PathBuf {
        specify_schema::cache::project_cache_dir(&self.root)
    }
}

/// The two-guest deployment (workflow + in-repo echo) over `mount`,
/// with the project mount and the derived-cache mount the generated
/// manifest carries. The echo guest satisfies the workflow guest's
/// adapter-contract links without needing the sibling adapters
/// checkout — no test here dispatches a seam call.
fn manifest(mount: &Path) -> Result<TempManifest> {
    let workflow = common::guest_wasm(WORKFLOW_WASM);
    let echo = common::guest_wasm(ECHO_WASM);
    let cache = specify_schema::cache::project_cache_dir(mount);
    fs::create_dir_all(&cache)?;
    temp_manifest(&format!(
        "[[guest]]\n\
         id = \"workflow\"\n\
         source.path = \"{workflow}\"\n\
         link = [\"specify:adapter/source@0.1.0\", \"specify:adapter/target@0.1.0\"]\n\n\
         [[guest]]\n\
         id = \"source:echo\"\n\
         source.path = \"{echo}\"\n\n\
         [[mount]]\n\
         name = \".\"\n\
         path = \"{mount}\"\n\
         writable = true\n\n\
         [[mount]]\n\
         name = \"{cache_name}\"\n\
         path = \"{cache}\"\n\
         writable = true\n\n\
         [[route.http]]\n\
         prefix = \"/mcp/echo\"\n\
         guest = \"source:echo\"\n\n\
         [transport]\n\
         default = \"in-process\"\n",
        workflow = workflow.display(),
        echo = echo.display(),
        mount = mount.display(),
        cache_name = specify_schema::cache::GUEST_CACHE_MOUNT,
        cache = cache.display(),
    ))
}

/// One command-mode run of the deployment with the given guest argv.
async fn run(mount: &Path, args: &[&str]) -> Result<ExitStatus> {
    let manifest = manifest(mount)?;
    let builder = DeploymentBuilder::new()
        .config(manifest.path().to_path_buf())
        .mode(Mode::Command)
        .args(args.iter().map(ToString::to_string).collect::<Vec<_>>());
    omnia::run::<StubBundle, Quiet>(builder).await
}

// The omnia telemetry resource is a process-global `OnceLock`, so each
// test (one nextest process) gets exactly one in-process `omnia::run`;
// multi-step flows stage their preconditions natively instead.

/// Stage a one-project `registry.yaml` on the mount as the guest-run
/// precondition (test fixture, not an operator surface).
fn stage_registry(root: &Path) {
    fs::write(
        root.join("registry.yaml"),
        "version: 1\nprojects:\n  - name: alpha\n    url: git@example.com:org/alpha.git\n",
    )
    .expect("stage registry.yaml");
}

// `registry add` in-guest: the guest mints `registry.yaml` on the
// `"."` mount with the entry recorded.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn registry_add() -> Result<()> {
    let project = Project::initialised();

    let status =
        run(&project.root, &["registry", "add", "alpha", "--url", "git@example.com:org/alpha.git"])
            .await?;
    assert_eq!(status.code(), 0, "guest registry add exits 0");
    let registry = fs::read_to_string(project.root.join("registry.yaml")).expect("registry.yaml");
    assert!(registry.contains("name: alpha"), "the add landed on the mount:\n{registry}");
    Ok(())
}

// A duplicate `registry add` fails in-guest with the native handler's
// typed diagnostic (`Diag` → generic failure, exit 1) — exit parity on
// the error leg.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn registry_add_duplicate() -> Result<()> {
    let project = Project::initialised();
    stage_registry(&project.root);

    let status =
        run(&project.root, &["registry", "add", "alpha", "--url", "git@example.com:org/alpha.git"])
            .await?;
    assert_eq!(status.code(), 1, "duplicate add fails in-guest exactly as natively");
    Ok(())
}

// `registry validate` in-guest over a staged catalogue.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn registry_validate() -> Result<()> {
    let project = Project::initialised();
    stage_registry(&project.root);

    let status = run(&project.root, &["registry", "validate"]).await?;
    assert_eq!(status.code(), 0, "guest registry validate exits 0");
    Ok(())
}

// `registry remove` in-guest: the entry disappears from the mount.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn registry_remove() -> Result<()> {
    let project = Project::initialised();
    stage_registry(&project.root);

    let status = run(&project.root, &["registry", "remove", "alpha"]).await?;
    assert_eq!(status.code(), 0, "guest registry remove exits 0");
    let registry = fs::read_to_string(project.root.join("registry.yaml")).expect("registry.yaml");
    assert!(!registry.contains("name: alpha"), "the remove landed on the mount:\n{registry}");
    Ok(())
}

/// Stage two dated archive entries under `.specify/archive/`.
fn stage_archive(root: &Path) -> PathBuf {
    let archive = root.join(".specify/archive");
    for name in ["2020-01-01-old", "2026-01-01-new"] {
        fs::create_dir_all(archive.join(name)).expect("stage archive entry");
        fs::write(archive.join(name).join("spec.md"), "# archived\n").expect("stage file");
    }
    archive
}

// `archive prune` in-guest: retention GC over `.specify/archive/` on
// the `"."` mount — the newest-`--keep` window survives.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn archive_prune() -> Result<()> {
    let project = Project::initialised();
    let archive = stage_archive(&project.root);

    let status = run(&project.root, &["archive", "prune", "--keep", "1"]).await?;
    assert_eq!(status.code(), 0, "guest archive prune exits 0");
    assert!(!archive.join("2020-01-01-old").exists(), "the stale entry is pruned");
    assert!(archive.join("2026-01-01-new").exists(), "the newest entry survives --keep 1");
    Ok(())
}

// A bound-less `archive prune` refuses in-guest with the argument exit
// (2) — the same `Error::Argument` mapping the native run produces.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn archive_prune_requires_bound() -> Result<()> {
    let project = Project::initialised();
    stage_archive(&project.root);

    let status = run(&project.root, &["archive", "prune"]).await?;
    assert_eq!(status.code(), 2, "a bound-less prune refuses with the argument exit");
    Ok(())
}

// `rules export` in-guest: the shared codex materialized into the
// per-project cache natively is resolved by the guest through the
// derived-cache mount — proving the cache tenants are reachable
// in-guest.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rules_export_reads_cache_mount() -> Result<()> {
    let project = Project::initialised();
    let now = "2026-01-02T03:04:05Z".parse().expect("fixed timestamp parses");
    specify_workflow::init::sync_codex(&project.root, false, now).expect("sync codex");

    let status =
        run(&project.root, &["rules", "export", "--target", "demo", "--format", "json"]).await?;
    assert_eq!(status.code(), 0, "the guest resolves the codex through the cache mount");
    Ok(())
}

// Without a codex anywhere the guest probe fails with the same
// `rules-root-required` validation exit (2) the native run produces.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rules_export_without_codex() -> Result<()> {
    let project = Project::initialised();

    let status =
        run(&project.root, &["rules", "export", "--target", "demo", "--format", "json"]).await?;
    assert_eq!(status.code(), 2, "no codex anywhere is rules-root-required (exit 2)");
    Ok(())
}

// The scaffold leg on a bare directory, workspace mode: the guest
// writes `.specify/project.yaml { workspace: true }` and mints
// `registry.yaml` on the `"."` mount.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scaffold_workspace() -> Result<()> {
    let project = Project::bare();

    let status =
        run(&project.root, &["init", "--workspace", "--scaffold-only", "--name", "demo-workspace"])
            .await?;
    assert_eq!(status.code(), 0, "the guest scaffold leg exits 0");

    let config =
        fs::read_to_string(project.root.join(".specify/project.yaml")).expect("project.yaml");
    assert!(config.contains("workspace: true"), "workspace mode is recorded:\n{config}");
    assert!(config.contains("name: demo-workspace"), "the name override lands:\n{config}");
    assert!(project.root.join("registry.yaml").is_file(), "workspace init mints registry.yaml");
    Ok(())
}

// The scaffold leg on a bare directory, regular mode: the guest
// resolves the bare-name adapter against the staged development build
// under the mount (metadata from the pre-warmed describe sidecar — the
// guest dispatches no describe), scaffolds `.specify/`, records the
// adapter on `project.yaml`, and materializes the shared codex through
// the derived-cache mount.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scaffold_regular() -> Result<()> {
    let project = Project::bare();

    // Stage a fake `demo` component at the resolver's in-repo dev
    // probe path with a digest-valid describe sidecar beside it: the
    // resolver only probes file presence, and the sidecar supplies the
    // metadata the guest cannot dispatch for.
    let dev_dir = project.root.join("target/wasm32-wasip2/release");
    fs::create_dir_all(&dev_dir).expect("mkdir dev release dir");
    let component = dev_dir.join("specify_demo.wasm");
    fs::write(&component, b"\0asm-component").expect("stage component");
    let digest = specify_schema::cache::file_content_digest(&component);
    fs::write(
        dev_dir.join("specify_demo.wasm.describe.json"),
        format!("{{ \"digest\": \"{digest}\", \"manifest\": {{}} }}"),
    )
    .expect("stage describe sidecar");

    let status =
        run(&project.root, &["init", "demo", "--scaffold-only", "--name", "demo-project"]).await?;
    assert_eq!(status.code(), 0, "the guest scaffold leg exits 0");

    let config =
        fs::read_to_string(project.root.join(".specify/project.yaml")).expect("project.yaml");
    assert!(config.contains("adapter: demo"), "the adapter is recorded:\n{config}");
    assert!(project.root.join(".specify/slices").is_dir(), "the slice tree is scaffolded");
    assert!(
        project.cache_dir().join("codex/codex/rules/universal").is_dir(),
        "the shared codex materialized through the derived-cache mount"
    );
    Ok(())
}

// `completions` through the replay binary: the guest's stdout carries
// the exact script the shared clap grammar renders natively — the
// cheapest whole-envelope native-equivalence check the widened surface
// offers.
#[test]
fn completions_matches_native() -> Result<()> {
    let engine = common::workspace_root();
    let project = Project::bare();
    let manifest = manifest(&project.root)?;

    let port = std::net::TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
    let output = assert_cmd::Command::cargo_bin("specify-runtime-replay")?
        .current_dir(&engine)
        .env("HTTP_ADDR", format!("127.0.0.1:{port}"))
        .env_remove("RUST_LOG")
        .args(["run", "--config"])
        .arg(manifest.path())
        .args(["--", "completions", "zsh"])
        .output()?;
    assert!(
        output.status.success(),
        "runtime exited {:?}; stderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let mut native = Vec::new();
    let mut cmd = <specify_dispatch::cli::Cli as clap::CommandFactory>::command();
    clap_complete::generate(clap_complete::Shell::Zsh, &mut cmd, "specify", &mut native);
    assert!(!native.is_empty(), "the native grammar renders a script");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&native),
        "the guest completions script is byte-identical to the native grammar's"
    );
    Ok(())
}
