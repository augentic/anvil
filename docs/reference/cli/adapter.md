# emery adapter, source resolve, target resolve

The adapter component surface: seed the project component cache and debug adapter resolution by axis.

## emery adapter add

Seed a local `.wasm` component into the project component cache so a bare (unpinned) binding resolves it.

```bash
emery adapter add <path.wasm> [--project-dir <dir>]
```

Mirrors the component to `<project-cache>/components/<name>.wasm` — the kebab-case name derives from the component filename (`source.wasm` → `source`, underscores fold to dashes, a `emery_` prefix is stripped) — and stamps a per-component provenance sidecar at `components/<name>.meta.yaml`.

The command is **pre-init** and **axis-neutral**: `.emery/` need not exist (seed first, then `emery init <name>` with the bare name), and the component's exports are not inspected — the binding that later resolves the name (the project target in `project.yaml`, a plan source in `plan.yaml`) supplies the expected axis, and a wrong-world component fails at the dispatch/metadata gate. Re-seeding the same name replaces the entry and its sidecar; the explicit command is the approval act.

Relative component paths anchor at `--project-dir`, which also selects the project the cache is keyed by; when the flag is absent, the project root is the nearest ancestor carrying `.emery/project.yaml`, falling back to the current directory pre-init. The seed runs in the engine guest like every other verb: the shipped binary's deployment policy projects the `adapter add` request from argv pre-boot and preopens the component's parent directory read-only under its absolute host path, so the guest opens the argv path unchanged — the component may live anywhere on the host, outside the project mounts.

This is the only route into bare-name resolution besides a local component at init: there is no build-tree probe (`target/wasm32-wasip2/release/` is never consulted) and no sibling-checkout probe.

A seeded cache entry also pins the ensure-time behavior of a bare name: `emery init <name>` and `emery plan author` bindings stay bare (and resolve the seed) when the cache hits, and auto-pin to the binary's embedded first-party adapter train (`emery:<name>@<train>`, pulled on miss) when it does not. Cache hits always win — the co-dev seed is never shadowed by a published component. Bare at resolve/dispatch time (the verbs below) remains cache-only and never pulls.

## emery source resolve

Resolve a source adapter by identity and emit the wire-stable envelope.

```bash
emery source resolve <name>
```

Resolves the single `.wasm` component: the global store entry for a pinned identity, else the seeded project component cache for a bare name. The `survey` and `extract` prompts are compiled into the adapter guest.

## emery target resolve

Resolve a target adapter by identity and emit the wire-stable envelope.

```bash
emery target resolve <value>
```

`<value>` may be a bare adapter name (`omnia`), a package reference (`emery:omnia@1.0.0`), or a local `.wasm` path. The `guidance`, `build`, and `merge` prompts are compiled into the adapter guest.

## Resolve envelope

Both resolve verbs emit `axis`, `name`, `version` (omitted for an unpinned cache resolve — a seeded component carries no package identity), `resolved-path`, `location` (`store` / `cache` for the component deployment), and `operations`.

## See also

- [Anatomy of an adapter](../../explanation/adapter-anatomy.md) — source vs target contract
- [Directory layout](../directory-layout.md) — the store, cache, and `EMERY_HOME`
- [Target adapters](../targets/index.md) — first-party target adapters
