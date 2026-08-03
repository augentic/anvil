# Emery on Omnia — The Effect-Oriented Architecture

> Status: This is the standing architecture — the agreed direction being worked toward. It sequences the work into independently valuable stages and lives alongside [roadmap.md](roadmap.md).
>
> Terminology (**runtime core**, Law 2, host-injected tools): [Omnia glossary](../../omnia/docs/glossary.md).

## The core idea

The architecture boils down to a single concept:

> The "Emery" CLI is Omnia compiled with Emery-specific Wasm guests.

Everything that follows is a consequence of this idea. The runtime hosts Wasm guests and satisfies a fixed vocabulary of typed effects; it holds no domain, workflow, or model knowledge. Emery's behaviour — orchestrating the workflow, extracting from sources, building for targets, and development tooling — lives in the guests and in the backends bound behind Omnia's host interfaces.

Context comes from artifacts, not conversational history. Each model evaluation is self-contained: a guest hands the model one **whole brief** and a typed tool surface scoped to concrete artifacts (like `spec.md` or a build request), never an accumulated chat transcript. This avoids the overloaded context windows that cause failures.

This approach provides three major benefits:

1. **Cloud-native portability**: Emery scales from a desktop CLI to a cloud service. Because the execution environment is abstracted behind Omnia's host interfaces, moving to the cloud swaps backends, not guest code.
2. **Scalability and auditability**: An operation runs identically whether triggered from an editor or a CI pipeline.
3. **Cost efficiency**: Because model evaluation is a typed call over specific inputs, a task can be routed to a frontier LLM, a small local model, or deterministic replay by swapping the model backend.



## The shape of the system

The system is two roles communicating over one contract: a generic runtime and the guests that run on it.

- **Omnia is the foundation**: a command-line executable that instantiates a guest and satisfies its typed effect imports from the backends bound for this deployment. It knows nothing about adapters, workflows, or models.
- **Everything else is a guest**: the engine (`plan`, `execute`), the adapters (source and target), and the development tooling all run as peers on the runtime.
- **Capabilities are host interfaces**: a guest reaches the outside world only by importing a host interface (`wasi:filesystem`, `wasi:keyvalue`, `wasi-model`, …) and calling it. Each interface is satisfied by a swappable backend.
- **The boundary is typed**: only typed records and handles cross it — never untyped text.

### Guest instantiation

A guest instance is created to serve exactly one trigger, then discarded. There are four triggers:

- an **HTTP request**,
- a **message on a topic** (NATS, Kafka),
- a **WebSocket call**, or
- a **CLI command** (`omnia <guest>.wasm <args…>`).

Guests hold no state between calls, so every trigger gets a **fresh instance**; the same holds for every host→guest callback. This is first a statelessness and isolation choice — it is what makes the runtime horizontally scalable and free of a whole class of aliasing complexity. It also sidesteps the one kind of reentrance the component model still traps: *recursive* reentrance, re-entering an instance already on the stack. (*Sibling* reentrance — a fresh task into a component whose other tasks are suspended — is business-as-usual under the component model's async ABI.)

### Calls in both directions

The runtime and guests interact both ways:

- **Guest → host**: the guest imports a host interface and calls it whenever it needs something impure — read a file (`wasi:filesystem`), cache a value (`wasi:keyvalue`), record a lifecycle event (`journal`), or evaluate a prompt (`wasi-model`).
- **Host → guest**: while servicing a guest call, a host may need content only a guest can produce. It instantiates a fresh guest and calls one of its exported functions. The `wasi-model` host does exactly this when it resolves a brief's references (below).

> **A quick note on naming:** "Omnia" refers to two things — the runtime itself, and the `omnia` target guest (the adapter that generates code for the Omnia runtime). This document is explicit when the context isn't obvious.

## The runtime: Omnia

Omnia is built on Wasmtime. Its design centers on pluggable host services behind typed interfaces, so a backend can be swapped without changing guest code. Three properties make this architecture possible:

- **One binary, guest-selected behaviour**: `omnia <guest>.wasm <args…>` runs, and the guest decides what to do. There is no bespoke `emery` host.
- **Instance-per-call execution**: a fresh instance spins up every time a guest is called, so a host→guest callback can never *recursively* re-enter an instance already on the stack — the one kind of reentrance the component model still traps (*sibling* reentrance, into a component whose other tasks are suspended, is allowed under the async ABI) — avoiding a class of aliasing complexity by construction.
- **Stateless guests, host-held state**: guests cannot hold state in memory between calls. Persistent data lives in a host service — filesystem-backed locally, or Redis / S3 in the cloud. This decoupling is what lets Emery move from a desktop tool to a horizontally scalable service unchanged.

Emery extends this surface in exactly one sanctioned way: **custom backends behind Omnia's host interfaces** — a git-aware `wasi:filesystem` backend that materializes the [working tree](#the-working-tree), and the **model backend** behind `wasi-model`. The model id and any vendor SDK live in that backend, never in the runtime core.

## Judgment: the `wasi-model` host

Model evaluation is a host capability like any other. Omnia exposes a `wasi-model` host whose `eval` export a guest calls to have a prompt evaluated:

```wit
// wasi-model host — judgment as a typed effect a guest imports
eval: func(prompt: prompt) -> result<answer, error>;
```

Behind the host sits a **swappable model backend**. The backend runs an LLM tool-use loop: it drives a model through its API, advertises a typed tool surface, dispatches the model's tool calls, runs the verify-repair cycle, and returns a validated, typed answer to the calling guest. The guest treats `eval` exactly like `wasi:keyvalue.get` — a typed call whose backend it never sees.

### Resolving references — the host calls back into a guest

A brief points at internal references (e.g. `../references/business-logic.md`). The model emits a `resolve` tool call for each; the backend follows it by selecting the relevant **adapter guest**, instantiating it, and calling its exported references:

```wit
// adapter references — the model backend calls this back into the guest
resolve: func(id: adapter-id, reference: reference) -> result<list<u8>, error>;
```

Because recursively re-entering a live instance would trap, this resolution lands in a **fresh adapter instance** every time — isolated from whatever guest called `eval`. The adapter's prose (briefs and references) is **embedded in its module at build time**, so `resolve` is an in-module lookup, not a host filesystem read. The references server is the adapter's, not the runtime's: a *computed* reference is served by a fresh instance, and the runtime core stays free of any reference-injection machinery.

Logical sequence: extract

The model reads and mutates a working tree through the same tool surface — `read` / `list` to scan existing code, `write` to accumulate an edit, `verify` to check itself — so it never holds a descriptor or an OS path. A filesystem-capable spawned-agent backend instead reads and writes the working tree directly through the `local-path` it is lent.

### The model backend is swappable

Swapping the model backend is how deployment modes are chosen; Omnia core never learns which model is bound:

- **Frontier / hosted** — a hosted inference API (via `[genai](https://github.com/jeremychone/rust-genai)`) for hard synthesis and review.
- **Spawned agent** — a fresh, context-free agent session for the filesystem-capable path.
- **Small local model** — a local SLM for narrow, high-volume transformations, with constrained decoding for valid typed reports.
- **Replay** — serves recorded `(prompt + tool transcript) → answer` fixtures, turning model evaluations into deterministic regression tests in CI.

Record/replay is a property of the backend boundary: a recording backend logs request→response around the live model; the replay backend serves them.

## Guest-to-guest interaction: host-mediated dynamic linking

A single operation spans several guests: the engine guest plus the source and target adapter guests it drives. Guests reach each other through **host-mediated dynamic linking** — never by composing them into one module ahead of time.

- **How it works**: the caller imports the per-axis host interfaces (`source` / `target`) and names a plan-bound `adapter-id` as the first argument of each call (`build(id, …)`, `survey(id)`, …) — the very interfaces the adapters export, so there is no separate dispatch facade to keep in sync. The Omnia host intercepts these imports through the Wasmtime `Linker` and issues a wRPC invocation to the named adapter's matching export (`emery:adapter/source` / `target`) over the bound transport.
- **The host's role**: the host selects the adapter **by identity**, instantiates a fresh, stateless instance, carries the typed WIT records to it over wRPC, invokes the exported function, and returns the typed result.
- **Why it fits**: it preserves strict WIT typing with no manual byte serialization, supports dynamic (config-driven, OCI-resolved) adapter selection, and enforces instance-per-call — so a dispatched call cannot recursively re-enter its caller. The `wasi-model` `eval → resolve` callback is this same mechanism applied by the model backend.

Because the interfaces (`target` / `source` / `references`) are statically known and only the adapter *instances* are dynamic, the host serves them with `wit-bindgen-wrpc`**-generated typed bindings** rather than wRPC's dynamic value-introspection path; the dynamic path remains available if an interface is ever unknown at host-compile time.

The seam is a contract, not a wire protocol: every selected call rides [wRPC](https://github.com/bytecodealliance/wrpc) — a WIT-native, transport-agnostic RPC backend that encodes the typed records (and their async `stream` / `future` values) — over whatever transport the deployment binds: an in-process or Unix-domain-socket transport on a single node, NATS or QUIC across a cluster. Moving from desktop to cloud is therefore a transport swap, not a code change. Plain records (`revision`, `changeset`, `input`, `report`, `lead`, `evidence`) cross by value; a live resource such as the [working tree](#the-working-tree)'s `descriptor` never crosses. [RFC-86](rfc-86-working-trees.md) settles local materialization; [RFC-91](rfc-91-node-sync.md) transports those values and re-materializes private trees remotely. wRPC stays behind the backend boundary — pinned and swappable, never in the `emery:adapter` contract — so the guest's view stays purely typed and the seam keeps a native in-process fast-path available if it is ever needed.

### Many guests, selected by identity

The binary holds every guest on **one runtime** and picks among them in native code. The registry boots with exactly one static entry — the engine guest, embedded in the binary as component bytes — and every adapter is admitted lazily by exact opaque identity through Emery's fail-closed `GuestResolver` ([RFC-71](rfc-71-deployment.md)):

```text
GuestRegistry  (one wasmtime::Engine + one Linker<StoreCtx>)
  "emery"                  -> InstancePre  (the command guest, embedded bytes —
                                              registered statically at boot)
  "source:typescript@0.5.0"  -> InstancePre  ┐  faulted in mid-run by exact routed id —
  "source:documentation"     -> InstancePre  │  store (pinned, pull-on-miss) or
  "target:omnia@0.5.0"       -> InstancePre  ┘  project cache (unpinned / bare)
  (any miss)                 -> GuestResolver (single-flight resolve-validate-register;
                                              unresolvable identities fail the dispatch)
```

Each call selects an `InstancePre` by identity from the registry — resolving it on first miss — instantiates a fresh instance on a new `Store`, calls the typed export, and discards it. **Identity is data, resolved by the host — not topology**: it arrives as an `adapter-id` call argument on the host-satisfied `source` / `target` imports, so one caller instance can drive many same-axis adapters in a loop. Two same-world adapters (two sources, two targets) are distinct registry entries, so there is no collision and no ahead-of-time composition. Which adapter a call targets comes from the operation's context:

- the `wasi-model` callback resolves against the adapter whose brief is being evaluated — its identity is fixed for the duration of that `eval`;
- an engine→target call (`build`, `merge`, `guidance`) targets the slice's bound target; an engine→source call (`survey`, `extract`) targets a bound source. Both bindings come from the plan.

The same select-by-identity resolves an **inbound trigger**, not only a guest-to-guest call:

- A CLI command names its guest directly (`omnia <guest>.wasm`), or — on the Emery product path — the runtime forwards raw argv to the statically-registered command guest (the embedded engine)
- An HTTP request carries no `adapter-id`, so the host derives the identity from the request and looks it up in the registry above. Emery's ordinary deployment uses one fixed projection (`/mcp/<axis>/<name>[@<version>]` → guest id via `launcher::mcp_route`; see [CLI architecture](../docs/contributing/cli-architecture.md)) — no authored `[[route.http]]` table. Only guests that **export** `wasi:http/incoming-handler` are routable: the host instantiates the matched entry fresh and invokes its handler, so a guest without that export stays reachable solely through the CLI trigger and host-mediated dynamic linking. A static prefix table remains available for hand-authored Omnia deployments (plain Omnia apps / optional file-flag manifests). Either way the dispatch is the same one every other trigger uses: select an `InstancePre` by identity, instantiate on a fresh `Store`, call the typed export, and discard.

## Lifecycle of an operation

Logical sequence: build

A `build` flows like this:

1. The engine guest resolves the slice to a base revision and asks the host to materialize the slice's [working tree](#the-working-tree); the slice and its inputs stay pure, node-independent data, while the mutable tree is the one capability.
2. It runs any deterministic setup (a `tool` adapter export, reached by host-mediated dynamic linking).
3. For the judgment leg it calls `wasi-model.eval` with the `build` brief.
4. The model backend drives the model. `resolve` follows the brief's references (the adapter's `references` export); `read` / `list` scan existing code through the working tree; `write` accumulates an edit.
5. When the brief calls for it the model emits `verify(<check>)`; the backend runs that vetted, sandboxed profile and feeds the severity-tiered `report` back; the model repairs and re-verifies.
6. `eval` returns the validated, typed answer to the guest.
7. The report carries only judgment (status and findings); the host extracts the resulting mutations as a content-addressed `change-set` (a `git diff` against the base revision), and the guest requests the lifecycle `transition` effect.

In short: deterministic control lives in guest code, judgment is a typed `eval` call, references load lazily through the references server, and what crosses out is a typed report plus a content-addressed change-set.

## The working tree

A `build` generates a slice *into a pre-existing project*, reading existing code and conventions and writing changes back in place. Modeling that tree as a bare `project-path` string is the one thing that pins an operation to a single machine, so the contract models it as a host-materialized **working tree** capability instead.

The host materializes the tree from a content-addressed **base revision** (a git commit, in the git backend) onto whichever node runs the operation — a local clone on a desktop, a fresh checkout on a cluster node. The capability exposes two faces:

- a `wasi:filesystem` **descriptor**, for deterministic guest code that reads or validates the tree through capability-scoped handles;
- a host-reported node-local `local-path`, for the one consumer that cannot hold a descriptor: the filesystem-capable **spawned-agent** model backend, which reads and writes through real OS paths. An absent path means no real local tree exists on this node — a clean capability signal that an agent-driven build is unavailable there.

The agent's read-modify-write loop is irreducibly node-local, so it is not abstracted away — it is *quarantined* between two portable boundaries: a host-materialized tree on the way in, and a content-addressed **change-set** (adds, modifies, deletes against the base revision) on the way out. Neither `build` nor `merge` returns the delta; the report carries only judgment, and the host extracts the change-set from the tree. `build` is lent the slice's tree and the caller extracts its delta; `merge` is lent the *baseline* tree and folds a change-set into it in place. What crosses the contract is the change-set, never a shared mount — which is what lets `build` and `merge` run on different nodes. Git provides exactly this content-addressing, so it is the natural first backend, carried as a **custom git-aware** `wasi:filesystem` **backend** (native code, so git stays native and there is no in-guest VCS). The mechanism is specified in [RFC-86](rfc-86-working-trees.md).

## Host services and state

Guests are stateless and instance-per-call, so anything that must outlive a call lives in a host service behind a swappable backend:

- `wasi:filesystem` — inputs, assets, and the project tree; the working tree is a custom git-aware backend.
- `wasi:keyvalue` (`state`) — host-held scratch and memoization (a computed reference, a model session's accumulating edits); filesystem locally, Redis / NATS for fleet-shared state.
- `journal` — the durable lifecycle log and its legal transitions; a JSON store over a filesystem backend.
- `wasi-model` — model evaluation, backed by a frontier API, a spawned agent, a local SLM, or replay.

Because guests interact only with typed interfaces, the deployment topology is dictated entirely by the bound backends. A local CLI wires these to the local filesystem and a model API; a cloud deployment wires the same interfaces to S3, Redis, and a fleet model backend. The guests do not change.

## The four laws

When evaluating a design decision — prose or code, what a function takes, where it runs — keep this principle in mind:

> Run every adapter and the engine as a guest on a runtime that understands only typed effects. Keep structure in deterministic guest code, reach the model through the `wasi-model` host behind a swappable backend, and pass handles instead of raw text across boundaries.

1. **Typed boundaries**: WIT records for data, WIT interfaces for effects. Untyped text is not passed across boundaries.
2. **The runtime core only knows effects**: Omnia core doesn't know about workflows, adapters, or models — only how to host guests and satisfy typed effects. Which backend satisfies an interface — including which model backs `wasi-model` — is deployment configuration the runtime core never sees.
3. **Determinism by default, judgment by exception**: control flow lives in deterministic guest code. `wasi-model.eval` returns a typed, validated decision that steers the next deterministic step. Models do not guess control flow.
4. **Laziness is key**: handles (file paths, reference ids) cross boundaries instead of corpora, keeping context windows small and operations scalable.

## Deployment modes

Because each evaluation is self-contained, the bound model backend is chosen per deployment (or per call, by a routing backend):

- **Interactive**: a frontier API or a spawned headless agent, run against concrete artifacts.
- **Headless**: a hosted API or a local SLM at fleet scale, no editor in the loop.
- **CI / testing**: the replay backend serves recorded fixtures, turning evaluations into reliable regression tests.

This enables progressive optimisation: as a transformation becomes well-understood, move it from a frontier model to a local SLM, or to deterministic code, by changing the bound backend — no guest rewrite.

## CLI bootstrapping

Because "Emery is Omnia compiled with Emery-specific backends," there is no separate runtime to download — the binary *is* the runtime, linked with its backends. The shipped `emery` binary is one `omnia::runtime!` command-mode invocation (`src/main.rs` — no handwritten `main`): the engine guest rides the macro's `guests:` key as embedded component bytes, `program:` forwards raw argv (minus the reserved host log flags `--debug` / `--quiet`, peeled into the host log preset), and the mounts and the fail-closed `GuestResolver` are expressions the `launcher` crate evaluates once per process. Every invocation runs in the engine guest — help, version, grammar rejections, and `adapter add` (over its read-only seed preopen) included — with envelopes and exit codes passing through verbatim. The host layer carries no Emery vocabulary; it reads only the macro's deployment keys. See [RFC-71](rfc-71-deployment.md) and [CLI architecture](../docs/contributing/cli-architecture.md).

The runtime admits adapter guests by **resolver-backed admission on first dispatch** ([RFC-71](rfc-71-deployment.md)): pinned identities resolve the **global single-file store** at `$HOME/.emery/store` (relocatable via `$EMERY_HOME`) with launcher pull-on-miss from the fixed first-party GHCR mapping ([RFC-76](archive/rfc-76-adapter-install.md)); bare names resolve local-first — project component cache seed, else newest installed store version, else pull-latest provisioning; component selectors resolve the project component cache only.

- **Adapter resolution at init**: `emery init` ensures the adapter needed for project scaffolding (pull-latest provisioning on a bare-name total local miss; `emery adapter upgrade <name>` is the explicit refresh). There is no separate adapter hydration command.
- **Engine embedded in the binary**: the engine guest ships as static component bytes inside the binary (`include_bytes!` over the artifact the root `build.rs` writes to `$OUT_DIR/emery.bin` — AOT-serialized in release, raw wasm in debug), registered at boot as the sole `wasi:cli/run` exporter — the binary version *is* the engine version, one knob, no store install and no first-launch fetch. Local engine iteration rebuilds the wasm32 product and the native build re-embeds it.
- **Deployment policy**: the `launcher` crate's expressions anchor the project root from argv and the working directory, capture the layout once, create the writable project / cache mount directories, derive the optional `adapter add` seed preopen, construct the fail-closed adapters-only `GuestResolver` with pull-on-miss, and bind the MCP HTTP listener + route hook ([RFC-71](rfc-71-deployment.md)). The global store is host-owned (no guest mount). Persisted `resolution.json` / deployment-doctor diagnostics remain RFC-71 Stage 2.

## Deferred relatives

Local value-backed working trees are step 1 of the platform-migration critical path — [RFC-86](rfc-86-working-trees.md), before single-node detached forge discovery, source selection, and ephemeral slots ([RFC-87](rfc-87-detached-changes.md)). [RFC-91](rfc-91-node-sync.md) later binds those completed local tree and change contracts to hosted multi-node execution. Host-owned verify profiles ([RFC-89](rfc-89-verify-profiles.md)) sit on the scale track. See [platform.md](platform.md) for the full sequence.

## Key trade-offs

- **Omnia is the sole runtime**: the *same* binary and guests run from desktop to cloud with only backends swapping (filesystem → S3, kv → Redis, model → fleet) — at the cost of a hard dependency on Omnia's host surface.
- **Model evaluation needs egress (or a local model) at** `eval` **time**: the binary carries a model-backend dependency; the replay backend covers CI, and a local SLM backend covers air-gapped runs.

