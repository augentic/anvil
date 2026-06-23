# RFC-56: The Runtime Move — the generic Omnia binary and the multi-guest registry

> Status: Draft · Order 6 of 8 · Stage S4 · Depends: [RFC-52](rfc-52-effect.md), [RFC-54](rfc-54-orchestration.md) · Binds: [RFC-55](rfc-55-working-tree.md) · Enables: [RFC-57](rfc-57-specify-guests.md) · Owns: the runtime and guest selection

## Abstract

This is the keystone: the move that makes "Specify is Omnia compiled with Specify-specific backends" literally true. The generic `omnia <guest>.wasm <args…>` binary replaces the bespoke `specify` host. It instantiates guests per call, satisfies their typed effects ([RFC-52](rfc-52-effect.md)) from bound backends — `wasi:filesystem` (the working-tree backend, [RFC-55](rfc-55-working-tree.md)), `wasi:keyvalue`, lifecycle, and the `wasi-model` model backend ([RFC-53](rfc-53-wasi-model.md)) — and holds **many guests at once** in a registry, selecting among them by identity through host-mediated dynamic linking.

## The model

- **Four instantiation triggers.** A guest instance serves one trigger then is discarded: an HTTP request, a topic message (NATS / Kafka), a WebSocket call, or a **CLI command** (`omnia <guest>.wasm <args…>`).
- **Instance-per-call.** Component instances are not reentrant; every trigger and every host->guest callback gets a fresh instance on a new `Store`.
- **The multi-guest registry.** One `wasmtime::Engine` and one `Linker` provide every host interface once. A registry maps guest identity -> a pre-instantiated component (`InstancePre`): `workflow`, `source:<id>`, `target:<id>`. Each call selects an `InstancePre` by identity, instantiates fresh, calls the typed export, and discards it.
- **Host-mediated dynamic linking.** A caller reaches an adapter through the host `selection` interface, naming a plan-bound `adapter-id` per call (`build(id, …)`, `survey(id)`, …); the host looks the identity up in the registry, instantiates a fresh instance, marshals the typed records with bindgen closures, invokes the adapter's `source` / `target` export, and returns the typed result. **Identity is data at the call site** — a plan binding the caller carries (a slice's bound source / target), or, for the `eval` `resolve` callback, the adapter whose brief is being evaluated (fixed for that `eval`). Because the id is a call argument, one caller instance fans out across many same-axis adapters in a loop (e.g. `survey` over every bound source) without re-instantiation — the deterministic for-each stays in guest code. There is no ahead-of-time composition; two same-world adapters are distinct registry entries, so they cannot collide.
- **Guest acquisition.** Core guests (the workflow) embed in the binary (`include_bytes!`) for offline, zero-skew startup; adapters resolve lazily by digest from an OCI store into a local cache — only the identities a plan binds are instantiated.

**Transport is pluggable behind the seam.** `selection` is a typed contract, not a wire protocol — the host decides how each selected call reaches its callee. In-process host-mediated dynamic linking is the default (local, synchronous); when the callee lives on another node the host carries the same `build(id, …)` over a messaging backend — `wasi:messaging` request-reply (NATS) — owning the serialize -> request -> await -> deserialize round-trip. Either way the guest exchanges only typed WIT records, so the typed-boundary guarantee holds and deployment topology stays a function of the bound backends, not the guest code.

## The component mandate

Standing the guests on the generic runtime is the point at which shipping a WASM component stops being optional: the runtime instantiates a component per call, so an adapter with no component cannot be a guest. Both axes ship a component implementing their world (and the `references` shelf), including the agent-only source adapters (`intent`, `documentation`, `typescript`, `screenshots`, `captures`), whose `survey` / `extract` may still be satisfied through `eval` even though the world exports the interface.

## The cross-repo boundary

- **Omnia** provides the generic floor: the Wasmtime interpreter, the pluggable host-service framework, and the general-purpose host interfaces — `wasi:filesystem`, `wasi:keyvalue`, `wasi:blobstore`, `wasi-model` (`eval`). It carries zero Specify domain and zero model knowledge.
- **Specify** provides the backends (working tree [RFC-55](rfc-55-working-tree.md), model [RFC-58](rfc-58-model-backends.md), kv / lifecycle), the guests, the registry, and the operator CLI.

## Scope

- The generic `omnia <guest>.wasm <args…>` surface and the CLI trigger.
- Instance-per-call execution and the multi-guest `InstancePre` registry keyed by identity.
- Host-mediated dynamic linking through the `selection` interface — per-call `adapter-id` (from plan / session context) resolved against the registry.
- Binding real backends for the deterministic effects and the `wasi-model` host.
- The component-on-both-axes mandate; retiring the bespoke `specify` host.

## Acceptance criteria

1. Workflow and adapter guests run via `omnia <guest>.wasm <args…>`; the bespoke `specify` host is gone.
2. Multiple guests are co-resident on one Engine + Linker, selected by identity; two same-world adapters resolve without collision and without composition.
3. The deterministic effects and `wasi-model` are satisfied by real backends; execution is instance-per-call with no durable in-guest state.
4. The runtime floor holds zero adapter names, zero workflow knowledge, and zero model knowledge.
5. Every source and target adapter ships a WASM component implementing its world; there are no prose-only adapters.
6. `make lint` and `cargo make ci` stay green.

## Risks and invariants

- **Cross-repo sequencing.** The Omnia framework capability and the Specify backends land in order; the WIT seams version independently.
- **Statelessness is load-bearing.** Instance-per-call concurrency requires the kv and data backends to be correct; what persists lives in host services.
- **Law 2 preserved.** Everything Specify-specific lives in backends, guests, and native orchestration — never in the generic floor.
- **Toolchain cost.** Components + `wit-bindgen` add a build step for every adapter author, including the agent-only source adapters; this is the principal adoption cost, borne here because the generic runtime is what makes a component mandatory.
