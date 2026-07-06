# Workflow, standards, and artifacts

Specify separates three concerns that often collapse in agentic delivery stacks. Keep them distinct when authoring skills, adapters, CI jobs, and operator docs.

## The triad

| Layer | Role | On disk / in CLI |
| --- | --- | --- |
| **Workflow** | Orchestrate phases and lifecycle transitions | `/spec:*` phase skills; `specify plan`, `specify slice`, `specify workspace` |
| **Artifacts** | Capture slice-local and baseline product intent | `proposal.md`, `spec.md`, `design.md`, `tasks.md`, `plan.yaml`, baseline under `.specify/specs/` |
| **Engineering standards** | Durable policy that outlives any slice | Rules under `adapters/**/rules/`; `specify rules export` and `specify lint project` |

Workflow **mutates** `.specify/` state through a closed set of CLI verbs. Artifacts **record** what a slice means to build and merge. Engineering standards **constrain** how work is done — they do not transition plans, slices, or changes.


## Authoring standards vs engineering standards

Two different uses of "standards" appear in this repository:

| Term | Meaning | Location |
| --- | --- | --- |
| **Authoring standards** | House style for skills, docs, and framework contributions | [`docs/standards/`](../standards/) — enforced by `specify lint framework` (`make lint`) on `augentic/specify`; framework invariants also ship as [`CORE-*` rules](../../adapters/shared/rules/core/) resolved by a generic dispatcher — each rule is either a declarative hint (Road A) or a name-resolved in-process checker (Road B), with all policy in the rule's `config:`) |
| **Engineering standards** | Durable engineering policy for generated and hand-written code | Codex markdown under `adapters/shared/rules/`, `adapters/targets/<name>/prose/rules/`, and optional source overlays — resolved by `specify rules export` and enforced by `specify lint project` |

Do not conflate them. `docs/standards/skill-authoring.md` governs how to write a `SKILL.md`; `UNI-*` / `OMNIA-*` codex files govern what Omnia guest code must never do.

## How standards are enforced

Enforcement splits by audience, not by rule id:

| Surface | Binary | Audience | What it checks |
| --- | --- | --- | --- |
| Framework authoring | `specify lint framework` | `augentic/specify` contributors | Skill frontmatter, rule *shape*, links, marketplace consistency |
| Consumer standards | `specify lint project` | Downstream projects with `.specify/` | Applicable rules with `rule_hints`; emits structured findings |
| Build-time judgment | Target `build/review.md` briefs | Active slice during `/spec:build` | Model-assisted application of codex policy → human `REVIEW.md` |

`specify lint project` is **not** a workflow phase. It is CI-native **standards enforcement**: findings may block a pipeline (exit code `2`) but never call `specify slice transition` or write lifecycle fields. Phase skills already use deterministic CLI gates where lifecycle depends on them (`specify slice validate` during refine; verify-repair during build).

Plan **Gate 1** (`specify plan transition <name> approved`) is operator approval of a *plan*, not engineering-standards enforcement. Reserve **review** for build-time judgment (`REVIEW.md`, `build/review.md` briefs).

## Type-system enforcement of the lint boundary

"No lifecycle authority in lint" is a structural invariant of the `cli/` workspace, not a coding convention. The shared codex parser **and** the deterministic review surface (`specify lint project`, the WorkspaceModel indexer, hint interpreter, and diagnostic formatters) live in the `specify-standards` crate. `specify-standards` is a **sibling** of `specify-workflow`, not a child: neither crate imports the other, and the standards crate has no dependency on workflow types (slice, change, plan, journal). `specify-workflow` retains the workflow surface and gains nothing review-specific.

The split means lint code physically cannot construct or transition a slice, plan entry, or change — the symbols are not in scope at compile time. The only place both crates meet is the root `specify` binary, which wires them together to resolve project context for `specify lint project`.

## Related reading

- [Core concepts](concepts.md) — change rhythm and slice loop
- [Artifacts in depth](artifacts.md) — artifact responsibilities
- [Consistency checks](../contributing/checks.md) — `specify lint framework` vs `specify lint project`
- [Shared UNI-* codex inventory](../../adapters/shared/rules/universal/README.md)
- [Ignore directives](../reference/ignore-directives.md) — in-source `specify-ignore` grammar, status taxonomy, and exit semantics
