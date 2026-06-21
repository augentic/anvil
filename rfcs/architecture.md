# Specify on Omnia — The Effect-Oriented Architecture

> Status: This is the standing architecture — the agreed-upon direction being worked toward. It helps sequence work rather than landing as a single massive change, and it lives alongside [roadmap.md](roadmap.md).

## The core idea

The refreshed Specify architecture boils down to a single concept:

> Specify is a family of Wasm components running on the [Omnia](https://github.com/augentic/omnia) runtime. The workflow and every adapter are guests. Judgment is treated as an effect that the runtime delegates to a pluggable fleet of models. The "Specify" CLI is Omnia compiled with Specify-specific backends.

Everything that follows — the system's shape, core principles, how operations flow, and deployment modes — is a consequence of this idea. The runtime doesn't need to understand the domain: it knows how to host Wasm guests and handle a fixed set of effects. Specify's behaviour — orchestrating workflows, extracting from sources, building for targets, and development tooling — lives within the guests.

Context comes from artifacts, not conversational history. Every call out to a model for "judgement" is self-contained. It points to a brief and makes a typed request against concrete artifacts (like `spec.md` or a build request), rather than relying on an accumulated chat transcript. This avoids the overloaded context windows that often cause failures. 

This approach provides three major benefits:
1. **Cloud-native portability**: Specify scales seamlessly from a desktop CLI to a cloud-hosted service. Because the execution environment is entirely abstracted behind Omnia's effects, moving to the cloud requires zero changes to Specify's core logic—it only requires swapping Omnia's backends.
2. **Scalability and auditability**: An operation runs exactly the same way whether triggered from an editor or run in a CI pipeline.
3. **Cost efficiency**: Because judgment is a typed call over specific inputs, selected tasks can be routed to deterministic code or to small local models, reserving expensive frontier LLMs only for the hardest problems.

## The shape of the system

![The shape of the system](../docs/assets/diagrams/effect-architecture/system-shape.svg)

The system is split into two roles communicating over a single contract: a generic runtime and the guests that run on it.

- **Omnia is the foundation**: It's a command-line executable that runs a guest (e.g., `omnia workflow.wasm plan …`) and passes along arguments. It instantiates the guest and handles a small vocabulary of effects, but knows nothing about adapters, workflows, or AI models.
- **Everything else is a guest**: Workflows (`plan`, `execute`), development tools, and adapters (source or target) are all guests that run as peers on the runtime.
- **The model fleet sits below**: When a guest needs judgment, it requests the `eval` effect. Omnia dispatches this to a pluggable backend (a frontier LLM, local model, or replay stub). The "brain" is swappable.
- **The boundary is typed**: Every piece of data and effect crosses the boundary with a defined type. Untyped text is never passed across this line — only typed data and handles.

The runtime and guests interact in both directions. Omnia instantiates a guest and calls its exported functions (like `build` or `extract`). In turn, the guest calls back into Omnia's host services (like `eval`, `resolve`, or `journal`) whenever it needs to do something impure. 

> **A quick note on naming:** In this document, "Omnia" refers to two things: the runtime itself, and the `omnia` target guest (the adapter that generates code for the Omnia runtime). This document will be explicit when the context isn't obvious.

## The runtime: Omnia

Omnia is built on Wasmtime. Its design centers around providing pluggable host services (like HTTP, key-value storage, or observability) behind typed interfaces, so implementations can be swapped without changing the guest code. Specify's effects are another set of host services, with the model service being the most notable addition ([RFC-54](rfc-54-model-host.md)).

Three key properties of the runtime make this architecture possible:

- **One binary, guest-selected behaviour**: `omnia <guest>.wasm <args…>` is run, and the guest decides what to do. There is no longer a bespoke `specify` host.
- **Instance-per-call execution**: A fresh instance spins up every time a guest is called. Wasm component instances aren't re-entrant, avoiding a whole class of async complexity.
- **Stateless guests, host-held state**: Guests cannot hold state in memory between calls. Persistent data lives in a host service. In a local CLI context, this might be a filesystem-backed store; in a cloud context, it can be swapped for Redis or S3. This decoupling is what allows Specify to instantly transition from a desktop tool to a horizontally scalable cloud service.

## The mental model: programs, effects, and interpreters

![The effect model](../docs/assets/diagrams/effect-architecture/effect-model.svg)

This design borrows heavily from the concept of algebraic effects. The guest program handles pure control flow, and whenever it needs to interact with the non-deterministic outside world, it requests a typed effect. A separate interpreter (Omnia) decides how to perform that effect. 

Here is the vocabulary used:

- **Effect**: A typed request from a guest for something it can't do itself (like running a brief, reading a file, or logging). It declares *what* is needed, not *how* to do it.
- **Interpreter**: Omnia. It handles the *how* for every effect. Guests remain deterministic, while Omnia deals with the impurity of the outside world.
- **`eval`**: The primary effect for judgment. It takes a brief and a typed request, and returns a typed answer (usually via an AI model).
- **Oracle**: How the guest views the model — as an opaque source of typed answers. The guest trusts the shape of the response, not the generator.
- **Handle**: A reference (like a file path or ID) that points to data without carrying it, keeping context windows lazy and small.

For context, here is how these concepts map to the tech stack:

| Concept               | Technology                                                                                  |
| --------------------- | ------------------------------------------------------------------------------------------- |
| Program / Guest       | A Wasm component (`wasm32-wasip2`)                                                          |
| Runtime / Interpreter | Omnia (a Wasmtime-based CLI binary)                                                         |
| Effect                | A WIT interface import                                                                      |
| Typed boundary        | WIT records and interfaces                                                                  |
| Handle                | A brief path, artifact path, or reference ID                                                |
| `eval` backend       | The model service (LLM, local model, or replay stub)                                        |
| `resolve`             | Omnia's fallback for resolving references when the model can't read the filesystem directly |

## How the model reads a brief

When a guest calls `eval`, it doesn't pass the entire text of a brief. Instead, it passes a handle — usually the file path to the brief on disk. 

If the backend is a filesystem-capable agent (like Cursor or Claude Code), Omnia gives it the path. The agent reads the brief, follows any relative links to supporting docs, and only pulls in the context it needs to make a decision. This keeps context windows lean.

Therefore, the adapters remain a hybrid of Wasm and prose. The Wasm handles the orchestration, while the prose (the briefs and references) lives on disk, linked together naturally.

![Logical sequence: extract](../docs/assets/diagrams/effect-architecture/sequence-extract.svg)

If a backend is used that can't read the filesystem (like a raw API endpoint), Omnia steps in to help. It resolves the brief and uses the `resolve` fallback to inject the necessary context. This is safe because any computed references are handled by a fresh guest instance. Most of the time, the agent reads the files it needs directly.

## Guest-to-guest interaction

When the workflow guest needs to call an adapter guest (for instance, when `/spec:execute` needs to call a target adapter's `build` export), there are two primary mechanisms supported by the WebAssembly Component Model and Wasmtime. Host-mediated dynamic linking is the preferred approach as it aligns with Omnia's dynamic resolution capabilities.

### 1. Host-Mediated Dynamic Linking (Preferred)

Because Omnia resolves guests dynamically at runtime (e.g., from an OCI store), static linking is often not feasible. Instead, Wasmtime's host-mediated dynamic linking provides a strictly typed boundary between guests without requiring ahead-of-time composition.

- **How it works**: The `workflow` world imports the target adapter's interface. The Omnia host intercepts this call using Wasmtime's `wasmtime::component::Linker` and Dynamic API (`wasmtime::component::Val`).
- **The Host's role**: The host dynamically instantiates a fresh, stateless instance of the target adapter, marshals the typed WIT records natively from the caller's memory to the callee's memory, invokes the exported function, and returns the typed result.
- **Why it fits**: This perfectly satisfies Omnia's requirements. It maintains strict WIT typing without manual byte serialization, supports dynamic adapter resolution, and enforces the "instance-per-call" principle, completely avoiding Wasmtime re-entrancy issues.

### 2. Static Component Composition

In this approach, guest-to-guest calls are resolved inside the Wasm sandbox without trapping to the host runtime.

- **How it works**: The `workflow` world *imports* `augentic:specify/target` and `augentic:specify/source`. The target and source adapters *export* these exact interfaces.
- **The Host's role**: Before Omnia executes the workflow, the WebAssembly Component Model allows statically composing the `workflow.wasm` and the relevant adapter `.wasm` files into a single, merged `composed.wasm` using tools like `wac` (WebAssembly Composition) or `wasm-tools compose`.
- **Why it fits**: Omnia instantiates this single `composed.wasm`. When the workflow calls `build()`, it is a direct function call to the adapter within the same sandbox. However, this requires resolving and linking all adapters ahead of time, which conflicts with Omnia's dynamic, config-driven resolution.

## Lifecycle of an operation

![Logical sequence: build](../docs/assets/diagrams/effect-architecture/sequence-build.svg)

Let's look at how a `build` operation flows in practice. (Note that in this logical view, every step is an Omnia invocation running either Wasm or a model backend).

1. The workflow guest starts the loop and invokes the target adapter (e.g., `build(build-request)`).
2. The target guest runs its deterministic setup code.
3. When it needs to make a judgment call, it requests `eval(brief-path, request)`.
4. Omnia routes this to the configured model backend.
5. The model reads the brief and lazily pulls in any supporting references it needs.
6. The model returns its answer. Omnia validates that it matches the expected type and hands it back to the target guest.
7. The guest uses this typed result to decide what to do next (loop, validate, or make another `eval` call).
8. Finally, the guest returns a typed `build-report`, and Omnia handles the lifecycle transition.

In short: control moves *into* guests via exports, effects flow *up* to Omnia, judgment is handled by a swappable backend, and references are loaded lazily. 

## Core principles

When evaluating future design decisions — whether something should be prose or code, what a function should take, or where it should run — keep this guiding principle in mind:

> Run everything as a guest on a runtime that only understands effects. Keep the structure in deterministic guest code, delegate judgment to the `eval` effect, and pass handles instead of raw text across boundaries.

These specific principles apply across the entire architecture. If a proposed change conflicts with them, it's worth reconsidering the approach.

1. **Typed boundaries**: WIT records are used for data and WIT interfaces for effects. Untyped text is not passed across boundaries.
2. **The runtime only knows about effects**: Omnia doesn't know about workflows, adapters, or AI models — it only knows how to handle effects. 
3. **Determinism by default, judgment by exception**: Control flow lives in deterministic guest code. `eval` returns a typed decision to steer the next deterministic step. Models do not guess control flow.
4. **Laziness is key**: Handles (like file paths) are passed across boundaries instead of massive text bodies, keeping context windows manageable and operations scalable.

## The model fleet and deployment modes

![The model fleet and deployment topologies](../docs/assets/diagrams/effect-architecture/model-fleet.svg)

Because Omnia is a thin interpreter for effects and every `eval` call is self-contained, there is no reliance on a single, long-lived conversation. The model service can thus act as a router, choosing the right backend for each specific task based on difficulty and cost. Several different deployment modes naturally fall out of this design:

- **Interactive (Frontier LLMs)**: When triggered from an editor, `eval` uses a frontier LLM against concrete artifacts (not chat history) for complex synthesis and review.
- **Headless (Small Local Models)**: For fleet-scale operations and narrow transformations, `eval` routes to a hosted API or local model without an editor in the loop. Constrained decoding ensures valid typed reports.
- **CI / Testing (Deterministic Replay)**: In CI, `eval` swaps to a deterministic replay stub, turning AI evaluations into reliable regression tests.

This setup enables progressive optimisation. As a specific transformation becomes well-understood, it can be migrated from an expensive LLM down to a cheaper local model, or even to deterministic code, without having to rewrite the guest that calls it. This architecture also enforces "runtime-agnostic" via the type system, and because context is loaded lazily, deep orchestration doesn't blow token budgets.

## Host services and state

Guests are stateless and instance-per-call, so anything that must outlive a single call lives in a host service, not in guest memory. These are the *deterministic* effects — the counterpart to the judgment backend above — and each is satisfied by a swappable backend the guest never sees:

- **Data** (`read`): Narrow, typed access to the project tree and assets through one accessor returning bytes, replacing a broad filesystem grant.
- **`state`**: Host-held scratch and memoization (e.g., caching a computed reference). uses KeyValue interface backed locally by filesystem, or Redis/NATS for fleet-shared state.
- **`journal`**: The durable lifecycle log and its legal moves. Uses `JsonStore` backed by a filesystem backend.

Because guests only interact with typed interfaces (like `KeyValue` or `read`), the deployment topology is dictated entirely by the host backends. A local CLI binary wires these interfaces to the local filesystem. A cloud deployment wires the exact same interfaces to cloud-native infrastructure (like S3 for `read` and Redis for `state`). The Specify Wasm components do not change.

### Journalling progress

Because guests hold nothing, progress and restart state live in the durable `journal`. A guest records forward progress by emitting typed, closed-taxonomy events (`slice.build.started`, `slice.build.succeeded`, …).

Omnia's [`wasi-jsondb`](https://github.com/augentic/omnia/tree/main/crates/wasi-jsondb) host service is a natural fit for this functionality. Each event is a JSON document and it can readily be backed by a new filesystem backend.

## CLI bootstrapping

Because "Specify is Omnia compiled with Specify-specific backends", there is no separate runtime to download — the binary *is* the runtime, linked with its backends. The runtime acquires its guests through a composed approach:

- **Embedded core seed**: Core first-party guests (like the workflow) compile directly into the binary using `include_bytes!`, ensuring offline initialization and zero version skew.
- **Config-driven resolution**: Adapters and third-party guests resolve dynamically via digest-pinned references from an OCI store into a local cache.

This combination provides a small binary that can still bootstrap offline, while decoupling the release cadence of most adapters from the core runtime.

## The incremental path

![Staged path to the architecture](../docs/assets/diagrams/effect-architecture/roadmap.svg)

The architecture is being approached in stages. Each stage is valuable on its own and forward-compatible:

- **S0–S1 (Typed contract)**: Authoring the typed-records WIT package as the single source of truth (callable `tool` dispatch and the schema-drift retirement follow in S3 / S2).
- **S2 (Name the effects)**: Formalising `eval`, data, and lifecycle events as typed WIT imports. This unlocks record/replay testing and retires the schema-drift surface.
- **S3 (Guests orchestrate)**: Deterministic tools become callable through the typed bindings, and adapters start running their own multi-step operations on the runtime.
- **S4 (Runtime move)**: Replacing the bespoke `specify` host with the generic Omnia binary, running the workflow as a guest.
- **Parallel (The model fleet)**: Building routing and backends for the `eval` effect.

For more details on the staging, see [roadmap.md](roadmap.md).

## Key trade-offs

A few deliberate bets are being made with this architecture:

- **Omnia is the sole runtime**: Provides flexible deployment modes, but creates a hard dependency on Omnia's host-service surface.
- **Judgment requires a host**: Running a judgment step means a host must satisfy the `eval` effect. The runtime is not embedded directly inside a live editor session.
