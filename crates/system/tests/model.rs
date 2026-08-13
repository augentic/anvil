//! `system.yaml` and `decisions/` — typed loads, named-state
//! validation, and the D4 overlay persist tail (identity reapply,
//! explicit gaps, decision stamping).

use std::collections::BTreeMap;
use std::path::Path;

use error::Error;
use system::decision;
use system::model::overlay::persist_as_is;
use system::model::{
    ClaimRef, Element, ElementKind, Model, Relationship, RelationshipKind, State, Status,
};

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("create dir");
    std::fs::write(path, text).expect("write");
}

fn element(id: &str, status: Status, claims: &[(&str, &str)]) -> Element {
    Element {
        id: id.to_string(),
        kind: ElementKind::Service,
        status,
        claims: claims
            .iter()
            .map(|(source, id)| ClaimRef {
                source: (*source).to_string(),
                id: (*id).to_string(),
            })
            .collect(),
        decision: None,
        context_only: false,
        attributes: BTreeMap::new(),
    }
}

const MODEL: &str = "\
version: 1
identities:
  - id: orders
    aliases: [legacy-order-svc]
    supersedes: [order-monolith]
as-is:
  elements:
    - id: orders
      kind: service
      status: evidenced
      claims: [{ source: orders-code, id: orders.api }]
    - id: orders-store
      kind: data-store
      status: evidenced
      claims: [{ source: orders-code, id: orders.store }]
  relationships:
    - id: orders-reads-store
      kind: read
      from: orders
      to: orders-store
      status: evidenced
      claims: [{ source: orders-code, id: orders.store-read }]
target: { elements: [], relationships: [] }
transition-strangler:
  elements: []
  relationships: []
";

mod model_file {
    use super::*;

    #[test]
    fn loads() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("system.yaml");
        write(&path, MODEL);
        let model = Model::load(&path).expect("valid model loads");
        assert_eq!(model.identities[0].aliases, ["legacy-order-svc"]);
        assert_eq!(model.as_is.elements.len(), 2);
        assert!(model.state("target").expect("target state").elements.is_empty());
        assert!(model.state("transition-strangler").is_some());
        assert!(model.state("transition-absent").is_none());
    }

    #[test]
    fn missing_fails_closed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = Model::load(&dir.path().join("system.yaml")).expect_err("absent file");
        assert!(
            matches!(
                &err,
                Error::Diag {
                    code: "system-model-missing",
                    ..
                }
            ),
            "{err}"
        );
    }

    #[test]
    fn unknown_key_rejected() {
        // The flattened transition map catches stray top-level keys;
        // the grammar check turns them into a typed rejection.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("system.yaml");
        write(
            &path,
            "version: 1\nas-is: { elements: [], relationships: [] }\nstray: { elements: [], relationships: [] }\n",
        );
        let err = Model::load(&path).expect_err("stray key");
        assert!(err.to_string().contains("stray"), "{err}");
    }

    #[test]
    fn endpoint_resolves() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("system.yaml");
        write(
            &path,
            "version: 1\nas-is:\n  elements: []\n  relationships:\n    - id: r1\n      kind: read\n      from: a\n      to: b\n      status: unknown\n",
        );
        let err = Model::load(&path).expect_err("dangling endpoint");
        assert!(err.to_string().contains("endpoint"), "{err}");
    }

    #[test]
    fn status_coherence() {
        // `evidenced` without claims is incoherent.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("system.yaml");
        write(
            &path,
            "version: 1\nas-is:\n  elements:\n    - id: a\n      kind: service\n      status: evidenced\n  relationships: []\n",
        );
        let err = Model::load(&path).expect_err("claimless evidenced");
        assert!(err.to_string().contains("claim"), "{err}");
    }

    #[test]
    fn state_digests() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("system.yaml");
        write(&path, MODEL);
        let model = Model::load(&path).expect("model");
        let as_is = model.as_is.digest().expect("digest");
        assert_ne!(as_is, model.state("target").expect("target").digest().expect("digest"));
        // Same content re-encoded digests identically.
        assert_eq!(as_is, model.as_is.digest().expect("digest"));
    }
}

mod decisions {
    use super::*;

    #[test]
    fn absent_dir_is_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let loaded = decision::load_all(&dir.path().join("decisions")).expect("empty set");
        assert!(loaded.is_empty());
    }

    #[test]
    fn loads_sorted() {
        let dir = tempfile::tempdir().expect("tempdir");
        let decisions = dir.path().join("decisions");
        write(
            &decisions.join("b-later.yaml"),
            "version: 1\nid: b-later\ncontext: c\ndecision: d\nconsequences: q\n",
        );
        write(
            &decisions.join("a-first.yaml"),
            "version: 1\nid: a-first\napplies-to: [orders]\ncontext: c\ndecision: d\nconsequences: q\n",
        );
        let loaded = decision::load_all(&decisions).expect("two records");
        assert_eq!(loaded[0].id, "a-first");
        assert_eq!(loaded[0].applies_to, ["orders"]);
        assert_eq!(loaded[1].id, "b-later");
    }

    #[test]
    fn stem_mismatch_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let decisions = dir.path().join("decisions");
        write(
            &decisions.join("other.yaml"),
            "version: 1\nid: not-other\ncontext: c\ndecision: d\nconsequences: q\n",
        );
        let err = decision::load_all(&decisions).expect_err("stem mismatch");
        assert!(err.to_string().contains("stem"), "{err}");
    }

    #[test]
    fn one_decision_per_id() {
        let dir = tempfile::tempdir().expect("tempdir");
        let decisions = dir.path().join("decisions");
        for id in ["first", "second"] {
            write(
                &decisions.join(format!("{id}.yaml")),
                &format!(
                    "version: 1\nid: {id}\napplies-to: [orders]\ncontext: c\ndecision: d\nconsequences: q\n"
                ),
            );
        }
        let err = decision::load_all(&decisions).expect_err("shared applies-to");
        assert!(err.to_string().contains("orders"), "{err}");
    }
}

mod overlay {
    use super::*;

    #[test]
    fn first_creation() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("system.yaml");
        let state = State {
            elements: vec![element("orders", Status::Evidenced, &[("orders-code", "orders.api")])],
            relationships: Vec::new(),
        };
        let model = persist_as_is(&path, state, &[]).expect("first persist creates the file");
        assert!(model.identities.is_empty(), "first creation mints empty identities");
        assert_eq!(model.as_is.elements[0].id, "orders");
        assert_eq!(Model::load(&path).expect("reload"), model);
    }

    #[test]
    fn identity_reapplied() {
        // An aliased name folds onto its identity: claims union, and
        // relationship endpoints follow the rename.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("system.yaml");
        write(
            &path,
            "version: 1\nidentities:\n  - id: orders\n    aliases: [legacy-order-svc]\nas-is: { elements: [], relationships: [] }\n",
        );
        let mut store = element("orders-store", Status::Evidenced, &[("db", "schema.orders")]);
        store.kind = ElementKind::DataStore;
        let state = State {
            elements: vec![
                element("orders", Status::Evidenced, &[("orders-code", "orders.api")]),
                element("legacy-order-svc", Status::Evidenced, &[("docs", "legacy.api")]),
                store,
            ],
            relationships: vec![Relationship {
                id: "legacy-reads".to_string(),
                kind: RelationshipKind::Read,
                from: "legacy-order-svc".to_string(),
                to: "orders-store".to_string(),
                status: Status::Evidenced,
                claims: vec![ClaimRef {
                    source: "orders-code".to_string(),
                    id: "orders.store-read".to_string(),
                }],
                decision: None,
                context_only: false,
                attributes: BTreeMap::new(),
            }],
        };
        let model = persist_as_is(&path, state, &[]).expect("persist");
        let orders = &model.as_is.elements[0];
        assert_eq!(orders.id, "orders");
        assert_eq!(orders.claims.len(), 2, "merged claims union: {:?}", orders.claims);
        assert_eq!(model.as_is.elements.len(), 2, "alias folded, store kept");
        assert_eq!(model.as_is.relationships[0].from, "orders", "endpoint renamed");
    }

    #[test]
    fn vanished_id_is_a_gap() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("system.yaml");
        write(
            &path,
            "version: 1\nidentities:\n  - id: billing\nas-is: { elements: [], relationships: [] }\n",
        );
        let state = State {
            elements: vec![element("orders", Status::Evidenced, &[("orders-code", "orders.api")])],
            relationships: Vec::new(),
        };
        let model = persist_as_is(&path, state, &[]).expect("persist");
        let gap = model
            .as_is
            .elements
            .iter()
            .find(|element| element.id == "billing")
            .expect("explicit gap element");
        assert_eq!(gap.status, Status::Unknown);
        assert!(gap.attributes.contains_key("gap"), "{:?}", gap.attributes);
    }

    #[test]
    fn vanished_link_keeps_shape() {
        // A declared relationship id the new survey missed stays a
        // relationship gap while its endpoints survive — never a
        // placeholder element with an invented kind.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("system.yaml");
        write(
            &path,
            "version: 1\nidentities:\n  - id: owns-orders\nas-is:\n  elements:\n    - id: \
             orders\n      kind: service\n      status: inferred\n    - id: orders-db\n      \
             kind: data-store\n      status: inferred\n  relationships:\n    - id: owns-orders\n      \
             kind: ownership\n      from: orders\n      to: orders-db\n      status: inferred\n",
        );
        let state = State {
            elements: vec![
                element("orders", Status::Evidenced, &[("orders-code", "orders.api")]),
                element("orders-db", Status::Inferred, &[]),
            ],
            relationships: Vec::new(),
        };
        let model = persist_as_is(&path, state, &[]).expect("persist");
        let gap = model
            .as_is
            .relationships
            .iter()
            .find(|relationship| relationship.id == "owns-orders")
            .expect("relationship-shaped gap");
        assert_eq!(gap.status, Status::Unknown);
        assert!(gap.attributes.contains_key("gap"), "{:?}", gap.attributes);
    }

    #[test]
    fn decision_stamped() {
        let dir = tempfile::tempdir().expect("tempdir");
        let home = dir.path();
        write(
            &home.join("decisions/order-owner.yaml"),
            "version: 1\nid: order-owner\napplies-to: [orders]\ncontext: c\ndecision: d\nconsequences: q\n",
        );
        let decisions = decision::load_all(&home.join("decisions")).expect("decisions");
        let state = State {
            elements: vec![element("orders", Status::Evidenced, &[("orders-code", "orders.api")])],
            relationships: Vec::new(),
        };
        let model = persist_as_is(&home.join("system.yaml"), state, &decisions).expect("persist");
        let orders = &model.as_is.elements[0];
        assert_eq!(orders.status, Status::Decided);
        assert_eq!(orders.decision.as_deref(), Some("order-owner"));
        assert!(!orders.claims.is_empty(), "provenance survives the stamp");
    }

    #[test]
    fn decided_input_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("system.yaml");
        let mut decided = element("orders", Status::Decided, &[]);
        decided.decision = Some("order-owner".to_string());
        let state = State {
            elements: vec![decided],
            relationships: Vec::new(),
        };
        let err = persist_as_is(&path, state, &[]).expect_err("correlation cannot decide");
        assert!(err.to_string().contains("decided"), "{err}");
    }

    #[test]
    fn declared_states_kept() {
        // Persist replaces `as-is` only; `target` and `transition-*`
        // survive byte-for-byte re-encodes.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("system.yaml");
        write(&path, MODEL);
        let state = State {
            elements: vec![element("orders", Status::Evidenced, &[("orders-code", "orders.api")])],
            relationships: Vec::new(),
        };
        let model = persist_as_is(&path, state, &[]).expect("persist");
        assert!(model.target.is_some(), "target kept");
        assert!(model.transitions.contains_key("transition-strangler"), "transition kept");
        assert_eq!(model.as_is.elements.len(), 1, "as-is replaced");
    }
}
