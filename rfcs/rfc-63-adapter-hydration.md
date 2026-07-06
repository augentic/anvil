# RFC-63: Adapter Hydration and the Central Store

> Status: Proposed · Depends: [RFC-61](rfc-61-omnia-migration.md) (guests by path, the generated deployment), RFC-47/48 (adapter identity, OCI transport, the global store — landed; see the CLI workspace's [DECISIONS.md](../DECISIONS.md#rfc-48-adapter-store-and-transport)) · Owns: how `specify init` turns a `project.yaml` into a fully provisioned, runnable deployment — locally and in the cloud

## Abstract

Post-cutover, `specify` is an Omnia `runtime!` binary that co-loads the workflow guest plus one wasm component per bound adapter, and Omnia loads guests **by path**. Today the only deployment manifest is the in-tree `omnia.toml` pointing at a sibling `specify-adapters` checkout — a developer posture, not a consumer one. This RFC owns the consumer path: `specify init` reads the project's declared adapter identities, downloads each missing adapter package into the **central store at `$HOME/.specify/adapters/<name>@<version>/`**, and generates the deployment manifest from the resolved store entries. Hydration is one idempotent, concurrency-safe, non-interactive kernel with three triggers — init, plan-time source binding, and an explicit sync verb — so the same mechanism provisions a laptop, a CI runner, and a cloud agent with only an environment variable between them.

## What already exists

Most of the machinery landed with RFC-47/48; this RFC composes it rather than re-inventing it:

- **Identity.** An adapter is `name@<semver>` (`AdapterRef`), required on every manifest; `project.yaml.adapter` records the target's pinned identity, and `plan.yaml.sources.<key>` carries an optional per-binding version pin.
- **Packaging and transport.** `specify-registry` packs an adapter tree — prose and the built component together — as one content-addressed `tar+zstd` OCI layer, pushed and pulled under the immutable `${SPECIFY_REGISTRY:-augentic.io}/<namespace>/<name>:<version>` reference.
- **The store.** `store::install_tofu` materializes a pulled layer as a read-only entry keyed by `(name, version)`, published by atomic temp-then-rename under a blocking sibling install lock, with a recorded tree-content digest verified on every resolve (`adapter-digest-mismatch`).
- **Resolution order.** `locate_axis` probes the store first for a pinned identity, then the per-project manifest cache, then the vendored `adapters/` tree; a miss everywhere is `adapter-not-found`. Resolution is project-local plus the store — no environment-variable fallback to a framework checkout.

The store root today resolves `$SPECIFY_ADAPTER_STORE` → `$XDG_CACHE_HOME/specify/adapters` → `~/.cache/specify/adapters` (`crates/schema/src/cache.rs`). Install happens only when the operator hands `specify init` a `specify:<name>@<semver>` package ref; nothing reads `project.yaml` and hydrates, and nothing generates the deployment manifest the RFC-61 runtime needs.

## The store root moves to `$HOME/.specify/adapters`

The store is an **install store, not a disposable cache**: entries are immutable, digest-verified, and load-bearing at runtime (the deployment manifest references `guest.wasm` inside them; evicting one bricks every project pinned to it). XDG cache semantics invite eviction, and cloud runners want one obvious directory to persist or mount. The root therefore becomes:

1. `$SPECIFY_ADAPTER_STORE` (unchanged — the relocation lever for sandboxes and tests),
2. else `$HOME/.specify/adapters`.

Layout is unchanged: `<root>/<name>@<version>/` entries with sibling `<name>@<version>.meta` digest sidecars and `.lock` install locks. `$HOME/.specify/` becomes Specify's per-user home (the store today; a natural later home for auth and channel config), distinct from the per-project `.specify/` system-of-record and the per-project derived cache under `$XDG_CACHE_HOME/specify/projects/`, which stay where they are.

## The hydration kernel

One kernel, `hydrate(refs) -> resolved set`: for each pinned `name@<semver>`, probe the store; on a miss, pull the OCI layer and `install_tofu` it; verify the digest; return the resolved entry paths. Properties the kernel guarantees, all inherited from the RFC-48 substrate:

- **Idempotent.** A warm store makes hydration a no-op probe per adapter — cheap enough to run on every invocation.
- **Concurrency-safe.** Parallel hydrators of one identity serialize behind the blocking install lock; losers find the entry materialized and move on.
- **Non-interactive.** No prompts, no credential dialogs: registry auth comes from the environment (standard OCI credential resolution), and any failure is a typed error naming the identity and the probe that failed.
- **Exact pins only.** Hydration never resolves a version range or a bare name over the network — an unpinned name keeps today's project-local resolution, and network version-resolution stays deferred (RM-21). Determinism in the cloud comes from pins, not from "latest".

Three triggers share the kernel:

- **`specify init`** hydrates the target identity recorded on `project.yaml.adapter`, plus every identity in the new optional `project.yaml.adapters:` prefetch list (both axes, pinned) — so a project that knows its source set up front provisions everything in one command. Init then generates the deployment manifest (below). `specify init --upgrade` re-runs hydration against the (possibly re-pinned) declared set.
- **Plan-time source binding** hydrates on demand: when `plan.yaml.sources.<key>` binds a pinned adapter the store lacks, plan validation invokes the kernel instead of failing — the lazy path for sources the operator didn't prefetch.
- **`specify adapters sync`** is the explicit verb: read `project.yaml` (and `plan.yaml` when present), hydrate every declared identity, print the resolved set with per-adapter store paths and digests. It is the one-line cloud bootstrap and the operator's cache-priming and diagnosis surface. `--frozen` turns any would-be fetch into a typed failure (`adapter-not-installed`) for offline and reproducibility-strict CI.

## Deployment manifest generation

The runtime binary stops reading a hand-authored `omnia.toml` in consumer projects. After hydration, the CLI **generates** the deployment manifest into the per-project derived cache (out-of-tree, per the cache-layout decision): one `[[guest]]` per resolved adapter pointing at `<store>/<name>@<version>/guest.wasm`, the `[[mount]]` of the project directory as writable `"."`, one `[[route.http]]` MCP prefix per adapter, and the workflow guest's link allow-list — exactly the shape the in-tree developer manifest models today. The manifest is a derived artifact: regenerated whenever the declared adapter set or pins change, never committed, never hand-edited.

The workflow guest itself ships **with the CLI release**, not through the adapter store: the runtime binary embeds its component bytes and materializes them beside the generated manifest (Omnia loads by path), keyed by the CLI's own version. One binary therefore carries the whole non-adapter half of the deployment, which is what makes `specify init && specify plan …` work on a fresh machine with nothing but the binary and network access.

## Cloud posture

The cloud story is the local story with the knobs exposed, not a second mechanism:

- **Relocatable root.** `SPECIFY_ADAPTER_STORE` points the store at a mounted volume or a restored cache directory; nothing else changes. `$HOME/.specify/adapters` is the default that makes the unmounted case still work.
- **Cache priming.** CI restores the store directory keyed on the project's pinned identities (a digest over `project.yaml.adapter` + `adapters:` + plan source pins is a stable cache key); `specify adapters sync` after restore is a no-op probe on a hit and a fetch on a miss. `--frozen` converts "miss" into "fail loudly" where fetching is forbidden.
- **Cross-machine digest pinning.** RFC-48 left the trust model at TOFU per machine. Hydration closes the gap with a committed lock: `.specify/adapters.lock` records each identity's tree-content digest at first install, and every subsequent hydration — any machine — verifies the unpacked entry against the committed digest before use (`adapter-digest-mismatch` on drift). The lock is written by the kernel, committed like any lockfile, and makes a cloud runner's install byte-equivalent to the laptop that authored the pin.
- **No interactive seams.** Registry credentials via environment, typed errors on every failure path, exit codes carrying through — a cloud agent drives hydration exactly as an operator does.

## Scope

- The store-root relocation to `$HOME/.specify/adapters` (env override unchanged).
- The hydration kernel and its three triggers, including the `project.yaml.adapters:` prefetch list (additive, optional) and plan-time on-demand hydration.
- `specify adapters sync` with `--frozen`.
- Deployment-manifest generation from resolved store entries, and the embedded workflow-guest materialization.
- The committed `.specify/adapters.lock` digest pin.

## Out of scope

- **Version-range resolution and a release index** — hydration requires exact pins; RM-21 owns ranges, floors, and the compatibility matrix.
- **Third-party adapter namespaces** — the `specify:` namespace posture is unchanged.
- **Store garbage collection** — entries are immutable and shared across projects; a retention policy over unreferenced identities is a follow-up, not a blocker (the store grows by one entry per `(name, version)` ever used).
- **Omnia OCI guest sources** — Omnia keeps loading guests by path; the store is the path namespace. If Omnia later accepts OCI guest references directly, the generated manifest can adopt them without changing hydration.
- **Registry hosting and publish workflow** — the publish side (packing, pushing, tagging) is RFC-48 as landed.

## Acceptance criteria

1. On a fresh machine, `specify init` against a `project.yaml` with pinned identities downloads every declared adapter into `$HOME/.specify/adapters/<name>@<version>/`, generates the deployment manifest, and leaves the project runnable — no sibling checkout, no vendored tree, no hand-authored `omnia.toml`.
2. A warm store makes init and sync no-op probes; two concurrent hydrations of one identity produce one immutable entry and two successes.
3. `specify adapters sync --frozen` fails with a typed error naming any missing identity and fetches nothing.
4. A plan binding a pinned source adapter absent from the store hydrates it at plan validation; an unpinned name resolves project-locally exactly as today.
5. Every hydrated entry is verified against `.specify/adapters.lock` when the lock carries its identity; drift aborts with `adapter-digest-mismatch` before any guest loads.
6. Relocating the store via `SPECIFY_ADAPTER_STORE` changes no behavior other than the root path — the generated manifest follows the resolved entries.

## Risks and invariants

- **The store is load-bearing at runtime.** Deleting an entry breaks every project whose generated manifest references it; the manifest generator must verify entry presence at generation time and the runtime must fail with a typed error (not an Omnia load panic) on a dangling path. This is also why the root leaves `$XDG_CACHE_HOME`.
- **TOFU is only closed by the lock.** Without a committed `.specify/adapters.lock`, a first install on a new machine still trusts the registry; teams wanting supply-chain strictness commit the lock and run `--frozen` in CI.
- **Pins are the determinism boundary.** Nothing in hydration consults "latest"; a bare name never crosses the network. Loosening this requires RM-21's version-resolution design, not an ad-hoc default.
- **The generated manifest is derived, never authored.** Hand edits are lost on regeneration by design; deployment customization (if ever needed) enters through project configuration, not through the artifact.
- **Resolution order is unchanged.** Store-first for pins, then project-local — hydration adds fetch-on-miss, not a new probe order, so vendored-tree and manifest-cache workflows (this repo's own development posture included) keep working untouched.
