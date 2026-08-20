//! Walking-skeleton journey: scripted specify over the mock source.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

struct Home {
    temp: tempfile::TempDir,
    script: &'static str,
}

impl Home {
    fn stage(names: &[&str]) -> Self {
        let temp = tempfile::tempdir().expect("journey tempdir");
        fs::create_dir(temp.path().join("project")).expect("mkdir project");
        for name in names {
            let staged = temp.path().join(format!("project/{name}.wasm"));
            fs::copy(component(), &staged).expect("stage component");
        }
        Self {
            temp,
            script: "tests/journey-script-minimal",
        }
    }

    fn scaffold() -> Self {
        let home = Self::stage(&["source"]);
        let init = home.emery(&["init", "source.wasm"]);
        assert!(
            home.project().join(".emery-cache/components/source.wasm").is_file(),
            "the local component is mirrored into the CWD-relative project cache"
        );
        assert!(
            init.status.success(),
            "init over the built source component must scaffold:\n{}",
            String::from_utf8_lossy(&init.stderr)
        );
        home
    }

    fn project(&self) -> PathBuf {
        self.temp.path().join("project")
    }

    fn emery(&self, args: &[&str]) -> Output {
        self.emery_in(&self.project(), args)
    }

    fn emery_in(&self, dir: &Path, args: &[&str]) -> Output {
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join(self.script);
        std::process::Command::new(harness())
            .current_dir(dir)
            .env("EMERY_JOURNEY_SCRIPT", script)
            // Ephemeral bind so parallel runs do not share :8080.
            .env("HTTP_ADDR", "127.0.0.1:0")
            .args(args)
            .output()
            .expect("run the journey host")
    }

    fn specify(&self) -> Output {
        self.emery(&["specify"])
    }
}

#[test]
fn journey() {
    let home = Home::scaffold();

    let specify = home.specify();
    assert!(
        specify.status.success(),
        "`emery specify` must complete the journey (remediation Phase 3 exit criterion):\n{}",
        String::from_utf8_lossy(&specify.stderr)
    );

    let spec = read(&home.project(), "spec.md");
    let design = read(&home.project(), "design.md");
    assert!(!design.is_empty(), "design.md carries the rebuild design");
    assert!(
        spec.contains("[unknown]"),
        "gaps are preserved as `[unknown]`, never guessed (product artifact authority):\n{spec}"
    );
}

#[test]
fn adr_0001_generation_swap() {
    let home = Home::scaffold();

    let first = home.specify();
    assert!(
        first.status.success(),
        "`emery specify` must commit a spec set before convergence can hold:\n{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let before = snapshot(&home.project());

    let second = home.specify();
    assert!(second.status.success(), "re-run is the resume path (ADR-0001)");
    assert_eq!(before, snapshot(&home.project()), "re-run must be byte-stable (§8 item 2)");

    // Partial generation + unnamed temp, as a kill mid-swap would leave.
    let spec_home = home.project().join(".emery/spec");
    let partial = spec_home.join("generations/deadbeef");
    fs::create_dir_all(&partial).expect("stage the partial generation");
    fs::write(partial.join("spec.md"), "half-written").expect("stage the partial document");
    fs::write(spec_home.join(".tmp-crash"), "temp litter").expect("stage the temp litter");

    let recovered = home.specify();
    assert!(
        recovered.status.success(),
        "re-run after a crash is the recovery path (ADR-0001):\n{}",
        String::from_utf8_lossy(&recovered.stderr)
    );
    assert!(!partial.exists(), "crash litter is pruned on the next commit");
    assert_eq!(
        before,
        snapshot(&home.project()),
        "the previous set survives the crash and recovery converges on it"
    );
}

#[test]
fn init_contract() {
    let home = Home::stage(&["source"]);
    let manifest = home.project().join(".emery/project.yaml");

    let refused = home.emery(&["init"]);
    assert_eq!(refused.status.code(), Some(2), "init-source-required is a validation refusal");
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains("init-source-required"), "{stderr}");
    assert!(!manifest.exists(), "a refused init scaffolds nothing");

    let duplicate = home.emery(&["init", "source.wasm", "source.wasm"]);
    assert_eq!(duplicate.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&duplicate.stderr);
    assert!(stderr.contains("init-source-duplicate"), "{stderr}");
    let malformed = home.emery(&["init", "--value", "no-equals-sign"]);
    assert_eq!(malformed.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&malformed.stderr);
    assert!(stderr.contains("--value"), "{stderr}");

    let first = home.emery(&["init", "source.wasm"]);
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));
    let before = fs::read_to_string(&manifest).expect("project.yaml");
    let reentry = home.emery(&["init", "source.wasm"]);
    assert!(reentry.status.success(), "{}", String::from_utf8_lossy(&reentry.stderr));
    assert_eq!(before, fs::read_to_string(&manifest).expect("project.yaml"), "re-entry is a noop");

    // Age the pin; `--upgrade` restores this binary's version.
    let aged = before.replace(&format!("emery: {}", env!("CARGO_PKG_VERSION")), "emery: 0.1.0");
    assert_ne!(aged, before, "the fixture must carry the pin to age");
    fs::write(&manifest, aged).expect("age the pin");
    let upgrade = home.emery(&["init", "--upgrade"]);
    assert!(upgrade.status.success(), "{}", String::from_utf8_lossy(&upgrade.stderr));
    let upgraded = fs::read_to_string(&manifest).expect("project.yaml");
    assert!(
        upgraded.contains(&format!("emery: {}", env!("CARGO_PKG_VERSION"))),
        "the pin returns to the running binary's version:\n{upgraded}"
    );
    assert!(upgraded.contains("source"), "bindings survive the upgrade:\n{upgraded}");
}

#[test]
fn adr_0010_remine_diff() {
    let home = Home::scaffold();

    let first = home.specify();
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));
    let stdout = String::from_utf8_lossy(&first.stdout);
    assert!(!stdout.contains("diff vs"), "a first run has nothing to diff against: {stdout}");

    let rerun = home.specify();
    assert!(rerun.status.success(), "{}", String::from_utf8_lossy(&rerun.stderr));
    let stdout = String::from_utf8_lossy(&rerun.stdout);
    assert!(
        stdout.contains("diff vs") && stdout.contains("none (byte-stable)"),
        "an unchanged re-run reports an explicit empty diff: {stdout}"
    );

    let generations: Vec<_> = fs::read_dir(home.project().join(".emery/spec/generations"))
        .expect("generations dir")
        .collect();
    assert_eq!(generations.len(), 1, "the superseded generation is pruned, never retained");
}

// Deployment policy is CWD-rooted with no ancestor walk: a verb run
// below the project root fails typed instead of anchoring to a
// parent's `.emery/project.yaml`.
#[test]
fn cwd_is_the_project_root() {
    let home = Home::scaffold();
    let nested = home.project().join("src/deeply/nested");
    fs::create_dir_all(&nested).expect("mkdir nested dir");

    let specify = home.emery_in(&nested, &["specify"]);
    assert!(
        !specify.status.success(),
        "a nested CWD must not discover the parent project:\n{}",
        String::from_utf8_lossy(&specify.stdout)
    );
    let stderr = String::from_utf8_lossy(&specify.stderr);
    assert!(stderr.contains("not-initialized"), "{stderr}");
}

// Bare-name and pin dispatch beyond guests declared in the runtime
// invocation is parked with the dynamic resolver.

fn component() -> PathBuf {
    let built = target_dir().join("wasm32-wasip2/release/examples/source.wasm");
    assert!(
        built.is_file(),
        "seam fixture missing at {}; run `cargo make journey` (or `cargo make source`)",
        built.display()
    );
    built
}

fn harness() -> PathBuf {
    let built = target_dir().join("debug/examples/runtime");
    assert!(
        built.is_file(),
        "journey host missing at {}; run `cargo make journey`",
        built.display()
    );
    built
}

fn target_dir() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from)
}

fn find(dir: &Path, suffix: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            if let Some(found) = find(&path, suffix) {
                return Some(found);
            }
        } else if path.ends_with(suffix) {
            return Some(path);
        }
    }
    None
}

fn read(dir: &Path, name: &str) -> String {
    let path = find(dir, name)
        .unwrap_or_else(|| panic!("`{name}` must exist in the output home (§8 item 1)"));
    fs::read_to_string(path).expect("read spec artifact")
}

fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, tree: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                walk(root, &path, tree);
            } else {
                let relative = path.strip_prefix(root).expect("under root").to_path_buf();
                tree.insert(relative, fs::read(&path).expect("read snapshot file"));
            }
        }
    }
    let mut tree = BTreeMap::new();
    walk(dir, dir, &mut tree);
    tree
}
