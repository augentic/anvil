pub mod agent_teams;
pub mod brief;
mod docs_quality;
mod prose;

use crate::context::Context;
use crate::finding::{Check, Finding};

pub use agent_teams::AgentTeamsCheck;
pub use brief::BriefCheck;
pub use docs_quality::{MissingDiagramAsset, RfcCitationInDocs, TextPipelineDiagram};
pub use prose::{InvocationPositional, OperationalVocabulary, SkillNumericCaps};

// Predicate modules (`adapter`, `skill_frontmatter`, …) land as sibling files under
// `src/check/` in RFC-5 Changes 3–14.

/// Run every registered check predicate sequentially.
pub fn run(ctx: &Context) -> Vec<Finding> {
    let checks: [&dyn Check; 8] = [
        &AgentTeamsCheck,
        &BriefCheck,
        &RfcCitationInDocs,
        &MissingDiagramAsset,
        &TextPipelineDiagram,
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
