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

- **`.emery/project.yaml`** — per-project manifest: `target:`, `emery-version`, `sources:` list of available adapters.
- **Adapter components** — each source / target adapter is a single WebAssembly component whose metadata is its own `metadata` export (no manifest file; operations derive from the WIT contract per axis).
- **Typed wire shapes** — the authoritative Evidence, lead, proposal, and `plan.yaml` shapes are the Rust serde types compiled into the `emery` binary (`artifacts::evidence`, `artifacts::discovery`, `project::plan`); the judgment-answer schemas the model host consumes are generated from those types by `project::answers` / `slice::answers`.
- **`AGENTS.md` Emery-owned block** — generated guidance the framework owns inside an otherwise operator-owned file.

The CLI verbs that read or change Layer 0 state:

- **`emery init <target>`** — one-time scaffold of `.emery/`, writes `project.yaml`.
- **`emery source resolve <name>`** / **`emery target resolve <value>`** — resolve an adapter and validate its metadata (there is no manifest file — metadata comes from the component's `metadata` export). The adapter loader (`crates/project/src/adapter/`) routes by axis.

Layer 0 settles before any change starts. Once `project.yaml` exists and the relevant adapters resolve, Layer 1 and Layer 2 can run.

## Layer 1: Executing one slice

Layer 1 is the per-slice `refine → build → merge` rhythm. It operates on **one slice** inside `.emery/slices/<name>/`: refinement runs inside the `emery plan refine` drain, and the build and merge phases run inside `emery plan execute` — there are no per-slice phase-breakout verbs or skills.

<div class="pipeline">

![Layer 1 slice loop](../assets/diagrams/layered-stack/slice-loop.svg)

<p class="pipeline-caption">Refinement runs as a guest orchestration inside emery plan refine; build and merge inside emery plan execute.</p>
</div>

The guest orchestrations own the sequencing, the judgment legs (extract, synthesis, the target's build prompts), and the validation:

| Phase    | Role                                                                              |
| -------- | --------------------------------------------------------------------------------- |
| `refine` | Extract per bound source, synthesis, validation, the `refinement.yaml` manifest write; transitions the slice `refined` (inside `emery plan refine`) |
| `build`  | The engine-driven build loop over the target's build/verify/repair/review operations and the `built` gate (inside `emery plan execute`) |
| `merge`  | Baseline delta merge, archive; only writer of per-entry `done` (inside `emery plan execute`) |

The matching CLI surface is read-only inspection: `slice list`, `slice validate` (including refinement-freshness and baseline-conflict advisories), `slice provenance`, and `slice model show`. Abandoning a slice is a plan act: `emery plan drop <entry>`.

## Layer 2: Planning and driving a change

Layer 2 carries every change through one rhythm: plan, review, refine, review, execute, finalize. There is no separate "single-slice mode" — N=1 uses the same rhythm as N=12, with `intent.survey` producing one lead.

| Skill            | Role                                                                                                |
| ---------------- | --------------------------------------------------------------------------------------------------- |
| `/emery:plan`     | Wrap `emery plan author`: survey each bound source, reconcile `slices[]`, validate; exit for review |
| `emery plan refine` | The specification drain: extract + synthesize every in-scope leaf and write its refinement manifest, no code work (guest-routed verb, wrapped by `/emery:refine`) |
| `emery plan execute` | The drive: opens `plan.execute.started` over the exact refinement digests and runs the build → merge loop under gap gates (guest-routed verb, wrapped by `/emery:execute`) |
| `/emery:finalize` | Confirm operator-owned publication is complete, then archive the plan                                |

The plan is the change's table of contents. `/emery:plan` produces it by invoking `emery plan author`, which surveys each source, reconciles leads across sources, and halts for operator review. It prints the literal `emery plan refine` command in its closing hint. The operator continues by running that verb — `/emery:plan` never runs it itself.

`emery plan refine` drains refinement over the closed plan and stops; `emery plan execute` advances the next eligible entry, runs the build → merge loop, and updates per-entry status. After execution drains, the operator publishes the affected repositories through normal tooling; `/emery:finalize` then archives `plan.yaml`.

The matching CLI surface is **`emery plan {author, refine, execute, add, amend, remove, drop, validate, status, gaps, archive}`**. Branch publication is operator-owned outside Emery.

### The operator review seams

The pause between `/emery:plan` and `emery plan refine` (topology review) and the pause between `emery plan refine` and `emery plan execute` (specification review) are the two review seams Emery ships — there is no separate approve verb and no approval file; invoking execute journals the authorization epoch over the exact refinement digests and enforces gap gates before build. The seams are opportunities for review, not attestations — an automation may run the stages back to back. [Core concepts](concepts.md) owns the full review story; [Amend a plan before executing](../how-to/amend-a-plan.md) covers the curation verbs available during the pauses.

## The layers compose

A key design principle: higher layers invoke lower layers, but lower layers are unaware of what sits above them. `emery plan refine` drives the refinement orchestration and `emery plan execute` the build and merge orchestrations one slice at a time; the phase orchestrations themselves do not know their position in the loop.

This means you can always drop down a layer:

- If `/emery:plan` produces a plan you want to adjust, use **re-propose** or `emery plan add` / `emery plan remove` for grouping and deferral, and `emery plan amend <entry>` for divergence stamps, authority overrides, and single-source fixes — then run `emery plan refine` when ready.
- If `emery plan refine` or `emery plan execute` parks on a slice, fix the input the stop card points at, then re-run the same command — refine skips fresh manifests; the execute loop resumes at the parked phase and continues to the next entry.
- If publication is incomplete, finish the repository's normal branch and review workflow before running `/emery:finalize`.
- If a skill does something unexpected, inspect the underlying state by reading `plan.yaml` and `.emery/slices/<name>/metadata.yaml` directly — they are plain YAML files.
