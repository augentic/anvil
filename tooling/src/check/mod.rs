pub mod adapter;
pub mod agent_teams;
pub mod brief;
mod docs_quality;
pub mod links;
mod plugins;
mod prose;
pub mod tools;

use crate::context::Context;
use crate::finding::{Check, Finding};

pub use adapter::{run_adapter_check, AdapterCheck, RULE_MISSING_MANIFEST, RULE_SCHEMA_VIOLATION};
pub use agent_teams::AgentTeamsCheck;
pub use brief::BriefCheck;
pub use docs_quality::{MissingDiagramAsset, RfcCitationInDocs, TextPipelineDiagram};
pub use links::LinksCheck;
pub use plugins::{BrokenSymlinkCheck, MarketplaceDriftCheck};
pub use prose::{InvocationPositional, OperationalVocabulary, SkillNumericCaps};
pub use tools::{DeclaredToolEquivalentInvocations, FirstPartyToolDeclarations};

/// Run every registered check predicate sequentially.
pub fn run(ctx: &Context) -> Vec<Finding> {
    let checks: [&dyn Check; 14] = [
        &AdapterCheck,
        &AgentTeamsCheck,
        &BriefCheck,
        &RfcCitationInDocs,
        &MissingDiagramAsset,
        &TextPipelineDiagram,
        &LinksCheck,
        &BrokenSymlinkCheck,
        &MarketplaceDriftCheck,
        &FirstPartyToolDeclarations,
        &DeclaredToolEquivalentInvocations,
        &OperationalVocabulary,
        &SkillNumericCaps,
        &InvocationPositional,
    ];
    let mut findings = Vec::new();

    for check in checks {
        findings.extend(check.run(ctx));
    }

    findings
}
