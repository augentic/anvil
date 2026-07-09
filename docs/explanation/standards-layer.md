# Workflow, standards, and artifacts

Specify separates three concerns that often collapse in agentic delivery stacks. Keep them distinct when authoring skills, adapters, CI jobs, and operator docs.

## The triad

| Layer                     | Role                                            | On disk / in CLI                                                                                 |
| ------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| **Workflow**              | Orchestrate phases and lifecycle transitions    | `/spec:*` phase skills; `specify plan`, `specify slice`, `specify workspace`                     |
| **Artifacts**             | Capture slice-local and baseline product intent | `proposal.md`, `spec.md`, `design.md`, `tasks.md`, `plan.yaml`, baseline under `.specify/specs/` |
| **Engineering standards** | Durable policy that outlives any slice          | Rules under `codex/rules/` and per-adapter `prose/rules/` overlays, embedded in each target adapter |

Workflow **mutates** `.specify/` state through a closed set of CLI verbs. Artifacts **record** what a slice means to build and merge. Engineering standards **constrain** how work is done — they do not transition plans, slices, or changes.


## Authoring standards vs engineering standards

Two different uses of "standards" appear in this repository:

| Term                      | Meaning                                                        | Location                                                                                                                                                                                                                                                                                       |
| ------------------------- | -------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Authoring standards**   | House style for skills, docs, and framework contributions      | [`docs/standards/`](../standards/) — enforced on `augentic/specify` by the framework-quality cargo tests ([`tests/framework/`](../../tests/framework/)), with policy as constants in each test module                                                                                          |
| **Engineering standards** | Durable engineering policy for generated and hand-written code | Cross-target `UNI-*` rules under `codex/rules/universal/` ([`augentic/specify-adapters`](https://github.com/augentic/specify-adapters)); per-adapter overlays under `targets/<name>/prose/rules/` and `sources/<name>/prose/rules/` — embedded in each adapter component and served by its references server |

Do not conflate them. `docs/standards/skill-authoring.md` governs how to write a `SKILL.md`; `UNI-*` / `OMNIA-*` codex files govern what Omnia guest code must never do.

## How standards are enforced

Enforcement splits by audience, not by rule id:

| Surface             | Binary                          | Audience                             | What it checks                                                     |
| ------------------- | ------------------------------- | ------------------------------------ | ------------------------------------------------------------------ |
| Framework authoring | `cargo test --test framework`   | `augentic/specify` contributors      | Skill frontmatter, links, marketplace consistency, docs prose      |
| Rule shape          | The adapters repo's `rule_shape` cargo test | `augentic/specify-adapters` contributors | Frontmatter fields, `## Rule` heading, id uniqueness, namespace ownership |
| Build-time judgment | Target `build/review.md` briefs | Active slice during `/spec:build`    | Model-assisted application of codex policy → human `REVIEW.md`     |

There is no lint verb on the CLI: the framework checks are cargo tests here, and there is no deterministic consumer-project scanner — if one earns its way in, it lands as developer tooling. The rules themselves reach consumer projects inside the target adapter components: each target embeds the universal pack plus its own overlays and its build review prompts apply them. Standards enforcement is **not** a workflow phase either way: findings may block a pipeline but never call `specify slice transition` or write lifecycle fields. Phase skills already use deterministic CLI gates where lifecycle depends on them (`specify slice validate` during refine; verify-repair during build).

Plan **Gate 1** (`specify plan transition <name> approved`) is operator approval of a *plan*, not engineering-standards enforcement. Reserve **review** for build-time judgment (`REVIEW.md`, `build/review.md` briefs).

## Structural enforcement of the standards boundary

"No lifecycle authority in review" is structural, not a coding convention. Engineering-standards rules live in `augentic/specify-adapters` and ship inside the adapter components; no engine crate parses or resolves them, so standards prose physically cannot construct or transition a slice, plan entry, or change — the code that owns those lifecycles never sees a rule.

## Related reading

- [Core concepts](concepts.md) — change rhythm and slice loop
- [Artifacts in depth](artifacts.md) — artifact responsibilities
- [Consistency checks](../contributing/checks.md) — the framework-repo authoring checks
- [Shared UNI-* codex inventory](https://github.com/augentic/specify-adapters/blob/main/codex/rules/universal/README.md)
