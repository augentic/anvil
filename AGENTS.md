# Augentic Plugins - Agent Instructions

## Cursor Cloud specific instructions

This is a **documentation/prompt-engineering repository**. The codebase consists of markdown skill definitions, reference docs, templates, and shell scripts. Generated Rust crates and Swift shells appear in downstream projects, not in this repository itself.

### Vocabulary

Two lifecycle nouns recur throughout this codebase. RFC-13 §Migration locked their meaning:

- **Slice** — the single unit that flows through the fixed `define → build → merge` loop. Each slice has its own proposal, specs, design, tasks, and merge step. Lives at `.specify/slices/<name>/`. Driven by `/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop` and the `specify slice *` CLI verbs.
- **Change** — the operator-defined umbrella that coordinates one or more slices through `change.md` + `plan.yaml`. Driven by `/change:plan`, `/change:execute`, and the `specify change *` CLI verbs (which include the `specify change plan *` subresource).

Use *slice loop* for the per-slice lifecycle; reserve *change* for the umbrella that owns `change.md` and `plan.yaml`.

### Workflow overview

Humans are expected to work through stock Specify:

- `/spec:init` (once per project)
- `/spec:define`
- `/spec:build`
- `/spec:merge`
- `/spec:drop`
- `/spec:extract` (extract Specify artifacts from existing source code)
- `/change:execute` (drive a change's `plan.yaml` through define → build → merge; RFC-2 Layer 2, fully landed — `dry-run` preview, supervised single-slice run, self-heal on startup, `loop` mode with terminal summary + SIGINT/SIGTERM handling, and `sources` execution wiring).
- `/change:plan` (author `plan.yaml` via the planning brief pipeline; RFC-2 Layer 3 + RFC-3a + RFC-3b — discovery through `/spec:analyze`, optional **sync-peers** when `registry.yaml` declares multiple projects (`specify workspace sync` + `workspace.md`), propose with glob or **manifest** scopes (Stage C), **project assignment** step for multi-repo plans (RFC-3b: infers `project` per entry from registry descriptions, writes via `specify change plan amend --project`), `.specify/plans/<name>/` artefacts archived with the plan; see [rfcs/archive/rfc-3a-monoliths.md](rfcs/archive/rfc-3a-monoliths.md) and [rfcs/archive/rfc-3b-platform.md](rfcs/archive/rfc-3b-platform.md)).
- `/change:plan <name> orchestrate` (Layer 4 umbrella mode that strings the cross-repo loop into one operator action: brief → registry validate → `/change:plan` (default mode) → `/change:execute loop` → `specify workspace push` → operator PR merge → `specify change finalize`; RFC-9 §2C + RFC-14 — composition only, idempotent on re-entry, opens/updates PRs but never merges them, supports `migrate-legacy` / `new-feature` / `update-existing` shapes through a single uniform sequence)

This repository provides specialist skills and references that support that workflow.

### Skill / CLI responsibility split

The phase skills (`/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`, `/spec:init`) are agent-driven orchestrators. Every deterministic operation — kebab-case name validation, `.metadata.yaml` reads and writes, lifecycle transitions, capability and brief-pipeline resolution, artifact-completion checks, spec-merge preview, baseline conflict detection, delta merge, coherence validation, archive move — runs through the `specify` CLI. The skill markdown drives the agent-side work: eliciting user intent, reading brief bodies, writing artifacts, invoking plugin skills (e.g. `/omnia:crate-writer`), and rendering summaries.

CLI surface the skills depend on:

- `specify init <capability>` — scaffold `.specify/`, resolve/cache the capability identifier (a bare name, `https://…` URL, or `file:///…` URI), and write `project.yaml` with `capability:` set. `--hub` (RFC-9 §1D) is the mutually exclusive alternative: it scaffolds a registry-only platform hub whose `project.yaml` carries only `hub: true` (the `capability:` field is omitted). `specify init` invoked with neither (or both) errors with `init-requires-capability-or-hub`. See [RFC-13 §Migration "Hub project shape"](rfcs/archive/rfc-13-extensibility.md#migration).
- `specify status` — project dashboard summarising registry, active change, and active slices (single-slice view lives at `specify slice status <name>`).
- `specify slice {create, list, status, transition, touched-specs, overlap, archive, drop, validate, merge {preview, conflict-check, run}, task {progress, mark}, outcome {set, show}, journal {append, show}}` — every per-slice verb (renamed from `specify change *` by RFC-13 §3.2). `outcome set` stamps the `.metadata.yaml:outcome` that `/change:execute` reads; `journal append` writes `question` / `failure` / `recovery` entries into `journal.yaml`.
- `specify change plan {create, validate, doctor, next, status, add, amend, transition, archive, lock}` — plan CRUD and lifecycle. `create` scaffolds an empty plan; `add` appends an entry; `doctor` is a strict superset of `validate` with cycle / orphan-source / stale-clone / unreachable-entry diagnostics; `lock {acquire, release, status}` manages `.specify/plan.lock` for `/change:execute`.
- `specify change {create, show, finalize}` — operator brief at `change.md` plus the canonical closure verb (RFC-9 §4C; replaces the v1.x `specify change *` group, which was renamed to `specify change *` by RFC-13 §3.5). `create` was renamed from v1 `init`; `finalize` confirms every per-project PR has merged before archiving.
- `specify registry {add, remove, show, validate}` — platform registry at `registry.yaml`. `add` and `remove` were added by RFC-9 §2A; both validate the resulting shape (including the `description-missing-multi-repo` invariant) after the write.
- `specify workspace {sync, status, push}` — `sync` materialises `.specify/workspace/<peer>/` for multi-repo planning and selected execution preparation; `push` transports prepared `specify/<change-name>` branches and creates/updates PRs only. `specify workspace merge` has been removed and must not be called by skills; operators merge through the forge UI or explicit `gh pr merge`, then `specify change finalize` verifies remote PR state.
- `specify capability {resolve, check, pipeline}` — capability resolution and brief topology (renamed from `specify schema {resolve, check, pipeline}` by RFC-13 §Migration).

Today the per-slice verbs live under `specify slice *` and the umbrella verbs live under `specify change *`.

Never hand-edit `.metadata.yaml`, never `mkdir -p .specify/...`, and never `mv` anything into `.specify/archive/`. Route through the CLI — it enforces the legal set of lifecycle states and validates inputs in one place for humans, agents, and CI alike.

### Contract skills

The contract plugin provides format-first specialist skills for API contract generation and validation. Each skill carries author / import / verify intents internally and dispatches via its own intent table:

- `/contract:openapi` — author, import, or verify HTTP / resource-style contracts (OpenAPI 3.1)
- `/contract:asyncapi` — author, import, or verify evented / pub-sub / streaming contracts (AsyncAPI 3.0)
- `/contract:json-schema` — author, import, or verify reusable payload schemas (JSON Schema)

Each skill exposes the same three intents through sibling files: `author.md` (generate or extend), `importer.md` (normalise an external document), and `verifier.md` (internal consistency and merge-time baseline validation via `--mode cross-project`). These skills are invoked by the `contracts` brief in the define pipeline (the brief id, the `contracts@v1` capability, and the `contracts/` baseline directory keep their original names — `contract` is the Cursor plugin / slash-command surface only). The brief is present in the `contracts` capability (for dedicated contract slices) and in the Omnia and Vectis capabilities (for alignment validation during implementation slices). Cross-project consumer-impact classification is exposed separately as `specify compatibility`.

The matching CLI surface is the declared `contract` WASI tool, run through `specify tool run contract -- "$PROJECT_ROOT/contracts" --format json` (RFC-13 §"Merge and adoption contract" + RFC-15). It walks a baseline `contracts/` directory and runs the SemVer + id-format + cross-repo id-uniqueness checks (RFC-12 §"CLI surface"), exiting `0` clean / `1` findings / `2` tool or invocation error. The pre-RFC-13 in-binary `specify contract { list, validate }` family was retired in chunk 2.7 when contracts became a first-party capability owning its own validation behavior; the contracts capability merge brief now shells out through `specify tool run` as the post-merge baseline gate.

### Plan-driven loop (RFC-2, all three layers landed)

When a change is coordinated through a `plan.yaml`, the recommended path is:

1. **Author.** `/change:plan <change-name> source <key>=<path-or-url> ...` — Layer 3 skill runs the planning brief pipeline, optionally **sync-peers** + `workspace.md` when the registry is multi-project, then `specify change plan create` + one `specify change plan add` per accepted slice (globs or `--scope-manifest` per RFC-3a Stage C). Plan-time sync-peers is discovery-oriented and may sync all registered peers.
2. **Execute.** `/change:execute loop` — Layer 2 driver that repeatedly picks `specify change plan next`, prepares only the selected entry's project slot on exact branch `specify/<change-name>` when `project` is set, runs `/spec:define → /spec:build → /spec:merge`, reads the phase outcome off `.metadata.yaml`, and transitions the plan entry to `done` / `failed` / `blocked`. Exits on `all-done`, `stuck`, self-heal halt, or SIGINT/SIGTERM.
3. **Archive.** `specify change plan archive` sweeps `plan.yaml` and the `.specify/plans/<name>/` authoring trail into `.specify/archive/plans/<YYYYMMDD>-<name>/`.

Hand-driven fallback (RFC-2 Layer 1): skip `/change:plan` and `/change:execute`, author `plan.yaml` entry-by-entry with `specify change plan {create, add, amend}`, and drive the loop yourself via `specify change plan next → transition in-progress → /spec:define → /spec:build → /spec:merge → transition done`.

The phase skills themselves stay unaware of the plan — they operate slice-by-slice. Plan *entries* are only ever written via `specify change plan add` / `specify change plan amend`; plan *status* is only ever written via `specify change plan transition`. A phase that discovers a neighbouring slice mid-run (e.g. a define brief uncovering a bug fix that should be tracked) may shell out to `specify change plan add` / `specify change plan amend` — the same commands humans run. See [rfcs/archive/rfc-2-execution.md](rfcs/archive/rfc-2-execution.md) for the full design.

### Commands

All commands are run from the repository root:

- **`make checks`** -- runs `scripts/checks.ts` via Deno for documentation and workflow consistency checks.
- **`make test`** -- runs the direct cross-repo Deno acceptance test. It drives a temp hub, two fixture repos, fake `gh`/SSH, plan, execute, push, and finalize through the real `specify` binary; it skips cleanly when no suitable binary is available.
- **`make use-local-plugins`** -- use local plugins from the working tree for development/testing.
- **`make use-team-plugins`** -- use Augentic marketplace plugins (reload Cursor after either).

The cross-repo test requires a built `specify` binary. Set `SPECIFY_BIN=/absolute/path/to/specify-cli/target/release/specify` (the system PATH `specify` is typically the older v0.1.0 install and the test will skip against it). Full operator guide: [docs/contributing/acceptance.md](docs/contributing/acceptance.md).

### Markdown style

- Do not hard-wrap prose in Markdown files solely for column width. Keep paragraphs and list-item prose on a single line unless the line break is semantically meaningful.
- Preserve intentional line breaks in frontmatter, tables, lists, blockquotes, and fenced code blocks.

### Skill authoring

Every `SKILL.md` in this repository follows the house style codified in [.cursor/rules/project.mdc](.cursor/rules/project.mdc#skill-authoring-conventions); the long-form rationale (discovery model, why metadata is precious, examples of good/bad descriptions, the progressive-disclosure pattern, and the forbidden-frontmatter list) lives at [docs/contributing/skill-authoring.md](docs/contributing/skill-authoring.md). This section captures the mechanically enforced rules.

This is a pre-1.0 codebase. There are no backward-compatibility constraints on skill shape, frontmatter, or wire envelopes — when a rule changes, the SKILL.md changes with it. "Pre-RFC-N this used to be …" / "Phase 3.7 renamed it …" / "the v1.x verb was …" prose belongs in [docs/explanation/decision-log.md](docs/explanation/decision-log.md) and [docs/explanation/release-notes.md](docs/explanation/release-notes.md), not in the skill that operators read every day. Migration prose only stays in a SKILL.md when the skill itself documents a real legacy-migration feature (e.g. `/spec:extract` migrating off a legacy codebase).

### Description grammar

Each `SKILL.md` `description` field must:

- Lead with an **imperative verb** drawn from the curated allow-list in [scripts/checks/skill_frontmatter.ts](scripts/checks/skill_frontmatter.ts) (`IMPERATIVE_VERBS`). Extend the allow-list when a new verb is genuinely imperative; adding a verb is cheaper than rejecting a fine description.
- Contain a `Use when …` clause so the skill-discovery surface can match on intent rather than vocabulary. The clause describes a trigger condition (when an operator should reach for this skill over its siblings), not a restatement of what the skill does. "Use when an operator wants to X" is an anti-pattern; "Use when starting Y from scratch, not when resuming Z" is the shape.
- Stay **≤ 512 chars** total.

`checkDescriptionStartsWithVerb`, `checkDescriptionHasUseWhen`, and `checkDescriptionLength` enforce the three rules respectively.

### Argument-hint grammar

Each `SKILL.md` `argument-hint:` value is a whitespace-separated sequence of tokens drawn from a fixed grammar (enforced by `checkArgumentHintGrammar` in [scripts/checks/skill_frontmatter.ts](scripts/checks/skill_frontmatter.ts)):

- `<name>` — required positional, kebab-case noun (e.g. `<slice-dir>`, `<change-name>`).
- `[name]` — optional positional (e.g. `[crate-name]`, `[mode]`).
- `<name>...` / `[name]...` — repeated positional (e.g. `<image-path>...`, `[file]...`).
- `<a|b|c>` / `[a|b|c]` — mutually exclusive alternatives.
- `--flag` — long flag (no value); when a flag carries a value, model the value as a sibling token: `--kind <kind>`.

Names are kebab-case (`[a-z][a-z0-9-]*` per alternative). Bare prose ("the slice name"), mixed punctuation (`<arg>: arg2`), trailing `?`, and short flags (`-f`) are rejected. The hint is the slash-command placeholder, not the full CLI signature — secondary arguments still belong in the body's "Invocation" section.

### Body caps

- **Body line count** ≤ **250 lines** (`checkBodyLineCount`). New skills must comply; existing skills that still exceed the cap are grandfathered via per-file `bodyLineCount` baselines in `scripts/standards-allowlist.toml` and are expected to ratchet down with each touch.
- **Per-H2 section** ≤ **60 lines** (non-blank, non-comment) (`checkSectionLineCount`). Depth migrates into `references/<topic>.md`, linked from the section, rather than letting individual sections sprawl. Per-file `sectionLineCount` baselines in `scripts/standards-allowlist.toml` grandfather the irreducible remainder; new sections still fail fast.

All caps are floors, not budgets — overflow means the relocate-to-`references/` pattern needs to fire, not that the cap should be raised.

### Cross-cutting guardrails

Cross-cutting guardrails — the `.metadata.yaml` / slice-dir / plan-write rules that recur across skills — live in [plugins/references/guardrails.md](plugins/references/guardrails.md). SKILL.md files **link** to them; they do **not** restate them inline. Per-skill guardrails (don'ts that only apply to one skill) stay in the SKILL.md under a single `## Guardrails` (or `## Mode-specific guardrails`) H2; scattered IMPORTANT / Never / Critical scolding throughout the body trains agents to skim. Mechanically enforced by `checkOneGuardrailsBlockPerSkill`.

### Envelope examples

CLI envelope shapes (the `envelope-version` + `data` / `error` wrapper) live in [plugins/references/cli-output-shapes.md](plugins/references/cli-output-shapes.md) with stable anchors. SKILL.md bodies **link** to the reference; they do not embed the envelope. `checkNoEnvelopeExamples` flags fenced ` ```json ` / ` ```jsonc ` blocks whose body looks like an envelope wrapper (`"envelope-version"` key, or `"ok"` + `"data"` / `"error"` pair). Body shapes that describe only a command's `data` payload — a one-line config snippet, an analyze sidecar — are still fine; the predicate is intentionally narrow.

### Skill body discipline

The frontmatter and body caps above are the floor. These additional rules tighten the SKILL.md as a navigable artifact:

1. **No restating frontmatter in the body.** `description` and `argument-hint` already render on every invocation; do not repeat them in the first H2 (or any other body section). Mechanically enforced by `checkNoFrontmatterRestatement`.
2. **`## Critical Path` is the table of contents.** When a skill body is split into siblings, the SKILL.md keeps the Critical Path, the invocation surface, the dispatch table (when applicable), and the canonical decision points. Sibling files (`references/`, `examples/`, topical files) carry the long-form rules, examples, templates, and edge-case prose. The Critical Path may take either of two forms: a flat 5–7 entry numbered/bullet list, or 5–7 `### N. Title` H3 step headings (when each step has its own concise body); duplicating both forms in the same body is the anti-pattern this rule eliminated.
3. **No RFC citations in skill bodies.** `RFC-N` references in prose train operators on how the system was *built*, not how it works *today*. Move them to a trailing `## References` block as `[RFC-N](rfcs/...)` links, or to [docs/explanation/decision-log.md](docs/explanation/decision-log.md). Mechanically enforced by `checkNoRfcCitationsInSkillBody`.
4. **`## Phase outcome contract` is a single-line link, not a paragraph.** Replace the canonical opening prose with `> See [Phase outcome contract](../../references/phase-outcome-contract.md).` Mechanically enforced by `checkNoPhaseOutcomeContractRestatement`.

### Mechanical enforcement

`make checks` runs [scripts/checks.ts](scripts/checks.ts), a thin orchestrator over the per-concern modules under [scripts/checks/](scripts/checks/) (`links.ts`, `capability.ts`, `tools.ts`, `plugins.ts`, `skill_frontmatter.ts`, `skill_body.ts`, `skill_discipline.ts`, `prose.ts`, `scenarios.ts`, `codex.ts`, `docs_quality.ts`). Per-predicate per-file baselines for the skill-body discipline live in [scripts/standards-allowlist.toml](scripts/standards-allowlist.toml); a live count strictly greater than its baseline fails CI. New files start clean (missing entries default to 0).

**Ratchet** — any PR that touches a skill is expected to reduce its baselines where it can. A predicate baseline is grandfathering, not a license; raising a number requires a justification in the PR description.

| Predicate | What it counts |
|---|---|
| `checkArgumentHintGrammar` | Each whitespace-separated token in `argument-hint:` matches the canonical grammar (`<name>`, `[name]`, trailing `...`, `<a|b>`, `[a|b]`, `--flag`). |
| `checkBodyLineCount` | SKILL.md body line count, hard cap **250 lines**. Per-file `bodyLineCount` baselines in `scripts/standards-allowlist.toml` grandfather oversized files. |
| `checkDescriptionHasUseWhen` | SKILL.md `description` contains a `Use when …` clause. |
| `checkDescriptionLength` | SKILL.md `description` length, hard cap **512 chars**. |
| `checkDescriptionStartsWithVerb` | SKILL.md `description` starts with an imperative verb from the curated allow-list in `scripts/checks/skill_frontmatter.ts`. |
| `checkNoEnvelopeExamples` | Fenced ` ```json` / ` ```jsonc ` blocks whose body looks like a CLI envelope wrapper. Link to `plugins/references/cli-output-shapes.md` instead. |
| `checkNoFrontmatterRestatement` | Frontmatter `description` value re-appearing under the first H2. |
| `checkNoPhaseOutcomeContractRestatement` | Restated phase-outcome contract paragraph. Use the one-line link form. |
| `checkNoRfcCitationsInSkillBody` | `RFC[- ]?\d+` in skill body, fenced code excluded, `rfcs/` archive links excluded. |
| `checkOneGuardrailsBlockPerSkill` | Count of `## Guardrails` and `## Mode-specific guardrails` headings — exactly one per SKILL.md. |
| `checkOperationalVocabulary` | Active prose using retired slice paths, top-level CLI commands, or pre-cutover umbrella nouns outside archived/historical material. |
| `checkSectionLineCount` | Per-H2 section line count, hard cap **60 lines** (non-blank, non-comment). |
| `checkSkillNumericCaps` | Keeps the 512/250/60 caps synchronized across scripts, schema, rules, and docs. |

### Gotchas

- In a fresh clone, run `/spec:init` before using other `/spec:*` commands. The workflow skills expect the `.specify/` project structure to exist.
- `checks.ts` enforces documentation consistency; if you remove or rename workflow terms, update the checks in the same change.
- Some skills use symlinks to share reference documents from `plugins/references/`. If a symlink target is removed, the skill's documentation may reference content that no longer resolves.
