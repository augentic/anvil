# specify adapter, source resolve, target resolve

The adapter component surface: seed the project component cache and debug adapter resolution by axis.

## specify adapter add

Seed a local `.wasm` component into the project component cache so a bare (unpinned) binding resolves it.

```bash
specify adapter add <path.wasm> [--project-dir <dir>]
```

Mirrors the component to `<project-cache>/components/<name>.wasm` — the kebab-case name derives from the component filename (`mock_source.wasm` → `mock-source`, a `specify_` prefix is stripped) — and stamps a per-component provenance sidecar at `components/<name>.meta.yaml`.

The command is **pre-init** and **axis-neutral**: `.specify/` need not exist (seed first, then `specify init <name>` with the bare name), and the component's exports are not inspected — the binding that later resolves the name (the project target in `project.yaml`, a plan source in `plan.yaml`) supplies the expected axis, and a wrong-world component fails at the dispatch/metadata gate. Re-seeding the same name replaces the entry and its sidecar; the explicit command is the approval act.

Relative component paths anchor at `--project-dir` (default: the current directory), which also selects the project the cache is keyed by. In the shipped binary the deployment launcher performs the seed itself before the runtime starts — the component path may live anywhere on the host, outside the engine guest's mounts.

This is the only route into bare-name resolution besides a local component at init: there is no build-tree probe (`target/wasm32-wasip2/release/` is never consulted) and no sibling-checkout probe.

## specify source resolve

Resolve a source adapter by identity and emit the wire-stable envelope.

```bash
specify source resolve <name>
```

Resolves the single `.wasm` component: the global store entry for a pinned identity, else the seeded project component cache for a bare name. The `survey` and `extract` prompts are compiled into the adapter guest.

## specify target resolve

Resolve a target adapter by identity and emit the wire-stable envelope.

```bash
specify target resolve <value>
```

`<value>` may be a bare adapter name (`omnia`), a package reference (`specify:omnia@1.0.0`), or a local `.wasm` path. The `guidance`, `build`, and `merge` prompts are compiled into the adapter guest.

## Resolve envelope

Both resolve verbs emit `axis`, `name`, `version` (omitted for an unpinned cache resolve — a seeded component carries no package identity), `resolved-path`, `location` (`store` / `cache` for the component deployment), and `operations`.

## See also

- [Anatomy of an adapter](../../explanation/adapter-anatomy.md) — source vs target contract
- [Directory layout](../directory-layout.md) — the store, cache, and `SPECIFY_HOME`
- [Target adapters](../targets/index.md) — first-party target adapters
