//! Canonical handoff projection and `system review`: content-addressed
//! writes, fail-closed reference resolution, digest-exact current
//! selection, the stale-review refusal, and the `system.wave.reviewed`
//! fact with read-only re-entry.

use std::path::Path;

use system::{Coverage, Layout, Migration, Model, Scope, handoff, review, status};

const SCOPE: &str = "version: 1\nid: acme-estate\ndecision: consolidate the order stack\n";

const COVERAGE: &str = "version: 1\ncandidates:\n  - key: orders-code\n    location: ./orders\n    \
                        adapter: mock\n    disposition: included\n    reason: primary\n    \
                        observed-cid: sha256:1111111111111111111111111111111111111111111111111111111111111111\n";

const MODEL: &str = "version: 1\nidentities: []\nas-is:\n  elements:\n    - id: orders\n      \
                     kind: service\n      status: inferred\n  relationships: []\ntarget:\n  \
                     elements:\n    - id: orders\n      kind: service\n      status: inferred\n  \
                     relationships: []\n";

const MIGRATION: &str = "version: 1\ndispositions:\n  - id: keep-orders\n    treatment: \
                         preserve\n    applies-to: [orders]\n    reason: must survive\nwaves:\n  \
                         - id: wave-1\n    outcome: replatform orders\n    architecture:\n      \
                         before: as-is\n      after: target\n    dispositions: [keep-orders]\n    \
                         affected-elements: [orders]\n    evidence-scopes:\n      - source: \
                         orders-code\n        lead: greeting\n    gaps:\n      - id: g1\n        \
                         detail: retention unknown\n";

const EVIDENCE: &str = "lead: greeting\nauthority: documentation\nclaims:\n  - kind: \
                        requirement\n    id: orders.api\n";

/// Author a reviewed-shape definition home: declared files, model with
/// `target`, one-wave plan, and one persisted Evidence document.
fn author(home: &Path) {
    std::fs::write(home.join("scope.yaml"), SCOPE).expect("scope.yaml");
    std::fs::write(home.join("coverage.yaml"), COVERAGE).expect("coverage.yaml");
    std::fs::write(home.join("system.yaml"), MODEL).expect("system.yaml");
    std::fs::write(home.join("migration.yaml"), MIGRATION).expect("migration.yaml");
    let evidence = home.join("evidence/orders-code");
    std::fs::create_dir_all(&evidence).expect("evidence dir");
    std::fs::write(evidence.join("greeting.yaml"), EVIDENCE).expect("evidence doc");
}

/// Project and persist wave-1's handoff against the live files.
fn project(home: &Path) -> handoff::Projected {
    let layout = Layout::new(home);
    let scope = Scope::load(&layout.scope_path()).expect("scope");
    let coverage = Coverage::load(&layout.coverage_path()).expect("coverage");
    let model = Model::load(&layout.system_path()).expect("model");
    let migration = Migration::load(&layout.migration_path()).expect("migration");
    let wave = migration.wave("wave-1").expect("wave").clone();
    let projected = handoff::project(&layout, &scope, &coverage, &model, &migration, &[], &wave)
        .expect("projection resolves");
    handoff::write(&layout, &projected).expect("persist");
    projected
}

#[test]
fn round_trip() {
    let home = tempfile::tempdir().expect("tempdir");
    author(home.path());
    let projected = project(home.path());

    // The file lives at its content address and loads back verified.
    let path = home.path().join(format!("handoffs/{}.yaml", projected.digest.digest()));
    assert!(path.is_file(), "handoff persisted at its digest");
    let loaded = handoff::load(&path).expect("verified load");
    assert_eq!(loaded, projected);
    let wave = &loaded.handoff.wave;
    assert_eq!(wave.id, "wave-1");
    assert_eq!(wave.dispositions.len(), 1);
    assert_eq!(wave.gaps.len(), 1);
    assert_eq!(wave.affected_elements, ["orders"]);
    let scope = &wave.evidence_scopes[0];
    assert_eq!(scope.adapter, "mock", "adapter identity copied verbatim");
    assert!(scope.observed_cid.as_str().starts_with("sha256:"));

    // Reprojection is byte-idempotent: same digest, same single file.
    let again = project(home.path());
    assert_eq!(again.digest, projected.digest);
}

#[test]
fn edited_handoff_fails_load() {
    let home = tempfile::tempdir().expect("tempdir");
    author(home.path());
    let projected = project(home.path());
    let path = home.path().join(format!("handoffs/{}.yaml", projected.digest.digest()));
    let text = std::fs::read_to_string(&path).expect("read");
    std::fs::write(&path, text.replace("replatform orders", "rewrite orders")).expect("tamper");
    let err = handoff::load(&path).expect_err("content drift");
    assert!(err.to_string().starts_with("system-handoff-corrupt"), "{err}");
}

mod unresolved {
    use super::{Coverage, Layout, MIGRATION, Migration, Model, Scope, author, handoff};

    /// Project after swapping one input mutation in.
    fn project_with(mutate: impl FnOnce(&std::path::Path)) -> error::Error {
        let home = tempfile::tempdir().expect("tempdir");
        author(home.path());
        mutate(home.path());
        let layout = Layout::new(home.path());
        let scope = Scope::load(&layout.scope_path()).expect("scope");
        let coverage = Coverage::load(&layout.coverage_path()).expect("coverage");
        let model = Model::load(&layout.system_path()).expect("model");
        let migration = Migration::load(&layout.migration_path()).expect("migration");
        let wave = migration.wave("wave-1").expect("wave").clone();
        handoff::project(&layout, &scope, &coverage, &model, &migration, &[], &wave)
            .expect_err("projection fails closed")
    }

    #[test]
    fn missing_state() {
        let err = project_with(|home| {
            std::fs::write(
                home.join("migration.yaml"),
                MIGRATION.replace("after: target", "after: transition-ghost"),
            )
            .expect("rewrite");
        });
        assert!(err.to_string().starts_with("system-handoff-unresolved"), "{err}");
    }

    #[test]
    fn missing_element() {
        let err = project_with(|home| {
            std::fs::write(
                home.join("migration.yaml"),
                MIGRATION.replace("affected-elements: [orders]", "affected-elements: [ghost]"),
            )
            .expect("rewrite");
        });
        assert!(err.to_string().starts_with("system-handoff-unresolved"), "{err}");
    }

    #[test]
    fn missing_decision_record() {
        let err = project_with(|home| {
            std::fs::write(
                home.join("migration.yaml"),
                MIGRATION.replace(
                    "dispositions: [keep-orders]\n",
                    "dispositions: \
                                   [keep-orders]\n    decisions: [ghost]\n",
                ),
            )
            .expect("rewrite");
        });
        assert!(err.to_string().starts_with("system-handoff-unresolved"), "{err}");
    }

    #[test]
    fn evidence_scope_without_document() {
        let err = project_with(|home| {
            std::fs::write(
                home.join("migration.yaml"),
                MIGRATION.replace("lead: greeting", "lead: ghost"),
            )
            .expect("rewrite");
        });
        assert!(err.to_string().starts_with("system-handoff-unresolved"), "{err}");
    }
}

mod review_verb {
    use super::{Layout, MIGRATION, author, project, review, status};

    const fn now() -> jiff::Timestamp {
        jiff::Timestamp::UNIX_EPOCH
    }

    #[test]
    fn records_then_noops() {
        let home = tempfile::tempdir().expect("tempdir");
        author(home.path());
        let projected = project(home.path());
        let layout = Layout::new(home.path());

        // First review appends the fact; the bare digest form works.
        let outcome = review::review(&layout, "wave-1", projected.digest.digest(), now())
            .expect("review records");
        assert!(outcome.recorded);
        assert_eq!(outcome.handoff_digest, projected.digest.as_str());
        let events = std::fs::read_dir(home.path().join("events"))
            .expect("events dir")
            .map(|entry| entry.expect("entry").path())
            .collect::<Vec<_>>();
        assert_eq!(events.len(), 1, "one writer log");
        let log = std::fs::read_to_string(&events[0]).expect("log");
        assert!(log.contains("system.wave.reviewed"), "{log}");
        assert_eq!(log.lines().count(), 1);

        // Same-handoff re-entry is a read-only no-op (`sha256:` form).
        let again =
            review::review(&layout, "wave-1", projected.digest.as_str(), now()).expect("re-entry");
        assert!(!again.recorded);
        let log = std::fs::read_to_string(&events[0]).expect("log");
        assert_eq!(log.lines().count(), 1, "no second fact");

        // Status projects the reviewed standing.
        let projected_status = status::project(&layout).expect("status");
        assert!(matches!(projected_status.next, status::NextAction::Reviewed));
    }

    #[test]
    fn stale_supplied_digest_refused() {
        let home = tempfile::tempdir().expect("tempdir");
        author(home.path());
        let _projected = project(home.path());
        let layout = Layout::new(home.path());
        let err =
            review::review(&layout, "wave-1", &"0".repeat(64), now()).expect_err("stale digest");
        assert!(err.to_string().starts_with("system-review-stale"), "{err}");
        assert!(!home.path().join("events").exists(), "no fact on refusal");
    }

    #[test]
    fn moved_definition_refused() {
        // The handoff predates an operator edit to migration.yaml: no
        // current handoff matches, so review demands a re-plan.
        let home = tempfile::tempdir().expect("tempdir");
        author(home.path());
        let projected = project(home.path());
        std::fs::write(
            home.path().join("migration.yaml"),
            MIGRATION.replace("replatform orders", "rewrite orders"),
        )
        .expect("operator edit");
        let layout = Layout::new(home.path());
        let err = review::review(&layout, "wave-1", projected.digest.digest(), now())
            .expect_err("no current handoff");
        assert!(err.to_string().starts_with("system-review-handoff-stale"), "{err}");
    }

    #[test]
    fn ambiguity_fails_closed() {
        // A second content-valid handoff for the same wave whose
        // covered digests also match: selection refuses to choose.
        let home = tempfile::tempdir().expect("tempdir");
        author(home.path());
        let projected = project(home.path());
        let mut forged = projected.handoff.clone();
        forged.wave.outcome = "a diverging projection of the same wave".to_string();
        // Serialise through the same canonical encoder the engine uses
        // so the forged file passes the content-address check.
        let text = artifacts::atomic::serialise_yaml(&forged).expect("yaml");
        let digest = diagnostics::digest::sha256_hex(text.as_bytes());
        std::fs::write(home.path().join(format!("handoffs/{digest}.yaml")), &text)
            .expect("forged handoff");
        let layout = Layout::new(home.path());
        let err = review::review(&layout, "wave-1", projected.digest.digest(), now())
            .expect_err("two current handoffs");
        assert!(err.to_string().starts_with("system-review-ambiguous"), "{err}");
    }
}
