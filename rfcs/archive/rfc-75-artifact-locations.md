# Artifact Locations — Production Layout vs Co-dev Binding

> Status: Implemented (archived). Stage 1 complete; optional Stage 2 (bare-at-init materialization) deferred.
>
> Owns: on-disk location policy for resolvable artifacts (adapter store entries, digest sidecars, project component cache); the composition-root capture of the relocation environment overrides; removal of Cargo-layout and other co-dev probes from shipped crates.
>
> Builds on: [Specify on Omnia](../architecture.md), [RFC-70 Self-Assembling Wasm Deployment](../rfc-70-deployment.md), [Native Specify](native-deployment.md) (archived — implemented). Prior art for the configuration shape: the Omnia backends' `ConnectOptions` pattern (`backends/crates/kafka` — `#[derive(FromEnv)]`, production defaults in code, env vars the only override, resolved once at the composition root).
>
> Does not own: adapter identity grammar (`AdapterSelector`), registry discovery (RFC-71), native catalog composition, or Omnia guest linking.

## Abstract

Shipped Specify code today hardcodes co-development filesystem knowledge into the component resolver: bare names probe `target/wasm32-wasip2/release/<name>.wasm`, locations are labeled `"dev"`, unpinned resolves invent version `0.0.0`, and supporting exemptions cascade through launcher preflight and topology closure. That is an anti-pattern.

This RFC introduces a `Locations` **value** whose construction encodes the production layout. One umbrella location, `SPECIFY_HOME`, defaults to `$HOME/.specify` (falling back to `<temp>/specify` when `$HOME` is unavailable) and is the *only* override surface: `<home>/store` and `<home>/cache` are derived together, captured **once** at each composition root through the `FromEnv` pattern the Omnia backends use, then carried as plain data. There is no trait, no alternate layout binding, and no co-dev branch in the shipped binary: development loops **seed the production locations** (a new `specify adapter add` verb, `specify init ./path.wasm`, or a fully relocated sandbox).

After this change, production resolution knows exactly two places:

1. the global adapter store (pinned identities + digest sidecars);
2. the project component cache (operator-supplied and operator-seeded local components).

Everything else is Make/lab script territory outside the shipped crates.

## Motivation

### The anti-pattern

`project::adapter::resolver::locate` currently probes, for bare and local-component selectors:

1. `<project-cache>/components/<name>.wasm` — a legitimate production location;
2. `<project>/target/wasm32-wasip2/release/<name>.wasm` — a Cargo layout known only to in-repo developers.

Both hits become `AdapterLocation::Dev` with wire label `"dev"`. Related support then spreads:

| Surface | Co-dev leakage |
| ------- | -------------- |
| `dev_component_path` / `dev_component_filename` | Cargo artifact naming in `project` |
| `AdapterLocation::Dev` / origin `"dev"` | Misnames cache hits; advertises co-dev on the wire |
| `dev_version()` → `0.0.0` | Fake semver for unpinned resolves |
| `ensure::provision` bare no-op | Relies on live Cargo locate |
| `launcher::closure::slot_targets` | Skips topology entries at `0.0.0` |
| `init/context.rs` context fingerprint | `context.lock` inputs keyed on the `"dev"` origin label |
| `examples/Makefile.toml` | Stages mock source under sandbox `target/` to satisfy the probe |
| CLI help / AGENTS / workflow docs | Contract-lock the Cargo probe as product behavior |
| `SPECIFY_PROSE_OVERLAY` in `adapter` | Dead eval-era prompt override in the shipped SDK — including a `panic!` branch and a leaked-`String` path shipping inside every published adapter component |

The probe is hazardous as well as inelegant: any consumer project that happens to be a Rust workspace can have an unverified build artifact at `target/wasm32-wasip2/release/<name>.wasm` silently satisfy a bare `project.yaml.adapter` binding. No serious production resolver should hunt the build directory. The product's well-known locations are store and cache; tools that want a live guest build **stage into those locations**.

### The ambient-environment anti-pattern (secondary)

The relocation overrides are legitimate, but today they are resolved *ambiently*: `diagnostics::cache::adapter_store_root()` and `projects_root()` read `std::env` on every call, from every layer. Consequences:

- layout is process-global state rather than a carried value, so sandboxed sessions relocate the cache through `ExecutionPaths::isolated` but relocate the store through env mutation;
- the test suites pin layout with `unsafe { std::env::set_var(…) }` guards (`scoped_store`, the launcher `EnvGuard`) that depend on nextest's process-per-test isolation;
- `cfg!(target_arch = "wasm32")` branches inside the path helpers switch to the guest preopens, entangling deployment topology with path math.

The Omnia backends already solve this class of problem: a typed options struct (`ConnectOptions`) with production defaults in code, `#[env(from = "…")]` overrides captured once at the composition root, and the resolved value carried thereafter. This RFC applies the same shape to artifact layout.

### Why not a trait

The previous draft proposed `Locations` as a trait with production default methods and composition-root overrides. Review rejected that framing:

- by the draft's own preference (seed the cache, don't bind a Cargo layout), `Production` would be the trait's only implementor — exactly the `trait Foo` + sole `RealFoo` shape [style.md](../../docs/standards/style.md) forbids;
- the variation the trait was defending already exists twice: env relocation for the store, `ExecutionPaths` for the cache parent — a trait override of `store_entry` and setting the deployment's `SPECIFY_HOME` would be two ways to say the same thing;
- the wasm32 engine guest does not need layout polymorphism: it sees the `GUEST_STORE_MOUNT` / `GUEST_CACHE_MOUNT` preopens and binds them directly.

A value constructed from env-with-defaults keeps the single production layout explicit, gives sandboxes and tests a typed injection point (`Locations::explicit`), and adds zero API surface for hypothetical composition roots. If a second real layout ever materializes, promoting the value behind a trait is a mechanical follow-up.

### What already works

The engine guest path is already correct: the launcher hydrates `specify:engine@<binary version>` into the store; `examples/Makefile.toml` seeds store + `.meta` for local runs. There is no implemented `SPECIFY_ENGINE_PATH` probe in Rust (RFC architecture prose mentions it; this RFC rejects implementing it in the shipped launcher).

Relocation remains legitimate production configuration — sandboxes and installs — not a co-dev escape hatch. This RFC replaces the two independent variables and two independent default cascades with one deployment root, `SPECIFY_HOME`, captured once instead of read ambiently. Its effective default is `$HOME/.specify`; when `$HOME` is unavailable, `<temp>/specify`. Store and cache always derive atomically as `<SPECIFY_HOME>/store` and `<SPECIFY_HOME>/cache`.

## Goals

1. Make on-disk artifact layout a **typed value with production defaults**, relocatable as one unit through `SPECIFY_HOME`, captured once at each composition root (the `FromEnv` pattern) and carried explicitly.
2. Remove all Cargo `target/` knowledge from shipped crates (`project`, `launcher`, `transport` help, root binary).
3. Keep store pins and project-cache components as the only production resolve paths.
4. Preserve a working local iteration loop for **both axes** by seeding: a first-class cache-seeding CLI verb (`specify adapter add`), `specify init ./path.wasm`, and whole-sandbox env relocation. Source adapters have no init-time mirror today, so the verb is a Stage 1 requirement, not a nicety.
5. Stop inventing `0.0.0` as the component resolver's default identity for bare names — and settle the unpinned identity's wire shape **in Stage 1** (resolved identities carry `Option<semver::Version>`; envelopes omit the version and topology uses the bare name for cache-backed resolves).
6. Delete the unused `SPECIFY_PROSE_OVERLAY` branch from the adapter SDK.
7. Remove ambient env reads and `unsafe` test env mutation from the layout path.
8. Update the workflow contract and operator docs in the same change set so prose cannot reintroduce the probe.

## Non-goals

- Changing Omnia, WIT worlds, or adapter publication.
- Removing `AdapterSelector::Bare` from the grammar (policy for what bare *means* is in scope; deleting the variant is not).
- Replacing the native host's catalog resolution (native has no component files; it keeps catalog match).
- Implementing `SPECIFY_ENGINE_PATH` in the launcher.
- A `Locations` trait or any alternate layout implementation — deferred until a second real composition-root layout exists.
- A general virtual filesystem or searchable path list for arbitrary assets.
- Moving `mock` / `lab` / `eval` into the shipped binary.
- Softening digest verification for store-backed entries.
- Per-location or co-dev override variables. `SPECIFY_HOME` is the complete override surface; `SPECIFY_ADAPTER_STORE`, `SPECIFY_PROJECT_CACHE`, and `SPECIFY_DEV_ADAPTERS=1`-style flags are not retained as aliases.

## Decision

### D1 — `Locations` value: production defaults, one home override

Introduce a deployment-neutral **value** (not a trait) that owns the two well-known roots and the layout formulas over them. Construction encodes policy; methods are pure path math. The one override (`SPECIFY_HOME`) is a plain `std::env::var_os` read inside `from_env` — a single optional variable does not warrant a config-derive dependency.

```rust
/// How the cache root carried by [`Locations`] is interpreted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CachePlacement {
    /// Host parent: append the canonical project-id digest.
    Parent(PathBuf),
    /// Already-resolved per-project root, such as the guest preopen.
    Project(PathBuf),
}

/// Well-known on-disk locations for resolvable artifacts.
///
/// A plain value: construction is the only place layout policy lives.
/// [`Locations::from_env`] is the shipped production layout —
/// defaults in code, `SPECIFY_HOME` the only override,
/// captured once. [`Locations::explicit`] is the injection point for
/// sandboxes, tests, and the engine guest's preopens. There is no
/// trait and no alternate implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locations {
    store_root: PathBuf,
    cache: CachePlacement,
}

impl Locations {
    /// Production layout: capture `SPECIFY_HOME` once. A valid
    /// absolute override wins; otherwise the effective home is
    /// `$HOME/.specify`, then `<temp>/specify` when `$HOME` is
    /// unavailable. Derive `<home>/store` and `<home>/cache`.
    /// Composition-root only — never called from kernels or handlers.
    pub fn from_env() -> Self { /* env_path("SPECIFY_HOME") + defaults */ }

    /// Explicit layout — no env reads. Sandboxed sessions and tests
    /// pass `Parent`; the wasm32 guest passes `Project` for its
    /// already-resolved preopen.
    pub fn explicit(store_root: PathBuf, cache: CachePlacement) -> Self { /* … */ }

    /// Global store root used for hydration and the launcher mount.
    pub fn store_root(&self) -> &Path { /* … */ }

    /// Global store entry: `<store-root>/<name>@<version>.wasm`.
    pub fn store_entry(&self, name: &str, version: &str) -> PathBuf { /* … */ }

    /// Digest sidecar sibling of [`Self::store_entry`].
    pub fn store_meta(&self, name: &str, version: &str) -> PathBuf { /* … */ }

    /// Resolved cache directory for `project_root`. `Parent` appends
    /// `<project-id>`; `Project` returns the carried preopen directly.
    pub fn project_cache_dir(&self, project_root: &Path) -> PathBuf { /* … */ }

    /// Project component cache entry under
    /// `<project-cache>/components/<name>.wasm`.
    pub fn component(&self, project_root: &Path, name: &str) -> PathBuf { /* … */ }
}
```

**Hard rule:** the formulas mention only store and project-cache shapes. They never mention `target/`, Cargo triple directories, or `.eval/`.

`SPECIFY_HOME` is an all-or-nothing relocation. Empty or relative values are ignored and fall through to the effective default (`$HOME/.specify`, then `<temp>/specify`). There is no precedence matrix and no compatibility alias for the retired per-location variables: every deployment derives both roots from one absolute home.

Carriage: `ExecutionPaths` grows into the full layout carrier — project root plus `Locations` — constructed at each composition root (`ExecutionPaths::operator` calls `Locations::from_env()` once; `ExecutionPaths::isolated` takes `Locations::explicit` with `CachePlacement::Parent`; the engine guest constructs `explicit(GUEST_STORE_MOUNT, CachePlacement::Project(GUEST_CACHE_MOUNT))`). `ExecutionPaths::with_root` preserves the placement: a host `Parent` derives the new project's digest-keyed cache, while a guest `Project` keeps the one mounted preopen. Kernels, `locate`, ensure, and launcher hydrate receive the value through `ExecutionPaths` and never read `std::env` themselves.

The launcher constructs one `ExecutionPaths` after anchoring the project and passes that same value through closure hydration, preflight, and deployment assembly. It does not reconstruct `ExecutionPaths::operator` in `hydrate` or `deployment`, so one invocation captures the environment exactly once. `Locations::store_root` and `Locations::project_cache_dir` supply the host mount sources; entry and sidecar helpers derive from those same roots.

`diagnostics::cache` reduces to pure path math parameterized by roots (project-id digesting, sidecar naming, digest verify). Its env-reading resolvers (`adapter_store_root()`, `projects_root()`) and its `cfg!(target_arch = "wasm32")` mount branches migrate into `Locations::from_env` / the guest's `explicit` construction. This is the same seam discipline as the Omnia backends: `Client::connect_with(options)` never reads env; `ConnectOptions::from_env()` at the composition root does.

Dependency note: no new dependency — the single `SPECIFY_HOME` read is a hand-rolled `std::env::var_os` inside `Locations::from_env` in the crate that homes `Locations` (`project::handler`, beside `ExecutionPaths`). `from_env()` may be `#[cfg(not(target_arch = "wasm32"))]` — the guest only ever constructs `explicit`.

### D2 — `locate` becomes layout-blind

`resolver::locate` (and launcher hydrate's path derivation) ask the carried `Locations` and probe only what production defines:

| Selector | Production resolve |
| -------- | ------------------ |
| `Package { name, version, .. }` | `locations.store_entry` + verify-on-read against `store_meta` |
| `Bare { name }` | `locations.component` — miss → `adapter-not-found` |
| `Component { path }` | ensure mirrors into `locations.component`, then resolve from cache |

Delete from shipped crates:

- `dev_component_path`
- `dev_component_filename`
- the second probe in `locate`
- error / clap text that advertises `cargo build --release --target wasm32-wasip2`

**Explicit exemption:** `selector::name_from_component` keeps stripping the `specify_` / `specify-` artifact prefix and folding underscores to kebab. That is boundary normalization of operator-supplied filenames (operators legitimately hand over cargo-built `.wasm` files), not a resolve probe. It is the one permitted Cargo-naming site in shipped code; the acceptance-criteria grep documents it as such.

### D3 — Replace `AdapterLocation::Dev`

Collapse the location taxonomy to production mechanisms:

**Option A (preferred):** closed enum aligned with real mechanisms

```rust
pub enum AdapterLocation {
    Store(PathBuf),
    Cache(PathBuf),
}
```

Wire labels: `"store"` / `"cache"` (replacing `"dev"`). The init context fingerprint (`context.lock` inputs keyed on the origin label) re-keys in the same change.

**Option B:** opaque path + `Origin` only (no closed location enum). Closest to “engine does not enumerate deployment mechanisms,” but loses a cheap Store-vs-Cache distinction for diagnostics.

This RFC prefers **Option A** for Stage 1: small, honest, and matches preflight’s store-vs-non-store digest policy. Option B remains a follow-up if the enum earns no callers beyond label projection.

### D4 — Co-dev is seed-only

There is **no** co-dev layout binding — no `DevLocations` type, no optional trait method, no env flag. Local iteration uses production mechanisms:

1. **Seed the project component cache.** `specify adapter add ./target/wasm32-wasip2/release/foo.wasm` (D5) mirrors the built component into the cache; `specify init ./path.wasm` continues to do the same for the project's target adapter. Make/lab tasks wrap the build-then-seed sequence.
2. **Relocate the whole sandbox.** `SPECIFY_HOME` points store and cache at one scratch tree (`<home>/store`, `<home>/cache`). This is the env-override leg of D1 doing its production job, not a special co-dev mode.

Cargo layout strings exist **only** in Make/lab/example scripts — never in shipped crates.

### D5 — Cache seeding verb: `specify adapter add <path.wasm>`

Stage 1 must ship an operator-facing cache writer, because deleting the Cargo probe otherwise severs the source-adapter dev loop:

- plan source bindings accept only bare names and first-party pins — `parse_source_adapter` rejects component paths with `plan-source-adapter-invalid`;
- the only shipped writer of `<project-cache>/components/` today is the *target* adapter's init-time mirror;
- the cache directory is deliberately out-of-tree and keyed by a SHA-256 of the project path, so “just `cp` into the cache” requires either digest arithmetic or full-sandbox relocation.

`specify adapter add <path.wasm> [--project-dir <dir>]` is project-context-free: it defaults to the invocation directory, accepts an explicit project directory, and does not require `.specify/project.yaml` to exist. That permits `adapter add` followed by a bare `init`; relative component paths anchor at the selected project directory.

The verb reuses the existing mirror kernel's file check, canonicalization, name derivation, copy, and provenance stamp. It is deliberately axis-neutral and does not inspect component exports: adapter names are unique across axes, and the binding that later resolves the bare name supplies the expected axis. A wrong-world component therefore fails at the existing dispatch/metadata axis gate, not during seeding. The launcher's selector projection classifies `adapter add` as engine-only; the component path is input to a copy operation, not an adapter requirement that must be enumerated before the command runs.

Re-seeding the same name replaces that cache entry and its provenance sidecar; the explicit operator command is the approval act. Bare-name bindings (project target, plan sources) then resolve the seeded entry.

Multi-tenant provenance fix in the same change: `ComponentMeta` is currently one shared `component-meta.yaml` for the whole `components/` tenant, so each mirror clobbers the previous component's provenance. It becomes a per-component sidecar (`<name>.meta.yaml`).

### D6 — Bare-name product policy

`AdapterSelector::Bare` remains in the grammar. Its production meaning becomes:

> Resolve the project component cache entry for this name.

It is **not** “development shorthand for a live Cargo build.”

| Stage | Policy |
| ----- | ------ |
| Stage 1 (this RFC) | Bare → cache only; miss fails closed with guidance to pin, `specify adapter add` a local `.wasm`, or supply one at init |
| Stage 2 (optional follow-up) | Init may accept bare and materialize into cache, persisting a pin or component selector so committed state is not ambiguous |
| Rejected | Bare → Cargo `target/` probe in shipped code |

Native host behavior is unchanged: bare matches the linked catalog by name; unpublished `0.0.0` catalog identities remain bare-only (that placeholder lives in `mock` / unpublished adapters, not in the component resolver’s path logic).

### D7 — Stop minting `0.0.0` from component resolve (Stage 1, including the wire shape)

`dev_version()` as the default for every unpinned component resolve is removed. The previous draft deferred the replacement shape to Stage 2; review showed that is not landable — `ResolvedSource` / `ResolvedTarget` carry a non-optional `semver::Version`, so deleting `dev_version()` forces the decision at compile time. Stage 1 therefore decides it:

- `SourceAdapter.version` / `TargetAdapter.version` become `Option<semver::Version>`: the exact pin for store-resolved identities, `None` for cache-backed resolves.
- Resolve envelopes make `version` optional and omit it for unpinned resolves (no sentinel); text output likewise omits the `version:` line.
- `TopologyProject.target` remains a string: `name` when unpinned, `name@version` when pinned. Its documentation and every producer adopt that exact grammar.
- `launcher::closure::slot_targets` parses the topology target through `AdapterSelector`: a bare target is the explicit unpinned case that resolves against the slot's own tree and is skipped; a pinned package target joins the closure. There is no magic-version predicate.

Native/mock `0.0.0` identities remain valid **unpublished catalog** markers — a compiled-identity concern, not a component-resolve default.

### D8 — Preflight exemptions are “non-store,” not “development”

Launcher preflight continues to digest-verify only store-backed closure entries. Cache-backed and operator-local components are exempt because they are not content-addressed store installs — not because they are “dev artifacts.” Rename comments and docs accordingly.

### D9 — Delete the unused prose overlay

`SPECIFY_PROSE_OVERLAY` / `.eval/prose/` in `adapter::registry` is dead co-dev support in a crate that ships inside every published adapter component — including a `panic!` on unreadable overlay files and a leaked-`String` path. No eval code configures it, and the adapters repository's testing contract says prompt edits rebuild natively and there is no overlay mode. Stage 1 deletes the environment branch, filesystem lookup, leak, panic path, and overlay test. `resolve` / `body` become embed-only; published components carry no overlay branch.

### D10 — Engine guest iteration stays store-seeded

Do not implement `SPECIFY_ENGINE_PATH` (or an in-repo engine `target/` probe) in the shipped launcher. Local engine iteration continues to seed `engine@<version>.wasm` + `.meta` into the store (as `examples/Makefile.toml` already does). Strike or supersede the architecture RFC sentence that suggests otherwise.

### D11 — Capability placement

`Locations` is a value carried by `ExecutionPaths`, alongside — not instead of — the provider capabilities:

| Concern | Question it answers |
| ------- | ------------------- |
| `Locations` (value on `ExecutionPaths`) | Where on disk may this artifact live? |
| `adapter::Resolver` | Given a selector, what usable adapter identity + metadata do I have? |
| `ensure_*` | Make the selector resolvable (hydrate / mirror / catalog match), then resolve |

Kernels and `locate` take the value from `ExecutionPaths`. They do not call free-function layout helpers that read env — today's `component_cache_entry` / store-root resolvers fold into `Locations`; `diagnostics::cache` keeps only the pure math (`project-id` digesting, digest verify, sidecar shapes).

This deliberately does **not** introduce a trait ([style.md](../../docs/standards/style.md) — no traits for testability alone, and no second layout binding exists). Should a real second layout materialize, wrapping the value's formulas behind a trait is a mechanical, additive change.

## Design sketch

### Production resolve flow

```text
AdapterSelector
      │
      ▼
 ensure (hydrate pin / mirror component / no-op bare)
      │
      ▼
 locate(selector, paths)          paths: ExecutionPaths { root, Locations }
      │
      ├─ Package ─► locations.store_entry (+ verify store_meta)
      └─ Bare / Component ─► locations.component
      │
      ▼
 AdapterLocation::Store | AdapterLocation::Cache
      │
      ▼
 metadata dispatch → ResolvedSource / ResolvedTarget
```

### Co-dev loop (production mechanisms only)

```text
cargo build --target wasm32-wasip2
        │
        ▼
 ┌──────┴───────────────────────────────┐
 │ per-component seed                    │ whole-sandbox relocation
 │ specify adapter add ./foo.wasm        │ SPECIFY_HOME=…
 │   (or specify init ./foo.wasm)        │   ├── store/
 │                                        │   └── cache/
 └───────────────────────────────────────┘
        │
        ▼
 the one shipped Locations layout — no co-dev binding anywhere
```

### Composition roots

| Root | Construction |
| ---- | ------------ |
| Shipped binary / launcher | `Locations::from_env()` once per invocation, carried on one `ExecutionPaths::operator` through hydrate + deployment |
| Engine guest (wasm32) | `Locations::explicit(GUEST_STORE_MOUNT, CachePlacement::Project(GUEST_CACHE_MOUNT))` — mounts, no env or project-id suffix |
| Sandboxed sessions (`eval`, lab) | `Locations::explicit(tempdirs…, CachePlacement::Parent(…))` via `ExecutionPaths::isolated` |
| Tests | `Locations::explicit` with tempdir roots — the `unsafe` env guards (`scoped_store`, launcher `EnvGuard`) delete; a small retained suite covers the env capture itself |

## Migration plan

### Stage 1 — Cut the probe (this RFC’s landing)

1. Land `Locations` (value + `CachePlacement` + `from_env` / `explicit` constructors) and fold it into `ExecutionPaths`; migrate `diagnostics::cache` env reads and wasm32 mount branches into construction.
2. Thread one carried `ExecutionPaths` through `locate`, ensure mirror paths, launcher hydrate, and deployment assembly; kernels stop reading env and launcher mounts use the value's root accessors.
3. Delete Cargo probes and `AdapterLocation::Dev`; emit `"cache"`; update resolve envelopes and the init `context.lock` fingerprint keys that keyed on `"dev"`.
4. Make resolved identity versions `Option<semver::Version>`; omit the version for cache-backed resolve envelopes, project unpinned topology targets as bare names, and replace the launcher's `0.0.0` skip with the explicit bare-target predicate.
5. Ship `specify adapter add <path.wasm>` over the ensure mirror kernel; convert `ComponentMeta` to per-component sidecars.
6. Retarget tests: `stage_dev_component` → seed the project cache (via `adapter add` or direct `Locations::explicit` paths); drop sandbox `target/` staging in `examples/Makefile.toml` in favor of `adapter add` / `init` with local components for both mock source and target; replace its two location exports with `SPECIFY_HOME="$PWD/sandbox/wasm"`; delete the `unsafe` env guards.
7. Remove cargo-build advertising from errors and clap about strings.
8. Delete `SPECIFY_PROSE_OVERLAY` and its test.
9. Sync prose: `AGENTS.md`, `docs/standards/workflow.md`, `docs/standards/architecture.md`, `docs/reference/cli/init.md`, `docs/contributing/cli-architecture.md`, `docs/explanation/adapter-anatomy.md`, transport help, and the architecture RFC engine-override sentence.

### Stage 2 — Optional init materialization

If operator UX wants bare names at init without a prior `adapter add`, init may build-or-copy into the cache and persist a non-bare selector. Still no Cargo probe in `locate`.

## Compatibility and wire impact

| Surface | Change |
| ------- | ------ |
| Resolve JSON `location` | `"dev"` → `"cache"` (breaking for consumers of that diagnostic field) |
| Resolve version for unpinned | Field and text line omitted instead of `0.0.0` (Stage 1) |
| Topology target for unpinned | Bare `name`; pinned targets remain `name@version` |
| Bare name without cache entry | Hard fail (`adapter-not-found`) instead of falling through to `target/` |
| CLI surface | New verb: `specify adapter add <path.wasm>` |
| Relocation environment | `SPECIFY_ADAPTER_STORE` / `SPECIFY_PROJECT_CACHE` are replaced by `SPECIFY_HOME`; `<home>/store` and `<home>/cache` relocate together |
| Default store | `$HOME/.specify/adapters` → `$HOME/.specify/store` |
| Default project cache | XDG / `$HOME/.cache/specify/projects` → `$HOME/.specify/cache/<project-id>` |
| Component cache provenance | `component-meta.yaml` → per-component `<name>.meta.yaml` |
| Store pins + digest | Unchanged |
| Local `.wasm` at init | Unchanged (mirror to cache) |
| Native catalog | Unchanged |
| Major / migration framework | Pre-1.0 hard cut is acceptable per project policy; document in release notes |

## Testing

- Integration tests stage components into the **project cache** (via `adapter add` or `Locations::explicit` paths), not under `target/`.
- A focused test asserts that a file at `<project>/target/wasm32-wasip2/release/<name>.wasm` does **not** satisfy bare resolve.
- Keep the existing sibling-checkout non-probe test.
- Env capture gets its own small suite (`from_env` derives both roots from an absolute `SPECIFY_HOME`, ignores empty/relative values, defaults to `$HOME/.specify`, and falls back to `<temp>/specify` without `$HOME`); cache-placement coverage proves host `Parent` appends the project id while guest `Project` does not. Everything else injects `Locations::explicit` and the `unsafe` env-mutation guards delete.
- `adapter add` gets integration coverage before init and on both axes: seed a target then initialize by bare name; seed a source, bind it bare in `plan.yaml.sources`, and resolve. Seed a second component and assert per-component provenance sidecars do not clobber; re-seed one name and assert its bytes and sidecar are replaced.
- Launcher selector-projection coverage classifies `adapter add` as engine-only and verifies its component path does not become an axis-qualified closure requirement.
- Launcher / wasm example: seed store + cache only.

## Documentation impact

Same PR as Stage 1 code must update every hit for:

- `development release build`
- `target/wasm32-wasip2/release` (except the documented `name_from_component` exemption and Make/lab scripts)
- `AdapterLocation::Dev`
- origin label `"dev"` as a component-provider mechanism
- bare name as “development shorthand” for Cargo
- `SPECIFY_PROSE_OVERLAY`

Workflow contract language becomes: bare → project component cache; pin → store; local path → mirror then cache; `specify adapter add` seeds the cache for either axis. `SPECIFY_HOME` is documented on the `Locations` value as the complete override surface, with `$HOME/.specify` as its effective default.

## Open questions

Record answers here before implementation freezes Stage 1 wire behavior.

1. **Verb naming.** `specify adapter add` vs `specify adapter install` vs folding into `specify init --component`? (`add` preferred: it is a cache seed, not a store install.)
2. **Bare miss UX.** Exact `adapter-not-found` detail string — it should name `specify adapter add` and the pin form, and must not name Cargo.
3. **Plan source grammar.** Should plan sources also accept component selectors directly (mirroring at `plan author` time), making `adapter add` a convenience rather than the only path? Deferred; Stage 1's supported local path is `adapter add` followed by a bare binding.
4. **Env reads at the leaf.** `Locations` homes in `project::handler` beside `ExecutionPaths`; confirm `diagnostics` keeps only pure math (no environment reads on the leaf).
5. **Option A vs B for `AdapterLocation`.** Confirm Stage 1 enum vs opaque `Origin` only.

## Alternatives considered

<details>
<summary>Superseded: `Locations` as a trait with production default methods (rev 1 of this RFC)</summary>

The previous draft made `Locations` a trait whose default methods were the production layout, with an empty `Production` marker and a possible lab-only `DevLocations` override. Review rejected it: with seed-only co-dev the trait has exactly one implementor forever (the sole-impl trait shape style.md forbids); it overlaps the two relocation mechanisms that already exist (env overrides, `ExecutionPaths`); its `store_entry` default delegated to env-reading helpers, splitting layout policy between a capability and ambient environment; and the wasm32 guest never needed it. The value-with-`FromEnv` shape keeps the explicit production layout and the typed injection point while adding no polymorphism. A trait remains a mechanical follow-up if a second real layout binding ever appears.
</details>

<details>
<summary>Rejected: cfg / feature-gate the Cargo probe in shipped locate</summary>

Keeps Cargo layout strings and probe order in the production function. Features rot; defaults still teach the binary about `target/`. Rejected.
</details>

<details>
<summary>Rejected: env flag to enable Cargo probe</summary>

Same branch in the shipped binary; operators can trip it accidentally; still an anti-pattern. Rejected.
</details>

<details>
<summary>Rejected: `DevLocations` / any co-dev layout binding in-tree</summary>

Documents co-dev on a production seam and keeps Cargo layout strings in workspace crates. Seeding the cache through `specify adapter add` covers the same loop with production mechanisms. Rejected.
</details>

<details>
<summary>Rejected: feature-gate the prose overlay on `adapter`</summary>

Gated code still ships in the published crate source, still carries the panic branch, and features rot. There is no active consumer to preserve: native prompt iteration rebuilds the adapter crates. Delete the dead branch instead. Rejected.
</details>

<details>
<summary>Rejected: keep ambient env reads in `diagnostics::cache`</summary>

Works, but leaves layout as process-global state: sandboxes relocate the cache through `ExecutionPaths` and the store through `unsafe` env mutation, and every helper call re-reads the environment. The backends' `FromEnv` pattern (capture once at the composition root, carry a value) is strictly better and already the house style for Omnia configuration. Rejected.
</details>

<details>
<summary>Rejected: implement SPECIFY_ENGINE_PATH in launcher</summary>

Duplicates the store-seed path already used by the wasm example; adds another co-dev branch to the shipped binary. Rejected; supersede architecture prose.
</details>

## Acceptance criteria

Stage 1 is done when:

- [x] `Locations::from_env` / `Locations::explicit` are the only layout constructions; no kernel, handler, or `locate` path reads `std::env` for layout, and the launcher carries one captured value through hydration and deployment.
- [x] `SPECIFY_HOME` is the only layout environment variable; it defaults to `$HOME/.specify` (then `<temp>/specify` without `$HOME`), and every effective home derives `<home>/store` and `<home>/cache` while the retired per-location variables have no compatibility path.
- [x] No shipped crate contains `target/wasm32-wasip2` as a resolve probe (grep-clean except docs history / this RFC / Make-lab scripts / the documented `name_from_component` exemption).
- [x] Bare resolve succeeds only from the project component cache (or store for pins).
- [x] `specify adapter add` seeds the cache for either axis; a bare plan-source binding resolves a seeded component.
- [x] Resolve envelopes no longer emit `"dev"` and omit the version for cache-backed resolves; topology writes a bare target name for the same unpinned identity (no `0.0.0`).
- [x] `examples/Makefile.toml` does not stage adapters under sandbox `target/`.
- [x] Published adapter components carry no `SPECIFY_PROSE_OVERLAY` branch.
- [x] The `unsafe` env-mutation test guards are deleted; env-capture behavior has direct coverage.
- [x] Workflow / AGENTS / architecture docs describe store + cache only.
- [x] `cargo make ci` passes.

## Suggested PR sequence

1. **RFC freeze** — answer open questions; merge this document as Accepted.
2. **PR: Locations value + env capture** — the value, cache-placement distinction, one-instance `ExecutionPaths` carriage through launcher assembly, `diagnostics::cache` reduction, and test-guard deletion.
3. **PR: `adapter add` + per-component provenance** — land the pre-init seeding verb and `ComponentMeta` split before removing the fallback it replaces.
4. **PR: delete Cargo probe + unpinned identity + contract sync** — `locate`, `AdapterLocation::Cache`, optional versions, bare-string topology targets, launcher predicate, label rename, help/error text, Makefile, AGENTS, workflow, architecture, and release notes land together.
5. **PR: prose overlay deletion** — delete the unused SDK branch and test; independent of the location cut and may land earlier.

## Appendix — current inventory (pre-change)

Primary production touchpoints to retire or rewrite:

| Area | Path |
| ---- | ---- |
| Cargo probe | `crates/project/src/adapter/resolver.rs` (`dev_component_*`, `locate`) |
| `Dev` location | `crates/project/src/adapter/core.rs` |
| `dev_version` | `crates/project/src/adapter/core.rs`, `resolver.rs` |
| Bare ensure no-op docs | `crates/project/src/adapter/ensure.rs` |
| Selector / help copy | `selector.rs` (GitHub-refusal text), `crates/transport/src/command.rs` |
| Init fingerprint keyed on origin label | `crates/project/src/init/context.rs` |
| Topology `0.0.0` skip | `crates/launcher/src/closure.rs` |
| Preflight “development” wording | `crates/launcher/src/preflight.rs`, `hydrate.rs` |
| Ambient env reads / wasm mount branches | `crates/diagnostics/src/cache.rs` |
| Single-file cache provenance | `crates/project/src/adapter/ensure.rs` (`ComponentMeta`) |
| Example seed | `examples/Makefile.toml`, `examples/wasm/README.md` |
| Test staging + env guards | `crates/project/tests/support/mod.rs` (`stage_dev_component`, `scoped_store`), launcher tests (`EnvGuard`) |
| Prose overlay | `crates/adapter/src/registry.rs` |
| Contract prose | `AGENTS.md`, `docs/standards/{workflow,architecture}.md`, init/cli docs |

Explicitly retained (documented exemption): `selector::name_from_component` Cargo-filename normalization of operator-supplied components.

## Appendix — worked construction

```rust
// Shipped binary / launcher composition root: env is the override,
// defaults are production. Captured once, carried thereafter.
let paths = ExecutionPaths::operator(root); // internally: Locations::from_env()

// Engine guest (wasm32): already-resolved mounts, no env and no
// project-id suffix below /specify-cache.
let paths = ExecutionPaths::guest(); // explicit(store, CachePlacement::Project(cache))

// Sandbox / test: explicit tempdirs, no env mutation.
let locations = Locations::explicit(
    store_tmp.path().into(),
    CachePlacement::Parent(cache_tmp.path().into()),
);
let paths = ExecutionPaths::isolated_with(project_root, locations);
```

Prior art: `ConnectOptions` in `backends/crates/kafka` — `#[derive(FromEnv)]` with `#[env(from = "KAFKA_BROKERS")]` fields and literal defaults, finalized once in `omnia::FromEnv::from_env()` at the composition root, then carried as a value into `Backend::connect_with`. `Locations` is the same shape with path-typed fields and computed defaults applied at finalization.
