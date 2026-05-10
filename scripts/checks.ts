// Documentation consistency checks for the Augentic Plugins repository.
// Run via: make checks
// Exit code 0 = all checks pass; non-zero = one or more failures.
//
// This file is the thin orchestration layer. Predicates live in
// `scripts/checks/<concern>.ts`; shared helpers (the failure counter,
// REPO_ROOT, the standards-allowlist loader, etc.) live in
// `scripts/checks/_shared.ts`. The split keeps each concern under
// ~500 LoC and makes the per-PR diff easier to review.

import { errorCount, NC, RED } from "./checks/_shared.ts";
import {
  checkDirectives,
  checkMarkdownLinks,
  checkReferences,
} from "./checks/links.ts";
import {
  checkCapabilityIntegrity,
  checkInstructionPreambles,
  validateCapabilityYaml,
} from "./checks/capability.ts";
import {
  checkDeclaredToolEquivalentInvocations,
  checkFirstPartyToolDeclarations,
} from "./checks/tools.ts";
import { checkPluginConsistency, checkSymlinks } from "./checks/plugins.ts";
import {
  checkArgumentHint,
  checkDescriptionLength,
  checkNoLicense,
  validateSkillFrontmatter,
} from "./checks/skill_frontmatter.ts";
import {
  checkBodyLineCount,
  checkCriticalPath,
  checkInlineJsonBlocks,
  checkVariables,
} from "./checks/skill_body.ts";
import {
  checkNoFrontmatterRestatement,
  checkNoPhaseOutcomeContractRestatement,
  checkNoRfcCitationsInSkillBody,
  checkOneGuardrailsBlockPerSkill,
} from "./checks/skill_discipline.ts";
import {
  checkInvocationPositionals,
  checkLegacyLayout,
  checkRetiredAffectsField,
  checkRetiredSlashCommands,
  checkStaleClaims,
  checkWorkspaceLanding,
} from "./checks/prose.ts";
import {
  checkRecordedTraceFreshness,
  validateScenarioFrontmatter,
} from "./checks/scenarios.ts";
import { validateCodexRuleShape } from "./checks/codex.ts";

await Promise.all([
  checkMarkdownLinks(),
  checkStaleClaims(),
  checkSymlinks(),
]);
await Promise.all([
  validateCapabilityYaml(),
  checkCapabilityIntegrity(),
  checkFirstPartyToolDeclarations(),
  checkInstructionPreambles(),
  checkWorkspaceLanding(),
  checkRetiredAffectsField(),
  checkLegacyLayout(),
  validateScenarioFrontmatter(),
  checkRecordedTraceFreshness(),
  validateCodexRuleShape(),
]);
await Promise.all([
  validateSkillFrontmatter(),
  checkBodyLineCount(),
  checkCriticalPath(),
  checkDescriptionLength(),
  checkArgumentHint(),
  checkInvocationPositionals(),
  checkNoLicense(),
  checkInlineJsonBlocks(),
  checkReferences(),
  checkVariables(),
  checkDirectives(),
  checkPluginConsistency(),
  checkRetiredSlashCommands(),
  checkDeclaredToolEquivalentInvocations(),
  checkNoRfcCitationsInSkillBody(),
  checkOneGuardrailsBlockPerSkill(),
  checkNoPhaseOutcomeContractRestatement(),
  checkNoFrontmatterRestatement(),
]);

console.log();
const total = errorCount();
if (total > 0) {
  console.log(`${RED}${total} check(s) failed.${NC}`);
  Deno.exit(1);
}

console.log("All checks passed.");
