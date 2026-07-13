# The Layered Stack

Specify is organised in three layers above the `specify` CLI substrate. Each layer is independently useful, and each builds on the one below it. The CLI is not itself a layer; it is the deterministic medium through which every layer enforces correctness.

<div class="pipeline">

![Three-layer stack](../assets/diagrams/layered-stack/three-layers.svg)

<p class="pipeline-caption">Layer 2 plans and drives a change; Layer 1 executes one slice; Layer 0 holds configuration and adapters.</p>
</div>

## Layer 0: Configuration and adapters

Layer 0 is the static project configuration plus the adapter manifests every higher layer reads. It declares **what** a project is — which target adapter receives its slices, which source adapters supply evidence, what schemas are in scope, what tools are available — without describing **how** any change is planned or executed.

The configuration surfaces:

- **`.specify/project.yaml`** — per-project manifest: `target:` (or `workspace: true` for a registry-only workspace), `specify-version`, `sources:` list of available adapters.
- **Adapter components** — each source / target adapter is a single WebAssembly component whose metadata is its own `metadata` export (no manifest file; operations derive from the WIT contract per axis).
- **JSON Schemas** — the authoritative `evidence.schema.json`, `discovery/lead.schema.json`, `discovery/proposal.schema.json`, and `plan.yaml` schemas are owned by and distributed with the `specify` binary, with sources in-tree under [`schemas/`](../../schemas).
- **`AGENTS.md` Specify-owned block** — generated guidance the framework owns inside an otherwise operator-owned file.

The CLI verbs that read or change Layer 0 state:

- **`specify init <target>`** / **`specify init --workspace`** — one-time scaffold of `.specify/`, writes `project.yaml`.
- **`specify source resolve <name>`** / **`specify target resolve <value>`** — load and validate an adapter manifest. The adapter loader (`crates/project/src/adapter/`) routes by axis.

Layer 0 settles before any change starts. Once `project.yaml` exists and the relevant adapters resolve, Layer 1 and Layer 2 can run.

## Layer 1: Executing one slice

Layer 1 is the per-slice `refine → build → merge` loop. It operates on **one slice** inside `.specify/slices/<name>/` and is the breakout surface every operator reaches when execute parks or when they want to drive a slice by hand.

<div class="pipeline">

![Layer 1 slice loop](../assets/diagrams/layered-stack/slice-loop.svg)

<p class="pipeline-caption">Breakouts /spec:refine, /spec:build, /spec:merge invoke the same guest orchestrations as specify plan execute.</p>
</div>

Each skill is an ultrathin invoke-and-relay wrapper: it elicits any missing arguments, invokes the matching `specify slice` verb, and relays the output. The guest orchestration underneath owns the sequencing, the judgment legs (extract, synthesis, the target's build prompts), and the validation.

The full set of Layer 1 skills:

| Skill          | Role                                                                                       |
| -------------- | ------------------------------------------------------------------------------------------ |
| `/spec:refine` | Wrap `specify slice refine`: extract per bound source, synthesis, validation, `refined`    |
| `/spec:build`  | Wrap `specify slice build`: the target adapter's build operation and the `built` gate      |
| `/spec:merge`  | Wrap `specify slice merge run`: baseline delta merge, archive; only writer of per-entry `done` |
| `/spec:drop`   | Wrap `specify slice drop`: discard a slice without merging                                 |

The matching CLI surface is the **`specify slice ...`** family: `slice refine`, `slice build`, `slice merge run`, `slice drop`, plus the read-only `slice list` and `slice validate`. The skills are one-to-one wrappers over it.

## Layer 2: Planning and driving a change

Layer 2 carries every change through one rhythm: plan, Gate 1, execute, finalize. There is no separate "single-slice mode" — N=1 uses the same rhythm as N=12, with `intent.survey` producing one lead.

| Skill            | Role                                                                                                |
| ---------------- | --------------------------------------------------------------------------------------------------- |
| `/spec:plan`     | Wrap `specify plan author`: survey each bound source, reconcile `slices[]`, validate; exit at `pending` |
| `specify plan execute` | Drive the plan through the Layer 1 loop (guest-routed verb, no skill wrapper); refuses unless plan is `approved` |
| `/spec:finalize` | Confirm operator-owned publication is complete, then archive the plan                                |

The plan is the change's table of contents. `/spec:plan` produces it by invoking `specify plan author`, which surveys each source, reconciles leads across sources, and halts at `plan.lifecycle: pending`. It prints the literal `specify plan transition <name> approved` command in its closing hint. The operator stamps Gate 1 explicitly — `/spec:plan` never writes `approved` itself.

`specify plan execute` consumes the approved plan by claiming the next eligible entry, running the Layer 1 loop, and updating per-entry status. After execution drains, the operator publishes the affected repositories through normal tooling; `/spec:finalize` then archives `plan.yaml`.

The matching CLI surface is **`specify plan {create, author, execute, add, amend, remove, transition, next, status, archive}`**. Multi-repo slots and topology remain plan inputs, while slot materialization and branch publication are operator-owned outside Specify.

### Gate 1: the operator review seam

The pause between `/spec:plan` and `specify plan execute` is the only review seam Specify ships. `/spec:plan` writes `pending`; the operator writes `approved`. `specify plan execute` refuses on anything other than `approved`. This gives operators a deliberate point to inspect `plan.yaml`, read `change.md`, and curate entries with `specify plan add`, `specify plan remove`, or `specify plan amend <entry>` (or re-run `specify plan author` to re-reconcile wholesale) before any per-slice work runs.

The framework does not ship a single "do everything" command. Teams that want one-command flow compose plan, transition, and execute in their own shell wrapper, accepting that the wrapper opts out of Gate 1. The seam is observable on disk (`plan.lifecycle == approved`) so automation can opt-in cleanly.

## The layers compose

A key design principle: higher layers invoke lower layers, but lower layers are unaware of what sits above them. `specify plan execute` runs the same refine, build, and merge orchestrations the breakout skills invoke one slice at a time. The phase orchestrations themselves do not know whether they are running inside the loop or being driven by a human.

This means you can always drop down a layer:

- If `/spec:plan` produces a plan you want to adjust, use **re-propose** or `specify plan add` / `specify plan remove` for grouping and deferral, and `specify plan amend <entry>` for divergence stamps, authority overrides, and single-source fixes — then stamp `approved` when ready.
- If `specify plan execute` parks on a slice, finish it manually with `/spec:build` and `/spec:merge`, then re-run `specify plan execute` to pick up the next entry.
- If publication is incomplete, finish the repository's normal branch and review workflow before running `/spec:finalize`.
- If a skill does something unexpected, inspect the underlying state by reading `plan.yaml` and `.specify/slices/<name>/metadata.yaml` directly — they are plain YAML files.
