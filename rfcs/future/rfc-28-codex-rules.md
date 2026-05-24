# RFC-28: Codex Resolution and Structured Review Findings

> Status: Draft - Depends: [RFC-5](../rfc-5-lint.md), [RFC-25](../done/rfc-25-workflow.md), [RFC-27](../done/rfc-27-synthesis.md) - Enables: [roadmap RM-10](../roadmap.md#rm-10-ci-native-specify-review), [RFC-18](rfc-18-slm.md)

## Abstract

Define the durable rule-resolution and finding-output contract for Specify's review layer.

Specify already has first-party codex rule files under shared and per-target directories, plus target adapter review briefs that cite those rules in `REVIEW.md`. The missing piece is a deterministic bridge from "files on disk" to "review findings a CLI, CI job, PR comment, scorer, or dashboard can consume." This RFC adds that bridge without implementing the full `specify review` scanner.

This RFC adds:

1. **Resolved codex export** - a CLI-readable view that resolves shared rules plus source adapter and target adapter overlays into one ordered rule set for a project, target adapter, slice, or artifact path.
2. **Structured review finding schema** - a stable JSON shape for deterministic and model-assisted findings.
3. **Codex resolution rules** - namespace ownership, overlay precedence, applicability filters, deprecation handling, and stable ordering.
4. **Reviewer report alignment** - target adapter review briefs continue to write human `REVIEW.md`, but every machine-readable finding maps to the same schema.

The scanner that produces findings (`specify review`) is a follow-up surface. This RFC defines the rule and finding contract it consumes.

## Motivation

The roadmap calls for CI-native `specify review`, dependency-aware compatibility gates, SLM scoring, and hosted dashboards. All four need the same substrate:

- a stable answer to "which rules apply here?";
- a stable rule id that survives file moves and wording changes;
- structured findings that distinguish rule ids from report-local occurrence ids;
- deterministic fields for CI annotations, PR comments, retry loops, and future dashboards;
- a boundary between deterministic checks and model-assisted judgment.

Today those pieces exist only partially:

- codex rule markdown files have frontmatter validated by `scripts/checks/codex.ts`;
- shared `UNI-*` rules and per-target overlays exist under `adapters/shared/codex/` and `adapters/targets/<name>/codex/`;
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

RFC-5 ports the framework linter that validates this shape. RFC-28 adds runtime resolution and export semantics; it does not replace the markdown authoring format.

### Namespaces

Rule ids stay closed over the first-party namespaces already used by the repository:

| Namespace | Owner |
|---|---|
| `UNI-*` | Shared target-agnostic rules under `adapters/shared/codex/universal/`. |
| `OMNIA-*` | Omnia target adapter overlay under `adapters/targets/omnia/codex/`. |
| `RUST-*` | Rust rules owned by the Omnia target adapter until a shared Rust codex exists. |
| `SEC-*` | Security rules owned by the Omnia target adapter until a shared security codex exists. |
| `VECTIS-*` | Vectis target adapter overlay under `adapters/targets/vectis/codex/`. |
| `IFACE-*` | Contracts target adapter overlay under `adapters/targets/contracts/codex/`. |
| `ORG-*` | Organization-local rules outside the first-party repository. |

The framework linter keeps enforcing namespace ownership for first-party files. `ORG-*` is reserved for downstream projects and catalog imports; first-party adapters must not use it.

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

| Input | Meaning |
|---|---|
| `project_dir` | Project root used for adapter resolution and optional project-local overlays. |
| `target_adapter` | Target adapter name, optionally versioned as `<name>@<version>`. |
| `source_adapters[]` | Source adapter names bound by the active plan entry or supplied explicitly. |
| `artifact_paths[]` | Optional project-relative paths to narrow applicability. |
| `languages[]` | Optional language tokens inferred by a scanner or supplied by a caller. |
| `include_deprecated` | Whether deprecated rules appear in the export. Defaults to false. |

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

Human output is a compact ordered inventory. JSON output is the stable machine contract:

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

Ordering is stable:

1. non-deprecated before deprecated;
2. severity order: `critical`, `important`, `suggestion`, `optional`;
3. origin order: `target`, `source`, `shared`, `organization`;
4. `rule-id` lexical order.

### Structured review finding schema

Add `specify-cli/schemas/review/finding.schema.json` and corresponding Rust DTOs. The schema is shared by `specify review`, target adapter review briefs, SLM scorers, CI annotations, and any future dashboards.

Field names are kebab-case on the wire:

| Field | Required | Notes |
|---|---|---|
| `id` | yes | Producer-local stable id for this run, e.g. `FIND-0001`. Not the codex rule id. |
| `rule-id` | no | Codex id such as `UNI-014`. Required when the finding maps to a codex rule. |
| `related-rule-ids` | no | Additional codex ids that informed the finding. |
| `title` | yes | Short finding title. |
| `severity` | yes | `critical`, `important`, `suggestion`, or `optional`. |
| `source` | yes | `deterministic`, `model-assisted`, `hybrid`, or `human`. |
| `target-adapter` | no | Target adapter name when adapter-specific. |
| `source-adapter` | no | Source adapter name when source-extraction-specific. |
| `slice` | no | Slice name when the finding is slice-scoped. |
| `change` | no | Change name when known. |
| `artifact` | yes | `code`, `tests`, `contracts`, `specs`, `design`, `tasks`, `assets`, `tokens`, `composition`, or `unknown`. |
| `location` | no | File path plus optional line, column, and end positions. |
| `evidence` | yes | Bounded verbatim evidence or structured summary. |
| `impact` | yes | Operator-facing risk. |
| `remediation` | yes | Concrete action to clear the finding. |
| `confidence` | no | `high`, `medium`, or `low`; required for model-assisted findings. |
| `fingerprint` | yes | Stable hash over rule id, location, evidence digest, and title for dedupe. |
| `status` | no | `open`, `fixed`, `accepted`, or `false-positive`; omitted by raw scanners. |

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

### Relationship to `specify check`

`specify check` validates the Specify framework repository: rule file shape, duplicate ids, namespace ownership, broken links, skill frontmatter, and adapter brief discipline.

`specify codex export` resolves rules for consumers.

`specify review` will eventually scan consumer projects and emit findings.

These surfaces may share schemas and parsers, but they remain separate commands because their inputs, audiences, and failure semantics differ.

### Relationship to contracts and compatibility

Contract compatibility findings use the same finding schema. Contract-specific codex rules such as `IFACE-*` may add structured metadata inside `evidence` for producer project, consumer project, operation id, schema pointer, channel, message, classification, and `change-kind`.

The shared severity enum is not a compatibility classifier. Compatibility classifiers such as `additive`, `breaking`, `ambiguous`, and `unverifiable` remain contract-domain evidence fields.

## Implementation Plan

1. **Schemas.** Add `schemas/codex/resolved.schema.json` and `schemas/review/finding.schema.json` to `specify-cli`. Keep the first-party authoring schema in the plugin repo until RFC-5 ports the framework linter, then share the DTOs where practical.
2. **Domain types.** Add `CodexRule`, `ResolvedCodex`, `ReviewFinding`, `FindingLocation`, and `FindingEvidence` DTOs in `specify-domain` or a small `specify-review` crate if dependency direction requires it.
3. **Resolver.** Implement rule discovery and resolution with the roots, applicability, deprecation, and ordering rules above. Reuse the RFC-5 port's markdown/frontmatter parser rather than reimplementing it twice.
4. **CLI export.** Add `specify codex export` as a read-only subcommand. It does not require an initialized `.specify/` project when `--repo` and `--target` are supplied, but it may use project context when available.
5. **Finding validation.** Add a small validation helper and fixtures for good findings, missing required fields, invalid severities, oversize evidence, invalid fingerprints, and invalid rule ids.
6. **Review brief alignment.** Update Omnia, Vectis, and contracts review briefs to describe the structured finding fields and the distinction between report-local ids and `rule-id`.
7. **Roadmap alignment.** Point roadmap review and compatibility items at this RFC as the rule export and finding-schema source.
8. **Acceptance.** Add fixtures that export codex rules for `omnia`, `vectis`, and `contracts`, including shared-rule inclusion, overlay inclusion, deprecation filtering, and stable ordering.

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

- Implementing `specify review`.
- Defining every deterministic scanner.
- Replacing target adapter review briefs.
- Replacing `REVIEW.md`.
- Adding hosted dashboards.
- Adding SARIF output in v1.
- Mutating `plan.yaml`, slice artifacts, `sources.yaml`, or `targets.yaml` from review findings.
- Creating a new severity taxonomy beyond `critical`, `important`, `suggestion`, and `optional`.

## Open Questions

1. Should `specify codex export` live under `specify codex` or as `specify review rules` once `specify review` exists? Current preference: `specify codex export`, because rule resolution is useful outside scanning.
2. Should organization-local `ORG-*` rules live in `.specify/codex/`, a registry projection, or both? Current preference: reserve both, implement neither until a downstream project needs it.
3. Should the finding schema include `autofix` fields in v1? Current preference: no. Autofix belongs to a later command that consumes findings.
4. Should deterministic hints validate regex syntax in RFC-28, or only shape? Current preference: shape only; scanner-specific validation belongs with the scanner.
5. Should finding fingerprints include `severity`? Current preference: no, so severity changes do not create duplicate findings for the same underlying issue.
6. Should review results include model metadata? Current preference: no for v1; RFC-19 observability can log bounded model/tool metadata later without putting it into the finding contract.

## References

- [Specify Roadmap](../roadmap.md)
- [RFC-5: Framework Linter](../rfc-5-lint.md)
- [RFC-18: Specialized SLM Code Generation](rfc-18-slm.md)
- [RFC-25: Workflow](../done/rfc-25-workflow.md)
- [RFC-27: Synthesis Sharpening](../done/rfc-27-synthesis.md)
- [Shared target codex](../../adapters/shared/codex/universal/README.md)
- [Omnia review output template](../../adapters/targets/omnia/references/review-output-template.md)
- [Consumer impact classification](../../adapters/targets/contracts/codex/consumer-impact-classification.md)
