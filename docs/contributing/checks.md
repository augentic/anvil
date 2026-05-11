# Consistency Checks

The `specify` repo includes an automated consistency checker at `scripts/checks.ts` that validates documentation, skills, capability manifests, and the marketplace manifest. Run it before every pull request.

## Running checks

```bash
make checks
```

This runs `scripts/checks.ts` via [Deno](https://deno.land):

```bash
deno run --allow-read --allow-env scripts/checks.ts
```

Exit code `0` means all checks pass. Any failure prints `FAIL: <description>` and exits non-zero with a count of failures.

## What the checks enforce

### 1. Markdown link resolution

Every relative link in every `.md` file must resolve to an existing file. External links (`http://`, `mailto:`, `#` anchors) and `src/` paths are skipped. Fenced code blocks and HTML comments are stripped before scanning.

**Common fix:** update the link target or remove a stale link.

### 2. Stale claims

No markdown file may reference a stale checklist count from an earlier version of the documentation. The specific patterns are defined in `scripts/checks.ts`.

### 3. Capability manifest YAML validation

Every `capabilities/<name>/capability.yaml` file (at most two directory levels deep) must validate against `capabilities/capability.schema.json` using JSON Schema 2020-12.

**Common fix:** check that all required fields (`name`, `version`, `description`, `pipeline`) are present and correctly typed.

### 4. Capability referential integrity

For each `capability.yaml`, the check validates:

- **Brief files exist** -- every pipeline entry's `brief` path resolves to a file
- **Frontmatter present** -- each brief file has valid YAML frontmatter between `---` markers
- **ID match** -- the brief frontmatter `id` matches the pipeline entry `id`
- **Needs resolution** -- every `needs` reference in a brief points to a declared pipeline `id`
- **No cycles** -- the `needs` dependency graph is acyclic (verified by Kahn's topological sort)

**Common fix:** ensure the brief's frontmatter `id` matches the pipeline entry, and that `needs` references use exact `id` values from the same capability.

### 5. Symlink integrity

Every symlink under `plugins/` must resolve to a valid target. Skills often symlink shared references (e.g. `plugins/references/specify.md`) into their `references/` directory.

**Common fix:** recreate the symlink if the target was moved or renamed.

### 6. SKILL.md frontmatter validation

Every `SKILL.md` under `plugins/` is validated against `.cursor/schemas/skill.schema.json`:

- **Required fields** -- `name` (kebab-case) and `description` (minimum 10 characters)
- **Name match** -- the `name` field must match the parent directory name
- **Known tools** -- every entry in `allowed-tools` must be a recognized Cursor tool name or match the `mcp__*` prefix

The recognized tool set includes: `Read`, `Write`, `StrReplace`, `Shell`, `Grep`, `Glob`, `ReadLints`, `WebFetch`, `WebSearch`, `AskQuestion`, `Task`, `TodoWrite`, `SemanticSearch`, `EditNotebook`, `GenerateImage`.

Long `SKILL.md` bodies are also checked for structure: bodies over 250 post-frontmatter lines fail (per-file `bodyLineCount` baselines in `scripts/standards-allowlist.toml` grandfather oversized files), and bodies with at least 150 post-frontmatter lines must include a `## Critical Path` section with 5-7 bullets, numbered items, or `### N. Title` H3 step headings.

### 7. Skill reference link resolution

Links in `SKILL.md` bodies that point to `references/...` or `examples/...` paths are resolved relative to the skill directory. Every such link must resolve to an existing file.

### 8. Skill variable consistency

For skills that declare an `## Arguments` or `## Derived Arguments` section with `$VARIABLE = ...` definitions in ` ```text` blocks:

- Every defined variable must be referenced somewhere in the skill body
- Every `$VARIABLE` reference in the body (outside fenced blocks) must have a definition in the arguments section

Built-in variables (`$ARGUMENTS`, `$HOME`) are excluded from the check.

### 9. Skill directive validation

`<!-- skill: plugin:skill-name -->` directives in markdown files must reference a real skill. The check walks `plugins/` to build a registry of `plugin → skill` mappings and validates every directive against it. Files under `rfcs/` are excluded.

### 10. Marketplace manifest consistency

Cross-checks `plugins/` against `.cursor-plugin/marketplace.json`:

- Every plugin with a `.cursor-plugin/plugin.json` file must be listed in the manifest
- Every plugin listed in the manifest must have a `skills/` directory

### 12. Instruction file preambles

Files matching `capabilities/**/instructions/<name>.md` must contain an output location preamble:

```markdown
> **Output location**: `.specify/slices/...`
```

This prevents cross-plugin path contamination by making every instruction file declare where its output goes.

### 14. Acceptance scenario frontmatter

Acceptance scenario files are validated against `.cursor/schemas/scenario.schema.json` (JSON Schema 2020-12, validated through the same Ajv2020 path as the SKILL.md schema). Discovery follows these opt-in roots:

1. `tests/<suite>/scenario.md` — shared outside-in suites.
2. `tests/suites/<suite>/scenario.md` — legacy shared outside-in suites, when present.
3. `tests/plan/<scenario>.md` — shared plan-generation scenarios.
4. `capabilities/<capability>/tests/<scenario>.md` — flat owner-local capability scenarios.
5. `capabilities/<capability>/tests/<scenario>/scenario.md` — directory-form owner-local capability scenarios.
6. `plugins/<plugin>/skills/<skill>/fixtures/<scenario>/scenario.md` — promoted skill-owned fixtures.

Discovery is **opt-in by frontmatter**: a markdown file under one of those roots is validated only if it begins with a YAML frontmatter block (`---`). Prose-only docs in those roots — `tests/README.md`, `run-summary-template.md`, narrative — are skipped silently. Shared suites include the cross-repo manual acceptance scenario under [`tests/cross-repo/`](../../tests/cross-repo/) and the plan-generation scenario pack under [`tests/plan/`](../../tests/plan/). The first owner-local capability pack is the contracts test suite under [`capabilities/contracts/tests/`](../../capabilities/contracts/tests/README.md).

An opt-in scenario looks like:

```markdown
---
id: contracts-describe
owner: contracts
kind: capability
capability: contracts@v1
backend: manual
entrypoint: /spec:define
stages: [define, build, merge]
isolation: fresh-project
authorship-mode: prose
assertions:
  - files-exist
  - contract-validator-clean
expected-artifacts:
  - contracts/schemas/profile.yaml
negative-expectations:
  - artifacts-outside-contracts-directory
---

# Scenario Title

Scenario ID: `contracts-describe`
```

The check enforces:

- **Schema conformance** — `id`, `owner`, `kind`, `backend`, `entrypoint`, `stages`, `isolation` are required; `capability` is required when `kind` is `capability` or `capability-boundary`; `negative-expectations` is required (with at least one entry) when `kind` is `capability-boundary`. `kind` is an open enum (`capability`, `capability-boundary`, `suite`, `skill`); only the first two are actively required by C02. `backend` ∈ {`manual`, `agent`, `recorded`, `fixture`}. `isolation` ∈ {`fresh-project`, `shared-baseline`, `shared-slice`}. `capability` matches `^[a-z][a-z0-9-]*@v\d+$`. `entrypoint` matches `^/[a-z]+:[a-z][a-z0-9-]*$`. `id` matches `^[a-z][a-z0-9-]*$`.
- **Stages prefix** — `stages` must be a contiguous prefix of `[define, build, merge, drop]` starting at `define`. `[define, build, merge]` is valid; `[build, define]`, `[define, merge]`, `[merge]` are not.
- **Body-id consistency** — when the visible `Scenario ID:` body line is present (C02 doubles the id in prose for resilience against environments that suppress frontmatter), it must equal the frontmatter `id`.
- **Expected-artifact path safety** — every entry in `expected-artifacts` must be a relative path with no `..` segments and no leading `/`. The check stops short of pinning a per-capability prefix (e.g. `contracts/`) so future capabilities are not over-constrained.
- **Cross-file id uniqueness** — every opted-in scenario `id` is unique across the repo; duplicates are reported with both file paths.

Internal markdown link resolution within scenarios is handled by check 1 (markdown link resolution); the scenario validator does not duplicate it.

**Example failure messages:**

```text
FAIL: Scenario frontmatter: capabilities/contracts/tests/_probe.md — / must have required property 'negative-expectations'
FAIL: Scenario frontmatter: capabilities/contracts/tests/_probe.md — stages must be a contiguous prefix of [define, build, merge, drop] starting at 'define'; got ["build","define"]
FAIL: Scenario frontmatter: capabilities/contracts/tests/_probe.md — body 'Scenario ID: `contracts-foo`' does not match frontmatter id 'contracts-bar'; align the visible line with the frontmatter id
FAIL: Scenario frontmatter: capabilities/contracts/tests/_probe.md — expected-artifact '../escape.yaml' must not escape the scenario workspace ('..' segment not allowed)
FAIL: Scenario frontmatter: duplicate scenario id 'contracts-describe' across files: capabilities/contracts/tests/_probe.md, capabilities/contracts/tests/describe.md
```

Common fixes: align `kind`/`capability` per the schema, walk back `stages` to a contiguous prefix starting at `define`, keep the body `Scenario ID:` line in lockstep with the frontmatter `id`, rewrite expected-artifact paths to be relative to the scenario workspace root, and ensure new scenario ids are unique.

### 15. Recorded trace freshness

The recorded-trace check is opt-in. If a future suite adds
`tests/recorded/**/*.jsonl`, every trace must lead with a
`recorded-trace-header` line carrying `schemaVersion: 1`, `sourceBackend`,
`sourceRunId`, `sourceTimestamp`, and `scenarioId`.

### 16. First-party codex rule shape

First-party codex rule files are validated under `capabilities/*/codex/**/*.md`.
The optional repo-root `codex/**/*.md` overlay is also included when present.

The check is format-only. It does not run consumer-project review and does not
invoke the `specify` CLI validator. It validates:

- **Frontmatter schema** -- each file must begin with YAML frontmatter that
  conforms to `.cursor/schemas/codex-rule.schema.json`, mirrored from the CLI
  schema at `specify-cli/schemas/codex-rule.schema.json`.
- **Required body heading** -- each rule body must include a `## Rule` heading.
- **Cross-file id uniqueness** -- every codex `id` must be unique across the
  discovered first-party rule set.
- **Namespace ownership** -- `default` owns `UNI-*`; `omnia` owns `OMNIA-*`,
  `RUST-*`, and `SEC-*`; `contracts` owns `IFACE-*`; `vectis` owns `VECTIS-*`.

**Example failure messages:**

```text
FAIL: Codex rule frontmatter: capabilities/default/codex/example.md — / missing required property 'trigger'
FAIL: Codex rule frontmatter: capabilities/default/codex/example.md — /severity must be one of "critical", "important", "suggestion", "optional"
FAIL: Codex rule body: capabilities/default/codex/example.md — missing required '## Rule' heading
FAIL: Codex namespace ownership: capabilities/default/codex/example.md — capability 'default' may only use UNI-* ids, got 'SEC-001'
FAIL: Codex rule duplicate id 'UNI-001' across files: capabilities/default/codex/a.md, capabilities/default/codex/b.md
```

Common fixes: add the required `id`, `title`, `severity`, and `trigger`
frontmatter fields; use canonical severity values (`critical`, `important`,
`suggestion`, `optional`) and review modes (`deterministic`,
`model-assisted`, `hybrid`); keep ids in the reserved namespace-plus-three-digit
shape such as `UNI-001`; add the `## Rule` heading; and coordinate with content
subagents before reusing or moving ids between capability-owned namespaces.

## Extending the checks

To add a new check:

1. Write an `async function` in `scripts/checks.ts` following the existing pattern.
2. Call `fail(msg)` for each violation -- this increments the error counter and prints the failure.
3. Add the function to one of the `Promise.all` groups at the bottom of the file. Independent checks can run in the same group; checks that depend on earlier results go in a later group.
4. Run `make checks` to verify the new check works.

The checks are numbered but the numbers are not contiguous (check 11 does not exist). New checks should use the next available number.

## CLI checks

The specify-cli repo has its own check suite via `cargo-make`:

```bash
cargo make ci     # lint, test, test-docs, vet, outdated, deny, fmt
cargo make check  # audit, fmt, lint, outdated, deps
```

These are Rust-specific checks (clippy, formatting, dependency auditing, test suite) and are separate from the documentation checks in the specify repo.
