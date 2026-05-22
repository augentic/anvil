// Documentation consistency checks for the Augentic Plugins repository.
// Run via: make checks
// Exit code 0 = all checks pass; non-zero = one or more failures.
//
// This file is the thin orchestration layer. Predicates live in
// `scripts/checks/<concern>.ts`; shared helpers (the failure counter,
// REPO_ROOT, etc.) live in `scripts/checks/_shared.ts`. The split keeps
// each concern under ~500 LoC and makes the per-PR diff easier to review.

import { errorCount, NC, RED } from "./checks/_shared.ts";
import {
  checkDirectives,
  checkMarkdownLinks,
  checkReferences,
} from "./checks/links.ts";
import { validateAdapterYaml } from "./checks/adapter.ts";
import {
  checkDeclaredToolEquivalentInvocations,
  checkFirstPartyToolDeclarations,
} from "./checks/tools.ts";
import { checkPluginConsistency, checkSymlinks } from "./checks/plugins.ts";
import {
  checkArgumentHintCoversBodyArguments,
  checkDescriptionHasUseWhen,
  checkDescriptionLength,
  checkDescriptionStartsWithVerb,
  checkNoLicense,
  validateArgumentHints,
  validateSkillFrontmatter,
} from "./checks/skill_frontmatter.ts";
import {
  checkBodyAndSectionLineCounts,
  checkCriticalPath,
  checkInlineJsonBlocks,
  checkNoEnvelopeExamples,
  checkNoFrontmatterRestatement,
  checkNoStepBodyDuplicatesCriticalPath,
  checkVariables,
} from "./checks/skill_body.ts";
import {
  checkInvocationPositionals,
  checkOperationalVocabulary,
  checkSkillNumericCaps,
} from "./checks/prose.ts";
import { checkNoRfcCitationsInDocs } from "./checks/docs_quality.ts";
import { checkBriefSize } from "./checks/brief_size.ts";
import {
  checkRecordedTraceFreshness,
  validateScenarioFrontmatter,
} from "./checks/scenarios.ts";
import { validateCodexRuleShape } from "./checks/codex.ts";

await Promise.all([
  checkMarkdownLinks(),
  checkSymlinks(),
]);
await Promise.all([
  validateAdapterYaml(),
  checkFirstPartyToolDeclarations(),
  checkOperationalVocabulary(),
  checkSkillNumericCaps(),
  validateScenarioFrontmatter(),
  checkRecordedTraceFreshness(),
  validateCodexRuleShape(),
  checkBriefSize(),
]);
await Promise.all([
  validateSkillFrontmatter(),
  checkBodyAndSectionLineCounts(),
  checkCriticalPath(),
  checkDescriptionLength(),
  checkDescriptionStartsWithVerb(),
  checkDescriptionHasUseWhen(),
  validateArgumentHints(),
  checkArgumentHintCoversBodyArguments(),
  checkInvocationPositionals(),
  checkNoLicense(),
  checkInlineJsonBlocks(),
  checkNoEnvelopeExamples(),
  checkNoFrontmatterRestatement(),
  checkNoStepBodyDuplicatesCriticalPath(),
  checkReferences(),
  checkVariables(),
  checkDirectives(),
  checkPluginConsistency(),
  checkDeclaredToolEquivalentInvocations(),
  checkNoRfcCitationsInDocs(),
]);

console.log();
const total = errorCount();
if (total > 0) {
  console.log(`${RED}${total} check(s) failed.${NC}`);
  Deno.exit(1);
}

console.log("All checks passed.");
