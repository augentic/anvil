//! Case runner gates — config parsing, expect-path confinement, the
//! sandbox policy, and both case kinds end to end over the mock
//! catalog and the real native command surface.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use native::{CachePlacement, Catalog, DynModel, ExecutionPaths, Locations};
use omnia_testkit::model::Harness;
use probe::ModelFactory;
use probe::case::{self, Case, WorkflowUntil};
use project::plan::{Lifecycle, Status};
use project::slice::{LifecycleStatus, SliceMetadata};
use tempfile::TempDir;

fn catalog() -> Catalog {
    mock::catalog()
}

// A FIFO scripted model factory; every case run shares the answers.
fn scripted(answers: Vec<String>) -> ModelFactory {
    let model = Harness::answering(answers);
    Arc::new(move |_root| Ok(DynModel::new(model.clone())))
}

mod config {
    use super::*;

    #[test]
    fn workflow_defaults_to_execute() {
        let case = case::parse(
            "kind = \"workflow\"\ntarget = \"mock\"\nchange = \"demo\"\n\
             [sources]\nmain = \"mock:value:The greeting service.\"\n",
        )
        .expect("a minimal workflow case parses");
        let Case::Workflow(workflow) = case else {
            panic!("workflow kind parses to a workflow case");
        };
        assert_eq!(workflow.until, WorkflowUntil::Execute);
        assert!(workflow.fixture.is_none());
    }

    #[test]
    fn missing_kind_refused() {
        let err = case::parse("slice = \"demo\"\nexpect = [\"out\"]\n")
            .expect_err("a case without a kind refuses");
        assert!(format!("{err:#}").contains("kind"), "{err:#}");
    }

    #[test]
    fn unknown_kind_refused() {
        let err = case::parse("kind = \"merge\"\nslice = \"demo\"\n")
            .expect_err("an unknown kind refuses");
        assert!(format!("{err:#}").contains("unknown case `kind`"), "{err:#}");
    }

    #[test]
    fn unknown_key_refused() {
        let err = case::parse(
            "kind = \"build\"\nslice = \"demo\"\nexpect = [\"out\"]\nsurprise = true\n",
        )
        .expect_err("unknown keys refuse");
        assert!(format!("{err:#}").contains("surprise"), "{err:#}");
    }

    #[test]
    fn build_requires_expect() {
        let err = case::parse("kind = \"build\"\nslice = \"demo\"\nexpect = []\n")
            .expect_err("build without expect refuses");
        assert!(format!("{err:#}").contains("at least one `expect`"), "{err:#}");
    }

    #[test]
    fn workflow_requires_an_input() {
        let err = case::parse("kind = \"workflow\"\ntarget = \"mock\"\nchange = \"demo\"\n")
            .expect_err("a workflow case without intent or sources refuses");
        assert!(format!("{err:#}").contains("intent"), "{err:#}");
    }

    #[test]
    fn absolute_expect_refused() {
        let err = case::parse("kind = \"build\"\nslice = \"demo\"\nexpect = [\"/etc/passwd\"]\n")
            .expect_err("absolute expect refuses");
        assert!(format!("{err:#}").contains("absolute"), "{err:#}");
    }

    #[test]
    fn traversing_expect_refused() {
        let err = case::parse("kind = \"build\"\nslice = \"demo\"\nexpect = [\"../outside\"]\n")
            .expect_err("parent-traversing expect refuses");
        assert!(format!("{err:#}").contains("plain names"), "{err:#}");
    }

    #[test]
    fn empty_expect_entry_refused() {
        let err = case::parse("kind = \"build\"\nslice = \"demo\"\nexpect = [\"  \"]\n")
            .expect_err("blank expect entry refuses");
        assert!(format!("{err:#}").contains("empty expect entry"), "{err:#}");
    }

    #[test]
    fn workflow_clone_parses() {
        let case = case::parse(
            "kind = \"workflow\"\ntarget = \"mock\"\nchange = \"demo\"\n\
             clone = { url = \"https://example.com/tree.git\", dest = \"legacy/tree\" }\n\
             [sources]\nmain = \"mock:value:The greeting service.\"\n",
        )
        .expect("a workflow case with a clone parses");
        let Case::Workflow(workflow) = case else {
            panic!("workflow kind parses to a workflow case");
        };
        let clone = workflow.clone.expect("the clone spec is carried");
        assert_eq!(clone.url, "https://example.com/tree.git");
        assert_eq!(clone.dest, Path::new("legacy/tree"));
    }

    #[test]
    fn traversing_clone_dest_refused() {
        let err = case::parse(
            "kind = \"workflow\"\ntarget = \"mock\"\nchange = \"demo\"\n\
             clone = { url = \"https://example.com/tree.git\", dest = \"../outside\" }\n\
             [sources]\nmain = \"mock:value:The greeting service.\"\n",
        )
        .expect_err("a parent-traversing clone dest refuses");
        assert!(format!("{err:#}").contains("plain names"), "{err:#}");
    }

    #[test]
    fn empty_clone_url_refused() {
        let err = case::parse(
            "kind = \"workflow\"\ntarget = \"mock\"\nchange = \"demo\"\n\
             clone = { url = \" \", dest = \"legacy/tree\" }\n\
             [sources]\nmain = \"mock:value:The greeting service.\"\n",
        )
        .expect_err("a blank clone url refuses");
        assert!(format!("{err:#}").contains("empty clone url"), "{err:#}");
    }

    #[test]
    fn fixture_with_clone_refused() {
        let err = case::parse(
            "kind = \"workflow\"\ntarget = \"mock\"\nchange = \"demo\"\n\
             fixture = \"../shared\"\n\
             clone = { url = \"https://example.com/tree.git\", dest = \"legacy/tree\" }\n\
             [sources]\nmain = \"mock:value:The greeting service.\"\n",
        )
        .expect_err("fixture and clone together refuse");
        assert!(format!("{err:#}").contains("mutually exclusive"), "{err:#}");
    }

    #[test]
    fn clone_on_build_refused() {
        let err = case::parse(
            "kind = \"build\"\nslice = \"demo\"\nexpect = [\"out\"]\n\
             clone = { url = \"https://example.com/tree.git\", dest = \"legacy/tree\" }\n",
        )
        .expect_err("build cases carry no clone");
        assert!(format!("{err:#}").contains("clone"), "{err:#}");
    }

    #[test]
    fn nested_id_refused() {
        let tmp = TempDir::new().expect("tempdir");
        let err = case::load(tmp.path(), "mock/one").expect_err("nested ids refuse");
        assert!(format!("{err:#}").contains("flat directory names"), "{err:#}");
    }
}

mod expected {
    use super::*;

    #[test]
    fn missing_artifact_fails() {
        let tmp = TempDir::new().expect("tempdir");
        let err = case::enforce_expected("demo", tmp.path(), &["contracts/api.yaml".to_string()])
            .expect_err("a missing artifact fails the gate");
        assert!(format!("{err:#}").contains("contracts/api.yaml"), "{err:#}");
    }

    #[test]
    fn file_and_populated_dir_pass() {
        let tmp = TempDir::new().expect("tempdir");
        fs::create_dir_all(tmp.path().join("contracts/nested")).expect("mkdir");
        fs::write(tmp.path().join("contracts/nested/api.yaml"), "openapi: 3.1.0\n").expect("write");
        case::enforce_expected(
            "demo",
            tmp.path(),
            &["contracts".to_string(), "contracts/nested/api.yaml".to_string()],
        )
        .expect("a populated directory and an existing file both satisfy the gate");
    }

    #[test]
    fn empty_dir_fails() {
        let tmp = TempDir::new().expect("tempdir");
        fs::create_dir_all(tmp.path().join("contracts")).expect("mkdir");
        let err = case::enforce_expected("demo", tmp.path(), &["contracts".to_string()])
            .expect_err("an empty directory is a silent no-op");
        assert!(format!("{err:#}").contains("contracts"), "{err:#}");
    }

    #[cfg(unix)]
    #[test]
    fn symlink_cycle_terminates() {
        let tmp = TempDir::new().expect("tempdir");
        let dir = tmp.path().join("contracts/loop");
        fs::create_dir_all(&dir).expect("mkdir");
        std::os::unix::fs::symlink(tmp.path().join("contracts"), dir.join("back"))
            .expect("cycle link");
        let err = case::enforce_expected("demo", tmp.path(), &["contracts".to_string()])
            .expect_err("a cyclic, file-free tree fails rather than hanging");
        assert!(format!("{err:#}").contains("contracts"), "{err:#}");
    }

    #[cfg(unix)]
    #[test]
    fn escaping_symlink_fails() {
        let outer = TempDir::new().expect("outer tempdir");
        fs::write(outer.path().join("secret.yaml"), "outside\n").expect("write outside");
        let tmp = TempDir::new().expect("tempdir");
        std::os::unix::fs::symlink(outer.path().join("secret.yaml"), tmp.path().join("api.yaml"))
            .expect("escape link");
        let err = case::enforce_expected("demo", tmp.path(), &["api.yaml".to_string()])
            .expect_err("a symlink escaping the sandbox never satisfies the gate");
        assert!(format!("{err:#}").contains("api.yaml"), "{err:#}");
    }
}

// One `emery` verb through the native command surface, for staging
// fixtures the way an operator would produce them.
async fn invoke(root: &Path, model: &DynModel, argv: &[&str]) {
    let mut full = vec!["emery".to_string()];
    full.extend(argv.iter().map(ToString::to_string));
    let locations = Locations::explicit(
        root.join("adapter-store"),
        CachePlacement::Parent(root.join("project-cache")),
    );
    let paths = ExecutionPaths::new(root, locations);
    let response = native::command::execute(paths, model.clone(), catalog(), full)
        .await
        .expect("command executes");
    assert_eq!(
        response.exit,
        0,
        "`emery {}` failed: {}{}",
        argv.join(" "),
        String::from_utf8_lossy(&response.stdout),
        String::from_utf8_lossy(&response.stderr),
    );
}

// Drive the real verbs to a refined `greeting` slice at `root` — the
// exact state a committed Build fixture carries.
async fn stage_refined_fixture(root: &Path) {
    fs::create_dir_all(root).expect("mkdir fixture");
    let model = DynModel::new(Harness::answering(vec![
        mock::answers::greeting_grouping(),
        mock::answers::greeting_synthesis(),
    ]));
    invoke(root, &model, &["init", "mock"]).await;
    invoke(
        root,
        &model,
        &["plan", "author", "demo", "--source", "main=mock:value:The greeting service."],
    )
    .await;
    invoke(root, &model, &["slice", "refine", "greeting"]).await;
}

fn git(dir: &Path, args: &[&str]) {
    let status =
        std::process::Command::new("git").current_dir(dir).args(args).status().expect("git spawns");
    assert!(status.success(), "git {args:?} failed");
}

// A one-commit local repo standing in for a case's upstream tree.
fn stage_upstream_repo(dir: &Path) {
    fs::create_dir_all(dir).expect("mkdir upstream");
    git(dir, &["init", "-q"]);
    fs::write(dir.join("README.md"), "upstream\n").expect("write README");
    git(dir, &["add", "README.md"]);
    git(
        dir,
        &[
            "-c",
            "user.name=probe",
            "-c",
            "user.email=probe@test",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            "seed",
        ],
    );
}

fn stage_case(cases: &Path, id: &str, body: &str) {
    let dir = cases.join(id);
    fs::create_dir_all(&dir).expect("mkdir case");
    fs::write(dir.join("case.toml"), body).expect("write case.toml");
}

fn journal(root: &Path) -> String {
    fs::read_to_string(root.join(".emery/journal.jsonl")).expect("journal.jsonl")
}

#[tokio::test]
async fn build_case_reaches_built() {
    let tmp = TempDir::new().expect("tempdir");
    let cases = tmp.path().join("cases");
    stage_case(
        &cases,
        "greeting-build",
        "kind = \"build\"\nslice = \"greeting\"\nexpect = [\"mock-build/greeting.md\"]\n",
    );
    stage_refined_fixture(&cases.join("greeting-build/fixture")).await;

    let sandbox = tmp.path().join("sandbox");
    case::run(
        &cases,
        &sandbox,
        Some("greeting-build"),
        None,
        false,
        &catalog(),
        &scripted(Vec::new()),
    )
    .await
    .expect("the build case passes");

    let root = sandbox.join("greeting-build");
    let metadata = SliceMetadata::load(&root.join(".emery/slices/greeting"))
        .expect("slice metadata after build");
    assert_eq!(metadata.status, LifecycleStatus::Built);
    assert!(
        root.join(".emery/slices/greeting/build/report.yaml").is_file(),
        "the authoritative build report is persisted"
    );
    assert!(root.join("mock-build/greeting.md").is_file(), "the expected output exists");
    let journal = journal(&root);
    assert!(journal.contains("slice.build.started"), "{journal}");
    assert!(journal.contains("slice.build.succeeded"), "{journal}");
}

#[tokio::test]
async fn until_plan_leaves_gate_pending() {
    let tmp = TempDir::new().expect("tempdir");
    let cases = tmp.path().join("cases");
    stage_case(
        &cases,
        "greeting",
        "kind = \"workflow\"\ntarget = \"mock\"\nchange = \"demo\"\n\
         [sources]\nmain = \"mock:value:The greeting service.\"\n",
    );

    let sandbox = tmp.path().join("sandbox");
    case::run(
        &cases,
        &sandbox,
        Some("greeting"),
        Some(WorkflowUntil::Plan),
        false,
        &catalog(),
        &scripted(vec![mock::answers::greeting_grouping()]),
    )
    .await
    .expect("the plan-stopped workflow case passes");

    let root = sandbox.join("greeting");
    let plan = project::plan::Plan::load(&project::config::Layout::new(&root).plan_path())
        .expect("plan.yaml");
    assert_eq!(plan.lifecycle, Lifecycle::Pending, "Gate 1 stays pending");
    assert!(!plan.entries.is_empty(), "the authored plan carries entries");
    assert!(
        plan.entries.iter().all(|entry| entry.status == Status::Pending),
        "no entry advanced: {:?}",
        plan.entries
    );
    assert!(
        !journal(&root).contains("plan.transition.approved"),
        "no approval event was journaled"
    );
}

#[tokio::test]
async fn clone_populates_cache_once() {
    let tmp = TempDir::new().expect("tempdir");
    let upstream = tmp.path().join("upstream");
    stage_upstream_repo(&upstream);
    let cases = tmp.path().join("cases");
    stage_case(
        &cases,
        "cloned",
        &format!(
            "kind = \"workflow\"\ntarget = \"mock\"\nchange = \"demo\"\n\
             clone = {{ url = \"file://{}\", dest = \"legacy/upstream\" }}\n\
             [sources]\nmain = \"mock:value:The greeting service.\"\n",
            upstream.display()
        ),
    );

    let sandbox = tmp.path().join("sandbox");
    case::run(
        &cases,
        &sandbox,
        Some("cloned"),
        Some(WorkflowUntil::Plan),
        false,
        &catalog(),
        &scripted(vec![mock::answers::greeting_grouping()]),
    )
    .await
    .expect("the cloning workflow case passes");

    let cache = cases.join("cloned/fixture/legacy/upstream");
    assert!(cache.join("README.md").is_file(), "the clone lands in the fixture cache");
    assert!(!cache.join(".git").exists(), "the cached clone's .git is stripped");
    let dest = sandbox.join("cloned/legacy/upstream");
    assert!(dest.join("README.md").is_file(), "the cached tree is copied into the sandbox");

    // A restart with the upstream gone reuses the cache — no reclone.
    fs::remove_dir_all(&upstream).expect("drop the upstream");
    case::run(
        &cases,
        &sandbox,
        Some("cloned"),
        Some(WorkflowUntil::Plan),
        true,
        &catalog(),
        &scripted(vec![mock::answers::greeting_grouping()]),
    )
    .await
    .expect("a restart runs offline over the cached fixture");
    assert!(dest.join("README.md").is_file(), "the restart re-materializes from the cache");
}

#[tokio::test]
async fn existing_sandbox_refuses() {
    let tmp = TempDir::new().expect("tempdir");
    let cases = tmp.path().join("cases");
    stage_case(
        &cases,
        "greeting",
        "kind = \"workflow\"\ntarget = \"mock\"\nchange = \"demo\"\n\
         [sources]\nmain = \"mock:value:The greeting service.\"\n",
    );
    let sandbox = tmp.path().join("sandbox");
    fs::create_dir_all(sandbox.join("greeting")).expect("pre-existing sandbox");

    let err = case::run(
        &cases,
        &sandbox,
        Some("greeting"),
        None,
        false,
        &catalog(),
        &scripted(Vec::new()),
    )
    .await
    .expect_err("an existing sandbox refuses before mutation");
    let message = format!("{err:#}");
    assert!(message.contains("--restart"), "{message}");
    assert!(message.contains("--project-dir"), "{message}");
}

#[tokio::test]
async fn missing_fixture_is_focused() {
    let tmp = TempDir::new().expect("tempdir");
    let cases = tmp.path().join("cases");
    stage_case(
        &cases,
        "prepared",
        "kind = \"build\"\nslice = \"demo\"\nfixture = \"../prepared-tree\"\n\
         expect = [\"out\"]\n",
    );

    let err = case::run(
        &cases,
        &tmp.path().join("sandbox"),
        Some("prepared"),
        None,
        false,
        &catalog(),
        &scripted(Vec::new()),
    )
    .await
    .expect_err("an unprepared explicit fixture refuses");
    assert!(format!("{err:#}").contains("prepare it"), "{err:#}");
}

// Guards against PathBuf-typed fixture handling: relative fixtures
// resolve from the case directory.
#[tokio::test]
async fn relative_fixture_from_case_dir() {
    let tmp = TempDir::new().expect("tempdir");
    let cases = tmp.path().join("cases");
    stage_case(
        &cases,
        "shared",
        "kind = \"build\"\nslice = \"greeting\"\nfixture = \"../../shared-fixture\"\n\
         expect = [\"mock-build/greeting.md\"]\n",
    );
    stage_refined_fixture(&tmp.path().join("shared-fixture")).await;

    case::run(
        &cases,
        &tmp.path().join("sandbox"),
        Some("shared"),
        None,
        false,
        &catalog(),
        &scripted(Vec::new()),
    )
    .await
    .expect("a case over a shared relative fixture passes");
}
