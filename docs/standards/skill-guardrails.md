# Shared guardrails

Cross-cutting "do not / never / always" rules no longer live in a standalone prose file — each rule now ships with the surface that enforces or delivers it:

- **Single-writer for lifecycle state.** The CLI is the only writer for change and slice lifecycle state: plan entries go through `specify plan add` / `amend`, lifecycle stamps through `specify plan transition`, slice transitions through the guest orchestrations behind `specify slice refine` / `build` / `merge` / `drop`, and archive moves through `specify slice merge`, `specify slice drop`, and `specify plan archive`. This is enforced by the CLI (there are no hand-edit-equivalent verbs) and restated for consumer-project agents in the `## Boundaries` section of the `AGENTS.md` that `specify init` generates (`crates/project/src/agents/render.rs`).
- **Consumer tooling boundary.** During execute / build / merge, agents consume Specify and adapters — they do not maintain them. Delivered by the generated `AGENTS.md` boundaries fence and by the adapter build prompts via the spec-runtime bundle (`codex/references/runtime/guardrails.md` in [`augentic/specify-adapters`](https://github.com/augentic/specify-adapters), embedded in every adapter component).
- **Baseline immutability for contract authoring.** Contract authoring writes only inside the active slice directory; merging into the baseline is `specify slice merge run`'s job. Owned by the contracts target adapter's prose and the same spec-runtime bundle.

Each thin skill body keeps its own one-line guardrail inline (a single `## Guardrails` H2 per skill — see [skill authoring](./skill-authoring.md#cross-cutting-guardrails)); there is no shared runtime reference file for skills to link.
