# RFC-63: Adapter Hydration and the Central Store

> Status: Proposed · Depends: [RFC-61](rfc-61-omnia-migration.md) (guests by path, the generated deployment), RFC-64 (one-component adapter artifact, wasm-pkg transport — landed) · Revised against: [RFC-65](rfc-65-standalone-deployment.md) (the provisioning / guest surface split, the binary-versioned core guest) and [RFC-66](rfc-66-publishing-and-distribution.md) (registry backing, publish workflows) · Owns: how a project's declared component identities become a fully provisioned, runnable deployment — locally and in the cloud

## Abstract

Post-cutover, Specify's runtime co-loads the core guest plus one wasm component per bound adapter, and Omnia loads guests **by path**. Today the only deployment manifest is the in-tree `omnia.toml` pointing at a sibling `specify-adapters` checkout — a developer posture, not a consumer one. This RFC owns the consumer path: the provisioning surface reads the project's declared component identities, downloads each missing package into the **central store at `$HOME/.specify/adapters/`**, and generates the deployment manifest from the resolved store entries. Hydration is one idempotent, concurrency-safe, non-interactive kernel with two triggers — init and an explicit sync verb, both native verbs on RFC-65's provisioning surface — so the same mechanism provisions a laptop, a CI runner, and a cloud agent with only an environment variable between them. The guest never hydrates: a store miss at guest runtime is a typed error naming the identity and the sync command, never a network fetch.

## What already exists

Most of the machinery landed with RFC-48 and RFC-64; this RFC composes it rather than re-inventing it:

- **Identity.** An adapter is `name@<semver>` (`AdapterRef`), carried as a `specify:<name>@<semver>` package reference (the RFC-65 naming cut); `project.yaml.adapter` records the target's pinned identity, and `plan.yaml.sources.<key>` carries an optional per-binding version pin.
- **Packaging and transport.** A published artifact is exactly one wasm component (RFC-64: no manifest, no tree pack), pulled through the wasm-pkg client (`crates/registry/src/package.rs` — the same funnel the `tools[]` resolver uses). Registry backing — the `augentic.io` well-known file over GHCR, anonymous consume, `GITHUB_TOKEN` publish — is RFC-66's.
- **The store.** `store::install_tofu` pulls a component once (trust-on-first-use), stages it under a sibling install lock, publishes it read-only by atomic rename at the single-file entry `<store-root>/<name>@<version>.wasm`, and records the component-byte digest sidecar (`<name>@<version>.meta`) that `verify_store_entry` re-checks on every resolve (`adapter-digest-mismatch` on drift).
- **Resolution order.** A pinned identity resolves the store entry; a bare name resolves the project component cache, then the sibling/in-repo development release build (`target/wasm32-wasip2/release/specify_<name>.wasm`). A miss on every probe is `adapter-not-found`. Resolution is project-local plus the store — no environment-variable fallback to a framework checkout.

The store root today resolves `$SPECIFY_ADAPTER_CACHE` → `$XDG_CACHE_HOME/specify/adapters` → `~/.cache/specify/adapters` (`crates/schema/src/cache.rs`). Install happens only when the operator hands `specify init` a `specify:<name>@<semver>` package ref; nothing reads `project.yaml` and hydrates, and nothing generates the deployment manifest the runtime needs.

## The store root moves to `$HOME/.specify/adapters`

The store is an **install store, not a disposable cache**: entries are immutable, digest-verified, and load-bearing at runtime (the deployment manifest references components inside it; evicting one bricks every project pinned to it). XDG cache semantics invite eviction, and cloud runners want one obvious directory to persist or mount. The root therefore becomes:

1. `$SPECIFY_ADAPTER_STORE` (renamed from today's `$SPECIFY_ADAPTER_CACHE` to match the store-not-cache framing — the relocation lever for sandboxes and tests),
2. else `$HOME/.specify/adapters`.

Layout is unchanged from RFC-64 as landed: single-file `<root>/<name>@<version>.wasm` entries with sibling `<name>@<version>.meta` digest sidecars and dot-prefixed install locks. `$HOME/.specify/` becomes Specify's per-user home (the store today; a natural later home for auth and channel config), distinct from the per-project `.specify/` system-of-record and the per-project derived cache under `$XDG_CACHE_HOME/specify/projects/`, which stay where they are.

## The hydration kernel

One kernel, `hydrate(refs) -> resolved set`: for each pinned `name@<semver>`, probe the store; on a miss, pull the component through the wasm-pkg transport and `install_tofu` it; verify the digest; return the resolved entry paths. Properties the kernel guarantees, all inherited from the RFC-48/64 substrate:

- **Idempotent.** A warm store makes hydration a no-op probe per identity — cheap enough to run on every provisioning invocation.
- **Concurrency-safe.** Parallel hydrators of one identity serialize behind the blocking install lock; losers find the entry materialized and move on.
- **Non-interactive.** No prompts, no credential dialogs: first-party packages pull anonymously (RFC-66), private mirrors authenticate from the environment, and any failure is a typed error naming the identity and the probe that failed.
- **Exact pins only.** Hydration never resolves a version range or a bare name over the network — an unpinned name keeps today's project-local resolution, and network version-resolution stays deferred (RM-21). Determinism in the cloud comes from pins, not from "latest".

Two triggers share the kernel, both native verbs on RFC-65's **provisioning surface** (the kernel itself is surface-agnostic). Guided init elicitation (RFC-65 §"Operator onboarding") lives strictly above the kernel, gathering arguments — nothing at or below the kernel blocks on a TTY, so the non-interactive property holds on every machine shape:

- **`specify init`** hydrates the target identity recorded on `project.yaml.adapter`, plus every identity in the new optional `project.yaml.adapters:` prefetch list (both axes, pinned), plus `specify:core@<the binary's own version>` when the binary does not embed its core (below) — so a project that knows its source set up front provisions everything in one command. Init then generates the deployment manifest and hands off to the guest scaffold leg (RFC-65's bootstrap order). `specify init --upgrade` re-runs hydration against the (possibly re-pinned) declared set.
- **`specify adapters sync`** is the explicit verb: read `project.yaml` (and `plan.yaml` when present), hydrate every declared identity, regenerate the manifest, print the resolved set with per-identity store paths and digests. It is the one-line cloud bootstrap and the operator's cache-priming and diagnosis surface. `--frozen` turns any would-be fetch into a typed failure (`adapter-not-installed`) for offline and reproducibility-strict CI.

**The guest never hydrates.** Under RFC-65 plan validation runs in the core guest, which holds no network or global-store capability — and per RFC-65's fence, the provisioning surface must not regrow a runtime role, nor the runtime a fetching one. A plan binding a pinned source adapter absent from the store therefore surfaces as a typed `adapter-not-installed` error naming the identity and the literal sync command, and the operator (or the driving skill) runs the sync trigger. Runtime store misses behave as `--frozen` always would: fail loudly, fetch nothing.

## Deployment manifest generation

The runtime stops reading a hand-authored `omnia.toml` in consumer projects. After hydration, the provisioning surface **generates** the deployment manifest into the per-project derived cache (out-of-tree, per the cache-layout decision): one `[[guest]]` per resolved pulled component (every adapter, plus the core when not embedded) pointing at `<store>/<name>@<version>.wasm`, the `[[mount]]` of the project directory as writable `"."`, one `[[route.http]]` MCP prefix per adapter, and the core guest's link allow-list — exactly the shape the in-tree developer manifest models today. The manifest is a derived artifact: regenerated whenever the declared component set or pins change, never committed, never hand-edited.

**The core guest is resolved embedded-first.** A binary built with Omnia's generic `runtime!` embed option (RFC-65 move 4) carries its core and wires it itself — no hydration, no manifest entry. Otherwise the kernel hydrates `specify:core@<the binary's own version>` like any adapter and the manifest references its store entry. Both modes pin the core to the binary version, so there is no core pin surface either way. Today's committed workflow guest (RFC-61 D-dist) is the embed posture with a hand-rolled mechanism; the RFC-65 cut replaces it with the pulled baseline or the generic embed, changing nothing in this RFC's kernel or manifest generator.

## Cloud posture

The cloud story is the local story with the knobs exposed, not a second mechanism:

- **Relocatable root.** `SPECIFY_ADAPTER_STORE` points the store at a mounted volume or a restored cache directory; nothing else changes. `$HOME/.specify/adapters` is the default that makes the unmounted case still work.
- **Cache priming.** CI restores the store directory keyed on the project's pinned identities (a digest over `project.yaml.adapter` + `adapters:` + plan source pins is a stable cache key); `specify adapters sync` after restore is a no-op probe on a hit and a fetch on a miss. `--frozen` converts "miss" into "fail loudly" where fetching is forbidden.
- **Cross-machine digest pinning.** RFC-48 left the trust model at TOFU per machine. Hydration closes the gap with a committed lock: `.specify/adapters.lock` records each identity's component-byte digest at first install, and every subsequent hydration — any machine — verifies the store entry against the committed digest before use (`adapter-digest-mismatch` on drift). The lock is written by the kernel, committed like any lockfile, and makes a cloud runner's install byte-equivalent to the laptop that authored the pin. (It is also RFC-66's content-equivalence lever across a registry-host migration.)
- **No interactive seams.** Anonymous first-party pulls, environment credentials for mirrors, typed errors on every failure path, exit codes carrying through — a cloud agent drives hydration exactly as an operator does.

## Scope

- The store-root relocation to `$HOME/.specify/adapters`, including the `$SPECIFY_ADAPTER_CACHE` → `$SPECIFY_ADAPTER_STORE` env-override rename.
- The hydration kernel and its two provisioning-surface triggers, including the `project.yaml.adapters:` prefetch list (additive, optional).
- The typed `adapter-not-installed` posture for plan-time and runtime store misses (the guest-never-hydrates fence).
- `specify adapters sync` with `--frozen`.
- Deployment-manifest generation from resolved store entries (the core guest appears as one more resolved entry when pulled; an embedded core needs none — RFC-65 move 4).
- The committed `.specify/adapters.lock` digest pin.

## Out of scope

- **Version-range resolution and a release index** — hydration requires exact pins; RM-21 owns ranges, floors, and the compatibility matrix.
- **Registry backing and publish workflows** — the well-known file, GHCR packages, idempotent publish loops, and the `specify:core` publish job are [RFC-66](rfc-66-publishing-and-distribution.md)'s.
- **The surface split and the core guest's existence** — [RFC-65](rfc-65-standalone-deployment.md)'s; this RFC supplies the provisioning kernel both sides of that cut share.
- **Third-party adapter namespaces** — the first-party `specify:` posture is unchanged.
- **Store garbage collection** — entries are immutable and shared across projects; a retention policy over unreferenced identities is a follow-up, not a blocker (the store grows by one file per `(name, version)` ever used).

## Acceptance criteria

1. On a fresh machine, `specify init` against a `project.yaml` with pinned identities downloads every declared component into `$HOME/.specify/adapters/<name>@<version>.wasm`, generates the deployment manifest, and leaves the project runnable — no sibling checkout, no vendored tree, no hand-authored `omnia.toml`.
2. A warm store makes init and sync no-op probes; two concurrent hydrations of one identity produce one immutable entry and two successes.
3. `specify adapters sync --frozen` fails with a typed error naming any missing identity and fetches nothing.
4. A plan binding a pinned source adapter absent from the store fails validation with `adapter-not-installed`, naming the identity and the literal sync command; no guest code path performs a network fetch or a global-store write. An unpinned name resolves project-locally exactly as today.
5. Every hydrated entry is verified against `.specify/adapters.lock` when the lock carries its identity; drift aborts with `adapter-digest-mismatch` before any guest loads.
6. Relocating the store via `SPECIFY_ADAPTER_STORE` changes no behavior other than the root path — the generated manifest follows the resolved entries.

## Risks and invariants

- **The store is load-bearing at runtime.** Deleting an entry breaks every project whose generated manifest references it; the manifest generator must verify entry presence at generation time and the runtime must fail with a typed error (not an Omnia load panic) on a dangling path. This is also why the root leaves `$XDG_CACHE_HOME`.
- **TOFU is only closed by the lock.** Without a committed `.specify/adapters.lock`, a first install on a new machine still trusts the registry; teams wanting supply-chain strictness commit the lock and run `--frozen` in CI.
- **Pins are the determinism boundary.** Nothing in hydration consults "latest"; a bare name never crosses the network. Loosening this requires RM-21's version-resolution design, not an ad-hoc default.
- **The guest never fetches.** The typed-error posture on store misses is a fence, not a stopgap: an in-guest hydration convenience would hand the runtime a network capability and re-blur RFC-65's provisioning / runtime split. The fix for a friction-heavy miss is a better sync prompt, never a guest-side pull.
- **The generated manifest is derived, never authored.** Hand edits are lost on regeneration by design; deployment customization (if ever needed) enters through project configuration, not through the artifact.
- **Resolution order is unchanged.** Store-first for pins, then project-local (component cache, then the sibling development build) — hydration adds fetch-on-miss at the provisioning-surface triggers, not a new probe order, so the sibling-checkout development posture keeps working untouched.
