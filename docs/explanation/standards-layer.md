# Workflow, standards, and artifacts

Emery separates three concerns that often collapse in agentic delivery stacks. Keep them distinct when authoring adapters, CI jobs, operator docs, and the thin `/emery:*` skill wrappers.

## The triad

| Layer                     | Role                                            | On disk / in CLI                                                                                 |
| ------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| **Workflow**              | Orchestrate phases and lifecycle transitions    | `/emery:*` phase skills; `emery plan`, `emery slice`, `emery workspace`                     |
| **Artifacts**             | Capture slice-local and baseline product intent | `proposal.md`, `spec.md`, `design.md`, `tasks.md`, `plan.yaml`, baseline under `.emery/specs/` |
| **Engineering standards** | Durable policy that outlives any slice          | Rules under `codex/rules/` and per-adapter `prose/rules/` overlays, embedded in each target adapter |

Workflow **mutates** `.emery/` state through a closed set of CLI verbs. Artifacts **record** what a slice means to build and merge. Engineering standards **constrain** how work is done — they do not transition plans, slices, or changes.

This triad is orthogonal to [the layered stack](layered-stack.md): the stack cuts Emery by *invocation level* (configuration, one slice, a whole change), while the triad cuts it by *concern* — every layer of the stack touches workflow, artifacts, and standards.


## Authoring standards vs engineering standards

Two different uses of "standards" appear in this repository:

| Term                      | Meaning                                                        | Location                                                                                                                                                                                                                                                                                       |
| ------------------------- | -------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Authoring standards**   | House style for docs, skill wrappers, and framework contributions | [`docs/standards/`](../standards/doc-authoring.md) — Developer Guide links are enforced in CI by `mdbook-linkcheck2`; the rest (including skill-wrapper shape) is applied in review |
| **Engineering standards** | Durable engineering policy for generated and hand-written code | Cross-target `UNI-*` rules under `codex/rules/universal/` ([`augentic/emery-adapters`](https://github.com/augentic/emery-adapters)); per-adapter overlays under `targets/<name>/prose/rules/` and `sources/<name>/prose/rules/` — embedded in each adapter component and served by its references server |

Do not conflate them. Skill-wrapper body style is guidance in [`docs/standards/cli-contract.md`](../standards/cli-contract.md); `UNI-*` / `OMNIA-*` codex files govern what Omnia guest code must never do.

## How standards are enforced

Enforcement splits by audience, not by rule id:

| Surface             | Binary                          | Audience                             | What it checks                                                     |
| ------------------- | ------------------------------- | ------------------------------------ | ------------------------------------------------------------------ |
| Repo checks         | `cargo make links`                  | `augentic/emery` contributors      | Developer Guide link integrity (`mdbook build docs`) |
| Rule shape          | The adapters repo's `rule_shape` cargo test | `augentic/emery-adapters` contributors | Frontmatter fields, `## Rule` heading, id uniqueness, namespace ownership |
| Build-time judgment | Target `build/review.md` briefs | Active slice during `/emery:build`    | Model-assisted application of codex policy → human `REVIEW.md`     |

There is no lint verb on the CLI: repo consistency here is the mdBook links gate, and there is no deterministic consumer-project scanner — if one earns its way in, it lands as developer tooling. The rules themselves reach consumer projects inside the target adapter components: each target embeds the universal pack plus its own overlays and its build review prompts apply them. Standards enforcement is **not** a workflow phase either way: findings may block a pipeline but never transition a slice or write lifecycle fields. Phase skills already use deterministic CLI gates where lifecycle depends on them (`emery slice validate` during refine; verify-repair during build).

Plan **Gate 1** (the first `emery plan execute`) is operator approval of a *plan*, not engineering-standards enforcement. Reserve **review** for build-time judgment (`REVIEW.md`, `build/review.md` briefs).

## Structural enforcement of the standards boundary

"No lifecycle authority in review" is structural, not a coding convention. Engineering-standards rules live in `augentic/emery-adapters` and ship inside the adapter components; no engine crate parses or resolves them, so standards prose physically cannot construct or transition a slice, plan entry, or change — the code that owns those lifecycles never sees a rule.

## Related reading

- [Core concepts](concepts.md) — change rhythm and slice loop
- [Artifacts in depth](artifacts.md) — artifact responsibilities
- [Quality gates](../contributing/quality-gates.md#consistency-links) — Developer Guide link integrity
- [Shared UNI-* codex inventory](https://github.com/augentic/emery-adapters/blob/main/codex/rules/universal/README.md)
