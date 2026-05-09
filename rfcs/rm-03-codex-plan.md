# RM-03 Codex Implementation Plan

> Source: [`rm-03-codex.md`](rm-03-codex.md) and roadmap item RM-03 in [`roadmap.md`](roadmap.md).
> Goal: implement the codex rule format as a stable, capability-distributed, project-resolved rule surface for generators, reviewer skills, and future `specify review`.

## Planning Principles

- Keep deterministic parsing, validation, resolution, and JSON export in `specify-cli`.
- Keep human-authored first-party rule content in the `specify` plugin repository, colocated with capabilities.
- Do not extend `capability.yaml` for V1. Discover `codex/**/*.md` by convention.
- Preserve every existing `UNI-001` through `UNI-021` id during migration.
- Treat `.specify/codex/` as reserved future cache or lock state, not as a source location.
- Keep RM-04 and RM-11 out of scope: this plan exposes rule metadata and resolved JSON, but does not implement the final review finding schema or CI reviewer.

## Dependency Overview

The critical path is:

1. Define the codex file contract.
2. Implement the CLI parser and validator.
3. Implement project-aware rule resolution.
4. Expose `specify codex` commands.
5. Migrate first-party rule content.
6. Update reviewer skills and docs to cite resolved codex rules.

After the file contract is settled, most content migration can proceed in parallel with CLI resolver work. Reviewer skill updates should wait until migrated rule ids and provenance are stable.

## Change 01: Codex Contract And Schema

Subagent scope: one focused subagent.

Repositories:

- `specify-cli`
- `specify`

Primary files:

- `specify-cli/schemas/`
- `specify-cli/crates/capability/src/`
- `specify/.cursor/schemas/`
- `specify/rfcs/rm-03-codex.md`
- `specify/docs/contributing/checks.md`

Tasks:

- Finalize the V1 frontmatter fields from the RFC:
  - required: `id`, `title`, `severity`, `trigger`
  - optional: applicability metadata, review mode, deterministic hints, references, deprecation state
- Add the JSON Schema for codex rule frontmatter in the CLI repo.
- Mirror the schema into the plugin repo only if `scripts/checks.ts` will validate codex files there before the CLI validator is available.
- Document canonical values:
  - severity: `critical`, `important`, `suggestion`, `optional`
  - review mode: `deterministic`, `model-assisted`, `hybrid`
  - provenance kinds: `capability`, `catalog`, `repo`
- Decide the minimum V1 applicability shape. Keep it small enough for migration, and defer policy exceptions or suppressions.
- Add parser fixtures for valid and invalid frontmatter.

Acceptance checks:

- The schema rejects missing required fields, unknown severities, malformed rule ids, and invalid review modes.
- The schema does not require optional body sections beyond `## Rule`.
- The RFC or companion docs state that duplicate ids are resolved-set validation errors, not per-file schema errors.

Dependencies:

- None.

Parallelism:

- This change blocks all other implementation changes.

## Change 02: CLI Codex Parser And Format Validator

Subagent scope: one focused Rust subagent.

Repository:

- `specify-cli`

Primary files:

- `specify-cli/crates/capability/src/brief.rs`
- `specify-cli/crates/capability/src/lib.rs`
- new `specify-cli/crates/capability/src/codex.rs` or a new workspace crate if the module becomes large
- `specify-cli/crates/capability/src/tests.rs`
- `specify-cli/crates/error/src/lib.rs`

Tasks:

- Add a `CodexRule` model with:
  - source path
  - parsed frontmatter
  - Markdown body
  - normalized rule id
- Reuse the existing frontmatter delimiter pattern from brief parsing.
- Validate a single rule file:
  - leading YAML frontmatter exists
  - frontmatter parses
  - required fields are present
  - `severity` and `review_mode` use canonical values
  - `id` uses a reserved or locally allowed namespace
  - body contains a `## Rule` heading
- Return `ValidationResult` entries for deterministic failures.
- Add unit tests for valid files, malformed YAML, missing frontmatter, missing `## Rule`, malformed ids, and old severity labels such as `Warning` or `Info`.

Acceptance checks:

- Unit tests exercise parser edge cases without invoking the binary.
- User-facing errors use existing `specify-error::Error` patterns and stable kebab-case diagnostics.
- No command-dispatch or printing logic is added to the domain module.

Dependencies:

- Change 01.

Parallelism:

- Blocks Change 03 and Change 04.
- Can run in parallel with Change 05 once Change 01 is complete if the content subagent validates manually against the contract.

## Change 03: CLI Project Codex Resolver

Subagent scope: one focused Rust subagent.

Repository:

- `specify-cli`

Primary files:

- `specify-cli/crates/capability/src/capability.rs`
- `specify-cli/src/context.rs`
- new codex resolver module in `specify-cli/crates/capability/src/`
- `specify-cli/tests/capability.rs` or new `specify-cli/tests/codex.rs`

Tasks:

- Implement project-aware resolution for the active codex:
  - foundational `default` capability codex
  - resolved project capability codex
  - future shared catalog codex locations, represented as an empty V1 hook if config is not ready
  - repo-root `codex/` overlay
- Load `codex/**/*.md` by convention under each source root.
- Preserve deterministic ordering:
  - default capability first
  - project capability second
  - shared catalogs third
  - repo overlay last
  - lexical path order within each source
- Attach provenance to every rule:
  - `capability` with capability name and version for default and project capability rules
  - `catalog` for future shared catalogs
  - `repo` for repo-root overlays
- Reject duplicate rule ids within the resolved set.
- Make hub behavior explicit. Recommended V1 behavior: hub projects have no project capability codex, but may still validate repo-root `codex/` if present; if default capability location is unavailable, emit a clear resolver diagnostic.
- Add tests for default plus capability plus repo overlay ordering and duplicate id failure.

Acceptance checks:

- Resolution does not read `.specify/codex/` as human-authored input.
- Resolver tests do not depend on the real first-party repository layout; they use temp fixtures.
- Duplicate ids produce validation exit semantics when surfaced through CLI commands.

Dependencies:

- Change 02.

Parallelism:

- Blocks Change 04.
- Can run in parallel with Change 05 and Change 06 after Change 01.

Implementation note:

- The existing capability resolver still references historical local `schemas/<name>/` wording in some comments. The codex resolver should follow the current capability root returned by the resolver rather than hard-coding a first-party path.

## Change 04: `specify codex` CLI Surface

Subagent scope: one focused Rust subagent.

Repository:

- `specify-cli`

Primary files:

- `specify-cli/src/cli.rs`
- `specify-cli/src/commands.rs`
- new `specify-cli/src/commands/codex.rs`
- `specify-cli/src/output.rs`
- `specify-cli/docs/reference/cli/` if CLI reference docs live in this repo
- `specify-cli/tests/codex.rs`
- `specify-cli/tests/fixtures/e2e/goldens/` if export JSON gets a golden

Tasks:

- Add a top-level `Codex` command with subcommands:
  - `specify codex list`
  - `specify codex show <rule-id>`
  - `specify codex validate`
  - `specify codex export --format json`
- Use `CommandContext::require` for project-aware commands.
- Keep domain logic in the workspace crate. The command module should only dispatch, format, and map exit results.
- Implement text output:
  - `list`: concise id, severity, provenance, title
  - `show`: frontmatter summary plus rendered Markdown body
  - `validate`: pass/fail summary with failing rule ids and paths
- Implement JSON output using the existing v2 envelope and kebab-case keys.
- Make `export --format json` include:
  - frontmatter fields
  - Markdown body
  - source path
  - provenance kind
  - capability name and version where applicable, including `default`
- For `export` in text mode, either print a short hint to rerun with `--format json` or support a deterministic text rendering. Prefer the hint if the JSON export is the contract.
- Add integration tests for command help, successful list/show/export, invalid codex validation, duplicate ids, and missing rule id on `show`.

Acceptance checks:

- `codex validate` exits `0` when clean and validation failure exit code `2` when format validation fails.
- JSON responses include `schema-version`.
- Text output is stable enough for operators but tests assert structure rather than prose where possible.

Dependencies:

- Change 03.

Parallelism:

- Blocks Change 09.
- Can run in parallel with Change 06 and Change 07 after Change 03 is far enough to expose resolver APIs.

## Change 05: Default Capability And `UNI-*` Migration

Subagent scope: one content-focused subagent.

Repository:

- `specify`

Primary files:

- `specify/plugins/references/review-checks.md`
- new `specify/capabilities/default/capability.yaml`
- new `specify/capabilities/default/briefs/`
- new `specify/capabilities/default/codex/*.md`
- `specify/capabilities/README.md`
- `specify/docs/contributing/capability-anatomy.md`
- `specify/plugins/references/README.md`

Tasks:

- Add `capabilities/default/` as the foundational capability.
- Keep the default capability pipeline minimal and valid. If the CLI requires pipeline entries, use the smallest no-op or generic brief set that satisfies `capability.yaml` schema without changing workflow behavior.
- Split `plugins/references/review-checks.md` into one codex rule file per existing `UNI-*` id.
- Preserve ids exactly from `UNI-001` through `UNI-021`.
- Convert legacy severities:
  - `Critical` -> `critical`
  - `Warning` -> `important` unless the rule is clearly advisory
  - `Info` -> `suggestion`
- Give every rule a concise trigger and a self-contained `## Rule` section.
- Move "What to look for" bullets into `## Look For`.
- Move "Spec-change indicator" prose into `## Spec Guidance`.
- Keep `plugins/references/review-checks.md` as a transitional index or pointer until all reviewer skills stop depending on it directly.
- Update capability docs to mention optional `codex/` directories by convention.

Acceptance checks:

- All `UNI-001` through `UNI-021` ids exist exactly once.
- The migrated rules validate against the Change 01 contract.
- Existing reviewer skill links do not break during the transition.
- `make checks` in `specify` still passes, or any remaining failure is explicitly tied to a later change.

Dependencies:

- Change 01.

Parallelism:

- Can run in parallel with Change 02 and Change 03.
- Blocks Change 08.

## Change 06: Capability-Specific Codex Packs

Subagent scope: three independent content subagents after a shared starter brief.

Repository:

- `specify`

Primary files:

- `specify/capabilities/omnia/codex/`
- `specify/capabilities/contracts/codex/`
- `specify/capabilities/vectis/codex/`
- relevant reviewer references under `specify/plugins/omnia/`, `specify/plugins/contracts/`, and `specify/plugins/vectis/`

Tasks:

- Add Omnia rules:
  - provider usage and bypass prevention
  - WASM runtime constraints
  - Rust error handling and panic policy
  - secrets and unsafe host access where Omnia owns the concern
- Add Contracts rules:
  - OpenAPI compatibility
  - AsyncAPI compatibility
  - JSON Schema evolution
  - SemVer and consumer impact classification hooks
- Add Vectis rules:
  - Crux core/shell boundary
  - state transition discipline
  - interface and platform shell responsibilities
- Reserve stable ids using the RFC namespaces:
  - `OMNIA-*`
  - `RUST-*`
  - `SEC-*`
  - `IFACE-*`
  - `VECTIS-*`
- Avoid copying reviewer occurrence prefixes such as `SEC-1`; codex ids must be stable catalogue ids like `SEC-003`.

Acceptance checks:

- Each capability has a small, coherent first cut rather than a sprawling catalogue.
- No id duplicates exist across default, Omnia, Contracts, and Vectis packs.
- Rule body prose is normative and useful to both humans and model-assisted reviewers.

Dependencies:

- Change 01.
- Prefer Change 05 first for naming and severity examples, but the three capability packs can start once the schema is stable.

Parallelism:

- The Omnia, Contracts, and Vectis packs can be written in parallel by separate subagents.
- Blocks parts of Change 08 that cite capability-specific ids.

## Change 07: Plugin Repository Codex Shape Check

Subagent scope: one TypeScript/docs subagent.

Repository:

- `specify`

Primary files:

- `specify/scripts/checks.ts`
- `specify/docs/contributing/checks.md`
- `specify/.cursor/schemas/`
- `specify/capabilities/**/codex/**/*.md`

Tasks:

- Add a format-only `make checks` validation for first-party codex files.
- Discover files under:
  - `capabilities/*/codex/**/*.md`
  - optionally repo-root `codex/**/*.md` if this repo later adds an overlay
- Validate frontmatter against the codex schema.
- Validate required `## Rule` body heading.
- Validate duplicate ids across first-party codex files.
- Validate namespace ownership for first-party capabilities:
  - default owns `UNI-*`
  - Omnia owns `OMNIA-*` and Omnia-specific `RUST-*` or `SEC-*`
  - Contracts owns `IFACE-*`
  - Vectis owns `VECTIS-*`
- Update docs for the new check and common failures.

Acceptance checks:

- `make checks` fails on malformed codex files with actionable messages.
- The check is shape-only and does not attempt consumer-project review.
- Existing checks remain independently runnable in the current `Promise.all` grouping style.

Dependencies:

- Change 01.

Parallelism:

- Can run in parallel with Change 05 and Change 06 after the schema is settled.
- Should finish before Change 09.

## Change 08: Reviewer Skill Codex Citations

Subagent scope: one or two documentation/prompt-engineering subagents.

Repository:

- `specify`

Primary files:

- `specify/plugins/omnia/skills/code-reviewer/SKILL.md`
- `specify/plugins/omnia/skills/code-reviewer/categories.md`
- `specify/plugins/omnia/skills/code-reviewer/output.md`
- `specify/plugins/vectis/skills/core-reviewer/SKILL.md`
- `specify/plugins/vectis/skills/ios-reviewer/SKILL.md`
- `specify/plugins/vectis/skills/android-reviewer/SKILL.md`
- shared reviewer references under `specify/plugins/references/`

Tasks:

- Update reviewer instructions to cite stable codex rule ids when findings map to a codex rule.
- Preserve existing team protocols, specialist prefixes, and antagonist review behavior.
- Clarify the distinction between:
  - review-local occurrence ids, such as `SEC-1`
  - stable codex rule ids, such as `SEC-003`
- Replace direct dependence on `plugins/references/review-checks.md` where the migrated `default` codex rules are the better source.
- Keep transitional references if needed so current reviewer skills remain usable before `specify codex export` is wired into agent workflows.
- Do not embed full codex prose into `AGENTS.md`.

Acceptance checks:

- Reviewer outputs can carry both occurrence id and `rule_id`.
- No reviewer skill claims that RM-04 finding schema already exists.
- Skill docs stay within existing line-count and critical-path conventions.
- `make checks` passes.

Dependencies:

- Change 05.
- Relevant parts of Change 06 for capability-specific citations.

Parallelism:

- Omnia reviewer updates and Vectis reviewer updates can run in parallel after the rule ids are stable.

## Change 09: Cross-Repo Integration And Distribution Proof

Subagent scope: one integration subagent.

Repositories:

- `specify-cli`
- `specify`

Primary files:

- `specify-cli/tests/codex.rs`
- `specify-cli/tests/e2e.rs`
- `specify-cli/tests/fixtures/e2e/goldens/`
- `specify/docs/reference/cli/`
- `specify/plugins/spec/references/capability-resolution.md`
- `specify/docs/reference/capabilities/index.md`

Tasks:

- Create an end-to-end fixture that initializes or simulates a project with a capability and repo overlay.
- Verify `specify codex export --format json` returns default, capability, and repo rules in deterministic order.
- Verify `specify codex show UNI-002` can locate a default capability rule.
- Verify duplicate ids across capability and repo overlay fail validation.
- Confirm how first-party `default` codex is made available to the CLI in real projects:
  - from the same distribution tree as first-party capabilities
  - from cache populated by `specify init`
  - or from an explicit packaged fixture path
- Document the chosen distribution behavior so future capability authors know how `default` is found.
- Update CLI reference docs for `specify codex`.

Acceptance checks:

- `cargo make test` or the narrow documented equivalent passes in `specify-cli`.
- `make checks` passes in `specify`.
- The exported JSON shape is stable enough for RM-04 and RM-11 to consume later.

Dependencies:

- Change 04.
- Change 05.
- Change 07.

Parallelism:

- This is a final integration step and should not run until its dependencies are substantially complete.

## Change 10: RM-03 Closeout And Roadmap Update

Subagent scope: one final documentation subagent.

Repository:

- `specify`

Primary files:

- `specify/rfcs/rm-03-codex.md`
- `specify/rfcs/roadmap.md`
- any new codex reference docs

Tasks:

- Update RM-03 from design draft to implemented status, or add an implementation notes section if the repository keeps RFCs as historical proposals.
- Resolve or carry forward RM-03 open questions:
  - shared catalog config location
  - whether `codex validate` also surfaces under future `specify check`
  - deprecated rule visibility
  - minimum JSON export shape needed by RM-04
- Update the roadmap to show RM-03 completion only after CLI commands, rule migration, and reviewer citations are landed.
- Add follow-up notes for RM-04 and RM-11 consumers.

Acceptance checks:

- The roadmap no longer asks questions answered by RM-03 implementation.
- Remaining open questions are explicitly deferred with owners or dependent roadmap items.
- Documentation links pass `make checks`.

Dependencies:

- Change 09.

Parallelism:

- Serial closeout.

## Parallel Execution Groups

### Group A: Foundation

Run first:

- Change 01

### Group B: Core Buildout

Run after Change 01:

- Change 02
- Change 05
- Change 07, if the schema is stable enough for the check author

### Group C: Resolver And Content Expansion

Run after Change 02 for resolver work, and after Change 01 for content work:

- Change 03
- Change 06 Omnia codex pack
- Change 06 Contracts codex pack
- Change 06 Vectis codex pack

### Group D: CLI Surface And Skill Adoption

Run when their prerequisites are ready:

- Change 04 after Change 03
- Change 08 Omnia reviewer updates after Change 05 and Omnia rules from Change 06
- Change 08 Vectis reviewer updates after Change 05 and Vectis rules from Change 06

### Group E: Final Proof

Run serially:

- Change 09
- Change 10

## Suggested Subagent Prompts

### Change 01 Prompt

Implement the RM-03 codex V1 file contract. Add or update schemas and parser fixtures for Markdown files with YAML frontmatter. Keep the required fields to `id`, `title`, `severity`, and `trigger`; add optional applicability, review mode, deterministic hints, references, and deprecation fields only if they are cheap to validate. Do not implement CLI commands or migrate rule content.

### Change 02 Prompt

Implement codex rule parsing and format validation in `specify-cli`. Reuse the existing brief frontmatter parsing style, add a `CodexRule` domain model, return `ValidationResult` failures, and cover malformed frontmatter, invalid ids, missing `## Rule`, and legacy severity labels in unit tests. Do not add command dispatch.

### Change 03 Prompt

Implement project-aware codex resolution in `specify-cli`. Resolve default capability rules, active capability rules, future shared catalog hook points, and repo-root overlays in deterministic order. Attach provenance and reject duplicate ids. Use temp fixture tests and do not add user-facing command formatting.

### Change 04 Prompt

Add the `specify codex` command group in `specify-cli`: `list`, `show <rule-id>`, `validate`, and `export --format json`. Keep domain logic in the codex resolver/parser modules, emit JSON through the existing v2 envelope, and add integration tests for success and validation failure cases.

### Change 05 Prompt

Add `capabilities/default/` to the `specify` repo and migrate `plugins/references/review-checks.md` into one codex rule file per `UNI-001` through `UNI-021` id. Preserve ids exactly, convert severities to RM-03 canonical values, keep `review-checks.md` as a transitional index or pointer, and update capability docs.

### Change 06 Prompt

Create a small first cut of capability-specific codex rules for one capability: Omnia, Contracts, or Vectis. Use stable RM-03 namespaces, write normative `## Rule` prose, include useful `## Look For` guidance, and avoid reviewer occurrence numbering. Coordinate ids with the other capability pack subagents.

### Change 07 Prompt

Extend `specify/scripts/checks.ts` to validate first-party codex file shape under `capabilities/*/codex/**/*.md`. Validate frontmatter schema, required `## Rule`, duplicate ids, and basic namespace ownership. Update `docs/contributing/checks.md`.

### Change 08 Prompt

Update reviewer skills to cite codex rule ids while preserving their existing review-team protocols. Distinguish stable `rule_id` values from report-local occurrence ids, avoid claiming the RM-04 finding schema exists, and keep skill docs within repository conventions.

### Change 09 Prompt

Prove the cross-repo RM-03 path. Add CLI integration or e2e tests showing `specify codex export --format json` resolves default, capability, and repo overlay rules with provenance and deterministic ordering. Document how the default capability codex is distributed and found in real projects.

### Change 10 Prompt

Close out RM-03 documentation. Update the RFC, roadmap, and reference docs to reflect implemented behavior, carry forward only unresolved shared-catalog or RM-04/RM-11 questions, and verify documentation checks.

