//! The publication projection kernel (RFC-95 D7): Kahn ranks over the
//! contracted DAG, `merged-at` ordering including the equal-timestamp
//! failure, and external records validating against the wire type.

use project::plan::publication::{self, PublicationState, Record, Verification, project, ranks};
use project::plan::{Entry, Plan, TargetBinding};
use project::seam::{Forge, ForgeError, PrState, PullRequest};
use project::snapshot::SnapshotId;

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

fn entry(name: &str, target: &str, deps: &[&str]) -> Entry {
    serde_json::from_value(serde_json::json!({
        "name": name,
        "target": target,
        "depends-on": deps,
    }))
    .expect("entry parses")
}

mod kahn {
    use super::*;

    #[test]
    fn chain() {
        let entries = [entry("a1", "a", &[]), entry("b1", "b", &["a1"]), entry("c1", "c", &["b1"])];
        let ranks = ranks(&entries);
        assert_eq!(ranks.get("a"), Some(&1));
        assert_eq!(ranks.get("b"), Some(&2));
        assert_eq!(ranks.get("c"), Some(&3));
    }

    #[test]
    fn unrelated_member_unranked() {
        let entries = [entry("a1", "a", &[]), entry("b1", "b", &["a1"]), entry("z1", "z", &[])];
        let ranks = ranks(&entries);
        assert_eq!(ranks.len(), 2, "z carries no rank: {ranks:?}");
        assert!(!ranks.contains_key("z"));
    }

    #[test]
    fn sorted_ready_set() {
        // Both b and c become ready after a; the sorted ready set
        // ranks b before c regardless of entry order.
        let entries = [entry("c1", "c", &["a1"]), entry("a1", "a", &[]), entry("b1", "b", &["a1"])];
        let ranks = ranks(&entries);
        assert_eq!(ranks.get("a"), Some(&1));
        assert_eq!(ranks.get("b"), Some(&2));
        assert_eq!(ranks.get("c"), Some(&3));
    }

    #[test]
    fn same_target_no_edge() {
        let entries = [entry("a1", "a", &[]), entry("a2", "a", &["a1"])];
        assert!(ranks(&entries).is_empty());
    }
}

/// `plan validate`'s doctor sweep repeats the contraction check: an
/// acyclic leaf graph contracting to a target cycle is a finding.
#[test]
fn doctor_contraction() {
    let mut plan = Plan::named("demo");
    plan.entries =
        vec![entry("a1", "a", &[]), entry("b1", "b", &["a1"]), entry("a2", "a", &["b1"])];
    let findings = project::plan::doctor::doctor(&plan, None);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule_id.as_deref() == Some("publication-target-cycle")),
        "{findings:?}"
    );
}

/// A canned forge: answers every find with the same rows, tagging the
/// URL with the repository so members stay distinguishable.
struct Canned(Vec<(String, PullRequest)>);

impl Forge for Canned {
    async fn find(
        &self, repository: String, _branch: String,
    ) -> Result<Vec<PullRequest>, ForgeError> {
        Ok(self
            .0
            .iter()
            .filter(|(member, _)| repository.ends_with(&format!("/{member}")))
            .map(|(_, pull)| pull.clone())
            .collect())
    }
}

fn merged(member: &str, merged_at: &str, digest: &str) -> (String, PullRequest) {
    (
        member.to_string(),
        PullRequest {
            url: format!("https://github.com/o/{member}/pull/1"),
            body: format!("Emery-Change: demo\nEmery-Change-Digest: {digest}\n"),
            state: PrState::Merged,
            base: "main".to_string(),
            merged_at: Some(merged_at.to_string()),
            merge_commit: Some("8e43c".to_string()),
        },
    )
}

/// A two-member plan (`a → b` through leaf dependencies) written to a
/// temp change home, so `Plan::file_digest` resolves.
fn two_member_plan(root: &std::path::Path) -> (Plan, String) {
    let mut plan = Plan::named("demo");
    for member in ["a", "b"] {
        plan.targets.insert(
            member.to_string(),
            TargetBinding::new(
                project::adapter::catalog::Pin::parse("emery:mock@0.1.0").expect("pin"),
                format!("https://github.com/o/{member}@{SHA}"),
                SnapshotId::parse(&format!("sha256:{}", "1".repeat(64))).expect("cid"),
            ),
        );
    }
    plan.entries = vec![entry("a1", "a", &[]), entry("b1", "b", &["a1"])];
    let path = root.join(".emery/change/plan.yaml");
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    plan.save(&path).expect("save plan");
    let digest = Plan::file_digest(project::config::Layout::new(root)).expect("digest");
    (plan, digest)
}

async fn project_two(rows: Vec<(String, PullRequest)>) -> Record {
    let dir = tempfile::tempdir().expect("tempdir");
    let (plan, _) = two_member_plan(dir.path());
    let layout = project::config::Layout::new(dir.path());
    project(&Canned(rows), &plan, layout, &[]).await.expect("projection").record
}

#[tokio::test]
async fn in_order_verifies() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (plan, digest) = two_member_plan(dir.path());
    let layout = project::config::Layout::new(dir.path());
    let rows = vec![
        merged("a", "2026-08-15T01:00:00Z", &digest),
        merged("b", "2026-08-15T02:00:00Z", &digest),
    ];
    let record = project(&Canned(rows), &plan, layout, &[]).await.expect("projection").record;
    assert_eq!(record.verification, Verification::Verified, "{record:?}");
    assert!(record.failures.is_empty());
    assert_eq!(record.members[0].order, Some(1));
    assert_eq!(record.members[1].order, Some(2));
    assert_eq!(record.members[0].publication, PublicationState::Merged);
}

#[tokio::test]
async fn out_of_order_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (plan, digest) = two_member_plan(dir.path());
    let layout = project::config::Layout::new(dir.path());
    let rows = vec![
        merged("a", "2026-08-15T02:00:00Z", &digest),
        merged("b", "2026-08-15T01:00:00Z", &digest),
    ];
    let record = project(&Canned(rows), &plan, layout, &[]).await.expect("projection").record;
    assert_eq!(record.verification, Verification::Unverified);
    assert_eq!(record.failures.len(), 1);
    assert_eq!(record.failures[0].member, "b");
    assert!(record.failures[0].reason.contains("landed before its dependency `a`"));
}

#[tokio::test]
async fn equal_merged_at_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (plan, digest) = two_member_plan(dir.path());
    let layout = project::config::Layout::new(dir.path());
    let rows = vec![
        merged("a", "2026-08-15T01:00:00Z", &digest),
        merged("b", "2026-08-15T01:00:00Z", &digest),
    ];
    let record = project(&Canned(rows), &plan, layout, &[]).await.expect("projection").record;
    assert_eq!(record.verification, Verification::Unverified);
    assert!(record.failures[0].reason.contains("ties"), "{:?}", record.failures);
}

#[tokio::test]
async fn byte_stable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (plan, digest) = two_member_plan(dir.path());
    let layout = project::config::Layout::new(dir.path());
    let rows = vec![
        merged("a", "2026-08-15T01:00:00Z", &digest),
        merged("b", "2026-08-15T02:00:00Z", &digest),
    ];
    let first = project(&Canned(rows.clone()), &plan, layout, &[]).await.expect("projection");
    let second = project(&Canned(rows), &plan, layout, &[]).await.expect("projection");
    assert_eq!(
        serde_json::to_vec(&first.record).expect("json"),
        serde_json::to_vec(&second.record).expect("json"),
        "unchanged inputs produce byte-stable output"
    );
}

#[tokio::test]
async fn one_open_member_pending() {
    let digest_free = vec![(
        "a".to_string(),
        PullRequest {
            url: "https://github.com/o/a/pull/1".to_string(),
            body: "no trailers here".to_string(),
            state: PrState::Open,
            base: "main".to_string(),
            merged_at: None,
            merge_commit: None,
        },
    )];
    let record = project_two(digest_free).await;
    assert_eq!(record.verification, Verification::Pending);
    // Trailerless pull requests never match: both members project
    // `unpublished`.
    assert!(record.members.iter().all(|m| m.publication == PublicationState::Unpublished));
    assert_eq!(record.failures.len(), 2);
    assert!(record.failures.iter().all(|f| f.reason == "unpublished"));
}

mod generation_scope {
    use project::journal::{Event, EventKind};

    use super::*;

    fn materialized(plan: &Plan) -> Event {
        Event::new(
            jiff::Timestamp::UNIX_EPOCH,
            EventKind::PublicationMaterialized {
                plan_name: plan.name.clone(),
                plan_digest: "sha256:old".to_string(),
                target: "a".to_string(),
                parent_revision: SHA.to_string(),
                cid: SnapshotId::parse(&format!("sha256:{}", "1".repeat(64))).expect("cid"),
                worktree_path: "/tmp/worktree".to_string(),
                branch: "change/demo".to_string(),
            },
        )
    }

    fn reconciled(plan: &Plan) -> Event {
        Event::new(
            jiff::Timestamp::UNIX_EPOCH,
            EventKind::PlanReconcileCompleted {
                plan_name: plan.name.clone(),
                slice_count: 1,
                slice_names: Vec::new(),
            },
        )
    }

    /// A materialized fact in the current generation locks its target.
    #[test]
    fn current_fact_locks() {
        let plan = Plan::named("demo");
        let events = [reconciled(&plan), materialized(&plan)];
        let locked = publication::locked_targets(&plan, &events);
        assert!(locked.contains("a"), "{locked:?}");
    }

    /// The change events directory outlives archive and plan names
    /// recur: a fact from before the latest authoring never locks the
    /// re-authored plan.
    #[test]
    fn prior_fact_ignored() {
        let plan = Plan::named("demo");
        let events = [materialized(&plan), reconciled(&plan)];
        assert!(publication::locked_targets(&plan, &events).is_empty());
    }
}

mod external_record {
    use super::*;

    /// The RFC-95 worked example is an external record: it must parse
    /// against the wire type unchanged.
    const WORKED_EXAMPLE: &str = r#"{
      "change": "checkout-v2",
      "members": [
        {
          "target": "payments-api",
          "repository": "github.com/example/payments-api",
          "merge-commit": "8e43c",
          "branch": "change/checkout-v2",
          "pull-request": "https://github.com/example/payments-api/pull/412",
          "base": "main",
          "publication": "merged",
          "order": 1
        },
        {
          "target": "checkout-service",
          "repository": "github.com/example/checkout-service",
          "merge-commit": null,
          "branch": "change/checkout-v2",
          "pull-request": "https://github.com/example/checkout-service/pull/98",
          "base": "main",
          "publication": "open",
          "order": 2
        }
      ],
      "verification": "pending",
      "failures": [
        { "member": "checkout-service", "reason": "unmerged" }
      ]
    }"#;

    #[test]
    fn worked_example_validates() {
        let record: Record = serde_json::from_str(WORKED_EXAMPLE).expect("worked example parses");
        assert_eq!(record.change, "checkout-v2");
        assert_eq!(record.verification, Verification::Pending);
        assert_eq!(record.members[0].publication, PublicationState::Merged);
        assert_eq!(record.members[1].merge_commit, None);
        assert_eq!(record.members[1].order, Some(2));
    }

    #[test]
    fn unknown_field_refused() {
        let mut value: serde_json::Value =
            serde_json::from_str(WORKED_EXAMPLE).expect("example parses");
        value["surprise"] = serde_json::json!(true);
        let err = serde_json::from_value::<Record>(value).expect_err("unknown field refuses");
        assert!(err.to_string().contains("surprise"), "{err}");
    }

    #[test]
    fn order_omitted_when_absent() {
        let record = Record {
            change: "demo".to_string(),
            members: vec![publication::MemberRecord {
                target: "a".to_string(),
                repository: "github.com/o/a".to_string(),
                merge_commit: None,
                branch: "change/demo".to_string(),
                pull_request: None,
                base: None,
                publication: PublicationState::Unpublished,
                order: None,
            }],
            verification: Verification::Pending,
            failures: Vec::new(),
        };
        let rendered = serde_json::to_string(&record).expect("render");
        assert!(!rendered.contains("order"), "{rendered}");
        assert!(rendered.contains("\"merge-commit\":null"), "{rendered}");
    }
}
