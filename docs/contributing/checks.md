# Consistency Checks

This repo is linted by `specify lint framework`. The runtime lives in-tree under [`engine/`](../../engine), so `make lint` builds the in-tree binary and runs it against the prose: `cd engine && cargo run -q -p specify -- lint framework --framework-root ..`. A Rust toolchain is the only prerequisite — there is no source pin, published-binary download, or sibling checkout to resolve. Run checks before every pull request.

## Editor-first vs specify lint framework

Framework validation splits into two surfaces:

| Surface                                              | When it runs                                                                                       | What it covers                                                                                                                                                                    |
| ---------------------------------------------------- | -------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Editor-first (YAML/JSON LSP)**                     | While you edit plain YAML or JSON                                                                  | Shape violations for files the language server can bind to a schema: `adapter.yaml`, `.cursor-plugin/marketplace.json`, and other plain YAML/JSON artifacts that declare a schema |
| **`specify lint framework` (Markdown + cross-file)** | Local `make lint`, CI, and direct `cargo run -p specify -- lint framework` (under `engine/`) | Markdown frontmatter (`SKILL.md`, rules, scenario docs), symlink integrity, marketplace ↔ plugin consistency, link resolution, and every other predicate schemas cannot express   |

**Authoritative schemas** live in-tree under [`schemas/`](../../schemas) and are embedded in the `specify` binary; `specify lint framework` validates against those embedded copies. Editors resolve the same contract by binding to the published schemas via the remote `raw.githubusercontent.com` URLs in [`.vscode/settings.json`](../../.vscode/settings.json) — there is no vendored mirror to keep in sync.

**Plain YAML/JSON wiring.** Adapter manifests carry a first-line schema directive (and [`.vscode/settings.json`](../../.vscode/settings.json) binds `adapters/sources/*/adapter.yaml` and `adapters/targets/*/adapter.yaml` to the runtime schemas for editor squiggles):

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/augentic/specify/main/schemas/source.schema.json
```

Use the same pattern for other plain YAML files when a framework or runtime schema exists. Workflow and consumer schemas (`adapter`, `plan`, `evidence`, …) and framework authoring schemas (`authoring/skill`, `authoring/scenario`, `authoring/marketplace`, `rules/rule`) all ship in-tree under `schemas/`. JSON manifests can use a top-level `"$schema"` property — see [`.cursor-plugin/marketplace.json`](../../.cursor-plugin/marketplace.json).

**Markdown frontmatter.** Cursor's YAML language server validates standalone `.yaml` control files reliably, but does not yet surface the same diagnostics for YAML embedded in Markdown frontmatter. Until a frontmatter-aware editor integration lands, `specify lint framework` extracts the leading `---` block from `SKILL.md`, rules, and scenario Markdown files and validates it against the same JSON Schemas under `schemas/authoring/` and `schemas/rules/`.

## Enforcement surfaces (authoring vs engineering standards)

Framework and consumer validation are intentionally separate. See [Standards layer](../explanation/standards-layer.md).

| Surface                   | Command                                | Audience                           | Enforces                                                        |
| ------------------------- | -------------------------------------- | ---------------------------------- | --------------------------------------------------------------- |
| **Authoring standards**   | `specify lint framework` (`make lint`) | `augentic/specify` contributors    | Skill frontmatter, rule *shape*, links, marketplace consistency |
| **Engineering standards** | `specify lint project`                 | Consumer projects with `.specify/` | Applicable rules with `rule_hints`; structured findings for CI  |
| **Build-time judgment**   | Target `build/review.md` briefs        | Active slice during `/spec:build`  | Model-assisted codex policy → `REVIEW.md`                       |

Rule *content* lives under `adapters/**/rules/` (engineering standards). `docs/standards/` is **authoring** house style only.

## Running checks

```bash
make lint
```

Exit code `0` means all checks pass. Validation failures exit `2`; infrastructure errors exit `1`.

### The in-tree binary

`make lint` builds the `specify` binary from the in-tree Cargo workspace under [`engine/`](../../engine) and runs `lint framework --framework-root ..` against this repo:

```bash
cd engine && cargo run -q -p specify -- lint framework --framework-root ..
```

There is no source pin, no published-binary download, and no sibling checkout: the runtime and the prose are one repo, versioned and released together. A Rust toolchain is the only prerequisite. `make ci` runs the full Rust workspace gate (`cargo make ci` under `engine/`) followed by `make lint`.

**Performance.** Framework lint is a single generic pass over all resolved `CORE-*` / `UNI-*` rules: each rule resolves either as a declarative hint (Road A) or a name-resolved in-process checker (Road B). No imperative `Check` rule producer runs on `make lint`. On a **release** build this tree lints in single-digit seconds; benchmark on your own hardware with `/usr/bin/time make lint`. Always measure against `cargo build --release`: a debug/unoptimized binary is many times slower and is not representative.

The `specify-standards` framework predicate regression suite lives in-tree under `engine/` and runs with the rest of the Rust workspace via `cargo make test`:

```bash
cd engine && cargo test -p specify-standards
```

### CI

CI is one job. [`.github/workflows/ci.yaml`](../../.github/workflows/ci.yaml) builds the in-tree binary on a stable toolchain (with `Swatinem/rust-cache`), runs `cargo make ci` under `engine/` (the full Rust workspace gate), then runs `specify lint framework --framework-root .` over the prose plus a spec-runtime symlink check. The prose↔runtime contract is intra-repo: one PR, one CI run.

When invoking `specify lint framework` directly (not via `make lint`), run it from the repo root or pass `--framework-root` / set `SPECIFY_ROOT` to the plugin-repo root. Authoritative schemas are embedded in the `specify` binary.

### Diagnostic format

Each finding prints on stderr as:

```text
FAIL: <rule-id>: <message>
  at <repo-relative-path>:<line>
```

`<rule-id>` is stable kebab-case (for example `links.unresolved`, `scenarios.schema-violation`, `rules.namespace-ownership-violation`). Human text output may still say `codex.*` in older examples; wire ids use the `rules.*` family for codex shape checks. The location line is omitted when a finding is repo-wide (duplicate ids, missing checkout). A summary line reports the total failure count; success prints `All checks passed.` on stdout.

| Road                          | `CORE-*`                                                                                                                         | How enforced                                                                                                                                                                                                                                                                                   |
| ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Road A — declarative hint     | most of `CORE-001..060`                                                                                                          | `rule_hints` on the rule file (`kind:` ∈ `schema`, `reference-resolves`, `cardinality`, `set-coverage`, `constant-eq`, `unique`, `fenced-block`, `regex`, `path-pattern`, `presence`, `field-grammar`, `cross-reference`, `cli-contract`), interpreted over the workspace model |
| Road B — referenced tool      | `CORE-009`, `CORE-026`, `CORE-053`, and the scenarios / skill-body / links-registry / marketplace / prose families               | `kind: tool` + a sentinel `path-pattern`; the engine resolves the named in-process checker and folds its findings                                                                                                                                                                              |

All policy (caps, allow-lists, owner maps, expected sets) rides the rule's `config:`; the engine never embeds it.

| Authoring `rule_id` prefix | Topic                                                                |
| -------------------------- | -------------------------------------------------------------------- |
| `adapter.*`                | Adapter manifests                                                    |
| `links.*`                  | Markdown links, skill references, directives, tool-owned schema URLs |
| `skill.*`                  | `SKILL.md` frontmatter and body                                      |
| `scenarios.*`              | Eval scenario frontmatter and recorded traces                  |
| `rules.*`                  | Rule shape, namespace ownership                                      |

Rule files live under [`adapters/shared/rules/core/`](../../adapters/shared/rules/core/). The generic hint evaluators live in-tree under `engine/crates/standards/src/lint/eval/`; Road B checker source lives in-process in `specify-standards` under `engine/crates/standards/src/lint/framework_tools/`.

### JSON output

`specify lint framework` can emit the same structured result shape consumed by CI integrations. Run `specify lint framework --format json` (or set `SPECIFY_FORMAT=json`) to swap the human-oriented stderr stream for a single structured envelope written to stdout. Default `text` output remains canonical for humans; reach for `--format json` when wiring CI annotations, preparing dashboards, or comparing authoring findings with consumer-project `specify lint project` output.

```bash
specify lint framework --format json | jq '.findings[] | select(.severity == "critical")'
```

Envelope shape:

```json
{
  "version": 1,
  "summary": { "critical": 0, "important": 0, "suggestion": 0, "optional": 0 },
  "findings": []
}
```

The full wire contract, including per-finding fields and the canonical fingerprint algorithm, is pinned by the CLI schemas: `schemas/diagnostics/diagnostic-report.schema.json` for the envelope, `schemas/diagnostics/diagnostic.schema.json` for each `LintFinding`, and `schemas/rules/rule.schema.json` for rule authoring shape.

Exit codes follow the existing semantics — `0` on a clean tree, `2` when findings are present (validation failed), `1` on infrastructure errors. On a `1`, the JSON envelope on stdout collapses to `{"version": 1, "summary": {…all zero}, "findings": []}` and the underlying error surfaces on stderr.

**Severity.** Each rule declares its own `severity:` in frontmatter; `rules.schema-violation` (CORE-027) is `critical` because a malformed rule breaks every downstream consumer of the resolved rules export, while most other families default to `important`. Road B tools stamp the same severity onto their emitted findings.

**`rule-id` carries the closed `CORE-NNN` id.** Both roads set `rule_id` from the rule file's `id:` frontmatter — Road A hints inherit it directly, and Road B tools stamp each finding with the owning `CORE-NNN`. `CORE_ID_TABLE` is empty: there is no imperative namespace bridge.

**Consumer-project counterpart.** `specify lint framework --format json` is the **framework-repo** authoring surface; `specify lint project` is its **consumer-project** counterpart, scanning `.specify/`-bearing trees with deterministic codex hints. Both emit the same `LintFinding` envelope so CI tooling, dashboards, and PR bots that consume one can consume the other unchanged. See [Standards layer](../explanation/standards-layer.md) for the consumer-side scanner contract.

## What the checks enforce

### 1. Markdown link resolution

Every relative link in every `.md` file must resolve to an existing file. External links (`http://`, `mailto:`, `#` anchors) and `src/` paths are skipped. Fenced code blocks and HTML comments are stripped before scanning.

**Common fix:** update the link target or remove a stale link.

### 2. Adapter manifest YAML validation

Every `adapters/sources/<name>/adapter.yaml` validates against `source.schema.json`, and every `adapters/targets/<name>/adapter.yaml` validates against `target.schema.json`. Both schemas ship in-tree under `schemas/` and are loaded by the `specify-standards` crate.

**Common fix:** check that all required fields (`name`, `version`, `axis`, `operations`, `briefs`) are present and that `operations` matches the per-axis enum (`survey` + `extract` for sources; `shape` + `build` + `merge` for targets).

### 3. Adapter referential integrity

Manifests do not carry a `pipeline:` field. Brief existence and operation coverage are enforced by the per-axis schemas (`source.schema.json` / `target.schema.json`).

### 4. Symlink integrity

Every symlink under `plugins/` must resolve to a valid target.

The per-target-adapter `agent-teams.md` overlays are part of the same surface: each `adapters/targets/<name>/references/agent-teams.md` MUST be a real symlink resolving (through the shared runtime overlay) to `docs/reference/review-team-protocol.md`. Regular-file overlays are forbidden — a symlink chain cannot drift in content, so CORE-011 (canonical-document presence) plus the CI symlink check is the whole enforcement surface. The retired CORE-008 digest pin and CORE-012 `agent-teams` tool policed a regular-file overlay form that is no longer admitted.

**Common fix:** recreate the symlink if the target was moved or renamed; if an overlay was committed as a regular file, replace it with a symlink to the shared runtime overlay.

### 5. SKILL.md frontmatter validation

Every `SKILL.md` under `plugins/` is validated against the `specify-standards` framework skill schema (the embedded `schemas/authoring/skill.schema.json`):

- **Required fields** -- `name` (kebab-case) and `description` (minimum 10 characters)
- **Plugin-qualified name** -- `name` is **plugin-qualified** (`<plugin>-<skill>`, e.g. `specify-merge`, `omnia-crate-writer`), not the bare directory name; the per-plugin prefix invariant and global uniqueness across plugins are enforced by `specify lint framework` (CORE-043), since JSON Schema cannot see the surrounding directory
- **Known tools** -- every entry in `allowed-tools` must be a recognized Cursor tool name or match the `mcp__*` prefix

The recognized tool set includes: `Read`, `Write`, `StrReplace`, `Shell`, `Grep`, `Glob`, `ReadLints`, `WebFetch`, `WebSearch`, `AskQuestion`, `Task`, `TodoWrite`, `SemanticSearch`, `EditNotebook`, `GenerateImage`.

Long `SKILL.md` bodies are also checked for structure: bodies over 200 post-frontmatter lines fail (strict — no grandfathering), and bodies with at least 150 post-frontmatter lines must include a `## Critical Path` section with 5-7 bullets, numbered items, or `### N. Title` H3 step headings.

### 6. Skill reference link resolution

Links in `SKILL.md` bodies that point to `references/...` or `examples/...` paths are resolved relative to the skill directory. Every such link must resolve to an existing file.

### 6b. Deployable surfaces must not link into `docs/`

`links.docs-in-deployable-surface` (`CORE-052`) flags markdown links under `plugins/` and under `adapters/**/briefs/` + `adapters/**/references/` whose targets escape into `docs/`. Contributor codex under `adapters/shared/rules/` is excluded. Runtime canonical paths are `plugins/spec/references/` and, for adapters after `specify init`, `references/spec-runtime/` inside the cached adapter tree.

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

### 11. Eval scenario frontmatter

Eval scenario files are validated against [`schemas/authoring/scenario.schema.json`](../../schemas/authoring/scenario.schema.json) (JSON Schema 2020-12, validated through the same Ajv2020 path as the SKILL.md schema). Discovery follows these opt-in roots:

1. `evals/scenarios/<id>.md` — the flat platform scenario pack (one self-contained scenario per `.md`; the `README.md` catalog is skipped).
2. `adapters/targets/<target>/tests/<scenario>.md` — flat owner-local target scenarios.
3. `adapters/targets/<target>/tests/<scenario>/scenario.md` — directory-form owner-local target scenarios.
4. `plugins/<plugin>/skills/<skill>/fixtures/<scenario>/scenario.md` — promoted skill-owned fixtures.

Discovery is **opt-in by frontmatter**: a markdown file under one of those roots is validated only if it begins with a YAML frontmatter block (`---`). Prose-only docs in those trees — [`evals/README.md`](../../evals/README.md), `evals/shared/*`, `evals/runs/`, catalog READMEs, narrative — are skipped silently. The shared suite is the platform scenario pack under [`evals/scenarios/`](../../evals/scenarios/README.md). The first owner-local target pack is the contracts test suite under [`adapters/targets/contracts/tests/`](https://github.com/augentic/specify-adapters/blob/main/targets/contracts/tests/README.md).

An opt-in scenario looks like:

```markdown
---
id: contracts-describe
owner: contracts
kind: adapter
adapter: contracts@1.0.0
entrypoint: /spec:refine
stages: [refine, build, merge]
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

- **Schema conformance** — `id`, `owner`, `kind`, `entrypoint`, `stages`, `isolation` are required; `adapter` is required when `kind` is `adapter` or `adapter-boundary`; `negative-expectations` is required (with at least one entry) when `kind` is `adapter-boundary`. `kind` is an open enum (`adapter`, `adapter-boundary`, `suite`, `skill`); only the first two are actively required by C02. `isolation` ∈ {`fresh-project`, `shared-baseline`, `shared-slice`}. `adapter` matches `^[a-z][a-z0-9-]*@v\d+$`. `entrypoint` matches `^/[a-z]+:[a-z][a-z0-9-]*$`. `id` matches `^[a-z][a-z0-9-]*$`.
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

Common fixes: align `kind`/`adapter` per the schema, walk back `stages` to a contiguous slice anchored in `[plan, refine, build, merge, drop]`, keep the body `Scenario ID:` line in lockstep with the frontmatter `id`, rewrite expected-artifact paths to be relative to the scenario workspace, and ensure new scenario ids are unique.

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
  conforms to [`schemas/rules/rule.schema.json`](../../schemas/rules/rule.schema.json).
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
FAIL: rules.namespace-ownership-violation: Rule namespace ownership: adapters/shared/rules/universal/example.md — rule owner 'universal' may only use UNI-* ids, got 'SEC-001'
  at adapters/shared/rules/universal/example.md:1
FAIL: rules.duplicate-rule-id: Rule duplicate id 'UNI-001' across files: adapters/shared/rules/universal/a.md, adapters/shared/rules/universal/b.md
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

**Common fix:** verify the tool name and schema name in the URL. Use `specify extension schema <tool> <name>` to confirm the schema exists. If the schema was renamed or retired, update the URL or remove the reference.

### 15. CLI contract drift

[`CORE-057`](../../adapters/shared/rules/core/CORE-057-cli-contract-drift.md) checks every CLI citation in this repo's documentation against the contract of the **pinned binary that is running the lint** — the same payload `specify contract dump` emits. `specify …` command lines in `bash`/`sh` fences and inline code walk the verb tree (unknown verbs, undeclared `--flags`); cited journal event ids and fenced-JSON `"event"` / `"error"` values are membership-checked against the declared taxonomies. Because the contract is rebuilt from the binary on each run, every citation is re-checked against the in-tree binary in the same change.

[`CORE-060`](../../adapters/shared/rules/core/CORE-060-cli-test-citation-drift.md) rides the same kind's `test-citations` selector: "proven by a named test" claims — `tests/….rs` inline spans and `engine/tests/` link targets under `docs/**` and `AGENTS.md` — must exist in the binary's build-time test inventory. Adapter trees are out of scope because they legitimately describe generated downstream-crate `tests/` layouts.

**Common fix:** align the citation with the live CLI surface (`specify contract dump --format json` or `specify <verb> --help`). For intentional non-verbs — negative claims like "there is no `workspace merge` subcommand" or retired-verb history — drop the `specify ` prefix inside the code span so the citation stops being an invocation. Documented-ahead surfaces (verbs designed but not yet shipped) ride the rule's `config: ignore` with a comment, and the entry is removed when the verb lands.

## Extending the checks

Every framework check is a `CORE-*` rule under [`adapters/shared/rules/core/`](../../adapters/shared/rules/core/), resolved by a **generic, rule-agnostic dispatcher** in the in-tree `specify-standards` crate (`engine/crates/standards/`). The engine carries no rule-specific logic and no rule policy. A new check takes one of two roads, and the rule file owns both the check shape and the values it enforces.

### Road A — declarative hint

The rule carries one or more `rule_hints` of a closed kind interpreted over the workspace model. Reach for Road A for one-liner checks (schema conformance, link/symlink resolution, line caps, uniqueness, fenced-block scans, regex/path scoping, required-artifact presence, frontmatter-field grammar, and relational cross-reference joins). The kinds:

`schema`, `reference-resolves`, `cardinality`, `set-coverage`, `constant-eq`, `unique`, `fenced-block`, `regex`, `path-pattern`, `presence`, `field-grammar`, `cross-reference`, `cli-contract`.

`path-pattern` globs support `{a,b}` brace alternation (`*` never crosses `/`, `**` does), so prefer one alternation hint over a fan-out of near-identical patterns; `regex` values likewise prefer a single `|` alternation per concern.

`hint.value` names the mechanism selector each kind dispatches on:

- **`presence`** — `frontmatter` (a candidate file lacking frontmatter), `file` + `config: { path }` (a missing required path), `markdown-section` + `config: { title, level, when: { metric, min } }` (a candidate over a metric threshold lacking the section), or `directory-index` + `config: { roots, index, min-files }` (a corpus directory matching a one-depth root glob with enough files beneath it but no index file inside it).
- **`field-grammar`** — `field-tokens` + `config: { field, token-pattern }` (each whitespace token of the field matches the regex) or `field-first-word` + `config: { field, allowed }` (the field's first alphabetic word is allow-listed).
- **`cross-reference`** — a relational join from an `adapter-dir` (fact-family set difference) or `expected-set` + `config: { entries: [{ key, value }] }` (value-equality) source against a `config: { target }` family (`adapter-manifest`).
- **`set-coverage`** — the `adapter-briefs` source reads `config: { mode }` (`subset` default — CORE-004, one-sided missing-only; or `exact` — CORE-007, two-sided, additionally flagging unexpected keys) alongside `config: { expected-operations }`.
- **`schema`** and **`unique`** also accept a whole-tree `value: scenario` selector (the latter with `config: { field: id }`) that reads the scoped scenario fact family directly.
- **`cli-contract`** — `invocations` + `config: { langs }` (every `specify …` command line in matching fences and inline code walks the verb tree; unknown verbs and undeclared `--flags` flag), `event-ids` / `error-codes` + `config: { json-fields }` (cited journal event ids and error discriminants are membership-checked; event-id candidates are gated to the contract's own id families), or `test-citations` + `config: { link-prefixes }` (cited `tests/….rs` spans and `engine/tests/` link targets are membership-checked against the binary's build-time test inventory). The contract itself — verb tree, flags, event ids, error discriminants, tests — is injected by the running binary (the `specify contract dump` payload), so the rule carries exemptions in `config:` but never a verb list.

Each evaluator is generic: it reads its policy (cap, allowed set, owner map, expected operations, canonical path, required section, grammar pattern, expected entries) from the rule's `config:`, never from a constant in the engine. The new kinds serve `presence` → CORE-042 / CORE-011 / CORE-041 / CORE-059, `field-grammar` → CORE-035 / CORE-036, `cross-reference` → CORE-010, the `schema` scenario selector → CORE-032, the `unique` scenario selector → CORE-030, and `cli-contract` → CORE-057 / CORE-060. CORE-018 / CORE-020 (link-registry joins) and CORE-022 (marketplace) stay on Road B by design. The chassis worked example is [`CORE-001-adapter-schema.md`](../../adapters/shared/rules/core/CORE-001-adapter-schema.md). See [`adapters/shared/rules/core/README.md`](../../adapters/shared/rules/core/README.md) for the rule-file shape, hint-kind preference, and `config:` conventions.

**Engine cost.** Reusing an existing kind with a new `config:` shape touches `crates/standards/src/lint/eval/<kind>.rs` and the `schemas/rules/rule.schema.json` `$def` (which trips the `crates/schema/tests/schemas.rs` byte-match gate). A brand-new fact may also need an indexer extractor + `workspace-model.schema.json` update. New engine behaviour gets a **mechanism-named, rule-agnostic** unit test beside the evaluator in `crates/standards/src/lint/eval/<kind>.rs` (keyed to a placeholder `UNI-9xx` fixture — never a real `CORE-NNN`).

### Road B — referenced tool

The rule carries `kind: tool`, `value: <tool>`, plus a sentinel `path-pattern`. The engine resolves the named checker from the in-process framework inventory (`engine/crates/standards/src/lint/framework_tools.rs` in `specify-standards`), runs it once per lint, and folds its typed findings directly; the checker stamps each finding with its own `rule_id` / `severity`. Reach for Road B for branchy, whole-tree, cross-fact, registry-backed, or extractor-heavy checks (and for files the indexer does not walk, e.g. `evals/`).

The seven framework checkers are native modules under `engine/crates/standards/src/lint/framework_tools/` in `specify-standards`. Each one and the `CORE-*` rules it serves:

| Checker          | Serves                  |
| ---------------- | ----------------------- |
| `scenarios`      | CORE-028, 029, 033, 056 |
| `skill-body`     | CORE-040, 046, 048      |
| `links-registry` | CORE-018, 020           |
| `marketplace`    | CORE-022                |
| `prose`          | CORE-024                |
| `rules`          | CORE-009, 026, 053      |
| `extension`      | CORE-061                |

To add or extend one:

1. Add the pure check fn to the family checker module under `crates/standards/src/lint/framework_tools/<name>.rs`, stamping findings with the owning `CORE-NNN` / `severity`. Read any policy from the rule's `config:` (forwarded by the engine as a second positional argument) — never bake it into the checker.
2. Cover the new check with module-local unit tests beside the checker.
3. Author/point the `CORE-*` rule file at the checker, run `make lint` + `cargo make check` (under `engine/`).

> **Policy never lives in the engine.** The `lint_no_embedded_policy` Layer-3 guard test ([`engine/crates/standards/tests/lint_engine_guards.rs`](../../engine/crates/standards/tests/lint_engine_guards.rs)) fails if any eval arm reintroduces a rule-specific literal (operation-set array, owner→prefix map, value-bearing discriminator, canonical-doc path, or an un-allow-listed numeric cap). Put the value in the rule's `config:`.

> **No imperative escape hatch.** A `CORE-*` rule resolves only as a declarative hint (Road A) or a name-resolved in-process checker (Road B). Coverage rests on the per-kind evaluator suite, the schema byte-match gate, and each checker's in-crate tests.

To add a `CORE-*` rule (either road):

1. Pick the next free `CORE-NNN` id and add the rule file under [`adapters/shared/rules/core/`](../../adapters/shared/rules/core/) per the README's frontmatter shape, carrying every policy value in `config:`.
2. Run `make lint`; `specify lint framework` resolves the new file and runs it against the framework tree by default. The `--include-core` flag is consumer-side only (`specify lint project` / `specify rules export`); `specify lint framework` always sees `CORE-*` rules.

Checks are numbered 1–15 contiguously in this document for the narrative descriptions above; declarative `CORE-*` rules are listed by id in [`adapters/shared/rules/core/`](../../adapters/shared/rules/core/) and do not consume a number in this list.

## CLI checks

The Rust workspace under `engine/` has its own check suite via `cargo-make` (run from `engine/`):

```bash
cargo make ci     # lint, test, test-docs, vet, outdated, deny, fmt
cargo make check  # audit, fmt, lint, outdated, deps
```

These are Rust-specific checks (clippy, formatting, dependency auditing, test suite) and are separate from the framework documentation checks (`make lint`).
