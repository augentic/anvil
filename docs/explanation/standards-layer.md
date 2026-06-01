# Workflow, standards, and artifacts

Specify separates three concerns that often collapse in agentic delivery stacks. Keep them distinct when authoring skills, adapters, CI jobs, and operator docs.

## The triad

| Layer | Role | On disk / in CLI |
| --- | --- | --- |
| **Workflow** | Orchestrate phases and lifecycle transitions | `/spec:*` phase skills; `specrun plan`, `specrun slice`, `specrun workspace` |
| **Artifacts** | Capture slice-local and baseline product intent | `proposal.md`, `spec.md`, `design.md`, `tasks.md`, `plan.yaml`, baseline under `.specify/specs/` |
| **Engineering standards** | Durable policy that outlives any slice | Rules under `adapters/**/rules/`; `specrun rules export` and `specrun lint` |

Workflow **mutates** `.specify/` state through a closed set of CLI verbs. Artifacts **record** what a slice means to build and merge. Engineering standards **constrain** how work is done — they do not transition plans, slices, or changes.

See the [decision log](decision-log.md#workflow-standards-and-artifacts) for rationale.

## Authoring standards vs engineering standards

Two different uses of "standards" appear in this repository:

| Term | Meaning | Location |
| --- | --- | --- |
| **Authoring standards** | House style for skills, docs, and framework contributions | [`docs/standards/`](../standards/) — enforced by `specdev lint` (`make lint`) on `augentic/specify` |
| **Engineering standards** | Durable engineering policy for generated and hand-written code | Codex markdown under `adapters/shared/rules/`, `adapters/targets/<name>/rules/`, and optional source overlays — resolved by `specrun rules export` and enforced by `specrun lint` |

Do not conflate them. `docs/standards/skill-authoring.md` governs how to write a `SKILL.md`; `UNI-*` / `OMNIA-*` codex files govern what Omnia guest code must never do.

## How standards are enforced

Enforcement splits by audience, not by rule id:

| Surface | Binary | Audience | What it checks |
| --- | --- | --- | --- |
| Framework authoring | `specdev lint` | `augentic/specify` contributors | Skill frontmatter, rule *shape*, links, marketplace consistency |
| Consumer standards | `specrun lint` | Downstream projects with `.specify/` | Applicable rules with `deterministic_hints`; emits structured findings |
| Build-time judgment | Target `build/review.md` briefs | Active slice during `/spec:build` | Model-assisted application of codex policy → human `REVIEW.md` |

`specrun lint` is **not** a workflow phase. It is CI-native **standards enforcement**: findings may block a pipeline (exit code `2`) but never call `specrun slice transition` or write lifecycle fields. Phase skills already use deterministic CLI gates where lifecycle depends on them (`specrun slice validate` during refine; verify-repair during build).

Plan **Gate 1** (`specrun plan transition <name> approved`) is operator approval of a *plan*, not engineering-standards enforcement. Reserve **review** for build-time judgment (`REVIEW.md`, `build/review.md` briefs).

## Type-system enforcement of the lint boundary

"No lifecycle authority in lint" is a structural invariant of the `specify-cli` workspace, not a coding convention. The shared codex parser **and** the deterministic review surface (`specrun lint`, the WorkspaceModel indexer, hint interpreter, and diagnostic formatters) live in the `specify-standards` crate. `specify-standards` is a **sibling** of `specify-workflow`, not a child: neither crate imports the other, and the standards crate has no dependency on workflow types (slice, change, plan, journal). `specify-workflow` retains the workflow surface and gains nothing review-specific.

The split means lint code physically cannot construct or transition a slice, plan entry, or change — the symbols are not in scope at compile time. The only place both crates meet is the root `specify` binary, which wires them together to resolve project context for `specrun lint`. See the [decision log](decision-log.md#workflow-standards-and-artifacts) for the standards-vs-workflow split and implementation history.

## Related reading

- [Core concepts](concepts.md) — change rhythm and slice loop
- [Artifacts in depth](artifacts.md) — artifact responsibilities
- [Consistency checks](../contributing/checks.md) — `specdev lint` vs `specrun lint`
- [Shared UNI-* codex inventory](../../adapters/shared/rules/universal/README.md)
- [Ignore directives](../reference/ignore-directives.md) — in-source `specify-ignore` grammar, status taxonomy, and exit semantics
