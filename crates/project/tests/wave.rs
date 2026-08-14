//! One-member target wave manifests + `target.wave.opened` (RFC-86 D9).

use jiff::Timestamp;
use project::config::Layout;
use project::journal::{EventKind, read_union};
use project::name::SliceName;
use project::snapshot::SnapshotId;
use project::wave::{EpochRef, Wave};

const fn layout(root: &std::path::Path) -> Layout<'_> {
    Layout::new(root)
}

fn ts(second: i64) -> Timestamp {
    Timestamp::from_second(1_700_000_000 + second).expect("valid timestamp")
}

fn cid(hex64: char) -> SnapshotId {
    SnapshotId::from_digest(&hex64.to_string().repeat(64))
}

fn sample(target: &str, slice: &str) -> Wave {
    Wave::one_member(
        target,
        cid('a'),
        SliceName::from(slice),
        cid('b'),
        vec![SliceName::from("upstream")],
        EpochRef {
            writer: "local".into(),
            sequence: 7,
        },
    )
}

#[test]
fn loads_prior_actor_build() {
    let yaml = "\
target: demo
base: sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
members:
- slice: login-flow
  inputs:
    refinement: sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
build-authorization:
  actor: local
  sequence: 7
";
    let wave: Wave = serde_saphyr::from_str(yaml).expect("prior actor epoch ref");
    assert_eq!(
        wave.build_authorization,
        EpochRef {
            writer: "local".into(),
            sequence: 7,
        }
    );
}

#[test]
fn write_and_load_round_trip() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let layout = layout(tmp.path());
    let wave = sample("demo", "login-flow");

    let digest = wave.write(layout).expect("write");
    let loaded = Wave::load(layout, "demo", digest.digest()).expect("load by hex");
    assert_eq!(loaded, wave);

    let via_wire = Wave::load(layout, "demo", digest.as_str()).expect("load by sha256:");
    assert_eq!(via_wire, wave);

    let path = layout.target_wave_path("demo", digest.digest());
    assert!(path.is_file(), "manifest at {}", path.display());
    let yaml = std::fs::read_to_string(&path).expect("read yaml");
    assert!(yaml.contains("target: demo"), "{yaml}");
    assert!(yaml.contains("build-authorization:"), "{yaml}");
    assert!(yaml.contains("depends-on:"), "{yaml}");
    assert!(yaml.contains("login-flow"), "{yaml}");
    assert!(yaml.contains("refinement:"), "{yaml}");
}

#[test]
fn open_appends_target_wave() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let layout = layout(tmp.path());
    let wave = sample("demo", "login-flow");

    let opened = wave.open(layout, ts(0)).expect("open");
    assert!(opened.path.is_file());
    assert_eq!(opened.digest, wave.digest().expect("digest"));

    let events = read_union(layout).expect("union");
    assert_eq!(events.len(), 1);
    match &events[0].kind {
        EventKind::TargetWaveOpened {
            target,
            digest,
            slice_name,
        } => {
            assert_eq!(target, "demo");
            assert_eq!(digest, opened.digest.as_str());
            assert_eq!(slice_name.as_str(), "login-flow");
        }
        other => panic!("unexpected kind: {other:?}"),
    }
}

#[test]
fn digest_filename_matches() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let layout = layout(tmp.path());
    let wave = sample("demo", "orders-api");
    let digest = wave.write(layout).expect("write");
    let yaml = wave.canonical_yaml().expect("yaml");
    let expected = SnapshotId::from_digest(&diagnostics::digest::sha256_hex(yaml.as_bytes()));
    assert_eq!(digest, expected);
    let path = tmp
        .path()
        .join(".emery/change/targets/demo/waves")
        .join(format!("{}.yaml", digest.digest()));
    assert_eq!(layout.target_wave_path("demo", digest.digest()), path);
    assert_eq!(std::fs::read_to_string(&path).expect("bytes"), yaml);
}

#[test]
fn refuse_empty_multi_member() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let layout = layout(tmp.path());

    let mut empty = sample("demo", "login-flow");
    empty.members.clear();
    let err = empty.write(layout).expect_err("empty members");
    assert!(err.to_string().contains("target-wave-member-count"), "{err}");

    let mut multi = sample("demo", "login-flow");
    multi.members.push(multi.members[0].clone());
    let err = multi.open(layout, ts(1)).expect_err("two members");
    assert!(err.to_string().contains("target-wave-member-count"), "{err}");
}

#[test]
fn write_once_identical_ok() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let layout = layout(tmp.path());
    let wave = sample("demo", "login-flow");
    let first = wave.write(layout).expect("first write");
    let second = wave.write(layout).expect("idempotent rewrite");
    assert_eq!(first, second);

    // Same digest path with different bytes cannot occur via Wave::write
    // (digest covers payload). Plant a conflicting file by hand.
    let path = layout.target_wave_path("demo", first.digest());
    std::fs::write(&path, "target: tampered\n").expect("tamper");
    let err = wave.write(layout).expect_err("conflict");
    assert!(err.to_string().contains("target-wave-conflict"), "{err}");
}
