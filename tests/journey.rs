//! Walking-skeleton journey (target-architecture §8): the spec
//! generator over the mock source components, scripted and offline —
//! the Phase 3 exit criterion, required in CI (ADR-0008, ADR-0009).
//!
//! Excluded from the green per-push gate by the nextest
//! `default-filter`; run via `cargo make journey` (its own required
//! CI job). It drives the dev-only journey host (`crates/
//! journey-host`) — the shipped runtime shape with the same guest
//! bytes, mounts, and resolver, substituting only the `WasiModel`
//! host capability with a scripted backend answering from the
//! committed `tests/journey-script/` fixtures (ADR-0009 §5; ADR-0002
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
}

impl Home {
    /// Stage the built mock component under each `name` inside a
    /// fresh project directory — no init yet. Adapter names derive
    /// from the file stems, which select the `mock::behaviour`
    /// profiles. Staged inside the project: the launcher mounts only
    /// the project root and the cache, so the guest cannot read a
    /// component outside them.
    fn stage(names: &[&str]) -> Self {
        let temp = tempfile::tempdir().expect("journey tempdir");
        fs::create_dir(temp.path().join("project")).expect("mkdir project");
        for name in names {
            let staged = temp.path().join(format!("project/{name}.wasm"));
            fs::copy(component(), &staged).expect("stage component");
        }
        Self { temp }
    }

    /// Scaffold a project over the staged mock components: the
    /// component-seam fixture resolving through the real binary
    /// (ADR-0002 / CC-17 — the shipped seam is the tested seam).
    fn scaffold() -> Self {
        // `mock-docs` (documentation) and `mock-code` (behaviour)
        // stage the adversarial session-timeout pair; `mock-intent`
        // extracts the inline operator directive that outranks both
        // (ADR-0009 §1).
        let home = Self::stage(&["mock-docs", "mock-code", "mock-intent"]);
        let init = home.emery(&[
            "init",
            "mock-docs.wasm",
            "mock-code.wasm",
            "--value",
            "mock-intent.wasm=Sessions must expire after 30 minutes of inactivity.",
        ]);
        // Holds today: the built components cross the launcher
        // mounts and mirror into the project cache with provenance —
        // the seam fixture itself is sound (CC-17).
        let mirrored = find(home.temp.path(), "components/mock-docs.wasm");
        assert!(mirrored.is_some(), "the local component is mirrored into the project cache");
        assert!(
            init.status.success(),
            "init over the built mock components must scaffold:\n{}",
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
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/journey-script");
        std::process::Command::new(harness())
            .current_dir(self.project())
            .env("EMERY_HOME", self.temp.path().join("emery-home"))
            .env("EMERY_JOURNEY_SCRIPT", script)
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

/// ADR-0004 Option D tripwire (the `adr_0004_*` gate): the
/// documentation/behaviour disagreement staged by the adversarial
/// mock pair surfaces inline as `[conflict]` or `[divergence]` in
/// the spec — never auto-deferred, never silently won (§8 item 3).
#[test]
fn adr_0004_conflict_inline() {
    let home = Home::scaffold();

    let specify = home.specify();
    assert!(
        specify.status.success(),
        "`emery specify` must synthesise before conflicts can surface:\n{}",
        String::from_utf8_lossy(&specify.stderr)
    );

    let spec = read(&home.project(), "spec.md");
    assert!(
        spec.contains("[conflict]") || spec.contains("[divergence]"),
        "the staged session-timeout disagreement (docs 30m vs behaviour 15m) must appear \
         inline:\n{spec}"
    );
}

/// The `init` wire contract over the component seam (ADR-0009 §1):
/// typed refusals, first scaffold, idempotent re-entry, and the
/// `--upgrade` pin bump.
#[test]
fn init_contract() {
    let home = Home::stage(&["mock-docs"]);
    let manifest = home.project().join(".emery/project.yaml");

    // No sources is a typed refusal that writes nothing.
    let refused = home.emery(&["init"]);
    assert_eq!(refused.status.code(), Some(2), "init-source-required is a validation refusal");
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains("init-source-required"), "{stderr}");
    assert!(!manifest.exists(), "a refused init scaffolds nothing");

    // A twice-bound key and a `--value` without `=` refuse likewise.
    let duplicate = home.emery(&["init", "mock-docs.wasm", "mock-docs.wasm"]);
    assert_eq!(duplicate.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&duplicate.stderr);
    assert!(stderr.contains("init-source-duplicate"), "{stderr}");
    let malformed = home.emery(&["init", "--value", "no-equals-sign"]);
    assert_eq!(malformed.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&malformed.stderr);
    assert!(stderr.contains("--value"), "{stderr}");

    // First init scaffolds; re-entry changes nothing.
    let first = home.emery(&["init", "mock-docs.wasm"]);
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));
    let before = fs::read_to_string(&manifest).expect("project.yaml");
    let reentry = home.emery(&["init", "mock-docs.wasm"]);
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
    assert!(upgraded.contains("mock-docs"), "bindings survive the upgrade:\n{upgraded}");
}

/// The A8 required-extras gate over the component seam (CC-01): a
/// requirement claim crossing the seam without its `statement` extra
/// is a typed refusal naming source, claim, and key — never a
/// synopsis fallback — and commits nothing.
#[test]
fn adr_0009_extras_gate() {
    let home = Home::stage(&["mock-missing-extras"]);
    let init = home.emery(&["init", "mock-missing-extras.wasm"]);
    assert!(init.status.success(), "{}", String::from_utf8_lossy(&init.stderr));

    let specify = home.specify();
    assert_eq!(specify.status.code(), Some(2), "the A8 refusal is a validation failure");
    let stderr = String::from_utf8_lossy(&specify.stderr);
    assert!(stderr.contains("claim-extras-missing"), "{stderr}");
    assert!(stderr.contains("mock-missing-extras"), "names the source: {stderr}");
    assert!(stderr.contains("greeting.behaviour"), "names the claim: {stderr}");
    assert!(stderr.contains("statement"), "names the missing key: {stderr}");
    assert!(find(&home.project(), "spec.md").is_none(), "a refused run commits nothing");
}

/// A failing source propagates typed across the seam: the extract
/// error names the routed identity and no generation commits.
#[test]
fn extract_failure_typed() {
    let home = Home::stage(&["mock-fail-extract"]);
    let init = home.emery(&["init", "mock-fail-extract.wasm"]);
    assert!(init.status.success(), "{}", String::from_utf8_lossy(&init.stderr));

    let specify = home.specify();
    assert!(!specify.status.success(), "the failing source must fail the run");
    let stderr = String::from_utf8_lossy(&specify.stderr);
    assert!(stderr.contains("source-extract-failed"), "{stderr}");
    assert!(stderr.contains("mock-fail-extract"), "names the source: {stderr}");
    assert!(find(&home.project(), "spec.md").is_none(), "a failed run commits nothing");
}

/// The built seam fixture, honouring a redirected target directory.
fn component() -> PathBuf {
    let built = target_dir().join("wasm32-wasip2/release/mock_component.wasm");
    assert!(
        built.is_file(),
        "seam fixture missing at {}; run `cargo make journey` (or `cargo make mock-component`)",
        built.display()
    );
    built
}

/// The built journey host (ADR-0009 §5), honouring a redirected
/// target directory.
fn harness() -> PathBuf {
    let built = target_dir().join("debug/emery-journey-host");
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
