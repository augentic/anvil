# Specify on Omnia — The Effect-Oriented Architecture

> Status: This is the standing architecture — the agreed-upon direction being worked toward. It helps sequence work rather than landing as a single massive change, and it lives alongside [roadmap.md](roadmap.md).

## The core idea

The refreshed Specify architecture boils down to a single concept:

> Specify is a family of Wasm components running on the [Omnia](https://github.com/augentic/omnia) runtime. The workflow and every adapter are guests. Judgment is a **native tool-use loop** in the binary's own orchestration layer: it drives a model API directly and lets the model navigate, read, write, and verify through a typed tool surface. The "Specify" CLI is Omnia compiled with Specify-specific backends and that native loop.

Everything that follows — the system's shape, core principles, how operations flow, and deployment modes — is a consequence of this idea. The runtime doesn't need to understand the domain: it knows how to host Wasm guests and handle a fixed set of *deterministic* effects (filesystem, key-value, lifecycle). Specify's behaviour — orchestrating workflows, extracting from sources, building for targets, and development tooling — lives within the guests and the native orchestration layer that drives them; the runtime floor stays model-agnostic.

Context comes from artifacts, not conversational history. Every judgment run is self-contained: the native loop hands the model one **whole brief** and a typed tool surface scoped to concrete artifacts (like `spec.md` or a build request), rather than relying on an accumulated chat transcript. This avoids the overloaded context windows that often cause failures. 

This approach provides three major benefits:
1. **Cloud-native portability**: Specify scales seamlessly from a desktop CLI to a cloud-hosted service. Because the execution environment is entirely abstracted behind Omnia's effects, moving to the cloud requires zero changes to Specify's core logic—it only requires swapping Omnia's backends.
2. **Scalability and auditability**: An operation runs exactly the same way whether triggered from an editor or run in a CI pipeline.
3. **Cost efficiency**: Because judgment is a typed call over specific inputs, selected tasks can be routed to deterministic code or to small local models, reserving expensive frontier LLMs only for the hardest problems.

## The shape of the system

![The shape of the system](../docs/assets/diagrams/effect-architecture/system-shape.svg)

The system is split into two roles communicating over a single contract: a generic runtime and the guests that run on it.

- **Omnia is the foundation**: It's a command-line executable that runs a guest (e.g., `omnia workflow.wasm plan …`) and passes along arguments. It instantiates the guest and handles a small vocabulary of deterministic effects, but knows nothing about adapters, workflows, or AI models.
- **Everything else is a guest**: Workflows (`plan`, `execute`), development tools, and adapters (source or target) are all guests that run as peers on the runtime.
- **Judgment is native, beside the runtime**: When an operation needs judgment, the binary's **native orchestration layer** runs a tool-use loop — it calls a model API directly (behind a swappable `ModelClient` boundary) and dispatches the model's tool calls to a typed facade. The "brain" is swappable, but as native configuration, not a runtime effect, so Omnia core never learns a model id (law 2).
- **The boundary is typed**: Every piece of data and effect crosses the boundary with a defined type. Untyped text is never passed across this line — only typed data and handles.

The runtime and guests interact in both directions. Omnia instantiates a guest and calls its exported functions (like `build`, `extract`, or `resolve`). In turn, the guest calls back into Omnia's host services (like `wasi:filesystem` or `journal`) whenever it needs to do something impure. Judgment is the exception: it is not an effect a guest requests upward but a loop the native layer runs *around* the guests, driving the model and the guest's typed I/O surface. 

> **A quick note on naming:** In this document, "Omnia" refers to two things: the runtime itself, and the `omnia` target guest (the adapter that generates code for the Omnia runtime). This document will be explicit when the context isn't obvious.

## The runtime: Omnia

Omnia is built on Wasmtime. Its design centers around providing pluggable host services (like HTTP, key-value storage, or observability) behind typed interfaces, so implementations can be swapped without changing the guest code. Specify extends this surface in exactly one sanctioned way: **custom backends behind Omnia's existing general-purpose hosts** (for example, a git-aware `wasi:filesystem` backend that materializes the [working tree](#the-working-tree)) — it adds **no new runtime host**. Crucially, judgment is *not* a new host: the model client lives in the binary's own **native orchestration layer** — the code around guest instantiation that owns the judgment tool-loop (model client, tool dispatch, `verify`), `slice → revision` resolution, `change-set` extraction, and forge push. Keeping the model client there rather than behind a runtime interface is what lets Omnia core stay model-agnostic (law 2) while the *same binary* runs unchanged from desktop to cloud.

Three key properties of the runtime make this architecture possible:

- **One binary, guest-selected behaviour**: `omnia <guest>.wasm <args…>` is run, and the guest decides what to do. There is no longer a bespoke `specify` host.
- **Instance-per-call execution**: A fresh instance spins up every time a guest is called. Wasm component instances aren't re-entrant, avoiding a whole class of async complexity.
- **Stateless guests, host-held state**: Guests cannot hold state in memory between calls. Persistent data lives in a host service. In a local CLI context, this might be a filesystem-backed store; in a cloud context, it can be swapped for Redis or S3. This decoupling is what allows Specify to instantly transition from a desktop tool to a horizontally scalable cloud service.

## The mental model: programs, effects, and interpreters

![The effect model](../docs/assets/diagrams/effect-architecture/effect-model.svg)

This design borrows heavily from the concept of algebraic effects for everything *deterministic*. The guest program handles pure control flow, and whenever it needs to interact with the non-deterministic outside world, it requests a typed effect. A separate interpreter (Omnia) decides how to perform that effect. Judgment is the one thing held *outside* this effect system — it is a native loop the orchestration layer runs around the guests, not an effect a guest requests.

Here is the vocabulary used:

- **Effect**: A typed request from a guest for something it can't do itself (reading a file, logging, key-value state). It declares *what* is needed, not *how* to do it. Effects are deterministic; judgment is not one.
- **Interpreter**: Omnia. It handles the *how* for every effect. Guests remain deterministic, while Omnia deals with the impurity of the outside world.
- **Native tool-use loop**: The judgment mechanism. The orchestration layer hands a model one whole brief plus a typed tool surface, dispatches the model's tool calls, runs the verify-repair cycle, and returns a validated, typed answer. It lives in native code, not behind an effect.
- **`ModelClient`**: How the orchestration layer views the model — a swappable boundary over a model API (frontier, hosted SLM, spawned agent, or replay). The caller trusts the shape of the response, not the generator, and the boundary is where record/replay and the vendor model id live.
- **Tool surface**: The typed callbacks the model pulls on within a brief — resolve the reference shelf, read/list/write the working tree, `verify` — so the model never holds a descriptor or an OS path ([RFC-53](rfc-53-tool-server.md)).
- **Handle**: A reference (like a file path or ID) that points to data without carrying it, keeping context windows lazy and small.

For context, here is how these concepts map to the tech stack:

| Concept               | Technology                                                                                  |
| --------------------- | ------------------------------------------------------------------------------------------- |
| Program / Guest       | A Wasm component (`wasm32-wasip2`)                                                          |
| Runtime / Interpreter | Omnia (a Wasmtime-based CLI binary)                                                         |
| Effect                | A WIT interface import (deterministic only)                                                 |
| Typed boundary        | WIT records and interfaces                                                                  |
| Handle                | A brief path, artifact path, or reference ID                                                |
| Native tool-use loop  | Native orchestration in the binary (model client + tool dispatch + `verify`)               |
| `ModelClient`         | A native trait over [`genai`](https://github.com/jeremychone/rust-genai) (frontier / hosted SLM / spawned agent / replay) |
| Tool surface          | `augentic:tools` ([RFC-53](rfc-53-tool-server.md)) — `resolve` / `read` / `list` / `write` / `verify` |
| Reference shelf       | The adapter-exported `references` interface (`resolve(id) → bytes`)                          |

## How the model reads a brief

The native loop hands the model one **whole brief** and a typed tool surface; how the model then pulls in supporting context depends on which `ModelClient` strategy is bound:

- **A filesystem-capable spawned agent** (like a headless Cursor or Claude Code session) is given the brief path and reads the prose directly, following relative links to supporting docs and pulling in only what it needs. This is the same lean-context behavior as before, now expressed as one model strategy rather than a runtime effect.
- **An API model** (a raw frontier or hosted-SLM endpoint with no filesystem) cannot follow a link or hold a path. So the loop advertises a tool surface ([RFC-53](rfc-53-tool-server.md)): the model emits a `resolve(id)` tool call for each of the brief's internal references, which the loop forwards to the adapter's exported **`references` shelf** (`resolve(id) → bytes`); it reads and mutates the working tree through `read` / `list` / `write`. The model never holds a descriptor or an OS path.

Either way the adapters remain a hybrid of Wasm and prose. The Wasm handles deterministic exports and the reference shelf, while the prose (the briefs and references) lives on disk, linked together naturally; the native loop is what binds the two to a model.

![Logical sequence: extract](../docs/assets/diagrams/effect-architecture/sequence-extract.svg)

The reference shelf is the adapter's, not the runtime's: `resolve` is a stateless, instance-per-call guest export, so a *computed* reference is served by a fresh instance and the runtime floor stays free of any reference-injection machinery.

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

Let's look at how a `build` operation flows in practice.

1. The native orchestration layer resolves the slice to a base revision and asks the host to materialize the slice's [working tree](#the-working-tree); the slice and its inputs stay pure, node-independent data, while the mutable tree is the one capability.
2. It opens a tool session bound to that base revision and runs any deterministic setup (a `tool` adapter export, called directly through its typed binding).
3. For the judgment leg it runs the **native tool-use loop**: it hands the model the `build` brief plus the tool surface (`resolve` / `read` / `list` / `write` / `verify`), through the bound `ModelClient`.
4. The model emits tool calls. `resolve` follows the brief's reference shelf (the adapter's `references` export); `read` / `list` scan existing code; `write` accumulates an edit. (A filesystem-capable spawned-agent strategy does the same reads and writes directly against the materialized tree.)
5. When the brief calls for it the model emits `verify(<check>)`; the loop runs that vetted, sandboxed profile and feeds the severity-tiered `report` back, and the model repairs and re-verifies ([RFC-53](rfc-53-tool-server.md)).
6. The model returns its final answer; the loop validates it against the operation's typed `report` and commits the session.
7. The report carries only judgment (status and findings); the native orchestration layer extracts the resulting mutations as a content-addressed `change-set` (a `git diff` against the base revision) and requests the lifecycle `transition` effect.

In short: deterministic control lives in guest exports and the native layer, the model is driven directly through a typed tool surface, references are loaded lazily through the shelf, and what crosses out is a typed report plus a content-addressed change-set. 

## The four laws

When evaluating future design decisions — whether something should be prose or code, what a function should take, or where it should run — keep this guiding principle in mind:

> Run every adapter and workflow as a guest on a runtime that only understands deterministic effects. Keep the structure in deterministic code, drive judgment through a native tool-use loop behind a swappable model boundary, and pass handles instead of raw text across boundaries.

These four laws apply across the entire architecture; if a proposed change conflicts with one of them, it's worth reconsidering the approach. The RFCs refer to them by number — most often *law 2*, the runtime floor's agnosticism.

1. **Typed boundaries**: WIT records are used for data and WIT interfaces for effects. Untyped text is not passed across boundaries.
2. **The runtime floor only knows about effects**: Omnia core doesn't know about workflows, adapters, or AI models — it only knows how to handle deterministic effects. Model knowledge (vendor SDKs, model ids, routing policy) is confined to the binary's native orchestration layer, behind the `ModelClient` boundary, and never reaches Omnia core.
3. **Determinism by default, judgment by exception**: Control flow lives in deterministic code. The native tool-use loop returns a typed, validated decision to steer the next deterministic step. Models do not guess control flow.
4. **Laziness is key**: Handles (like file paths) are passed across boundaries instead of massive text bodies, keeping context windows manageable and operations scalable.

## The model fleet and deployment modes

![The model fleet and deployment topologies](../docs/assets/diagrams/effect-architecture/model-fleet.svg)

Because every judgment run is self-contained (one whole brief, no long-lived conversation), the native loop can route each run to the right **`ModelClient` strategy** by difficulty and cost. The strategy is native configuration behind the `ModelClient` boundary — Omnia core never sees it. Several deployment modes fall out:

- **Interactive (Frontier LLMs)**: When triggered from an editor, the bound strategy is a frontier API (via `genai`) or a spawned headless agent session, run against concrete artifacts (not chat history) for complex synthesis and review.
- **Headless (Small Local Models)**: For fleet-scale operations and narrow transformations, the strategy is a hosted API or a local SLM with no editor in the loop. Constrained decoding ensures valid typed reports.
- **CI / Testing (Deterministic Replay)**: In CI, the bound strategy is the `Replay` `ModelClient` impl, turning AI evaluations into reliable regression tests by serving recorded `(brief + tool transcript) → answer` fixtures.

This setup enables progressive optimisation. As a specific transformation becomes well-understood, it can be migrated from an expensive frontier model down to a cheaper local SLM, or even to deterministic code, by changing the bound strategy — no guest rewrite. Across `genai` (frontier ↔ hosted ↔ SLM) this is a config change *inside* the `ModelClient`; only swapping to the spawned-agent shape is a distinct native strategy. Runtime-agnosticism is structural: the model id lives behind the `ModelClient` boundary, never in Omnia core. The fleet's full shape — router, frontier, spawned agent, SLM — is [RFC-58](rfc-58-eval-fleet.md).

## Host services and state

Guests are stateless and instance-per-call, so anything that must outlive a single call lives in a host service, not in guest memory. These are the *deterministic* effects — the counterpart to the native judgment loop above — and each is satisfied by a swappable backend the guest never sees:

- **Filesystem** (`wasi:filesystem`): Access to inputs, assets, and the project tree through standard WASI filesystem capabilities, restricted by the host. For an operation that mutates a pre-existing tree — notably `build` — the project tree is handed over as a [working tree](#the-working-tree) capability rather than a bare path, served by a **custom git-aware `wasi:filesystem` backend** (native, so git stays native) rather than a new bespoke host.
- **`state`**: Host-held scratch and memoization (e.g., caching a computed reference). uses KeyValue interface backed locally by filesystem, or Redis/NATS for fleet-shared state.
- **`journal`**: The durable lifecycle log and its legal moves. Uses `JsonStore` backed by a filesystem backend.

Because guests only interact with typed interfaces (like `KeyValue` or `wasi:filesystem`), the deployment topology is dictated entirely by the host backends. A local CLI binary wires these interfaces to the local filesystem. A cloud deployment wires the exact same interfaces to cloud-native infrastructure (like S3 for `wasi:filesystem` and Redis for `state`). The Specify Wasm components do not change.

### The working tree

A `build` does not generate into a green field — it generates a slice *into a pre-existing project*, reading existing code and conventions and writing changes back in place. The original contract leaked this as a `project-path` string: a single local path that the guest, the model, and core all assumed they shared. That assumption is the one thing that pins an operation to a single machine, so the contract models the tree as a host-materialized **working tree** capability instead of a path.

The host materializes the tree from a content-addressed **base revision** (a git commit, in the git backend) onto whichever node runs the operation — a local clone on a desktop, a fresh checkout or snapshot on a cluster node. The capability exposes two deliberately different faces:

- **A `wasi:filesystem` descriptor**, for deterministic guest (Wasm) code that reads or validates the tree through capability-scoped handles.
- **A host-reported node-local path** (`local-path`), for the one consumer that cannot hold a descriptor: the filesystem-capable **spawned-agent** model strategy ([RFC-58](rfc-58-eval-fleet.md)), which reads existing code and writes its changes through real OS paths. The native loop — the one party holding both the descriptor and the path behind it — resolves and provisions this path; the opaque descriptor yields none on its own. An absent path means no real local tree exists on this node, so an agent-driven build is unavailable there — a clean capability signal rather than a deep failure. (An API model never needs this path: it reads and writes through the [RFC-53](rfc-53-tool-server.md) tool surface instead.)

Provisioning that path is not a new mechanism: it is the same **host-as-path-broker** that already grants WASI extension tools their filesystem capabilities. The host substitutes a logical root (`$PROJECT_DIR`, `$CAPABILITY_DIR`, or the working tree), resolves it within the allowed roots (rejecting `..` and symlink escapes), then hands it across the boundary in whatever form the consumer can use — a capability-scoped `descriptor` for a sandboxed guest, or a literal path (process env or cwd) for an out-of-process consumer that cannot hold one. The tool-side rules are documented in [tool declarations](../docs/explanation/tool-declarations.md); the working tree's `local-path` is that broker applied to the agent, and graduates into that explanation when the runtime move ships.

This is the honest seam. The agent's read-modify-write loop is irreducibly node-local and path-based, so it is not abstracted away — it is *quarantined* between two portable boundaries: a host-materialized tree on the way in, and a content-addressed **change-set** (a delta of adds, modifies, and deletes against the base revision) on the way out. Neither operation *returns* that delta: the report carries only judgment (status and findings), and the caller's native orchestration layer extracts the change-set from the tree (a `git diff` against the base revision). The two target operations then use the capability symmetrically — `build` is lent the slice's tree and the caller extracts its delta, while `merge` is lent the *baseline* tree and folds a change-set (the build's output, the only representation portable enough to have crossed from another node) into it in place. The messy local mutation is confined to one node's scratch space, while what crosses the contract is the change-set, never a shared mount — which is what lets `build` and `merge` be dispatched to different nodes in a cluster. Git already provides exactly this content-addressing (commit ids, cheap diffs, a merge model), so it is the natural first backend without the contract ever naming a VCS. That backend rides Omnia's existing `wasi:filesystem` host as a **custom git-aware backend** — native code, so git stays native and there is no in-guest VCS — while `slice → revision` resolution, `change-set` extraction, and forge push live in the binary's native orchestration layer. The mechanism — custom-backend materialization, object acquisition (a `wasi:blobstore`-backed object cache on fleet nodes), and out-of-sequence dependency-layering — is specified in [RFC-55](rfc-55-working-tree.md).

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
- **S2 (Name the effects)**: Formalising the *deterministic* effects — `wasi:filesystem`, `kv`, and lifecycle events — as typed WIT imports, and standing up the native `ModelClient` boundary for judgment. This unlocks record/replay testing (captured at the `ModelClient` boundary) and retires the schema-drift surface.
- **S3 (Guests orchestrate)**: Deterministic tools become callable through the typed bindings; the native tool-use loop drives multi-step judgment operations, and adapters expose their reference shelf alongside their deterministic exports.
- **S4 (Runtime move)**: Replacing the bespoke `specify` host with the generic Omnia binary and running every guest — adapters and the workflow — on it. The runtime move itself is committed; *how much* of each workflow phase compiles into deterministic guest control flow, versus stays model-driven through the native loop, is a separate per-phase call.
- **Parallel (The model fleet)**: Building the native router and `ModelClient` strategies (frontier, hosted SLM, spawned agent, replay).

Stages S0–S3 are unconditional and the S4 runtime move is a committed bet; graduating any individual workflow phase into compiled guest control flow is opt-in and evidence-gated, not part of the committed path. For more details on the staging, see [roadmap.md](roadmap.md).

## Key trade-offs

A few deliberate bets are being made with this architecture:

- **Omnia is the sole runtime**: Provides flexible deployment modes — the *same* binary and the *same* guests run from desktop to cloud, with only the deterministic backends swapping (filesystem → S3, kv → Redis) — but creates a hard dependency on Omnia's host-service surface.
- **Judgment is a native dependency, not a runtime effect**: The judgment loop lives in the binary's native orchestration layer and reaches a model over its API (or spawns an agent), so the binary carries a model-client dependency and needs model egress (or a local model) at judgment time. It is deliberately not embedded inside a live editor session. The trade for moving judgment off a runtime effect: it runs wherever the native binary runs (desktop and cloud workers alike), but a *pure-wasi host with no native binary* (a browser, or wasi-serverless) needs the optional Mode-B tool-server guest ([RFC-53](rfc-53-tool-server.md)) to reach the same surface.
