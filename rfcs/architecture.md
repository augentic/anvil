# Specify on Omnia — The Effect-Oriented Architecture

> Status: This is the standing architecture — the agreed-upon direction being worked toward. It helps sequence work rather than landing as a single massive change, and it lives alongside [roadmap.md](roadmap.md).

## The core idea

The new Specify architecture boils down to one core concept:

> Specify is a family of Wasm components running on the [Omnia](https://github.com/augentic/omnia) runtime. The workflow and every adapter are guests. Judgment is treated as an effect — `judge` — that the runtime delegates to a pluggable fleet of models. The "Specify" CLI is Omnia compiled with Specify-specific backends.

Everything that follows — the system's shape, core principles, how operations flow, and deployment modes — is a consequence of this idea. The runtime doesn't need to understand the domain: it knows how to host Wasm guests and handle a fixed set of effects. Specify's behaviour — orchestrating workflows, extracting from sources, building for targets, and development tooling — lives within these guests.

Context comes from artifacts, not from conversational history. Every `judge` call is self-contained. It points to a brief and makes a typed request against concrete artifacts (like `spec.md` or a build request), rather than relying on an accumulated chat transcript. This avoids the overloaded context windows that often cause failures. 

This approach provides two major benefits:
1. **Scalability and auditability**: An operation runs exactly the same way whether triggered from an editor or run in a massive CI pipeline.
2. **Cost efficiency**: Because judgment is a typed call over specific inputs, easy tasks can be routed to deterministic code and narrow tasks to small local models, reserving expensive frontier LLMs only for the hardest problems.

## The shape of the system

![The shape of the system](../docs/assets/diagrams/effect-architecture/system-shape.svg)

The system is split into two roles communicating over a single contract: a generic runtime and the guests that run on it.

- **Omnia is the foundation.** It's a command-line executable. Its main job is to run a guest (e.g., `omnia workflow.wasm plan …`) and pass along any arguments. It instantiates the guest and handles a small vocabulary of effects, but it doesn't know anything about adapters, workflows, or AI models.
- **Everything else is a guest.** The workflow (`plan`, `execute`) and the development tools are first-party guests. The adapters are guests too, whether they are source guests (like `typescript` or `documentation`) or target guests (like `omnia` or `vectis`). They all run as peers on the runtime.
- **The model fleet sits below.** When a guest needs to make a judgment call, it requests the `judge` effect. Omnia then dispatches this request to a pluggable backend — which could be a frontier LLM, a small local model, or even a deterministic replay stub for testing. The "brain" is a swappable implementation detail.
- **The boundary is typed.** Every piece of data and every effect crosses the boundary with a defined type. Untyped text is not passed across this line — only typed data and handles.

The runtime and guests interact in both directions. Omnia instantiates a guest and calls its exported functions (like `build` or `extract`). In turn, the guest calls back into Omnia's host services (like `judge`, `load-reference`, or `journal`) whenever it needs to do something impure. 

> **A quick note on naming:** In this document, "Omnia" refers to two things: the runtime itself, and the `omnia` target guest (the adapter that generates code for the Omnia runtime). This document will be explicit when the context isn't obvious.

## The runtime: Omnia

Omnia is built on Wasmtime. Its design centers around providing pluggable host services (like HTTP, key-value storage, or observability) behind typed interfaces, so implementations can be swapped without changing the guest code. Specify's effects are another set of host services, with the `judge` model service being the most notable addition ([RFC-57](rfc-57-omnia-model-host.md)).

Three key properties of the runtime make this architecture possible:

- **One binary, guest-selected behaviour.** There is no longer a bespoke `specify` host. Instead, `omnia <guest>.wasm <args…>` is run, and the guest decides what to do with the invocation. 
- **Instance-per-call execution.** Every time a guest is called, a fresh instance is spun up. Wasm component instances aren't re-entrant, so this avoids a whole class of async complexity. If a judgment step needs to resolve a reference by calling back into a guest, it happens in a brand new instance.
- **Stateless guests, host-held state.** Because fresh instances are used per call, guests can't hold onto state in memory. Anything that needs to persist lives in a host service, like a key-value store. This makes the runtime easy to scale horisontally.

## The mental model: programs, effects, and interpreters

![The effect model](../docs/assets/diagrams/effect-architecture/effect-model.svg)

This design borrows heavily from the concept of algebraic effects. The guest program handles pure control flow, and whenever it needs to interact with the non-deterministic outside world, it requests a typed effect. A separate interpreter (Omnia) decides how to perform that effect. 

Here is the vocabulary used:

- **Effect**: A typed request from a guest for something it can't do itself (like running a brief, reading a file, or logging an event). The effect says *what* is needed, not *how* to do it.
- **Interpreter**: Omnia. It handles the *how* for every effect. Guests remain deterministic, while Omnia deals with the impurity of the outside world.
- **`judge`**: The primary effect for judgment. It takes a brief and a typed request, and returns a typed answer. The implementation happens to be an AI model.
- **Oracle**: How the guest views the model — as an opaque source of typed answers. The guest trusts the shape of the response, not the system that generated it.
- **Handle**: A reference (like a file path or an ID) that points to data without carrying the data itself. This keeps things lazy to avoid blowing up context windows.

For context, here is how these concepts map to the tech stack:

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

If the backend is a filesystem-capable agent (like Cursor or Claude Code), Omnia gives it the path. The agent reads the brief, follows any relative links to supporting docs, and only pulls in the context it needs to make a decision. This keeps context windows lean.

Therefore, the adapters remain a hybrid of Wasm and prose. The Wasm handles the orchestration, while the prose (the briefs and references) lives on disk, linked together naturally.

![Logical sequence: extract](../docs/assets/diagrams/effect-architecture/sequence-extract.svg)

If a backend is used that can't read the filesystem (like a raw API endpoint), Omnia steps in to help. It resolves the brief and uses the `load-reference` fallback to inject the necessary context. This is safe because any computed references are handled by a fresh guest instance. Most of the time, the agent reads the files it needs directly.

## Lifecycle of an operation

![Logical sequence: build](../docs/assets/diagrams/effect-architecture/sequence-build.svg)

Let's look at how a `build` operation flows in practice. (Note that in this logical view, every step is an Omnia invocation running either Wasm or a model backend).

1. The workflow guest starts the loop and invokes the target adapter (e.g., `build(build-request)`).
2. The target guest runs its deterministic setup code.
3. When it needs to make a judgment call, it requests `judge(brief-path, request)`.
4. Omnia routes this to the configured model backend.
5. The model reads the brief and lazily pulls in any supporting references it needs.
6. The model returns its answer. Omnia validates that it matches the expected type and hands it back to the target guest.
7. The guest uses this typed result to decide what to do next (loop, validate, or make another `judge` call).
8. Finally, the guest returns a typed `build-report`, and Omnia handles the lifecycle transition.

In short: control moves *into* guests via exports, effects flow *up* to Omnia, judgment is handled by a swappable backend, and references are loaded lazily. 

## Core principles

When evaluating future design decisions — whether something should be prose or code, what a function should take, or where it should run — keep this guiding principle in mind:

> Run everything as a guest on a runtime that only understands effects. Keep the structure in deterministic guest code, delegate judgment to the `judge` effect, and pass handles instead of raw text across boundaries.

These specific principles apply across the entire architecture. If a proposed change conflicts with them, it's worth reconsidering the approach.

1. **Typed boundaries.** WIT records are used for data and WIT interfaces for effects. Untyped text is not passed across boundaries.
2. **The runtime only knows about effects.** Omnia doesn't know about workflows, adapters, or AI models. It knows how to handle effects. 
3. **Determinism by default, judgment by exception.** The core state machine (control flow) lives in deterministic guest code. When judgment is needed, `judge` is called and returns a typed decision that steers the next deterministic step. Models are not asked to guess control flow.
4. **Laziness is key.** Handles (like file paths) are passed across boundaries, not massive text bodies. This ensures context windows stay manageable and operations remain scalable.

## The model fleet and deployment modes

![The model fleet and deployment topologies](../docs/assets/diagrams/effect-architecture/model-fleet.svg)

Because Omnia is a thin interpreter for effects and every `judge` call is self-contained, there is no reliance on a single, long-lived conversation. The model service can thus act as a router, choosing the right backend for each specific task based on difficulty and cost. Several different deployment modes naturally fall out of this design:

- **Interactive (Frontier LLMs)**: When a phase is triggered from an editor, `judge` is handled by a spawned, context-free agent session using a frontier LLM for complex synthesis and review. It runs against concrete artifacts, not chat history.
- **Headless (Small Local Models)**: For fleet-scale operations and narrow, high-volume transformations, `judge` can be routed to a hosted API or a local model without any editor in the loop. Constrained decoding can be used to ensure valid typed reports are returned.
- **CI / Testing (Deterministic Replay)**: In a CI environment, `judge` can be swapped out for a deterministic replay stub, turning AI evaluations into reliable regression tests.

This setup enables progressive optimisation. As a specific transformation becomes well-understood, it can be migrated from an expensive LLM down to a cheaper local model, or even to deterministic code, without having to rewrite the guest that calls it. This architecture also enforces "runtime-agnostic" via the type system, and because context is loaded lazily, deep orchestration doesn't blow token budgets.

## Host services and state

Guests are stateless and instance-per-call, so anything that must outlive a single call lives in a host service, not in guest memory. These are the *deterministic* effects — the counterpart to the judgment backend above — and each is satisfied by a swappable backend the guest never sees:

- **Data** (`read-artifact` / `get-asset`) — narrow, typed access to the project tree and assets that flow into an operation, in place of a broad filesystem grant.
- **`kv`** — host-held scratch and memoization (where `load-reference` caches a computed reference, or a guest stashes a deterministic sub-result). Backed by filesystem locally, Redis or NATS when a fleet shares state.
- **`journal` / `transition`** — the durable lifecycle log and the legal moves over it.

### Journalling: progress and restarts

Because guests hold nothing, progress and restart state live in the durable `journal`. A guest records forward progress by emitting typed, closed-taxonomy events (`slice.build.started`, `slice.build.succeeded`, …).

Omnia's [`wasi-jsondb`](https://github.com/augentic/omnia/tree/main/crates/wasi-jsondb) host service is a natural fit: each event is a JSON document and `wasi-jsondb` is backed by filesystem, Redis, NATS, etc..

## CLI bootstrapping

Because "Specify is Omnia compiled with Specify-specific backends", there is no separate runtime to download — the binary *is* the runtime, linked with its backends. The runtime acquires its guests through a composed approach:

- **Embedded core seed:** Core first-party guests (like the workflow guest) can be compiled directly into the binary using `include_bytes!`. This ensures that initialization works offline and eliminates version skew between the runtime and its most critical guests.

- **Config-driven resolution:** The long tail of adapters and third-party guests are resolved dynamically via digest-pinned references. Omnia is enhanced with a generic resolver to fetch these into a local cache. Because adapters are mixed Wasm and prose trees (rather than pure Wasm components), they are distributed via an OCI store.

This combination provides a small binary that can still bootstrap offline, while decoupling the release cadence of most adapters from the core runtime.

## The incremental path

![Staged path to the architecture](../docs/assets/diagrams/effect-architecture/roadmap.svg)

The architecture is being approached in stages. Each stage is valuable on its own and forward-compatible:

- **S0–S1 (Typed contract)**: Moving to typed records and callable deterministic tools.
- **S2 (Name the effects)**: Formalising `judge`, data, and lifecycle events as typed WIT imports. This unlocks record/replay testing.
- **S3 (Guests orchestrate)**: Adapters start running their own multi-step operations on the runtime.
- **S4 (Runtime move)**: Replacing the bespoke `specify` host with the generic Omnia binary, and running the workflow as a guest.
- **Parallel (The model fleet)**: Building out the routing and backends for the `judge` effect.

For more details on the staging, see [roadmap.md](roadmap.md).

## Key trade-offs

A few deliberate bets are being made with this architecture:

- **Omnia is the sole runtime.** This provides flexible deployment modes, but it does mean there is a hard dependency on Omnia's host-service surface.
- **Judgment requires a host.** Running a judgment step means a host is needed that can satisfy the `judge` effect. There is no attempt to embed the runtime directly inside a user's live editor session.
