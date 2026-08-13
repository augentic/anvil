//! Decomposition substrate: DTO, validators, compiler, projector, retention.

use std::collections::BTreeMap;
use std::path::PathBuf;

use project::adapter::catalog::Pin;
use project::config::Layout;
use project::name::SliceName;
use project::plan::decomposition::{
    BoundProfile, Decomposition, Kind, Node, Scope, VERSION, compile, contraction, findings,
    matches_plan, slices,
};
use project::plan::{
    Plan, Projections, SliceSourceBinding, SourceBinding, TargetBinding, contributing_leads,
};
use project::profile::Profile;
use project::snapshot::SnapshotId;

fn code(err: &impl std::fmt::Display) -> String {
    err.to_string()
}

fn codes(tree: &Decomposition) -> Vec<String> {
    findings(tree).into_iter().filter_map(|item| item.rule_id).collect()
}

fn has(tree: &Decomposition, rule: &str) -> bool {
    codes(tree).iter().any(|item| item == rule)
}

fn digest() -> SnapshotId {
    SnapshotId::from_digest(&"ab".repeat(32))
}

fn profile() -> BoundProfile {
    BoundProfile::capture(&Profile::frontier_v1()).expect("profile")
}

fn profiles(targets: &[&str]) -> BTreeMap<String, BoundProfile> {
    targets.iter().map(|target| ((*target).to_string(), profile())).collect()
}

fn leaf(id: &str, target: &str, parent: &str, lead: &str, glob: &str) -> Node {
    let mut node = Node::leaf(target, id);
    node.parent = Some(parent.into());
    node.sources = vec![Scope::new("code", lead)];
    node.ownership = vec![glob.into()];
    node.acceptance = Some("acceptance".into());
    node
}

fn split(children: &[&str], sources: Vec<Scope>) -> Node {
    let mut node = Node::split(children.iter().map(|id| (*id).to_string()).collect::<Vec<_>>());
    node.sources = sources;
    node
}

fn tree(root: &str, nodes: BTreeMap<String, Node>, targets: &[&str]) -> Decomposition {
    Decomposition {
        version: VERSION,
        leads_digest: digest(),
        profiles: profiles(targets),
        root: root.into(),
        nodes,
    }
}

/// Root → one leaf. Unary splits skip scope-reduction.
fn degenerate() -> Decomposition {
    let mut root = split(&["orders-api"], vec![Scope::new("code", "orders-api")]);
    root.targets = vec!["orders".into()];
    tree(
        "root",
        BTreeMap::from([
            ("root".into(), root),
            (
                "orders-api".into(),
                leaf("orders-api", "orders", "root", "orders-api", "src/orders/**"),
            ),
        ]),
        &["orders"],
    )
}

/// Three domain levels, two targets, disjoint ownership.
fn multi_level() -> Decomposition {
    let docs = Scope::new("docs", "conventions");
    let api_lead = Scope::new("code", "payments-api");
    let web_lead = Scope::new("code", "payments-web");
    let mut root = split(&["payments"], vec![docs.clone(), api_lead.clone(), web_lead.clone()]);
    root.targets = vec!["api".into(), "web".into()];
    let mut payments = split(
        &["payments-api", "payments-web"],
        vec![docs.clone(), api_lead.clone(), web_lead.clone()],
    );
    payments.parent = Some("root".into());
    payments.targets = vec!["api".into(), "web".into()];
    let mut api = leaf("payments-api", "api", "payments", "payments-api", "src/api/**");
    api.sources = vec![docs.clone(), api_lead];
    let mut web = leaf("payments-web", "web", "payments", "payments-web", "src/web/**");
    web.sources = vec![docs, web_lead];
    tree(
        "root",
        BTreeMap::from([
            ("root".into(), root),
            ("payments".into(), payments),
            ("payments-api".into(), api),
            ("payments-web".into(), web),
        ]),
        &["api", "web"],
    )
}

fn plan_from(tree: &Decomposition) -> Plan {
    let mut plan = Plan::named("demo");
    plan.decomposition_digest = Some(tree.digest().expect("digest"));
    plan.entries = slices(tree).expect("project");
    for entry in &plan.entries {
        plan.targets.entry(entry.target.clone()).or_insert_with(|| {
            TargetBinding::new(Pin::emery("mock", semver::Version::new(0, 0, 0)), ".", digest())
        });
        for binding in &entry.sources {
            plan.sources.entry(binding.source().to_string()).or_insert_with(|| {
                SourceBinding::located(
                    Pin::emery("mock", semver::Version::new(0, 0, 0)),
                    ".",
                    digest(),
                )
            });
        }
    }
    plan
}

mod parse {
    use super::*;

    #[test]
    fn golden() {
        let actual = degenerate().canonical_yaml().expect("yaml");
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("answers").join("decomposition.yaml");
        if std::env::var_os("REGENERATE_GOLDENS").is_some() {
            std::fs::create_dir_all(path.parent().expect("answers")).expect("create");
            std::fs::write(&path, &actual).expect("regenerate");
        }
        let expected = std::fs::read_to_string(&path).expect("read golden");
        assert_eq!(actual, expected, "golden mismatch: {}", path.display());
        let again = Decomposition::parse(&expected).expect("parse");
        assert_eq!(again.digest().expect("digest"), degenerate().digest().expect("digest"));
    }

    #[test]
    fn unknown_field() {
        let mut yaml = degenerate().canonical_yaml().expect("yaml");
        yaml.push_str("extra: true\n");
        let err = Decomposition::parse(&yaml).expect_err("unknown");
        assert!(code(&err).contains("decomposition-malformed"), "{err}");
    }

    #[test]
    fn version() {
        let mut tree = degenerate();
        tree.version = VERSION + 1;
        let err = Decomposition::parse(&tree.canonical_yaml().expect("yaml")).expect_err("ver");
        assert!(code(&err).contains("decomposition-version"), "{err}");
    }

    #[test]
    fn digest_omits_nested() {
        let bound = profile();
        assert_eq!(bound.digest, bound.body().digest().expect("body"));
        assert!(!bound.body().canonical_yaml().expect("yaml").contains("digest:"));
    }
}

mod validate {
    use super::*;

    #[test]
    fn degenerate_ok() {
        degenerate().check().expect("valid");
        assert!(codes(&degenerate()).is_empty());
    }

    #[test]
    fn three_level_ok() {
        multi_level().check().expect("valid");
    }

    #[test]
    fn uncovered() {
        let mut tree = degenerate();
        tree.nodes.get_mut("orders-api").expect("leaf").sources.clear();
        assert!(has(&tree, "decomposition-lead-uncovered"));
    }

    #[test]
    fn dropped() {
        let docs = Scope::new("docs", "conventions");
        let mut root = split(
            &["a", "b", "c"],
            vec![
                docs.clone(),
                Scope::new("code", "a"),
                Scope::new("code", "b"),
                Scope::new("code", "c"),
            ],
        );
        root.targets = vec!["t".into()];
        let mut a = leaf("a", "t", "root", "a", "src/a/**");
        a.sources.push(docs.clone());
        let mut b = leaf("b", "t", "root", "b", "src/b/**");
        b.sources.push(docs);
        let c = leaf("c", "t", "root", "c", "src/c/**");
        let tree = tree(
            "root",
            BTreeMap::from([
                ("root".into(), root),
                ("a".into(), a),
                ("b".into(), b),
                ("c".into(), c),
            ]),
            &["t"],
        );
        assert!(has(&tree, "decomposition-lead-dropped"));
    }

    #[test]
    fn target_escape() {
        let mut tree = degenerate();
        tree.nodes.get_mut("root").expect("root").targets = vec!["orders".into()];
        tree.nodes.get_mut("orders-api").expect("leaf").target = Some("other".into());
        tree.profiles.insert("other".into(), profile());
        assert!(has(&tree, "decomposition-target-escape"));
    }

    #[test]
    fn non_reducing() {
        let mut root = split(&["a", "b"], vec![Scope::new("code", "shared")]);
        root.targets = vec!["t".into()];
        root.ownership = vec!["src/**".into()];
        let mut a = leaf("a", "t", "root", "shared", "src/a/**");
        a.sources = vec![Scope::new("code", "shared")];
        let mut b = leaf("b", "t", "root", "shared", "src/b/**");
        b.sources = vec![Scope::new("code", "shared")];
        let tree = tree(
            "root",
            BTreeMap::from([("root".into(), root), ("a".into(), a), ("b".into(), b)]),
            &["t"],
        );
        assert!(has(&tree, "decomposition-non-reducing"));
    }

    #[test]
    fn source_dup() {
        let mut tree = degenerate();
        let leaf = tree.nodes.get_mut("orders-api").expect("leaf");
        leaf.sources.push(Scope::new("code", "other"));
        assert!(has(&tree, "decomposition-source-dup"));
    }

    #[test]
    fn overlap_blocks() {
        let mut root = split(&["a", "b"], vec![Scope::new("code", "a"), Scope::new("code", "b")]);
        root.targets = vec!["t".into()];
        let a = leaf("a", "t", "root", "a", "src/shared/**");
        let b = leaf("b", "t", "root", "b", "src/shared/**");
        let tree = tree(
            "root",
            BTreeMap::from([("root".into(), root), ("a".into(), a), ("b".into(), b)]),
            &["t"],
        );
        assert!(has(&tree, "decomposition-overlap"));
    }

    #[test]
    fn overlap_order() {
        let mut root = split(&["a", "b"], vec![Scope::new("code", "a"), Scope::new("code", "b")]);
        root.targets = vec!["t".into()];
        let mut a = leaf("a", "t", "root", "a", "src/shared/**");
        a.depends_on = vec!["b".into()];
        let b = leaf("b", "t", "root", "b", "src/shared/**");
        let tree = tree(
            "root",
            BTreeMap::from([("root".into(), root), ("a".into(), a), ("b".into(), b)]),
            &["t"],
        );
        assert!(!has(&tree, "decomposition-overlap"));
        tree.check().expect("ordered overlap is explicit");
    }

    #[test]
    fn overlap_fan_in() {
        let mut root = split(
            &["a", "b", "c"],
            vec![Scope::new("code", "a"), Scope::new("code", "b"), Scope::new("code", "c")],
        );
        root.targets = vec!["t".into()];
        let a = leaf("a", "t", "root", "a", "src/shared/**");
        let b = leaf("b", "t", "root", "b", "src/shared/**");
        let mut c = leaf("c", "t", "root", "c", "src/c/**");
        c.depends_on = vec!["a".into(), "b".into()];
        let tree = tree(
            "root",
            BTreeMap::from([
                ("root".into(), root),
                ("a".into(), a),
                ("b".into(), b),
                ("c".into(), c),
            ]),
            &["t"],
        );
        assert!(!has(&tree, "decomposition-overlap"));
        tree.check().expect("fan-in");
    }

    #[test]
    fn leaf_incomplete() {
        let mut tree = degenerate();
        tree.nodes.get_mut("orders-api").expect("leaf").acceptance = None;
        assert!(has(&tree, "decomposition-leaf-incomplete"));
    }

    #[test]
    fn depth_cap() {
        let mut nodes = BTreeMap::new();
        let mut parent: Option<String> = None;
        for index in 1..=9 {
            let id = format!("n{index}");
            let mut node = if index == 9 {
                leaf(&id, "t", parent.as_deref().expect("parent"), "n9", "src/n9/**")
            } else {
                let child = format!("n{}", index + 1);
                let mut node = split(&[&child], vec![Scope::new("code", "n9")]);
                node.parent = parent.clone();
                node.targets = vec!["t".into()];
                node
            };
            if index == 1 {
                node.parent = None;
            }
            nodes.insert(id.clone(), node);
            parent = Some(id);
        }
        let tree = tree("n1", nodes, &["t"]);
        assert!(has(&tree, "decomposition-depth"));
    }

    #[test]
    fn nodes_cap() {
        let mut children = Vec::new();
        let mut nodes = BTreeMap::new();
        let mut sources = Vec::new();
        for index in 0..MAX_EXTRA {
            let id = format!("n{index}");
            children.push(id.clone());
            sources.push(Scope::new("code", &id));
            nodes.insert(id.clone(), leaf(&id, "t", "root", &id, &format!("src/{id}/**")));
        }
        let mut root = split(&children.iter().map(String::as_str).collect::<Vec<_>>(), sources);
        root.targets = vec!["t".into()];
        nodes.insert("root".into(), root);
        let tree = tree("root", nodes, &["t"]);
        assert!(has(&tree, "decomposition-nodes"));
    }

    #[test]
    fn leaf_cycle() {
        let mut root = split(&["a", "b"], vec![Scope::new("code", "a"), Scope::new("code", "b")]);
        root.targets = vec!["t".into()];
        let mut a = leaf("a", "t", "root", "a", "src/a/**");
        a.depends_on = vec!["b".into()];
        let mut b = leaf("b", "t", "root", "b", "src/b/**");
        b.depends_on = vec!["a".into()];
        let tree = tree(
            "root",
            BTreeMap::from([("root".into(), root), ("a".into(), a), ("b".into(), b)]),
            &["t"],
        );
        assert!(has(&tree, "decomposition-leaf-cycle"));
    }

    #[test]
    fn target_cycle() {
        // Leaf-acyclic: a→b and c→d. Contraction: t1→t2 and t2→t1.
        let mut root = split(
            &["a", "b", "c", "d"],
            vec![
                Scope::new("code", "a"),
                Scope::new("code", "b"),
                Scope::new("code", "c"),
                Scope::new("code", "d"),
            ],
        );
        root.targets = vec!["t1".into(), "t2".into()];
        let mut a = leaf("a", "t1", "root", "a", "src/a/**");
        a.depends_on = vec!["b".into()];
        let b = leaf("b", "t2", "root", "b", "src/b/**");
        let mut c = leaf("c", "t2", "root", "c", "src/c/**");
        c.depends_on = vec!["d".into()];
        let d = leaf("d", "t1", "root", "d", "src/d/**");
        let tree = tree(
            "root",
            BTreeMap::from([
                ("root".into(), root),
                ("a".into(), a),
                ("b".into(), b),
                ("c".into(), c),
                ("d".into(), d),
            ]),
            &["t1", "t2"],
        );
        assert!(!has(&tree, "decomposition-leaf-cycle"));
        assert!(has(&tree, "publication-target-cycle"));
        let entries = slices(&tree).expect("acyclic leaves still project");
        assert!(!contraction(&entries).is_empty());
    }

    #[test]
    fn kind_mismatch() {
        let mut tree = degenerate();
        tree.nodes.get_mut("orders-api").expect("leaf").kind = Some(Kind::Split);
        assert!(has(&tree, "decomposition-kind"));
    }

    #[test]
    fn profile_digest() {
        let mut tree = degenerate();
        tree.profiles.get_mut("orders").expect("profile").digest = digest();
        assert!(has(&tree, "decomposition-profile-digest"));
    }
}

/// Root plus 64 children = 65 nodes, one over [`project::plan::decomposition::MAX_NODES`].
const MAX_EXTRA: usize = 64;

mod compile_edges {
    use super::*;

    #[test]
    fn domain_to_domain() {
        let mut root =
            split(&["left", "right"], vec![Scope::new("code", "l"), Scope::new("code", "r")]);
        root.targets = vec!["t".into()];
        let mut left = split(&["l"], vec![Scope::new("code", "l")]);
        left.parent = Some("root".into());
        left.depends_on = vec!["right".into()];
        left.targets = vec!["t".into()];
        let mut right = split(&["r"], vec![Scope::new("code", "r")]);
        right.parent = Some("root".into());
        right.targets = vec!["t".into()];
        let l = leaf("l", "t", "left", "l", "src/l/**");
        let r = leaf("r", "t", "right", "r", "src/r/**");
        let tree = tree(
            "root",
            BTreeMap::from([
                ("root".into(), root),
                ("left".into(), left),
                ("right".into(), right),
                ("l".into(), l),
                ("r".into(), r),
            ]),
            &["t"],
        );
        tree.check().expect("valid");
        let edges = compile(&tree).expect("compile");
        let l_deps = edges.get("l").expect("l");
        assert_eq!(l_deps.iter().map(SliceName::as_str).collect::<Vec<_>>(), ["r"]);
        assert!(edges.get("r").expect("r").is_empty());
    }
}

mod projector {
    use super::*;

    #[test]
    fn degenerate_slices() {
        let tree = degenerate();
        let entries = slices(&tree).expect("project");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name.as_str(), "orders-api");
        assert_eq!(entries[0].target, "orders");
        assert_eq!(entries[0].sources, vec![SliceSourceBinding::structured("code", "orders-api")]);
        matches_plan(&tree, &plan_from(&tree)).expect("exact");
    }

    #[test]
    fn byte_stable() {
        let tree = multi_level();
        let first = slices(&tree).expect("a");
        let second = slices(&tree).expect("b");
        assert_eq!(first, second);
        let yaml = artifacts::atomic::serialise_yaml(&first).expect("yaml");
        assert_eq!(yaml, artifacts::atomic::serialise_yaml(&second).expect("yaml"));
    }

    #[test]
    fn three_level_targets() {
        let entries = slices(&multi_level()).expect("project");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name.as_str(), "payments-api");
        assert_eq!(entries[0].target, "api");
        assert_eq!(entries[1].name.as_str(), "payments-web");
        assert_eq!(entries[1].target, "web");
        assert!(entries.iter().all(|entry| !entry.target.is_empty()));
    }

    #[test]
    fn drift() {
        let tree = degenerate();
        let mut plan = plan_from(&tree);
        plan.entries[0].target = "other".into();
        let err = matches_plan(&tree, &plan).expect_err("drift");
        assert!(code(&err).contains("decomposition-plan-drift"), "{err}");
    }
}

mod projection_pin {
    use super::*;

    fn inventory() -> Vec<artifacts::leads::Lead> {
        vec![artifacts::leads::Lead::new("orders-api", "code", "Orders API")]
    }

    #[test]
    fn empty_ancestry() {
        // Single-node tree: the root is the leaf, so ancestry stays
        // canonical-empty and the digest matches a tree-less compute.
        let mut node = leaf("orders-api", "orders", "unused", "orders-api", "src/orders/**");
        node.parent = None;
        let tree = tree("orders-api", BTreeMap::from([("orders-api".into(), node)]), &["orders"]);
        tree.check().expect("valid");
        let plan = plan_from(&tree);
        let contributing = contributing_leads(&plan.entries[0], &inventory()).expect("leads");
        let without =
            Projections::compute(&plan, &plan.entries[0], &contributing, None).expect("a");
        let with =
            Projections::compute_with(&plan, &plan.entries[0], &contributing, None, Some(&tree))
                .expect("b");
        assert_eq!(without.decomposition, with.decomposition);
    }

    #[test]
    fn ancestry_moves() {
        let tree = degenerate();
        let plan = plan_from(&tree);
        let contributing = contributing_leads(&plan.entries[0], &inventory()).expect("leads");
        let without =
            Projections::compute(&plan, &plan.entries[0], &contributing, None).expect("a");
        let with =
            Projections::compute_with(&plan, &plan.entries[0], &contributing, None, Some(&tree))
                .expect("b");
        assert_ne!(without.decomposition, with.decomposition);
    }
}

mod retain {
    use super::*;

    #[test]
    fn first_copy() {
        let root = tempfile::tempdir().expect("tempdir");
        let layout = Layout::new(root.path());
        std::fs::create_dir_all(layout.change_root()).expect("change home");
        let tree = degenerate();
        tree.save(&layout.decomposition_path()).expect("write");
        let digest = project::plan::retain_decomposition(layout).expect("retain");
        let dest = layout.decomp_revision_path(&digest);
        assert!(dest.is_file(), "{}", dest.display());
        assert_eq!(
            std::fs::read(layout.decomposition_path()).expect("current"),
            std::fs::read(&dest).expect("retained")
        );
        let again = project::plan::retain_decomposition(layout).expect("idempotent");
        assert_eq!(again, digest);
    }

    #[test]
    fn edit_keeps_prior() {
        let root = tempfile::tempdir().expect("tempdir");
        let layout = Layout::new(root.path());
        std::fs::create_dir_all(layout.change_root()).expect("change home");
        let first = degenerate();
        first.save(&layout.decomposition_path()).expect("write");
        let first_digest = project::plan::retain_decomposition(layout).expect("retain");
        let first_bytes = std::fs::read(layout.decomp_revision_path(&first_digest)).expect("first");

        let mut edited = first;
        edited.nodes.get_mut("orders-api").expect("leaf").acceptance = Some("changed".into());
        edited.save(&layout.decomposition_path()).expect("rewrite");
        let second = project::plan::retain_decomposition(layout).expect("retain edited");
        assert_ne!(first_digest, second);
        assert_eq!(
            std::fs::read(layout.decomp_revision_path(&first_digest)).expect("prior"),
            first_bytes
        );
    }
}
