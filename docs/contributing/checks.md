# Consistency Checks

The `specify` repo is linted by the `specdev` authoring binary from `augentic/specify-cli`. `make lint` forwards to `specdev lint`; CI runs the same binary in release mode. Run checks before every pull request.

## Editor-first vs specdev lint

Framework validation splits into two surfaces:

| Surface | When it runs | What it covers |
| --- | --- | --- |
| **Editor-first (YAML/JSON LSP)** | While you edit plain YAML or JSON | Shape violations for files the language server can bind to a schema: `adapter.yaml`, `.cursor-plugin/marketplace.json`, and other plain YAML/JSON artifacts that declare a schema |
| **`specdev lint` (Markdown + cross-file)** | Local `make lint`, CI, and direct `cargo run … --bin specdev -- lint --framework-root .` | Markdown frontmatter (`SKILL.md`, rules, scenario docs), symlink integrity, marketplace ↔ plugin consistency, link resolution, and every other predicate schemas cannot express |

**Authoritative schemas** live in the `specify-authoring` crate under `crates/authoring/schemas/`. [`.cursor/schemas/`](../../.cursor/schemas/) holds editor-facing copies so Cursor's JSON/YAML language servers resolve the same contract.

**Plain YAML/JSON wiring.** Adapter manifests carry a first-line schema directive (and [`.vscode/settings.json`](../../.vscode/settings.json) binds `adapters/sources/*/adapter.yaml` and `adapters/targets/*/adapter.yaml` to the runtime schemas for editor squiggles):

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/augentic/specify-cli/main/schemas/source.schema.json
```

Use the same pattern for other plain YAML files when a framework or runtime schema exists. Runtime adapter schemas ship with `specify-cli` under `schemas/`; framework-only schemas (skill frontmatter shape, rules, scenarios, marketplace) ship in `crates/authoring/schemas/`. JSON manifests can use a top-level `"$schema"` property — see [`.cursor-plugin/marketplace.json`](../../.cursor-plugin/marketplace.json).

**Markdown frontmatter.** Cursor's YAML language server validates standalone `.yaml` control files reliably, but does not yet surface the same diagnostics for YAML embedded in Markdown frontmatter. Until a frontmatter-aware editor integration lands, `specdev lint` extracts the leading `---` block from `SKILL.md`, rules, and scenario Markdown files and validates it against the same JSON Schemas in `crates/authoring/schemas/`.

## Enforcement surfaces (authoring vs engineering standards)

Framework and consumer validation are intentionally separate. See [Standards layer](../explanation/standards-layer.md).

| Surface | Command | Audience | Enforces |
| --- | --- | --- | --- |
| **Authoring standards** | `specdev lint` (`make lint`) | `augentic/specify` contributors | Skill frontmatter, rule *shape*, links, marketplace consistency |
| **Engineering standards** | `specrun lint` | Consumer projects with `.specify/` | Applicable rules with `deterministic_hints`; structured findings for CI |
| **Build-time judgment** | Target `build/review.md` briefs | Active slice during `/spec:build` | Model-assisted codex policy → `REVIEW.md` |

Rule *content* lives under `adapters/**/rules/` (engineering standards). `docs/standards/` is **authoring** house style only.

## Running checks

```bash
make lint
```

This runs `cargo run --release --manifest-path ../specify-cli/Cargo.toml --bin specdev -- lint --framework-root .`. Exit code `0` means all checks pass. Validation failures exit `2`; infrastructure errors exit `1`.

Tooling contributors run the full local CI subset with:

```bash
make ci
```

`make ci` runs `lint`. When a full `specify-cli/` checkout exists at the repo root (CI layout), the Makefile uses it; otherwise it defaults to the sibling `../specify-cli` checkout. The `specify-authoring` predicate regression suite is owned by `specify-cli` and runs there via `cargo make test`; this repo does not re-run it.

Tooling contributors can also invoke the binary directly, and run the predicate suite from a `specify-cli` checkout:

```bash
cargo run --release --manifest-path ../specify-cli/Cargo.toml --bin specdev -- lint --framework-root .
cargo test --manifest-path ../specify-cli/Cargo.toml -p specify-authoring
```

The repo also ships a workspace `[alias]` shortcut in [`.cargo/config.toml`](../../.cargo/config.toml) so `cargo fcheck` runs the framework-checker from any directory at or below the framework root without `--manifest-path` boilerplate.

Set `SPECDEV_FRAMEWORK_ROOT` only when invoking `specdev` directly without `--framework-root`. Adapter schemas are loaded from the local `specify-cli` workspace.

### Diagnostic format

Each finding prints on stderr as:

```text
FAIL: <rule-id>: <message>
  at <repo-relative-path>:<line>
```

`<rule-id>` is stable kebab-case (for example `links.unresolved`, `scenarios.schema-violation`, `codex.namespace-ownership-violation`). The location line is omitted when a finding is repo-wide (duplicate ids, missing checkout). A summary line reports the total failure count; success prints `All checks passed.` on stdout.

| Rule id prefix | Check module | Topic |
| --- | --- | --- |
| `adapter.*` | `check::adapter` | Adapter manifest schema and missing manifests |
| `links.*` | `check::links`, `check::schema_links` | Markdown links, skill references, skill directives, tool-owned schema URLs |
| `skill.*` | `check::skill_frontmatter`, `check::skill_body` | SKILL.md frontmatter and body discipline |
| `scenarios.*` | `check::scenarios` | Acceptance scenario frontmatter and recorded traces |
| `codex.*` | `check::rules` | Rule shape and namespace ownership |

See the `specify-authoring` crate's `check` module for the full predicate list.

### JSON output

`specdev lint` can emit the same structured result shape consumed by CI integrations. Run `specdev lint --format json` (or set `SPECDEV_FORMAT=json`) to swap the human-oriented stderr stream for a single structured envelope written to stdout. Default `text` output remains canonical for humans; reach for `--format json` when wiring CI annotations, preparing dashboards, or comparing authoring findings with consumer-project `specrun lint` output.

```bash
specdev lint --framework-root . --format json | jq '.findings[] | select(.severity == "critical")'
```

Envelope shape:

```json
{
  "version": 1,
  "summary": { "critical": 0, "important": 0, "suggestion": 0, "optional": 0 },
  "findings": []
}
```

The full wire contract, including per-finding fields and the canonical fingerprint algorithm, is pinned by the CLI schemas: `schemas/lint/lint-result.schema.json` for the envelope, `schemas/lint/finding.schema.json` for each `LintFinding`, and `crates/authoring/schemas/rule.schema.json` for rule authoring shape.

Exit codes follow the existing semantics — `0` on a clean tree, `2` when findings are present (validation failed), `1` on infrastructure errors. On a `1`, the JSON envelope on stdout collapses to `{"version": 1, "summary": {…all zero}, "findings": []}` and the underlying error surfaces on stderr.

**Severity mapping.** Authoring imperative rule ids map to `LintFinding` severities through the table in [`src/authoring/severity.rs`](https://github.com/augentic/specify-cli/blob/main/src/authoring/severity.rs):

- `rules.schema-violation` → `critical` — a malformed rule breaks every downstream consumer of the resolved rules export.
- every other authoring family (`adapter.*`, `codex.duplicate-rule-id`, `codex.namespace-ownership-violation`, `links.*`, `scenarios.*`, `skill.*`, …) → `important`.
- unclassified rule ids fall through to the `important` default.

**`rule-id` is null for authoring findings.** The wire schema's `rule-id` field is constrained to the closed codex regex `^(UNI|SRC|FRAME|RUST|IFACE|SEC|OMNIA|VECTIS|ORG)-[0-9]{3}$`, which authoring imperative ids like `rules.schema-violation` and `skill.unknown-tool` do not match. The [authoring mapper](https://github.com/augentic/specify-cli/blob/main/src/authoring/map_finding.rs) therefore emits `rule_id: null` and preserves the authoring id as a `[rule_id]` prefix on the `title` field (e.g. `"[rules.schema-violation] Rule frontmatter failed schema validation."`). This is transitional; a future framework-rules migration may move authoring ids into a declarative `FRAME-NNN` codex namespace, at which point `rule-id` becomes populated and the bracketed title prefix retires.

**Consumer-project counterpart.** `specdev lint --format json` is the **framework-repo** authoring surface; `specrun lint` is its **consumer-project** counterpart, scanning `.specify/`-bearing trees with deterministic codex hints. Both emit the same `LintFinding` envelope so CI tooling, dashboards, and PR bots that consume one can consume the other unchanged. See [Standards layer](../explanation/standards-layer.md) for the consumer-side scanner contract.

## What the checks enforce

### 1. Markdown link resolution

Every relative link in every `.md` file must resolve to an existing file. External links (`http://`, `mailto:`, `#` anchors) and `src/` paths are skipped. Fenced code blocks and HTML comments are stripped before scanning.

**Common fix:** update the link target or remove a stale link.

### 2. Adapter manifest YAML validation

Every `adapters/sources/<name>/adapter.yaml` validates against `source.schema.json`, and every `adapters/targets/<name>/adapter.yaml` validates against `target.schema.json`. Both schemas ship with `specify-cli` under `schemas/` and are loaded by the `specify-authoring` crate.

**Common fix:** check that all required fields (`name`, `version`, `axis`, `operations`, `briefs`) are present and that `operations` matches the per-axis enum (`survey` + `extract` for sources; `shape` + `build` + `merge` for targets).

### 3. Adapter referential integrity

The 1.x pipeline-graph integrity check retired at the 2.0 cut — manifests no longer carry a `pipeline:` field. Brief existence and operation coverage are now enforced by the per-axis schemas (`source.schema.json` / `target.schema.json`).

### 4. Symlink integrity

Every symlink under `plugins/` must resolve to a valid target.

The companion `checkAgentTeamsCanonical` predicate additionally enforces the cross-tree canonicalisation for the per-target-adapter `agent-teams.md` overlays. Each `adapters/targets/<name>/references/agent-teams.md` must be either a real symlink resolving to `docs/reference/review-team-protocol.md` or a regular file whose SHA-256 matches the canonical doc. The symlink form is preferred; the SHA-256 fallback keeps the door open for adapters that need a non-symlink layout without inviting silent content drift.

**Common fix:** recreate the symlink if the target was moved or renamed; if the file diverged, replace it with a symlink or re-sync its content from the canonical doc.

### 5. SKILL.md frontmatter validation

Every `SKILL.md` under `plugins/` is validated against the `specify-authoring` skill schema (editor alias: [`.cursor/schemas/skill.schema.json`](../../.cursor/schemas/skill.schema.json)):

- **Required fields** -- `name` (kebab-case) and `description` (minimum 10 characters)
- **Name match** -- the `name` field must match the parent directory name
- **Known tools** -- every entry in `allowed-tools` must be a recognized Cursor tool name or match the `mcp__*` prefix

The recognized tool set includes: `Read`, `Write`, `StrReplace`, `Shell`, `Grep`, `Glob`, `ReadLints`, `WebFetch`, `WebSearch`, `AskQuestion`, `Task`, `TodoWrite`, `SemanticSearch`, `EditNotebook`, `GenerateImage`.

Long `SKILL.md` bodies are also checked for structure: bodies over 200 post-frontmatter lines fail (strict — no grandfathering), and bodies with at least 150 post-frontmatter lines must include a `## Critical Path` section with 5-7 bullets, numbered items, or `### N. Title` H3 step headings.

### 6. Skill reference link resolution

Links in `SKILL.md` bodies that point to `references/...` or `examples/...` paths are resolved relative to the skill directory. Every such link must resolve to an existing file.

### 7. Skill variable consistency

For skills that declare an `## Arguments` or `## Derived Arguments` section with `$VARIABLE = ...` definitions in ` ```text` blocks:

- Every defined variable must be referenced somewhere in the skill body
- Every `$VARIABLE` reference in the body (outside fenced blocks) must have a definition in the arguments section

Built-in variables (`$ARGUMENTS`, `$HOME`) are excluded from the check.

### 8. Skill directive validation

`<!-- skill: plugin:skill-name -->` directives in markdown files must reference a real skill. The check walks `plugins/` to build a registry of `plugin → skill` mappings and validates every directive against it. Files under the historical design-record tree are excluded.

### 9. Marketplace manifest consistency

Cross-checks `plugins/` against `.cursor-plugin/marketplace.json`:

- Every plugin with a `.cursor-plugin/plugin.json` file must be listed in the manifest
- Every plugin listed in the manifest must have a `skills/` directory

### 10. Instruction file preambles

Files matching `adapters/targets/**/instructions/<name>.md` must contain an output location preamble:

```markdown
> **Output location**: `.specify/slices/...`
```

This prevents cross-plugin path contamination by making every instruction file declare where its output goes.

### 11. Acceptance scenario frontmatter

Acceptance scenario files are validated against [`schemas/scenario.schema.json`](https://github.com/augentic/specify-cli/blob/main/schemas/scenario.schema.json) in the `specify-cli` repo (JSON Schema 2020-12, validated through the same Ajv2020 path as the SKILL.md schema). Discovery follows these opt-in roots:

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
- **Stages prefix** — `stages` must be a contiguous slice of `[plan, refine, build, merge, drop]` anchored at any element. `[plan, refine, build]` is valid; `[build, plan]`, `[plan, merge]`, `[merge]` are not.
- **Body-id consistency** — when the visible `Scenario ID:` body line is present (C02 doubles the id in prose for resilience against environments that suppress frontmatter), it must equal the frontmatter `id`.
- **Expected-artifact path safety** — every entry in `expected-artifacts` must be a relative path with no `..` segments and no leading `/`. The check stops short of pinning a per-adapter prefix (e.g. `contracts/`) so future adapters are not over-constrained.
- **Cross-file id uniqueness** — every opted-in scenario `id` is unique across the repo; duplicates are reported with both file paths.

Internal markdown link resolution within scenarios is handled by check 1 (markdown link resolution); the scenario validator does not duplicate it.

**Example failure messages:**

```text
FAIL: scenarios.schema-violation: Scenario frontmatter: adapters/targets/contracts/tests/_probe.md — / must have required property 'negative-expectations'
  at adapters/targets/contracts/tests/_probe.md:1
FAIL: scenarios.stages-not-contiguous: Scenario frontmatter: adapters/targets/contracts/tests/_probe.md — stages must be a contiguous slice of [plan, refine, build, merge, drop] anchored at any element; got ["build","define"]
  at adapters/targets/contracts/tests/_probe.md:1
FAIL: scenarios.body-id-mismatch: Scenario frontmatter: adapters/targets/contracts/tests/_probe.md — body 'Scenario ID: `contracts-foo`' does not match frontmatter id 'contracts-bar'; align the visible line with the frontmatter id
  at adapters/targets/contracts/tests/_probe.md:1
FAIL: scenarios.artifact-path-unsafe: Scenario frontmatter: adapters/targets/contracts/tests/_probe.md — expected-artifact '../escape.yaml' must not escape the scenario workspace ('..' segment not allowed)
  at adapters/targets/contracts/tests/_probe.md:1
FAIL: scenarios.duplicate-id: Scenario frontmatter: duplicate scenario id 'contracts-describe' across files: adapters/targets/contracts/tests/_probe.md, adapters/targets/contracts/tests/describe.md
```

Common fixes: align `kind`/`adapter` per the schema, walk back `stages` to a contiguous slice anchored in `[plan, refine, build, merge, drop]`, keep the body `Scenario ID:` line in lockstep with the frontmatter `id`, rewrite expected-artifact paths to be relative to the scenario workspace root, and ensure new scenario ids are unique.

### 12. Recorded trace freshness

The recorded-trace check is opt-in. If a future suite adds
`tests/recorded/**/*.jsonl`, every trace must lead with a
`recorded-trace-header` line carrying `schemaVersion: 1`, `sourceBackend`,
`sourceRunId`, `sourceTimestamp`, and `scenarioId`.

### 13. First-party rule shape

First-party rule files are validated in the shared tree at `adapters/shared/rules/universal/**/*.md` (UNI-* rules) and in per-adapter overlays at `adapters/sources/*/rules/**/*.md` and `adapters/targets/<name>/rules/**/*.md`.

The check is format-only. It does not run consumer-project review and does not
invoke any external validator. It validates:

- **Frontmatter schema** -- each file must begin with YAML frontmatter that
  conforms to [`schemas/rule.schema.json`](https://github.com/augentic/specify-cli/blob/main/schemas/rules/rule.schema.json) in the `specify-cli` repo.
- **Required body heading** -- each rule body must include a `## Rule` heading.
- **Cross-file id uniqueness** -- every codex `id` must be unique across the
  discovered first-party rule set.
- **Namespace ownership** -- `adapters/shared/rules/universal/` owns `UNI-*`;
  `omnia` owns `OMNIA-*`, `RUST-*`, and `SEC-*`; `contracts` owns `IFACE-*`;
  `vectis` owns `VECTIS-*`.

**Example failure messages:**

```text
FAIL: rules.schema-violation: Rule frontmatter: adapters/shared/rules/universal/example.md — / missing required property 'trigger'
  at adapters/shared/rules/universal/example.md:1
FAIL: rules.schema-violation: Rule frontmatter: adapters/shared/rules/universal/example.md — /severity must be one of "critical", "important", "suggestion", "optional"
  at adapters/shared/rules/universal/example.md:1
FAIL: rules.schema-violation: Rule body: adapters/shared/rules/universal/example.md — missing required '## Rule' heading
  at adapters/shared/rules/universal/example.md:1
FAIL: codex.namespace-ownership-violation: Codex namespace ownership: adapters/shared/rules/universal/example.md — codex owner 'universal' may only use UNI-* ids, got 'SEC-001'
  at adapters/shared/rules/universal/example.md:1
FAIL: codex.duplicate-rule-id: Rule duplicate id 'UNI-001' across files: adapters/shared/rules/universal/a.md, adapters/shared/rules/universal/b.md
```

Common fixes: add the required `id`, `title`, `severity`, and `trigger`
frontmatter fields; use canonical severity values (`critical`, `important`,
`suggestion`, `optional`) and review modes (`deterministic`,
`model-assisted`, `hybrid`); keep ids in the reserved namespace-plus-three-digit
shape such as `UNI-001`; add the `## Rule` heading; and coordinate with content
subagents before reusing or moving ids between adapter-owned namespaces.

### 14. Tool-owned schema link resolution

Every `schemas.specify.dev/<tool>/<name>.schema.json` URL in any `.md` file under `adapters/` must resolve to a known tool-owned schema. The check maintains a hardcoded registry of tool → schema-name mappings (currently `vectis` → `tokens`, `assets`, `composition`; the `contract` tool declares no embedded schemas). URLs inside fenced code blocks and inline code spans are skipped.

This enforces the tool-owned schema contract: plugin briefs cite schemas by canonical `$id` URL, and the check ensures every cited URL matches a real schema in the tool's embedded registry. The rule id is `links.brief-schema-link-resolve`.

**Common fix:** verify the tool name and schema name in the URL. Use `specrun tool schema <tool> <name>` to confirm the schema exists. If the schema was renamed or retired, update the URL or remove the reference.

## Extending the checks

Two surfaces are available for new framework checks: a declarative `CORE-*` rule under [`adapters/shared/rules/core/`](../../adapters/shared/rules/core/), or an imperative `Check` impl in the `specify-authoring` crate. **Default to a `CORE-*` rule.** Imperative `Check` impls remain a legitimate escape hatch, but new declarative rules are cheaper to author, ship with their `## Rule` body as the canonical agent-readable explanation, and run through the same deterministic-hint interpreter that consumer projects can adopt via `specrun lint`.

### Choose `CORE-*` (declarative) when

- The predicate can be expressed as one or more `deterministic_hints` of kind `path-pattern`, `regex`, `schema`, or `tool` (the kinds shipped today; reserved kinds land paired with their interpreter implementation).
- The check fits one of the closed `applicability.artifacts` framework tokens (`skill`, `adapter`, `brief`, `reference`, `codex`, `doc`).
- A subprocess is unnecessary, or the subprocess is already wired as a declared WASI tool reachable through a `tool` hint.

The chassis worked example is [`CORE-001-adapter-schema.md`](../../adapters/shared/rules/core/CORE-001-adapter-schema.md), which retired the previous imperative `adapter` schema-row predicate via the parity test at [`crates/authoring/tests/core_parity_adapter_schema.rs`](https://github.com/augentic/specify-cli/blob/main/crates/authoring/tests/core_parity_adapter_schema.rs). See [`adapters/shared/rules/core/README.md`](../../adapters/shared/rules/core/README.md) for the rule file shape, the applicability-token table, hint-kind preference, authoring conventions, and the pointer into the predicate migration map.

To add a `CORE-*` rule:

1. Pick the next free `CORE-NNN` id and add the rule file under [`adapters/shared/rules/core/`](../../adapters/shared/rules/core/) per the README's frontmatter shape.
2. Run `make lint`; `specdev lint` resolves the new file and runs its hints against the framework tree by default. The `--include-core` flag is consumer-side only (`specrun lint` / `specrun rules export`); `specdev` always sees `CORE-*` rules.
3. If retiring an imperative `Check` row alongside the rule, land the parity test at `crates/authoring/tests/core_parity_<rule>.rs` in `augentic/specify-cli` and delete the predicate row in the same PR; the fingerprint algorithm collapses duplicate findings during overlap.

### Choose an imperative `Check` when

- The predicate genuinely needs subprocess orchestration or stateful behaviour the hint interpreter cannot model (e.g. spawning a long-running validator with multi-turn interaction).
- The predicate is exploratory and you are not ready to commit to a stable `CORE-NNN` id.
- You are extending an existing imperative module with a small adjacent row that does not yet warrant a dedicated `CORE-*` rule.

To add an imperative check:

1. Add a module under [`crates/authoring/src/check/`](https://github.com/augentic/specify-cli/tree/main/crates/authoring/src/check/) implementing the `Check` trait (or a `run_*` helper returning `Vec<Finding>`).
2. Register the check in the `checks` array in [`crates/authoring/src/check.rs`](https://github.com/augentic/specify-cli/blob/main/crates/authoring/src/check.rs).
3. Add a fixture-based integration test under [`crates/authoring/tests/`](https://github.com/augentic/specify-cli/tree/main/crates/authoring/tests/) when the predicate needs regression coverage.
4. Run `make lint` to verify the new check works.

Checks are numbered 1–14 contiguously in this document. New imperative checks should use the next available number (currently 15); declarative `CORE-*` rules are listed by id in [`adapters/shared/rules/core/`](../../adapters/shared/rules/core/) and do not consume a number in this list.

## CLI checks

The specify-cli repo has its own check suite via `cargo-make`:

```bash
cargo make ci     # lint, test, test-docs, vet, outdated, deny, fmt
cargo make check  # audit, fmt, lint, outdated, deps
```

These are Rust-specific checks (clippy, formatting, dependency auditing, test suite) and are separate from the documentation checks in the specify repo.
