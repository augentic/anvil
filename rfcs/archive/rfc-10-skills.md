# RFC-10: Skill Improvements

> Status: Implemented
>
> **RFC-14 supersession note**: this archive includes historical skill argument examples that mention `auto-merge`. Current `/change:plan --orchestrate` treats `--auto-merge` as a retired flag and exits non-zero before side effects; Specify never calls `specify workspace merge` or `gh pr merge`.

## Abstract

Bring every plugin `SKILL.md` in this repository into alignment with Anthropic's published [Agent Skills overview](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview) and [authoring best practices](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices). The audit covers all 29 SKILL.md files across the current plugin set (`spec`, `omnia`, `vectis`, `contracts`, `rt`, and `plan`) and produces three categories of work:

1. **Frontmatter standard.** Drop the non-standard `license` field, rewrite every `argument-hint` so it reads as Cursor placeholder text rather than a CLI usage line, qualify every `name` with its plugin domain to eliminate cross-plugin collisions, add explicit "Use when…" triggers to every description, and decide a single policy for `allowed-tools`.
2. **Body-size compliance.** Bring every SKILL.md under Anthropic's 500-line ceiling using the progressive-disclosure pattern already established in `extract/`, `execute/`, `plan/`, and `merge/`. Factor the ~80 lines of phase-outcome / journal / plan-mutation boilerplate currently duplicated across `define`, `build`, `merge`, and `drop` into a single shared reference.
3. **Selective reorganisation.** Drop `rt/git-cloner` (it is a 200-line wrapper around `git clone`), rename the `plan` plugin to `client` to remove the active naming collision with `.specify/plan.yaml` and create a home for adjacent client-facing skills, and rename `contracts` to `interfaces` while replacing the lifecycle-oriented trio (`writer` / `validator` / `importer`) with format-family skills for OpenAPI, AsyncAPI, and JSON Schema.

No CLI verbs change. No Specify artifact schemas, plan schemas, or persisted contract paths change. Skill behaviour changes only where §C.3 replaces the generic contracts lifecycle surface with interface-format skills. The rest of the work is dominated by mechanical renames, file deletions, shared-reference factoring, and validation updates.

This RFC is intentionally narrow: it covers the *shape* of skill files, not their generated-code behaviour. It also makes two slash-command namespace corrections where the current labels actively obscure discovery: `plan` becomes `client`, and `contracts` becomes `interfaces` with format-specific skills.

## Motivation

### Anthropic's discovery model is two-stage

Skills load in two stages, and the boundary matters.

- **Stage 1 (always loaded).** `name` and `description` from every installed skill's YAML frontmatter are concatenated into the system prompt at session start. Anthropic budgets ~100 tokens per skill at this stage. The metadata must give Claude enough signal to pick the correct skill from a catalogue of dozens, with no body context loaded yet.
- **Stage 2 (loaded on trigger).** The full SKILL.md body is read via bash when the skill is selected. Every byte after that competes with conversation history for the remaining context window.

Two consequences shape this RFC. First, the always-loaded metadata is precious — every non-essential field (e.g. `license`) costs tokens for every skill in every session, forever. Second, the SKILL.md body has a soft ceiling: Anthropic's best-practices guide explicitly recommends "under 500 lines for optimal performance" and "split content into separate files when approaching this limit".

### Drift from published conventions

The published guide was authored after most of this repository's skills, and the divergence has accumulated. A pre-RFC-10 audit identifies five concrete drifts:

1. `**argument-hint` is being used as a CLI usage string.** Today's hints look like `"include glob...? exclude glob...? manifest path?"` — bare nouns with `?` for "optional" and dashes scraped off the flags. Cursor renders the field as a single line of placeholder text after the user types `/spec:plan`  in chat. It is a hint, not a usage spec; flags do not belong in it.
2. `**license: MIT` pollutes every SKILL.md.** Anthropic's frontmatter spec validates only `name` and `description`. License belongs in `plugin.json` (already present) and the repo `LICENSE` file. On every SKILL.md it inflates the always-loaded metadata for no purpose.
3. **Skill `name` collisions across plugins.** The bare `name` field is global from Claude's perspective. Today the repository contains two skills called `test-writer` (in `omnia` and `vectis`) and three single-word names — `writer`, `validator`, `importer` — under `contracts`. The `/contracts:writer` Cursor slash command disambiguates *invocation*; the SKILL.md `name` field does not disambiguate *discovery*. More importantly, those lifecycle verbs hide the domain noun operators actually use: OpenAPI, AsyncAPI, and JSON Schema.
4. **Generic phase-verb names.** `init`, `define`, `build`, `merge`, `drop`, `extract`, `analyze`, `plan`, `execute` are precisely the class Anthropic's guide calls out as "vague": each is a verb with hundreds of unrelated meanings outside Specify. The `/spec:` slash prefix saves the operator in Cursor; it does not save Claude when reasoning about whether to fire the skill.
5. **Descriptions miss the *when-to-use* trigger.** Anthropic's guide is emphatic: descriptions should describe both **what the skill does** and **when to use it**, in third person. Most `vectis/`* skills do this well ("Use when…"). The `omnia/*`, `contracts/*`, `rt/*`, and `plan/sow-writer` skills do not.

A sixth drift is internal rather than published-doc-driven: the four phase skills (`define`, `build`, `merge`, `drop`) each carry near-identical "Phase outcome contract", "Journal entries during the run", and "Mutating the plan mid-run" sections. The wording is intentionally normative and intentionally repeated, but the duplication invites one-of-four drift the next time the contract evolves.

### Why now

- The skill count has grown to 29 across six plugins, which is large enough that the always-loaded metadata budget is non-trivial and cross-plugin name collisions are no longer hypothetical.
- Plugins are not yet broadly adopted outside this repository. There is no muscle memory or downstream documentation to break, so the migration cost is bounded to this repository's docs, fixtures, and `marketplace.json`.
- The `extract/`, `execute/`, `plan/`, and `merge/` skills already use the progressive-disclosure pattern Anthropic recommends. Codifying that pattern as house style and applying it to the remaining skills is a low-cost, high-leverage move while the surface is still small enough to refactor in a single change.

### Non-goals

- **No CLI verb behavioural changes.** Skill bodies that shell out to `specify ...` continue to call the same verbs with the same arguments and get the same JSON back.
- **No Specify workflow schema changes.** Specify artifact schemas, plan schemas, brief topology, and `pipeline.`* ordering are unchanged. Brief frontmatter values that name a renamed or split skill are mechanically retargeted (see §C.2 and §C.3); the brief frontmatter *shape* is unchanged. `schemas/skill.schema.json` is updated only because it validates skill metadata, not Specify project artifacts.
- **No new plugins.** The `plan` → `client` and `contracts` → `interfaces` changes are label changes, not new plugins.
- **No general namespace doctrine.** This RFC makes two concrete namespace decisions (`client` for client-facing deliverables, `interfaces` for API/interface contracts) but does not settle a repository-wide layer-vs-domain split for future slash-command namespaces.
- **No deeper restructuring of `omnia/code-reviewer`'s agent-team protocol.** This RFC splits the file under the 500-line ceiling and stops there.

## Detailed Design

### A. Frontmatter standard

The canonical SKILL.md frontmatter shape after this RFC:

```yaml
---
name: <plugin-qualified-skill-name>      # globally unique
description: <what the skill does>; <when to use it>   # third person; ≤1024 chars
argument-hint: <placeholder-text>        # optional; Cursor slash-command hint
allowed-tools: <space-separated tool names>            # optional; omit = inherit caller's set
---
```

Four sub-decisions follow.

#### A.1 `name` — domain-qualify every skill

The `name` field is global from Claude's discovery perspective. Cross-plugin collisions and overly-generic single-word names are both lifted out by qualifying every `name` with its plugin domain. Cursor's `/plugin:skill` slash-command syntax is independent of this field; the frontmatter `name:` changes in this section do not change slash commands. The explicit slash-command namespace changes are listed separately in §C.

Anthropic's guide recommends considering gerund-form names (`processing-pdfs`, `testing-code`) but also explicitly accepts noun phrases and action-oriented names. This RFC uses that full accepted set rather than forcing gerunds everywhere: existing Specify phase verbs remain action-oriented because they are the product vocabulary operators already use, writer/reviewer/updater names remain artifact-role noun phrases where that is clearer than `writing-*`, and interface-format names remain noun phrases because operators and briefs naturally ask for OpenAPI, AsyncAPI, or JSON Schema rather than an internal lifecycle verb. New skills should prefer gerunds when the gerund reads naturally, but established product verbs and concrete artifact nouns are acceptable when they improve discovery.


| Plugin                     | Today                                                              | After                                                                                              | Slash command                                                            |
| -------------------------- | ------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| `spec`                     | `name: init`                                                       | `name: specify-init`                                                                               | `/spec:init`                                                             |
| `spec`                     | `name: define`                                                     | `name: specify-define`                                                                             | `/spec:define`                                                           |
| `spec`                     | `name: build`                                                      | `name: specify-build`                                                                              | `/spec:build`                                                            |
| `spec`                     | `name: merge`                                                      | `name: specify-merge`                                                                              | `/spec:merge`                                                            |
| `spec`                     | `name: drop`                                                       | `name: specify-drop`                                                                               | `/spec:drop`                                                             |
| `spec`                     | `name: extract`                                                    | `name: specify-extract`                                                                            | `/spec:extract`                                                          |
| `spec`                     | `name: analyze`                                                    | `name: specify-analyze`                                                                            | `/spec:analyze`                                                          |
| `spec`                     | `name: plan`                                                       | `name: specify-plan`                                                                               | `/spec:plan`                                                             |
| `spec`                     | `name: execute`                                                    | `name: specify-execute`                                                                            | `/spec:execute`                                                          |
| `omnia`                    | `name: crate-writer`                                               | `name: omnia-crate-writer`                                                                         | `/omnia:crate-writer`                                                    |
| `omnia`                    | `name: test-writer`                                                | `name: omnia-test-writer`                                                                          | `/omnia:test-writer`                                                     |
| `omnia`                    | `name: guest-writer`                                               | `name: omnia-guest-writer`                                                                         | `/omnia:guest-writer`                                                    |
| `omnia`                    | `name: code-reviewer`                                              | `name: omnia-code-reviewer`                                                                        | `/omnia:code-reviewer`                                                   |
| `vectis`                   | `name: core-writer`                                                | `name: vectis-core-writer`                                                                         | `/vectis:core-writer`                                                    |
| `vectis`                   | `name: core-reviewer`                                              | `name: vectis-core-reviewer`                                                                       | `/vectis:core-reviewer`                                                  |
| `vectis`                   | `name: ios-writer`                                                 | `name: vectis-ios-writer`                                                                          | `/vectis:ios-writer`                                                     |
| `vectis`                   | `name: ios-reviewer`                                               | `name: vectis-ios-reviewer`                                                                        | `/vectis:ios-reviewer`                                                   |
| `vectis`                   | `name: android-writer`                                             | `name: vectis-android-writer`                                                                      | `/vectis:android-writer`                                                 |
| `vectis`                   | `name: android-reviewer`                                           | `name: vectis-android-reviewer`                                                                    | `/vectis:android-reviewer`                                               |
| `vectis`                   | `name: design-system-writer`                                       | `name: vectis-design-system-writer`                                                                | `/vectis:design-system-writer`                                           |
| `vectis`                   | `name: test-writer`                                                | `name: vectis-test-writer`                                                                         | `/vectis:test-writer`                                                    |
| `vectis`                   | `name: template-updater`                                           | `name: vectis-template-updater`                                                                    | `/vectis:template-updater`                                               |
| `contracts` → `interfaces` | `name: writer`, `name: validator`, `name: importer` (three skills) | `name: interfaces-openapi`, `name: interfaces-asyncapi`, `name: interfaces-json-schema` (see §C.3) | `/interfaces:openapi`, `/interfaces:asyncapi`, `/interfaces:json-schema` |
| `rt`                       | `name: wiretapper`                                                 | `name: rt-wiretapper`                                                                              | `/rt:wiretapper`                                                         |
| `rt`                       | `name: replay-writer`                                              | `name: rt-replay-writer`                                                                           | `/rt:replay-writer`                                                      |
| `rt`                       | `name: git-cloner`                                                 | (deleted; see §C.1)                                                                                | (none)                                                                   |
| `plan`                     | `name: sow-writer`                                                 | `name: client-sow-writer` (plugin renames; see §C.2)                                               | `/client:sow-writer`                                                     |


The pattern matches Anthropic's open-source `[claude-api](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/claude-api-skill)` skill (single global namespace) and the pre-built `pdf` / `xlsx` / `pptx` skills (each globally unique).

#### A.2 `description` — what the skill does, when to use it, third person

Anthropic's published guidance: descriptions must describe **both** what the skill does **and** when to use it, in third person, ≤1024 characters. Concrete rewrites for the skills that miss the trigger today:


| Skill                                   | Today                                                                                                                                                                            | After                                                                                                                                                                                                                                                                                                                                                                                               |
| --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `omnia/code-reviewer`                   | "AI-powered code review for generated Rust crates, catching security issues and quality problems"                                                                                | "Reviews generated Omnia Rust WASM crates for security, error handling, WASM constraints, and code quality issues. Use when reviewing crates produced by `/omnia:crate-writer` or when the user mentions code review for a generated crate."                                                                                                                                                        |
| `omnia/crate-writer`                    | "Write Rust WASM crates from Specify artifacts -- greenfield creation or incremental updates -- following Omnia SDK patterns with provider-based dependency injection."          | (prepend trigger) "… Use when implementing crate tasks from a Specify change, regenerating a crate from updated artifacts, or when the user mentions `crate-writer`."                                                                                                                                                                                                                               |
| `omnia/test-writer`                     | "Generate or update test suites for Omnia Rust WASM crates from Specify artifacts -- MockProvider setup, integration tests, spec-to-test mapping, and drift detection."          | (prepend trigger) "… Use when implementing test tasks from a Specify change, regenerating tests after a crate update, or when the user mentions `test-writer`."                                                                                                                                                                                                                                     |
| `omnia/guest-writer`                    | "Generate a Rust project that exposes HTTP endpoints, subscribes to message topics, and handles WebSocket events in order to surface business logic via the Omnia WASI runtime." | (append trigger) "… Use when scaffolding the WASM guest wrapper for a set of generated Omnia crates or when the user mentions `guest-writer`."                                                                                                                                                                                                                                                      |
| `interfaces/openapi` (new per §C.3)     | (split from three `contracts/`* lifecycle skills)                                                                                                                                | "Authors, imports, and verifies OpenAPI 3.1 HTTP API contracts for Specify changes, including path operations, request and response schemas, parameters, auth, examples, and baseline deltas. Use when the contracts brief needs an HTTP API contract, when an operator supplies or asks for an OpenAPI document, or when verifying OpenAPI compatibility after a merge."                           |
| `interfaces/asyncapi` (new per §C.3)    | (split from three `contracts/*` lifecycle skills)                                                                                                                                | "Authors, imports, and verifies AsyncAPI 3.0 event, pub/sub, stream, and WebSocket-style contracts for Specify changes, including channels, messages, bindings, producers, consumers, and schema references. Use when the contracts brief needs an evented interface contract, when an operator supplies or asks for an AsyncAPI document, or when verifying AsyncAPI compatibility after a merge." |
| `interfaces/json-schema` (new per §C.3) | (split from three `contracts/*` lifecycle skills)                                                                                                                                | "Authors, imports, and verifies standalone JSON Schema documents shared by OpenAPI, AsyncAPI, and other interface contracts. Use when a Specify change needs reusable payload schemas, when an operator supplies schema files without a protocol wrapper, or when validating schema compatibility across generated interface contracts."                                                            |
| `plan/sow-writer`                       | "Generate a Statement of Work (SoW) document from Specify artifacts and project context."                                                                                        | (append trigger) "… Use when a delivery lead asks for a SoW from completed Specify artifacts, when exporting client deliverables from a change directory, or when the user mentions `sow-writer`."                                                                                                                                                                                                  |
| `rt/wiretapper`                         | "Add wiretap code to a cloned legacy TypeScript repo to capture request/response and side-effect data as fixture JSON; …"                                                        | (append trigger) "… Use when capturing fixture data from a legacy TypeScript service before migration, or when the user mentions `wiretapper`."                                                                                                                                                                                                                                                     |
| `rt/replay-writer`                      | "Add tests from real-life JSON fixtures in tests/data/replay/, run tests, and review code so tests pass. For crates already generated by the Specify workflow."                  | (append trigger) "… Use when turning captured legacy fixtures into regression tests for a generated crate, or when the user mentions `replay-writer`."                                                                                                                                                                                                                                              |


Two cleanup rules apply globally:

- **Drop RFC citations from descriptions.** `spec/plan` and `spec/init` cite `RFC-9 §2C`, `RFC-9 §1D`, "Layer 4 umbrella". These tokens are useless to Claude when picking a skill. They belong in the body, not in the always-loaded metadata.
- **Trim block-scalar descriptions.** `spec/plan`'s description is ~480 characters and uses a YAML literal block scalar. Halve it; relocate the detail to the body's "Overview" section.

#### A.3 `argument-hint` — placeholder text, not usage strings

Cursor's `argument-hint` renders as a single line of grey placeholder text after the user types `/plugin:skill`  in chat. The convention is a single short hint that names the **primary positional argument**, with `<>` for required and `[]` for optional. Flags do not appear in the hint at all — they are documented in the body's invocation section.


| Skill                                            | Today                                                                                                                                                                              | After                                |
| ------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------ |
| `spec/init`                                      | `"schema?"`                                                                                                                                                                        | `"[schema-url]"`                     |
| `spec/define`                                    | `"description? artifact-id? source key=path-or-url...?"`                                                                                                                           | `"[description]"`                    |
| `spec/build`                                     | `"change-name?"`                                                                                                                                                                   | `"[change-name]"`                    |
| `spec/merge`                                     | `"change-name?"`                                                                                                                                                                   | `"[change-name]"`                    |
| `spec/drop`                                      | `"change-name?"`                                                                                                                                                                   | `"[change-name]"`                    |
| `spec/extract`                                   | `"source-path change-dir include glob...? exclude glob...? manifest path?"`                                                                                                        | `"<source-path> <change-dir>"`       |
| `spec/analyze`                                   | `"input-path output-dir kind legacy-code|documentation source-key k?"`                                                                                                             | `"<input-path> <output-dir>"`        |
| `spec/plan`                                      | `"initiative-name from path...? against path? source key=path-or-url...? focus area? extend? dry-run? orchestrate? shape migrate-legacy|new-feature|update-existing? auto-merge?"` | `"<initiative-name>"`                |
| `spec/execute`                                   | `"dry-run? loop?"`                                                                                                                                                                 | (omit; flag-only invocation)         |
| `omnia/code-reviewer`                            | `"crate-path? fix?"`                                                                                                                                                               | `"[crate-path]"`                     |
| `omnia/crate-writer`                             | `"crate-name?"`                                                                                                                                                                    | `"[crate-name]"`                     |
| `omnia/test-writer`                              | `"crate-name?"`                                                                                                                                                                    | `"[crate-name]"`                     |
| `vectis/test-writer`                             | `"feature-name?"`                                                                                                                                                                  | `"[feature-name]"`                   |
| `vectis/{core,ios,android,design-system}-writer` | `"change-dir"`                                                                                                                                                                     | `"<change-dir>"`                     |
| `vectis/{core,ios,android}-reviewer`             | `"target-dir"`                                                                                                                                                                     | `"<target-dir>"`                     |
| `vectis/template-updater`                        | `"cli-repo-dir"`                                                                                                                                                                   | `"[cli-repo-dir]"` (defaults to CWD) |
| `plan/sow-writer`                                | `"change-dir? output-path? client-name? company-name? pdf?"`                                                                                                                       | `"<change-dir>"`                     |
| `rt/wiretapper`                                  | `"legacy-dir? app-name?"`                                                                                                                                                          | `"<legacy-dir>"`                     |
| `rt/replay-writer`                               | `"crate-name? project-dir?"`                                                                                                                                                       | `"<crate-name>"`                     |


The full set of optional flags and secondary positionals each skill accepts moves into a body section called "Invocation" using the existing convention from `spec/execute`:

```text
/spec:execute              # supervised mode: run one change, stop
/spec:execute --dry-run    # preview next change + progress; no writes
/spec:execute --loop       # run until no eligible change remains
```

#### A.4 `license: MIT` — drop entirely

The field is not part of Anthropic's `SKILL.md` spec. License is already declared in:

- the plugin manifest (`plugins/<plugin>/.cursor-plugin/plugin.json`)
- the repo root `LICENSE` file

Remove `license:` from every SKILL.md frontmatter. Saves ~~12 tokens per skill in always-loaded metadata across 29 skills (~~350 tokens repo-wide, every session).

#### A.5 `allowed-tools` — settle on a single policy

Today the field is set on some skills and omitted on others. The phase skills (`define`, `build`, `merge`, `drop`, `execute`, `plan`) omit it and inherit the caller's full toolbelt; the writer / reviewer / utility skills declare an explicit list. Several declarations are either too narrow (e.g. `omnia/code-reviewer` lists `Read Write StrReplace Shell Grep` but the body spawns specialist agents that need `Glob`, `ReadLints`, and `Task`) or stale.

Two policies are coherent:

1. **Omit everywhere.** Skills inherit the caller's full toolbelt. Simple; matches the de-facto behaviour of the phase skills. Trades the field's value (capability advertisement, principle of least authority) for zero maintenance burden.
2. **Declare on every skill, audited.** Tighter. Catches accidental capability creep. Requires an upfront audit and ongoing discipline.

**Recommend policy 1 for v1.** Drop every existing `allowed-tools` line in this RFC. If a future RFC wants the principle-of-least-authority benefits of policy 2, it can land that work as a focused follow-up with the audit done in one pass. Mixing the two policies, as today, gives the operational cost of (2) with the safety profile of (1).

### B. Body-size compliance

Anthropic's published rule: keep SKILL.md body under 500 lines, split into separate files when approaching that limit. Today's SKILL.md sizes (line counts):


| Skill                     | Lines | Status after this RFC             |
| ------------------------- | ----- | --------------------------------- |
| `omnia/code-reviewer`     | 691   | Split — see §B.1                  |
| `omnia/crate-writer`      | 507   | Split — see §B.1                  |
| `vectis/core-writer`      | 495   | Recount after §B.2; split if ≥500 |
| `vectis/android-writer`   | 490   | Recount after §B.2; split if ≥500 |
| `vectis/android-reviewer` | 489   | Recount after §B.2; split if ≥500 |
| `vectis/core-reviewer`    | 484   | Recount after §B.2; split if ≥500 |
| `plan/sow-writer`         | 477   | Recount after §B.2; split if ≥500 |
| `vectis/ios-reviewer`     | 456   | Watch                             |
| (others)                  | <450  | OK                                |


#### B.1 Split the two over-the-line skills

For each skill over 500 lines, factor out to siblings linked from a "Critical Path" header (Anthropic's [Pattern 1](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices#pattern-1-high-level-guide-with-references)). The shape is the one already in use under `plugins/spec/skills/extract/` and `plugins/spec/skills/execute/`.

`**omnia/code-reviewer` (691 → ≤300)**:

```
plugins/omnia/skills/code-reviewer/
├── SKILL.md                # critical path + invocation + output shape
├── categories.md           # SEC-/COR-/QUA-/UNI- check libraries
├── team-protocol.md        # specialist spawn / antagonist / synthesis rules
├── auto-fix.md             # --fix scope, success-rate, regression guard
├── output.md               # REVIEW.md template + finding-ID conventions
└── references/             # (existing)
```

`**omnia/crate-writer` (507 → ≤450)**:

The skill already has `references/` and `examples/` directories carrying most of the depth. Push the "Hard Rules" enumeration and the "Authority Hierarchy" body into a new `rules.md`; SKILL.md retains the Critical Path quick-reference, the mode-dispatch table, and the artifact-mapping section.

#### B.2 The "Critical Path (Quick Reference)" header pattern as house style

`extract/`, `execute/`, and `plan/` already use a 5-7 bullet quick-reference at the top of SKILL.md before any body sections. The pattern serves three audiences:

- An operator scanning to confirm the skill does what they expect.
- A future maintainer checking that a body change still respects the algorithm.
- Claude itself: when SKILL.md is loaded but a sibling hasn't been read yet, the Critical Path tells Claude which siblings to consult next.

**Codify the pattern.** Add a "Skill authoring conventions" section to `.cursor/rules/project.mdc` (or a new `docs/explanation/skill-authoring.md` linked from `AGENTS.md`) requiring every SKILL.md ≥150 lines to lead with a Critical Path quick-reference.

Near-limit skills are not exempt. For the five files within 25 lines of the ceiling (`vectis/core-writer`, `vectis/android-writer`, `vectis/android-reviewer`, `vectis/core-reviewer`, and `client/sow-writer` after the plugin rename), add the Critical Path block only together with an offsetting extraction or deletion. Recount after editing; if the body is ≥500 lines, split immediately rather than leaving a known violation for a later pass.

#### B.3 Factor the duplicated phase-outcome contract

The four phase skills (`define`, `build`, `merge`, `drop`) each carry near-identical sections titled:

- "Phase outcome contract"
- "Journal entries during the run"
- "Mutating the plan mid-run"

Wording and structure are intentionally normative — but the duplication is also load-bearing for one-of-four drift the next time `/spec:execute` evolves the contract. Compare the openings:

```text
plugins/spec/skills/define/SKILL.md  L52   "## Phase outcome contract … `specify change outcome set <name> define <outcome>`"
plugins/spec/skills/build/SKILL.md   L14   "## Phase outcome contract … `specify change outcome set <name> build <outcome>`"
plugins/spec/skills/merge/SKILL.md   L22   "## Phase outcome contract … (with three sub-paths)"
plugins/spec/skills/drop/SKILL.md    (the merge variant via `--reason`)
```

Factor into a single shared reference:

```
plugins/spec/references/phase-outcome-contract.md
```

The reference holds the parameterised contract: the three outcome values (`success` / `failure` / `deferred`), the journal kinds (`question` / `failure` / `recovery`), the plan-mutation allow/forbid table, and the byte-for-byte `--reason` rule. Each phase skill replaces its current ~80-line section with a 4-line block:

```markdown
## Phase outcome contract

This skill is the **<phase>** phase of the `/spec:execute` driver loop.
The shared phase contract — outcome values, journal kinds, plan-mutation rules,
the verbatim-`summary` rule, and the success/failure/deferred semantics — is
authored once at `[../../references/phase-outcome-contract.md](../../references/phase-outcome-contract.md)`.

This phase's outcome-specific deltas:

- `success` — <phase-specific success criteria>
- `failure` — <phase-specific failure modes>
- `deferred` — <phase-specific deferral triggers>
```

`merge/SKILL.md` retains its "success path / failure path / deferred path" classification because the success path is uniquely CLI-stamped (no `outcome set` call); that detail stays in the phase-specific delta block. The shared contract handles everything else.

Effect: ~240 lines removed from `define/`, `build/`, `merge/`, `drop/` combined; one new ~120-line authoritative reference. Drift becomes physically impossible: edit the contract in one place.

### C. Selective reorganisation

#### C.1 Drop `rt/git-cloner` as a standalone skill

```text
plugins/rt/skills/git-cloner/SKILL.md  → 206 lines wrapping `git clone <url> <dir>`
```

The skill provides input validation, a uniform error-message style, and an optional `--detach` mode. None of these justify a separate skill: `git clone` is a primitive Claude can invoke directly, and Anthropic's [solve-don't-punt](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices#solve-dont-punt) guidance is about scripts, not about packaging trivial wrappers as skills.

Concretely:

- Delete `plugins/rt/skills/git-cloner/`.
- Inline a 5-line "Cloning a source tree" block into the two callers that reach for the skill today: `plugins/spec/skills/analyze/SKILL.md` (for `--source` URLs reaching plan-time discovery) and `plugins/rt/skills/wiretapper/SKILL.md` (for legacy-repo bootstrap).
- Drop the entry from `marketplace.json`.

The `--detach` mode (remove `.git` after clone, preserve the working tree) becomes an inlined guarded sequence rather than a reusable skill: quote the destination path, verify it is the intended clone directory, and remove only `"$DEST/.git"` after confirming that path is a directory. Do not carry forward an unguarded `rm -rf` recipe.

Saves one always-loaded metadata entry (~100 tokens per session, every session).

#### C.2 Rename the `plan` plugin to `client`

```json
// .cursor-plugin/marketplace.json — today
{
  "name": "plan",
  "source": "plan",
  "description": "Skills to generate Statements of Work from Specify artifacts."
}
```

The `plan` plugin contains exactly one skill (`sow-writer`) and has no relationship to `.specify/plan.yaml`. The naming overlap with `/spec:plan` (the L2 plan-authoring skill) is actively confusing — `/plan:sow-writer` reads like part of plan authoring, but it is a client-facing deliverable generator.

Rename to `client` — a richer category with room to grow into adjacent client-facing skills (`proposal-writer`, `pricing-writer`, `case-study-writer`, and similar):

- Plugin directory: `plugins/plan/` → `plugins/client/`
- Slash command: `/plan:sow-writer` → `/client:sow-writer`
- Marketplace entry: `name: plan` → `name: client`
- Plugin description: "Skills to generate client-facing deliverables — Statements of Work, proposals, pricing summaries, and similar artefacts — from Specify artifacts."
- Skill directory: `plugins/plan/skills/sow-writer/` → `plugins/client/skills/sow-writer/`
- SKILL.md `name:` becomes `client-sow-writer` per §A.1's qualification rule (supersedes the `plan` row in §A.1's table, which currently reads `name: sow-writer` pending this rename).

A smaller alternative — rename to `sow` to mirror the single existing skill (`/sow:writer`) — is rejected. It removes the immediate `plan` collision but forecloses the obvious next step: the repository already has demand for adjacent client-facing skills, and housing them under a single `client` plugin from day one avoids a second rename later. The cost difference is a single character of typing per slash command, paid against a structural choice that lasts the lifetime of the plugin. See alternative §F for the trade-off in detail.

#### C.3 Rename `contracts` to `interfaces` and split by interface format

The current `contracts` plugin is organised by lifecycle verb:

- `contracts/writer` — generate a contract delta during define
- `contracts/validator` — validate during define and post-merge
- `contracts/importer` — normalise operator-supplied external contracts

Those verbs are useful implementation phases, but they are weak skill-discovery names. Operators and briefs are more likely to name the interface format they need: OpenAPI, AsyncAPI, or JSON Schema. Split the surface by format family and keep author / import / verify as internal intents inside each skill.

Rename the plugin to `interfaces`:

- Plugin directory: `plugins/contracts/` → `plugins/interfaces/`
- Slash commands: `/contracts:*` → `/interfaces:*`
- Marketplace entry: `name: contracts` → `name: interfaces`
- Plugin description: "Skills to author, import, and verify interface contracts — OpenAPI, AsyncAPI, JSON Schema, and future API/interface formats — from Specify artifacts."

This is a skill namespace rename, not a persisted artifact migration. The following names are intentionally stable:

- Schema identifier: `contracts@v1`
- Schema directory: `schemas/contracts/`
- Brief id: `contracts`
- Baseline artifact directory: `.specify/contracts/`
- Change-local artifact directory: `.specify/changes/<name>/contracts/`
- Artifact subdirectories: `contracts/http/`, `contracts/messages/`, `contracts/schemas/`
- `specify-cli` validation rule ids: `contracts.*`
- Registry project contract roles: `contracts.produces`, `contracts.consumes`, `contracts.imports`

The words `contract` and `contracts` remain correct for stored artifacts and validation rules. The word `interfaces` is reserved for the Cursor plugin / slash-command namespace where users select an authoring skill.

The new shape:

```
plugins/interfaces/skills/openapi/
├── SKILL.md          # frontmatter + critical path + intent-dispatch table
├── author.md         # generate / extend OpenAPI from spec
├── importer.md       # normalise external OpenAPI documents
├── verifier.md       # internal consistency + compatibility checks
└── references/       # OpenAPI-specific patterns and examples

plugins/interfaces/skills/asyncapi/
├── SKILL.md          # frontmatter + critical path + intent-dispatch table
├── author.md         # generate / extend AsyncAPI from spec
├── importer.md       # normalise external AsyncAPI documents
├── verifier.md       # bindings, messages, schemas, and compatibility checks
└── references/       # AsyncAPI-specific patterns and examples

plugins/interfaces/skills/json-schema/
├── SKILL.md          # frontmatter + critical path + intent-dispatch table
├── author.md         # generate / extend reusable schemas from spec
├── importer.md       # normalise external schema files
├── verifier.md       # $ref, metadata, and compatibility checks
└── references/       # JSON Schema-specific patterns and examples
```

Shared contract material stays shared. Place format-neutral references — artifact layout, baseline-vs-delta rules, `$ref` conventions, format detection, import upgrade policy, report shape, and cross-project compatibility vocabulary — under `plugins/interfaces/references/`. The three format skills link to those shared references and carry only format-specific author/import/verify guidance in their own sibling files.

Each `SKILL.md` opens with the same internal intent table, specialised to its format:


| Intent                                                              | Trigger                                                                                                     | Sibling to load |
| ------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- | --------------- |
| Author or extend the interface document from a spec                 | contracts brief during `/spec:define`; operator extending the baseline for new interactions                 | `author.md`     |
| Import or normalise an external document                            | operator drops an OpenAPI / AsyncAPI / JSON Schema file into a change's `contracts/` directory              | `importer.md`   |
| Verify internal consistency or run the cross-project consumer check | contracts brief post-merge (RFC-9 §3B); operator invoking validation against an existing interface artefact | `verifier.md`   |


Concrete changes:

- Slash commands: `/contracts:writer`, `/contracts:validator`, and `/contracts:importer` cease to exist. The new entry points are `/interfaces:openapi`, `/interfaces:asyncapi`, and `/interfaces:json-schema`.
- SKILL.md frontmatter: three names — `interfaces-openapi`, `interfaces-asyncapi`, `interfaces-json-schema` — with descriptions from §A.2 and `argument-hint: [change-dir]`.
- **Dispatch is format-first, then intent-driven.** Claude first picks the skill whose format matches the operator request or brief context, then reads the skill's intent table to load `author.md`, `importer.md`, or `verifier.md`. No explicit intent argument is parsed.
- Brief-body retargets (not frontmatter): pipelines that today mention `/contracts:writer`, `/contracts:validator`, or `/contracts:importer` retarget to the relevant `/interfaces:*` skill. The surrounding prose must name the format explicitly when the brief can infer it, or instruct Claude to select OpenAPI for HTTP/resource APIs, AsyncAPI for evented/pub-sub/streaming interfaces, and JSON Schema for shared payload schemas without a protocol wrapper. Mechanical edits touch `schemas/contracts/briefs/contracts.md`, `schemas/omnia/briefs/contracts.md`, `schemas/vectis/briefs/contracts.md`, `schemas/contracts/briefs/build.md`, and the cross-project consumer-check brief introduced by RFC-9 §3B. Brief frontmatter (`id`, `description`, `generates`, `needs`) does not name skills today and is unchanged.
- Validator's `--mode {single, cross-project}` flag becomes a verifier option inside each format skill. The mode distinction is not promoted to a top-level skill because it is a verification sub-mode, not an interface family.

Mixed-format changes use an explicit brief-level order:

1. Classify the specs or supplied documents into HTTP/resource interactions, evented/pub-sub/streaming interactions, and reusable payload schemas.
2. Run `/interfaces:json-schema` first when the change contains reusable payload vocabulary or when both OpenAPI and AsyncAPI outputs reference the same types. It owns `$id` assignment, one-type-per-file decomposition, and schema-file naming for shared payloads.
3. Run `/interfaces:openapi` for HTTP/resource interactions. It must reuse existing change-local or baseline `contracts/schemas/` files before authoring new schema files.
4. Run `/interfaces:asyncapi` for evented, pub/sub, streaming, or WebSocket-style interactions. It follows the same schema reuse rule.
5. Run the relevant verifier paths after all author/import passes complete. For mixed-format changes, the final verifier pass must check cross-format `$ref` consistency and report duplicate schema identities before the brief can complete.

If a later format skill needs a schema already authored by an earlier pass, it references that file. It must not create a competing schema with the same semantic type under a different filename or `$id`.

Worked examples:

- *Operator chat*: `/interfaces:openapi ./changes/foo` followed by "verify this for cross-project consumers" → Claude loads `interfaces/openapi/SKILL.md`, matches "verify" to `verifier.md`, and threads `--mode cross-project` through the verifier path.
- *Brief body* (`schemas/contracts/briefs/contracts.md`, post-retarget): `1. /interfaces:asyncapi — read baseline interface contracts at .specify/contracts/ and the change's specs under specs/; author the minimal AsyncAPI delta for uncovered event interactions.` → the format is explicit in the slash command and the prose intent "author the minimal AsyncAPI delta" selects `author.md`.
- *Operator import*: "Import this JSON Schema bundle into the change contracts directory" → Claude selects `/interfaces:json-schema`, loads `importer.md`, and normalises the schema files for later OpenAPI or AsyncAPI references.

Why now is the best time:

- §A's frontmatter pass already touches all three contracts SKILL.md files. Replacing lifecycle names with format names in the same pass avoids first blessing `contracts-management` and then renaming it later.
- OpenAPI and AsyncAPI have different mental models: resources, operations, status codes, parameters, and auth versus channels, messages, bindings, producers, and consumers. Their verification rules and examples should not be hidden behind a generic management skill.
- JSON Schema is already part of the advertised contract surface. Splitting only OpenAPI and AsyncAPI would orphan reusable schemas; giving JSON Schema its own skill makes shared payload shapes a first-class interface artifact.
- The split gives future formats an obvious path (`/interfaces:graphql`, `/interfaces:grpc`, `/interfaces:webhooks`, `/interfaces:mcp`) without another namespace migration.

The smaller alternative — merge the lifecycle skills into one `/contracts:management` entry point — is rejected. It removes the three generic verb names, but `management` is itself vague, and it keeps the always-loaded discovery metadata centred on an internal lifecycle abstraction rather than the interface formats operators name in practice. See alternative §B for the trade-off.

### D. House-style codification

The conventions §A and §B establish are written down in two places:

1. `**.cursor/rules/project.mdc`** — gain a "Skill authoring conventions" section that links the two Anthropic docs and asserts:
  - Frontmatter must contain only `name`, `description`, optional `argument-hint`, optional `allowed-tools`.
  - `name` must be globally unique, plugin-qualified, lowercase/hyphenated, and either gerund, action-oriented, or a noun phrase. Prefer gerunds for new skills when natural; preserve established product verbs and artifact nouns when they are more discoverable.
  - `description` must include both *what* and *when* in third person.
  - `argument-hint` must be Cursor placeholder text — single short hint with `<>` / `[]` brackets, no flag names, no `?` suffix.
  - SKILL.md ≥150 lines must lead with a "Critical Path (Quick Reference)" block.
  - SKILL.md must stay under 500 lines; longer content goes in sibling files linked one level deep.
2. `**docs/explanation/skill-authoring.md*`* — long-form companion explaining the *why*, with examples and links to the Anthropic docs. Linked from `AGENTS.md`.

A `make checks` invariant enforces the mechanical parts:

- No SKILL.md may contain a top-level `license:` key.
- Every SKILL.md `name:` must be globally unique, must start with the containing plugin's directory name plus `-`, must satisfy Anthropic's lowercase-letter / number / hyphen syntax, and must use one of the accepted naming forms from §A.1. After §C.2 and §C.3, examples include `specify-init`, `client-sow-writer`, and `interfaces-openapi`. This replaces today's check that `name:` equals the skill directory alone without forcing a one-to-one mapping between frontmatter names and slash-command directory names.
- No SKILL.md `argument-hint:` value may contain `?` (the trailing optional marker), `--` (flag dashes), or `|` (alternative-value pipe).
- No SKILL.md may exceed 500 body lines (post-frontmatter).
- Every `description:` must be ≤1024 characters.
- No active docs, schemas, skill bodies, fixtures, or tests may reference retired slash commands: `/plan:sow-writer`, `/rt:git-cloner`, `/contracts:writer`, `/contracts:validator`, `/contracts:importer`, or `/contracts:management`. Archived RFCs and this migration RFC may retain historical references under an explicit allowlist.
- `schemas/skill.schema.json` must encode the new frontmatter shape: required `name` and `description`; optional `argument-hint` and `allowed-tools`; no `license`, `compatibility`, `metadata`, `disable-model-invocation`, `when_to_use`, `user-invocable`, or `paths`.

The check follows the existing `scripts/checks.ts` pattern; once [RFC-5](../rfc-5-lint.md) lands, it migrates into `specify-check` alongside the other framework invariants.

### E. Cross-repo boundary

RFC-10 applies to both `specify` and `specify-cli`, but most changes live in `specify` because skills, plugin manifests, schema briefs, and reference docs live there. `specify-cli` changes are deliberately narrow:

- Update tests, fixtures, and examples that mention retired skill directives, especially `<!-- skill: contracts:writer -->`, to use the new `interfaces:`* directives.
- Keep `schema: contracts@v1` as the plan-entry identifier for project-less contract changes.
- Keep `.specify/contracts/` as the baseline contract location and `contracts/` as the change-local artifact directory.
- Keep merge behavior for contract artifacts unchanged.
- Keep workspace distribution of central contracts unchanged.
- Keep `contracts.*` validation rule ids and `rules_for("contracts")` unchanged unless a future schema/artifact migration RFC explicitly renames persisted contract vocabulary.

This boundary keeps the RFC-10 implementation focused on skill discovery and documentation shape. It avoids accidentally turning a skill namespace cleanup into a compatibility-breaking plan schema, merge, workspace, or validation-rule migration.

## Alternatives considered

### A. Status quo

Reject. The drift from Anthropic's published guidance compounds as the skill count grows; the metadata-budget cost is paid every session forever; and cross-plugin name collisions become harder to fix the longer they sit.

### B. Merge the contracts trio into one management skill

Considered. Collapsing `contracts/writer`, `contracts/validator`, and `contracts/importer` into a single `/contracts:management` skill with `writer.md`, `validator.md`, and `importer.md` siblings removes the immediate "three skills called `writer` / `validator` / `importer`" naming collision and centralises shared contract vocabulary. Rejected because `management` is still a vague discovery name, and the resulting skill remains organised around internal lifecycle phases rather than the interface formats operators and briefs naturally name. OpenAPI, AsyncAPI, and JSON Schema have different mental models, verification rules, examples, and failure modes; keeping them behind one generic entry point makes the always-loaded metadata less useful and risks recreating format-specific complexity inside one oversized skill. The `interfaces:*` split keeps author/import/verify as internal progressive-disclosure intents while making the top-level skill names concrete.

### C. Audit `allowed-tools` on every skill instead of dropping it everywhere

Considered. The principle-of-least-authority benefit is real. Rejected for v1 because the cost of getting the audit wrong (a writer skill missing a tool it needs at run time) is a hard halt for the operator, and the field is currently set inconsistently enough that "fix what's there" is more work than "land a uniform policy and start over". A future focused RFC can take this on.

### D. Inline the phase-outcome contract instead of factoring it

Reject. ~240 lines of repeated normative prose across four files is exactly the maintenance hazard Anthropic's [progressive-disclosure pattern](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices#progressive-disclosure-patterns) addresses. The factor-out is small (one new file, four ~80-line replacements with 4-line shims) and removes the drift hazard entirely.

### E. Promote `git-cloner` to a shared utility instead of deleting it

Considered. The skill's validation-and-error-style code could plausibly live under `plugins/references/` as a "how to clone a tree" reference. Rejected because the skill's value reduces to "remember to validate the URL and surface git's error message clearly" — Claude does this natively, and Anthropic's best-practices guide explicitly warns against packaging trivial wrappers as skills. A 5-line inlined snippet at the two call sites is cheaper and more discoverable.

### F. Rename `plan` plugin to `sow` instead of `client`

Considered. Renaming `plan` → `sow` mirrors the single existing skill 1:1 (`/sow:writer`) and is the smallest move that removes the `/spec:plan` collision. Rejected because the repository already has demand for adjacent client-facing skills (proposal authoring, pricing summaries, case studies). Naming the plugin after one of its deliverables forecloses that growth and forces a second rename later (`/sow:writer` → `/client:sow-writer`). Naming it after the *category* (`client`) absorbs the new skills without a re-org. The cost difference is a single character of typing per slash command, paid against a structural choice that lasts the lifetime of the plugin. Other names in the same shape (`delivery`, `bizdocs`) were considered briefly and rejected: `delivery` collides with the deployment / shipping connotation in this codebase's vocabulary, and `bizdocs` reads as an internal jargon abbreviation. `client` is unambiguous, short, and accurately scoped to "deliverables produced *for* a client".

### G. Layer-prefix every skill `name` (`l1-define`, `l2-execute`)

Reject. Numeric layer prefixes carry no semantic information for Claude or the operator, age badly (any new layer forces a rename), and conflict with the existing `<plugin>-<skill>` convention this RFC adopts.

## Migration plan

The work decomposes into a sequence of mechanical passes plus two focused restructures (the interfaces split and the two oversized-skill splits). Each pass is independently shippable; landing them in a single change is feasible but not required.

1. **Frontmatter sweep — the `license:` drop and `allowed-tools` policy.** Remove `license: MIT` from every SKILL.md. Remove every `allowed-tools:` line per §A.5. Trivial; mostly find-and-replace.
2. **Frontmatter sweep — `argument-hint` rewrite.** Replace each skill's `argument-hint` value per the §A.3 table. Where flags moved out of the hint, make sure the body's invocation block still documents them (most skills already do).
3. **Frontmatter sweep — descriptions.** Add "Use when…" tails per the §A.2 table. Drop RFC citations from description text. Trim block-scalar descriptions where they exceed ~250 characters.
4. **Name qualification.** Apply the §A.1 rename to every SKILL.md `name:` field. Update inbound references in `marketplace.json` (none today reference the bare `name`), in skill bodies that cross-link by name (rare; mostly handled by the slash-command form), and in `make checks` if it asserts on names.
5. **Plugin renames and deletions.** Move `plugins/plan/` → `plugins/client/`; move `plugins/contracts/` → `plugins/interfaces/`; delete `plugins/rt/skills/git-cloner/`; inline the 5-line clone snippet into `plugins/spec/skills/analyze/SKILL.md` and `plugins/rt/skills/wiretapper/SKILL.md`. Update `marketplace.json`. Update inbound references in `docs/`, `schemas/`, `AGENTS.md`, `README.md`, `.cursor/rules/`, and skill bodies that mention `/plan:sow-writer`, `/contracts:`*, `/rt:git-cloner`, or the `plan` / `contracts` plugins in their old roles. The slash-command updates affect:
  - `/plan:sow-writer` → `/client:sow-writer` (mentioned in `AGENTS.md`, `docs/reference/plugins/plan.md` (renamed to `client.md`), and the SoW skill's own examples)
  - `/contracts:writer`, `/contracts:validator`, `/contracts:importer` → `/interfaces:openapi`, `/interfaces:asyncapi`, or `/interfaces:json-schema` depending on the interface format
  - `/rt:git-cloner` → (deleted; replace with prose at the two call sites)
6. **Interfaces split.** Per §C.3: create `plugins/interfaces/skills/openapi/`, `plugins/interfaces/skills/asyncapi/`, and `plugins/interfaces/skills/json-schema/`. Each gets a SKILL.md with format-specific frontmatter and an intent-dispatch table, plus `author.md`, `importer.md`, and `verifier.md` siblings. Factor useful material from today's `writer/`, `validator/`, and `importer/` SKILL.md bodies into the corresponding siblings, split by format, then delete the three old lifecycle skill directories. Retarget brief *body* prose that names `/contracts:writer`, `/contracts:validator`, or `/contracts:importer` to the relevant `/interfaces:`* skill (mechanical edits across `schemas/contracts/briefs/contracts.md`, `schemas/contracts/briefs/build.md`, `schemas/omnia/briefs/contracts.md`, `schemas/vectis/briefs/contracts.md`, and the cross-project consumer-check brief introduced by RFC-9 §3B — brief frontmatter is unchanged because it does not name skills today). Update inbound references in `AGENTS.md`, `docs/`, `README.md`, fixtures, and any skill bodies that mention the three old slash commands.
7. **Phase-outcome factor.** Author `plugins/spec/references/phase-outcome-contract.md` from the union of the four phase skills' duplicated sections. Replace each phase skill's three duplicated sections with the 4-line shim per §B.3. Verify each phase skill's outcome-specific delta block still names the phase-unique success / failure / deferred semantics.
8. **Body splits and near-limit recounts.** Apply §B.1 to `omnia/code-reviewer` (the larger of the two); then `omnia/crate-writer`. Each split lands as a new SKILL.md under 500 lines plus 3-5 sibling reference files linked one level deep. Recount every file marked "Recount after §B.2" after adding Critical Path blocks; if any body is ≥500 lines, extract a sibling reference in the same pass. Test by re-reading each edited skill body end-to-end and confirming the Critical Path block still names every algorithmic step.
9. **House-style codification.** Add the "Skill authoring conventions" section to `.cursor/rules/project.mdc`. Author `docs/explanation/skill-authoring.md`. Cross-link from `AGENTS.md`.
10. **Schema and checks invariants.** Update `schemas/skill.schema.json` to match §D's frontmatter shape. Update `scripts/checks.ts` so the existing skill-name check requires global uniqueness, valid Anthropic name syntax, the containing plugin prefix, and one of §A.1's accepted naming forms; the retired slash-command check scans active prose references as well as HTML skill directives, and the line-count / description / argument-hint checks enforce the §D list. Run on the post-migration tree to confirm green.
11. `**specify-cli` cross-repo sweep.** Apply §E: update tests, examples, and fixtures that mention retired skill directives such as `<!-- skill: contracts:writer -->`; deliberately leave `contracts@v1`, `.specify/contracts/`, `contracts/`, `contracts.`* validation rule ids, merge behavior, and workspace contract distribution unchanged.
12. **Marketplace bump.** Bump the `.cursor-plugin/marketplace.json` version. Note the renames in the changelog under "renamed" (`/plan:sow-writer` → `/client:sow-writer`, `contracts` plugin → `interfaces` plugin), "split" (`/contracts:writer`, `/contracts:validator`, `/contracts:importer` → `/interfaces:openapi`, `/interfaces:asyncapi`, `/interfaces:json-schema`), and "removed" (`/rt:git-cloner`) sections.

The total touch surface is large but mechanical: ~28 SKILL.md frontmatter rewrites (29 current files minus `rt/git-cloner` deleted, with the three contracts lifecycle skills replaced by three interfaces format skills), four phase-skill body simplifications, two guaranteed body splits plus any near-limit splits triggered by recount, one interfaces split with brief-body prose retargets, shared interface references, one new phase-outcome reference, one new explanatory doc, two plugin directory renames, several skill directory moves/deletions, `schemas/skill.schema.json`, `scripts/checks.ts`, and a narrow `specify-cli` test/example sweep. No CLI verb changes, no persisted schema identifier changes, no JSON shape changes, and no contract artifact path changes. The slash-command form `/plugin:skill` changes for `/plan:sow-writer` (now `/client:sow-writer`), the contracts trio (now `/interfaces:openapi`, `/interfaces:asyncapi`, or `/interfaces:json-schema`), and `/rt:git-cloner` (deleted); fixture transcripts that exercise those surfaces re-baseline as part of the corresponding pass.

## Recommendation

Adopt §A in full (frontmatter standard), §B.1 for the two skills currently over the 500-line line plus the near-limit recount rule in §B.2, §B.3 (phase-outcome factor), §C.1 (delete `rt/git-cloner`), §C.2 (rename `plan` plugin to `client`), §C.3 (rename `contracts` to `interfaces` and split the lifecycle trio into OpenAPI, AsyncAPI, and JSON Schema skills), §D (house-style codification), and §E (cross-repo boundary).