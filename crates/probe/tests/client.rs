//! The client composition: tracing init and the `EVAL_LOG` file copy.
#![cfg(feature = "client")]

use std::ffi::OsStr;

use tempfile::TempDir;

/// Set one process-env knob before any runtime thread exists.
#[expect(
    unsafe_code,
    reason = "env is the client's composition seam; nextest isolates the process"
)]
fn setenv(key: &str, value: impl AsRef<OsStr>) {
    // SAFETY: callers run single-threaded, before the runtime spawns.
    unsafe { std::env::set_var(key, value) };
}

/// Clear one process-env knob before any runtime thread exists.
#[expect(
    unsafe_code,
    reason = "env is the client's composition seam; nextest isolates the process"
)]
fn unsetenv(key: &str) {
    // SAFETY: callers run single-threaded, before the runtime spawns.
    unsafe { std::env::remove_var(key) };
}

#[test]
fn eval_log_mirror() {
    let tmp = TempDir::new().expect("tempdir");
    let log = tmp.path().join("logs").join("run.log");
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).expect("mkdir");

    setenv("RUST_LOG", "info");
    setenv("EVAL_LOG", &log);

    // Any deterministic passthrough installs the subscriber; the engine
    // crates emit no events themselves, so the test emits the probe.
    let argv = vec![
        "probe".to_string(),
        "--project-dir".to_string(),
        project.display().to_string(),
        "slice".to_string(),
        "list".to_string(),
    ];
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    runtime.block_on(probe::client::run(argv, mock::catalog(), None, None)).expect("run");

    // Span fields are the ANSI-leak surface: both layers used to share
    // a DefaultFields cache, so the file reused the console's colored
    // formatting (and double-appended on every Span::record).
    let span = tracing::info_span!("eval.case", case = "smoke", until = tracing::field::Empty,);
    let _guard = span.enter();
    tracing::Span::current().record("until", "execute");
    tracing::info!("eval-log-smoke");

    let contents = std::fs::read_to_string(&log).expect("EVAL_LOG file exists");
    assert!(contents.contains("eval-log-smoke"), "the file mirrors events: {contents:?}");
    assert!(!contents.contains('\u{1b}'), "the file copy is ANSI-free: {contents:?}");
    assert!(
        contents.contains("case=\"smoke\"") && contents.contains("until=\"execute\""),
        "span fields are plain: {contents:?}"
    );
    assert_eq!(
        contents.matches("until=\"execute\"").count(),
        1,
        "Span::record must not double-append: {contents:?}"
    );
}

#[test]
fn log_flags_peel_dispatch() {
    let tmp = TempDir::new().expect("tempdir");
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).expect("mkdir");

    unsetenv("EVAL_LOG");
    unsetenv("RUST_LOG");

    // `--quiet` ahead of `--project-dir` and repeated after the verb:
    // both occurrences peel before the grammar (which would reject an
    // unknown flag) and the `--project-dir` probe (which only inspects
    // argv[1]) ever see them.
    let argv = vec![
        "probe".to_string(),
        "--quiet".to_string(),
        "--project-dir".to_string(),
        project.display().to_string(),
        "slice".to_string(),
        "list".to_string(),
        "--quiet".to_string(),
    ];
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    runtime.block_on(probe::client::run(argv, mock::catalog(), None, None)).expect("run");
}

#[test]
fn reserved_log_flags() {
    let argv = vec![
        "probe".to_string(),
        "--debug".to_string(),
        "--quiet".to_string(),
        "slice".to_string(),
        "list".to_string(),
    ];
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let err = runtime
        .block_on(probe::client::run(argv, mock::catalog(), None, None))
        .expect_err("the flags are mutually exclusive");
    assert!(format!("{err:#}").contains("mutually exclusive"), "{err:#}");
}

#[test]
fn eval_log_inferred_per() {
    let tmp = TempDir::new().expect("tempdir");
    let sandbox = tmp.path().join("sandbox");

    unsetenv("EVAL_LOG");
    setenv("RUST_LOG", "info");

    // The destination is inferred before dispatch, so the run may fail
    // afterwards (this composition carries no `cases` root) and the
    // announced log file still exists.
    let argv = vec!["probe".to_string(), "eval".to_string(), "some-case".to_string()];
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let err = runtime
        .block_on(probe::client::run(argv, mock::catalog(), None, Some(&sandbox)))
        .expect_err("no cases root is carried");
    assert!(format!("{err:#}").contains("no cases"), "{err:#}");

    let logs = sandbox.join("logs").join("some-case");
    let entries: Vec<_> = std::fs::read_dir(&logs).expect("inferred log dir").flatten().collect();
    assert_eq!(entries.len(), 1, "one per-run log file: {entries:?}");
    let name = entries[0].file_name();
    let name = name.to_string_lossy();
    assert!(name.starts_with("eval-") && name.ends_with(".log"), "timestamped name: {name}");
    let contents = std::fs::read_to_string(entries[0].path()).expect("log file");
    assert!(contents.contains("eval log: "), "the destination is announced: {contents:?}");
}
