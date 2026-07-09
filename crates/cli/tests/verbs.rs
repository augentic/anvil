//! End-to-end coverage of the pure project-scoped verbs through the
//! shared grammar and the guest route table: argv parses via
//! [`cli::guest::parse`], routes via [`cli::guest::route`],
//! and the handler's filesystem effects and exit code are asserted
//! against a throw-away project tree.
//!
//! This is the always-on native home of the coverage the parked
//! composed-deployment rig (`core/tests/composed_widened.rs`) proves
//! over the wasm seam: `registry {add,validate,remove}`,
//! `archive prune`, `init --scaffold-only`, and `journal {emit,show}`.
//! Each test pins the process CWD to its own tempdir — safe because
//! nextest runs every test in its own process.

use std::fs;
use std::path::{Path, PathBuf};

use cli::guest::{Route, parse, route};
use cli::output::Exit;
use tempfile::TempDir;

/// A throw-away project tree the routed verbs run against, with the
/// process CWD anchored at its root and the derived project cache
/// pinned beneath it so cache writes are hermetic.
struct Project {
    _tmp: TempDir,
    root: PathBuf,
}

impl Project {
    /// A bare directory — nothing scaffolded (the scaffold-leg input).
    #[expect(unsafe_code, reason = "pin the cache-root env var into the test tempdir")]
    fn bare() -> Self {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path().canonicalize().expect("canonical tempdir");
        // SAFETY: nextest runs each test in its own process, so no
        // other thread observes the env mutation.
        unsafe { std::env::set_var("SPECIFY_PROJECT_CACHE", root.join("project-cache")) };
        std::env::set_current_dir(&root).expect("enter project root");
        Self { _tmp: tmp, root }
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
}

/// Parse and route one argv line, expecting an in-process handler run.
fn run(argv: &[&str]) -> Exit {
    let mut full = vec!["specify"];
    full.extend_from_slice(argv);
    let cli = parse(full.iter().map(ToString::to_string))
        .unwrap_or_else(|exit| panic!("argv {argv:?} failed to parse (exit {})", exit.code()));
    match route(cli) {
        Route::Handled(exit) => exit,
        Route::Orchestrate(orchestration) => {
            panic!("argv {argv:?} must run in-process, got {orchestration:?}")
        }
    }
}

/// Stage a one-project `registry.yaml` at the project root.
fn stage_registry(root: &Path) {
    fs::write(
        root.join("registry.yaml"),
        "version: 1\nprojects:\n  - name: alpha\n    url: git@example.com:org/alpha.git\n",
    )
    .expect("stage registry.yaml");
}

mod registry {
    use super::*;

    #[test]
    fn add_mints_registry() {
        let project = Project::initialised();
        let exit = run(&["registry", "add", "alpha", "--url", "git@example.com:org/alpha.git"]);
        assert_eq!(exit, Exit::Success);
        let registry =
            fs::read_to_string(project.root.join("registry.yaml")).expect("registry.yaml");
        assert!(registry.contains("name: alpha"), "the add landed:\n{registry}");
    }

    #[test]
    fn duplicate_add_fails() {
        let project = Project::initialised();
        stage_registry(&project.root);
        let exit = run(&["registry", "add", "alpha", "--url", "git@example.com:org/alpha.git"]);
        assert_eq!(exit, Exit::GenericFailure, "duplicate add fails with the typed diagnostic");
    }

    #[test]
    fn validate_staged_catalogue() {
        let project = Project::initialised();
        stage_registry(&project.root);
        assert_eq!(run(&["registry", "validate"]), Exit::Success);
    }

    #[test]
    fn remove_drops_entry() {
        let project = Project::initialised();
        stage_registry(&project.root);
        assert_eq!(run(&["registry", "remove", "alpha"]), Exit::Success);
        let registry =
            fs::read_to_string(project.root.join("registry.yaml")).expect("registry.yaml");
        assert!(!registry.contains("name: alpha"), "the remove landed:\n{registry}");
    }
}

mod archive {
    use super::*;

    /// Stage two dated archive entries under `.specify/archive/`.
    fn stage(root: &Path) -> PathBuf {
        let archive = root.join(".specify/archive");
        for name in ["2020-01-01-old", "2026-01-01-new"] {
            fs::create_dir_all(archive.join(name)).expect("stage archive entry");
            fs::write(archive.join(name).join("spec.md"), "# archived\n").expect("stage file");
        }
        archive
    }

    #[test]
    fn prune_keeps_newest() {
        let project = Project::initialised();
        let archive = stage(&project.root);
        assert_eq!(run(&["archive", "prune", "--keep", "1"]), Exit::Success);
        assert!(!archive.join("2020-01-01-old").exists(), "the stale entry is pruned");
        assert!(archive.join("2026-01-01-new").exists(), "the newest entry survives --keep 1");
    }

    #[test]
    fn prune_requires_bound() {
        let project = Project::initialised();
        stage(&project.root);
        assert_eq!(
            run(&["archive", "prune"]),
            Exit::ArgumentError,
            "a bound-less prune refuses with the argument exit"
        );
    }
}

mod scaffold {
    use super::*;

    #[test]
    fn workspace_mode() {
        let project = Project::bare();
        let exit = run(&["init", "--workspace", "--scaffold-only", "--name", "demo-workspace"]);
        assert_eq!(exit, Exit::Success);
        let config =
            fs::read_to_string(project.root.join(".specify/project.yaml")).expect("project.yaml");
        assert!(config.contains("workspace: true"), "workspace mode is recorded:\n{config}");
        assert!(config.contains("name: demo-workspace"), "the name override lands:\n{config}");
        assert!(project.root.join("registry.yaml").is_file(), "workspace init mints registry.yaml");
    }

    #[test]
    fn regular_mode() {
        let project = Project::bare();

        // Stage a fake `demo` component at the resolver's in-repo dev
        // probe path with a digest-valid describe sidecar beside it:
        // the resolver probes file presence and the sidecar supplies
        // the metadata, so no describe dispatch runs.
        let dev_dir = project.root.join("target/wasm32-wasip2/release");
        fs::create_dir_all(&dev_dir).expect("mkdir dev release dir");
        let component = dev_dir.join("demo.wasm");
        fs::write(&component, b"\0asm-component").expect("stage component");
        let digest = schema::cache::file_content_digest(&component);
        fs::write(
            dev_dir.join("demo.wasm.describe.json"),
            format!("{{ \"digest\": \"{digest}\", \"manifest\": {{}} }}"),
        )
        .expect("stage describe sidecar");

        let exit = run(&["init", "demo", "--scaffold-only", "--name", "demo-project"]);
        assert_eq!(exit, Exit::Success);
        let config =
            fs::read_to_string(project.root.join(".specify/project.yaml")).expect("project.yaml");
        assert!(config.contains("adapter: demo"), "the adapter is recorded:\n{config}");
        assert!(project.root.join(".specify/slices").is_dir(), "the slice tree is scaffolded");
    }
}

mod journal {
    use super::*;

    #[test]
    fn emit_appends_line() {
        let project = Project::initialised();
        let exit = run(&[
            "journal",
            "emit",
            "slice.build.started",
            "--payload",
            r#"{"slice-name":"billing"}"#,
        ]);
        assert_eq!(exit, Exit::Success);
        let journal =
            fs::read_to_string(project.root.join(".specify/journal.jsonl")).expect("journal");
        assert!(
            journal.contains(r#""event":"slice.build.started""#),
            "the event landed:\n{journal}"
        );
    }

    #[test]
    fn emit_unknown_event_refused() {
        let _project = Project::initialised();
        assert_eq!(
            run(&["journal", "emit", "no.such.event"]),
            Exit::ValidationFailed,
            "an id outside the closed taxonomy exits 2"
        );
    }

    #[test]
    fn emit_bad_payload_refused() {
        let project = Project::initialised();
        assert_eq!(
            run(&["journal", "emit", "slice.build.started", "--payload", "{}"]),
            Exit::ValidationFailed,
            "a payload missing the variant's fields exits 2"
        );
        assert!(
            !project.root.join(".specify/journal.jsonl").exists(),
            "a refused emit appends nothing"
        );
    }

    #[test]
    fn show_reads_filtered() {
        let project = Project::initialised();
        assert_eq!(
            run(&[
                "journal",
                "emit",
                "slice.build.started",
                "--payload",
                r#"{"slice-name":"billing"}"#,
            ]),
            Exit::Success
        );
        assert_eq!(run(&["journal", "show", "--filter", "slice.build"]), Exit::Success);
        assert_eq!(
            run(&["journal", "show", "--filter", "plan."]),
            Exit::Success,
            "a filter with no matches still exits 0"
        );
        assert!(project.root.join(".specify/journal.jsonl").is_file(), "the journal persists");
    }
}
