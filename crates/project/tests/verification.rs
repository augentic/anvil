//! Closed RFC-97 Phase A verification vocabulary.

use std::str::FromStr;

use error::Error;
use project::verification::{
    Discriminant, ExecutionAssurance, FINDING_SOURCE_TOOL, OracleAssurance, ProfileName,
    SandboxFeature, VerificationContextKind, ci_exclusive, finding_source_tool,
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
        assert!(ProfileName::from_str("unknown").is_err());
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
        assert!(VerificationContextKind::from_str("slice").is_err());
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
        assert_eq!(
            SandboxFeature::NoInheritedCredentials.to_string(),
            "no-inherited-credentials"
        );
        assert_eq!(SandboxFeature::EgressDeny.to_string(), "egress-deny");
        assert_eq!(SandboxFeature::ResourceLimits.to_string(), "resource-limits");
        assert_eq!(SandboxFeature::ProcessTreeReap.to_string(), "process-tree-reap");
        assert_eq!(SandboxFeature::EphemeralWriteRoots.to_string(), "ephemeral-write-roots");
        assert_eq!(
            SandboxFeature::ProtectedInputReadonly.to_string(),
            "protected-input-readonly"
        );
    }
}

mod ci {
    use super::*;

    #[test]
    fn exclusive() {
        assert!(
            ci_exclusive(&[ProfileName::Fmt, ProfileName::Ci]),
            "fmt+ci is incoherent"
        );
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
        assert!(Discriminant::from_str("verification-unknown").is_err());
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
