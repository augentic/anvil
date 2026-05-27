# RFC-28: Engineering Standards — Codex Contract and Findings

> Status: Implemented - Depends: [RFC-5](done/rfc-5-tooling.md) for codex authoring validation, [RFC-25](done/rfc-25-workflow.md), [RFC-27](done/rfc-27-synthesis.md) - Enables: [RFC-32](rfc-32-standards-enforcement.md), [RFC-34](rfc-34-framework-convergence.md), [roadmap RM-10](roadmap.md#rm-10-ci-native-standards-enforcement), [RFC-18](future/rfc-18-slm.md)
>
> **Post-implementation note (RFC-34):** the declarative `FRAME-*` rules, framework scan profile, and `specdev review` verb originally attributed to "RFC-32 Phase 3 Option B" throughout this RFC's body were carved out as [RFC-34](rfc-34-framework-convergence.md) after RFC-28 was implemented. RFC-34 also normatively widens the closed `Origin` enum below by one value (`framework`) and amends the origin sort order (see [RFC-34 §A1–A2](rfc-34-framework-convergence.md#rfc-28-amendments)). Historical attributions to "RFC-32 Phase 3 Option B" in the body still resolve correctly via RFC-32's forward pointer to RFC-34; this note exists so a reader does not have to chase the chain.

## Abstract

Define the **engineering standards** contract for Specify: how durable policy is stored, resolved, cited, and reported — without mutating workflow state.

Specify separates **workflow** (phase skills and lifecycle CLI), **artifacts** (slice and baseline intent), and **engineering standards** (codex policy). Codex is the on-disk rule format under `adapters/**/codex/`; this RFC defines the resolution and finding wire shape, not the workflow loop. See [docs/explanation/standards-layer.md](../docs/explanation/standards-layer.md).

Specify already has first-party codex rule files under shared and per-target directories, plus target adapter review briefs that cite those rules in `REVIEW.md`. The missing piece is a structured bridge from "agent-readable policy files on disk" to "resolved rule context and review findings a CLI, CI job, PR comment, scorer, dashboard, or reviewing agent can consume." This RFC adds that bridge without implementing the full `specrun review` standards scanner.

This RFC adds:

1. **Resolved codex export** - a CLI- and agent-readable view that resolves shared rules plus source adapter and target adapter overlays into one ordered rule set for a project, target adapter, slice, or artifact path, including the policy text needed to apply each rule.
2. **Structured review finding schema** - a stable JSON shape for deterministic and model-assisted findings.
3. **Codex resolution rules** - namespace ownership, overlay precedence, applicability filters, deprecation handling, and stable ordering.
4. **Reviewer report alignment** - target adapter review briefs continue to write human `REVIEW.md`, but every machine-readable finding maps to the same schema.
5. **Framework finding export (Phase 3)** - `specdev check --format json` maps imperative authoring findings to the same `ReviewFinding` envelope (RFC-32 Phase 3 Option A); imperative predicates remain the framework gate.

The scanner that produces deterministic findings (`specrun review` — CI-native **standards enforcement**, not a workflow phase) is a follow-up surface defined by [RFC-32](rfc-32-standards-enforcement.md). This RFC defines the rule and finding contract that scanner consumes, but codex rules remain agent-readable Markdown policy first; deterministic hints are optional metadata for mechanically observable subsets.

## Motivation

The roadmap calls for CI-native `specrun review`, dependency-aware compatibility gates, SLM scoring, and hosted dashboards. All four need the same substrate:

- a stable answer to "which rules apply here?";
- a stable rule id that survives file moves and wording changes;
- structured findings that distinguish rule ids from report-local occurrence ids;
- structured fields for CI annotations, PR comments, retry loops, reviewing agents, and future dashboards;
- a boundary between deterministic checks and model-assisted judgment.

Today those pieces exist only partially:

- codex rule markdown files have frontmatter validated by `specdev check` (`specify-authoring` `check::codex`) against `crates/authoring/schemas/codex-rule.schema.json`;
- shared `UNI-`* rules and per-target overlays exist under `adapters/shared/codex/` and `adapters/targets/<name>/codex/`;
- target adapter review briefs instruct reviewers to add `rule_id` fields in `REVIEW.md`;
- contract rules already mention consumer-impact classifications that future cross-project review needs.

What is missing is the contract that joins them. Without it, every reviewer, scorer, CI integration, agent prompt pack, or dashboard would invent its own rule context and finding shape. That would make rule ids hard to aggregate and would blur the current 2.0 boundary between workflow control (`specrun`), engineering standards (codex), target adapter guidance, and agent-authored prose.

## Design

### Standards layer (workflow / artifacts / standards)


| Layer                 | Owns                                                                 | Must not                                                                  |
| --------------------- | -------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Workflow              | Phase skills; `specrun plan` / `slice` / `workspace` lifecycle verbs | Encode durable engineering policy in lifecycle files                      |
| Artifacts             | `proposal.md`, `spec.md`, `design.md`, `tasks.md`, baseline specs    | Substitute for codex policy or auto-enforce standards                     |
| Engineering standards | Codex rules; `specrun codex export`; future `specrun review`         | Transition plans, slices, or changes; mutate `.specify/` lifecycle fields |


**Authoring standards** (`docs/standards/` in the plugin repo, enforced by `specdev check`) govern how contributors write skills and docs. **Engineering standards** (this RFC) govern what generated and hand-written code must satisfy in consumer projects. The words overlap; the enforcement surfaces do not.

### Principles

1. **Codex rules are engineering standards, not workflow state.** Rule files live with adapters and shared references. They do not mutate `plan.yaml`, slice artifacts, `sources.yaml`, or `targets.yaml`.
2. **Rules are authored for agent judgment first.** The Markdown body is the canonical policy explanation for reviewing agents and humans; frontmatter makes that policy resolvable, filterable, and citeable.
3. **Deterministic hints are advisory and partial.** Hints help scanners catch mechanically observable subsets of a rule, but absence of a hint never means the rule is unenforceable by an agent or reviewer.
4. **The CLI owns resolution, not judgment.** The CLI can resolve, validate, and export rule sets. A scanner, model, or human decides whether a rule is violated.
5. **Findings cite rule ids, not file paths.** File paths help humans inspect rules; `rule-id` is the durable machine key.
6. **Report-local ids stay local.** A `REVIEW.md` occurrence id like `SEC-1` or `UNI-3` is not the same as a codex `rule-id` like `SEC-001` or `UNI-014`.
7. **Deterministic and model-assisted findings share one schema.** The producer changes, not the output contract.
8. **Source adapter and target adapter vocabulary is explicit.** Shared rules may apply everywhere; overlays are axis-specific under `adapters/sources/<name>/codex/` or `adapters/targets/<name>/codex/`.
9. **No lifecycle authority moves into review.** Review findings may block CI or operator approval, but they never transition a plan entry, slice, or change directly.

### CLI and binary split

[RFC-5](done/rfc-5-tooling.md) ports framework checks into the `specify-authoring` crate behind the `specdev` binary. Workflow operations live in `specrun`. RFC-28 preserves that split:


| Binary    | Crate                                                | Audience                                | RFC-28 role                                                                                         |
| --------- | ---------------------------------------------------- | --------------------------------------- | --------------------------------------------------------------------------------------------------- |
| `specdev` | `specify-authoring` (+ Phase 3 mapper in the binary) | Contributors editing `augentic/specify` | Codex **authoring** validation; Phase 3 adds optional `ReviewFinding` JSON export (`--format json`) |
| `specrun` | `specify-domain` + runtime handlers                  | Operators on consumer projects          | Codex **resolution**, export, and the `ReviewFinding` wire contract                                 |


Framework validation does not move into `specrun`. Runtime export does not replace `specdev check`. Both binaries ship from `augentic/specify-cli`; `make check` in the plugin repo forwards to `specdev check --codex-root .`.

### Codex file shape

The existing codex rule frontmatter shape remains the authoring surface:

```yaml
---
id: UNI-014
title: Hardcoded Configuration
severity: important
trigger: Generated code embeds environment-specific configuration instead of routing it through declared configuration.
applicability:
  adapters: [omnia]
  languages: [rust]
  artifacts: [code]
review_mode: hybrid
deterministic_hints:
  - kind: regex
    value: "https?://"
    description: Literal URL in generated code.
---

## Rule

Configuration values that vary between deployments must not be hardcoded in generated code.
```

[RFC-5](done/rfc-5-tooling.md) defines the framework authoring checks now implemented as `specdev check`: `check::codex` enforces the rule-id schema, namespace ownership, and frontmatter discipline. RFC-28 adds runtime resolution and export semantics to `specrun`; it does not replace the markdown authoring format or move framework validation into the runtime binary. [RFC-32](rfc-32-standards-enforcement.md) adds hint execution and the WorkspaceModel indexer that `specrun review` uses for deterministic subsets on consumer projects; RFC-28 **Phase 3** converges framework `specdev check` toward the same finding shape (Option A). Declarative `FRAME-`* migration (RFC-32 Phase 3 Option B) stays optional in RFC-32.

**Body conventions.** Rule bodies SHOULD open with `## Rule` (required by `check::codex`) and MAY add `## Look For` and `## Spec Guidance` sections to scope reviewer attention. RFC-28 exports `body` as verbatim markdown; a future RFC may surface named sections as parsed fields without breaking the wire shape, so authors are encouraged to use these section names rather than ad hoc alternatives.

### Namespaces

Rule ids stay closed over the first-party namespaces already used by the repository:


| Namespace  | Owner                                                                                                                                    |
| ---------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `UNI-`*    | Shared target-agnostic rules under `adapters/shared/codex/universal/`.                                                                   |
| `SRC-*`    | Shared source-axis rules owned jointly by source adapter authors under `adapters/sources/<name>/codex/`.                                 |
| `OMNIA-*`  | Omnia target adapter overlay under `adapters/targets/omnia/codex/`.                                                                      |
| `RUST-*`   | Rust rules owned by the Omnia target adapter until a shared Rust codex exists.                                                           |
| `SEC-*`    | Security rules owned by the Omnia target adapter until a shared security codex exists.                                                   |
| `VECTIS-*` | Vectis target adapter overlay under `adapters/targets/vectis/codex/`.                                                                    |
| `IFACE-*`  | Contracts target adapter overlay under `adapters/targets/contracts/codex/`.                                                              |
| `ORG-*`    | Organization-local rules outside the first-party repository.                                                                             |
| `FRAME-*`  | Reserved by [RFC-32](rfc-32-standards-enforcement.md) for optional framework-repo declarative checks; not used in consumer codex export. |


Framework authoring keeps enforcing namespace ownership for first-party files via `specdev check`. `ORG-*` is reserved for downstream projects and catalog imports; first-party adapters must not use it. `FRAME-*` is reserved for RFC-32 Phase 3 framework convergence and must not appear under `adapters/*/codex/`. `SRC-*` is the single shared namespace for **every** source-adapter overlay in v1 — each owner under `adapters/sources/<name>/codex/` may use `SRC-`* only (mirroring how target owners map to their closed namespaces). Per-source-adapter namespaces (e.g. `TS-*`, `DOC-*`) MAY be introduced in a follow-up RFC if any source adapter accumulates more than five `SRC-*` rules of its own. Adding both `SRC` and `FRAME` to the closed `ruleId` regex (`^(UNI|SRC|FRAME|RUST|IFACE|SEC|OMNIA|VECTIS|ORG)-[0-9]{3}$`) so RFC-32 Phase 3 declarative framework rules do not require a second schema bump, mapping every discovered source-adapter owner in `crates/authoring/src/check/codex.rs::CODEX_PROFILE_NAMESPACES` to `{"SRC"}` (same owner-discovery rule as today — first path segment under `adapters/sources/<name>/codex/`), and enforcing `FRAME-*` placement via the existing namespace-ownership predicate (reject any `FRAME-*` rule discovered under `adapters/{sources,targets}/<name>/codex/`) is the schema and authoring change required for first-party source overlays to land (Phase 1). Regex membership alone never grants placement — placement remains an explicit `check::codex` predicate so adding future namespaces to the regex never silently allows them under adapter trees.

### Resolution roots

The resolver reads these roots in order:

1. Shared universal rules: `adapters/shared/codex/universal/`.
2. Shared language or artifact packs, if added later under `adapters/shared/codex/<pack>/`.
3. Source adapter overlays for any bound source adapter: `adapters/sources/<name>/codex/`.
4. Target adapter overlay for the resolved target adapter: `adapters/targets/<name>/codex/`.
5. Project-local organization overlays, if configured later: `.specify/codex/` or an imported catalog projection.

The first implementation only needs roots 1, 3, and 4. Roots 2 and 5 are reserved by this RFC so the resolver shape does not need to change when shared packs or organization overlays land.

Root 1 (shared universal rules) resolves from a **codex root** — a checkout or packaged projection that contains first-party codex content such as `adapters/shared/codex/universal/`. Target and source adapter overlays (roots 3 and 4) use a closed location order:

1. project-local adapter overlay at `adapters/{sources,targets}/<name>/codex/`;
2. manifest-cache overlay at `.specify/.cache/manifests/{sources,targets}/<name>/codex/`;
3. codex-root fallback overlay at `{codex_root}/adapters/{sources,targets}/<name>/codex/` when `--codex-root` is supplied;
4. omit that overlay root when none of the above exists.

This keeps standalone export useful for agent prompt assembly and golden tests while preserving project-local/cache overlays as the authoritative runtime source when present. Init sparse-checkout caches only the target adapter parent (e.g. `adapters/targets/`), not `adapters/shared/`; v1 does **not** infer a codex root from `project.yaml:adapter` (GitHub sparse paths, `file://` adapter dirs, and temp checkouts are too unreliable).

#### Codex root resolution (v1)

Shared `UNI-`* inclusion uses this closed probe order:

1. `**--codex-root` when supplied** — use for root 1 and fallback overlays.
2. **Else if `{project_dir}/adapters/shared/codex/universal/` exists** — treat `project_dir` as the codex root (monorepo or full checkout co-located with the consumer project).
3. **Else** — fail with `codex-root-required` and a message that shared `UNI-`* rules require `--codex-root` pointing at a tree containing `adapters/shared/codex/universal/`.

Auto-derivation from `project.yaml:adapter` is deferred until init cache or project config explicitly carries a full codex tree.

### Resolution inputs

The resolver accepts a narrow context:


| Input                | Meaning                                                                                                                                                      |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `project_dir`        | Project root used for adapter resolution and optional project-local overlays.                                                                                |
| `codex_root`         | Root containing first-party codex content for shared rules and fallback overlays. Resolved per §"Codex root resolution (v1)"; omit only when step 2 applies. |
| `target_adapter`     | Target adapter name, optionally versioned as `<name>@v<major>`.                                                                                              |
| `source_adapters[]`  | Source adapter names bound by the active plan entry or supplied explicitly.                                                                                  |
| `artifact_paths[]`   | Optional project-relative paths to narrow applicability.                                                                                                     |
| `languages[]`        | Optional language tokens inferred by a scanner or supplied by a caller.                                                                                      |
| `include_deprecated` | Whether deprecated rules appear in the export. Defaults to false.                                                                                            |
| `include_unmatched`  | Whether rules with populated applicability dimensions the caller did not satisfy are included. Defaults to false.                                            |


The resolver may be called without a slice. Slice awareness belongs to the scanner; codex resolution is adapter- and artifact-aware, not lifecycle-aware.

### Applicability

Applicability filters are inclusive narrowing hints. No first-party codex file uses `applicability` today; v1 still implements the filter so [RFC-32](rfc-32-standards-enforcement.md) can pass `--artifact` and `--language` without a resolver shape change.

**V1 matching rules:**

- A rule with no `applicability` block applies wherever its root applies (pass-through after root and deprecation filters).
- `applicability.adapters` matches source adapter or target adapter names, with optional major versions.
- `applicability.languages` matches caller-supplied or scanner-inferred language tokens.
- `applicability.artifacts` matches broad artifact categories such as `code`, `tests`, `contracts`, `specs`, `design`, or `tasks`.
- `applicability.paths` matches the caller's `--artifact` path against project-relative glob patterns (see below).
- All populated dimensions must match (**AND** semantics).
- A populated dimension the caller did not supply → **exclude** the rule unless `--include-unmatched` is set.

#### Path glob semantics (`applicability.paths`)

Patterns follow the same constraints as the authoring schema (project-relative, no leading `/`, no `..`, no URI schemes). Matching uses the Rust `[glob](https://docs.rs/glob)` crate with case-sensitive path segments and `/` as the only separator in patterns:

- `*` matches within one path segment.
- `*`* matches across segments (e.g. `crates/**/src/**/*.rs` matches `crates/billing/src/lib.rs`).

When `--artifact` is omitted, `applicability.paths` is treated as an unsatisfied caller input (exclude unless `--include-unmatched`). The matcher compares against the single supplied artifact path, not a directory walk.

### Overlay precedence

Rules do not override each other by sharing ids. Duplicate rule ids are always invalid, including when one duplicate is deprecated. Deprecation only allows one rule to point to a different replacement id.

Overlay precedence controls review guidance, not rule identity:

1. Target adapter overlays are more specific than shared rules.
2. Source adapter overlays are more specific than shared rules for source-extraction findings.
3. Shared rules remain applicable unless the shared rule itself declares `deprecated.replaced_by` pointing to an overlay replacement id.
4. When multiple applicable rules describe the same concern, a scanner may report the most specific rule and list related rules in `related-rule-ids`.

This preserves stable historical citations while letting adapters sharpen shared guidance.

Export includes every rule that passes applicability filtering. Overlay precedence guides finding producers and human reviewers; it does not suppress shared rules from the resolved export.

### Resolved codex export

Add a read-only CLI surface on `specrun`:

```bash
specrun codex export --codex-root ../specify --target omnia --format json
specrun codex export --target omnia --source code-typescript --artifact crates/billing/src/lib.rs --format json
specrun codex export --codex-root ../specify --target contracts --include-deprecated --format json
```

When run from a consumer project whose tree contains `adapters/shared/codex/universal/`, `--codex-root` MAY be omitted (see §"Codex root resolution (v1)"). Otherwise pass `--codex-root` explicitly.

**Output format (v1).** JSON is the only supported export format. Human inspection uses `jq` or an editor; a compact text inventory and `--format text` are deferred to a follow-up. Reserved for later: pretty-printed JSON as a debugging alias.

Exported entries carry every codex-frontmatter field that is part of the rule contract: `rule-id`, `title`, `severity`, `trigger`, `review-mode`, `applicability`, `deterministic-hints`, `references`, `deprecated`. They also carry the markdown policy body after frontmatter as `body`, plus resolver-only fields: `origin` (`shared` | `source` | `target` | `organization`), `path-root` (`codex-root` | `project-dir`), and `path`.

`body` is the canonical agent-readable rule text, including headings such as `## Rule`; it is exported verbatim after the closing frontmatter delimiter. `references` are exported because reviewing agents need the same supporting links and local references a human would use. Frontmatter `snake_case` keys become kebab-case on the wire at every nesting level, so `review_mode`, `deterministic_hints`, and `deprecated.replaced_by` export as `review-mode`, `deterministic-hints`, and `deprecated.replaced-by`. `schemas/codex/resolved.schema.json` is the source of truth for exported field names, and Phase 2 fixtures MUST cover `deprecated.replaced_by` → `deprecated.replaced-by`.

`path` is relative to `path-root`: shared rules and codex-root fallback overlays use `codex-root`; project-local and cached overlays use `project-dir` (including `.specify/.cache/manifests/...` when that is the resolved adapter location). `references[].path` resolves relative to the same `path-root` as the rule file unless a future schema explicitly adds a separate reference root. The export does not include absolute paths, so golden output remains stable across machines.

```json
{
  "version": 1,
  "target-adapter": "omnia",
  "source-adapters": ["code-typescript"],
  "rules": [
    {
      "rule-id": "UNI-014",
      "title": "Hardcoded Configuration",
      "severity": "important",
      "trigger": "Generated code embeds environment-specific configuration instead of routing it through declared configuration.",
      "review-mode": "hybrid",
      "origin": "shared",
      "path-root": "codex-root",
      "path": "adapters/shared/codex/universal/hardcoded-configuration.md",
      "applicability": {
        "adapters": ["omnia"],
        "languages": ["rust"],
        "artifacts": ["code"]
      },
      "deterministic-hints": [
        {
          "kind": "regex",
          "value": "https?://",
          "description": "Literal URL in generated code."
        }
      ],
      "references": [
        {
          "label": "Omnia guardrails",
          "path": "adapters/targets/omnia/references/guardrails.md"
        }
      ],
      "body": "## Rule\n\nConfiguration values that vary between deployments must not be hardcoded in generated code.\n",
      "deprecated": null
    }
  ]
}
```

The severity enum is **ordered** for sort stability: `critical < important < suggestion < optional`. Both the resolved export and finding producers MUST use this sequence when severity participates in a comparator tuple. The enum is closed; widening it requires an RFC and a v2 export envelope.

Ordering is stable:

1. non-deprecated before deprecated;
2. severity order per the closed enum above;
3. origin order: `target`, `source`, `shared`, `organization`;
4. `rule-id` lexical order.

### Deterministic hints extensibility

RFC-28 validates `deterministic_hints` shape only; it does not execute hints. Hints are optional signals for deterministic scanners, not the primary expression of codex policy. A rule with no hints remains fully valid and may still be applied by a model-assisted or human reviewer using `body`, `trigger`, `severity`, `applicability`, and `references`.

The closed v1 authoring enum is:


| Kind                                                                                                                           | RFC-28 validation                         | Execution owner                                                    |
| ------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------- | ------------------------------------------------------------------ |
| `regex`                                                                                                                        | shape                                     | [RFC-32](rfc-32-standards-enforcement.md) Phase 2                  |
| `path-pattern`                                                                                                                 | shape                                     | RFC-32 Phase 2                                                     |
| `schema`                                                                                                                       | shape                                     | RFC-32 Phase 2                                                     |
| `tool`                                                                                                                         | shape                                     | RFC-32 Phase 2                                                     |
| `unique`, `reference-resolves`, `set-coverage`, `cardinality`, `constant-eq`, `set-eq`, `content-digest-eq`, `namespace-owner` | shape with `"x-rfc32-status": "reserved"` | RFC-32 reserved; interpreter returns unsupported until implemented |


Rules may declare reserved kinds in frontmatter so authors can land policy before the interpreter catches up. RFC-28 export includes reserved hints verbatim; scanners must not treat reserved kinds as executable unless RFC-32 implements them.

When extending the codex authoring schema, add new kinds to the enum and document them in RFC-32 before implementation. Do not embed scripts, SQL, or unconstrained query strings in hint `value` fields.

### Structured review finding schema

Add `schemas/review/finding.schema.json` and `schemas/codex/resolved.schema.json` under `augentic/specify-cli`, with corresponding Rust DTOs in `specify-domain`. The schema is shared by `specrun review`, target adapter review briefs, SLM scorers, CI annotations, and any future dashboards.

Field names are kebab-case on the wire:


| Field              | Required | Notes                                                                                                      |
| ------------------ | -------- | ---------------------------------------------------------------------------------------------------------- |
| `id`               | yes      | Producer-local stable id for this run, e.g. `FIND-0001`. Not the codex rule id.                            |
| `rule-id`          | no       | Codex id such as `UNI-014`. Required when the finding maps to a codex rule.                                |
| `related-rule-ids` | no       | Additional codex ids that informed the finding.                                                            |
| `title`            | yes      | Short finding title.                                                                                       |
| `severity`         | yes      | `critical`, `important`, `suggestion`, or `optional`.                                                      |
| `source`           | yes      | `deterministic`, `model-assisted`, `hybrid`, or `human`.                                                   |
| `target-adapter`   | no       | Target adapter name when adapter-specific.                                                                 |
| `source-adapter`   | no       | Source adapter name when source-extraction-specific.                                                       |
| `slice`            | no       | Slice name when the finding is slice-scoped.                                                               |
| `change`           | no       | Change name when known.                                                                                    |
| `artifact`         | yes      | `code`, `tests`, `contracts`, `specs`, `design`, `tasks`, `assets`, `tokens`, `composition`, or `unknown`. |
| `location`         | no       | File path plus optional line, column, and end positions.                                                   |
| `evidence`         | yes      | Bounded verbatim evidence, digest summary, or structured evidence; see union below.                        |
| `impact`           | yes      | Operator-facing risk.                                                                                      |
| `remediation`      | yes      | Concrete action to clear the finding.                                                                      |
| `confidence`       | no       | `high`, `medium`, or `low`; required for model-assisted findings.                                          |
| `fingerprint`      | yes      | Stable hash over rule id, location, evidence digest, and title for dedupe. Algorithm defined below.        |
| `status`           | no       | `open`, `fixed`, `accepted`, or `false-positive`; omitted by raw scanners.                                 |


Minimal JSON example:

```json
{
  "id": "FIND-0001",
  "rule-id": "UNI-014",
  "title": "Literal deployment URL in generated handler",
  "severity": "important",
  "source": "hybrid",
  "target-adapter": "omnia",
  "slice": "billing-invoice-export",
  "artifact": "code",
  "location": {
    "path": "crates/invoice_export/src/config.rs",
    "line": 18
  },
  "evidence": {
    "kind": "snippet",
    "value": "const BASE_URL: &str = \"https://api.example.com\";"
  },
  "impact": "Generated code will point every deployment at the same external endpoint.",
  "remediation": "Read the endpoint from Omnia configuration and add a required config key to the design.",
  "confidence": "high",
  "fingerprint": "sha256:..."
}
```

Evidence payloads have a 16 KiB cap after UTF-8 serialization. Longer evidence is replaced with `kind: digest`, `sha256`, `summary`, and an optional location list. Findings must not include secrets, full prompts, model transcripts, or full source files.

**Evidence union (v1).** `evidence.kind` is closed:


| Kind         | Required fields     | Notes                                                                                                                                      |
| ------------ | ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `snippet`    | `value`             | Bounded verbatim excerpt. Use for local code/prose evidence a reviewer can inspect directly.                                               |
| `digest`     | `sha256`, `summary` | Used when the underlying evidence is too large or sensitive to include. `locations[]` may point at contributing files.                     |
| `structured` | `summary`, `data`   | Used for domain evidence such as contract compatibility metadata. `data` is a JSON object; producers must keep it bounded and secret-free. |


The schema encodes this union with `oneOf` so each `kind` has one legal field set. It rejects additional top-level evidence fields except `locations` on `digest` and `structured`. `locations[]`, when present, uses the same location object shape as `location`.

**Fingerprint algorithm.** The `fingerprint` field is computed as:

```text
fingerprint = "sha256:" + hex(sha256(
    "v1\n"
  + rule-id-or-empty + "\n"
  + canonical(location) + "\n"
  + hex(sha256(evidence-payload))
))
```

where `canonical(location)` is `path + ":" + line.unwrap_or(0) + ":" + column.unwrap_or(0)` when `location` is present and the empty string otherwise. `evidence-payload` is the bytes of `evidence.value` for `kind: snippet`, the bytes of `evidence.summary` for `kind: digest`, and the bytes of `evidence.summary + "\n" + canonical-json(evidence.data)` for `kind: structured`. `canonical-json` means objects sorted by key, no insignificant whitespace, and UTF-8 strings; Phase 2 adds one shared helper in `specify-domain` for producers and validators rather than hand-rolling serializers at call sites. Producer-local `id`, `title`, `severity`, `confidence`, `status`, `change`, `slice`, `target-adapter`, and `source-adapter` are **excluded** so re-grading severity, attaching slice/change context after the fact, rephrasing a title between scanner runs, or migrating between producers does not duplicate findings for the same underlying issue. Distinguishing two genuinely-separate occurrences at the same `(rule-id, location)` is the job of `evidence` (which is in the hash via `evidence-payload`) and the `location` line/column, not of producer-controlled prose. The `v1` literal pins the algorithm; a future change requires a `v2` envelope.

### Review result envelope

The future `specrun review --format json` command emits:

```json
{
  "version": 1,
  "summary": {
    "critical": 0,
    "important": 2,
    "suggestion": 1,
    "optional": 0
  },
  "findings": []
}
```

This RFC defines the envelope so target adapter review briefs and scorers can adopt it early, even before the CLI scanner exists.

### Markdown report alignment

Target adapter review briefs may continue to emit `REVIEW.md` for humans. When a structured finding is also rendered to markdown:

- the markdown heading may use a report-local id (`SEC-1`, `UNI-3`, `FIND-0001`);
- the codex id appears as `rule_id: <id>` or `Rule: <id>`;
- severity words map exactly to the schema enum;
- file references map to `location`;
- "Risk" maps to `impact`;
- "Fix" or "Remediation" maps to `remediation`.

Markdown is presentation. JSON is the contract.

### Producer responsibilities

Deterministic producers must:

- cite a `rule-id` when the check exists to enforce a codex rule;
- produce byte-stable ordering by `(severity, rule-id, location.path, location.line, title)`;
- produce a stable `fingerprint`;
- avoid reporting safe additive changes as warnings when a codex rule classifies them as safe.

Model-assisted producers must:

- use a resolved codex export as input context;
- read `body` and `references` as the rule guidance, not just frontmatter;
- set `confidence`;
- include evidence specific enough for a reviewer to verify;
- leave `rule-id` absent rather than inventing one;
- never transition lifecycle state or mutate artifacts based on findings.

Human producers may use the same schema for triage decisions. Human-authored status changes belong in review reports or CI state, not in Specify lifecycle files.

### Relationship to framework authoring (`specdev`)

`specdev check` validates the Specify framework repository: rule file shape, duplicate ids, namespace ownership, broken links, skill frontmatter, and adapter brief discipline.

`specrun codex export` resolves rules for consumers.

`specrun review` scans deterministic subsets of consumer projects and emits findings per [RFC-32](rfc-32-standards-enforcement.md).

Authoring checks and runtime resolution share the codex **authoring schema** (duplicated once, guarded by `codex.schema-drift` in `specdev check`). Frontmatter parsing and resolution logic live in `specify-domain` for `specrun`; `specify-authoring` does not depend on `specify-domain` in Phases 1–2 — the drift predicate is the coupling surface between those phases. **Phase 3** adds a `Finding` → `ReviewFinding` mapper in the `specdev` binary layer (both crates are already dependencies of the root package) so `specify-authoring` stays free of `specify-domain`.

**Framework convergence (Phase 3).** Once Phase 2 defines the finding schema, Phase 3 maps today's imperative authoring rule ids (`skill.duplicate-name`, `links.unresolved`, …) into `ReviewFinding` JSON via `specdev check --format json`. Terminal output and non-zero exit semantics stay unchanged when `--format` is omitted. Declarative `FRAME-*` rules and framework WorkspaceModel (RFC-32 Phase 3 Option B) remain out of scope for this RFC; imperative predicates may remain indefinitely.

### Relationship to contracts and compatibility

Contract compatibility findings use the same finding schema. Contract-specific codex rules such as `IFACE-*` may add structured metadata inside `evidence` for producer project, consumer project, operation id, schema pointer, channel, message, classification, and `change-kind`.

The shared severity enum is not a compatibility classifier. Compatibility classifiers such as `additive`, `breaking`, `ambiguous`, and `unverifiable` remain contract-domain evidence fields.

## Implementation Plan

RFC-28 lands as **three sequenced phases** merged to main in a **single PR** across `augentic/specify` and `augentic/specify-cli`. Phase 1 extends authoring validation and plugin-repo docs; Phase 2 adds runtime resolution and export on `specrun`; Phase 3 converges framework `specdev check` to the same `ReviewFinding` wire shape. Implement phases in order on one branch; keep `make check` and `cargo make ci` green after each phase commit. [RFC-32](rfc-32-standards-enforcement.md) (`specrun review`, hint execution, WorkspaceModel, and optional `FRAME-*` migration) is out of scope except as the consumer-project enforcement layer and follow-on for declarative framework rules.

### Phase 1 — Authoring validation and plugin-repo alignment

**Repos:** `augentic/specify-cli` (primary), `augentic/specify` (docs, fixtures, editor schema copies)

**Goal:** Extend the codex authoring contract and align review docs before runtime export ships.

1. **Authoring schema** — `crates/authoring/schemas/codex-rule.schema.json`. Add both `SRC` and `FRAME` to the closed `ruleId` regex (`FRAME` is reserved for RFC-32 Phase 3 and rejected under `adapters/{sources,targets}/<name>/codex/` via the namespace-ownership predicate in step 2; landing the regex entry now avoids a second schema bump when RFC-32 Phase 3 lands); extend `deterministic_hints.kind` with RFC-32 reserved kinds (documented, not executed) only where the schema needs to accept already-authored policy. Do not make deterministic hints required for any rule.
2. `**check::codex`** — `crates/authoring/src/check/codex.rs`. Map every source-adapter owner discovered under `adapters/sources/<name>/codex/` to `{"SRC"}` in `CODEX_PROFILE_NAMESPACES` (not a single hardcoded adapter name). Add a placement predicate that rejects any `FRAME-*` rule discovered under `adapters/{sources,targets}/<name>/codex/`; `FRAME-*` placement under `adapters/shared/codex/framework/` is owned by [RFC-34 §F3](rfc-34-framework-convergence.md#f3--check-codex-placement-and-resolution) and lands when RFC-34 ships.
3. **Editor copies** — `.cursor/schemas/codex-rule.schema.json` in `augentic/specify` stays aligned with the authoring schema.
4. **Source-overlay smoke fixture** — one `SRC-*` rule under `adapters/sources/documentation/codex/` to exercise resolution root 3 and future export `origin: source`.
5. **Review brief alignment** — rewrite severity vocabulary (`CRITICAL → critical`, `HIGH → important`, `MEDIUM → suggestion`, `LOW → optional`), `rule_id` examples, and finding-shape callouts in:
  - Omnia: `adapters/targets/omnia/references/{review-output-template,review-categories,review-team-protocol}.md` and `adapters/targets/omnia/briefs/build/review.md`.
  - Vectis: `adapters/targets/vectis/briefs/build/{core,ios,android}/review.md` and `adapters/targets/vectis/references/review/{team-protocol-core,team-protocol-ios,team-protocol-android,iteration-report}.md`. Vectis `rule_id` examples MUST use the valid `VECTIS-NNN` form; the current `VECTIS-CORE-001` placeholder fails the codex `ruleId` regex.
  - Contracts: `adapters/targets/contracts/briefs/merge.md` and the per-format verifier references under `adapters/targets/contracts/references/`.
  - Shared codex README: `adapters/shared/codex/universal/README.md` — schema link points at `crates/authoring/schemas/codex-rule.schema.json` (documented via [docs/contributing/checks.md](../docs/contributing/checks.md)), not retired `tooling/` paths.
6. **Acceptance** — `make check` passes in `augentic/specify`; `cargo make check` passes in `specify-cli`; `specdev check` fixtures cover `SRC-*` namespace ownership and reserved hint kind shape.

**Done when:** authoring schema and review docs are settled; no stale `tooling/` references remain in **codex contributor paths** (primarily `adapters/shared/codex/universal/README.md` and related codex docs).

**Editor hygiene (same release train, not blocking Phase 2).** Retired `tooling/schemas/` paths in `.vscode/settings.json` and `.cursor-plugin/marketplace.json` should point at `.cursor/schemas/` or `specify-cli` authoring schemas in a separate small PR. Do not expand RFC-28 to delete the legacy `tooling/` tree or rewrite historical RFC-5 prose.

### Phase 2 — Runtime resolution and export (`specrun`)

**Repo:** `augentic/specify-cli` (primary); golden fixtures reference adapter trees from `augentic/specify` via fixture paths

**Goal:** Read-only codex resolution, export, and the structured finding contract.

1. **Runtime schemas** — add `schemas/codex/resolved.schema.json`, `schemas/codex/codex-rule.schema.json` (vendored copy of the authoring schema), and `schemas/review/finding.schema.json`.
2. **Schema drift predicate** — `codex.schema-drift` in `specdev check` asserts SHA-256 parity between `crates/authoring/schemas/codex-rule.schema.json` and `schemas/codex/codex-rule.schema.json`; fails with a single "regenerate via `scripts/sync-codex-schema.sh`" hint. The sync script is a deterministic byte-for-byte copy from the authoring schema to the vendored runtime schema (no `jq` pipeline, no reformatting); contributors run it after touching the authoring schema, and CI uses the predicate — not the script — to gate parity.
3. **Domain types** — `CodexRule`, `ResolvedCodex`, `ReviewFinding`, `FindingLocation`, and `FindingEvidence` in `crates/domain/src/codex/` (or `codex.rs` module tree). Frontmatter parsing and resolution live here (same `include_str!` pattern as adapter schemas in `crates/domain/src/adapter/core.rs`).
4. **Resolver** — roots, applicability, deprecation filtering, and stable ordering per §Design; shared universal rules from `--codex-root`.
5. `**specrun codex export`** — read-only subcommand under `specrun codex`. Flags: `--codex-root`, `--target`, `--source` (repeatable), `--artifact`, `--language`, `--include-deprecated`, `--include-unmatched`. Output is JSON only (`--format json`, default). Does not require `.specify/` when `--codex-root` and `--target` are supplied; uses project adapter resolution when available and codex-root overlay fallback when not. Codex-root resolution follows §"Codex root resolution (v1)"; golden test: cached consumer + `--target omnia` without shared tree → `codex-root-required`; with `--codex-root` → stable output including `UNI-*`, target overlay rules, `body`, `references`, `path-root`, and `path`.
6. **Finding validation** — helper and fixtures for valid findings, missing required fields, invalid severities, oversize evidence, invalid fingerprints, invalid rule ids, and strict `oneOf` evidence variants. Fingerprint fixtures MUST cover: identical inputs → identical fingerprint; changing only producer-side excluded fields (`id`, `title`, `severity`, `confidence`, `status`, `change`, `slice`, `target-adapter`, `source-adapter`) → identical fingerprint; changing `rule-id`, `location`, or `evidence-payload` → different fingerprint.
7. **Acceptance** — golden tests export codex for `omnia`, `vectis`, and `contracts` with shared-rule inclusion, overlay inclusion, deprecation filtering, stable ordering, applicability pass-through (rules without `applicability` always export), and the Phase 1 `SRC-*` smoke fixture (`origin: source`). At least one golden asserts the export is agent-consumable: `body` contains the markdown `## Rule` section, `references` survive frontmatter parsing, `deprecated.replaced_by` exports as `deprecated.replaced-by`, and `path-root` + `path` resolve to the source file without absolute paths in the JSON.
8. **Roadmap alignment** — point RM-10 review and compatibility items at this RFC as the rule export and finding-schema source.

**Done when:** `specrun codex export --codex-root … --target omnia --format json` produces stable golden output that can be used directly as reviewing-agent context; `codex.schema-drift` passes; `cargo make ci` green.

### Phase 3 — Framework finding export (`specdev`)

**Repo:** `augentic/specify-cli` (primary); golden fixtures may reference `augentic/specify` paths

**Depends on:** Phase 2 (`ReviewFinding` types, `schemas/review/finding.schema.json`, validation helpers)

**Goal:** Emit RFC-28 findings from framework authoring checks without migrating predicates or merging binaries (RFC-32 Phase 3 Option A).

1. **Mapper** — `Finding` → `ReviewFinding` in the `specdev` command path (`src/authoring/` or `crates/authoring/src/` module imported only from the binary). Map `rule_id` from today's imperative ids (`skill.duplicate-name`, `codex.namespace-owner`, …) into `rule-id` unchanged; set `source: deterministic`; derive `location` and `evidence` from existing `Finding` fields; compute `fingerprint` per §"Structured review finding schema". Do not add `specify-authoring` → `specify-domain` as a crate dependency — keep the mapper at the binary boundary.
2. `**specdev check --format`** — `json` emits a versioned envelope of `ReviewFinding` objects to stdout on success and on validation failure (findings present, exit `2` per existing validation semantics); default (omit flag) keeps today's human-oriented stderr/stdout. Document in `docs/contributing/checks.md` and `specdev` `--help`.
3. **Severity mapping** — map authoring severities to the closed RFC-28 enum (`critical` | `important` | `suggestion` | `optional`); document the table in the mapper module and cover with fixtures.
4. **Golden fixtures** — at least one fixture run: `specdev check --codex-root <fixture> --format json` with stable finding JSON (fingerprints, rule ids, locations). Assert schema validation against `schemas/review/finding.schema.json`.
5. **Acceptance** — `cargo make ci` green; `make check` behavior unchanged without `--format json`; optional CI job or documented recipe may consume JSON for unified annotations (RM-10 prep).

**Done when:** `specdev check --codex-root … --format json` produces stable golden `ReviewFinding` output for a representative framework tree; imperative checks are unchanged; no `FRAME-`* rules or hint interpreter shipped.

**Out of scope for Phase 3:** declarative `FRAME-`* rules, framework WorkspaceModel, retiring imperative `check::*` modules, shared pretty-print formatters (RFC-32 follow-on).

### Rollout order

Implement **Phase 1** first (authoring contract + plugin docs + smoke fixture), then **Phase 2** (runtime export consumes the settled schema), then **Phase 3** (framework JSON findings; requires Phase 2 finding types). Merge all three phases to main in one PR. The drift predicate in Phase 2 is the byte-level coupling between Phase 1 and Phase 2; Phase 3 couples only through `ReviewFinding` + schema validation.

### Cross-document alignment

When this RFC moves to **Accepted**, run a lightweight editorial pass on sibling docs — not blocking Phase 2 implementation if implementers treat RFC-28 as canonical:

- [RFC-32](rfc-32-standards-enforcement.md): rename stale `specify codex export` / `tooling check` references to `specrun codex export` / `specdev check`, note Phase 3 owns Option A finding export, and describe RFC-32 as consumer-project deterministic enforcement over agent-readable codex exports.
- [roadmap.md](roadmap.md) RM-10 / RM-16: same renames.
- [RFC-5](done/rfc-5-tooling.md): historical body may stay; add a one-line note at the top pointing to [docs/contributing/checks.md](../docs/contributing/checks.md) for current `specdev` paths if needed.

### Out of scope (RFC-32 and beyond Phase 3)

- `specrun review` scanner and hint interpreter
- WorkspaceModel indexer
- Declarative `FRAME-`* framework rules and framework `scan_profile` (RFC-32 Phase 3 Option B)
- Retiring imperative `specify-authoring` predicates in favor of hints
- Shared pretty-print / GitHub annotation formatters for findings
- `--format text` human inventory for `specrun codex export`
- Auto-derivation of `--codex-root` from `project.yaml:adapter`

## Migration

For operators:

- Existing `REVIEW.md` reports remain readable. Structured findings are additive.
- CI integrations should consume JSON findings when available and treat markdown as human presentation.

For adapter authors:

- Existing codex files remain valid if they pass the current frontmatter schema.
- New rules should pick a stable namespace id, write the body as agent-usable review guidance, and avoid embedding scanner-specific instructions in the rule body.
- Target adapter review briefs should emit `rule-id` separately from report-local ids.

For framework contributors (after Phase 3):

- `make check` behavior is unchanged. Use `specdev check --codex-root . --format json` when CI or dashboards need `ReviewFinding` JSON; imperative rule ids (`skill.*`, `links.*`, …) are unchanged.

For CLI maintainers:

- Keep `specrun codex export` read-only.
- Keep scanner behavior out of the resolver.
- Keep review findings separate from lifecycle transition logic.
- Keep framework authoring validation on `specdev`; do not fold it into `specrun`.
- Keep the Phase 3 mapper at the `specdev` binary boundary; do not require `specify-authoring` → `specify-domain`.

## Alternatives Considered

**Let `specrun review` define findings later.** Rejected. The review command depends on this contract; delaying the schema would force every early producer to invent incompatible output.

**Use SARIF directly as the primary output.** Rejected for v1. SARIF is useful as an export format, but Specify needs workflow-specific fields such as adapter names, slice, change, codex rule id, evidence kind, and authority boundaries. A SARIF adapter can be added later.

**Make codex markdown the only source of truth and skip export.** Rejected. Agents, CI, scorers, and dashboards need a structured rule inventory that has already applied root selection, applicability, deprecation, and ordering.

**Make Cursor `.mdc` or `rules.md` the canonical rule layer.** Rejected. Editor-specific rule files are useful prompt delivery surfaces, but they are not stable enough to be the durable Specify contract. Codex Markdown remains the canonical agent-readable policy; resolved export can feed prompt packs or generated editor rules later.

**Treat target adapter overlays as overrides by id.** Rejected. Stable ids are audit keys. Reusing an id with different semantics makes historical findings ambiguous. Replacement uses deprecation metadata instead.

**Allow findings to auto-open slices.** Rejected. Review can recommend follow-up work, but plan and slice lifecycle transitions stay under the existing `/spec:plan` and `/spec:execute` workflow.

## Non-Goals

- Implementing `specrun review` (owned by [RFC-32](rfc-32-standards-enforcement.md)).
- Declarative `FRAME-`* migration and framework WorkspaceModel (owned by RFC-32 Phase 3 Option B).
- Executing `deterministic_hints` or building WorkspaceModel (owned by RFC-32).
- Defining every deterministic scanner.
- Requiring every codex rule to be deterministic or hint-backed.
- Replacing codex Markdown with Cursor `.mdc`, `rules.md`, or another editor-specific rule format.
- Replacing target adapter review briefs.
- Replacing `REVIEW.md`.
- Adding hosted dashboards.
- Adding SARIF output in v1.
- `--format text` export for `specrun codex export`.
- Inferring `--codex-root` from `project.yaml:adapter` in v1.
- Mutating `plan.yaml`, slice artifacts, `sources.yaml`, or `targets.yaml` from review findings.
- Creating a new severity taxonomy beyond `critical`, `important`, `suggestion`, and `optional`.

## Resolved Decisions

Questions originally raised as Open Questions and resolved in this RFC:

- **CLI surface.** `specrun codex export` is the read-only verb under a `specrun codex` namespace. A future `specrun review` (RFC-32) MAY add a `rules` subcommand that returns the same envelope, but the resolver does not move; rule resolution is useful outside scanning, and `review rules` would conflate enforcement and read-only semantics.
- **Binary split.** Framework codex authoring validation stays on `specdev check` (`specify-authoring`). Resolution, export, and the `ReviewFinding` contract land on `specrun` (`specify-domain`). Both binaries ship from `augentic/specify-cli`; RFC-28 does not merge them.
- **Codex root for shared rules.** v1 uses the closed probe in §"Codex root resolution (v1)": explicit `--codex-root`, else `{project_dir}/adapters/shared/codex/universal/`, else `codex-root-required`. Target and source overlays resolve from project-local adapter trees first, then manifest cache, then codex-root fallback when `--codex-root` is supplied. Init sparse-checkout does not include shared rules; inferring codex root from `project.yaml:adapter` is deferred.
- **Export output format.** `specrun codex export` emits JSON only in v1; text inventory is deferred.
- **Agent-readable export.** Resolved export includes the markdown `body` and `references`; frontmatter alone is not enough context for reviewing agents.
- `**SRC-`* namespace ownership.** Every source adapter under `adapters/sources/<name>/codex/` owns `SRC-`* only; `check::codex` maps each discovered source owner to `{"SRC"}`.
- **Applicability in v1.** Filter logic ships even though no first-party rule populates `applicability` yet; missing block means pass-through; populated dimension + missing caller input means exclude unless `--include-unmatched`.
- **Path globs.** `applicability.paths` uses `glob` crate semantics with `*` / `*`* as documented in §Applicability; matches the single `--artifact` path when supplied.
- **Export vs overlay precedence.** Resolved export includes every applicability-matching rule; overlay precedence guides finding producers only.
- **Regex-syntax validation.** RFC-28 validates `deterministic_hints` shape only. Regex compilation and the choice of regex flavor belong to RFC-32's hint interpreter; the runtime resolver MUST NOT compile a regex it never executes.
- **Deterministic hints.** Hints are optional metadata for mechanically observable subsets. Codex policy remains valid without hints and is primarily consumed by agents and human reviewers.
- **Fingerprint composition.** Fingerprints exclude `id`, `title`, `severity`, `confidence`, `status`, `change`, `slice`, `target-adapter`, and `source-adapter`. Re-grading severity is a common stabilization activity, and `title` is producer-controlled prose that rephrases between deterministic patch releases and model-assisted runs; including either would multiply the same underlying issue across CI history. Distinguishing two genuine occurrences at the same `(rule-id, location)` is the job of `evidence` (which is in the hash) and the `location` line/column. See the algorithm in §"Structured review finding schema".

## Open Questions

1. Should organization-local `ORG-`* rules live in `.specify/codex/`, a registry projection, or both? Current preference: reserve both, implement neither until a downstream project needs it.
2. Should the finding schema include `autofix` fields in v1? Current preference: no. Autofix belongs to a later command that consumes findings.
3. Should review results include model metadata? Current preference: no for v1; RFC-19 observability can log bounded model/tool metadata later without putting it into the finding contract.

## References

- [Specify Roadmap](roadmap.md)
- [RFC-5: Framework Developer Tooling](done/rfc-5-tooling.md)
- [RFC-32: Engineering Standards — Deterministic Enforcement](rfc-32-standards-enforcement.md)
- [Standards layer (explanation)](../docs/explanation/standards-layer.md)
- [RFC-18: Specialized SLM Code Generation](future/rfc-18-slm.md)
- [RFC-25: Workflow](done/rfc-25-workflow.md)
- [RFC-27: Synthesis Sharpening](done/rfc-27-synthesis.md)
- [Shared target codex](../../adapters/shared/codex/universal/README.md)
- [Omnia review output template](../../adapters/targets/omnia/references/review-output-template.md)
- [Consumer impact classification](../../adapters/targets/contracts/codex/consumer-impact-classification.md)

