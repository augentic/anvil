//! Adapter catalog matching, source keys, and exact-pin rules.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mock::definition::Spec;
use project::adapter::catalog::{self, Catalog, Hint, Pin, Profile, Recognition, Row, Source};
use project::binding::{Location, Origin};

fn catalog() -> Catalog {
    Catalog::first_party()
}

fn write(dir: &Path, rel: &str, bytes: &[u8]) {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("mkdir");
    }
    std::fs::write(path, bytes).expect("write");
}

fn code(err: &impl std::fmt::Display) -> String {
    err.to_string()
}

fn loc(raw: &str) -> Origin {
    Origin::Location(Location::parse(raw, None).expect("locator"))
}

fn row(origin: Origin, pin: &str) -> Row {
    Row {
        origin,
        pin: Pin::parse(pin).expect("pin"),
    }
}

mod pins {
    use super::*;

    #[test]
    fn package_and_sugar() {
        let pin = Pin::parse("emery:typescript@1.2.0").expect("pin");
        assert_eq!(pin.wire(), "emery:typescript@1.2.0");
        assert_eq!(Pin::parse("typescript@1.2.0").expect("sugar").wire(), pin.wire());
    }

    #[test]
    fn bare_and_component() {
        let err = Pin::parse("typescript").expect_err("bare");
        assert!(code(&err).contains("adapter-unversioned"), "{err}");

        let err = Pin::parse("./adapter.wasm").expect_err("component");
        assert!(code(&err).contains("adapter-unversioned"), "{err}");
    }

    #[test]
    fn handoff_pin_reuse() {
        let spec = Spec::degenerate("ship the reviewed intent");
        let scope = &spec.scopes[0];
        let pin = catalog::select(
            &catalog(),
            Hint::Pin(scope.adapter.as_deref().expect("pinned")),
            &Origin::Value(scope.value.clone().expect("intent value")),
        )
        .expect("reuse");
        assert_eq!(pin.wire(), "emery:intent@1.0.0");

        let spec = Spec::multi_target();
        let scope = &spec.scopes[0];
        let pin = catalog::select(
            &catalog(),
            Hint::Pin(scope.adapter.as_deref().expect("pinned")),
            &loc("https://github.com/example/orders@main"),
        )
        .expect("reuse typescript");
        assert_eq!(pin.wire(), "emery:typescript@1.0.0");
        assert_ne!(pin.version, semver::Version::new(0, 12, 0));
    }
}

mod fingerprint {
    use super::*;

    fn tree(files: &[(&str, &[u8])]) -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("src");
        std::fs::create_dir_all(&root).expect("mkdir");
        for (rel, bytes) in files {
            write(&root, rel, bytes);
        }
        (tmp, root)
    }

    #[test]
    fn one_match_pins() {
        let (_tmp, root) = tree(&[("package.json", b"{}"), ("src/app.ts", b"export {}")]);
        let pin = catalog::select(&catalog(), Hint::Open(&root), &loc("./src")).expect("ts");
        assert_eq!(pin.wire(), "emery:typescript@0.12.0");

        let (_tmp, root) = tree(&[("guide.md", b"# Docs")]);
        let pin = catalog::select(&catalog(), Hint::Open(&root), &loc("./docs")).expect("docs");
        assert_eq!(pin.name, "documentation");

        let (_tmp, root) = tree(&[("home.png", b"\x89PNG")]);
        let pin = catalog::select(&catalog(), Hint::Open(&root), &loc("./shots")).expect("shots");
        assert_eq!(pin.name, "screenshots");

        let (_tmp, root) = tree(&[("tests/data/replays/list/ok.json", b"{}")]);
        let pin = catalog::select(&catalog(), Hint::Open(&root), &loc("./captures")).expect("cap");
        assert_eq!(pin.name, "captures");
    }

    #[test]
    fn no_match_and_ambiguous() {
        let (_tmp, root) = tree(&[("notes.txt", b"plain")]);
        let err = catalog::select(&catalog(), Hint::Open(&root), &loc("./misc")).expect_err("none");
        assert!(code(&err).contains("source-adapter-no-match"), "{err}");

        let (_tmp, root) = tree(&[("package.json", b"{}"), ("README.md", b"# hi")]);
        let err = catalog::select(&catalog(), Hint::Open(&root), &loc("./repo")).expect_err("many");
        assert!(code(&err).contains("source-adapter-ambiguous"), "{err}");
        assert!(code(&err).contains("documentation"), "{err}");
        assert!(code(&err).contains("typescript"), "{err}");
    }

    #[test]
    fn intent_not_probed() {
        let (_tmp, root) = tree(&[("brief.txt", b"do the thing")]);
        let err =
            catalog::select(&catalog(), Hint::Open(&root), &loc("./brief.txt")).expect_err("txt");
        assert!(code(&err).contains("source-adapter-no-match"), "{err}");
    }

    #[test]
    fn open_value_no_match() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let err =
            catalog::select(&catalog(), Hint::Open(tmp.path()), &Origin::Value("ship it".into()))
                .expect_err("value");
        assert!(code(&err).contains("source-adapter-no-match"), "{err}");
    }
}

mod intent {
    use super::*;

    #[test]
    fn locator_refused() {
        let origin = loc("https://github.com/acme/brief@main");
        let err =
            catalog::select(&catalog(), Hint::Pin("emery:intent@1.0.0"), &origin).expect_err("pin");
        assert!(code(&err).contains("source-intent-locator"), "{err}");

        let err = catalog::assign(&[row(origin, "emery:intent@1.0.0")], &BTreeMap::new())
            .expect_err("key");
        assert!(code(&err).contains("source-intent-locator"), "{err}");
    }

    #[test]
    fn second_is_duplicate() {
        let rows = [
            row(Origin::Value("one".into()), "emery:intent@1.0.0"),
            row(Origin::Value("two".into()), "emery:intent@1.0.0"),
        ];
        let err = catalog::assign(&rows, &BTreeMap::new()).expect_err("dup");
        assert!(code(&err).contains("source-adapter-duplicate"), "{err}");
    }
}

mod keys {
    use super::*;

    #[test]
    fn basename_value_and_intent() {
        let rows = [
            row(
                loc("https://github.com/acme/orders@0123456789abcdef0123456789abcdef01234567"),
                "emery:typescript@1.0.0",
            ),
            row(Origin::Value("notes".into()), "emery:documentation@1.0.0"),
            row(Origin::Value("ship it".into()), "emery:intent@1.0.0"),
        ];
        let keys = catalog::assign(&rows, &BTreeMap::new()).expect("keys");
        assert_eq!(keys, ["orders", "documentation", "intent"]);
    }

    #[test]
    fn path_selector_basename() {
        let origin = Origin::Location(
            Location::parse("https://github.com/acme/monorepo@main", Some("packages/payments"))
                .expect("loc"),
        );
        let keys = catalog::assign(&[row(origin, "emery:typescript@1.0.0")], &BTreeMap::new())
            .expect("key");
        assert_eq!(keys, ["payments"]);
    }

    #[test]
    fn collision_stable() {
        let a = loc("https://github.com/acme/orders@aaa");
        let b = loc("https://github.com/other/orders@bbb");
        let rows =
            [row(a.clone(), "emery:typescript@1.0.0"), row(b.clone(), "emery:typescript@1.0.0")];
        let keys = catalog::assign(&rows, &BTreeMap::new()).expect("keys");
        assert_eq!(keys.iter().filter(|key| *key == "orders").count(), 1);
        assert!(keys.iter().any(|key| key.starts_with("orders-") && key.len() == 15));

        let swapped = [row(b, "emery:typescript@1.0.0"), row(a, "emery:typescript@1.0.0")];
        let swapped_keys = catalog::assign(&swapped, &BTreeMap::new()).expect("swapped");
        let first = catalog::identity(&rows[0]);
        let map: BTreeMap<_, _> =
            rows.iter().map(catalog::identity).zip(keys.iter().cloned()).collect();
        let swapped_map: BTreeMap<_, _> =
            swapped.iter().map(catalog::identity).zip(swapped_keys.iter().cloned()).collect();
        assert_eq!(map, swapped_map);
        assert_eq!(map.get(&first).map(String::as_str), Some("orders"));
    }

    #[test]
    fn prior_survives() {
        let kept = loc("https://github.com/other/orders@bbb");
        let incoming = loc("https://github.com/acme/orders@aaa");
        let prior_row = row(kept.clone(), "emery:typescript@1.0.0");
        let prior = BTreeMap::from([(catalog::identity(&prior_row), "orders".into())]);
        let keys = catalog::assign(
            &[row(incoming, "emery:typescript@1.0.0"), row(kept, "emery:typescript@1.0.0")],
            &prior,
        )
        .expect("keys");
        assert_eq!(keys[1], "orders");
        assert_ne!(keys[0], "orders");
        assert!(keys[0].starts_with("orders-"));
    }

    #[test]
    fn duplicate_locator() {
        let origin = loc("https://github.com/acme/orders@main");
        let err = catalog::assign(
            &[
                row(origin.clone(), "emery:typescript@1.0.0"),
                row(origin, "emery:documentation@1.0.0"),
            ],
            &BTreeMap::new(),
        )
        .expect_err("dup");
        assert!(code(&err).contains("source-adapter-duplicate"), "{err}");
    }

    #[test]
    fn intent_not_stolen() {
        let origin = Origin::Location(Location::parse("./intent.md", None).expect("path"));
        let keys = catalog::assign(&[row(origin, "emery:documentation@1.0.0")], &BTreeMap::new())
            .expect("key");
        assert_ne!(keys[0], "intent");
        assert!(keys[0].starts_with("source-"), "{}", keys[0]);
    }
}

mod inventory {
    use super::*;

    #[test]
    fn first_party_shape() {
        let catalog = Catalog::first_party();
        let sources: Vec<&str> = catalog.sources.iter().map(|row| row.pin.name.as_str()).collect();
        assert_eq!(sources, ["intent", "typescript", "documentation", "screenshots", "captures"]);
        assert!(matches!(catalog.sources[0].recognition, Recognition::Explicit));
        let targets: Vec<&str> = catalog.targets.iter().map(|row| row.pin.name.as_str()).collect();
        assert_eq!(targets, ["omnia", "vectis", "contracts"]);
        assert!(catalog.targets[1].platforms.as_ref().is_some_and(|p| p.required));
    }

    #[test]
    fn intent_not_profile() {
        let err = Catalog::new(
            vec![Source {
                pin: Pin::emery("intent", semver::Version::new(1, 0, 0)),
                recognition: Recognition::Profile(Profile::default()),
            }],
            Vec::new(),
        )
        .expect_err("intent");
        assert!(code(&err).contains("adapter-catalog-invalid"), "{err}");
    }
}
