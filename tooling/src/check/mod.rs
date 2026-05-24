pub mod adapter;
pub mod agent_teams;
pub mod brief;
pub mod codex;
mod docs_quality;
pub mod links;
mod plugins;
mod prose;
pub mod scenarios;
pub mod skill_frontmatter;
mod skill_body;
pub mod tools;

use crate::context::Context;
use crate::finding::{Check, Finding};

pub use adapter::{run_adapter_check, AdapterCheck, RULE_MISSING_MANIFEST, RULE_SCHEMA_VIOLATION};
pub use agent_teams::AgentTeamsCheck;
pub use brief::BriefCheck;
pub use codex::{
    run_codex_check, CodexCheck, RULE_DUPLICATE_RULE_ID, RULE_NAMESPACE_OWNERSHIP_VIOLATION,
};
pub use docs_quality::{MissingDiagramAsset, RfcCitationInDocs, TextPipelineDiagram};
pub use links::LinksCheck;
pub use plugins::{BrokenSymlinkCheck, MarketplaceDriftCheck};
pub use prose::{InvocationPositional, OperationalVocabulary, SkillNumericCaps};
pub use scenarios::{
    check_recorded_trace_freshness, validate_scenario_frontmatter, ScenariosCheck,
    RULE_ARTIFACT_PATH_UNSAFE as SCENARIO_RULE_ARTIFACT_PATH_UNSAFE,
    RULE_BODY_ID_MISMATCH as SCENARIO_RULE_BODY_ID_MISMATCH,
    RULE_DUPLICATE_ID as SCENARIO_RULE_DUPLICATE_ID,
    RULE_RECORDED_TRACE_VIOLATION, RULE_SCHEMA_VIOLATION as SCENARIO_RULE_SCHEMA_VIOLATION,
    RULE_STAGES_NOT_CONTIGUOUS, RULE_STALE_RECORDED_TRACE,
};
pub use skill_frontmatter::{
    SkillArgumentHintGrammarCheck, SkillDescriptionGrammarCheck, SkillDuplicateNameCheck,
    SkillFrontmatterSchemaCheck, SkillNameDirectoryMismatchCheck, SkillUnknownToolCheck,
    RULE_ARGUMENT_HINT_GRAMMAR, RULE_DESCRIPTION_GRAMMAR, RULE_DUPLICATE_NAME,
    RULE_MISSING_FRONTMATTER, RULE_NAME_DIRECTORY_MISMATCH,
    RULE_SCHEMA_VIOLATION as SKILL_RULE_SCHEMA_VIOLATION, RULE_UNKNOWN_TOOL,
};
pub use skill_body::{
    SkillBodyLineCount, SkillEnvelopeJsonInBody, SkillFrontmatterRestatement,
    SkillInlineJsonTooLong, SkillInvalidCriticalPath, SkillMissingCriticalPath,
    SkillSectionLineCount, SkillStepBodyDuplicatesCriticalPath, SkillVariableCoverage,
};
pub use tools::{DeclaredToolEquivalentInvocations, FirstPartyToolDeclarations};

/// Run every registered check predicate sequentially.
pub fn run(ctx: &Context) -> Vec<Finding> {
    let checks: [&dyn Check; 31] = [
        &AdapterCheck,
        &AgentTeamsCheck,
        &BriefCheck,
        &CodexCheck,
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
        &ScenariosCheck,
        &SkillFrontmatterSchemaCheck,
        &SkillNameDirectoryMismatchCheck,
        &SkillDuplicateNameCheck,
        &SkillUnknownToolCheck,
        &SkillDescriptionGrammarCheck,
        &SkillArgumentHintGrammarCheck,
        &SkillBodyLineCount,
        &SkillSectionLineCount,
        &SkillMissingCriticalPath,
        &SkillInvalidCriticalPath,
        &SkillInlineJsonTooLong,
        &SkillEnvelopeJsonInBody,
        &SkillStepBodyDuplicatesCriticalPath,
        &SkillFrontmatterRestatement,
        &SkillVariableCoverage,
    ];
    let mut findings = Vec::new();

    for check in checks {
        findings.extend(check.run(ctx));
    }

    findings
}
