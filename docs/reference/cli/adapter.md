# emery adapter, source resolve, target resolve

The adapter component surface: seed the project component cache, upgrade installed adapters, and debug adapter resolution by axis.

## emery adapter add

Seed a local `.wasm` component into the project component cache so a bare (unpinned) binding resolves it.

```bash
emery adapter add <path.wasm> [--project-dir <dir>]
```

Mirrors the component to `<project-cache>/components/<name>.wasm` — the kebab-case name derives from the component filename (`source.wasm` → `source`, underscores fold to dashes, a `emery_` prefix is stripped) — and stamps a per-component provenance sidecar at `components/<name>.meta.yaml`.

The command is **pre-init** and **axis-neutral**: `.emery/` need not exist (seed first, then `emery init <name>` with the bare name), and the component's exports are not inspected — the binding that later resolves the name (the project target in `project.yaml`, a plan source in `plan.yaml`) supplies the expected axis, and a wrong-world component fails at the dispatch/metadata gate. Re-seeding the same name replaces the entry and its sidecar; the explicit command is the approval act.

Relative component paths anchor at `--project-dir`, which also selects the project the cache is keyed by; when the flag is absent, the project root is the nearest ancestor carrying `.emery/project.yaml`, falling back to the current directory pre-init. The seed runs in the engine guest like every other verb: the shipped binary's deployment policy projects the `adapter add` request from argv pre-boot and preopens the component's parent directory read-only under its absolute host path, so the guest opens the argv path unchanged — the component may live anywhere on the host, outside the project mounts.

This is the only local-component route into bare-name resolution besides a local component at init: there is no build-tree probe (`target/wasm32-wasip2/release/` is never consulted) and no sibling-checkout probe.

A seeded cache entry always wins bare-name resolution — the co-dev seed is never shadowed by a published component, including during an explicit upgrade. Without a seed, a bare name resolves local-first: the newest installed store version, else pull-latest provisioning from the fixed first-party registry.

## emery adapter upgrade

Explicitly refresh a bare adapter name — or every bare binding in the project — to the newest published version.

```bash
emery adapter upgrade <name> [--project-dir <dir>]
emery adapter upgrade --all [--project-dir <dir>]
```

Forces a registry check for each name: the runtime lists the first-party registry's tags (`ghcr.io/augentic/emery-adapters/<name>`), takes the newest exact-SemVer tag, and installs it into the global adapter store when it is newer than (or absent from) what is installed. This is the only routine path that consults the registry for an already-provisioned bare name — day-to-day resolution is local-first and never pulls. A registry failure during an upgrade is the typed `adapter-latest-failed`; a repository with no SemVer tags is `adapter-latest-none`.

`<name>` and `--all` are mutually exclusive, and one is required. `--all` collects every **bare** adapter binding the project records — the `project.yaml` target plus each `plan.yaml.sources.<key>` adapter — and upgrades them all in one invocation; pinned bindings are skipped, and an empty set (nothing bare bound) succeeds with `no bare adapter bindings to upgrade`. `--all` requires an initialized project (`.emery/project.yaml`); the named form does not.

A seeded project-cache entry is never shadowed: upgrading a name whose cache seed exists still resolves the seed (the store may still gain the newer version for other projects). Explicit pins (`emery:<name>@<semver>`) are not upgrade targets — re-pin instead.

## emery source resolve

Resolve a source adapter by identity and emit the wire-stable envelope.

```bash
emery source resolve <name>
```

Resolves the single `.wasm` component: the global store entry for a pinned identity; for a bare name, the seeded project component cache, else the newest installed store version (pull-latest provisioning only when nothing local exists). The `survey` and `extract` prompts are compiled into the adapter guest.

## emery target resolve

Resolve a target adapter by identity and emit the wire-stable envelope.

```bash
emery target resolve <value>
```

`<value>` may be a bare adapter name (`omnia`), a package reference (`emery:omnia@1.0.0`), or a local `.wasm` path. The `guidance`, `build`, and `merge` prompts are compiled into the adapter guest.

## Resolve envelope

Both resolve verbs emit `axis`, `name`, `version` (omitted for an unpinned resolve — the version settles host-side and is logged to stderr), `resolved-path`, `location` (`store` / `cache` for the component deployment), and `operations`.

## See also

- [Anatomy of an adapter](../../explanation/adapter-anatomy.md) — source vs target contract
- [Directory layout](../directory-layout.md) — the store, cache, and `EMERY_HOME`
- [Target adapters](../targets/index.md) — first-party target adapters
