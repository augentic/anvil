# The Effect-Oriented Harness (Architecture North Star)

> Status: Standing architecture (north star) — durable direction, not a change RFC; it does not land or get deleted · Implemented by: RFC-53 (effect interfaces), RFC-54 (orchestration components), RFC-55 (workflow as effects) · Foundation: RFC-51 (typed adapter contract), Stages 0–1 · Preserves: RFC-47 (identity), RFC-48 (packaging), RFC-50 (adapter-agnostic core) · Sibling to [roadmap.md](roadmap.md)

## Abstract

This document names the destination the recent adapter RFCs have been bending toward, so future work converges on one shape instead of fumbling toward it a decision at a time. The thesis is a single mental model:

> **Specify is a typed effect system. The workflow is a program. The LLM is an effect. Adapters are composite components — deterministic exports plus prose `infer`-bodies. The harness is the interpreter that handles the effects.**

Everything else in this document — the four invariants, the target harness, the per-layer line, the staged roadmap — is a consequence of taking that model seriously. RFC-51's typed records, "wasm as the adapter surface," the tool-vs-agent split, and lazy reference discovery are not four separate features; they are four facets of this one architecture.

This is **not a new direction.** It is the completion of a bifurcation already present in the codebase: the deterministic skeleton (plan lifecycle, lock enforcement, `specify plan next` refusing illegal transitions, validation, merge) already lives in the CLI; the LLM judgment (survey, extract, synthesis, review) already lives in briefs; the skills are glue in between. This document names that endpoint and makes the seam typed.

## Why this document exists (and why it is not an RFC)

Specify has accumulated point decisions — RFC-47 identity, RFC-48 packaging, RFC-50 adapter-agnostic core, RFC-51 typed contract — that each independently push toward the same structure without ever stating it. The cost of leaving the destination implicit is that every new decision is re-litigated from first principles, and the answers drift. This document fixes the north star and, more importantly, hands down a **decision rule** (below) that resolves the recurring questions ("prose or code?", "callable export or handoff?", "what does this function take?") the same way every time.

It is deliberately **not** a numbered RFC. In this repo an RFC is a discrete, landable change proposal that is deleted once merged (RFC-47–50 already have been). This is the opposite: a durable, framing document that must persist and that *sequences* the change RFCs rather than being one. It therefore lives alongside [roadmap.md](roadmap.md) as a standing strategic doc, and the change RFCs (RFC-53/54/55) cite it by name.

## The model

![The effect model](../docs/assets/diagrams/effect-architecture/effect-model.svg)

An **orchestration program** (a wasm component) expresses deterministic control flow and, wherever it needs something it cannot compute, **requests a typed effect** from the **interpreter** (the harness). The interpreter satisfies a small, fixed vocabulary of effects and returns typed results that steer the program's next step. The marquee effect is `infer`: run a brief on the LLM and return its output. Prose is the *body* of the `infer` effect; the LLM is its *interpreter*; the brief's typed request and report are the effect's *signature*.

The deep move is directional. Today a child process (the CLI) cannot reach up into its parent agent's context, so agent work is handed *back up* via prepare/finalize. In the effect model the program calls *down* into wasm and the wasm calls *up* into the interpreter through an import — so the LLM step runs in the interpreter's context, where the agent already lives.

## The four invariants

These hold at **every** stage. A proposed change that violates one is wrong — this is the test that was missing.

1. **One typed contract is the currency of every boundary.** WIT records for data (RFC-51 stratum 1); WIT interfaces for effects. Nothing crosses a boundary untyped. This is the foundation, and it is already in flight under RFC-51.
2. **The host knows effects, not adapters and not brains.** The host stays adapter-agnostic (RFC-50) and gains a fixed, small effect vocabulary (`infer`, `read-artifact` / `get-asset`, `load-reference`, `journal` / `transition`). The "brain" becomes a *pluggable `infer` implementation*, not an assumption baked into prose. This *strengthens* RFC-50's agnosticism rather than weakening it.
3. **Determinism by default; judgment by exception — and judgment returns typed decisions that steer the deterministic skeleton.** The state machine (loops, gates, sequencing) is code. When a branch needs judgment, the code calls `infer` and gets back a *typed* value (`retry | abort | escalate`, a `build-report`, a reconciliation) that drives the next deterministic step. Control flow is never encoded in prose; the LLM never guesses at control flow.
4. **Laziness is law: handles cross boundaries, never corpora.** Every effect carries references (`brief-ref`, artifact paths, reference ids); the executor pulls bodies on demand. This is RFC-51 §D/§G discipline promoted to a universal rule — it is what keeps the agent's context budget intact.

## What the harness becomes

![Today to north star](../docs/assets/diagrams/effect-architecture/evolution.svg)

The harness becomes a thin **effect interpreter** that instantiates orchestration components and satisfies a small, fixed set of imports:

| Effect (WIT import) | What it does | Who satisfies it |
| --- | --- | --- |
| `infer(brief-ref, request) -> output` | Run prose with the LLM, in context | Pluggable: Cursor session / headless API / **CI replay stub** |
| `read-artifact` / `get-asset` | Narrow, pull-based host/project data | Host (RFC-51 `host-data`) |
| `load-reference(id)` | Lazy adapter-bundle prose, on demand | Host, reading the adapter bundle |
| `journal` / `transition` | Lifecycle events + legal transitions | The CLI's existing lifecycle owner |

Three deployment modes then fall out of one architecture **for free**:

- **Interactive** — Cursor satisfies `infer` with the live session.
- **Headless** — a CLI/API `infer` backend.
- **CI** — a record/replay `infer` stub, so an entire workflow run is deterministic and gradable (RFC-51's Phase-7 conformance idea generalized to the whole system).

"Agent-runtime-agnostic" stops being a fragile property defended by grep tests and becomes a **type-enforced interface**. Skills thin to entry points; the orchestration that matters is typed.

## Where the deterministic / agent line sits — per layer

The single most important guard against over-engineering: the deterministic-vs-agent line sits in a **different place per layer**, and collapsing every layer into wasm is the main way this vision goes wrong.

- **Adapters lean toward orchestration components.** Their build / extract / merge flows are deep, repeatable, and benefit most from typed multi-step control flow + lazy refs + replay. This is where the `infer`-as-effect model earns its cost, and where it should be proven first (RFC-54).
- **The workflow leans toward an agent-driver over deterministic CLI guardrails.** The lifecycle skeleton is *already* deterministic in the CLI (lock, gates, `plan next`). Workflow-level recovery and operator intent are exactly where the LLM's adaptability is a feature, not a bug. The workflow layer therefore adopts the *effect interfaces* (typed `infer`, typed data, record/replay) but need not collapse into a monolithic orchestration component (RFC-55, deferred).

Both layers speak the same effects; they simply draw the structure / judgment line differently.

## The staged path

![Staged path to the effect-oriented harness](../docs/assets/diagrams/effect-architecture/roadmap.svg)

Each stage is independently mergeable, independently valuable, and forward-compatible on the same typed contract. None requires the next.

| Stage | Ships | RFC | Independently valuable? |
| --- | --- | --- | --- |
| **S0 — Typed records** | Stratum-1 WIT records; retire the `*_JSON_SCHEMA` drift surface | RFC-51 | Yes — kills schema drift |
| **S1 — Typed exports** | `execution: tool` ops callable through generated bindings | RFC-51 | Yes — typed tool dispatch |
| **S2 — Name the effects** | `infer` / data / refs / lifecycle as typed WIT imports, *initially backed by today's handoff + CLI*; unlock record/replay | RFC-53 | Yes — pivot; makes the implicit boundary explicit and testable |
| **S3 — Orchestration components** | Adapters orchestrate their own multi-step ops (Realization B → A); the `infer` effect calls the brief | RFC-54 | Yes — where the vision first becomes visible |
| **S4 — Workflow as effects** | The workflow phases run as effect-driven orchestration; harness becomes a thin interpreter | RFC-55 | Optional — **deferred**; the system is coherent without it |

**S0–S2 are near-pure wins** on the contract already being built. **S3** is the first stage that demands the async-ABI judgment call and is the proving ground. **S4** is a genuine bet that can be deferred for years. There is a **coherent stopping point after S3**: adapters orchestrate over typed effects, the workflow stays agent-driven over deterministic guardrails, and nothing is half-built.

## Where RFC-51 sits (kept, reframed — not disposed)

RFC-51 **fits and stays.** It is Stages 0–1: the typed contract and typed tool dispatch that everything rides on. Two of its sections are *recontextualized* by this north star rather than contradicted:

- **§D (capabilities / resources)** is the first **data effect**. RFC-53 generalizes it from "capability grant" to "named effect," so RFC-51 §D is the seed of the effect vocabulary, not a dead end.
- **§F/§G (typed brief contract, lazy discovery)** survive in spirit but change owner. Once a brief is the *body of the `infer` effect*, the "brief binds a WIT signature" claim becomes "the `infer` call-site declares the signature." The heavy `implements` / `consumes` / `produces` / `capabilities` frontmatter machinery (RFC-51 §F1–F4, Phases 4–7) should therefore be treated as **provisional** and re-evaluated under RFC-54 before it is fully built out — some of it is subsumed by the typed `infer` boundary. Lazy discovery (§G) is *promoted* to invariant 4.

No part of RFC-51 is disposed. The only editorial action is a positioning note pointing up to this document (added in the same change).

## The bets (commit with eyes open)

- **The async / effect ABI.** Streaming, concurrency, and cancellation for `infer` want the Component Model async path, whose ergonomics on the pinned wasmtime are unconfirmed (RFC-51 already flags this). S0–S2 do not need it; S3+ increasingly do. *De-risk by confirming it before S3, not before S0.*
- **The LLM-required host.** The effect model means a host must satisfy `infer` to run a judgment step — "zero-LLM-config execution" is given up. Mitigation: the replay stub *is* the zero-config path, and it is strictly better (deterministic). Accept this trade explicitly; it is the price of the whole vision.
- **How far to push S4.** A real fork, decided with data from S3 — not now.

## The decision rule

Stop evaluating each change as an isolated point decision. Evaluate every future change against one sentence:

> **Push structure to code, judgment to the `infer` effect, and never let a corpus cross a boundary.**

When a design question arises — prose or code? a callable export or a handoff? what does this function take as an argument? — that rule answers it, and the four invariants tell you whether you have drifted. This is what converts "iterative movement" into "deliberate convergence."

## Relationship to prior invariants

- **RFC-50 (adapter-agnostic core)** — preserved and strengthened: the host still holds zero adapter names/taxonomy, and now its LLM coupling is an explicit, swappable interface rather than an implicit assumption.
- **RFC-47 / RFC-48 (identity / packaging)** — unchanged: adapters remain composite extensions (wasm + prose) published to the registry; this architecture only changes what the wasm half *does* and how the prose half is *invoked*.
- **Lazy discovery (RFC-51 §G)** — promoted from an adapter-local concern to invariant 4, binding on every layer.

## Acceptance criteria (architecture-level)

1. **One contract.** Exactly one typed WIT contract is the currency for every boundary; no hand-rolled DTO or embedded JSON-Schema constant duplicates a shape.
2. **Effect-shaped host.** The host's adapter/runtime coupling is expressed as a small, fixed set of typed effect interfaces; it carries no adapter names and no LLM dependency it cannot replace through the `infer` interface.
3. **Handles, not corpora.** Every effect carries references/handles; no brief body or artifact content crosses a boundary as an inlined value.
4. **Replayable.** Mocking the `infer` effect makes a run deterministic end-to-end, so a workflow can be recorded and replayed in CI.
5. **Staged and green.** Each stage (S0–S4) is independently mergeable and keeps `make lint` and `cargo make ci` green; S0–S3 form a coherent system without S4.
6. **Per-layer line respected.** Adapters orchestrate over effects; the workflow stays agent-driven over deterministic CLI guardrails; neither is forced into the other's shape.
