# Specify on Omnia — The Effect-Oriented Architecture

> Status: This is our standing architecture — the agreed-upon direction we are working toward. It helps sequence our work rather than landing as a single massive change, and it lives alongside [roadmap.md](roadmap.md).

## The core idea

The new Specify architecture boils down to one core concept:

> Specify is a family of Wasm components running on the [Omnia](https://github.com/augentic/omnia) runtime. The workflow and every adapter are guests. Judgment is treated as an effect — `judge` — that the runtime delegates to a pluggable fleet of models. The "Specify" CLI is simply Omnia compiled with Specify-specific backends.

Everything that follows — the system's shape, core principles, how operations flow, and deployment modes — is a consequence of this idea. The runtime doesn't need to understand our domain: it just knows how to run Wasm and how to handle a small, fixed set of effects. Specify's behaviour — orchestrating workflows, extracting from sources, building for targets, and development tooling — lives entirely within the guests.

This formalises a split that was already happening in the codebase. Previously, the deterministic parts (plan lifecycle, validation, merging) lived in a bespoke `specify` binary, while judgment (surveying, extracting, synthesising) lived in text briefs run by a model. This new architecture moves both onto a single generic runtime and ensures the boundary between structure and judgment is strictly typed.

There's a second important lesson we've learned: context should come from artifacts, not from conversational history. Every `judge` call is self-contained. It points to a brief and makes a typed request against concrete artifacts (like `spec.md` or a build request), rather than relying on an accumulated chat transcript. This is a deliberate choice to avoid the bloated, overloaded context windows that often cause failures. 

This approach gives us two major benefits:
1. **Scalability and auditability**: An operation runs exactly the same way whether you trigger it from your editor or it runs in a massive CI pipeline.
2. **Cost efficiency**: Because judgment is a typed call over specific inputs, we can route easy tasks to deterministic code and narrow tasks to small local models, reserving expensive frontier LLMs only for the hardest problems.

## The shape of the system

![The shape of the system](../docs/assets/diagrams/effect-architecture/system-shape.svg)

The system is split into two roles communicating over a single contract: a generic runtime and the guests that run on it.

- **Omnia is the foundation.** It's a command-line executable. Its main job is to run a guest (e.g., `omnia workflow.wasm plan …`) and pass along any arguments. It instantiates the guest and handles a small vocabulary of effects, but it doesn't know anything about adapters, workflows, or AI models.
- **Everything else is a guest.** The workflow (`plan`, `execute`) and our development tools are first-party guests. The adapters are guests too, whether they are source guests (like `typescript` or `documentation`) or target guests (like `omnia` or `vectis`). They all run as peers on the runtime.
- **The model fleet sits below.** When a guest needs to make a judgment call, it requests the `judge` effect. Omnia then dispatches this request to a pluggable backend — which could be a frontier LLM, a small local model, or even a deterministic replay stub for testing. The "brain" is just a swappable implementation detail.
- **The boundary is strictly typed.** Every piece of data and every effect crosses the boundary with a strict type. We don't pass raw, untyped text corpora across this line — only typed data and handles.

The runtime and guests interact in both directions. Omnia instantiates a guest and calls its exported functions (like `build` or `extract`). In turn, the guest calls back into Omnia's host services (like `judge`, `load-reference`, or `journal`) whenever it needs to do something impure. 

> **A quick note on naming:** In this document, "Omnia" refers to two things: the runtime itself, and the `omnia` target guest (the adapter that generates code for the Omnia runtime). We'll try to be explicit when the context isn't obvious.

## The runtime: Omnia

Omnia is built on Wasmtime. Its design centers around providing pluggable host services (like HTTP, key-value storage, or observability) behind typed interfaces, so you can swap implementations without changing the guest code. Specify's effects are just another set of host services, with the `judge` model service being the most notable addition ([RFC-57](rfc-57-omnia-model-host.md)).

Three key properties of the runtime make this architecture possible:

- **One binary, guest-selected behaviour.** We no longer have a bespoke `specify` host. Instead, you run `omnia <guest>.wasm <args…>`, and the guest decides what to do with the invocation. 
- **Instance-per-call execution.** Every time we call a guest, we spin up a fresh instance. Wasm component instances aren't re-entrant, so this avoids a whole class of async complexity. If a judgment step needs to resolve a reference by calling back into a guest, it happens in a brand new instance.
- **Stateless guests, host-held state.** Because we use fresh instances per call, guests can't hold onto state in memory. Anything that needs to persist lives in a host service, like a key-value store. This makes the runtime incredibly easy to scale horisontally.

## The mental model: programs, effects, and interpreters

![The effect model](../docs/assets/diagrams/effect-architecture/effect-model.svg)

This design borrows heavily from the concept of algebraic effects. The guest program handles pure control flow, and whenever it needs to interact with the messy, non-deterministic outside world, it requests a named, typed effect. A separate interpreter (Omnia) decides how to actually perform that effect. 

Here's the vocabulary we use:

- **Effect**: A typed request from a guest for something it can't do itself (like running a brief, reading a file, or logging an event). The effect says *what* is needed, not *how* to do it.
- **Interpreter**: Omnia. It handles the *how* for every effect. Guests remain deterministic, while Omnia deals with the impurity of the outside world.
- **`judge`**: Our primary effect for judgment. It takes a brief and a typed request, and returns a typed answer. The implementation just happens to be an AI model.
- **Oracle**: How the guest views the model — as an opaque source of typed answers. The guest trusts the shape of the response, not the system that generated it.
- **Handle**: A reference (like a file path or an ID) that points to data without actually carrying the data itself. This is how we keep things lazy and avoid blowing up context windows.

For context, here is how these concepts map to our actual tech stack:

| Concept | Technology |
| --- | --- |
| Program / Guest | A Wasm component (`wasm32-wasip2`) |
| Runtime / Interpreter | Omnia (a Wasmtime-based CLI binary) |
| Effect | A WIT interface import |
| Typed boundary | WIT records and interfaces |
| Handle | A brief path, artifact path, or reference ID |
| `judge` backend | The model service (LLM, local model, or replay stub) |
| `load-reference` | Omnia's fallback for resolving references when the model can't read the filesystem directly |

## How the model reads a brief

When a guest calls `judge`, it doesn't pass the entire text of a brief. Instead, it passes a handle — usually the file path to the brief on disk. 

If the backend is a filesystem-capable agent (like Cursor or Claude Code), Omnia just gives it the path. The agent reads the brief, follows any relative links to supporting docs, and only pulls in the context it actually needs to make a decision. This keeps our context windows lean.

This is why our adapters remain a hybrid of Wasm and prose. The Wasm handles the orchestration, while the prose (the briefs and references) lives on disk, linked together naturally.

![Logical sequence: extract](../docs/assets/diagrams/effect-architecture/sequence-extract.svg)

If we're using a backend that can't read the filesystem (like a raw API endpoint), Omnia steps in to help. It resolves the brief and uses the `load-reference` fallback to inject the necessary context. This is safe because any computed references are handled by a fresh guest instance, but it's strictly a fallback. Most of the time, the agent just reads the files it needs directly.

## Lifecycle of an operation

![Logical sequence: build](../docs/assets/diagrams/effect-architecture/sequence-build.svg)

Let's look at how a `build` operation flows in practice. (Note that in this logical view, every step is technically an Omnia invocation running either Wasm or a model backend).

1. The workflow guest starts the loop and invokes the target adapter (e.g., `build(build-request)`).
2. The target guest runs its deterministic setup code.
3. When it needs to make a judgment call, it requests `judge(brief-path, request)`.
4. Omnia routes this to the configured model backend.
5. The model reads the brief and lazily pulls in any supporting references it needs.
6. The model returns its answer. Omnia validates that it matches the expected type and hands it back to the target guest.
7. The guest uses this typed result to decide what to do next (loop, validate, or make another `judge` call).
8. Finally, the guest returns a typed `build-report`, and Omnia handles the lifecycle transition.

The key takeaway here is the flow: control moves *into* guests via exports, effects flow *up* to Omnia, judgment is handled by a swappable backend, and references are loaded lazily. 

## Core principles

When evaluating future design decisions — whether something should be prose or code, what a function should take, or where it should run — keep this guiding principle in mind:

> Run everything as a guest on a runtime that only understands effects. Keep the structure in deterministic guest code, delegate judgment to the `judge` effect, and always pass handles instead of raw text across boundaries.

These specific principles apply across the entire architecture. If a proposed change conflicts with them, we should step back and reconsider the approach.

1. **Strictly typed boundaries.** We use WIT records for data and WIT interfaces for effects. We don't pass raw, untyped text corpora across boundaries.
2. **The runtime only knows about effects.** Omnia doesn't know about workflows, adapters, or AI models. It just knows how to handle effects. 
3. **Determinism by default, judgment by exception.** The core state machine (loops, logic, sequencing) lives in deterministic guest code. When we need judgment, we call `judge` and get back a typed decision that steers the next deterministic step. We don't ask models to guess our control flow.
4. **Laziness is essential.** We pass handles (like file paths) across boundaries, not massive text bodies. This ensures our context windows stay manageable and our operations remain scalable.

## The model fleet and deployment modes

![The model fleet and deployment topologies](../docs/assets/diagrams/effect-architecture/model-fleet.svg)

Because Omnia is just a thin interpreter for effects and every `judge` call is self-contained, we don't have to rely on a single, long-lived conversation. This allows our model service to act as a router, choosing the right backend for each specific task based on difficulty and cost. Several different deployment modes naturally fall out of this design:

- **Interactive (Frontier LLMs)**: When you trigger a phase from your editor, `judge` is handled by a spawned, context-free agent session using a frontier LLM for complex synthesis and review. It runs against concrete artifacts, not your chat history.
- **Headless (Small Local Models)**: For fleet-scale operations and narrow, high-volume transformations, `judge` can be routed to a hosted API or a local model without any editor in the loop. We can use constrained decoding to ensure they return valid typed reports.
- **CI / Testing (Deterministic Replay)**: In a CI environment, `judge` can be swapped out for a deterministic replay stub, turning AI evaluations into reliable regression tests.

This setup allows us to progressively optimise. As a specific transformation becomes well-understood, we can migrate it from an expensive LLM down to a cheaper local model, or even to deterministic code, without having to rewrite the guest that calls it. This architecture also means "runtime-agnostic" is actually enforced by the type system, and because we load context lazily, deep orchestration doesn't blow our token budgets.

## The incremental path

![Staged path to the architecture](../docs/assets/diagrams/effect-architecture/roadmap.svg)

We are moving toward this architecture in stages. Each stage is valuable on its own and forward-compatible:

- **S0–S1 (Typed contract)**: Moving to typed records and callable deterministic tools.
- **S2 (Name the effects)**: Formalising `judge`, data, and lifecycle events as typed WIT imports. This unlocks record/replay testing.
- **S3 (Guests orchestrate)**: Adapters start running their own multi-step operations on the runtime.
- **S4 (Runtime move)**: Replacing the bespoke `specify` host with the generic Omnia binary, and running the workflow as a guest.
- **Parallel (The model fleet)**: Building out the routing and backends for the `judge` effect.

For more details on the staging, see [roadmap.md](roadmap.md).

## Key trade-offs

We are making a few deliberate bets with this architecture:

- **Omnia is the runtime.** We are retiring the bespoke `specify` host in favor of Omnia. This gives us flexible deployment modes, but it does mean we have a hard dependency on Omnia's host-service surface.
- **Judgment requires a host.** Running a judgment step means we need a host that can satisfy the `judge` effect. We aren't trying to embed the runtime directly inside the user's live editor session.

## How this builds on what exists

This architecture doesn't throw away what we've built; it just makes the foundations more explicit:

- **Adapter-agnostic core**: The runtime still doesn't know about specific adapters, and now it doesn't know about the workflow either.
- **Identity and packaging**: Adapters are still published as a mix of Wasm and prose. We've just changed the Wasm to be an orchestrating guest and improved how the prose is loaded.
- **The typed contract**: This remains our foundation, but we've expanded it to cover effects and lazy discovery as well.
