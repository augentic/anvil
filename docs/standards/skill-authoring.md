# Skill Authoring Standards

Mechanically enforced rules for every `SKILL.md` in this repository: frontmatter shape, body discipline, references hand-off, and what skills must never do. The rule file at [.cursor/rules/project.mdc](../../.cursor/rules/project.mdc#skill-authoring-conventions) is the normative checklist. This document captures the rules `make lint` enforces, the cross-cutting policies skills inherit by convention, and the long-form rationale (discovery model, why metadata is precious, the progressive-disclosure pattern, worked description examples, and the forbidden-frontmatter list) under `## Rationale` at the bottom.

This is a pre-1.0 codebase. There are no backward-compatibility constraints on skill shape, frontmatter, or wire envelopes — when a rule changes, the SKILL.md changes with it. "Phase 3.7 renamed it …" / "the v1.x verb was …" prose is deleted, not relocated; git history is the record, and the skill describes only what it does today. Migration prose only stays in a SKILL.md when the skill itself documents a real migration feature (e.g. the `typescript` source adapter consuming a legacy codebase).

## Description grammar

Each `SKILL.md` `description` field must:

- Lead with an **imperative verb** drawn from the curated allow-list in `specify_standards::framework::check::skill_frontmatter` (`IMPERATIVE_VERBS`). Extend the allow-list when a new verb is genuinely imperative; adding a verb is cheaper than rejecting a fine description.
- Contain a `Use when …` clause so the skill-discovery surface can match on intent rather than vocabulary. The clause describes a trigger condition (when an operator should reach for this skill over its siblings), not a restatement of what the skill does. "Use when an operator wants to X" is an anti-pattern; "Use when starting Y from scratch, not when resuming Z" is the shape.
- Stay **≤ 512 chars** total.

`SkillDescriptionGrammarCheck` and the skill schema enforce the three rules respectively. See `specify_standards::framework::check` modules for the implementation.

## Argument-hint grammar

Each `SKILL.md` `argument-hint:` value is a whitespace-separated sequence of tokens drawn from a fixed grammar (enforced by `SkillArgumentHintGrammarCheck` in `specify_standards::framework::check::skill_frontmatter`):

- `<name>` — required positional, kebab-case noun (e.g. `<slice-dir>`, `<change-name>`).
- `[name]` — optional positional (e.g. `[crate-name]`, `[mode]`).
- `<name>...` / `[name]...` — repeated positional (e.g. `<image-path>...`, `[file]...`).
- `<a|b|c>` / `[a|b|c]` — mutually exclusive alternatives.
- `--flag` — long flag (no value); when a flag carries a value, model the value as a sibling token: `--kind <kind>`.

Names are kebab-case (`[a-z][a-z0-9-]*` per alternative). Bare prose ("the slice name"), mixed punctuation (`<arg>: arg2`), trailing `?`, and short flags (`-f`) are rejected. The hint is the slash-command placeholder, not the full CLI signature — secondary arguments still belong in the body's "Invocation" section.

## Body caps

- **Body line count** ≤ **200 lines**. Strictly enforced — no per-file grandfathering.
- **Per-H2 section** ≤ **45 lines** (non-blank, non-comment). Depth migrates into `references/<topic>.md`, linked from the section, rather than letting individual sections sprawl. Strictly enforced — no per-file grandfathering.

The body cap is enforced by the declarative [`CORE-005`](../../adapters/shared/rules/core/CORE-005-skill-body-line-count.md) rule (via the `cardinality` reserved-kind interpreter); the per-section cap is enforced by `SkillSectionLineCount` (see `specify_standards::framework::check::skill_body`).

All caps are floors, not budgets — overflow means the relocate-to-`references/` pattern needs to fire, not that the cap should be raised. The 200 / 45 / 512 numbers are kept synchronized across scripts, schema, rules, and docs by `checkSkillNumericCaps`.

## References discipline

Long-form rules, code-block examples, output templates, and edge-case enumerations belong in siblings the SKILL.md body links to (Anthropic's [progressive disclosure](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices#progressive-disclosure-patterns) pattern). The SKILL.md keeps the Critical Path, the invocation surface, the dispatch table (when applicable), and the canonical decision points; sibling files (`references/`, `examples/`, topical files) carry the prose.

Push prose to `references/<topic>.md` under the plugin (runtime canonical) before raising any cap. Use [docs/](../../docs/) only for contributor-facing book prose, not for links agents must resolve at skill runtime. The relocate-to-`references/` pattern is the canonical response when a section approaches the 45-line ceiling.

## Skill body discipline

The frontmatter and body caps above are the floor. These additional rules tighten the SKILL.md as a navigable artifact:

1. **No restating frontmatter in the body.** `description` and `argument-hint` already render on every invocation; do not repeat them in the first H2 (or any other body section). Mechanically enforced by `checkNoFrontmatterRestatement`.
2. **`## Critical Path` is the table of contents.** When a skill body is split into siblings, the SKILL.md keeps the Critical Path, the invocation surface, the dispatch table (when applicable), and the canonical decision points. Sibling files (`references/`, `examples/`, topical files) carry the long-form rules, examples, templates, and edge-case prose. The Critical Path may take either of two forms: a flat 5–7 entry numbered/bullet list, or 5–7 `### N. Title` H3 step headings (when each step has its own concise body); duplicating both forms in the same body is the anti-pattern this rule eliminated.
3. **No historical design-record citations in skill bodies.** Implementation-history references in prose train operators on how the system was *built*, not how it works *today*. Delete the historical reference and cite current references from the skill body. Mechanically enforced by `checkNoRfcCitationsInSkillBody`.
4. **If present, `## Phase outcome contract` is a single-line link, not a paragraph.** Phase skills are *not* required to carry this section — none do today, and no predicate enforces its presence; the canonical contract lives in [`plugins/spec/references/phase-outcome-contract.md`](../../plugins/spec/references/phase-outcome-contract.md). When a skill does include the section, replace any opening prose with the single-line `> See [Phase outcome contract](../../references/phase-outcome-contract.md).` rather than restating the contract.

## Cross-cutting guardrails

Cross-cutting guardrails — the `.metadata.yaml` / slice-dir / plan-write rules that recur across skills — live in [`plugins/spec/references/guardrails.md`](../../plugins/spec/references/guardrails.md) (runtime canonical). SKILL.md files **link** to them; they do **not** restate them inline. The mdBook page [`skill-guardrails.md`](./skill-guardrails.md) is a stub pointer. Per-skill guardrails (don'ts that only apply to one skill) stay in the SKILL.md under a single `## Guardrails` (or `## Mode-specific guardrails`) H2; scattered IMPORTANT / Never / Critical scolding throughout the body trains agents to skim. Mechanically enforced by `checkOneGuardrailsBlockPerSkill`.

The canonical "skills MUST NOT" list:

- **Never hand-edit `.metadata.yaml`.** Every lifecycle transition flows through `specify slice transition` or `specify plan transition`.
- **Never `mkdir -p .specify/...`.** Slice and plan directories are minted by `specify slice create` / `specify plan create`; the CLI owns directory shape.
- **Never `mv` anything into `.specify/archive/`.** Archive moves are owned by `specify slice merge`, `specify slice transition <name> dropped`, and `specify plan archive`.
- **Never reimplement validation, adapter resolution, or merge logic in skill prose.** Those are deterministic operations owned by the CLI; see [cli-contract.md](cli-contract.md).
- **Never embed raw CLI envelope JSON in a SKILL.md body.** Link to [docs/reference/cli-output-shapes.md](../reference/cli-output-shapes.md) with a stable anchor instead.

## Brief authoring

Adapter briefs live at `adapters/targets/<name>/briefs/{shape,build,merge}.md` (target adapters) and `adapters/sources/<name>/briefs/{survey,extract}.md` (source adapters). They are markdown documents the agent reads when a phase skill (`/spec:build`, `/spec:refine`, `/spec:merge`) loads the adapter. They are **not** skills: they carry no `name` / `description` / `argument-hint` frontmatter, they are not loaded by Stage 1 discovery, and the Stage 2 line caps (200 body / 45 section) do not apply.

Briefs split into two roles:

- **Parent briefs** orchestrate. They declare bindings, mode dispatch, the phase order, cross-phase loops (verify-repair, remediation), and the stop-hint contract — then load phase sub-briefs by relative-link instruction. The CLI resolves only the parent path declared in `adapter.yaml`; the agent walks links into sub-briefs.
- **Phase sub-briefs** carry the operational body of one phase. They live under `adapters/targets/<name>/briefs/build/<phase>.md` (or deeper: `build/<platform>/<phase>.md` for per-platform targets) and `adapters/sources/<name>/briefs/extract/<axis>.md`.

The discipline:

1. **No frontmatter on briefs.** Briefs are not skills. They do not declare `name`, `description`, `argument-hint`, `id`, or any other YAML frontmatter — the loader resolves briefs by path from `adapter.yaml` and never reads frontmatter, so any leading `---` block is decoration that drifts and duplicates the body H1. Mechanically enforced by `BriefCheck` in `specify_standards::framework::check::brief`.
2. **Parent briefs cap at 150 non-blank lines (hard).** Parent briefs orchestrate; orchestration that needs more than 150 lines means a sub-brief is missing. Enforced by `BriefCheck` in `specify_standards::framework::check::brief`.
3. **Phase sub-briefs cap at 500 non-blank lines (soft warn) and 800 non-blank lines (hard fail).** Above 800, split into sub-phase briefs (`build/<phase>/<subphase>.md`) or move material to `plugins/<name>/references/`. Enforced by `BriefCheck`.
4. **References are cited via markdown links, never inlined.** Briefs use relative paths into `plugins/<name>/references/` so that broken links surface as `checkMarkdownLinks` failures. Inlining a template body in a brief defeats the cap discipline and removes the link-resolution safety net.
5. **Worked examples live under `plugins/<name>/references/examples/<flavour>/`.** Briefs cite paths like `examples/<flavour>/…`; they never inline an example. The `references/examples/` tree is exempt from brief size caps because it is not a brief.

The pattern that emerges:

```text
adapters/targets/<name>/briefs/
  shape.md                  parent: synthesis idiom guidance, <=150 LOC
  build.md                  parent: orchestrator, <=150 LOC
  merge.md                  parent: pre-merge gate, <=150 LOC
  build/<phase>.md          phase sub-brief, soft cap 500 / hard cap 800 LOC

plugins/<name>/references/
  <topic>.md                load-on-demand depth
  examples/<flavour>/...    worked examples (no size cap)
```

A 5th phase lands as one new `build/<phase>.md` file plus three lines added to the parent's phase-order list. The same shape works for source adapters (`adapters/sources/<name>/briefs/extract/<axis>.md`).

## Envelope examples and wire contract

CLI envelope shapes (the `envelope-version` + `data` / `error` wrapper) live in [docs/reference/cli-output-shapes.md](../reference/cli-output-shapes.md) with stable anchors. SKILL.md bodies **link** to the reference; they do not embed the envelope. `checkNoEnvelopeExamples` flags fenced ` ```json ` / ` ```jsonc ` blocks whose body looks like an envelope wrapper (`"envelope-version"` key, or `"ok"` + `"data"` / `"error"` pair). Body shapes that describe only a command's `data` payload — a one-line config snippet, an analyze sidecar — are still fine; the predicate is intentionally narrow.

The wire contract itself — exit codes, kebab-case `error` discriminants, the `envelope-version` floor — is owned by the CLI repo. See [cli-contract.md](cli-contract.md) for the surface skills depend on and the link to the authoritative exit-code table.

## Markdown style

- Do not hard-wrap prose in Markdown files solely for column width. Keep paragraphs and list-item prose on a single line unless the line break is semantically meaningful.
- Preserve intentional line breaks in frontmatter, tables, lists, blockquotes, and fenced code blocks.

## Skill / CLI responsibility split

The phase skills are agent-driven orchestrators; every deterministic operation runs through the `specify` CLI. The canonical statement of this split — the full operation list and the "never hand-edit `.metadata.yaml`" rule — lives in [`AGENTS.md` §"Skill / CLI responsibility split"](../../AGENTS.md#skill--cli-responsibility-split) and the CLI surface skills depend on is enumerated in [cli-contract.md](cli-contract.md); this page does not restate either, to keep a single source of truth.

The skill-authoring consequence is the rule unique to this page: when a skill currently does something deterministic in prose (parsing YAML, validating shape, computing topology, transitioning state), the right fix is to add a CLI verb and have the skill call it — not to make the skill smarter.

## Rationale

The upstream specs are Anthropic's [Agent Skills overview](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview) and the [best-practices guide](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices). Where this repository diverges, the reasoning is below.

**Discovery model: two-stage loading.** Every SKILL.md *frontmatter* in the repo is loaded into context at session start so the agent can pick which skill applies (Stage 1); the *body* is loaded once the skill triggers (Stage 2). Stage 1 metadata is therefore precious — with ~10 skills, ~100 tokens of metadata per skill is ~1,000 tokens spent before the operator has typed a request. Stage 2 should layer via [progressive disclosure](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices#progressive-disclosure-patterns): the body keeps the algorithm spine plus the Critical Path; long-form rules, code examples, output templates, and edge-case enumerations move into siblings the body links to. The `name` field is global across every loaded SKILL.md, which is why every name carries its plugin directory as a `<plugin>-` prefix (the `spec/` plugin uses `specify-` for product-name alignment).

**Why the 200-line body cap.** Every line of a SKILL.md body is loaded into context the moment the skill triggers. A 1,200-line skill crowds out the operator's request, the artifacts under inspection, and every other skill body that fires later. 200 specifically leaves room for the algorithm spine + Critical Path + a moderate amount of inline prose, but not enough to absorb every example, every flag re-documentation, and every edge case forever — the previous 400-line cap permitted "Critical Path quick reference" + parallel `## Steps` restatement; the 200-line cap forces a single canonical step list with the rest in siblings.

**Description examples — good.**

> "Build the active in-progress slice by driving the two-phase `specify slice build` verb and running its target adapter's build brief. Use when `/spec:execute` parks on a build failure, when running build standalone after `/spec:refine`, or to retry the brief after fixing a failing task."

What + when, with concrete triggers (`/spec:execute`, `/spec:refine`) the discovery scorer can match.

> "Authors, imports, and verifies OpenAPI 3.1 HTTP API contracts for Specify changes, including path operations, request and response schemas, parameters, auth, examples, and baseline deltas. Use when a contracts build needs an HTTP API contract, when an operator supplies or asks for an OpenAPI document, or when verifying OpenAPI compatibility after a merge."

The format word (`OpenAPI`) appears in both halves; three concrete triggers cover the operator-supplied, build-driven, and post-merge paths.

**Description examples — bad.**

> "AI-powered code review for generated Rust crates, catching security issues and quality problems."

What is fine; *when* is missing. `AI-powered` is filler — the scorer cannot tell this apart from a generic Rust linter.

> "Review code (per the internal §3B writer-protocol classification)."

Internal section citations and layer numbers occupy Stage 1 budget without telling the scorer anything; no adapter or trigger. Repo-history references do not belong in a discovery `description`.

**Forbidden frontmatter (and why).** The Anthropic spec is permissive; this repository narrows the surface explicitly. Enforced via `additionalProperties: false` in the CLI-embedded `schemas/authoring/skill.schema.json`, applied by `specify lint framework`.

- **`license`** — already declared in the plugin manifest and the repo `LICENSE`; not part of the Anthropic SKILL.md spec.
- **`compatibility`** — environment requirements belong in a body "Prerequisites" / "Setup" section.
- **`metadata`** — open-ended key-value bag; if a value is worth carrying it deserves a named field or body prose.
- **`disable-model-invocation`** — Claude Code knob. Source skills stay agent-invocable in Cursor; export adapters can add this for mutating workflows on stricter hosts.
- **`when_to_use`** — Claude Code's appended-trigger field. Triggers belong in `description` per Anthropic's own guidance; carrying them twice doubles the metadata cost without doubling the signal.
- **`user-invocable`** — Claude Code's hide-from-`/`-menu knob; Cursor source visibility is governed by plugin manifests and slash commands.
- **`context`** — Claude Code context-attachment metadata. Source skills load durable context through body links and `<!-- skill: ... -->` directives.
- **`paths`** — Claude Code auto-activation glob. Specify skills are invoked by slash command, by phase pipeline, or by direct trigger phrases — not by file pattern.

**Portability posture.** Source skills are Cursor-plugin-first; the Cursor-only surface (marketplace layout, `/plugin:skill` slash routing, `argument-hint` placeholder text, Cursor tool names, MCP tool prefixes, `<!-- skill: plugin:skill -->` directives) is part of the source contract because that is how skills compose inside Cursor. A Claude Code / upstream Agent Skills consumer should ship a separate compatibility *adapter* that strips or maps Cursor-only fields, replaces Cursor-only tool names, translates `<!-- skill: ... -->` delegation, and (where useful) adds target-only metadata such as `disable-model-invocation`, `user-invocable`, `context`, or `paths`. Source visibility for mutating workflows is governed at runtime by skill bodies and the `specify` CLI; export adapters may choose stricter activation policy.
