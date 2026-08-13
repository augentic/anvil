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
    fn evidence_doc_missing() {
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

/// The RFC fixture estate (AC6–AC8): a target architecture that moves
/// `orders-db` ownership out of the legacy monolith by decision, one
/// boundary-change wave carrying the full D9 field set, and one
/// evidence-collection wave that yields no delivery mapping.
mod full_estate {
    use std::path::Path;

    use system::migration::Treatment;
    use system::{Coverage, Layout, Migration, Model, Scope, handoff};

    const MODEL: &str = "\
version: 1
identities: []
as-is:
  elements:
    - id: legacy-monolith
      kind: service
      status: inferred
    - id: orders-db
      kind: data-store
      status: inferred
      attributes:
        ownership: legacy-monolith is the single writer
        consistency: order and line items commit in one transaction
        retention: unknown - no policy evidenced
    - id: nightly-reconcile
      kind: scheduled-job
      status: inferred
      attributes:
        temporal: settles the ledger once per day; reruns are idempotent
    - id: billing-vendor
      kind: external-actor
      status: inferred
      context-only: true
  relationships:
    - id: monolith-owns-orders-db
      kind: ownership
      from: legacy-monolith
      to: orders-db
      status: decided
      decision: split-orders-db
transition-coexist:
  elements:
    - id: legacy-monolith
      kind: service
      status: inferred
    - id: orders-service
      kind: service
      status: decided
      decision: split-orders-db
    - id: orders-db
      kind: data-store
      status: inferred
    - id: nightly-reconcile
      kind: scheduled-job
      status: inferred
    - id: billing-vendor
      kind: external-actor
      status: inferred
      context-only: true
  relationships: []
target:
  elements:
    - id: orders-service
      kind: service
      status: inferred
    - id: orders-db
      kind: data-store
      status: inferred
  relationships:
    - id: orders-service-owns-orders-db
      kind: ownership
      from: orders-service
      to: orders-db
      status: decided
      decision: split-orders-db
";

    const MIGRATION: &str = "\
version: 1
dispositions:
  - id: split-orders
    treatment: change
    applies-to: [monolith-owns-orders-db]
    reason: ownership moves to the orders service by split-orders-db
waves:
  - id: wave-split
    outcome: move orders ownership out of the monolith
    architecture:
      before: as-is
      after: transition-coexist
    preconditions:
      - id: schema-freeze
        detail: orders schema freeze agreed with the vendor
    affected-elements: [legacy-monolith, orders-db]
    touched-elements: [orders-service]
    context-elements: [billing-vendor]
    dispositions: [split-orders]
    evidence-scopes:
      - source: orders-code
        lead: greeting
    targets:
      - id: orders-service-repo
        locator: https://github.com/acme/orders-service
        adapter: omnia
    delivery-mappings:
      - source: orders-code
        lead: greeting
        target: orders-service-repo
    state-movements:
      - id: backfill
        detail: dual-write then backfill historical orders rows
    coexistence:
      - id: dual-write
        detail: monolith and service dual-write for the window
    cutover:
      - id: flip-reads
        detail: flip the read path once drift holds at zero
    rollback:
      - id: revert-reads
        detail: point reads back at the monolith
    operational-readiness:
      - id: drift-dashboard
        detail: row-level drift dashboard live before cutover
    acceptance:
      - id: parity
        detail: seven days of zero drift
    verification:
      - id: replay
        detail: replay captured traffic against both paths
    conservation:
      - id: retention
        detail: order history survives the move
    gaps:
      - id: retention-window
        detail: legal retention window unconfirmed
    assumptions:
      - id: vendor-contract
        detail: the billing contract permits the new caller
    decisions: [split-orders-db]
  - id: wave-observe
    outcome: collect evidence on the nightly reconcile job
    architecture:
      before: transition-coexist
      after: transition-coexist
    predecessors: [wave-split]
    context-elements: [nightly-reconcile]
    evidence-scopes:
      - source: orders-code
        lead: greeting
";

    const DECISION: &str = "\
version: 1
id: split-orders-db
applies-to: [monolith-owns-orders-db]
context: order state lives inside the legacy monolith today
decision: the orders service owns orders-db once wave-split cuts over
consequences: the monolith becomes a coexistence-window reader
";

    /// Author the estate home: rich model, two-wave plan, and one
    /// decision record beside the shared scope/coverage/Evidence.
    pub fn author(home: &Path) {
        std::fs::write(home.join("scope.yaml"), super::SCOPE).expect("scope.yaml");
        std::fs::write(home.join("coverage.yaml"), super::COVERAGE).expect("coverage.yaml");
        std::fs::write(home.join("system.yaml"), MODEL).expect("system.yaml");
        std::fs::write(home.join("migration.yaml"), MIGRATION).expect("migration.yaml");
        let evidence = home.join("evidence/orders-code");
        std::fs::create_dir_all(&evidence).expect("evidence dir");
        std::fs::write(evidence.join("greeting.yaml"), super::EVIDENCE).expect("evidence doc");
        std::fs::create_dir_all(home.join("decisions")).expect("decisions dir");
        std::fs::write(home.join("decisions/split-orders-db.yaml"), DECISION).expect("decision");
    }

    /// Project and persist one wave's handoff against the live files.
    pub fn project(home: &Path, wave: &str) -> handoff::Projected {
        let layout = Layout::new(home);
        let scope = Scope::load(&layout.scope_path()).expect("scope");
        let coverage = Coverage::load(&layout.coverage_path()).expect("coverage");
        let model = Model::load(&layout.system_path()).expect("model");
        let migration = Migration::load(&layout.migration_path()).expect("migration");
        let decisions = system::decision::load_all(&layout.decisions_dir()).expect("decisions");
        let wave = migration.wave(wave).expect("wave").clone();
        let projected =
            handoff::project(&layout, &scope, &coverage, &model, &migration, &decisions, &wave)
                .expect("projection resolves");
        handoff::write(&layout, &projected).expect("persist");
        projected
    }

    #[test]
    fn boundary_change_wave() {
        // AC6: the reviewed target moves a legacy state boundary under
        // a `change` disposition (never `preserve`) and the plan
        // records the responsible decision, transition state, data
        // movement, reconciliation, cutover, and rollback. AC8: every
        // reference kind resolves and round-trips content-addressed.
        let home = tempfile::tempdir().expect("tempdir");
        author(home.path());
        let projected = project(home.path(), "wave-split");
        let layout = Layout::new(home.path());

        let migration = Migration::load(&layout.migration_path()).expect("migration");
        let disposition = migration.disposition("split-orders").expect("disposition");
        assert_eq!(disposition.treatment, Treatment::Change, "boundary is not preserved");

        // AC5: the stateful element records ownership, transaction /
        // consistency, and temporal invariants on the attribute map.
        let model = Model::load(&layout.system_path()).expect("model");
        let store = model
            .as_is
            .elements
            .iter()
            .find(|element| element.id == "orders-db")
            .expect("stateful element");
        for key in ["ownership", "consistency", "retention"] {
            assert!(store.attributes.contains_key(key), "orders-db records `{key}`");
        }

        let wave = &projected.handoff.wave;
        assert_eq!(wave.architecture.before.id, "as-is");
        assert_eq!(wave.architecture.after.id, "transition-coexist", "transition state named");
        assert_eq!(wave.decisions[0].id, "split-orders-db", "responsible decision referenced");
        for refs in [&wave.state_movements, &wave.coexistence, &wave.cutover, &wave.rollback] {
            assert_eq!(refs.len(), 1);
        }
        assert_eq!(wave.targets[0].adapter, "omnia", "bare adapter name copied verbatim");
        assert_eq!(wave.delivery_mappings.len(), 1);
        assert_eq!(wave.context_elements, ["billing-vendor"], "context-only system carried");
        assert!(!wave.preconditions.is_empty());
        assert!(!wave.operational_readiness.is_empty());
        assert!(!wave.acceptance.is_empty());
        assert!(!wave.verification.is_empty());
        assert!(!wave.conservation.is_empty());
        assert!(!wave.assumptions.is_empty());

        let path = home.path().join(format!("handoffs/{}.yaml", projected.digest.digest()));
        let loaded = handoff::load(&path).expect("verified load");
        assert_eq!(loaded, projected);
    }

    #[test]
    fn evidence_collection_wave() {
        // AC7: an evidence-collection wave over context-only systems
        // projects a canonical handoff with no delivery target and no
        // delivery mapping — nothing for RFC-88 to slice.
        let home = tempfile::tempdir().expect("tempdir");
        author(home.path());
        let projected = project(home.path(), "wave-observe");
        let wave = &projected.handoff.wave;
        assert!(wave.targets.is_empty(), "no delivery target");
        assert!(wave.delivery_mappings.is_empty(), "no delivery mapping");
        assert_eq!(wave.context_elements, ["nightly-reconcile"]);
        assert_eq!(wave.dependencies[0].id, "wave-split", "predecessor resolved");
        assert!(!wave.evidence_scopes.is_empty(), "evidence still selected");
    }
}

/// Review validates the definition it grants authority over (D1/D4):
/// a stale overlay or an edited projection refuses before selection.
mod review_gates {
    use system::{Layout, Model, architecture, review};

    const fn now() -> jiff::Timestamp {
        jiff::Timestamp::UNIX_EPOCH
    }

    fn review_err(mutate: impl FnOnce(&std::path::Path)) -> error::Error {
        let home = tempfile::tempdir().expect("tempdir");
        super::full_estate::author(home.path());
        let projected = super::full_estate::project(home.path(), "wave-split");
        mutate(home.path());
        let layout = Layout::new(home.path());
        review::review(&layout, "wave-split", projected.digest.digest(), now())
            .expect_err("review refuses")
    }

    #[test]
    fn unfolded_decision() {
        // A decision added after the last survey has not been stamped
        // onto `as-is`: review demands a re-survey, never restamps.
        let err = review_err(|home| {
            std::fs::write(
                home.join("decisions/keep-billing.yaml"),
                "version: 1\nid: keep-billing\napplies-to: [billing-vendor]\ncontext: c\n\
                 decision: d\nconsequences: q\n",
            )
            .expect("new decision");
        });
        assert!(err.to_string().starts_with("system-overlay-stale"), "{err}");
    }

    #[test]
    fn vanished_decision() {
        // `as-is` is stamped by a decision record that no longer
        // exists: the overlay is stale, not silently un-decided.
        let err = review_err(|home| {
            std::fs::remove_file(home.join("decisions/split-orders-db.yaml"))
                .expect("remove decision");
        });
        assert!(err.to_string().starts_with("system-overlay-stale"), "{err}");
    }

    #[test]
    fn stale_projection() {
        // A projection whose digest stamp no longer matches the live
        // state cannot be reviewed into authority.
        let err = review_err(|home| {
            let layout = Layout::new(home);
            let model = Model::load(&layout.system_path()).expect("model");
            architecture::project(&layout, "as-is", &model.as_is).expect("project");
            let path = layout.state_doc_path("as-is");
            let text = std::fs::read_to_string(&path).expect("read");
            let live = model.as_is.digest().expect("digest");
            std::fs::write(&path, text.replace(live.digest(), &"0".repeat(64)))
                .expect("tamper the stamp");
        });
        assert!(err.to_string().starts_with("system-projection-stale"), "{err}");
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
    fn stale_digest_refused() {
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
    fn full_estate_review_loop() {
        // AC6/AC9 end to end over the rich estate: review the
        // boundary-change wave's exact handoff and observe the fact.
        let home = tempfile::tempdir().expect("tempdir");
        super::full_estate::author(home.path());
        let projected = super::full_estate::project(home.path(), "wave-split");
        let layout = Layout::new(home.path());
        let outcome = review::review(&layout, "wave-split", projected.digest.digest(), now())
            .expect("review records");
        assert!(outcome.recorded);
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
