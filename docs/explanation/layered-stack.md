# The Layered Stack

Emery is organised in three layers above the `emery` CLI substrate. Each layer is independently useful, and each builds on the one below it. The CLI is not itself a layer; it is the deterministic medium through which every layer enforces correctness.

This stack is orthogonal to the [workflow / artifacts / engineering-standards triad](standards-layer.md): the stack cuts Emery by *invocation level*, the triad by *concern* — do not map layers onto triad rows.

<div class="pipeline">

![Three-layer stack](../assets/diagrams/layered-stack/three-layers.svg)

<p class="pipeline-caption">Layer 2 plans and drives a change; Layer 1 executes one slice; Layer 0 holds configuration and adapters.</p>
</div>

## Layer 0: Configuration and adapters

Layer 0 is the static project configuration plus the adapter components every higher layer resolves. It declares **what** a project is — which target adapter receives its slices, which source adapters supply evidence, what schemas are in scope, what tools are available — without describing **how** any change is planned or executed.

The configuration surfaces:

- **`.emery/project.yaml`** — per-project manifest: `target:` (or `workspace: true` for a registry-only workspace), `emery-version`, `sources:` list of available adapters.
- **Adapter components** — each source / target adapter is a single WebAssembly component whose metadata is its own `metadata` export (no manifest file; operations derive from the WIT contract per axis).
- **Typed wire shapes** — the authoritative Evidence, lead, proposal, and `plan.yaml` shapes are the Rust serde types compiled into the `emery` binary (`artifacts::evidence`, `artifacts::discovery`, `project::plan`); the judgment-answer schemas the model host consumes are generated from those types by `project::answers` / `slice::answers`.
- **`AGENTS.md` Emery-owned block** — generated guidance the framework owns inside an otherwise operator-owned file.

The CLI verbs that read or change Layer 0 state:

- **`emery init <target>`** / **`emery init --workspace`** — one-time scaffold of `.emery/`, writes `project.yaml`.
- **`emery source resolve <name>`** / **`emery target resolve <value>`** — resolve an adapter and validate its metadata (there is no manifest file — metadata comes from the component's `metadata` export). The adapter loader (`crates/project/src/adapter/`) routes by axis.

Layer 0 settles before any change starts. Once `project.yaml` exists and the relevant adapters resolve, Layer 1 and Layer 2 can run.

## Layer 1: Executing one slice

Layer 1 is the per-slice `refine → build → merge` loop. It operates on **one slice** inside `.emery/slices/<name>/` and is the breakout surface every operator reaches when execute parks or when they want to drive a slice by hand.

<div class="pipeline">

![Layer 1 slice loop](../assets/diagrams/layered-stack/slice-loop.svg)

<p class="pipeline-caption">Breakouts /emery:refine, /emery:build, /emery:merge invoke the same guest orchestrations as emery plan execute.</p>
</div>

Each skill is an ultrathin invoke-and-relay wrapper: it elicits any missing arguments, invokes the matching `emery slice` verb, and relays the output. The guest orchestration underneath owns the sequencing, the judgment legs (extract, synthesis, the target's build prompts), and the validation.

The full set of Layer 1 skills:

| Skill          | Role                                                                                       |
| -------------- | ------------------------------------------------------------------------------------------ |
| `/emery:refine` | Wrap `emery slice refine`: extract per bound source, synthesis, validation, `refined`    |
| `/emery:build`  | Wrap `emery slice build`: the target adapter's build operation and the `built` gate      |
| `/emery:merge`  | Wrap `emery slice merge`: baseline delta merge, archive; only writer of per-entry `done` |
| `/emery:drop`   | Wrap `emery slice drop`: discard a slice without merging                                 |

The matching CLI surface is the **`emery slice ...`** family: `slice refine`, `slice build`, `slice merge`, `slice drop`, plus the read-only `slice list` and `slice validate`. The skills are one-to-one wrappers over it.

## Layer 2: Planning and driving a change

Layer 2 carries every change through one rhythm: plan, Gate 1, execute, finalize. There is no separate "single-slice mode" — N=1 uses the same rhythm as N=12, with `intent.survey` producing one lead.

| Skill            | Role                                                                                                |
| ---------------- | --------------------------------------------------------------------------------------------------- |
| `/emery:plan`     | Wrap `emery plan author`: survey each bound source, reconcile `slices[]`, validate; exit at `pending` |
| `emery plan execute` | Gate 1 plus the drive: the first run on a `pending` plan stamps `approved`, then runs the Layer 1 loop (guest-routed verb, wrapped by `/emery:execute`) |
| `/emery:finalize` | Confirm operator-owned publication is complete, then archive the plan                                |

The plan is the change's table of contents. `/emery:plan` produces it by invoking `emery plan author`, which surveys each source, reconciles leads across sources, and halts at `plan.lifecycle: pending`. It prints the literal `emery plan execute` command in its closing hint. The operator stamps Gate 1 by running that verb — `/emery:plan` never runs it itself.

`emery plan execute` claims the next eligible entry, runs the Layer 1 loop, and updates per-entry status. After execution drains, the operator publishes the affected repositories through normal tooling; `/emery:finalize` then archives `plan.yaml`.

The matching CLI surface is **`emery plan {author, execute, add, amend, remove, undo, next, status, archive}`**. Multi-repo slots and topology remain plan inputs, while slot materialization and branch publication are operator-owned outside Emery.

### Gate 1: the operator review seam

The pause between `/emery:plan` and `emery plan execute` is the only review seam Emery ships — there is no separate approve verb; invoking execute is the approval act, observable on disk as `plan.lifecycle: approved`. [Core concepts](concepts.md) owns the full Gate 1 story; [Amend a plan at Gate 1](../how-to/amend-plan-at-gate-1.md) covers the curation verbs available during the pause.

## The layers compose

A key design principle: higher layers invoke lower layers, but lower layers are unaware of what sits above them. `emery plan execute` runs the same refine, build, and merge orchestrations the breakout skills invoke one slice at a time. The phase orchestrations themselves do not know whether they are running inside the loop or being driven by a human.

This means you can always drop down a layer:

- If `/emery:plan` produces a plan you want to adjust, use **re-propose** or `emery plan add` / `emery plan remove` for grouping and deferral, and `emery plan amend <entry>` for divergence stamps, authority overrides, and single-source fixes — then run `emery plan execute` when ready.
- If `emery plan execute` parks on a slice, finish it manually with `/emery:build` and `/emery:merge`, then re-run `emery plan execute` to pick up the next entry.
- If publication is incomplete, finish the repository's normal branch and review workflow before running `/emery:finalize`.
- If a skill does something unexpected, inspect the underlying state by reading `plan.yaml` and `.emery/slices/<name>/metadata.yaml` directly — they are plain YAML files.
