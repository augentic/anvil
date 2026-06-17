//! `specify plan archive` CLI tests, including the working-directory
//! co-move (L1.K / L3.B).

use crate::support::*;

// -- plan archive (L1.K) ----------------------------------------------
//
// Determinism: the archive verb stamps its filename from
// `Timestamp::now()` read *inside* the CLI subprocess, and the CLI
// exposes no clock-injection seam (`Ctx::now()` hardcodes
// `Timestamp::now()`; `plan archive` passes `Timestamp::now()` straight
// through). So tests must not reconstruct the stamp from their own
// clock — a midnight roll between the two reads would desync them.
// Discovery assertions match the produced `<name>-YYYYMMDD` shape with
// a regex; the two collision tests, which must pre-create the exact
// destination, seed the whole `date_window()` instead.

/// UTC `YYYYMMDD` stamps for yesterday / today / tomorrow. The CLI
/// reads its clock a beat after the test reads its own, so its stamp is
/// always within this window; seeding all three guarantees a collision
/// regardless of a midnight roll.
fn date_window() -> Vec<String> {
    let day = jiff::SignedDuration::from_hours(24);
    let now = jiff::Timestamp::now();
    [now.checked_sub(day).expect("now - 24h"), now, now.checked_add(day).expect("now + 24h")]
        .iter()
        .map(|ts| ts.strftime("%Y%m%d").to_string())
        .collect()
}

/// Entry names directly under `.specify/archive/plans` (empty when the
/// dir is absent).
fn archived_entries(project: &Project) -> Vec<String> {
    fs::read_dir(archive_dir(project))
        .map(|rd| {
            rd.filter_map(|e| e.ok().and_then(|e| e.file_name().into_string().ok())).collect()
        })
        .unwrap_or_default()
}

/// Locate the archived plan file `<name>-YYYYMMDD.yaml`, if the verb
/// wrote one — matched by shape rather than a clock-derived literal.
fn archived_plan_file(project: &Project, name: &str) -> Option<PathBuf> {
    let re = regex::Regex::new(&format!(r"^{}-\d{{8}}\.yaml$", regex::escape(name)))
        .expect("regex compiles");
    archived_entries(project)
        .into_iter()
        .find(|f| re.is_match(f))
        .map(|f| archive_dir(project).join(f))
}

/// Locate the co-moved archive directory `<name>-YYYYMMDD`, if any.
fn archived_plan_dir(project: &Project, name: &str) -> Option<PathBuf> {
    let re =
        regex::Regex::new(&format!(r"^{}-\d{{8}}$", regex::escape(name))).expect("regex compiles");
    archived_entries(project)
        .into_iter()
        .find(|f| re.is_match(f))
        .map(|f| archive_dir(project).join(f))
}

/// Replace any `-YYYYMMDD` date stamp in JSON strings with a stable
/// placeholder so the archive-success golden is date-insensitive.
fn strip_date_stamps(value: &mut Value) {
    fn visit(re: &regex::Regex, v: &mut Value) {
        match v {
            Value::String(s) if re.is_match(s) => {
                *s = re.replace_all(s, "-<YYYYMMDD>").into_owned();
            }
            Value::Array(items) => {
                for item in items {
                    visit(re, item);
                }
            }
            Value::Object(map) => {
                for (_k, v) in map.iter_mut() {
                    visit(re, v);
                }
            }
            _ => {}
        }
    }
    let re = regex::Regex::new(r"-\d{8}\b").expect("regex compiles");
    visit(&re, value);
}

fn archive_dir(project: &Project) -> PathBuf {
    project.root().join(".specify/archive/plans")
}

#[test]
fn plan_archive_happy_path_json() {
    let project = Project::init();
    project.seed_plan(ALL_DONE);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "archive"])
        .assert()
        .success();
    let mut actual = parse_stdout(&assert.get_output().stdout, project.root());

    assert_eq!(actual["plan"]["name"], "demo");
    assert!(
        actual["archived"].as_str().unwrap_or_default().contains("demo-"),
        "archived path should contain the plan name, got: {}",
        actual["archived"]
    );

    // Filesystem effects of the move, asserted here rather than in a
    // separate text-format twin.
    assert!(!project.plan_path().exists(), "original plan.yaml must be gone");
    assert!(
        archived_plan_file(&project, "demo").is_some(),
        "archived plan file not found under {}",
        archive_dir(&project).display()
    );

    strip_date_stamps(&mut actual);
    assert_golden("archive-success.json", actual);
}

#[test]
fn plan_archive_refuses_without_force() {
    let project = Project::init();
    project.seed_plan(A_DONE_B_PENDING);

    let assert =
        specify_cmd().current_dir(project.root()).args(["plan", "archive"]).assert().failure();
    assert_eq!(assert.get_output().status.code(), Some(1));
    let stderr = std::str::from_utf8(&assert.get_output().stderr).expect("utf8 stderr");
    assert!(
        stderr.contains('b'),
        "stderr should mention the pending entry name 'b', got: {stderr:?}"
    );
    assert!(stderr.contains("--force"), "stderr should suggest --force, got: {stderr:?}");

    assert!(project.plan_path().exists(), "plan.yaml must still exist");
    assert!(
        archived_plan_file(&project, "demo").is_none(),
        "no archive file should be written on refusal"
    );
}

#[test]
fn plan_archive_refuses_json_lists_entries() {
    let project = Project::init();
    project.seed_plan(A_DONE_B_PENDING);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "archive"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(1));

    // The typed failure envelope is written to stderr.
    let actual = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(actual["error"], "plan-has-outstanding-work");
    assert_eq!(actual["exit-code"], 1);
    let message = actual["message"].as_str().expect("message string");
    assert!(message.contains('b'), "message should mention the pending entry 'b': {message}");

    assert_golden("archive-outstanding-work.json", actual);
}

#[test]
fn plan_archive_with_force_succeeds() {
    let project = Project::init();
    project.seed_plan(A_DONE_B_PENDING);

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "archive", "--force"])
        .assert()
        .success();

    let archived =
        archived_plan_file(&project, "demo").expect("archived plan file must exist after --force");
    let contents = fs::read_to_string(&archived).expect("read archived yaml");
    assert!(
        contents.contains("name: b"),
        "archived yaml should preserve pending entry 'b':\n{contents}"
    );
    assert!(
        contents.contains("status: pending"),
        "archived yaml should preserve pending status verbatim:\n{contents}"
    );
}

#[test]
fn archive_filename_kebab_plus_date() {
    let project = Project::init();
    project.seed_plan(
        "\
name: my-change
slices: []
",
    );

    specify_cmd().current_dir(project.root()).args(["plan", "archive"]).assert().success();

    let re = regex::Regex::new(r"^my-change-\d{8}\.yaml$").expect("regex compiles");
    let entries: Vec<String> = fs::read_dir(archive_dir(&project))
        .expect("read archive dir")
        .filter_map(|e| e.ok().and_then(|e| e.file_name().into_string().ok()))
        .collect();
    assert_eq!(entries.len(), 1, "expected exactly one archive file, got: {entries:?}");
    assert!(
        re.is_match(&entries[0]),
        "archive filename {} should match `my-change-<YYYYMMDD>.yaml`",
        entries[0]
    );
}

#[test]
fn plan_archive_refuses_when_dest_exists() {
    let project = Project::init();
    project.seed_plan(ALL_DONE);

    let dest_dir = archive_dir(&project);
    fs::create_dir_all(&dest_dir).expect("mkdir archive dir");
    // Seed a collision file for every stamp the CLI's clock might pick
    // (see `date_window`) so the dest-exists guard fires deterministically.
    let seeded: Vec<PathBuf> = date_window()
        .iter()
        .map(|d| {
            let dest = dest_dir.join(format!("demo-{d}.yaml"));
            fs::write(&dest, "prior: content\n").expect("seed prior archive");
            dest
        })
        .collect();

    let assert =
        specify_cmd().current_dir(project.root()).args(["plan", "archive"]).assert().failure();
    assert_eq!(assert.get_output().status.code(), Some(1));
    let stderr = std::str::from_utf8(&assert.get_output().stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("already exists"),
        "stderr should mention 'already exists', got: {stderr:?}"
    );

    assert!(project.plan_path().exists(), "original plan.yaml must be untouched");
    for dest in &seeded {
        assert_eq!(
            fs::read_to_string(dest).expect("read prior archive"),
            "prior: content\n",
            "pre-existing archive destination must not be overwritten"
        );
    }
}

#[test]
fn plan_archive_missing_file_errors() {
    let project = Project::init();
    // Deliberately do NOT seed plan.yaml.

    let assert =
        specify_cmd().current_dir(project.root()).args(["plan", "archive"]).assert().failure();
    assert_eq!(assert.get_output().status.code(), Some(1));
    let stderr = std::str::from_utf8(&assert.get_output().stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("plan.yaml not found at"),
        "stderr should mention 'plan.yaml not found at', got: {stderr:?}"
    );
}

// -- plan archive co-move of working directory (L3.B) ---------------

/// Seed `.specify/plans/<name>/` with the given files and return
/// the directory path.
fn seed_working_dir(project: &Project, plan_name: &str, files: &[(&str, &[u8])]) -> PathBuf {
    let dir = project.root().join(".specify/plans").join(plan_name);
    fs::create_dir_all(&dir).expect("mkdir plans working dir");
    for (name, bytes) in files {
        fs::write(dir.join(name), bytes).expect("seed working file");
    }
    dir
}

#[test]
fn plan_archive_co_moves_working_dir() {
    let project = Project::init();
    project.seed_plan(ALL_DONE);
    let working_dir = seed_working_dir(
        &project,
        "demo",
        &[("discovery.md", b"# discovery\n"), ("proposal.md", b"# proposal\n")],
    );

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "archive"])
        .assert()
        .success();
    let mut actual = parse_stdout(&assert.get_output().stdout, project.root());

    assert_eq!(actual["plan"]["name"], "demo");
    assert!(
        actual["archived"].as_str().unwrap_or_default().contains("demo-"),
        "archived path should contain the plan name"
    );
    assert!(
        actual["archived-plans-dir"].as_str().unwrap_or_default().contains("demo-"),
        "archived-plans-dir should contain the plan name, got: {}",
        actual["archived-plans-dir"]
    );

    assert!(!working_dir.exists(), ".specify/plans/demo/ must be gone after archive");
    let archived_dir =
        archived_plan_dir(&project, "demo").expect("co-moved archive directory must exist");
    assert!(archived_dir.is_dir(), "co-moved dir missing at {}", archived_dir.display());
    assert_eq!(
        fs::read_to_string(archived_dir.join("discovery.md")).expect("read"),
        "# discovery\n"
    );
    assert_eq!(fs::read_to_string(archived_dir.join("proposal.md")).expect("read"), "# proposal\n");

    strip_date_stamps(&mut actual);
    assert_golden("archive-success-with-working-dir.json", actual);
}

#[test]
fn plan_archive_no_working_dir_json() {
    let project = Project::init();
    project.seed_plan(ALL_DONE);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "archive"])
        .assert()
        .success();
    let actual = parse_stdout(&assert.get_output().stdout, project.root());

    assert_eq!(
        actual["archived-plans-dir"],
        Value::Null,
        "no working dir must surface archived-plans-dir: null, got: {}",
        actual["archived-plans-dir"]
    );
}

#[test]
fn plan_archive_co_move_collision_halts() {
    let project = Project::init();
    project.seed_plan(ALL_DONE);
    let working_dir = seed_working_dir(&project, "demo", &[("notes.md", b"# notes\n")]);

    // Pre-create the co-move destination dir for the whole date window so
    // the working-dir preflight collides regardless of a midnight roll;
    // the plan.yaml archive destination stays clear, isolating the
    // working-dir preflight specifically.
    let seeded_dirs: Vec<PathBuf> = date_window()
        .iter()
        .map(|d| {
            let dir = archive_dir(&project).join(format!("demo-{d}"));
            fs::create_dir_all(&dir).expect("seed collision dir");
            dir
        })
        .collect();

    let assert =
        specify_cmd().current_dir(project.root()).args(["plan", "archive"]).assert().failure();
    assert_eq!(assert.get_output().status.code(), Some(1));
    let stderr = std::str::from_utf8(&assert.get_output().stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("already exists"),
        "stderr should name 'already exists', got: {stderr:?}"
    );

    // Preflight contract: plan.yaml must be untouched on collision.
    assert!(
        project.plan_path().exists(),
        "plan.yaml MUST be untouched when working-dir preflight fails"
    );
    assert!(working_dir.is_dir(), "source working dir must be untouched on collision");
    assert!(
        archived_plan_file(&project, "demo").is_none(),
        "plan.yaml must not have been archived on collision"
    );
    for dir in &seeded_dirs {
        assert!(
            dir.is_dir() && fs::read_dir(dir).expect("read").next().is_none(),
            "pre-existing collision dir must remain empty"
        );
    }
}
