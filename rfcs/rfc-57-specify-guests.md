# RFC-57: Workflow and Development as Guests

> Status: Draft · Order 7 of 8 · Stage S4 · Depends: [RFC-54](rfc-54-orchestration.md), [RFC-56](rfc-56-runtime-move.md) · Owns: the workflow and development guests

## Abstract

With the runtime under everything ([RFC-56](rfc-56-runtime-move.md)), the two remaining first-party concerns move onto it as guests: the **workflow** (`/spec:plan`, `/spec:execute`, the slice loop) and the framework's own **development tooling** (authoring and standards checks). The workflow guest sequences deterministic steps in Wasm, reaches adapters by host-mediated dynamic linking, and calls `eval` for judgment — so "everything is a guest" becomes literally true.

## The model

- **The workflow as a guest.** `/spec:plan` sequences the runtime's deterministic operations (`plan add`, validation, Gate 1) and calls `eval` for the judgment legs — a survey pass per bound source, then a reconcile-leads pass. `/spec:execute` is the drained-loop reducer over plan entries. The guest *requests* lifecycle transitions; it does not own them.
- **Reaching adapters.** When the workflow needs an adapter operation (`survey` / `extract` / `build` / `merge`), it calls the host `selection` interface with the plan-bound `adapter-id`, and the host routes it to a fresh adapter instance ([RFC-56](rfc-56-runtime-move.md)). Because identity is a call argument, the workflow guest **owns its own fan-out** — it loops `survey(id)` over every bound source in guest code (Law 3: control flow lives in deterministic guest code), rather than being re-instantiated per source by native orchestration. When it needs judgment, it calls `eval`.
- **Development tooling as a guest.** `specify lint framework`, `rules export`, and the `CORE-*` checkers are Specify behaviour, so they ride the same runtime as a guest. They are deterministic, carry no judgment leg, and are sequenced last.

## Lifecycle authority stays in the runtime

`transition` / `journal` / lock ownership stay in the runtime's deterministic lifecycle host service ([RFC-52](rfc-52-effect.md)). The workflow guest requests transitions as effects; it never writes them. Adaptive, recovery-heavy phases stay model-driven through `eval` rather than compiled into rigid control flow — deterministic sequencing graduates to guest code; judgment does not.

## Scope

- Running the workflow as a guest on the runtime, with the slice loop expressed over deterministic effects and `eval`.
- The boundary between runtime-owned transitions and guest-owned sequencing.
- Moving the development / standards tooling onto the runtime as a guest.

## Open questions

- Which phases express as deterministic guest sequencing vs stay model-driven through `eval` (decided per-phase, by where the value lies).
- What survives of the skill markdown once the workflow guest owns orchestration.
- Which development-tooling operations are worth moving off the CLI.

## Acceptance criteria

1. The workflow runs as a guest on the runtime; the bespoke driver is gone.
2. Every phase is reachable, with lifecycle authority still in the runtime's host service.
3. The workflow reaches adapters by host-mediated dynamic linking and judgment by `eval`.
4. Adaptive phases stay model-driven; only deterministic-sequencing phases are compiled into the guest.
5. `make lint` and `cargo make ci` stay green.

## Risks and invariants

- **Don't ossify the fluid.** Encoding adaptive, recovery-heavy orchestration as rigid control flow is the chief risk; a phase that needs the model's judgment to sequence stays on `eval`.
- **Lifecycle authority.** Authority stays in the runtime's lifecycle host service, never in a guest or skill.
