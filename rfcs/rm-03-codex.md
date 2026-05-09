# RM-03 Codex Rule Format Design Draft

> Purpose: design notes for implementing the codex rule format called for by RM-03 in `rfcs/roadmap.md`.

## Context

RM-03 introduces the stable rule surface that generators, reviewers, and future `specify review` checks can cite:

> **Goal:** Give generators and reviewers stable, citable engineering rules.
> **Seed:** `plugins/references/review-checks.md` and its existing `UNI-`* catalogue.
> **Each rule carries:** stable id, concise trigger, normative guidance, examples or references where useful, and applicability metadata.
> **First cut:** reserve namespaces such as `RUST-`*, `IFACE-`*, and `SEC-*`; add filtering metadata; migrate the seed catalogue without breaking existing ids.

The key design choice is storage. Because every Specify-owned repo builds against a capability, the rules that describe correct implementation should travel with capabilities. The foundational default codex should be modeled as a capability too, included for every Specify project rather than living as a separate root-level exception. Omnia owns Omnia SDK provider usage and WASM constraints. Vectis owns Crux core/shell boundaries. Contracts owns OpenAPI, AsyncAPI, JSON Schema, and compatibility policy. A repo-local rule directory remains useful, but only as an overlay for local standards.

Two roadmap principles bound the design:

- **Separate workflow, standards, and artifacts.** Codex rules are durable engineering standards. They are not slice artifacts, and they should not live inside `.specify/slices/`.
- **Keep enforcement surfaces distinct.** Codex rules feed consumer-project review (`specify review`) and model guidance. Framework-repo linting (`specify check`) may validate codex file shape, but it does not become the consumer-project reviewer.

## Scope

### In Scope

- A Markdown plus YAML-frontmatter rule format.
- Stable rule IDs and reserved namespaces.
- Capability-owned codex directories.
- Project-local and shared-catalog overlays.
- A deterministic rule resolver that returns the active codex for a project.
- Migration of `UNI-001` through `UNI-021` without changing IDs.
- Metadata that lets `specify review` decide whether a rule is deterministic, model-assisted, or hybrid.

### Out of Scope

- The final `specify review` finding schema. RM-04 owns the structured finding shape.
- The full reviewer implementation. RM-11 owns CI-native `specify review`.
- Suppression syntax, waiver workflow, or policy exceptions. Those should wait until findings exist.
- A dashboard or hosted rule catalog. RM-03 only defines the local source and resolved shape.
- Embedding codex prose into `AGENTS.md`. RM-02 stays concise and context-oriented.

## Design Summary

Codex rules are **capability-distributed and project-resolved**.

The first-party source tree uses this layout:

```text
capabilities/
  default/
    capability.yaml
    briefs/
    codex/
      input-validation.md
      persisted-state.md

  omnia/
    capability.yaml
    briefs/
    codex/
      rust/errors.md
      omnia/providers.md
      omnia/wasm-constraints.md
      security/secrets.md

  vectis/
    capability.yaml
    briefs/
    codex/
      crux/state.md
      interfaces/host-core-boundaries.md

  contracts/
    capability.yaml
    briefs/
    codex/
      openapi/compatibility.md
      asyncapi/compatibility.md
      json-schema/evolution.md
```

The default capability's `codex/` directory is intentionally flat:

```text
capabilities/default/codex/
  input-validation.md
  persisted-state.md
```

Consumer projects may add repo-local overlays:

```text
codex/
  project/logging.md
  project/ownership.md
```

The resolved rule set for a project is the union of:

1. The foundational `default` capability's `codex/` directory (`UNI-*` and other capability-independent rules).
2. The resolved project capability's `codex/` directory.
3. Any configured shared catalog codex.
4. The project's repo-root `codex/` overlay.

Rules are cited with both ID and provenance when provenance matters:

```text
default@1:UNI-002
omnia@1:RUST-003
contracts@1:IFACE-007
repo:ORG-004
```

Rule IDs remain globally unique within the resolved rule set. V1 should reject duplicate IDs rather than allow overriding. If a repo wants stricter policy than a capability rule, it should add a new local rule ID instead of redefining the capability rule.

## Rule Format

Rules are Markdown files with YAML frontmatter. The frontmatter is the machine-readable contract; the body is the human and model-readable guidance.

```markdown
---
id: UNI-002
title: Unvalidated Input
severity: critical
trigger: External or user-supplied data enters code without boundary validation.
---

## Rule

Validate all user-supplied or external data at the boundary before domain logic consumes it.

## Look For

- Empty or whitespace-only strings accepted as meaningful input.
- Numeric parameters without range or sign validation.
- ID lookups that assume the referenced object exists.
- External API payloads consumed without schema or type validation.

## Good

`title` is trimmed and rejected when empty before creating a task.

## Bad

A handler writes `request.title` directly into persisted state.

## Spec Guidance

If validation rules are absent from the spec, propose explicit acceptance criteria rather than only fixing code.
```

### Frontmatter Fields

Required frontmatter should stay small enough that a useful rule is cheap to write. Fields that route review, narrow applicability, or enrich catalogue browsing are optional and defaulted.


| Field      | Required | Meaning                                                                                                                                                                          |
| ---------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `id`       | yes      | Stable rule ID, e.g. `UNI-002`, `RUST-003`, `IFACE-007`.                                                                                                                         |
| `title`    | yes      | Short human-readable title.                                                                                                                                                      |
| `severity` | yes      | Default review severity: `critical`, `important`, `suggestion`, or `optional`. RM-04 may refine labels, but RM-03 should not use the old `warning` / `info` labels as canonical. |
| `trigger`  | yes      | One-sentence condition that tells a generator or reviewer when the rule matters.                                                                                                 |


### Body Headings

The only required body heading is `## Rule`. It should state the normative rule in self-contained prose. Other headings, such as `## Look For`, `## Good`, `## Bad`, and `## Spec Guidance`, are recommended when they make the rule easier for humans or models to apply, but V1 validation should not require them.

## Namespaces

V1 reserves these namespaces:


| Namespace  | Owner                                      | Examples                                                                                 |
| ---------- | ------------------------------------------ | ---------------------------------------------------------------------------------------- |
| `UNI-*`    | Default capability codex                   | Capability-independent review rules migrated from `plugins/references/review-checks.md`. |
| `RUST-*`   | Rust-producing capabilities                | Error handling, type safety, async, ownership, unsafe code.                              |
| `IFACE-*`  | Contracts and interface-aware capabilities | Backward compatibility, schema evolution, SemVer, consumer impact.                       |
| `SEC-*`    | Security rules                             | Secrets, injection, authz/authn, unsafe deserialization.                                 |
| `OMNIA-*`  | Omnia capability                           | Provider usage, WASM runtime constraints, guest wiring.                                  |
| `VECTIS-*` | Vectis capability                          | Crux state, core/shell contracts, platform shell boundaries.                             |
| `ORG-*`    | Repo or shared catalog                     | Organization-specific policy overlays.                                                   |


The seed `UNI-*` IDs must not be renumbered during migration. If a seed rule is split, keep the old ID as an umbrella rule and add narrower new IDs.

## Storage And Resolution

### Source Locations

Codex source locations are:

```text
<specify-distribution>/capabilities/default/codex/
<resolved-capability-root>/codex/
<shared-catalog-root>/codex/
<project-root>/codex/
```

`<project-root>/.specify/codex/` is not an authoring location. `.specify/` may later hold a resolved lock or cache, but human-authored rules should live in reviewable source directories.

### Capability Integration

V1 should discover a capability's codex by convention: if a capability root contains `codex/`, load every `*.md` rule beneath it. The default capability is always loaded first, then the project's resolved capability is loaded. This avoids changing the closed `capability.yaml` schema before the rule format settles.

A later capability manifest revision may add an explicit `codex:` field if capabilities need non-default paths or generated rule bundles. That should be a separate compatibility decision.

### Shared Catalogs

Shared catalogs are optional. They are useful for company-wide policy, but they should not be the first-class home for Omnia, Vectis, or Contracts rules. A shared catalog can add rules such as `ORG-*` and can be resolved from project config once the project config has a catalog field.

### Resolution Command

RM-03 should add a read-only CLI surface:

```bash
specify codex list
specify codex show <rule-id>
specify codex validate
specify codex export --format json
```

`specify codex export --format json` is the handoff point for `specify review`, skills, and hosted runners. The JSON shape should include:

- rule frontmatter fields,
- rendered Markdown body,
- source path,
- provenance (`capability`, `catalog`, or `repo`),
- capability identifier and version where applicable, including `default`.

## Deterministic Versus Model-Assisted Review

The codex format should classify how a rule can be enforced, but `specify review` decides how to run the checks.

### Deterministic CLI Logic

Use deterministic checks when the finding is structural, parseable, or reproducible:

- Codex rule parsing, duplicate IDs, namespace validation, and applicability validation.
- Artifact completeness and schema validation.
- Plan and registry consistency.
- Stale `AGENTS.md` by calling `specify context check`.
- Contract validation and compatibility classification where the contract tool can prove the result.
- Forbidden imports, forbidden APIs, unresolved links, missing files, and malformed frontmatter.
- High-confidence source patterns such as obvious hardcoded secrets, `unwrap()` policy, forbidden WASM APIs, or direct provider bypasses.

These checks should run in the CLI or declared tools and emit structured findings without model judgment.

### Model-Assisted Analysis

Use model-assisted review when the finding depends on intent, semantics, or cross-file reasoning:

- Logic bugs and incorrect state transitions.
- Whether implementation actually satisfies specs.
- Whether source changes are missing spec coverage.
- Whether specs lack implementation evidence.
- Responsibility-boundary violations that require domain understanding.
- Security issues involving data flow, authorization context, or trust boundaries.
- Performance and resource concerns that require understanding usage patterns.
- Spec-change indicators where the correct remediation may be to amend specs rather than code.

### Hybrid Rules

Some rules are hybrid. For example, `SEC-* hardcoded secrets` can have deterministic regex scanners, but a model may be needed to distinguish fixture placeholders from live credentials. Hybrid rules should include deterministic hints and clear model guidance.

## Migration Plan

1. **Add codex validation as a format-only parser.** Validate frontmatter shape, rule IDs, the required `## Rule` body heading, duplicate IDs, and provenance. Do not wire review yet.
2. **Migrate `UNI-`*.** Split `plugins/references/review-checks.md` into `capabilities/default/codex/` files while preserving every `UNI-001` through `UNI-021` ID.
3. **Add capability codex directories.** Start with the default foundational rules, Omnia provider/WASM/Rust rules, Contracts interface compatibility rules, and Vectis core/shell boundary rules.
4. **Expose `specify codex export --format json`.** This gives skills and future hosted runners one resolved rule surface.
5. **Teach reviewer skills to cite codex IDs.** Existing reviewer skills can keep their current team protocol while citing the resolved rule IDs.
6. **Feed RM-04 and RM-11.** Once the finding schema exists, `specify review` maps deterministic checks and model-assisted codex findings into that schema.

## Relationship To Existing Review Skills

The current Omnia reviewer already has three useful concepts:

- specialist prefixes (`SEC-`, `COR-`, `QUA-`),
- capability-independent checks (`UNI-`*),
- antagonist/model-assisted confirmation.

RM-03 should not preserve the current report numbering style as the rule ID model. `SEC-1` in a report is an occurrence number; `SEC-003` in codex is a stable rule. `specify review` findings should carry both:

```text
finding_id: review-local occurrence id
rule_id: SEC-003
rule_provenance: omnia@1
```

This keeps reports readable while making rules stable and citable.

## Open Questions

- Should shared catalog locations be configured in `.specify/project.yaml`, `registry.yaml`, or a future org-level config?
- Should `specify codex validate` live under `specify check` for framework repos, under `specify codex`, or both?
- Should deprecated rules remain active for historical finding links, or should they be hidden by default from `codex list`?
- What is the minimum JSON export shape RM-04 needs to avoid schema churn?

## Recommended Roadmap Answer

The short answer for RM-03 is:

- The rule format is Markdown with YAML frontmatter. Frontmatter gives stable IDs, triggers, severity, applicability, review mode, and provenance; the body gives normative guidance and examples.
- Codex rules should live with capabilities, including a foundational `default` capability for `UNI-*` rules, with repo-root `codex/` reserved as a local overlay. `.specify/codex/` should be reserved for generated cache or lock state, not human-authored rules.
- Deterministic review belongs in CLI logic and declared tools. Model-assisted review applies codex rules that require semantic judgment. Hybrid rules expose deterministic hints but still allow model review.

