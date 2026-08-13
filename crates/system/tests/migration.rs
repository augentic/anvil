//! `migration.yaml` load/validate contract: structural rules fail
//! typed, digests are canonical, and cross-file resolution is
//! deliberately left to the handoff projection.

use system::Migration;

fn parse(yaml: &str) -> Result<Migration, error::Error> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("migration.yaml");
    std::fs::write(&path, yaml).expect("write migration.yaml");
    Migration::load(&path)
}

/// The stable discriminant Display leads with (`{code}: {detail}`).
fn code(err: &error::Error) -> String {
    err.to_string().split(':').next().unwrap_or_default().to_string()
}

const MINIMAL: &str = "version: 1\ndispositions:\n  - id: keep-orders\n    treatment: preserve\n    \
                       applies-to: [orders]\n    reason: must survive\nwaves:\n  - id: wave-1\n    \
                       outcome: replatform orders\n    architecture:\n      before: as-is\n      \
                       after: target\n    dispositions: [keep-orders]\n";

#[test]
fn minimal_loads() {
    let migration = parse(MINIMAL).expect("valid plan");
    assert_eq!(migration.waves.len(), 1);
    assert_eq!(migration.wave("wave-1").expect("wave").outcome, "replatform orders");
    assert!(migration.disposition("keep-orders").is_some());
}

#[test]
fn missing_is_typed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let err = Migration::load(&dir.path().join("migration.yaml")).expect_err("absent file");
    assert_eq!(code(&err), "system-migration-missing");
}

#[test]
fn digest_ignores_format() {
    let compact = parse(MINIMAL).expect("valid plan");
    let block_style =
        parse(&MINIMAL.replace("applies-to: [orders]", "applies-to:\n      - orders"))
            .expect("valid plan");
    assert_eq!(
        compact.digest().expect("digest"),
        block_style.digest().expect("digest"),
        "canonical digest ignores on-disk formatting"
    );
}

mod invalid {
    use super::{code, parse};

    #[test]
    fn unknown_field() {
        let err = parse("version: 1\nnovel: true\n").expect_err("unknown field");
        assert!(matches!(err, error::Error::YamlDe(_)), "{err:?}");
    }

    #[test]
    fn duplicate_wave() {
        let yaml = "version: 1\nwaves:\n  - id: w\n    outcome: a\n    architecture:\n      \
                    before: as-is\n      after: target\n  - id: w\n    outcome: b\n    \
                    architecture:\n      before: as-is\n      after: target\n";
        assert_eq!(code(&parse(yaml).expect_err("duplicate")), "system-migration-invalid");
    }

    #[test]
    fn unresolved_predecessor() {
        let yaml = "version: 1\nwaves:\n  - id: w\n    outcome: a\n    architecture:\n      \
                    before: as-is\n      after: target\n    predecessors: [ghost]\n";
        assert_eq!(code(&parse(yaml).expect_err("unresolved")), "system-migration-invalid");
    }

    #[test]
    fn self_predecessor() {
        let yaml = "version: 1\nwaves:\n  - id: w\n    outcome: a\n    architecture:\n      \
                    before: as-is\n      after: target\n    predecessors: [w]\n";
        assert_eq!(code(&parse(yaml).expect_err("self")), "system-migration-invalid");
    }

    #[test]
    fn unresolved_disposition() {
        let yaml = "version: 1\nwaves:\n  - id: w\n    outcome: a\n    architecture:\n      \
                    before: as-is\n      after: target\n    dispositions: [ghost]\n";
        assert_eq!(code(&parse(yaml).expect_err("unresolved")), "system-migration-invalid");
    }

    #[test]
    fn duplicate_item_id() {
        let yaml = "version: 1\nwaves:\n  - id: w\n    outcome: a\n    architecture:\n      \
                    before: as-is\n      after: target\n    gaps:\n      - id: g\n        detail: \
                    one\n      - id: g\n        detail: two\n";
        assert_eq!(code(&parse(yaml).expect_err("duplicate")), "system-migration-invalid");
    }

    #[test]
    fn unresolved_mapping_target() {
        let yaml = "version: 1\nwaves:\n  - id: w\n    outcome: a\n    architecture:\n      \
                    before: as-is\n      after: target\n    delivery-mappings:\n      - source: \
                    src\n        lead: l\n        target: ghost\n";
        assert_eq!(code(&parse(yaml).expect_err("unresolved")), "system-migration-invalid");
    }

    #[test]
    fn empty_reason() {
        let yaml = "version: 1\ndispositions:\n  - id: d\n    treatment: change\n    reason: \
                    \"\"\n";
        assert_eq!(code(&parse(yaml).expect_err("empty reason")), "system-migration-invalid");
    }
}
