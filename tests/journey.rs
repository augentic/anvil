//! Walking-skeleton journey (target-architecture §8): the shipped
//! binary over the mock source component, scripted and offline —
//! red by design until Phase 3 (ADR-0008).
//!
//! Excluded from the green per-push gate by the nextest
//! `default-filter`; run via `cargo make journey`. Red today at the
//! `init` step: the shipped guest refuses `adapter-axis-removed`
//! because residual v1 init still resolves its adapter on the deleted
//! target axis — the composition finding this rung exists to surface
//! (T1/T3), repaired only by Phase 3's re-derived init, never by
//! patching the deletion frontier. Behind it, `specify` is a typed
//! stub. Green is the Phase 3 exit criterion, flipped to a required
//! CI rung when the spec generator lands.
//!
//! The scripted-model host wiring is not needed while red — the stub
//! fails before any model dispatch (ADR-0002 §1 records the model as
//! a host capability, so scripting it needs no guest change).

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
    /// Scaffold a project over the staged `mock-docs` component: the
    /// component-seam fixture resolving through the real binary
    /// (ADR-0002 / CC-17 — the shipped seam is the tested seam).
    fn scaffold() -> Self {
        let temp = tempfile::tempdir().expect("journey tempdir");
        fs::create_dir(temp.path().join("project")).expect("mkdir project");
        // The adapter name derives from the file stem; `mock-docs`
        // selects the documentation half of the adversarial pair in
        // `mock::behaviour`. The behaviour (`mock-code`) half and an
        // inline intent source join when Phase 3 designs the
        // source-binding surface — the fixture is ready for both.
        // Staged inside the project: the launcher mounts only the
        // project root and the cache, so the guest cannot read a
        // component outside them.
        let staged = temp.path().join("project/mock-docs.wasm");
        fs::copy(component(), &staged).expect("stage component");

        let home = Self { temp };
        let init = home.emery(&["init", "mock-docs.wasm"]);
        // Holds today: the built component crosses the launcher
        // mounts and mirrors into the project cache with provenance —
        // the seam fixture itself is sound (CC-17).
        let mirrored = find(home.temp.path(), "components/mock-docs.wasm");
        assert!(mirrored.is_some(), "the local component is mirrored into the project cache");
        // Red today (`adapter-axis-removed`): residual v1 init
        // resolves the deleted target axis. Phase 3 re-derives init
        // for the spec generator; this assertion flips green there.
        assert!(
            init.status.success(),
            "init over the built mock component must scaffold:\n{}",
            String::from_utf8_lossy(&init.stderr)
        );
        home
    }

    fn project(&self) -> PathBuf {
        self.temp.path().join("project")
    }

    /// Run the shipped binary in the project, isolated from any real
    /// operator state and network: `EMERY_HOME` is inside the temp
    /// home and the staged local component needs no registry.
    fn emery(&self, args: &[&str]) -> Output {
        std::process::Command::new(env!("CARGO_BIN_EXE_emery"))
            .current_dir(self.project())
            .env("EMERY_HOME", self.temp.path().join("emery-home"))
            .args(args)
            .output()
            .expect("run emery")
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
/// pointer, so a re-run converges to a byte-stable output home.
/// Phase 3 extends this with crash injection (a crash mid-write
/// leaves the previous set) once the generation layout exists.
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

/// The built seam fixture, honouring a redirected target directory.
fn component() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let target =
        std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from);
    let built = target.join("wasm32-wasip2/release/mock_component.wasm");
    assert!(
        built.is_file(),
        "seam fixture missing at {}; run `cargo make journey` (or `cargo make mock-component`)",
        built.display()
    );
    built
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
