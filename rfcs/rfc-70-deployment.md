# Self-Assembling Wasm Deployment

> Status: Draft
>
> Owns: the operator-facing `specify` executable, deployment assembly, and the transition from a hand-authored Omnia guest list to a derived runtime deployment.
>
> Builds on: [Specify on Omnia](architecture.md) and [Native Specify](native-deployment.md).

## Abstract

Make the installed `specify` executable the only command an operator needs to run. The executable remains an Omnia command-mode runtime hosting the Specify engine guest and independently versioned adapter components, but it assembles that deployment itself.

The target operator surface is:

```bash
specify init omnia@0.5.0 --name payments
specify plan author migrate-payments --source legacy=typescript@0.5.0:./legacy
specify plan transition migrate-payments approved
specify plan execute
```

Operators do not write `omnia.toml`, locate the engine component, enumerate adapter guests, configure adapter MCP routes, or provide cache and store mounts. Those are derived deployment details.

This RFC deliberately does not collapse adapters into the native executable. Source adapters and target adapters remain separate Wasm components selected by identity.

## Motivation

The composed Wasm example currently requires three separate concerns in one operator-maintained file:

- the Specify engine component;
- one guest entry per source adapter and target adapter;
- runtime routes and mounts required by those guests.

The example then wraps Omnia's generic invocation:

```bash
run() {
  cargo run -p specify -- run --config examples/wasm/omnia.toml -- "$@"
}
```

The wrapper has the desired command shape, but it does not own deployment closure. Adding a new adapter still requires changing the Omnia configuration before the engine can dispatch to it.

The repository already has most of the required substrate:

- the shipped binary is an Omnia runtime;
- engine-to-adapter calls carry an axis-qualified adapter id;
- adapter selectors preserve bare, package, and local-component inputs;
- `Resolver::ensure_source` and `Resolver::ensure_target` hydrate or mirror components;
- the global adapter store and per-project cache have stable locations;
- the architecture defines the generated deployment manifest as derived state.

The missing layer is deployment assembly around those pieces.

## Goals

1. Make `specify <args...>` the ordinary Wasm operator invocation.
2. Preserve dynamically selected, independently versioned adapter components.
3. Derive the runtime's guest inventory, routes, links, and mounts.
4. Keep the Omnia runtime generic and free of Specify domain vocabulary.
5. Preserve exact package pins, digest verification, host-CLI floors, and local development components.
6. Give operators a diagnostic view of the effective deployment without making its generated configuration an authored artifact.
7. Provide an incremental path that works before Omnia supports lazy component resolution.



## Non-goals

- Replacing Omnia with a bespoke runtime.
- Statically linking first-party adapters into the released Wasm distribution.
- Making the native host the default operator distribution.
- Selecting which adapters a project should use. [Adapter Descriptors and Registry Trust](rfc-71-adapter-discovery.md) owns the descriptor and trust substrate; [Migration Intake and Source Selection](rfc-72-migration-intake.md) and [Migration Programs](rfc-74-migration-program.md) own the selection surfaces.
- Adding engine orchestration to the launcher.
- Making generated deployment state part of the project artifact model.
- Hot-reloading a component during an active workflow operation.



## Decision



### One product executable, two internal layers

The installed `specify` executable has two narrowly separated layers:

1. **Deployment launcher** — resolves the project root, computes the required component closure, materializes a derived Omnia deployment, and starts the runtime.
2. **Omnia runtime** — hosts the engine component, satisfies its effect imports, dispatches axis-qualified adapter calls, and forwards the command to the engine guest.

The launcher may understand adapter selectors and deployment files. It must not parse or implement plan, slice, validation, or lifecycle semantics.

The engine guest remains the only `wasi:cli/run` exporter. Command envelopes and exit codes pass through unchanged.

### Derived deployment state

The launcher writes deployment state beneath the out-of-tree per-project cache:

```text
<project-cache>/deployment/
├── omnia.toml
└── resolution.json
```

`omnia.toml` is an implementation detail consumed by the current Omnia runtime. `resolution.json` records enough information for diagnostics:

- Specify binary and engine component versions;
- each adapter's axis, selector, resolved identity, component path, origin, and digest;
- the generated routes, links, and mounts;
- the project root and cache/store mount sources;
- the input fingerprint used to decide whether regeneration is required.

For the `payments` project from the abstract — target `specify:omnia@0.5.0`, one plan-bound source `legacy=typescript@0.5.0:./legacy`, binary version `0.28.0` — the derived `omnia.toml` is the closure projected onto Omnia's guest / route / mount shape. Guests and MCP routes are not independently authored: every adapter guest that exports `wasi:http/incoming-handler` gets exactly one route by the [MCP route projection](#mcp-route-projection) rule below.

```toml
# GENERATED by specify 0.28.0 — never committed, never edited.
# Regenerated whenever the resolution fingerprint changes; safe to delete.

[[guest]]
id = "specify"
source.path = "/home/op/.specify/adapters/engine@0.28.0.wasm"
link = ["specify:adapter/source@0.1.0", "specify:adapter/target@0.1.0"]

[[guest]]
id = "source:typescript"
source.path = "/home/op/.specify/adapters/typescript@0.5.0.wasm"

[[guest]]
id = "target:omnia"
source.path = "/home/op/.specify/adapters/omnia@0.5.0.wasm"

[[route.http]]
prefix = "/mcp/typescript"
guest = "source:typescript"

[[route.http]]
prefix = "/mcp/omnia"
guest = "target:omnia"

[[mount]]
name = "."
path = "/work/payments"
writable = true

[[mount]]
name = "/specify-cache"
path = "/home/op/.cache/specify/projects/6b1f…"
writable = true

[[mount]]
name = "/specify-store"
path = "/home/op/.specify/adapters"
writable = true
```

#### MCP route projection

MCP routes are a pure function of the adapter guest set. The launcher never invents, elides, or hand-tunes prefixes.

| Input | Output |
| ----- | ------ |
| Adapter guest id `source:<name>` or `target:<name>` that exports `wasi:http/incoming-handler` | `[[route.http]]` with `prefix = "/mcp/<name>"` and `guest` equal to that id |
| Engine guest (`specify`) | no MCP route — it is not a references server |

The formula is `/mcp/<name>`: the store already keys components by name with no axis segment, and first-party adapter names are unique across axes, so the axis need not appear in the URL. Guest ids stay axis-qualified (`source:typescript`, `target:omnia`); only the HTTP prefix drops the axis.

When a closure contains the same name on both axes (unpublished fixtures such as `source:mock` / `target:mock`; never a published first-party pair), the projection falls back to `/mcp/<axis>/<name>` so the two HTTP surfaces stay distinct. Doctor treats a prefix collision under the ordinary rule as a deployment error.

The hand-authored `examples/wasm/omnia.toml` is a development stand-in for this projection, not a second vocabulary: it must follow the same formula (including the dual-axis fallback for `mock`).

The matching `resolution.json` carries the provenance the TOML flattens away:

```json
{
  "specify": "0.28.0",
  "engine": {
    "package": "specify:engine@0.28.0",
    "component": "/home/op/.specify/adapters/engine@0.28.0.wasm",
    "origin": "registry",
    "digest": "sha256:9c41…"
  },
  "adapters": [
    {
      "axis": "source",
      "selector": "specify:typescript@0.5.0",
      "resolved": "typescript@0.5.0",
      "component": "/home/op/.specify/adapters/typescript@0.5.0.wasm",
      "origin": "store",
      "digest": "sha256:5b0e…"
    },
    {
      "axis": "target",
      "selector": "specify:omnia@0.5.0",
      "resolved": "omnia@0.5.0",
      "component": "/home/op/.specify/adapters/omnia@0.5.0.wasm",
      "origin": "store",
      "digest": "sha256:1d9c…"
    }
  ],
  "project": {
    "root": "/work/payments",
    "cache": "/home/op/.cache/specify/projects/6b1f…",
    "store": "/home/op/.specify/adapters"
  },
  "fingerprint": "sha256:d81f…"
}
```

Neither file is committed or hand-edited. The authored selectors remain in `project.yaml`, `plan.yaml`, and command inputs. A generated deployment never becomes a second source of truth.

### Required component closure

The closure for a command is the union of:

- `specify:engine@<binary-version>`;
- the current project's target adapter;
- target adapters declared by materialized workspace slots relevant to the active plan;
- source adapters bound in the active plan;
- source adapters behind approved catalogue bindings referenced through `@key` selectors in the current command;
- source or target selectors present in the current command's typed arguments;
- the temporary first-party bootstrap profile described below.

Entries are deduplicated by `(axis, name, version, digest)`. A same-name source and target remain distinct runtime guest ids even where unpublished fixtures use both axes.

A worked example. The authored state for the `payments` project is two files the operator already owns:

```yaml
# .specify/project.yaml (written by `specify init omnia@0.5.0 --name payments`)
name: payments
adapter: specify:omnia@0.5.0
specify: 0.28.0
```

```yaml
# plan.yaml (written by `specify plan author migrate-payments --source legacy=typescript@0.5.0:./legacy`)
name: migrate-payments
sources:
  legacy:
    adapter: typescript@0.5.0
    path: ./legacy
slices: [...]
```

For `specify plan execute` the launcher derives:


| Closure entry              | Guest id            | Reason                     |
| -------------------------- | ------------------- | -------------------------- |
| `specify:engine@0.28.0`      | `specify`           | always — the engine guest  |
| `specify:omnia@0.5.0`      | `target:omnia`      | `project.yaml.adapter`     |
| `specify:typescript@0.5.0` | `source:typescript` | `plan.yaml.sources.legacy` |


`specify slice refine` and `specify slice build` on the same project derive the same three entries. `specify journal show` reaches no adapter, so the engine guest alone would satisfy it; in practice the launcher reuses the persisted deployment while its fingerprint still matches and regenerates only when the project or plan bindings change. A `specify init vectis@1.2.0 --name other` in a different project contributes nothing here: closures never leak across project caches.

The closure includes only components that the command can reach. It does not scan the global store and expose every installed adapter.

### Bootstrap profile

There is a bootstrap cycle before lazy loading exists:

1. the engine guest parses the command and calls `ensure_*`;
2. `ensure_*` may install a previously missing component;
3. Omnia must already have a guest entry before it can dispatch the component's `metadata` export.

The first implementation breaks that cycle with a release-versioned **first-party bootstrap profile**:

- release packaging installs the Specify engine component and the supported first-party adapter set into the global store;
- the launcher includes those known identities while processing `init` and `plan author`;
- after project and plan bindings exist, ordinary commands use the exact derived closure;
- third-party components must be installed through an explicit selector-bearing command or configuration overlay until lazy resolution lands.

The bootstrap profile is a transition mechanism, not a permanent adapter registry. It must be generated from the release inventory and tested against the first-party adapter index; it must not be a hand-maintained list in Rust.

Concretely, release packaging emits one generated document beside the platform binaries and embeds it in the launcher:

```json
{
  "schema-version": 1,
  "specify": "0.28.0",
  "engine": { "package": "specify:engine@0.28.0", "digest": "sha256:9c41…" },
  "adapters": [
    { "axis": "source", "package": "specify:intent@0.9.0",        "digest": "sha256:44aa…" },
    { "axis": "source", "package": "specify:documentation@0.7.2", "digest": "sha256:0be1…" },
    { "axis": "source", "package": "specify:typescript@0.5.0",    "digest": "sha256:5b0e…" },
    { "axis": "source", "package": "specify:screenshots@0.5.1",   "digest": "sha256:e77d…" },
    { "axis": "source", "package": "specify:captures@0.4.0",      "digest": "sha256:83f2…" },
    { "axis": "target", "package": "specify:omnia@0.5.0",         "digest": "sha256:1d9c…" },
    { "axis": "target", "package": "specify:vectis@1.2.0",        "digest": "sha256:71c8…" },
    { "axis": "target", "package": "specify:contracts@0.8.3",     "digest": "sha256:2ab4…" }
  ]
}
```

Every entry is an exact identity with the digest recorded at publication — the profile widens the guest inventory during `init` and `plan author`, but installation and dispatch still flow through the same store-verify path as an explicit pin. A CI assertion compares the generated document against the first-party adapter index, so a released binary cannot carry a stale or hand-edited profile.

### First-launch engine installation

The engine guest reaches the store the same way every adapter does: on first launch the launcher hydrates `specify:engine@<binary-version>` from the registry into the global store and verifies the recorded digest. There is no `include_bytes!` payload — the architecture fixes the binary version as the engine version with no committed or embedded guest artifact ([Specify on Omnia §CLI bootstrapping](architecture.md#cli-bootstrapping)) — and one hydration path keeps the operator-local CLI and a hosted deployment identical apart from the bound store backend.

Offline and air-gapped installations are served by the release archive shipping the engine and first-party adapter components *beside* the binary; the launcher installs them into the store through the same verify-on-write path, recorded as `origin: release-archive`. The store entry is canonical in every case; nothing executes outside it.

After first launch and one `init` plus `plan author` on the `payments` project, the global store carries the same entry-plus-sidecar pairs the existing hydration path writes today:

```text
~/.specify/adapters/
├── engine@0.28.0.wasm          # hydrated on first launch (origin: registry)
├── engine@0.28.0.meta          # tree-digest: sha256:9c41…
├── omnia@0.5.0.wasm          # hydrated by `specify init omnia@0.5.0` (origin: registry)
├── omnia@0.5.0.meta
├── typescript@0.5.0.wasm     # hydrated by `plan author --source legacy=typescript@0.5.0:…`
└── typescript@0.5.0.meta
```



### Typed preflight, not a second CLI

The launcher reuses transport argument types to extract selectors from `init` and `plan author`. It does not define a second clap grammar.

Illustrative shape — the preflight is a thin projection over the grammar the engine guest already owns (`transport::command`'s `InitArgs` / plan `AuthorArgs`) and the existing `AdapterSelector::parse`:

```rust
/// What preflight learned from one command line. Never a parallel
/// interpretation: `Profile` means "could not project — defer to the
/// bootstrap profile and the guest's own resolution".
enum Preflight {
    Selectors(Vec<(Axis, AdapterSelector)>),
    Profile,
}

fn preflight(argv: &[String]) -> Result<Preflight, Error> {
    match transport::command::peek(argv)? {
        Peeked::Init(args) => Ok(match args.adapter {
            Some(token) => Preflight::Selectors(vec![(
                Axis::Target,
                AdapterSelector::parse(&token)?, // same errors as the guest:
            )]),                                 // adapter-github-uri-unsupported, …
            None => Preflight::Profile,          // `init --workspace`
        }),
        Peeked::PlanAuthor(args) => args
            .sources
            .iter()
            .map(|assign| Ok((Axis::Source, AdapterSelector::parse(&assign.adapter)?)))
            .collect::<Result<_, Error>>()
            .map(Preflight::Selectors),
        // Every other verb: the persisted closure (project.yaml +
        // plan.yaml) already names the components; no argv projection.
        Peeked::Other => Ok(Preflight::Profile),
    }
}
```

The sketch's load-bearing property is that `AdapterSelector::parse` and the transport `Args` types are the same items the guest compiles — a selector the launcher accepts but the guest refuses (or vice versa) is impossible by construction, and any residual disagreement fails closed as a deployment-preflight error rather than a half-assembled runtime.

Catalogue selectors do not name adapters directly: `plan author --source @key` ([Migration Intake and Source Selection](rfc-72-migration-intake.md)) resolves through the committed `sources.yaml`. Preflight reads the catalogue read-only to project approved bindings into the closure; when the catalogue is absent or a referenced binding is not approved, preflight falls back to the bootstrap profile and the engine guest's own resolution remains authoritative.

If an argument shape cannot be projected without duplicating command parsing, the launcher starts with the bootstrap profile and lets the engine guest remain authoritative. Parse disagreement must fail closed and report that deployment preflight could not determine the component closure.

### Runtime invocation

The launcher calls Omnia's in-process drive surface. It does not spawn an `omnia` subprocess and does not shell out to the installed `specify` binary recursively.

The whole launcher `main` is four steps around the host set the shipped binary already declares (`WasiHttp`, `WasiOtel`, `WasiModel` over the Cursor backend in `src/omnia.rs`):

```rust
fn main() -> std::process::ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();

    // 1. Anchor: the project root and its out-of-tree cache.
    let project = resolve_project_root();

    // 2. Closure: persisted resolution when the fingerprint matches,
    //    else recompute from project/plan/preflight inputs.
    let closure = match closure::load_or_compute(&project, &argv) {
        Ok(closure) => closure,
        Err(err) => return report_preflight(err), // fail closed, nothing started
    };

    // 3. Materialize: write omnia.toml + resolution.json beneath
    //    <project-cache>/deployment/. Pure projection of the closure.
    let deployment = deployment::materialize(&project, &closure);

    // 4. Drive: in-process Omnia command mode; argv and the engine
    //    guest's exit code pass through byte-for-byte.
    omnia::drive(deployment.manifest(), &argv)
}
```

Steps 1–3 never interpret the command beyond preflight selector projection; step 4 never sees selectors. The layer boundary from the decision above is the function boundary here.

The runtime configuration continues to carry only deployment concerns:

- guest component locations;
- WIT interface link allow-lists;
- MCP routes (projected from the adapter guest set — never a separate authored table);
- project, cache, and store mounts;
- host backend configuration.

Workflow artifacts and lifecycle state remain invisible to Omnia.

### Diagnostic commands

Add a deployment-oriented read surface:

```bash
specify deployment show [--format json]
specify deployment doctor [--format json]
```

`show` projects the effective closure and generated configuration. `doctor` verifies:

- engine and adapter components exist;
- recorded digests match;
- every adapter exports the expected axis world;
- metadata floors are compatible;
- project/cache/store mounts resolve;
- MCP routes match the [projection rule](#mcp-route-projection) and are collision-free;
- the generated deployment fingerprint matches its inputs.

`show --format json` is a projection of `resolution.json` (the example above) plus the derived guest/route/mount tables — it never re-resolves. `doctor` re-verifies and reports through the ordinary finding currency, for example after a store entry has been tampered with:

```console
$ specify deployment doctor
engine  specify:engine@0.28.0   ok
target  omnia@0.5.0           ok
source  typescript@0.5.0      adapter-digest-mismatch: recorded sha256:5b0e… but recomputed sha256:99d1…
mounts  project cache store   ok
routes  2 prefixes            ok
fingerprint                   ok
$ echo $?
2
```

These commands do not install, mutate, or select adapters.

## Omnia follow-up: lazy guest resolution

Most of the plumbing already exists on the runtime side. Host-mediated dynamic linking treats adapter identity as *data* (the `adapter-id` call argument), the `GuestRegistry` selects an `InstancePre` by identity on every call, and the architecture already notes the mechanism "supports dynamic (config-driven, OCI-resolved) adapter selection" ([Specify on Omnia §Many guests, selected by identity](architecture.md#many-guests-selected-by-identity)). The only constraint this RFC works around is that the registry preloads *only manifest-named identities*. The missing capability is therefore narrow: let the registry consult a deployment-supplied resolver on a miss. This resolves open question territory in favour of the registry-miss hook rather than a general guest-source interface — the hook rides the existing dispatch path and adds no new seam.

Because the change is narrow, the preferred build order is **Omnia hook first**: land the resolver hook before (or alongside) Stage 1, in which case the bootstrap profile and the selector-aware typed preflight below are never built — Stage 1 collapses to the launcher, the derived effects/mounts/routes configuration, and the resolver policy. The bootstrap profile and preflight remain specified as the fallback plan if Omnia sequencing does not cooperate; they are contingency, not the destination.

The end state removes the bootstrap profile and most command preflight. Omnia gains a deployment-supplied guest resolver:

```text
adapter-id + expected WIT world
    → exact component identity
    → verified component bytes
    → cached InstancePre
```

On the first `source:*` or `target:*` call, the runtime asks the resolver for the component, validates the expected export, instantiates it, and caches the compiled component for later calls. The resolver is configured with an allow-list and store policy; it does not search arbitrary network locations.

A contract sketch, phrased from Omnia's side of the seam:

```rust
/// Deployment-supplied guest resolution. Omnia calls this on the
/// first dispatch to a guest id absent from the static guest list.
trait GuestResolver: Send + Sync {
    /// Map `source:typescript` / `target:omnia` + the expected WIT
    /// world to verified component bytes. Refusal is a dispatch error
    /// on the caller; the runtime never falls back to a broader search.
    fn resolve(&self, guest_id: &str, expected_world: &str)
        -> Result<ResolvedGuest, ResolveRefused>;
}

struct ResolvedGuest {
    /// Exact identity for diagnostics and the compilation cache key.
    identity: String,          // "specify:typescript@0.5.0"
    digest: String,            // verified before compilation
    bytes: Vec<u8>,
}
```

Specify's implementation of that trait is the existing store probe plus verify-on-read — the same code path `ensure_*` leaves its results in — behind an allow-list carried by the generated manifest. After Stage 3 the derived `omnia.toml` shrinks to the engine guest, effects, mounts, and that policy block; no per-adapter `[[guest]]` entries and no per-adapter `[[route.http]]` rows remain. HTTP dispatch uses the same identity rule as dynamic linking: the host maps `/mcp/<name>` (or `/mcp/<axis>/<name>` under the dual-axis fallback) onto a guest id and resolves that id through the registry-miss hook ([Specify on Omnia §Many guests, selected by identity](architecture.md#many-guests-selected-by-identity)).

```toml
[[guest]]
id = "specify"
source.path = "/home/op/.specify/adapters/engine@0.28.0.wasm"
link = ["specify:adapter/source@0.1.0", "specify:adapter/target@0.1.0"]

[resolver]
store = "/home/op/.specify/adapters"
allow = ["source:*", "target:*"]     # axis worlds only; ids resolve via the store
verify = "digest"                     # sidecar digest required before compilation

# [[mount]] entries unchanged from the Stage 1–2 example.
# No [[route.http]] — prefixes are computed from the request path by the
# same /mcp/<name> rule Stage 1–2 materialised as a static table.
```

This is an Omnia runtime capability, not Specify engine code. Specify supplies the resolver policy and store implementation through deployment composition.

After lazy resolution lands:

- the generated manifest names the engine guest, effects, mounts, and resolver policy;
- it no longer names every adapter guest or every MCP route;
- inbound MCP traffic and engine→adapter calls share one identity resolution path;
- a component installed by `ensure_*` becomes dispatchable in the same process;
- third-party adapters follow the same path as first-party adapters.



## Omnia follow-up: typed deployment value

A second, independent runtime simplification: `omnia::drive` is already mounted in-process, so if it accepts a **typed deployment value** instead of a manifest file path, the launcher stops writing `omnia.toml` altogether. The derived-state directory shrinks to `resolution.json` — pure diagnostics — and the regeneration fingerprint disappears with the file it guarded (the closure is simply recomputed or reused in memory). Nothing in this RFC's semantics changes; only the materialization step evaporates.

This also serves the dual deployment posture directly: steps 1–4 of the launcher are deployment-neutral, and a hosted runner composing the same closure should not need to synthesize a TOML file on a node-local filesystem to start a runtime it holds in process. The file format remains available for hand-authored deployments such as the composed example.

## Supply-chain posture

Deployment assembly must not turn model output into executable trust.

- Only exact package identities or explicitly supplied local components may be loaded.
- Package components must pass digest verification before compilation.
- Registry namespace and publisher policy are evaluated before hydration.
- Auto-selection may recommend an adapter, but installation requires the trust policy in [Adapter Descriptors and Registry Trust](rfc-71-adapter-discovery.md) and the approval surfaces in [Migration Intake and Source Selection](rfc-72-migration-intake.md) and [Migration Programs](rfc-74-migration-program.md).
- The generated deployment records origin and digest for every component.
- A component cannot gain a filesystem or network capability merely by declaring one; Omnia's configured host links remain authoritative.



## Implementation stages

Stages 1–2 are independent of the migration RFC set and ship on their own — they are the near-term operator-ergonomics win. Stage 3 is the piece [Migration Programs](rfc-74-migration-program.md) eventually wants, because it lets a component installed mid-run become dispatchable without regenerating the guest list.

The stage order below is the fallback plan. Since the Stage 3 runtime change is a narrow registry-miss hook (see [Omnia follow-up: lazy guest resolution](#omnia-follow-up-lazy-guest-resolution)), the preferred order lands it first — in which case Stage 1 skips the bootstrap-profile generation and Stage 2 skips the selector preflight, and neither is ever built.

### Stage 1 — Direct invocation with the first-party profile

1. Add the launcher entry around the existing runtime macro.
2. Generate a release inventory for the engine component and first-party adapters.
3. Generate the Omnia configuration and mounts in the project cache.
4. Forward all workflow arguments and exits unchanged.
5. Change the composed example to invoke `specify ...` directly.



### Stage 2 — Exact project closure

1. Project typed command inputs onto adapter selectors where required for bootstrap.
2. Resolve target selectors from project and workspace-slot configuration.
3. Resolve source selectors from the active plan.
4. Persist `resolution.json` and add `deployment show|doctor`.
5. Use the first-party profile only for commands that have no persisted closure yet.



### Stage 3 — Omnia lazy resolver

1. Define the deployment-side guest-resolver contract in Omnia.
2. Load verified components by axis-qualified adapter id.
3. Cache compilation, not guest instance state.
4. Remove the bootstrap profile and selector-aware command preflight.
5. Retain generated configuration only for effects, mounts, routes, and resolver policy.



## Acceptance criteria

1. A released Wasm installation runs `specify --help` and every workflow command without `run --config`.
2. The ordinary operator path requires no authored `omnia.toml`.
3. Adding a supported first-party source binding does not require editing runtime configuration.
4. Project and plan selectors remain the authoritative adapter bindings.
5. The generated deployment lives outside the repository and is safe to delete.
6. The engine guest remains the sole owner of command semantics and lifecycle transitions.
7. The launcher and workflow command grammar share typed argument definitions.
8. Exact pins and component digests are visible in `specify deployment show --format json`.
9. A missing, incompatible, wrong-axis, or digest-mismatched component fails before its operation runs.
10. The native host remains a separate deployment and does not become a fallback for missing Wasm components.
11. The composed Wasm example and at least one third-party-style local component exercise the same operator invocation.
12. After Stage 3, a component installed by `ensure_*` is callable without restarting into a newly generated guest list.



## Testing

- Closure computation, deployment generation, fingerprinting, and `deployment show|doctor` are crate-level integration tests over fixture stores and caches; no live registry access in CI.
- Missing, digest-mismatched, wrong-axis, and floor-incompatible components are asserted at the CLI boundary through exit codes and the kebab-case error discriminants.
- The launcher-to-runtime seam is owned by the operator-invoked composed wasm example (`cargo make wasm-run`), updated to invoke `specify ...` directly per acceptance criterion 11; there is no new per-push WASM gate.



## Open questions

1. Should `deployment show|doctor` be top-level commands or a projection under `adapter`?
2. During Stage 2, should a workspace closure include every materialized slot target or only targets reachable from the active plan?
3. Should local component selectors require an explicit trust flag when they resolve outside the project root?
4. Which third-party installation command breaks the bootstrap cycle before lazy resolution lands without reintroducing a general package-manager surface into the engine CLI? (Moot if the Omnia hook lands first.)

