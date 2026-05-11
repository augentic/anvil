# Skill Authoring Standards

Mechanically enforced rules for every `SKILL.md` in this repository: frontmatter shape, body discipline, references hand-off, and what skills must never do. The long-form rationale (discovery model, why metadata is precious, examples, the progressive-disclosure pattern, and the forbidden-frontmatter list) lives at [docs/contributing/skill-authoring.md](../contributing/skill-authoring.md); the rule file at [.cursor/rules/project.mdc](../../.cursor/rules/project.mdc#skill-authoring-conventions) is the normative checklist. This document captures the rules `make checks` enforces and the cross-cutting policies skills inherit by convention.

This is a pre-1.0 codebase. There are no backward-compatibility constraints on skill shape, frontmatter, or wire envelopes — when a rule changes, the SKILL.md changes with it. "Pre-RFC-N this used to be …" / "Phase 3.7 renamed it …" / "the v1.x verb was …" prose belongs in [docs/explanation/decision-log.md](../explanation/decision-log.md) and [docs/explanation/release-notes.md](../explanation/release-notes.md), not in the skill that operators read every day. Migration prose only stays in a SKILL.md when the skill itself documents a real legacy-migration feature (e.g. `/spec:extract` migrating off a legacy codebase).

## Description grammar

Each `SKILL.md` `description` field must:

- Lead with an **imperative verb** drawn from the curated allow-list in [scripts/checks/skill_frontmatter.ts](../../scripts/checks/skill_frontmatter.ts) (`IMPERATIVE_VERBS`). Extend the allow-list when a new verb is genuinely imperative; adding a verb is cheaper than rejecting a fine description.
- Contain a `Use when …` clause so the skill-discovery surface can match on intent rather than vocabulary. The clause describes a trigger condition (when an operator should reach for this skill over its siblings), not a restatement of what the skill does. "Use when an operator wants to X" is an anti-pattern; "Use when starting Y from scratch, not when resuming Z" is the shape.
- Stay **≤ 512 chars** total.

`checkDescriptionStartsWithVerb`, `checkDescriptionHasUseWhen`, and `checkDescriptionLength` enforce the three rules respectively. See [predicates.md](predicates.md) for the full table.

## Argument-hint grammar

Each `SKILL.md` `argument-hint:` value is a whitespace-separated sequence of tokens drawn from a fixed grammar (enforced by `checkArgumentHintGrammar` in [scripts/checks/skill_frontmatter.ts](../../scripts/checks/skill_frontmatter.ts)):

- `<name>` — required positional, kebab-case noun (e.g. `<slice-dir>`, `<change-name>`).
- `[name]` — optional positional (e.g. `[crate-name]`, `[mode]`).
- `<name>...` / `[name]...` — repeated positional (e.g. `<image-path>...`, `[file]...`).
- `<a|b|c>` / `[a|b|c]` — mutually exclusive alternatives.
- `--flag` — long flag (no value); when a flag carries a value, model the value as a sibling token: `--kind <kind>`.

Names are kebab-case (`[a-z][a-z0-9-]*` per alternative). Bare prose ("the slice name"), mixed punctuation (`<arg>: arg2`), trailing `?`, and short flags (`-f`) are rejected. The hint is the slash-command placeholder, not the full CLI signature — secondary arguments still belong in the body's "Invocation" section.

## Body caps

- **Body line count** ≤ **250 lines** (`checkBodyLineCount`). New skills must comply; existing skills that still exceed the cap are grandfathered via per-file `bodyLineCount` baselines in [scripts/standards-allowlist.toml](../../scripts/standards-allowlist.toml) and are expected to ratchet down with each touch.
- **Per-H2 section** ≤ **60 lines** (non-blank, non-comment) (`checkSectionLineCount`). Depth migrates into `references/<topic>.md`, linked from the section, rather than letting individual sections sprawl. Per-file `sectionLineCount` baselines in `scripts/standards-allowlist.toml` grandfather the irreducible remainder; new sections still fail fast.

All caps are floors, not budgets — overflow means the relocate-to-`references/` pattern needs to fire, not that the cap should be raised. The 250 / 60 / 512 numbers are kept synchronized across scripts, schema, rules, and docs by `checkSkillNumericCaps`.

## References discipline

Long-form rules, code-block examples, output templates, and edge-case enumerations belong in siblings the SKILL.md body links to (Anthropic's [progressive disclosure](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices#progressive-disclosure-patterns) pattern). The SKILL.md keeps the Critical Path, the invocation surface, the dispatch table (when applicable), and the canonical decision points; sibling files (`references/`, `examples/`, topical files) carry the prose.

Push prose to `references/<topic>.md` (or, for cross-skill prose, [plugins/references/](../../plugins/references/)) before raising any cap. The relocate-to-`references/` pattern is the canonical response when a section approaches the 60-line ceiling.

## Skill body discipline

The frontmatter and body caps above are the floor. These additional rules tighten the SKILL.md as a navigable artifact:

1. **No restating frontmatter in the body.** `description` and `argument-hint` already render on every invocation; do not repeat them in the first H2 (or any other body section). Mechanically enforced by `checkNoFrontmatterRestatement`.
2. **`## Critical Path` is the table of contents.** When a skill body is split into siblings, the SKILL.md keeps the Critical Path, the invocation surface, the dispatch table (when applicable), and the canonical decision points. Sibling files (`references/`, `examples/`, topical files) carry the long-form rules, examples, templates, and edge-case prose. The Critical Path may take either of two forms: a flat 5–7 entry numbered/bullet list, or 5–7 `### N. Title` H3 step headings (when each step has its own concise body); duplicating both forms in the same body is the anti-pattern this rule eliminated.
3. **No RFC citations in skill bodies.** `RFC-N` references in prose train operators on how the system was *built*, not how it works *today*. Move them to a trailing `## References` block as `[RFC-N](rfcs/...)` links, or to [docs/explanation/decision-log.md](../explanation/decision-log.md). Mechanically enforced by `checkNoRfcCitationsInSkillBody`.
4. **`## Phase outcome contract` is a single-line link, not a paragraph.** Replace the canonical opening prose with `> See [Phase outcome contract](../../references/phase-outcome-contract.md).` Mechanically enforced by `checkNoPhaseOutcomeContractRestatement`.

## Cross-cutting guardrails

Cross-cutting guardrails — the `.metadata.yaml` / slice-dir / plan-write rules that recur across skills — live in [plugins/references/guardrails.md](../../plugins/references/guardrails.md). SKILL.md files **link** to them; they do **not** restate them inline. Per-skill guardrails (don'ts that only apply to one skill) stay in the SKILL.md under a single `## Guardrails` (or `## Mode-specific guardrails`) H2; scattered IMPORTANT / Never / Critical scolding throughout the body trains agents to skim. Mechanically enforced by `checkOneGuardrailsBlockPerSkill`.

The canonical "skills MUST NOT" list:

- **Never hand-edit `.metadata.yaml`.** Every lifecycle transition flows through `specify slice transition`, `specify slice outcome set`, `specify slice journal append`, or `specify change plan transition`.
- **Never `mkdir -p .specify/...`.** Slice and plan directories are minted by `specify slice create` / `specify change plan create`; the CLI owns directory shape.
- **Never `mv` anything into `.specify/archive/`.** Archive moves are owned by `specify slice archive`, `specify slice drop`, `specify change plan archive`, and `specify change finalize`.
- **Never reimplement validation, capability resolution, or merge logic in skill prose.** Those are deterministic operations owned by the CLI; see [cli-contract.md](cli-contract.md).
- **Never embed raw CLI envelope JSON in a SKILL.md body.** Link to [plugins/references/cli-output-shapes.md](../../plugins/references/cli-output-shapes.md) with a stable anchor instead.

## Envelope examples and wire contract

CLI envelope shapes (the `envelope-version` + `data` / `error` wrapper) live in [plugins/references/cli-output-shapes.md](../../plugins/references/cli-output-shapes.md) with stable anchors. SKILL.md bodies **link** to the reference; they do not embed the envelope. `checkNoEnvelopeExamples` flags fenced ` ```json ` / ` ```jsonc ` blocks whose body looks like an envelope wrapper (`"envelope-version"` key, or `"ok"` + `"data"` / `"error"` pair). Body shapes that describe only a command's `data` payload — a one-line config snippet, an analyze sidecar — are still fine; the predicate is intentionally narrow.

The wire contract itself — exit codes, kebab-case `error` discriminants, the `envelope-version` floor — is owned by the CLI repo. See [cli-contract.md](cli-contract.md) for the surface skills depend on and the link to the authoritative exit-code table.

## Markdown style

- Do not hard-wrap prose in Markdown files solely for column width. Keep paragraphs and list-item prose on a single line unless the line break is semantically meaningful.
- Preserve intentional line breaks in frontmatter, tables, lists, blockquotes, and fenced code blocks.

## Skill / CLI responsibility split

The phase skills (`/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`, `/spec:init`) are agent-driven orchestrators. Every deterministic operation — kebab-case name validation, `.metadata.yaml` reads and writes, lifecycle transitions, capability and brief-pipeline resolution, artifact-completion checks, spec-merge preview, baseline conflict detection, delta merge, coherence validation, archive move — runs through the `specify` CLI. The skill markdown drives the agent-side work: eliciting user intent, reading brief bodies, writing artifacts, invoking plugin skills (e.g. `/omnia:crate-writer`), and rendering summaries.

When a skill currently does something deterministic in prose (parsing YAML, validating shape, computing topology, transitioning state), the right fix is to add a CLI verb and have the skill call it. The wrong fix is to make the skill smarter. The CLI surface the skills depend on is documented in [cli-contract.md](cli-contract.md).
