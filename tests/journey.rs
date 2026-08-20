//! Walking-skeleton journey (target-architecture §8): the spec
//! generator over the mock source components, scripted and offline —
//! the Phase 3 exit criterion, required in CI (ADR-0008, ADR-0009).
//!
//! Excluded from the green per-push gate by the nextest
//! `default-filter`; run via `cargo make journey` (its own required
//! CI job). It drives the runtime example (`examples/runtime.rs`) — the shipped runtime shape with the same guest
//! bytes, mounts, and resolver, substituting only the `WasiModel`
//! host capability with a scripted backend answering from the
//! committed `tests/journey-script-minimal/` fixtures (ADR-0009 §5; ADR-0002
//! §1 records the model as a host capability, so scripting it needs
//! no guest change). Since the T10 spine cut this binary also carries
//! the engine/transport integration legs the native provider used to
//! host — the component seam is the one integration rung (ADR-0002
//! §2). Extension is expected; weakening an assertion to pass is
//! never the fix (the R3 lesson).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

/// One scaffolded journey home: an isolated `EMERY_HOME`, a project
/// directory, and the staged mock component (already `init`ed).
struct Home {
    temp: tempfile::TempDir,
    /// Manifest-relative scripted-model fixture directory.
    script: &'static str,
}

impl Home {
    /// Stage the built mock component under each `name` inside a
    /// fresh project directory — no init yet. The journey binds one
    /// adapter (`source`). Staged inside the project: the launcher
    /// mounts only the project root and the cache, so the guest
    /// cannot read a component outside them.
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

    /// Scaffold a project over the one staged source component: the
    /// component-seam fixture resolving through the real binary
    /// (ADR-0002 / CC-17 — the shipped seam is the tested seam).
    fn scaffold() -> Self {
        let home = Self::stage(&["source"]);
        let init = home.emery(&["init", "source.wasm"]);
        // Holds today: the built component crosses the launcher
        // mounts and mirrors into the project cache with provenance —
        // the seam fixture itself is sound (CC-17).
        let mirrored = find(home.temp.path(), "components/source.wasm");
        assert!(mirrored.is_some(), "the local component is mirrored into the project cache");
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

    /// Run the journey host in the project, isolated from any real
    /// operator state and network: `EMERY_HOME` is inside the temp
    /// home, the staged local component needs no registry, and the
    /// scripted model answers from the committed fixture directory.
    fn emery(&self, args: &[&str]) -> Output {
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join(self.script);
        std::process::Command::new(harness())
            .current_dir(self.project())
            .env("EMERY_HOME", self.temp.path().join("emery-home"))
            .env("EMERY_JOURNEY_SCRIPT", script)
            // Ephemeral bind so parallel journey processes do not
            // collide on Omnia's default `0.0.0.0:8080`.
            .env("HTTP_ADDR", "127.0.0.1:0")
            .args(args)
            .output()
            .expect("run the journey host")
    }

    fn specify(&self) -> Output {
        self.emery(&["specify"])
    }
}

/// §8 items 1–2: `emery specify` synthesises `spec.md` / `design.md`,
/// gaps stay typed `[unknown]`, and a re-run is byte-stable.
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

/// ADR-0001 Option C tripwire (the `adr_0001_*` gate): each run
/// commits one complete generation behind the atomically swapped
/// pointer, so a re-run converges to a byte-stable output home; a
/// crash between generation write and pointer swap leaves the
/// previous set intact (the spike clause, ADR-0009 §6).
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

    // Crash injection: stage exactly the residue a kill between
    // generation write and pointer swap leaves — a partial generation
    // directory and temp litter the pointer never named.
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

/// The `init` wire contract over the component seam (ADR-0009 §1):
/// typed refusals, first scaffold, idempotent re-entry, and the
/// `--upgrade` pin bump.
#[test]
fn init_contract() {
    let home = Home::stage(&["source"]);
    let manifest = home.project().join(".emery/project.yaml");

    // No sources is a typed refusal that writes nothing.
    let refused = home.emery(&["init"]);
    assert_eq!(refused.status.code(), Some(2), "init-source-required is a validation refusal");
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains("init-source-required"), "{stderr}");
    assert!(!manifest.exists(), "a refused init scaffolds nothing");

    // A twice-bound key and a `--value` without `=` refuse likewise.
    let duplicate = home.emery(&["init", "source.wasm", "source.wasm"]);
    assert_eq!(duplicate.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&duplicate.stderr);
    assert!(stderr.contains("init-source-duplicate"), "{stderr}");
    let malformed = home.emery(&["init", "--value", "no-equals-sign"]);
    assert_eq!(malformed.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&malformed.stderr);
    assert!(stderr.contains("--value"), "{stderr}");

    // First init scaffolds; re-entry changes nothing.
    let first = home.emery(&["init", "source.wasm"]);
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));
    let before = fs::read_to_string(&manifest).expect("project.yaml");
    let reentry = home.emery(&["init", "source.wasm"]);
    assert!(reentry.status.success(), "{}", String::from_utf8_lossy(&reentry.stderr));
    assert_eq!(before, fs::read_to_string(&manifest).expect("project.yaml"), "re-entry is a noop");

    // Age the pin (simulating a project initialised by an older
    // binary), then `--upgrade`: bindings survive, the pin returns to
    // the running binary's version.
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

/// ADR-0010: `emery specify` reports the re-mine diff in its success
/// envelope — computed at commit time against the generation it
/// supersedes, never persisted (ADR-0009 §2). A first run has no
/// diff; a byte-stable re-run reports an explicit empty diff.
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

    // Nothing persists for the diff: one generation, no retained
    // history, no diff artifact (ADR-0009 §2).
    let generations: Vec<_> = fs::read_dir(home.project().join(".emery/spec/generations"))
        .expect("generations dir")
        .collect();
    assert_eq!(generations.len(), 1, "the superseded generation is pruned, never retained");
}

// ADR-0002 embedded-registry and CC-17 exact-pin admission are parked
// while `resolver` is unused by `src/main.rs` / `examples/runtime.rs`.
// Bare `source` and `source@1.2.3` dispatch guest ids the static
// `source:source` registration does not serve.

/// The built seam fixture, honouring a redirected target directory.
fn component() -> PathBuf {
    let built = target_dir().join("wasm32-wasip2/release/examples/source.wasm");
    assert!(
        built.is_file(),
        "seam fixture missing at {}; run `cargo make journey` (or `cargo make source`)",
        built.display()
    );
    built
}

/// The built journey host (ADR-0009 §5), honouring a redirected
/// target directory.
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

/// Find one file by trailing path components anywhere under `dir` —
/// the output-home layout is Phase 3 design, so the journey pins
/// names, not full paths.
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

/// Every file under `dir` by relative path and bytes.
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
