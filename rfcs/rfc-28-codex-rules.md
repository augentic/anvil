# RFC-28: Codex Rules

> Status: Draft - Depends: [RFC-5](done/rfc-5-tooling.md) for codex authoring validation, [RFC-25](done/rfc-25-workflow.md), [RFC-27](done/rfc-27-synthesis.md) - Enables: [RFC-31](rfc-31-declarative-rules.md), [roadmap RM-10](roadmap.md#rm-10-ci-native-specify-review), [RFC-18](future/rfc-18-slm.md)

## Abstract

Define the durable rule-resolution and finding-output contract for Specify's review layer.

Specify already has first-party codex rule files under shared and per-target directories, plus target adapter review briefs that cite those rules in `REVIEW.md`. The missing piece is a deterministic bridge from "files on disk" to "review findings a CLI, CI job, PR comment, scorer, or dashboard can consume." This RFC adds that bridge without implementing the full `specify review` scanner.

This RFC adds:

1. **Resolved codex export** - a CLI-readable view that resolves shared rules plus source adapter and target adapter overlays into one ordered rule set for a project, target adapter, slice, or artifact path.
2. **Structured review finding schema** - a stable JSON shape for deterministic and model-assisted findings.
3. **Codex resolution rules** - namespace ownership, overlay precedence, applicability filters, deprecation handling, and stable ordering.
4. **Reviewer report alignment** - target adapter review briefs continue to write human `REVIEW.md`, but every machine-readable finding maps to the same schema.

The scanner that produces findings (`specify review`) is a follow-up surface defined by [RFC-31](rfc-31-declarative-rules.md). This RFC defines the rule and finding contract that scanner consumes.

## Motivation

The roadmap calls for CI-native `specify review`, dependency-aware compatibility gates, SLM scoring, and hosted dashboards. All four need the same substrate:

- a stable answer to "which rules apply here?";
- a stable rule id that survives file moves and wording changes;
- structured findings that distinguish rule ids from report-local occurrence ids;
- deterministic fields for CI annotations, PR comments, retry loops, and future dashboards;
- a boundary between deterministic checks and model-assisted judgment.

Today those pieces exist only partially:

- codex rule markdown files have frontmatter validated by `tooling/src/check/codex.rs` against `tooling/schemas/codex-rule.schema.json`;
- shared `UNI-`* rules and per-target overlays exist under `adapters/shared/codex/` and `adapters/targets/<name>/codex/`;
- target adapter review briefs instruct reviewers to add `rule_id` fields in `REVIEW.md`;
- contract rules already mention consumer-impact classifications that future cross-project review needs.

What is missing is the contract that joins them. Without it, every reviewer, scorer, CI integration, or dashboard would invent its own finding shape. That would make rule ids hard to aggregate and would blur the current 2.0 boundary between workflow control (`specify` CLI), target adapter guidance, and agent-authored prose.

## Design

### Principles

1. **Codex rules are policy, not workflow state.** Rule files live with adapters and shared references. They do not mutate `plan.yaml`, slice artifacts, `sources.yaml`, or `targets.yaml`.
2. **The CLI owns resolution, not judgment.** The CLI can resolve, validate, and export rule sets. A scanner or model decides whether a rule is violated.
3. **Findings cite rule ids, not file paths.** File paths help humans inspect rules; `rule-id` is the durable machine key.
4. **Report-local ids stay local.** A `REVIEW.md` occurrence id like `SEC-1` or `UNI-3` is not the same as a codex `rule-id` like `SEC-001` or `UNI-014`.
5. **Deterministic and model-assisted findings share one schema.** The producer changes, not the output contract.
6. **Source adapter and target adapter vocabulary is explicit.** Shared rules may apply everywhere; overlays are axis-specific under `adapters/sources/<name>/codex/` or `adapters/targets/<name>/codex/`.
7. **No lifecycle authority moves into review.** Review findings may block CI or operator approval, but they never transition a plan entry, slice, or change directly.

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

[RFC-5](done/rfc-5-tooling.md) defines the framework dev-tooling workspace at `augentic/specify/tooling/` that validates this shape: the `check::codex` module enforces the rule-id schema, namespace ownership, and frontmatter discipline from `tooling check`. RFC-28 adds runtime resolution and export semantics to the operator `specify` binary; it does not replace the markdown authoring format or move framework validation into the runtime CLI. [RFC-31](rfc-31-declarative-rules.md) adds hint execution and the WorkspaceModel indexer that `specify review` uses; optional Phase 3 there may later converge `tooling check` toward the same finding shape.

### Namespaces

Rule ids stay closed over the first-party namespaces already used by the repository:


| Namespace  | Owner                                                                                                                              |
| ---------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `UNI-*`    | Shared target-agnostic rules under `adapters/shared/codex/universal/`.                                                             |
| `SRC-*`    | Shared source-axis rules owned jointly by source adapter authors under `adapters/sources/<name>/codex/`.                           |
| `OMNIA-*`  | Omnia target adapter overlay under `adapters/targets/omnia/codex/`.                                                                |
| `RUST-*`   | Rust rules owned by the Omnia target adapter until a shared Rust codex exists.                                                     |
| `SEC-*`    | Security rules owned by the Omnia target adapter until a shared security codex exists.                                             |
| `VECTIS-*` | Vectis target adapter overlay under `adapters/targets/vectis/codex/`.                                                              |
| `IFACE-*`  | Contracts target adapter overlay under `adapters/targets/contracts/codex/`.                                                        |
| `ORG-*`    | Organization-local rules outside the first-party repository.                                                                       |
| `FRAME-*`  | Reserved by [RFC-31](rfc-31-declarative-rules.md) for optional framework-repo declarative checks; not used in consumer codex export. |


Framework tooling keeps enforcing namespace ownership for first-party files. `ORG-*` is reserved for downstream projects and catalog imports; first-party adapters must not use it. `FRAME-*` is reserved for Phase 3 framework convergence and must not appear under `adapters/*/codex/`. `SRC-*` is the single shared namespace for every source-adapter overlay in v1; per-source-adapter namespaces (e.g. `TS-*`, `DOC-*`) MAY be introduced in a follow-up RFC if any source adapter accumulates more than five `SRC-*` rules of its own. Adding `SRC` to the closed `ruleId` regex (`^(UNI|SRC|RUST|IFACE|SEC|OMNIA|VECTIS|ORG)-[0-9]{3}$`) and an `SRC` entry to `tooling/src/check/codex.rs::CODEX_PROFILE_NAMESPACES` is the one-line schema and tooling change required for first-party source overlays to land.

### Resolution roots

The resolver reads these roots in order:

1. Shared universal rules: `adapters/shared/codex/universal/`.
2. Shared language or artifact packs, if added later under `adapters/shared/codex/<pack>/`.
3. Source adapter overlays for any bound source adapter: `adapters/sources/<name>/codex/`.
4. Target adapter overlay for the resolved target adapter: `adapters/targets/<name>/codex/`.
5. Project-local organization overlays, if configured later: `.specify/codex/` or an imported catalog projection.

The first implementation only needs roots 1, 3, and 4. Roots 2 and 5 are reserved by this RFC so the resolver shape does not need to change when shared packs or organization overlays land.

### Resolution inputs

The resolver accepts a narrow context:


| Input                | Meaning                                                                       |
| -------------------- | ----------------------------------------------------------------------------- |
| `project_dir`        | Project root used for adapter resolution and optional project-local overlays. |
| `target_adapter`     | Target adapter name, optionally versioned as `<name>@<version>`.              |
| `source_adapters[]`  | Source adapter names bound by the active plan entry or supplied explicitly.   |
| `artifact_paths[]`   | Optional project-relative paths to narrow applicability.                      |
| `languages[]`        | Optional language tokens inferred by a scanner or supplied by a caller.       |
| `include_deprecated` | Whether deprecated rules appear in the export. Defaults to false.             |


The resolver may be called without a slice. Slice awareness belongs to the scanner; codex resolution is adapter- and artifact-aware, not lifecycle-aware.

### Applicability

Applicability filters are inclusive narrowing hints:

- A rule with no `applicability` applies wherever its root applies.
- `applicability.adapters` matches source adapter or target adapter names, with optional major versions.
- `applicability.languages` matches caller-supplied or scanner-inferred language tokens.
- `applicability.artifacts` matches broad artifact categories such as `code`, `tests`, `contracts`, `specs`, `design`, or `tasks`.
- `applicability.paths` matches project-relative path or glob patterns.

All populated applicability dimensions must match. Missing caller context does not match a populated dimension unless the caller explicitly asks for unresolved rules with `--include-unmatched`.

### Overlay precedence

Rules do not override each other by sharing ids. Duplicate live ids are invalid unless one rule is deprecated and points to a replacement.

Overlay precedence controls review guidance, not rule identity:

1. Target adapter overlays are more specific than shared rules.
2. Source adapter overlays are more specific than shared rules for source-extraction findings.
3. Shared rules remain applicable unless an overlay declares `deprecated.replaced_by` from the shared id to the overlay id.
4. When multiple applicable rules describe the same concern, a scanner may report the most specific rule and list related rules in `related-rule-ids`.

This preserves stable historical citations while letting adapters sharpen shared guidance.

### Resolved codex export

Add a read-only CLI surface:

```bash
specify codex export --target omnia --format json
specify codex export --target omnia --source code-typescript --artifact crates/billing/src/lib.rs --format json
specify codex export --target contracts --include-deprecated --format json
```

Human output is a compact ordered inventory. JSON output is the stable machine contract.

Exported entries carry every codex-frontmatter field that is part of the rule contract: `rule-id`, `title`, `severity`, `trigger`, `review-mode`, `applicability`, `deterministic-hints`, `deprecated`. They also carry resolver-only fields: `origin` (`shared` | `source` | `target` | `organization`) and `path` (repo-relative authoring path, for humans). `references[]` and the markdown body are not exported; consumers that need them MUST resolve `path`.

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

RFC-28 validates `deterministic_hints` shape only; it does not execute hints. The closed v1 authoring enum is:


| Kind                                                                                                                           | RFC-28 validation                         | Execution owner                                                    |
| ------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------- | ------------------------------------------------------------------ |
| `regex`                                                                                                                        | shape                                     | [RFC-31](rfc-31-declarative-rules.md) Phase 2                        |
| `path-pattern`                                                                                                                 | shape                                     | RFC-31 Phase 2                                                     |
| `schema`                                                                                                                       | shape                                     | RFC-31 Phase 2                                                     |
| `tool`                                                                                                                         | shape                                     | RFC-31 Phase 2                                                     |
| `unique`, `reference-resolves`, `set-coverage`, `cardinality`, `constant-eq`, `set-eq`, `content-digest-eq`, `namespace-owner` | shape with `"x-rfc31-status": "reserved"` | RFC-31 reserved; interpreter returns unsupported until implemented |


Rules may declare reserved kinds in frontmatter so authors can land policy before the interpreter catches up. RFC-28 export includes reserved hints verbatim; scanners must not treat undeclared kinds as errors.

When extending the codex authoring schema, add new kinds to the enum and document them in RFC-31 before implementation. Do not embed scripts, SQL, or unconstrained query strings in hint `value` fields.

### Structured review finding schema

Add `specify-cli/schemas/review/finding.schema.json` and corresponding Rust DTOs. The schema is shared by `specify review`, target adapter review briefs, SLM scorers, CI annotations, and any future dashboards.

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
| `evidence`         | yes      | Bounded verbatim evidence or structured summary.                                                           |
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

Evidence payloads have a 16 KiB cap. Longer evidence is replaced with `kind: digest`, `sha256`, `summary`, and an optional location list. Findings must not include secrets, full prompts, model transcripts, or full source files.

**Fingerprint algorithm.** The `fingerprint` field is computed as:

```text
fingerprint = "sha256:" + hex(sha256(
    "v1\n"
  + rule-id-or-empty + "\n"
  + canonical(location) + "\n"
  + hex(sha256(evidence-payload)) + "\n"
  + title
))
```

where `canonical(location)` is `path + ":" + line.unwrap_or(0) + ":" + column.unwrap_or(0)` when `location` is present and the empty string otherwise, and `evidence-payload` is the bytes of `evidence.value` for `kind: snippet` or the bytes of `evidence.summary` for `kind: digest`. Producer-local `id`, `severity`, `confidence`, `status`, `change`, `slice`, `target-adapter`, and `source-adapter` are **excluded** so re-grading severity, attaching slice/change context after the fact, or migrating between producers does not duplicate findings for the same underlying issue. The `v1` literal pins the algorithm; a future change requires a `v2` envelope.

### Review result envelope

The future `specify review --format json` command emits:

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
- set `confidence`;
- include evidence specific enough for a reviewer to verify;
- leave `rule-id` absent rather than inventing one;
- never transition lifecycle state or mutate artifacts based on findings.

Human producers may use the same schema for triage decisions. Human-authored status changes belong in review reports or CI state, not in Specify lifecycle files.

### Relationship to framework tooling

`tooling check` validates the Specify framework repository: rule file shape, duplicate ids, namespace ownership, broken links, skill frontmatter, and adapter brief discipline.

`specify codex export` resolves rules for consumers.

`specify review` scans consumer projects and emits findings per [RFC-31](rfc-31-declarative-rules.md).

These surfaces share schemas, DTOs, and parsers through `specify-domain`, but they remain separate commands because their inputs, audiences, and failure semantics differ.

**RFC-31 Phase 3 (optional).** Framework-repo checks may later emit the same `ReviewFinding` JSON or migrate select predicates to declarative `FRAME-`* rules. RFC-28 does not require that convergence; imperative `tooling check` may remain indefinitely.

### Relationship to contracts and compatibility

Contract compatibility findings use the same finding schema. Contract-specific codex rules such as `IFACE-*` may add structured metadata inside `evidence` for producer project, consumer project, operation id, schema pointer, channel, message, classification, and `change-kind`.

The shared severity enum is not a compatibility classifier. Compatibility classifiers such as `additive`, `breaking`, `ambiguous`, and `unverifiable` remain contract-domain evidence fields.

## Implementation Plan

1. **Schemas.** Add `schemas/codex/resolved.schema.json` and `schemas/review/finding.schema.json` to `specify-cli`. Keep the codex authoring schema aligned with RFC-5's schema-first framework-tooling pass so `tooling check` validates the same shape that the resolver consumes. Extend the codex authoring schema enum with RFC-31 reserved hint kinds (documented, not executed here).
2. **Domain types.** Add `CodexRule`, `ResolvedCodex`, `ReviewFinding`, `FindingLocation`, and `FindingEvidence` DTOs in `specify-domain` or a small `specify-review` crate if dependency direction requires it.
3. **Resolver and shared parser.** Implement rule discovery and resolution in `specify-domain` (`crates/domain/src/codex.rs`) with the roots, applicability, deprecation, and ordering rules above. The runtime embeds a vendored copy of `tooling/schemas/codex-rule.schema.json` at `specify-cli/schemas/codex/codex-rule.schema.json` via `include_str!` (same pattern as the adapter schemas in `crates/domain/src/adapter/core.rs`). A new `tooling check` predicate (`codex.schema-drift`) asserts SHA-256 parity between the framework-authoritative copy and the runtime-embedded copy and fails with a single "regenerate via `scripts/sync-codex-schema.sh`" hint. This keeps frontmatter parsing in one place (`specify-domain`) without creating a runtime → framework dependency direction; the schema is duplicated once, checked once.
4. **CLI export.** Add `specify codex export` as a read-only subcommand. It does not require an initialized `.specify/` project when `--repo` and `--target` are supplied, but it may use project context when available.
5. **Finding validation.** Add a small validation helper and fixtures for good findings, missing required fields, invalid severities, oversize evidence, invalid fingerprints, and invalid rule ids.
6. **Review brief alignment (sub-deliverable).** As a single named change inside the implementing PR, rewrite severity vocabulary (`CRITICAL → critical`, `HIGH → important`, `MEDIUM → suggestion`, `LOW → optional`), `rule_id` examples, and finding-shape callouts in:
    - Omnia: `adapters/targets/omnia/references/{review-output-template,review-categories,review-team-protocol}.md` and `adapters/targets/omnia/briefs/build/review.md`.
    - Vectis: `adapters/targets/vectis/briefs/build/{core,ios,android}/review.md` and `adapters/targets/vectis/references/review/{team-protocol-core,team-protocol-ios,team-protocol-android,iteration-report}.md`. Vectis `rule_id` examples MUST use the valid `VECTIS-NNN` form; the current `VECTIS-CORE-001` placeholder fails the codex `ruleId` regex.
    - Contracts: `adapters/targets/contracts/briefs/merge.md` and the per-format verifier references under `adapters/targets/contracts/references/`.
    - Shared codex README: `adapters/shared/codex/universal/README.md` retargets its `.cursor/schemas/codex-rule.schema.json` link to the post-RFC-5 authoritative path `tooling/schemas/codex-rule.schema.json`.
7. **Roadmap alignment.** Point roadmap review and compatibility items at this RFC as the rule export and finding-schema source.
8. **Acceptance.** Add fixtures that export codex rules for `omnia`, `vectis`, and `contracts`, including shared-rule inclusion, overlay inclusion, deprecation filtering, and stable ordering. Include a single `SRC-*` smoke fixture under `adapters/sources/documentation/codex/` to exercise §"Resolution roots" root 3 (source-adapter overlay loading, `SRC-*` namespace ownership, export `origin: source`); this avoids leaving the source-axis walk in `tooling/src/check/codex.rs::discover_codex_rule_files` as untested dead code.
9. **Rollout order.** The implementing change spans both `augentic/specify` and `augentic/specify-cli`. Land in `specify` first (codex authoring schema enum extension for reserved hint kinds, `SRC-*` namespace addition in schema + `CODEX_PROFILE_NAMESPACES`, source-overlay smoke fixture, review-brief vocabulary rewrite, shared-codex README link retarget) and tag a release. Then land in `specify-cli` (re-vendored codex authoring schema, new `schemas/codex/` and `schemas/review/` files, `specify-domain` codex/review modules, `specify codex export` clap surface, golden tests). This direction matches the existing `tooling → specify-domain` git-tag dependency from RFC-5 and keeps `make check` / `cargo make ci` green at every commit in either repo.

## Migration

For operators:

- Existing `REVIEW.md` reports remain readable. Structured findings are additive.
- CI integrations should consume JSON findings when available and treat markdown as human presentation.

For adapter authors:

- Existing codex files remain valid if they pass the current frontmatter schema.
- New rules should pick a stable namespace id and avoid embedding scanner-specific instructions in the rule body.
- Target adapter review briefs should emit `rule-id` separately from report-local ids.

For CLI maintainers:

- Keep codex export read-only.
- Keep scanner behavior out of the resolver.
- Keep review findings separate from lifecycle transition logic.

## Alternatives Considered

**Let `specify review` define findings later.** Rejected. The review command depends on this contract; delaying the schema would force every early producer to invent incompatible output.

**Use SARIF directly as the primary output.** Rejected for v1. SARIF is useful as an export format, but Specify needs workflow-specific fields such as adapter names, slice, change, codex rule id, evidence kind, and authority boundaries. A SARIF adapter can be added later.

**Make codex markdown the only source of truth and skip export.** Rejected. Agents, CI, scorers, and dashboards need a structured rule inventory that has already applied root selection, applicability, deprecation, and ordering.

**Treat target adapter overlays as overrides by id.** Rejected. Stable ids are audit keys. Reusing an id with different semantics makes historical findings ambiguous. Replacement uses deprecation metadata instead.

**Allow findings to auto-open slices.** Rejected. Review can recommend follow-up work, but plan and slice lifecycle transitions stay under the existing `/spec:plan` and `/spec:execute` workflow.

## Non-Goals

- Implementing `specify review` (owned by [RFC-31](rfc-31-declarative-rules.md)).
- Executing `deterministic_hints` or building WorkspaceModel (owned by RFC-31).
- Defining every deterministic scanner.
- Replacing target adapter review briefs.
- Replacing `REVIEW.md`.
- Adding hosted dashboards.
- Adding SARIF output in v1.
- Mutating `plan.yaml`, slice artifacts, `sources.yaml`, or `targets.yaml` from review findings.
- Creating a new severity taxonomy beyond `critical`, `important`, `suggestion`, and `optional`.

## Resolved Decisions

Questions originally raised as Open Questions and resolved in this RFC:

- **CLI surface.** `specify codex export` is the read-only verb under a `specify codex` namespace. A future `specify review` (RFC-31) MAY add a `rules` subcommand that returns the same envelope, but the resolver does not move; rule resolution is useful outside scanning, and `review rules` would conflate enforcement and read-only semantics.
- **Regex-syntax validation.** RFC-28 validates `deterministic_hints` shape only. Regex compilation and the choice of regex flavor belong to RFC-31's hint interpreter; the runtime resolver MUST NOT compile a regex it never executes.
- **Fingerprint composition.** Fingerprints exclude `severity`, `confidence`, `status`, `change`, `slice`, `target-adapter`, and `source-adapter`. Re-grading severity is a common stabilization activity; including it would multiply the same underlying issue across CI history. See the algorithm in §"Structured review finding schema".

## Open Questions

1. Should organization-local `ORG-*` rules live in `.specify/codex/`, a registry projection, or both? Current preference: reserve both, implement neither until a downstream project needs it.
2. Should the finding schema include `autofix` fields in v1? Current preference: no. Autofix belongs to a later command that consumes findings.
3. Should review results include model metadata? Current preference: no for v1; RFC-19 observability can log bounded model/tool metadata later without putting it into the finding contract.

## References

- [Specify Roadmap](roadmap.md)
- [RFC-5: Framework Developer Tooling](done/rfc-5-tooling.md)
- [RFC-31: WorkspaceModel and Declarative Rule Execution](rfc-31-declarative-rules.md)
- [RFC-18: Specialized SLM Code Generation](future/rfc-18-slm.md)
- [RFC-25: Workflow](done/rfc-25-workflow.md)
- [RFC-27: Synthesis Sharpening](done/rfc-27-synthesis.md)
- [Shared target codex](../../adapters/shared/codex/universal/README.md)
- [Omnia review output template](../../adapters/targets/omnia/references/review-output-template.md)
- [Consumer impact classification](../../adapters/targets/contracts/codex/consumer-impact-classification.md)

