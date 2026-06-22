# RFC-54: The Omnia Model Host (the `eval` Host Service and Backend Contract)

> Status: Draft (skeleton) · Repo: implemented in [augentic/omnia](https://github.com/augentic/omnia); authored here as the cross-repo companion to [RFC-56](rfc-56-runtime-move.md) · Provides: the model-host slot + backend plug-in contract [RFC-56](rfc-56-runtime-move.md) depends on and [RFC-57](rfc-57-eval-fleet.md) plugs into · Hosts: the Specify-owned `eval` interface ([RFC-52](rfc-52-effect.md))

## Abstract

This is the **cross-repo companion** to [RFC-56](rfc-56-runtime-move.md). The runtime move stands Specify's guests on the generic Omnia runtime, but it assumes one capability Specify cannot supply from its own side: a first-class, pluggable **model host** — the host service that satisfies the `eval` effect — and the **backend plug-in contract** that real model backends register into. This RFC specifies that Omnia-side capability: `eval` as "one more host service in Omnia's mould" ([architecture](architecture.md) — *"`eval` (the model service) as the marquee addition"*), the generic `ModelBackend` trait plus the inbound `resolve` callback, per-deployment backend binding (the same mechanism that swaps an in-memory KV for Redis), and the dispatch primitive that routes a guest's `eval` call to the bound backend. Omnia provides the **slot**; Specify owns the **interface** (RFC-52) and provides the **backends** (RFC-57). The slot carries zero vendor knowledge, so [law 2](architecture.md#the-four-laws) holds at the runtime floor.

## Motivation

RFC-56 lists "a host-service capability in `augentic/omnia`" as a hard dependency and names the extension point an `eval` backend plugs into without specifying it. This RFC is that specification, and it earns its own document for three reasons:

- **The model host is unusual among host services.** Unlike KV or SQL it *dispatches* to a swappable backend, it supports an **inbound** reference leg (the model calling back for prose), and it must be backend-selectable per deployment (real model service vs replay stub). That deserves an explicit contract, not an assumption buried in RFC-56.
- **It is where law 2 is won or lost at the floor.** If a vendor SDK or a model id leaks into Omnia core, "the runtime knows only effects" stops being true. A generic backend trait is the structural guard.
- **It is the seam two Specify RFCs lean on.** RFC-56 needs *a* model host to bind `eval` to; RFC-57's fleet backends need a *contract* to register into. Specifying it once, here, keeps both honest and lets the two repos version against a named boundary.

## Scope

**In scope (Omnia side):**

- Registering `eval` as a pluggable host service in Omnia's host-service framework, bound to the Specify-owned WIT interface ([RFC-52](rfc-52-effect.md)).
- The generic **`ModelBackend` trait** — the plug-in contract every concrete backend implements.
- The **`ReferenceResolver` callback** the host exposes to a backend — the host side of `resolve`, including the fresh-instance path for *computed* references and KV memoization.
- **Per-deployment backend binding/selection** (one backend per host), reusing Omnia's existing host-service backend-selection mechanism.
- The **dispatch primitive**: guest `eval` call → bound backend → typed result handed back for the guest to validate.
- The generic **host capabilities** a backend may need under deployment policy (subprocess spawn, network egress, filesystem read) — granted brain-agnostically.
- Forward-compatible (but not required) **async/streaming** host plumbing.

### Non-goals

- **The `eval` interface itself.** Specify owns and versions the WIT interface ([RFC-52](rfc-52-effect.md)); this RFC *hosts* it, it does not define it.
- **The concrete backends and the fleet router.** The frontier-API and spawned-agent backends and the difficulty/cost router are [RFC-57](rfc-57-eval-fleet.md); the replay backend is RFC-52; the SLM backend is [RFC-18](future/rfc-18-slm.md). Omnia provides the slot, never the fleet.
- **The deterministic effect host services.** Data, `kv`, and lifecycle ride the same generic framework; their backends are [RFC-56](rfc-56-runtime-move.md)'s subject (with working-tree materialization — this RFC's deterministic-effects mirror — carved out to [RFC-55](rfc-55-working-tree.md)), not this RFC's.
- **The generic host-service framework at large.** This RFC is scoped to the *model* host (the marquee addition); the interpreter and the conventional host plumbing are assumed Omnia machinery.
- **No vendor coupling in Omnia core.** No provider SDKs, no model ids, no router policy in the runtime — all of that lives in Specify-provided backends.

## The cross-repo boundary (who owns what)

| Concern                                                                         | Owner                                          |
| ------------------------------------------------------------------------------- | ---------------------------------------------- |
| The model-host **slot** in the host-service framework; registration + dispatch  | **Omnia** (this RFC)                           |
| The `ModelBackend` trait + `ReferenceResolver` callback                         | **Omnia** (this RFC)                           |
| Per-deployment backend binding; generic host capabilities (spawn / egress / fs) | **Omnia** (this RFC)                           |
| The `eval` **WIT interface** the host satisfies                                | **Specify** — [RFC-52](rfc-52-effect.md)      |
| The **replay backend** (zero-config member that proves the slot)                | **Specify** — [RFC-52](rfc-52-effect.md)      |
| The **model-service backend**: fleet router + frontier-API + spawned-agent      | **Specify** — [RFC-57](rfc-57-eval-fleet.md)  |
| The **SLM backend**                                                             | **Specify** — [RFC-18](future/rfc-18-slm.md)   |
| Binding `eval` during the runtime move (Specify-side adoption)                 | **Specify** — [RFC-56](rfc-56-runtime-move.md) |

The shared seam is the `ModelBackend` trait (Omnia-owned) plus the `eval` WIT (Specify-owned, authored in [`wit/model.wit`](../wit/model.wit) under the `augentic:model` package — `augentic:`-namespaced to match the Specify-ownership claim, with Omnia merely hosting it); the two are versioned across the repo boundary, never released in lockstep.

## The model (sketch)

There are two layers. The guest imports the Specify-owned `eval` **WIT interface** (RFC-52); Omnia's model host *implements that import* by forwarding to the deployment-bound **`ModelBackend`**:

```rust
// Omnia core — generic and brain-agnostic. No vendor SDKs, no model ids,
// no router policy. Specify provides the implementations.

/// The model host's plug-in contract. One backend is bound per deployment.
pub trait ModelBackend: Send + Sync {
    /// Satisfy one guest `eval` call. `brief_path` is a HANDLE: a
    /// filesystem-capable backend reads it directly and follows its links;
    /// a non-filesystem backend resolves references through `refs`. Returns
    /// the report's JSON projection; the host hands it back to the guest,
    /// which validates it against the operation's report type.
    fn eval(
        &self,
        brief_path: &Utf8Path,
        request: &str,
        refs: &dyn ReferenceResolver,
    ) -> Result<String, ModelError>;
}

/// The inbound (model-initiated) reference leg — the host side of
/// `resolve`. Backed by a host file-read, or a FRESH guest instance
/// for a computed reference (safe under RFC-56 instance-per-call), memoized in KV.
pub trait ReferenceResolver {
    fn load_reference(&self, id: &str) -> Result<Vec<u8>, ModelError>;
}
```

Omnia binds exactly one `Box<dyn ModelBackend>` per deployment, selected by config — the Specify **model service** for real work, or the **replay stub** for CI — the same selection mechanism that already swaps an in-memory KV for Redis. When a guest calls the imported `eval` interface, the host adapter forwards to the bound backend, passing a `ReferenceResolver` wired to Omnia's data/KV host services and the fresh-instance path for computed refs. **The fleet lives inside the Specify backend** ([RFC-57](rfc-57-eval-fleet.md)): Omnia sees one backend and honours one-backend-per-host; the router fans out *within* it.

## Decisions to record (open until reviewed)

- **Trait shape & async.** Whether `ModelBackend::eval` is sync (baseline) with an async variant gated behind streaming/concurrency (mirrors the architecture's narrowed-async bet), and whether the contract is a native Rust trait in Omnia or itself a WIT-component boundary.
- **`ReferenceResolver` surface.** Exactly what it exposes (load-by-id; brief-path resolution?), and how a *computed*-ref re-entry is scheduled onto a fresh guest instance.
- **Backend registration mechanism.** How a Specify backend is selected/registered at deployment (config key, plugin discovery, or compiled-in), reusing Omnia's host-service selection.
- **Capability exposure.** Which generic host capabilities (subprocess spawn for the agent backend, network egress for the API backend, filesystem read) the model host grants, and the per-deployment policy that governs them.
- **Recording seam.** Whether Omnia provides a generic record/replay *wrapper* around any backend (so any backend is recordable, consumed by RFC-52's replay backend) or recording is each backend's concern. Leaning: host-level wrapper.
- **Error taxonomy.** The `ModelError` shape and how it maps onto the WIT `error` Specify guests see.
- **Cross-repo versioning.** How the Omnia-owned trait and the Specify-owned `eval` WIT are versioned so neither repo blocks the other.

## Phased plan

1. Add the model host as a pluggable host service in Omnia bound to the RFC-52 `eval` interface; ship the **replay backend** as the first, zero-config `ModelBackend` to prove the slot end-to-end.
2. Add the `ReferenceResolver` callback — host file-read + fresh-instance computed refs + KV memoization — wired to RFC-56's data/KV host services.
3. Add per-deployment backend binding/selection and the generic capabilities (spawn/egress/fs) the Specify backends need; prove a real RFC-57 backend plugs in with **no Omnia change**.
4. *(Gated)* add async/streaming host plumbing if and when RFC-57 needs streaming output or concurrent slices.

## Acceptance criteria

1. `eval` is a pluggable Omnia host service bound to the Specify-owned WIT interface; a guest's `eval` import resolves to a deployment-bound `ModelBackend`.
2. At least the replay backend plugs in via the trait with **zero vendor code in Omnia core** — no model ids, no provider SDKs, no router policy (law 2 holds at the floor).
3. A non-filesystem backend resolves references through the host `ReferenceResolver`, with computed refs served by a **fresh** guest instance and memoized in KV.
4. Backend selection is per-deployment config; swapping replay ↔ a real backend needs no guest, interface, or Omnia-core change.
5. The fleet (RFC-57) plugs in as **exactly one** backend; Omnia binds one backend per host — no second `eval` binding.
6. [RFC-56](rfc-56-runtime-move.md) can bind `eval` to this host, closing its cross-repo dependency; the `augentic/omnia` workspace gate stays green, and this RFC's prose passes `make lint` here.

## Risks and invariants

- **A brain leaking into the runtime.** The chief risk is a vendor SDK or a model id creeping into Omnia core. The `ModelBackend` trait is the structural guard — every brain is a backend behind it, never a runtime assumption.
- **Cross-repo version skew.** The Omnia trait and the Specify `eval` WIT can drift; version the seam explicitly and treat it as a contract, not a shared build.
- **One-backend-per-host honoured.** The fleet must stay *inside* the single Specify backend; a second `eval` host binding is a regression of the architecture's one-backend-per-host rule.
- **Reentrancy via the resolver.** The inbound `resolve` leg must only ever re-enter on a fresh instance (RFC-56), never the operation instance suspended in `eval`.
- **Async stays gated.** The host must not force async on backends that do not stream; the synchronous contract is the baseline.
