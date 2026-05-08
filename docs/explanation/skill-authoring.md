# Skill Authoring

This page is the long-form companion to the "Skill authoring conventions" section of [`.cursor/rules/project.mdc`](../../.cursor/rules/project.mdc). The rule file is the normative checklist; this page explains *why* each rule exists and walks through the trade-offs that shaped it.

The upstream specs are Anthropic's [Agent Skills overview](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview) and the [best-practices guide](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices). Where this repository diverges from upstream, the rationale is documented inline.

## Discovery model: two-stage loading

A SKILL.md is consumed in two stages:

1. **Stage 1 — always loaded.** Every SKILL.md frontmatter in the repository is loaded into the agent's context at session start. The agent uses these short blocks to decide *which* skill applies to the operator's request. Roughly ~100 tokens of metadata budget per skill, paid every session, every time.
2. **Stage 2 — loaded on trigger.** Once Claude has selected a skill, it reads the body. Sibling files linked from the body (`references/`, `examples/`, `author.md`, `verifier.md`, etc.) are read on demand as the body's instructions point Claude at them.

Two consequences follow.

**Stage 1 metadata is precious.** The session-wide cost scales linearly with the number of skills, so every byte of `description`, `argument-hint`, and `name` is paid by every operator on every turn. With ~29 skills in this repository, a 100-token metadata block per skill is roughly ~2,900 tokens of context spent before the operator has even typed a request. Every line of RFC-citation noise, layer-number jargon, or prose that only matters mid-execution multiplies across the whole catalogue and crowds out room for the operator's actual task.

**Stage 2 should layer.** The body is allowed to be longer than the frontmatter, but it is still always loaded once the skill triggers. Long-form rules, code-block examples, output templates, and edge-case enumerations belong in siblings the body links to — Anthropic calls this [progressive disclosure](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices#progressive-disclosure-patterns), and the existing skills in this repo follow it (see *Progressive disclosure in practice* below).

## Why `name` is plugin-qualified

The `name` field is not scoped per plugin — it is global across every SKILL.md the agent has loaded. Two skills with the same `name` collide, and a skill named `writer` competes for selection with every other writer-shaped skill in the catalogue.

The fix is mechanical: every skill name carries its plugin domain as a prefix, separated by `-`. Examples:

- `omnia-crate-writer`, `omnia-test-writer`, `omnia-code-reviewer`
- `vectis-core-writer`, `vectis-ios-reviewer`, `vectis-android-writer`
- `contract-openapi`, `contract-asyncapi`, `contract-json-schema`
- `client-sow-writer`
- `rt-wiretapper`, `rt-replay-writer`

The `spec/` plugin is the one exception: its skills use the `specify-` prefix (`specify-init`, `specify-define`, `specify-build`, `specify-merge`, `specify-drop`, `specify-extract`, `specify-analyze`, `specify-plan`, `specify-execute`). The operator-facing product name is "Specify"; the plugin directory and slash-command prefix `spec` are an internal artefact. Carrying `specify-` into the discovery namespace keeps the metadata aligned with the vocabulary operators already use. The full rename table is in RFC-10 §A.1 (`rfcs/archive/rfc-10-skills.md`).

The slash-command syntax is independent of `name`. Cursor's `/plugin:skill` form addresses the plugin directory and the skill subdirectory; renaming the frontmatter `name` does not move the slash command. `name` is a discovery handle, not a routing key.

## Why `description` carries both *what* and *when*

Anthropic's published rule is that descriptions must answer two questions:

1. *What* does this skill do?
2. *When* should an agent reach for it?

Stage 1 selection is exactly the moment the second question gets asked. A description that names only the deliverable (`"Generate a Statement of Work from Specify artifacts"`) does not tell Claude when to fire that skill versus a sibling. A description that includes the trigger (`"… Use when a delivery lead asks for a SoW from completed Specify artifacts, when exporting client deliverables from a slice directory, or when the user mentions 'sow-writer'."`) gives Claude an actionable test it can run against the operator's request.

Two cleanup rules apply globally:

- **No RFC citations in descriptions.** RFC-9 §2C and "Layer 4 umbrella" mean nothing to the discovery scorer. They belong in the body's overview.
- **Prefer single-line descriptions.** Block-scalar literals are allowed, but using them as a way to smuggle several paragraphs of prose into Stage 1 metadata is exactly the failure mode the 1024-character ceiling exists to prevent. If a description does not fit in ~250 characters, the missing detail is body content, not metadata.

### Examples — good

> "Reviews generated Omnia Rust WASM crates for security, error handling, WASM constraints, and code quality issues. Use when reviewing crates produced by `/omnia:crate-writer` or when the user mentions code review for a generated crate."

What: reviews Omnia Rust WASM crates for a named set of failure modes. When: after `/omnia:crate-writer` or on operator request. The trigger phrases match the words an operator would type.

> "Authors, imports, and verifies OpenAPI 3.1 HTTP API contracts for Specify changes, including path operations, request and response schemas, parameters, auth, examples, and baseline deltas. Use when a contracts build needs an HTTP API contract, when an operator supplies or asks for an OpenAPI document, or when verifying OpenAPI compatibility after a merge."

What: authors / imports / verifies OpenAPI documents, with an explicit list of what is in scope. When: three concrete triggers — a contracts build, an operator-supplied document, and the post-merge verification path. Note the format word `OpenAPI` appearing in both halves; that gives the discovery scorer something concrete to match.

> "Captures fixture data from a legacy TypeScript service before migration by adding wiretap code that records requests, responses, and side effects to JSON. Use when capturing fixture data from a legacy TypeScript service before migration, or when the user mentions `wiretapper`."

What: an explicit description of what the skill does to the source tree. When: the migration-prep trigger and the explicit-name fallback.

### Examples — bad

> "AI-powered code review for generated Rust crates, catching security issues and quality problems."

The "what" half is OK, but there is no "when". `AI-powered` is filler. The discovery scorer cannot tell this skill apart from a generic Rust linter.

> "Review code (RFC-9 §3B, Layer 4)."

RFC citations and layer numbers occupy Stage 1 budget without telling the scorer anything. There is no description of capability or trigger.

> A 480-character block-scalar literal that lists every flag the skill accepts and every artifact path it touches.

The capability list is body content. The flag enumeration is body content. The Stage 1 budget is being spent on Stage 2 detail.

## `argument-hint` is Cursor placeholder text, not a usage spec

Cursor renders `argument-hint` as a single line of grey placeholder text after the user types `/plugin:skill ` in chat. It is a *hint* about the primary positional argument, not a usage line.

The convention is:

- One short hint, naming the primary positional.
- `<arg>` for required, `[arg]` for optional.
- No `--flag` prefixes. No trailing `?` optionality marker. No `|` alternative pipes.

Examples after RFC-10:

| Skill | `argument-hint` |
|---|---|
| `/spec:init` | `<capability>` |
| `/spec:define` | `[description]` |
| `/spec:build` | `[slice-name]` |
| `/change:plan` | `<change-name>` |
| `/spec:extract` | `<source-path> <slice-dir>` |
| `/omnia:crate-writer` | `[crate-name]` |
| `/contract:openapi` | `[slice-dir]` |
| `/change:execute` | `[mode]` |

The complete set of secondary positionals each skill accepts moves into a body section called "Invocation". Slash-skill examples use positional arguments only; reserve `--flag` notation for underlying CLI commands such as `specify ... --format json`, not for `/plugin:skill` arguments.

The shape constraints are enforced by `make checks`. A SKILL.md that ships `argument-hint: "crate-name?"` fails the check because the trailing `?` is the old in-line-optional marker, replaced by the angle/square-bracket convention.

## Body-length ceiling

Anthropic's published guidance: SKILL.md body should stay under 500 lines, with longer content split into sibling files. RFC-10 codifies this as a hard ceiling enforced in `make checks`.

Why a ceiling at all: every line of the SKILL.md body is loaded into context the moment the skill triggers. A 1,200-line skill body crowds out the operator's request, the artefacts under inspection, and any other skill body that fires later. The model's attention is not free.

Why 500 specifically: Anthropic's number; we did not invent it. The lower the ceiling, the harder the pressure to factor; 500 lines is enough room for the algorithm spine plus the Critical Path block plus a moderate amount of inline prose, but not enough to absorb every example and edge-case forever.

Long-form material moves out of the body and into siblings linked one level deep. The body keeps the algorithm, the dispatch table (when relevant), the invocation block, and pointers to the depth.

## The Critical Path (Quick Reference) block

A SKILL.md ≥150 body lines opens with a `## Critical Path (Quick Reference)` section: 5–7 numbered or bulleted lines that name the algorithmic spine of the skill. Each bullet may link to the sibling file that owns the depth.

The pattern serves three audiences:

1. **An operator scanning to confirm the skill does what they expect.** A 7-bullet quick-reference is enough to disambiguate "did I pick the right skill" without reading 400 lines of prose.
2. **A future maintainer checking that a body change still respects the algorithm.** When the Critical Path drifts from the body, one or the other is wrong. The block is a load-bearing summary.
3. **Claude itself, when SKILL.md is loaded but a sibling hasn't been read yet.** The Critical Path tells Claude which siblings to consult and in what order. This is the difference between "Claude finishes the skill correctly" and "Claude wanders into a sibling file that does not apply to the current intent".

The 150-line trigger is a soft heuristic: short skills don't need the block (the body itself acts as the quick reference), but anything that approaches half the ceiling has earned one. Skills near the 500-line limit must add the block only together with an offsetting extraction — adding 20 lines of Critical Path on top of a 480-line body is a slow-motion ceiling violation.

## Progressive disclosure in practice

The pattern is in active use under several skills in this repository — both as exemplars to copy and as live targets when authoring new skills:

- `plugins/spec/skills/extract/SKILL.md` — Critical Path links to `business-logic.md`, `external-api.md`, `dependencies.md`, `observability.md`, `design-template.md`, `verification.md`.
- `plugins/change/skills/execute/SKILL.md` — Critical Path drives the loop semantics; per-mode and per-failure detail lives in siblings.
- `plugins/change/skills/plan/SKILL.md` — orchestrate / propose / refine modes each have their own sibling file.
- `plugins/spec/skills/merge/SKILL.md` — the success / failure / deferred paths are summarised in the Critical Path; the per-path prose is kept compact.
- `plugins/omnia/skills/code-reviewer/SKILL.md` — after RFC-10 Chunk 15: Critical Path + invocation; categories, team protocol, auto-fix, and output template each live in siblings (`categories.md`, `team-protocol.md`, `auto-fix.md`, `output.md`).
- `plugins/omnia/skills/crate-writer/SKILL.md` — after RFC-10 Chunk 16: hard rules and authority hierarchy factored into `rules.md`; SKILL.md retains the mode-dispatch table and artifact-mapping section.
- `plugins/contract/skills/openapi/SKILL.md`, `.../asyncapi/SKILL.md`, `.../json-schema/SKILL.md` — each opens with a format-specific Critical Path and dispatches to `author.md`, `importer.md`, `verifier.md` based on intent.

When in doubt, look at one of these and copy the shape.

### When to factor a sibling

A useful rule of thumb when a SKILL.md grows past ~300 body lines:

- If a section is **referenced from multiple skills**, factor it into a shared reference under `plugins/<plugin>/references/` (or `plugins/references/` if cross-plugin).
- If a section is **only consulted on a specific intent** (an operator chose "import" rather than "author"), factor it into a per-intent sibling (`importer.md`, `verifier.md`).
- If a section is a **template, table, or example library**, factor it into `references/` or `examples/` and link from the body.
- If a section is **load-bearing for every run** (a hard rules list a writer skill must consult on every invocation), keep it in the body — factoring it out trades brevity for an extra read every time the skill fires.

The default direction is "factor sooner rather than later". A 480-line SKILL.md with no Critical Path is harder to maintain than a 250-line SKILL.md plus three siblings, even if the total line count is similar.

## Worked example: the phase-outcome contract

The four phase skills (`/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`) used to carry near-identical sections titled "Phase outcome contract", "Journal entries during the run", and "Mutating the plan mid-run". The wording was intentionally normative — but the duplication was a drift hazard the next time `/change:execute` evolved the contract.

After RFC-10 Chunk 14, the parameterised contract lives once at [`plugins/spec/references/phase-outcome-contract.md`](../../plugins/spec/references/phase-outcome-contract.md). Each phase skill replaces its previous ~80-line section with a 4-line shim that names the shared reference and lists the phase's outcome-specific deltas (the success / failure / deferred semantics that vary by phase).

This is exactly the factor-out pattern progressive disclosure exists to enable:

- **What's in the SKILL.md:** the link to the shared contract and the phase-specific deltas. Always-loaded once the phase fires.
- **What's in the reference:** the three outcome values, the journal kinds, the plan-mutation allow/forbid table, and the verbatim-`summary` rule. Loaded only when a phase needs to consult the contract directly.
- **What's removed:** ~240 lines of duplicated normative prose across four files.

Drift is now physically impossible: editing the contract in one place is the only way to change it.

When you find yourself copying ≥30 lines of prose between two SKILL.md files, that is a candidate for the same factor-out. The reference goes under `plugins/<plugin>/references/` (or `plugins/references/` if it is cross-plugin); each consumer keeps a 4-line shim plus a link.

## Portability posture

Specify skills are authored first for Cursor plugins. The repository deliberately uses the Cursor marketplace layout, `/plugin:skill` slash-command routing, `argument-hint` placeholder text, Cursor tool names, MCP tool prefixes, and `<!-- skill: plugin:skill -->` body directives. Those pieces are part of the source profile because they are how the skills compose inside Cursor.

The source profile is still shaped by Anthropic Agent Skills guidance: concise `name` and `description`, progressive disclosure, a 500-line body ceiling, and small frontmatter. It is not, however, a claim that the repository can be copied unchanged into Claude Code or every Agent Skills host.

A Claude Code or upstream Agent Skills package should be generated as a separate compatibility profile. That profile can make host-specific choices without weakening the Cursor source contract:

- Strip or map Cursor-only fields such as `argument-hint`.
- Convert `<!-- skill: plugin:skill -->` delegation into host-native composition, or rewrite the body so it names the referenced skill and loading step explicitly.
- Replace Cursor-only tool names with the target host's tools.
- Add target-only metadata such as `disable-model-invocation`, `user-invocable`, `context`, or path activation fields where the target host benefits from them.

The source skills intentionally do not add `disable-model-invocation` to mutating workflow skills. `/spec:*` and `/change:*` skills are side-effecting by design, but their Cursor contract is explicit slash-command or pipeline invocation, with runtime guardrails and deterministic mutations routed through the `specify` CLI. Marking them non-model-invocable in the source profile would fight direct operator requests like "run build for this slice" and the plan-driven delegation flow. A Claude Code export may choose stricter activation metadata for those same skills; that is export policy, not source schema policy.

## Forbidden frontmatter

The Anthropic spec is permissive — it accepts a number of optional fields beyond the four this repository uses. RFC-10 narrows the surface explicitly:

- **`license`** — not part of the Anthropic SKILL.md spec. License is already declared in the plugin manifest (`plugins/<plugin>/.cursor-plugin/plugin.json`) and the repo root `LICENSE` file. Saved ~12 tokens per skill in always-loaded metadata across all skills.
- **`compatibility`** — environment requirements (system packages, network access). When relevant, document them in the body's "Prerequisites" or "Setup" section.
- **`metadata`** — arbitrary key-value mapping. The use cases are too open-ended to standardise; if information is worth carrying, it deserves a named field or body prose.
- **`disable-model-invocation`** — Claude Code / host-specific knob to hide the skill from auto-trigger. Source skills stay agent-invocable in Cursor; export profiles can add this field for mutating workflows if the target host needs it.
- **`when_to_use`** — Claude Code's appended-trigger field. Triggers belong in `description` per Anthropic's own guidance; carrying them twice doubles the metadata cost without doubling the signal.
- **`user-invocable`** — Claude Code's hide-from-`/`-menu knob. Source visibility is governed by Cursor plugin manifests and slash commands; export profiles can decide target-menu visibility.
- **`context`** — Claude Code / host-specific context attachment metadata. Source skills load durable context through body links and `<!-- skill: ... -->` directives instead.
- **`paths`** — Claude Code's auto-activation glob. Specify skills are invoked by slash command, by phase pipeline, or by direct trigger phrases — not by file pattern.

The forbidden list is enforced in [`.cursor/schemas/skill.schema.json`](../../.cursor/schemas/skill.schema.json) via `additionalProperties: false`. A skill shipping any of these keys fails `make checks`.

## Validation

`make checks` (running `scripts/checks.ts` via Deno) enforces the mechanical parts of this house style. The relevant invariants:

- **Name shape.** `name` matches `^[a-z][a-z0-9-]*$`, with `≤64` characters.
- **Plugin prefix.** `name` starts with the containing plugin's directory name plus `-`. The `spec/` plugin uses `specify-` per the override in `scripts/checks.ts`.
- **Global uniqueness.** No two SKILL.md files in the repo carry the same `name`.
- **Description length.** `description` is ≤1024 characters.
- **Argument-hint shape.** `argument-hint` contains no `?`, no `--`, and no `|`.
- **Slash invocation shape.** `/plugin:skill` examples use positional arguments only; leading double-dash option tokens after a slash skill are rejected.
- **Body length.** SKILL.md body (post-frontmatter) is ≤500 lines.
- **Critical Path.** SKILL.md bodies with ≥150 post-frontmatter lines include a `## Critical Path (Quick Reference)` block with 5–7 bullets or numbered items.
- **Forbidden keys.** No top-level `license`, `compatibility`, `metadata`, `disable-model-invocation`, `when_to_use`, `user-invocable`, `context`, or `paths`. Enforced by the `additionalProperties: false` clause in `.cursor/schemas/skill.schema.json`.

A skill that fails any of these checks will fail CI. When a check fires, the right fix is to bring the skill into compliance, not to relax the check; the rules are deliberately mechanical so `make checks` can keep them honest without operator review.

## Further reading

- [Agent Skills overview](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview) — Anthropic's high-level introduction to skills, frontmatter, and discovery.
- [Agent Skills best practices](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices) — the published patterns for naming, descriptions, body length, and progressive disclosure.
- [`docs/contributing/skill-anatomy.md`](../contributing/skill-anatomy.md) — the contributor-facing reference for skill directory structure and body sections.
- [`rfcs/archive/rfc-10-skills.md`](../../rfcs/archive/rfc-10-skills.md) — the RFC that landed this house style, including the rename tables and the alternatives that were considered and rejected.
