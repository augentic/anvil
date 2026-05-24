# Consistency Checks

The `specify` repo includes an automated consistency checker at `scripts/check.ts` that validates documentation, skills, adapter manifests, and the marketplace manifest. Run it before every pull request.

## Running checks

```bash
make check
```

This runs `scripts/check.ts` via [Deno](https://deno.land):

```bash
deno run --allow-read --allow-env scripts/check.ts
```

Exit code `0` means all checks pass. Any failure prints `FAIL: <description>` and exits non-zero with a count of failures.

## What the checks enforce

### 1. Markdown link resolution

Every relative link in every `.md` file must resolve to an existing file. External links (`http://`, `mailto:`, `#` anchors) and `src/` paths are skipped. Fenced code blocks and HTML comments are stripped before scanning.

**Common fix:** update the link target or remove a stale link.

### 2. Stale claims

No markdown file may reference a stale checklist count from an earlier version of the documentation. The specific patterns are defined in `scripts/check.ts`.

### 3. Adapter manifest YAML validation

Every `adapters/sources/<name>/adapter.yaml` validates against `source.schema.json`, and every `adapters/targets/<name>/adapter.yaml` validates against `target.schema.json`. Both schemas ship with the `specify-cli` binary under `schemas/` and are loaded by `scripts/checks/adapter.ts` through the `SPECIFY_CLI_DIR` resolver (defaults to `../specify-cli`).

**Common fix:** check that all required fields (`name`, `version`, `axis`, `operations`, `briefs`) are present and that `operations` matches the per-axis enum (`enumerate` + `extract` for sources; `shape` + `build` + `merge` for targets).

### 4. Adapter referential integrity

The 1.x pipeline-graph integrity check retired at the 2.0 cut — manifests no longer carry a `pipeline:` field. Brief existence and operation coverage are now enforced by the per-axis schemas (`source.schema.json` / `target.schema.json`).

### 5. Symlink integrity

Every symlink under `plugins/` must resolve to a valid target.

The companion `checkAgentTeamsCanonical` predicate additionally enforces the cross-tree canonicalisation for the per-target-adapter `agent-teams.md` overlays. Each `adapters/targets/<name>/references/agent-teams.md` must be either a real symlink resolving to `docs/reference/review-team-protocol.md` or a regular file whose SHA-256 matches the canonical doc. The symlink form is preferred; the SHA-256 fallback keeps the door open for adapters that need a non-symlink layout without inviting silent content drift.

**Common fix:** recreate the symlink if the target was moved or renamed; if the file diverged, replace it with a symlink or re-sync its content from the canonical doc.

### 6. SKILL.md frontmatter validation

Every `SKILL.md` under `plugins/` is validated against `.cursor/schemas/skill.schema.json`:

- **Required fields** -- `name` (kebab-case) and `description` (minimum 10 characters)
- **Name match** -- the `name` field must match the parent directory name
- **Known tools** -- every entry in `allowed-tools` must be a recognized Cursor tool name or match the `mcp__*` prefix

The recognized tool set includes: `Read`, `Write`, `StrReplace`, `Shell`, `Grep`, `Glob`, `ReadLints`, `WebFetch`, `WebSearch`, `AskQuestion`, `Task`, `TodoWrite`, `SemanticSearch`, `EditNotebook`, `GenerateImage`.

Long `SKILL.md` bodies are also checked for structure: bodies over 200 post-frontmatter lines fail (strict — no grandfathering), and bodies with at least 150 post-frontmatter lines must include a `## Critical Path` section with 5-7 bullets, numbered items, or `### N. Title` H3 step headings.

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

Files matching `adapters/targets/**/instructions/<name>.md` must contain an output location preamble:

```markdown
> **Output location**: `.specify/slices/...`
```

This prevents cross-plugin path contamination by making every instruction file declare where its output goes.

### 14. Acceptance scenario frontmatter

Acceptance scenario files are validated against `.cursor/schemas/scenario.schema.json` (JSON Schema 2020-12, validated through the same Ajv2020 path as the SKILL.md schema). Discovery follows these opt-in roots:

1. `tests/<suite>/scenario.md` — shared outside-in suites.
2. `tests/suites/<suite>/scenario.md` — legacy shared outside-in suites, when present.
3. `tests/plan/<scenario>.md` — shared plan-generation scenarios.
4. `adapters/targets/<target>/tests/<scenario>.md` — flat owner-local target scenarios.
5. `adapters/targets/<target>/tests/<scenario>/scenario.md` — directory-form owner-local target scenarios.
6. `plugins/<plugin>/skills/<skill>/fixtures/<scenario>/scenario.md` — promoted skill-owned fixtures.

Discovery is **opt-in by frontmatter**: a markdown file under one of those roots is validated only if it begins with a YAML frontmatter block (`---`). Prose-only docs in those roots — `tests/README.md`, `run-summary-template.md`, narrative — are skipped silently. Shared suites include the cross-repo manual acceptance scenario under [`tests/cross-repo/`](../../tests/cross-repo/) and the plan-generation scenario pack under [`tests/plan/`](../../tests/plan/). The first owner-local target pack is the contracts test suite under [`adapters/targets/contracts/tests/`](../../adapters/targets/contracts/tests/README.md).

An opt-in scenario looks like:

```markdown
---
id: contracts-describe
owner: contracts
kind: adapter
adapter: contracts@v1
backend: manual
entrypoint: /spec:refine
stages: [define, build, merge]
isolation: fresh-project
authorship-mode: prose
assertions:
  - files-exist
  - contract-validator-clean
expected-artifacts:
  - contracts/schemas/adapter.yaml
negative-expectations:
  - artifacts-outside-contracts-directory
---

# Scenario Title

Scenario ID: `contracts-describe`
```

The check enforces:

- **Schema conformance** — `id`, `owner`, `kind`, `backend`, `entrypoint`, `stages`, `isolation` are required; `adapter` is required when `kind` is `adapter` or `adapter-boundary`; `negative-expectations` is required (with at least one entry) when `kind` is `adapter-boundary`. `kind` is an open enum (`adapter`, `adapter-boundary`, `suite`, `skill`); only the first two are actively required by C02. `backend` ∈ {`manual`, `agent`, `recorded`, `fixture`}. `isolation` ∈ {`fresh-project`, `shared-baseline`, `shared-slice`}. `adapter` matches `^[a-z][a-z0-9-]*@v\d+$`. `entrypoint` matches `^/[a-z]+:[a-z][a-z0-9-]*$`. `id` matches `^[a-z][a-z0-9-]*$`.
- **Stages prefix** — `stages` must be a contiguous prefix of `[define, build, merge, drop]` starting at `define`. `[define, build, merge]` is valid; `[build, define]`, `[define, merge]`, `[merge]` are not.
- **Body-id consistency** — when the visible `Scenario ID:` body line is present (C02 doubles the id in prose for resilience against environments that suppress frontmatter), it must equal the frontmatter `id`.
- **Expected-artifact path safety** — every entry in `expected-artifacts` must be a relative path with no `..` segments and no leading `/`. The check stops short of pinning a per-adapter prefix (e.g. `contracts/`) so future adapters are not over-constrained.
- **Cross-file id uniqueness** — every opted-in scenario `id` is unique across the repo; duplicates are reported with both file paths.

Internal markdown link resolution within scenarios is handled by check 1 (markdown link resolution); the scenario validator does not duplicate it.

**Example failure messages:**

```text
FAIL: Scenario frontmatter: adapters/targets/contracts/tests/_probe.md — / must have required property 'negative-expectations'
FAIL: Scenario frontmatter: adapters/targets/contracts/tests/_probe.md — stages must be a contiguous prefix of [define, build, merge, drop] starting at 'define'; got ["build","define"]
FAIL: Scenario frontmatter: adapters/targets/contracts/tests/_probe.md — body 'Scenario ID: `contracts-foo`' does not match frontmatter id 'contracts-bar'; align the visible line with the frontmatter id
FAIL: Scenario frontmatter: adapters/targets/contracts/tests/_probe.md — expected-artifact '../escape.yaml' must not escape the scenario workspace ('..' segment not allowed)
FAIL: Scenario frontmatter: duplicate scenario id 'contracts-describe' across files: adapters/targets/contracts/tests/_probe.md, adapters/targets/contracts/tests/describe.md
```

Common fixes: align `kind`/`adapter` per the schema, walk back `stages` to a contiguous prefix starting at `define`, keep the body `Scenario ID:` line in lockstep with the frontmatter `id`, rewrite expected-artifact paths to be relative to the scenario workspace root, and ensure new scenario ids are unique.

### 15. Recorded trace freshness

The recorded-trace check is opt-in. If a future suite adds
`tests/recorded/**/*.jsonl`, every trace must lead with a
`recorded-trace-header` line carrying `schemaVersion: 1`, `sourceBackend`,
`sourceRunId`, `sourceTimestamp`, and `scenarioId`.

### 16. First-party codex rule shape

First-party codex rule files are validated in the shared tree at `adapters/shared/codex/universal/**/*.md` (UNI-* rules) and in per-adapter overlays at `adapters/sources/*/codex/**/*.md` and `adapters/targets/<name>/codex/**/*.md`.

The check is format-only. It does not run consumer-project review and does not
invoke any external validator. It validates:

- **Frontmatter schema** -- each file must begin with YAML frontmatter that
  conforms to `.cursor/schemas/codex-rule.schema.json`.
- **Required body heading** -- each rule body must include a `## Rule` heading.
- **Cross-file id uniqueness** -- every codex `id` must be unique across the
  discovered first-party rule set.
- **Namespace ownership** -- `adapters/shared/codex/universal/` owns `UNI-*`;
  `omnia` owns `OMNIA-*`, `RUST-*`, and `SEC-*`; `contracts` owns `IFACE-*`;
  `vectis` owns `VECTIS-*`.

**Example failure messages:**

```text
FAIL: Codex rule frontmatter: adapters/shared/codex/universal/example.md — / missing required property 'trigger'
FAIL: Codex rule frontmatter: adapters/shared/codex/universal/example.md — /severity must be one of "critical", "important", "suggestion", "optional"
FAIL: Codex rule body: adapters/shared/codex/universal/example.md — missing required '## Rule' heading
FAIL: Codex namespace ownership: adapters/shared/codex/universal/example.md — codex owner 'universal' may only use UNI-* ids, got 'SEC-001'
FAIL: Codex rule duplicate id 'UNI-001' across files: adapters/shared/codex/universal/a.md, adapters/shared/codex/universal/b.md
```

Common fixes: add the required `id`, `title`, `severity`, and `trigger`
frontmatter fields; use canonical severity values (`critical`, `important`,
`suggestion`, `optional`) and review modes (`deterministic`,
`model-assisted`, `hybrid`); keep ids in the reserved namespace-plus-three-digit
shape such as `UNI-001`; add the `## Rule` heading; and coordinate with content
subagents before reusing or moving ids between adapter-owned namespaces.

## Extending the checks

To add a new check:

1. Write an `async function` in `scripts/check.ts` following the existing pattern.
2. Call `fail(msg)` for each violation -- this increments the error counter and prints the failure.
3. Add the function to one of the `Promise.all` groups at the bottom of the file. Independent checks can run in the same group; checks that depend on earlier results go in a later group.
4. Run `make check` to verify the new check works.

The checks are numbered but the numbers are not contiguous (check 11 does not exist). New checks should use the next available number.

## CLI checks

The specify-cli repo has its own check suite via `cargo-make`:

```bash
cargo make ci     # lint, test, test-docs, vet, outdated, deny, fmt
cargo make check  # audit, fmt, lint, outdated, deps
```

These are Rust-specific checks (clippy, formatting, dependency auditing, test suite) and are separate from the documentation checks in the specify repo.
