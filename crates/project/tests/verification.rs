//! Closed RFC-97 Phase A verification vocabulary and profile reports.

use std::str::FromStr;

use diagnostics::{Artifact, Diagnostic, Severity};
use error::Error;
use project::Platform;
use project::snapshot::SnapshotId;
use project::verification::{
    Context, Discriminant, Edit, ExecutionAssurance, FINDING_SOURCE_TOOL, Handle, OracleAssurance,
    ProfileName, ProfileReport, RawOutput, SandboxFeature, SuggestionGroup, ToolPin,
    VerificationContextKind, ci_exclusive, finding_source_tool, regression, unchanged_failure_set,
};
use strum::VariantArray;

fn assert_wire<T>(value: T)
where
    T: Copy + PartialEq + std::fmt::Debug + std::fmt::Display + FromStr,
    T::Err: std::fmt::Debug,
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let name = value.to_string();
    assert_eq!(name.parse::<T>().unwrap(), value, "FromStr(Display) round trip");
    let yaml = serde_saphyr::to_string(&value).unwrap();
    assert_eq!(yaml.trim(), name, "serde wire name must match Display");
    let parsed: T = serde_saphyr::from_str(&name).unwrap();
    assert_eq!(parsed, value, "serde parse of Display");
}

mod wire {
    use super::*;

    #[test]
    fn profile_names() {
        for name in ProfileName::VARIANTS {
            assert_wire(*name);
        }
        assert_eq!(ProfileName::Fmt.to_string(), "fmt");
        assert_eq!(ProfileName::Ci.to_string(), "ci");
        ProfileName::from_str("unknown").unwrap_err();
    }

    #[test]
    fn context_kinds() {
        for kind in VerificationContextKind::VARIANTS {
            assert_wire(*kind);
        }
        assert_eq!(VerificationContextKind::SliceAttempt.to_string(), "slice-attempt");
        assert_eq!(VerificationContextKind::FrontierDomain.to_string(), "frontier-domain");
        assert_eq!(VerificationContextKind::CompleteDomain.to_string(), "complete-domain");
        assert!(VerificationContextKind::SliceAttempt.accepted());
        assert!(!VerificationContextKind::FrontierDomain.accepted());
        assert!(!VerificationContextKind::CompleteDomain.accepted());
        VerificationContextKind::from_str("slice").unwrap_err();
    }

    #[test]
    fn oracle_assurance() {
        for value in OracleAssurance::VARIANTS {
            assert_wire(*value);
        }
        assert_eq!(OracleAssurance::Candidate.to_string(), "candidate");
        assert_eq!(OracleAssurance::Protected.to_string(), "protected");
        assert_eq!(OracleAssurance::Mixed.to_string(), "mixed");
    }

    #[test]
    fn execution_assurance() {
        for value in ExecutionAssurance::VARIANTS {
            assert_wire(*value);
        }
        assert_eq!(ExecutionAssurance::ModelAssisted.to_string(), "model-assisted");
        assert_eq!(ExecutionAssurance::HostAttested.to_string(), "host-attested");
        assert_eq!(ExecutionAssurance::Hybrid.to_string(), "hybrid");
    }

    #[test]
    fn sandbox_features() {
        for feature in SandboxFeature::VARIANTS {
            assert_wire(*feature);
        }
        assert_eq!(SandboxFeature::WorkdirBind.to_string(), "workdir-bind");
        assert_eq!(SandboxFeature::EnvAllowlist.to_string(), "env-allowlist");
        assert_eq!(SandboxFeature::NoInheritedCredentials.to_string(), "no-inherited-credentials");
        assert_eq!(SandboxFeature::EgressDeny.to_string(), "egress-deny");
        assert_eq!(SandboxFeature::ResourceLimits.to_string(), "resource-limits");
        assert_eq!(SandboxFeature::ProcessTreeReap.to_string(), "process-tree-reap");
        assert_eq!(SandboxFeature::EphemeralWriteRoots.to_string(), "ephemeral-write-roots");
        assert_eq!(SandboxFeature::ProtectedInputReadonly.to_string(), "protected-input-readonly");
    }
}

mod ci {
    use super::*;

    #[test]
    fn exclusive() {
        assert!(ci_exclusive(&[ProfileName::Fmt, ProfileName::Ci]), "fmt+ci is incoherent");
        assert!(
            ci_exclusive(&[ProfileName::Ci, ProfileName::Test, ProfileName::Build]),
            "ci with any other name is incoherent"
        );
        assert!(!ci_exclusive(&[ProfileName::Ci]), "ci alone is not incoherent");
        assert!(
            !ci_exclusive(&[ProfileName::Fmt, ProfileName::Build, ProfileName::Clippy]),
            "a non-ci set is not incoherent"
        );
        assert!(!ci_exclusive(&[]), "an empty set is not incoherent");
        assert!(!ci_exclusive(&[ProfileName::Ci, ProfileName::Ci]), "ci repeated is still only ci");
    }
}

mod discriminants {
    use super::*;

    const EXIT_2: &[(Discriminant, &str)] = &[
        (Discriminant::ProfileUnavailable, "verification-profile-unavailable"),
        (Discriminant::SandboxDenied, "verification-sandbox-denied"),
        (Discriminant::ToolMissing, "verification-tool-missing"),
        (Discriminant::ParserMissing, "verification-parser-missing"),
        (Discriminant::LimitExhausted, "verification-limit-exhausted"),
        (Discriminant::PlatformUnsupported, "verification-platform-unsupported"),
        (Discriminant::AttestationMismatch, "verification-attestation-mismatch"),
        (Discriminant::AttestationDuplicate, "verification-attestation-duplicate"),
        (Discriminant::ProfilesIncoherent, "verification-profiles-incoherent"),
    ];

    const EXIT_1: &[(Discriminant, &str)] = &[
        (Discriminant::Cancelled, "verification-cancelled"),
        (Discriminant::AttestationPersistFailed, "verification-attestation-persist-failed"),
    ];

    #[test]
    fn codes() {
        for (discriminant, code) in EXIT_2.iter().chain(EXIT_1) {
            assert_eq!(discriminant.code(), *code);
            assert_eq!(discriminant.to_string(), *code);
            assert_eq!(Discriminant::from_str(code).unwrap(), *discriminant);
            assert_eq!(discriminant.error("detail").variant_str(), *code);
        }
        assert_eq!(Discriminant::VARIANTS.len(), EXIT_2.len() + EXIT_1.len());
        Discriminant::from_str("verification-unknown").unwrap_err();
    }

    #[test]
    fn exits() {
        for (discriminant, _) in EXIT_2 {
            assert!(discriminant.validation(), "{} is exit 2", discriminant.code());
            assert!(
                matches!(discriminant.error("detail"), Error::Validation { .. }),
                "{} routes through Error::Validation",
                discriminant.code()
            );
        }
        for (discriminant, _) in EXIT_1 {
            assert!(!discriminant.validation(), "{} is exit 1", discriminant.code());
            assert!(
                matches!(discriminant.error("detail"), Error::Diag { .. }),
                "{} routes through Error::Diag",
                discriminant.code()
            );
        }
    }

    #[test]
    fn finding_sibling() {
        let err = finding_source_tool("adapter finding claimed source tool");
        assert_eq!(err.variant_str(), FINDING_SOURCE_TOOL);
        assert_eq!(FINDING_SOURCE_TOOL, "target-phase-finding-source-tool");
        assert!(matches!(err, Error::Diag { .. }));
    }
}

fn digest(fill: char) -> SnapshotId {
    SnapshotId::from_digest(&fill.to_string().repeat(64))
}

fn finding(severity: Severity, fill: char) -> Diagnostic {
    let mut diagnostic =
        Diagnostic::violation("test.rule", "title", "detail", Artifact::Code, None);
    diagnostic.severity = severity;
    diagnostic.fingerprint = digest(fill).to_string();
    diagnostic
}

fn report(findings: Vec<Diagnostic>) -> ProfileReport {
    ProfileReport {
        profile: ProfileName::Fmt,
        platform: Platform::Core,
        context: Context {
            kind: VerificationContextKind::SliceAttempt,
            change: "demo".into(),
            slice: "greeting".into(),
            attempt: 1,
        },
        candidate: digest('c'),
        policy_digest: digest('1'),
        report_digest: digest('2'),
        oracle_assurance: OracleAssurance::Candidate,
        protected_inputs: Vec::new(),
        oracles: Vec::new(),
        enforced_sandbox: vec![SandboxFeature::WorkdirBind, SandboxFeature::EnvAllowlist],
        toolchain_identity: vec![ToolPin {
            name: "rustc".into(),
            version: "1.80.0".into(),
            digest: digest('3'),
        }],
        findings,
        suggestion_group: Some(SuggestionGroup {
            edits: vec![Edit {
                path: "src/lib.rs".into(),
                preimage_digest: digest('a'),
                result_digest: digest('b'),
            }],
        }),
        raw: Some(RawOutput {
            digest: digest('d'),
            tail: "error: unused import".into(),
        }),
    }
}

mod report {
    use super::*;

    #[test]
    fn yaml_round_trip() {
        let original = report(vec![finding(Severity::Important, '1')]);
        let yaml = original.canonical_yaml().expect("yaml");
        assert!(yaml.contains("oracle-assurance:"));
        assert!(yaml.contains("policy-digest:"));
        assert!(yaml.contains("report-digest:"));
        assert!(yaml.contains("protected-inputs:"));
        assert!(yaml.contains("enforced-sandbox:"));
        assert!(yaml.contains("toolchain-identity:"));
        assert!(yaml.contains("suggestion-group:"));
        assert!(yaml.contains("preimage-digest:"));
        assert!(!yaml.contains("execution-assurance"));
        let parsed: ProfileReport = serde_saphyr::from_str(&yaml).expect("parse");
        assert_eq!(parsed, original);
    }

    #[test]
    fn digest_stable() {
        let original = report(vec![finding(Severity::Critical, '1')]);
        let yaml = original.canonical_yaml().expect("yaml");
        let reordered =
            yaml.replacen("profile: fmt\nplatform: core\n", "platform: core\nprofile: fmt\n", 1);
        assert_ne!(reordered, yaml, "fixture must actually reorder fields");
        let parsed: ProfileReport = serde_saphyr::from_str(&reordered).expect("parse reordered");
        assert_eq!(parsed, original);
        assert_eq!(parsed.canonical_yaml().expect("reserialise"), yaml);
        assert_eq!(parsed.handle().expect("handle"), original.handle().expect("handle"));
    }

    #[test]
    fn unknown_field_rejected() {
        let mut yaml = report(Vec::new()).canonical_yaml().expect("yaml");
        yaml.push_str("execution-assurance: host-attested\n");
        serde_saphyr::from_str::<ProfileReport>(&yaml).unwrap_err();
    }
}

mod compare {
    use super::*;

    #[test]
    fn predicates() {
        let critical = finding(Severity::Critical, '1');
        let important = finding(Severity::Important, '2');
        let suggestion = finding(Severity::Suggestion, '3');
        let same_block = report(vec![critical.clone(), suggestion.clone()]);
        let same_block_extra =
            report(vec![critical.clone(), suggestion, finding(Severity::Optional, '4')]);
        assert!(
            unchanged_failure_set(&same_block, &same_block_extra),
            "non-blocking findings do not change the failure set"
        );

        let other_block = report(vec![important.clone()]);
        assert!(!unchanged_failure_set(&same_block, &other_block));

        let best = report(vec![important]);
        let worse = report(vec![critical]);
        assert!(regression(&worse, &best), "more criticals is a regression");
        assert!(!regression(&best, &worse));
        assert!(!regression(&best, &best), "equal counts are not a regression");
    }
}

mod store {
    use super::*;

    #[test]
    fn persist_path_layout() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let attempt = tmp.path().join("build").join("attempts").join("0001");
        std::fs::create_dir_all(&attempt).expect("attempt dir");
        let original = report(vec![finding(Severity::Important, '1')]);
        let handle = original.persist(&attempt).expect("persist");
        let path = handle.path(&attempt);
        assert_eq!(
            path,
            attempt.join("attestations").join(handle.digest()),
            "attestations live beside phases/ under the attempt"
        );
        assert_eq!(handle.as_str(), format!("sha256:{}", handle.digest()));
        assert_eq!(path.file_name().and_then(|name| name.to_str()), Some(handle.digest()));
        assert_eq!(handle, original.handle().expect("handle"));
        assert_eq!(Handle::from_bytes(std::fs::read(&path).expect("bytes").as_slice()), handle);
        let loaded = ProfileReport::load(&attempt, &handle).expect("load");
        assert_eq!(loaded, original);
    }

    #[test]
    fn persist_failed() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let attempt = tmp.path().join("not-a-dir");
        std::fs::write(&attempt, b"file").expect("blocker");
        let err = report(Vec::new()).persist(&attempt).expect_err("persist must fail");
        assert_eq!(err.variant_str(), "verification-attestation-persist-failed");
        assert!(matches!(err, Error::Diag { .. }));
    }

    #[test]
    fn load_mismatch() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let attempt = tmp.path().join("build").join("attempts").join("0001");
        let original = report(Vec::new());
        let handle = original.persist(&attempt).expect("persist");
        std::fs::write(handle.path(&attempt), b"tampered\n").expect("tamper");
        let err = ProfileReport::load(&attempt, &handle).expect_err("tamper must fail");
        assert_eq!(err.variant_str(), "verification-attestation-mismatch");
    }
}
