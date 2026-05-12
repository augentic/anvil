# Augentic Plugins - Agent Instructions

## Cursor Cloud specific instructions

This is a **documentation/prompt-engineering repository**. The codebase consists of markdown skill definitions, reference docs, templates, and shell scripts. Generated Rust crates and Swift shells appear in downstream projects, not in this repository itself.

### Vocabulary

Two lifecycle nouns recur throughout this codebase:

- **Slice** — the single unit that flows through the fixed `define → build → merge` loop. Each slice has its own proposal, specs, design, tasks, and merge step. Lives at `.specify/slices/<name>/`. Driven by `/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop` and the `specify slice *` CLI verbs.
- **Change** — the operator-defined umbrella that coordinates one or more slices through `change.md` + `plan.yaml`. Driven by `/change:plan`, `/change:execute`, and the `specify change *` CLI verbs (which include the `specify change plan *` subresource).

Use *slice loop* for the per-slice lifecycle; reserve *change* for the umbrella that owns `change.md` and `plan.yaml`.

### Workflow overview

Slash commands operators reach for, in roughly the order they appear in a project's life:

- `/spec:init` — scaffold `.specify/`, run once per project.
- `/spec:extract` — extract Specify artifacts from existing source code.
- `/spec:define` — author a new slice (proposal, design, specs, tasks).
- `/spec:build` — implement a slice's tasks.
- `/spec:merge` — fold a slice's deltas into the baseline and archive it.
- `/spec:drop` — abandon a slice without merging.
- `/change:plan` — author a change's `plan.yaml` via the planning brief pipeline; in multi-project hubs `sync-peers` + `workspace.md` precede the propose step.
- `/change:plan <name> orchestrate` — umbrella mode that strings the cross-repo loop into one operator action: brief → plan → execute → push → operator PR merge → finalize. Opens/updates PRs, never merges them.
- `/change:execute` — drive a change's `plan.yaml` through define → build → merge; supports `dry-run`, single-slice supervised run, and `loop` mode with self-heal and SIGINT/SIGTERM handling.

For the four-layer composition (CLI primitives → slice lifecycle → plan & drive → change orchestration) and the rename trail from earlier verb names, see [docs/explanation/decision-log.md](docs/explanation/decision-log.md).

### Skill / CLI responsibility split

Phase skills (`/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`, `/spec:init`) are agent-driven orchestrators. Every deterministic operation — kebab-case validation, `.metadata.yaml` reads and writes, lifecycle transitions, capability and brief-pipeline resolution, artifact-completion checks, spec-merge preview, baseline conflict detection, delta merge, coherence validation, archive move — runs through the `specify` CLI. Skill markdown drives the agent-side work: eliciting user intent, reading brief bodies, writing artifacts, invoking plugin skills (e.g. `/omnia:crate-writer`), and rendering summaries.

The CLI surface skills depend on is documented in the [`specify` `--help`](https://github.com/augentic/specify-cli) output. The headline groups: `specify init`, `specify status`, `specify slice {…}` (per-slice verbs), `specify change plan {…}` (plan CRUD + lifecycle), `specify change {create, show, finalize}` (operator brief + canonical closure), `specify registry {add, remove, show, validate}`, `specify workspace {sync, status, push}`, `specify capability {resolve, check, pipeline}`, and `specify tool run` (WASI tool dispatch — `contract`, `vectis`, …).

Never hand-edit `.metadata.yaml`, never `mkdir -p .specify/...`, and never `mv` anything into `.specify/archive/`. Route through the CLI — it enforces the legal set of lifecycle states and validates inputs in one place for humans, agents, and CI alike.

### Contract skills

The contract plugin provides format-first specialist skills for API contract generation and validation. Each skill carries author / import / verify intents internally and dispatches via its own intent table:

- `/contract:openapi` — author, import, or verify HTTP / resource-style contracts (OpenAPI 3.1).
- `/contract:asyncapi` — author, import, or verify evented / pub-sub / streaming contracts (AsyncAPI 3.0).
- `/contract:json-schema` — author, import, or verify reusable payload schemas (JSON Schema).

The matching CLI surface is the declared `contract` WASI tool, run via `specify tool run contract -- "$PROJECT_ROOT/contracts" --format json`. Cross-project consumer-impact classification is exposed separately as `specify compatibility`.

### Plan-driven loop

`/change:plan` authors the plan, `/change:execute loop` drives it, `specify change plan archive` sweeps it. Plan *entries* are only ever written via `specify change plan add` / `specify change plan amend`; plan *status* is only ever written via `specify change plan transition`. The phase skills themselves stay unaware of the plan — they operate slice-by-slice. Hand-driven fallback: skip `/change:plan` and `/change:execute` and drive the loop yourself via `specify change plan next → transition in-progress → /spec:define → /spec:build → /spec:merge → transition done`.

### Commands

All commands are run from the repository root:

- `make checks` — runs `scripts/checks.ts` via Deno for documentation and workflow consistency checks.
- `make test` — direct cross-repo Deno acceptance test (skips cleanly when no suitable binary is available).
- `make use-local-plugins` / `make use-team-plugins` — choose plugin source (reload Cursor after either).

The cross-repo test requires a built `specify` binary. Set `SPECIFY_BIN=/absolute/path/to/specify-cli/target/release/specify` (the system PATH `specify` is typically the older v0.1.0 install and the test will skip against it). Full operator guide: [docs/contributing/acceptance.md](docs/contributing/acceptance.md).

### Markdown style

- Do not hard-wrap prose in Markdown files solely for column width. Keep paragraphs and list-item prose on a single line unless the line break is semantically meaningful.
- Preserve intentional line breaks in frontmatter, tables, lists, blockquotes, and fenced code blocks.

### Skill authoring

Long-form rationale (discovery model, why metadata is precious, examples of good/bad descriptions, progressive-disclosure pattern, forbidden-frontmatter list) lives at [docs/contributing/skill-authoring.md](docs/contributing/skill-authoring.md); the normative checklist is [docs/standards/skill-authoring.md](docs/standards/skill-authoring.md) and [.cursor/rules/project.mdc](.cursor/rules/project.mdc#skill-authoring-conventions). This section captures the rules `make checks` enforces.

This is a pre-1.0 codebase. There are no backward-compatibility constraints on skill shape, frontmatter, or wire envelopes — when a rule changes, the SKILL.md changes with it. Migration prose ("Pre-RFC-N this used to be …", "Phase 3.7 renamed it …", "the v1.x verb was …") belongs in [docs/explanation/decision-log.md](docs/explanation/decision-log.md), not in skills.

**Description grammar.** Each `description` (a) starts with an imperative verb from the curated allow-list in [scripts/checks/skill_frontmatter.ts](scripts/checks/skill_frontmatter.ts) (`IMPERATIVE_VERBS`); (b) contains a `Use when …` clause describing a trigger condition, not a restatement ("Use when starting Y from scratch, not when resuming Z" — not "Use when an operator wants to X"); (c) stays ≤ **512 chars**.

**Argument-hint grammar.** Each `argument-hint:` value is a whitespace-separated sequence of tokens drawn from a fixed grammar (enforced by `checkArgumentHintGrammar`): `<name>` (required positional), `[name]` (optional), trailing `...` (repeated), `<a|b|c>` / `[a|b|c]` (alternatives), `--flag` (long flag; value-bearing flags model the value as a sibling token `--kind <kind>`). Names are kebab-case (`[a-z][a-z0-9-]*`). Bare prose, mixed punctuation, trailing `?`, and short flags are rejected.

**Body caps.** SKILL.md body ≤ **200 lines** and per-H2 section ≤ **45 lines** non-blank, non-comment (`checkBodyAndSectionLineCounts`). Caps are floors — overflow means push depth into `references/<topic>.md`, not raise the cap. Pre-existing oversized files are grandfathered via per-file baselines in [scripts/standards-allowlist.toml](scripts/standards-allowlist.toml) and are expected to ratchet down with each touch.

**Cross-cutting guardrails.** The recurring `.metadata.yaml` / slice-dir / plan-write rules live in [plugins/references/guardrails.md](plugins/references/guardrails.md). SKILL.md files **link** to them; they do not restate them inline. Per-skill guardrails (don'ts that only apply to one skill) stay in a single `## Guardrails` (or `## Mode-specific guardrails`) H2; scattered IMPORTANT / Never / Critical scolding trains agents to skim. Enforced by `checkOneGuardrailsBlockPerSkill`.

**Envelope examples.** CLI envelope shapes (flat `envelope-version` + body keys on success; `error` / `message` / `exit-code` on failure — no `ok`, no `data` wrapper) live in [plugins/references/cli-output-shapes.md](plugins/references/cli-output-shapes.md). The reference is regenerated from the CLI's `tests/fixtures/` via `make doc-envelopes`. SKILL.md bodies **link** to the reference; `checkNoEnvelopeExamples` flags any fenced ` ```json` / ` ```jsonc ` block whose body carries an `"envelope-version"` key (or, for legacy embeddings, pairs `"ok"` with `"data"` / `"error"`).

**Skill body discipline.** `## Critical Path` is the table of contents: sibling files carry the long-form prose; the body keeps the Critical Path, invocation surface, dispatch table (when applicable), and canonical decision points. Critical Path is either a flat 5–7 entry list or 5–7 `### N. Title` H3 step headings — never both (`checkNoStepBodyDuplicatesCriticalPath`). Don't restate frontmatter under the first H2, don't cite RFC numbers inline, and use `> See [Phase outcome contract](../../references/phase-outcome-contract.md).` rather than restating the contract — these are review-time concerns, not predicates.

### Mechanical enforcement

`make checks` runs [scripts/checks.ts](scripts/checks.ts), a thin orchestrator over per-concern modules under [scripts/checks/](scripts/checks/) (`links.ts`, `capability.ts`, `tools.ts`, `plugins.ts`, `skill_frontmatter.ts`, `skill_body.ts`, `prose.ts`, `scenarios.ts`, `codex.ts`, `docs_quality.ts`). Per-predicate per-file baselines live in [scripts/standards-allowlist.toml](scripts/standards-allowlist.toml); a live count strictly greater than its baseline fails CI.

**Ratchet** — any PR that touches a skill is expected to reduce its baselines where it can. A baseline is grandfathering, not a license; raising a number requires a justification in the PR description.

| Predicate | What it counts |
|---|---|
| `checkArgumentHintCoversBodyArguments` | Every `$VAR_NAME` reference in the SKILL.md body resolves to a kebab-case token in `argument-hint:` (e.g. `$SOURCE_PATH` ↔ `<source-path>`), or is defined inline as `$VAR = ...` in a body code block. `$ARGUMENTS` and `$ARGUMENTS[N]` are framework-provided and skipped. |
| `checkArgumentHintGrammar` | Each whitespace-separated token in `argument-hint:` matches the canonical grammar. |
| `checkBodyAndSectionLineCounts` | SKILL.md body line count, hard cap **200 lines**; per-H2 section line count, hard cap **45 lines** (non-blank, non-comment). |
| `checkDescriptionHasUseWhen` | SKILL.md `description` contains a `Use when …` clause. |
| `checkDescriptionLength` | SKILL.md `description` length, hard cap **512 chars**. |
| `checkDescriptionStartsWithVerb` | SKILL.md `description` starts with an imperative verb from the curated allow-list in `scripts/checks/skill_frontmatter.ts`. |
| `checkNoEnvelopeExamples` | Fenced ` ```json` / ` ```jsonc ` blocks whose body looks like a full CLI envelope (carries `"envelope-version"`, or pairs `"ok"` with `"data"` / `"error"`). |
| `checkNoStepBodyDuplicatesCriticalPath` | Whitespace-normalised verbatim duplication between an entry in `## Critical Path` and any line under `## Process`. |
| `checkOperationalVocabulary` | Active prose using retired slice paths, top-level CLI commands, or pre-cutover umbrella nouns outside archived/historical material. |
| `checkSkillNumericCaps` | Keeps the 512/200/45 caps synchronized across scripts, schema, rules, and docs. |

### Gotchas

- In a fresh clone, run `/spec:init` before using other `/spec:*` commands. The workflow skills expect the `.specify/` project structure to exist.
- `checks.ts` enforces documentation consistency; if you remove or rename workflow terms, update the checks in the same change.
- Some skills use symlinks to share reference documents from `plugins/references/`. If a symlink target is removed, the skill's documentation may reference content that no longer resolves.

### Related coding standards

- CLI binary and crate conventions (errors, DTOs, hint colocation, brevity) live in the CLI repo's [AGENTS.md](https://github.com/augentic/specify-cli/blob/main/AGENTS.md) and [docs/standards/](https://github.com/augentic/specify-cli/blob/main/docs/standards/). Skills that shell out to `specify` rely on the kebab-case `error` discriminants documented there.
