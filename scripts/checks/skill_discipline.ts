// Skill body discipline (Skills-1):
//
//   - noRfcCitationsInSkillBody
//   - oneGuardrailsBlockPerSkill
//   - noPhaseOutcomeContractRestatement
//   - noFrontmatterRestatement
//
// Per-file baselines come from `scripts/standards-allowlist.toml`. A
// live count strictly greater than the baseline fails CI; missing
// entries default to 0 (new files start clean). Skills-2 drives every
// entry to 0.

import {
  baselineFor,
  fail,
  relative,
  REPO_ROOT,
  skillBodyLines,
  skillFrontmatter,
  walkSkillFiles,
} from "./_shared.ts";

const PHASE_OUTCOME_CONTRACT_OPENING =
  /Every phase (?:exits|ends) by stamping/i;
const PHASE_OUTCOME_LINK = /references\/phase-outcome-contract\.md/;

export async function checkNoRfcCitationsInSkillBody(): Promise<void> {
  const RFC_RE = /RFC[- ]?\d+/g;
  for (const path of await walkSkillFiles()) {
    const rel = relative(REPO_ROOT, path);
    const lines = skillBodyLines(await Deno.readTextFile(path));
    if (!lines) continue;
    let count = 0;
    let inFence = false;
    for (const line of lines) {
      if (line.startsWith("```")) {
        inFence = !inFence;
        continue;
      }
      if (inFence) continue;
      const text = line.replace(/\[[^\]]*\]\([^)]*rfcs\/[^)]*\)/g, "");
      const matches = text.match(RFC_RE);
      if (matches) count += matches.length;
    }
    const baseline = await baselineFor("noRfcCitationsInSkillBody", rel);
    if (count > baseline) {
      fail(
        `RFC citation in skill body: ${rel} — ${count} > baseline ${baseline} (move to a trailing ## References block or rfcs/ archive link)`,
      );
    }
  }
}

export async function checkOneGuardrailsBlockPerSkill(): Promise<void> {
  for (const path of await walkSkillFiles()) {
    const rel = relative(REPO_ROOT, path);
    const lines = skillBodyLines(await Deno.readTextFile(path));
    if (!lines) continue;
    let guardrails = 0;
    let modeGuardrails = 0;
    for (const line of lines) {
      const trimmed = line.trim();
      if (trimmed === "## Guardrails") guardrails++;
      else if (trimmed === "## Mode-specific guardrails") modeGuardrails++;
    }
    if (guardrails > 1) {
      fail(
        `Multiple ## Guardrails blocks in ${rel} (found ${guardrails}; expected ≤1)`,
      );
    }
    if (modeGuardrails > 1) {
      fail(
        `Multiple ## Mode-specific guardrails blocks in ${rel} (found ${modeGuardrails}; expected ≤1)`,
      );
    }
  }
}

export async function checkNoPhaseOutcomeContractRestatement(): Promise<void> {
  for (const path of await walkSkillFiles()) {
    const rel = relative(REPO_ROOT, path);
    const lines = skillBodyLines(await Deno.readTextFile(path));
    if (!lines) continue;
    const headingIndex = lines.findIndex((line) =>
      line.trim() === "## Phase outcome contract"
    );
    if (headingIndex < 0) continue;
    const bodyLines: string[] = [];
    for (let i = headingIndex + 1; i < lines.length; i++) {
      const line = lines[i];
      if (line.startsWith("## ")) break;
      if (line.trim().length > 0) bodyLines.push(line);
    }
    const restated = bodyLines.some((line) =>
      PHASE_OUTCOME_CONTRACT_OPENING.test(line)
    );
    const isLink = bodyLines.length === 1 &&
      PHASE_OUTCOME_LINK.test(bodyLines[0]);
    if (restated && !isLink) {
      const baseline = await baselineFor(
        "noPhaseOutcomeContractRestatement",
        rel,
      );
      if (baseline === 0) {
        fail(
          `Phase outcome contract restated in ${rel}; replace with a single-line link to references/phase-outcome-contract.md`,
        );
      }
    }
  }
}

export async function checkNoFrontmatterRestatement(): Promise<void> {
  for (const path of await walkSkillFiles()) {
    const rel = relative(REPO_ROOT, path);
    const content = await Deno.readTextFile(path);
    const fm = skillFrontmatter(content);
    const description = typeof fm?.description === "string"
      ? fm.description
      : null;
    if (!description) continue;
    const lines = skillBodyLines(content);
    if (!lines) continue;
    const firstH2Index = lines.findIndex((line) => line.startsWith("## "));
    if (firstH2Index < 0) continue;
    let body = "";
    for (let i = firstH2Index + 1; i < lines.length; i++) {
      const line = lines[i];
      if (line.startsWith("## ")) break;
      body += line + " ";
    }
    if (body.includes(description.trim())) {
      const baseline = await baselineFor("noFrontmatterRestatement", rel);
      if (baseline === 0) {
        fail(
          `Frontmatter description restated under first H2 in ${rel}; lead with new prose, not a copy of the description.`,
        );
      }
    }
  }
}
