# Workflow, standards, and artifacts

Specify separates three concerns that often collapse in agentic delivery stacks. Keep them distinct when authoring skills, adapters, CI jobs, and operator docs.

## The triad

| Layer | Role | On disk / in CLI |
| --- | --- | --- |
| **Workflow** | Orchestrate phases and lifecycle transitions | `/spec:*` phase skills; `specrun plan`, `specrun slice`, `specrun workspace` |
| **Artifacts** | Capture slice-local and baseline product intent | `proposal.md`, `spec.md`, `design.md`, `tasks.md`, `plan.yaml`, baseline under `.specify/specs/` |
| **Engineering standards** | Durable policy that outlives any slice | Codex rules under `adapters/**/codex/`; future `specrun codex export` and `specrun review` |

Workflow **mutates** `.specify/` state through a closed set of CLI verbs. Artifacts **record** what a slice means to build and merge. Engineering standards **constrain** how work is done — they do not transition plans, slices, or changes.

See the [decision log](decision-log.md#workflow-standards-and-artifacts) for rationale and RFC sources.

## Authoring standards vs engineering standards

Two different uses of "standards" appear in this repository:

| Term | Meaning | Location |
| --- | --- | --- |
| **Authoring standards** | House style for skills, docs, and framework contributions | [`docs/standards/`](../standards/) — enforced by `specdev check` (`make check`) on `augentic/specify` |
| **Engineering standards** | Durable engineering policy for generated and hand-written code | Codex markdown under `adapters/shared/codex/`, `adapters/targets/<name>/codex/`, and optional source overlays — resolved by `specrun codex export` and enforced by `specrun review` (planned) |

Do not conflate them. `docs/standards/skill-authoring.md` governs how to write a `SKILL.md`; `UNI-*` / `OMNIA-*` codex files govern what Omnia guest code must never do.

## How standards are enforced

Enforcement splits by audience, not by rule id:

| Surface | Binary | Audience | What it checks |
| --- | --- | --- | --- |
| Framework authoring | `specdev check` | `augentic/specify` contributors | Skill frontmatter, codex rule *shape*, links, marketplace consistency |
| Consumer standards | `specrun review` (planned, RM-10) | Downstream projects with `.specify/` | Applicable codex rules with `deterministic_hints`; emits structured findings |
| Build-time judgment | Target `build/review.md` briefs | Active slice during `/spec:build` | Model-assisted application of codex policy → human `REVIEW.md` |

`specrun review` is **not** a workflow phase. It is CI-native **standards enforcement**: findings may block a pipeline (exit code `2`) but never call `specrun slice transition` or write lifecycle fields. Phase skills already use deterministic CLI gates where lifecycle depends on them (`specrun slice validate` during refine; verify-repair during build).

Plan **Gate 1** (`specrun plan transition <name> reviewed`) is operator approval of a *plan*, not engineering-standards enforcement — reserve the word "reviewed" for that gate when discussing lifecycle.

## Related reading

- [Core concepts](concepts.md) — change rhythm and slice loop
- [Artifacts in depth](artifacts.md) — artifact responsibilities
- [Consistency checks](../contributing/checks.md) — `specdev check` vs future `specrun review`
- [Shared UNI-* codex inventory](../../adapters/shared/codex/universal/README.md)
