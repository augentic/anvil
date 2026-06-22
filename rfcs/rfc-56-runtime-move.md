# RFC-56: The Omnia Runtime Move (Stage 4 — Retire the Bespoke Host)

> Status: Draft (skeleton) · Implements: the effect-oriented architecture (Stage 4 — runtime) · Depends: RFC-52 (deterministic effects), sequenced after RFC-54 (orchestration proven on the adapter axis) · Absorbs from RFC-51: the component-on-both-axes mandate · Cross-repo: the Omnia generic floor in [augentic/omnia](https://github.com/augentic/omnia) (no model host — judgment is native, [RFC-53](rfc-53-tool-server.md)) · Binds: the working-tree backend behind `wasi:filesystem` ([RFC-55](rfc-55-working-tree.md)) · Gates: [RFC-57](rfc-57-specify-guests.md) (workflow and development as guests)

## Abstract

This is the keystone of the [effect-oriented architecture](architecture.md): the move that makes the one-idea sentence literally true — *"Specify is the binary resulting from Omnia compiled with Specify-specific backends."* [RFC-52](rfc-52-effect.md) named the deterministic effect vocabulary but left every effect *initially backed by the machinery that exists today* (the prepare/finalize handoff, the bespoke CLI). This RFC replaces that scaffolding with the real runtime: the generic `omnia <guest>.wasm <args…>` binary, instance-per-call execution, stateless guests with host-held state, and **real Specify-specific backends** for the deterministic effects — the working tree (a custom git-aware `wasi:filesystem` backend; [RFC-55](rfc-55-working-tree.md)), `kv`, and lifecycle (`journal` / `transition`), each a custom backend behind one of Omnia's *existing* general-purpose hosts. Specify adds **no new generic host**: judgment is not an effect but the binary's native tool-use loop ([RFC-53](rfc-53-tool-server.md)), which ships *with* the binary alongside the other native orchestration. The bespoke `specify` wasm host retires; what is left is a generic interpreter, a set of swappable host-service backends, and the native orchestration layer (working tree, judgment loop, forge push).

## Motivation

After RFC-52 the effects are typed and mockable but still satisfied by the old host. That leaves the architecture's headline properties aspirational rather than structural:

- **Agnosticism is not yet structural (law 2).** While a bespoke `specify` binary owns dispatch, "the runtime knows only effects" is a posture, not a fact. Standing the guests on a generic runtime makes it a fact.
- **Statelessness is not yet load-bearing.** Instance-per-call execution and host-held KV state are what make the runtime horizontally trivial; today's host does not enforce them.
- **Deployment modes are not yet "just backend swaps."** They become swaps only once the deterministic host-service surface is the single seam every backend plugs into — with judgment a native dependency that ships with the same binary, desktop to cloud.

This RFC is the engineering behind the architecture's "Omnia as the runtime — committed" bet. It is deliberately *not* the model fleet (that is [RFC-58](rfc-58-eval-fleet.md)) and *not* the workflow-as-guest move (that is [RFC-57](rfc-57-specify-guests.md)); it is the floor both of those stand on.

## Scope

**In scope:** the generic `omnia <guest>.wasm <args…>` invocation surface and guest selection; instance-per-call execution; the real host-service backends for the deterministic effects (data, `kv`, lifecycle) — with working-tree materialization, the substantial sub-contract delivered as a custom `wasi:filesystem` backend, carved out to [RFC-55](rfc-55-working-tree.md); the `kv` backend set (filesystem · Redis · NATS) that holds what stateless guests cannot; binding one backend per host per deployment; retiring the bespoke `specify` wasm-host dispatch on the adapter axis; and the **component-on-both-axes mandate** (relocated from RFC-51) — every source and target adapter ships a WASM component implementing its axis world, co-packaged with its prose as one composite extension, ending the prose-only adapter now that guests are instantiated generically.

### Non-goals

- **The model fleet is out of scope.** The `ModelClient` strategies, the router, and the deployment topologies are [RFC-58](rfc-58-eval-fleet.md); this RFC only requires that the native loop's `ModelClient` boundary exists (satisfied at minimum by the `Replay` strategy).
- **The workflow guest is out of scope.** Moving `/spec:plan` / `/spec:execute` onto the runtime is [RFC-57](rfc-57-specify-guests.md); through this RFC the workflow may keep running on the legacy driver behind a shim.
- **No adapter-orchestration redesign.** RFC-54's component orchestration is consumed unchanged; this RFC swaps the *host*, not the guests.
- **No effect-vocabulary change.** The interfaces are RFC-52's; this RFC supplies their real backends.

## The cross-repo boundary

The runtime is a separate project, so this RFC is genuinely two coordinated halves:

- **Omnia (the repo) provides the generic floor** — the Wasmtime-based interpreter, the pluggable host-service framework (the same mechanism that swaps an in-memory KV for Redis), and the general-purpose host traits (`wasi:filesystem`, `wasi:blobstore`, `kv`, …) that Specify's deterministic-effect backends plug into. It adds **no model host** — judgment is native ([RFC-53](rfc-53-tool-server.md)) — so it carries zero Specify domain knowledge and zero model knowledge.
- **Specify provides the backends and the native orchestration** — the concrete host-service implementations for its deterministic effect vocabulary, the native judgment loop (model client + tool dispatch + `verify`), the guest packaging, and the operator CLI surface. "Specify" *is* Omnia compiled with these.

**Who owns the effect WIT.** The deterministic effect interfaces are Specify's host services, not Omnia's, so Specify owns and versions them; Omnia owns only the generic framework that hosts them. This keeps the runtime brain- and domain-agnostic by construction — and with judgment native rather than a hosted effect, there is no model-host capability to coordinate across the repo boundary at all.

## The model (sketch)

`omnia <guest>.wasm <args…>` instantiates a fresh guest instance, forwards the remaining arguments for the guest to interpret, and satisfies the guest's effect imports from the backends bound for this deployment:

- **Data / lifecycle** — backed by real host services over the project tree and the lifecycle store, replacing the handoff/CLI scaffolding RFC-52 stood up. The working-tree backend — a **custom git-aware `wasi:filesystem` backend**, native so git stays native (its contract is [RFC-55](rfc-55-working-tree.md)) — is what materializes the `working-tree` an operation edits — a local clone on a desktop, a fresh checkout or snapshot on a cluster node — so the content-addressed `change-set` the caller extracts from it (RFC-52) is what distributes between `build` and `merge`, not a shared mount.
- **`kv`** — backed by filesystem locally, Redis / NATS when a fleet shares state; this is where the `references` shelf memoizes computed references and where any deterministic sub-result is cached.
- **Judgment** — *not* an effect bound here. The native tool-use loop ([RFC-53](rfc-53-tool-server.md)) ships with the binary and reaches a model through the `ModelClient` boundary; the `Replay` strategy suffices until [RFC-58](rfc-58-eval-fleet.md).

Each call is a new instance (component instances are not re-entrant), so the `references` shelf — a stateless guest export the native loop calls to resolve a *computed* reference — lands in a fresh instance every time, with no async machinery.

**Every adapter is a component now (relocated from RFC-51).** Standing the guests on the generic `omnia <guest>` surface is the point at which shipping a WASM component stops being optional: the runtime instantiates a component per call, so an adapter with no component cannot be a guest. Both axes therefore ship the composite's wasm half — including the agent-only source adapters (`intent`, `documentation`, `typescript`, `screenshots`, `captures`), which export the `source` interface (per `world source-adapter`) even though their `survey` / `extract` may still be satisfied through the native tool-use loop rather than deterministic component code. RFC-51 authored the axis world; this stage is where *implementing* it becomes mandatory, because the toolchain cost only buys something once the generic runtime is the thing instantiating the component.

## Decisions to record (open until reviewed)

- **Operator CLI surface.** Whether operators keep typing `specify <verb>` against a thin shim that forwards to `omnia <guest> <args…>`, or the raw runtime surface is exposed. Pre-1.0 posture favours a hard cut over a long-lived compatibility shim.
- **Effect-WIT ownership & versioning.** Specify owns the deterministic effect interfaces (above), all under the `augentic:specify` package ([`wit/specify.wit`](../wit/specify.wit), RFC-51) — including the `references` shelf. There is no separate `augentic:model` / `eval` package: judgment is native, not a WIT effect. Still open: how the `augentic:specify` package version relates to the host floor (RFC-47).
- **KV backend selection.** How a deployment picks filesystem vs Redis vs NATS, and the key-namespace discipline that keeps memoization correct across guests and versions.
- **Diagnostics / lint / validate placement.** How `specify-diagnostics`, `validate` (lifecycle-gating), and `lint` ride the move without changing their authority.
- **Migration shape.** The sequence by which the bespoke host's adapter dispatch is retired (a hard cut at the extension chokepoint, consistent with RFC-51's ABI cut).

## Phased plan

1. Stand up the generic `omnia <guest>.wasm` binary running one target-adapter guest, with real data + lifecycle backends replacing the RFC-52 handoff scaffolding (no behaviour change versus RFC-54).
2. Add the `kv` host service + its backend set; route the `references` shelf's memoization through it.
3. Retire the bespoke `specify` wasm-host dispatch on the adapter axis; both axes now run via `omnia <guest>`.
4. Leave the workflow on the legacy driver/shim until [RFC-57](rfc-57-specify-guests.md).

## Acceptance criteria

1. Source and target adapter guests run via `omnia <guest>.wasm <args…>`; the bespoke `specify` wasm host is gone on the adapter axis.
2. The deterministic effects (data, `kv`, lifecycle) are satisfied by real Omnia host-service backends, not the prepare/finalize handoff.
3. Execution is instance-per-call; no durable in-process guest state — what persists lives in `kv`.
4. The runtime floor holds zero adapter names, zero workflow knowledge, and zero model knowledge (law 2); the no-adapter-names guard from RFC-50 still passes.
5. A run is still recordable / replayable end-to-end (the record/replay property is preserved across the host swap, captured at the `ModelClient` boundary — [RFC-53](rfc-53-tool-server.md)).
6. Every source and target adapter ships a WASM component implementing its axis world, co-packaged with its prose as one composite extension; there are no prose-only adapters.
7. `make lint` and `cargo make ci` stay green.

## Risks and invariants

- **Cross-repo sequencing.** The Omnia framework capability and the Specify backends must land in a deliberate order; the WIT contract is the seam that lets them be versioned independently.
- **Operator-UX disruption.** Retiring the bespoke binary changes the operator surface; the shim decision above bounds the blast radius.
- **Backend correctness.** KV memoization and the data host service must be correct under instance-per-call concurrency; statelessness is the invariant that makes this tractable.
- **Law 2 preserved.** The move must not smuggle domain or model knowledge into the runtime; everything Specify-specific lives in the backends, the guests, and the native orchestration layer (including the judgment loop and its `ModelClient` boundary), never in the generic floor.
- **Toolchain cost (relocated from RFC-51).** Components + `wit-bindgen` add build steps for every adapter author, now including the agent-only source adapters that previously shipped prose only. This is the principal adoption cost of the move; it is borne here because the generic runtime is what makes a component mandatory rather than merely available.
