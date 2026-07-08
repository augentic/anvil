//! Judgment-leg integration tests: the propose reconciliation and slice
//! synthesis legs against a scripted [`MockModel`], proving the
//! schema-gate + kernel tails and the bounded repair loop.

use std::collections::BTreeMap;

use artifacts::evidence::{AuthorityClass, ClaimKind};
use guest_model::MockModel;
use workflow_lib::change::{ProposalKind, ProposalRequest};
use workflow_lib::judgment::synthesize::Kernel;
use workflow_lib::judgment::{propose, synthesize};
use workflow_lib::slice::{BaselineIndex, ProjectionHeader, SynthesisInputs};

fn request() -> ProposalRequest {
    serde_json::from_value(serde_json::json!({
        "version": 1,
        "kind": "request",
        "projects": [{ "name": "shop", "target": "omnia@1.0.0" }],
        "leads": [{ "source": "legacy", "lead": "user-registration", "synopsis": "Registration endpoint." }]
    }))
    .expect("request fixture parses")
}

const GROUPING: &str = r#"{
  "version": 1,
  "kind": "response",
  "slices": [{
    "name": "user-registration",
    "sources": [{ "source": "legacy", "lead": "user-registration" }]
  }]
}"#;

mod propose_leg {
    use super::*;

    #[tokio::test]
    async fn happy_path() {
        let model = MockModel::answering([GROUPING]);
        let response = propose::reconcile(&model, &request(), None, |_| Ok(()))
            .await
            .expect("reconcile succeeds");
        assert_eq!(response.kind, ProposalKind::Response);
        assert_eq!(response.slices.len(), 1);
        assert_eq!(response.slices[0].name, "user-registration");
        assert!(response.gate.is_none(), "no gate prose without a context");

        let calls = model.requests();
        assert_eq!(calls.len(), 1);
        let system = calls[0].system.as_deref().expect("system prompt present");
        assert!(system.contains("Split on doubt"), "propose prompt carries the grouping rules");
        assert!(calls[0].lend_workspace, "the workspace preopen is lent");
        assert!(
            calls[0].messages[0].content.contains("user-registration"),
            "the request rides the user prompt"
        );
        assert!(
            !calls[0].messages[0].content.contains("## Plan context"),
            "no plan context without a gate context"
        );
    }

    #[tokio::test]
    async fn gate_context_rides_user_message() {
        let answer = serde_json::to_string(&serde_json::json!({
            "version": 1,
            "kind": "response",
            "slices": [{
                "name": "user-registration",
                "sources": [{ "source": "legacy", "lead": "user-registration" }]
            }],
            "gate": {
                "change": "## Intent\n\nRefresh registration.",
                "discovery-summary": "Sources: 1. Leads: 1.",
                "discovery-source-inventory": "| key | adapter | path |"
            }
        }))
        .expect("answer serialises");
        let model = MockModel::answering([Box::leak(answer.into_boxed_str()) as &'static str]);
        let sources = BTreeMap::from([(
            "legacy".to_string(),
            serde_json::from_value::<workflow_lib::change::SourceBinding>(serde_json::json!({
                "adapter": "typescript",
                "path": "./vendor/legacy"
            }))
            .expect("binding fixture parses"),
        )]);
        let context = propose::GateContext {
            plan: "account-revamp",
            sources: &sources,
        };
        let response = propose::reconcile(&model, &request(), Some(context), |_| Ok(()))
            .await
            .expect("reconcile succeeds");
        let gate = response.gate.expect("gate prose parsed");
        assert_eq!(gate.discovery_summary, "Sources: 1. Leads: 1.");

        let user = &model.requests()[0].messages[0].content;
        assert!(user.contains("## Plan context"), "plan context section present: {user}");
        assert!(user.contains("- plan: account-revamp"), "{user}");
        assert!(user.contains("legacy: adapter `typescript`, path `./vendor/legacy`"), "{user}");
    }

    #[tokio::test]
    async fn schema_failure_repairs() {
        // First answer fails the schema gate (a request-kind document);
        // the repair attempt succeeds.
        let bad = r#"{ "version": 1, "kind": "request", "projects": [], "leads": [] }"#;
        let model = MockModel::answering([bad, GROUPING]);
        let response = propose::reconcile(&model, &request(), None, |_| Ok(()))
            .await
            .expect("repair succeeds");
        assert_eq!(response.slices.len(), 1);

        let calls = model.requests();
        assert_eq!(calls.len(), 2, "one repair attempt");
        let repair = &calls[1].messages[0].content;
        assert!(repair.contains("## Findings"), "repair prompt carries the findings");
        assert!(repair.contains("Previous answer"), "repair prompt carries the failed answer");
    }

    #[tokio::test]
    async fn kernel_check_participates_in_repair() {
        let mut rejected = 0;
        let model = MockModel::answering([GROUPING, GROUPING]);
        let response = propose::reconcile(&model, &request(), None, |_| {
            if rejected == 0 {
                rejected += 1;
                return Err(error::Error::validation_failed(
                    "plan-reconcile-lead-uncovered",
                    "every lead is covered",
                    "lead `x/y` is not referenced by any slice",
                ));
            }
            Ok(())
        })
        .await
        .expect("kernel-rejected grouping repairs");
        assert_eq!(response.slices.len(), 1);
        assert_eq!(model.requests().len(), 2);
    }

    #[tokio::test]
    async fn model_failure_never_repairs() {
        // On the live backend a schema-invalid answer surfaces as a
        // model `invalid-answer` failure — which must propagate without
        // burning repair attempts (the request did not change).
        let model = MockModel::scripted([Err(guest_model::Error::InvalidAnswer(
            "answer failed the create gate".to_string(),
        ))]);
        let err = propose::reconcile(&model, &request(), None, |_| Ok(()))
            .await
            .expect_err("model failure propagates");
        assert!(err.to_string().contains("judgment-model-failed"), "{err}");
        assert_eq!(model.requests().len(), 1, "no repair attempt");
    }

    #[tokio::test]
    async fn repair_budget_exhausts() {
        let bad = r#"{ "version": 1, "kind": "request", "projects": [], "leads": [] }"#;
        let model = MockModel::answering([bad, bad, bad]);
        let err = propose::reconcile(&model, &request(), None, |_| Ok(()))
            .await
            .expect_err("budget exhausts");
        assert!(err.to_string().contains("proposal-schema"), "last tail failure surfaces: {err}");
        assert_eq!(model.requests().len(), 3, "initial attempt plus MAX_REPAIRS");
    }
}

mod synthesize_leg {
    use super::*;

    const ANSWER: &str = r###"{
      "version": 1,
      "kind": "response",
      "slice": "user-registration",
      "model": {
        "requirements": [{
          "title": "Register with email",
          "statement": "The system accepts registrations with RFC 5322 emails.",
          "domain": "identity",
          "claims": [{ "source": "legacy", "id": "users.register", "kind": "requirement" }]
        }],
        "tasks": [{ "id": "TASK-001", "text": "Implement registration" }]
      },
      "artifacts": {
        "proposal": "## Proposal",
        "design": "## Design",
        "tasks": "## Tasks",
        "specs": [{ "domain": "identity", "content": "## Identity" }]
      }
    }"###;

    fn inputs() -> SynthesisInputs {
        serde_json::from_value(serde_json::json!({
            "version": 1,
            "kind": "inputs",
            "slice": "user-registration",
            "sources": [],
            "guidance-brief": "Guidance brief body."
        }))
        .expect("inputs fixture parses")
    }

    fn header() -> ProjectionHeader {
        ProjectionHeader {
            version: 1,
            slice: "user-registration".to_string(),
            project: None,
        }
    }

    #[tokio::test]
    async fn happy_path_projects_kernel_fields() {
        let dir = tempfile::tempdir().expect("tempdir");
        let baseline = BaselineIndex::build(&dir.path().join("specs")).expect("empty baseline");
        let authority = BTreeMap::from([("legacy".to_string(), AuthorityClass::Behaviour)]);
        let claims = BTreeMap::from([(
            ("legacy".to_string(), "users.register".to_string()),
            ClaimKind::Requirement,
        )]);
        let overrides = BTreeMap::new();
        let kernel = Kernel {
            header: header(),
            authority: &authority,
            overrides: &overrides,
            evidence_claims: &claims,
            baseline_index: &baseline,
        };

        let model = MockModel::answering([ANSWER]);
        let synthesized =
            synthesize::synthesize(&model, &inputs(), &kernel).await.expect("synthesis succeeds");
        assert_eq!(synthesized.response.artifacts.specs.len(), 1);
        let req = &synthesized.projected.requirements[0];
        assert_eq!(req.id.as_deref(), Some("REQ-001"), "kernel allocates the requirement id");

        let calls = model.requests();
        assert_eq!(calls.len(), 1);
        let system = calls[0].system.as_deref().expect("system prompt present");
        assert!(system.contains("requirement-block.md"), "playbook references are pasted");
    }

    #[tokio::test]
    async fn unanchored_claim_repairs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let baseline = BaselineIndex::build(&dir.path().join("specs")).expect("empty baseline");
        let authority = BTreeMap::new();
        // No matching Evidence claim: the first answer's claim is
        // unanchored, the kernel rejects it in-loop, and the (identical)
        // repair answer fails the same way until the budget exhausts.
        let claims = BTreeMap::new();
        let overrides = BTreeMap::new();
        let kernel = Kernel {
            header: header(),
            authority: &authority,
            overrides: &overrides,
            evidence_claims: &claims,
            baseline_index: &baseline,
        };

        let model = MockModel::answering([ANSWER, ANSWER, ANSWER]);
        let err = synthesize::synthesize(&model, &inputs(), &kernel)
            .await
            .expect_err("unanchored claims exhaust the repair budget");
        assert_eq!(model.requests().len(), 3, "the kernel failure re-prompted twice");
        let repair = &model.requests()[1].messages[0].content;
        assert!(repair.contains("## Findings"), "kernel findings ride the repair prompt");
        assert!(err.to_string().contains("orphan") || err.to_string().contains("anchor"), "{err}");
    }
}
