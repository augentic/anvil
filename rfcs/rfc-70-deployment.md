# Self-Assembling Wasm Deployment

> Status: Draft
>
> Owns: the operator-facing `specify` executable, deployment assembly, and Specify's composition of Omnia's generic guest-resolver onto the adapter store.
>
> Builds on: [Specify on Omnia](architecture.md) and [Native Specify](native-deployment.md).
>
> Program: RFCs 70–74 are a coordinated Omnia + Specify change set. Omnia remains a generic WebAssembly runtime with no Specify domain vocabulary; Specify owns closure, store policy, and launcher composition.

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

Adapters stay separate Wasm components selected by identity. Omnia loads them through a **deployment-supplied guest resolver** on registry miss — a generic runtime capability. Specify plugs store probe, path-glob policy, and digest verification into that hook; Omnia never learns `source:` / `target:` vocabulary, plan bindings, or workflow semantics.

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

The missing layer is deployment assembly around those pieces, plus one Omnia capability: consult a deployment-supplied resolver when the guest registry misses an identity. That hook is in scope for this program because Omnia and Specify move together; it is still specified as a **generic** runtime seam, not a Specify special case — though it touches more than the dispatch lookup (see [Lazy guest resolution](#lazy-guest-resolution)).

## Goals

1. Make `specify <args...>` the ordinary Wasm operator invocation.
2. Preserve dynamically selected, independently versioned adapter components.
3. Derive the runtime deployment: engine guest, mounts, effects, and resolver policy — not a hand-maintained per-adapter guest / route table.
4. Keep the Omnia runtime generic and free of Specify domain vocabulary.
5. Preserve exact package pins, digest verification, host-CLI floors, and local development components.
6. Give operators a diagnostic view of the effective deployment without making its generated configuration an authored artifact.
7. Land Omnia's registry-miss guest resolver in the same program so a component installed by `ensure_*` is dispatchable in-process without regenerating a static guest list.

## Non-goals

- Replacing Omnia with a bespoke runtime.
- Teaching Omnia Specify vocabulary (`source:` / `target:`, plan bindings, lifecycle, artifact schemas).
- Statically linking first-party adapters into the released Wasm distribution.
- Making the native host the default operator distribution.
- Selecting which adapters a project should use. [Adapter Descriptors and Registry Trust](rfc-71-discovery.md) owns the descriptor and trust substrate; [Migration Intake and Source Selection](rfc-72-migration.md) and [Migration Programs](rfc-74-program.md) own the selection surfaces.
- Adding engine orchestration to the launcher.
- Making generated deployment state part of the project artifact model.
- Hot-reloading a component during an active workflow operation.



## Decision



### One product executable, two internal layers

The installed `specify` executable has two narrowly separated layers:

1. **Deployment launcher** — resolves the project root, computes the required component closure for hydration and diagnostics, assembles the typed Omnia deployment value (engine guest, mounts, resolver policy), and starts the runtime.
2. **Omnia runtime** — hosts the engine component, satisfies its effect imports, dispatches guest calls by identity (static entries or resolver miss), and forwards the command to the engine guest.

The launcher may understand adapter selectors and the derived deployment. It must not parse or implement plan, slice, validation, or lifecycle semantics.

The engine guest remains the only `wasi:cli/run` exporter. Command envelopes and exit codes pass through unchanged.

### Omnia `runtime!` composition

`omnia::runtime!` expands both a CLI `main` and a reusable `run` over the same host-wiring path. Specify nests the macro in a host submodule, owns crate-root `main`, and calls that module's `run` after the launcher steps. Plain Omnia apps keep invoking the macro at crate root and using the generated `main`.

Specify's shipped binary therefore splits:

1. a host submodule that invokes `runtime!` with the existing host set (`WasiHttp` / `WasiOtel` / `WasiModel` over Cursor) — still Specify-vocabulary-free;
2. a crate-root `main` that runs the launcher steps below and calls that module's `run` with the assembled typed deployment value (see [Typed deployment value](#typed-deployment-value)).

Omnia's remaining composition deliverable for this program is that `run` accept a **typed deployment value** (today it takes a `DeploymentBuilder`); the file/flag path stays on the generated `main` for plain Omnia apps and hand-authored examples.

### Lazy guest resolution

Most of the plumbing already exists on the runtime side. Host-mediated dynamic linking treats adapter identity as *data* (the `adapter-id` call argument), the `GuestRegistry` selects an `InstancePre` by identity on every call, and the architecture already notes the mechanism "supports dynamic (config-driven, OCI-resolved) adapter selection" ([Specify on Omnia §Many guests, selected by identity](architecture.md#many-guests-selected-by-identity)). The missing capability is a **deployment-supplied** resolver consulted on a registry miss. The *import* side already rides the existing dispatch path — polyfilled imports resolve identity per call through the dispatch handle — but the hook touches three structures Omnia freezes at bootstrap today, all of which are Omnia deliverables in this program:

1. **Registry late insertion.** The registry map gains a concurrent-safe insert path: on miss, ask the resolver, compile, pre-instantiate against the shared linker, insert. Concurrent misses on one id are single-flight (losers await the winner's `InstancePre`); a refusal is **not** negatively cached, so an identity installed by a mid-run `ensure_*` becomes dispatchable on the next call.
2. **Serve-at-resolve.** The host-mediated link serve side is wired at bootstrap from the registered guest set; a late-resolved guest that exports a linked interface must gain its serve side at resolve time, or an engine→adapter call to it dispatches into a hole.
3. **Programmatic trigger routing.** HTTP triggers gain a deployment-supplied path→identity projection beside the static route table. Static-route validation (every route target must be registered) applies only to the static table; projected identities go through the ordinary registry lookup plus the miss-hook.

These land as an Omnia-side design note (*registry-miss guest resolution*) reviewed in `augentic/omnia`, so the generic-runtime constraint is defended by Omnia's own review. The typed-deployment argument to `run` is specified in the same note.

Omnia gains a generic guest resolver:

```text
guest id + expected WIT world
    → exact component identity
    → verified component bytes
    → cached InstancePre
```

On the first call to an identity absent from the static guest list, the runtime asks the resolver for the component, validates the expected export, instantiates it, and caches the compiled component for later calls. The resolver is configured with filesystem path globs and a digest policy; it does not search arbitrary network locations.

A contract sketch, phrased from Omnia's side of the seam — guest ids and worlds are opaque strings to Omnia:

```rust
/// Deployment-supplied guest resolution. Omnia calls this on the
/// first dispatch to a guest id absent from the static guest list.
trait GuestResolver: Send + Sync {
    /// Map a guest id + the expected WIT world to verified component
    /// bytes. Refusal is a dispatch error on the caller; the runtime
    /// never falls back to a broader search.
    fn resolve(&self, guest_id: &str, expected_world: &str)
        -> Result<ResolvedGuest, ResolveRefused>;
}

struct ResolvedGuest {
    /// Exact identity for diagnostics and the compilation cache key.
    identity: String,
    digest: String,            // verified before compilation
    bytes: Vec<u8>,
}
```

Specify's implementation is the existing store probe plus verify-on-read — the same code path `ensure_*` leaves its results in — constrained to path globs carried by the generated deployment. Guest-id → filename mapping (`<name>@<version>.wasm`) and axis-qualified ids (`source:typescript`, `target:omnia`) stay inside that implementor. Axis / world gating is the `expected_world` argument on the call.

HTTP dispatch uses the same identity rule as dynamic linking: the host maps `/mcp/<name>` (or `/mcp/<axis>/<name>` under the dual-axis fallback) onto a guest id and resolves that id through the registry-miss hook ([Specify on Omnia §Many guests, selected by identity](architecture.md#many-guests-selected-by-identity)). Prefixes are computed from the request path.

After this lands:

- the derived deployment value names the engine guest, effects, mounts, and resolver policy;
- inbound MCP traffic and engine→adapter calls share one identity resolution path;
- a component installed by `ensure_*` becomes dispatchable in the same process;
- third-party adapters follow the same path as first-party adapters.

### Derived deployment state

The launcher writes deployment diagnostics beneath the out-of-tree per-project cache:

```text
<project-cache>/deployment/
└── resolution.json     # Specify diagnostics
```

The deployment itself is a **typed value** handed to `run` in memory (see [Typed deployment value](#typed-deployment-value)); the ordinary path writes no `omnia.toml`. `resolution.json` records enough information for diagnostics:

- Specify binary and engine component versions;
- each adapter's axis, selector, resolved identity, component path, origin, and digest;
- resolver path globs, mounts, and engine link allow-list;
- the project root and cache/store mount sources;
- the input fingerprint used to decide whether the persisted resolution can be reused.

For the `payments` project from the abstract — target `specify:omnia@0.5.0`, one plan-bound source `legacy=typescript@0.5.0:./legacy`, binary version `0.28.0` — the derived deployment value, shown here in the TOML format the generated Omnia `main` / hand-authored path accepts, is:

```toml
# The typed deployment value specify 0.28.0 assembles in memory,
# rendered as TOML for illustration.

[[guest]]
id = "specify"
source.path = "/home/op/.specify/adapters/engine@0.28.0.wasm"
link = ["specify:adapter/source@0.1.0", "specify:adapter/target@0.1.0"]

[resolver]
# Filesystem roots only (Goal 4).
# Multi-root: global store + project component cache.
paths = [
  "/home/op/.specify/adapters/*.wasm",
  "/home/op/.cache/specify/projects/6b1f…/components/*.wasm",
]
verify = "digest"                     # sidecar digest required before compilation

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

`verify = "digest"` is **fail-closed**: an entry without a sidecar, or with a mismatched digest, is a resolver refusal (`adapter-sidecar-missing` / `adapter-digest-mismatch` on the dispatching caller). This is deliberately stricter than the engine's `verify_store_entry`, whose missing-sidecar fail-open remains for non-executable reads of legacy entries — anything reachable by the executable-loading path meets the stricter bar, and `ensure_*` has always written sidecars. `specify deployment doctor` reports sidecar-less store entries so operators can re-hydrate them before they become refusals.

#### MCP route projection

MCP identity is a pure function of the request path.

| Input | Output |
| ----- | ------ |
| Request path `/mcp/<name>` (ordinary case) | Guest id `source:<name>` or `target:<name>` resolved through the miss-hook; only guests that export `wasi:http/incoming-handler` succeed |
| Request path `/mcp/<axis>/<name>` | Dual-axis fallback when the same name exists on both axes (unpublished fixtures such as `mock`) |

The formula is `/mcp/<name>`: the store already keys components by name with no axis segment, and first-party adapter names are unique across axes, so the axis need not appear in the URL. Guest ids stay axis-qualified inside Specify's resolver implementor; only the HTTP prefix drops the axis. The projection is carried by Omnia's deployment-supplied path→identity routing hook ([Lazy guest resolution](#lazy-guest-resolution), deliverable 3); Specify supplies the `/mcp/<name>` formula as the deployment's projection function. The engine guest is reached through CLI and host-mediated linking.

Doctor treats a prefix collision under the ordinary rule as a deployment error.

The hand-authored `examples/wasm/omnia.toml` may still enumerate guests and routes as a development stand-in for the composed example; it must follow the same identity formula (including the dual-axis fallback for `mock`).

The matching `resolution.json` carries the provenance the deployment value flattens away:

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
  "resolver": {
    "paths": [
      "/home/op/.specify/adapters/*.wasm",
      "/home/op/.cache/specify/projects/6b1f…/components/*.wasm"
    ],
    "verify": "digest"
  },
  "project": {
    "root": "/work/payments",
    "cache": "/home/op/.cache/specify/projects/6b1f…",
    "store": "/home/op/.specify/adapters"
  },
  "fingerprint": "sha256:d81f…"
}
```

`resolution.json` is safe to delete. The authored selectors remain in `project.yaml`, `plan.yaml`, and command inputs. Beyond diagnostics, the recorded digests double as the resolver's host-held expectation for closure identities (see [Supply-chain posture](#supply-chain-posture)).

### Required component closure

The closure for a command is the union of components the launcher must **hydrate and record** for diagnostics:

- `specify:engine@<binary-version>`;
- the current project's target adapter;
- target adapters declared by materialized workspace slots only where reachable from the active plan — a materialized slot whose target no current plan references contributes nothing (`deployment doctor --all-slots` audits the whole workspace);
- source adapters bound in the active plan;
- source adapters behind approved catalogue bindings referenced through `@key` selectors in the current command;
- source or target selectors present in the current command's typed arguments.

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


`specify slice refine` and `specify slice build` on the same project derive the same three entries. `specify journal show` reaches only the engine guest; in practice the launcher reuses the persisted resolution while its fingerprint still matches and regenerates only when the project or plan bindings change. Closures are per project cache: a `specify init vectis@1.2.0 --name other` in a different project contributes nothing here.

The closure includes only components that the command can reach. With the miss-hook, a component hydrated mid-command becomes dispatchable in the same process.

### First-launch engine installation

The engine guest reaches the store the same way every adapter does: on first launch the launcher installs `specify:engine@<binary-version>` into the global store — from the release archive beside the binary when present, else by registry hydration — and verifies the recorded digest. The architecture fixes the binary version as the engine version ([Specify on Omnia §CLI bootstrapping](architecture.md#cli-bootstrapping)), and one hydration path keeps the operator-local CLI and a hosted deployment identical apart from the bound store backend.

The release archive is the **primary** distribution: it ships the engine and first-party adapter components *beside* the binary, and first launch installs them into the store through the same verify-on-write path, recorded as `origin: release-archive`. Registry hydration covers everything else — a bare binary install, a pin the archive does not carry, a third-party component. Pre-project hydration resolves its registry from the user-level `~/.specify/wasm-pkg.toml` when present, else the compiled default; a project's `.specify/wasm-pkg.toml` overrides both once it exists (precedence: project → user → compiled default — mirrored/enterprise installs set the user file). The store entry is canonical in every case.

`--version` is the launcher's one native fast path — answered from the binary version, which *is* the engine version. Every other invocation, including `--help`, forwards to the engine guest, which stays the sole owner of command semantics.

After first launch and one `init` plus `plan author` on the `payments` project, the global store carries the same entry-plus-sidecar pairs the existing hydration path writes today:

```text
~/.specify/adapters/
├── engine@0.28.0.wasm          # installed on first launch (origin: release-archive | registry)
├── engine@0.28.0.meta          # tree-digest: sha256:9c41…
├── omnia@0.5.0.wasm          # hydrated by `specify init omnia@0.5.0` (origin: registry)
├── omnia@0.5.0.meta
├── typescript@0.5.0.wasm     # hydrated by `plan author --source legacy=typescript@0.5.0:…`
└── typescript@0.5.0.meta
```

### Runtime invocation

The launcher calls `run` from the nested `runtime!` expansion in-process.

The whole launcher `main` is four steps; host wiring lives in the nested macro expansion:

```rust
// Host submodule (Specify-vocabulary-free):
mod host {
    use omnia_cursor::Client as Cursor;
    use omnia_wasi_http::{HttpDefault, WasiHttp};
    use omnia_wasi_model::WasiModel;
    use omnia_wasi_otel::{OtelDefault, WasiOtel};

    omnia::runtime!({
        mode: command,
        hosts: {
            WasiHttp: HttpDefault,
            WasiOtel: OtelDefault,
            WasiModel: Cursor,
        }
    });
}

// Launcher binary (crate root):
fn main() -> std::process::ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();

    // 1. Anchor: the project root and its out-of-tree cache.
    let project = resolve_project_root();

    // 2. Closure: persisted resolution when the fingerprint matches,
    //    else recompute from project/plan inputs (hydration + diagnostics).
    let closure = match closure::load_or_compute(&project, &argv) {
        Ok(closure) => closure,
        Err(err) => return report_preflight(err), // fail closed, nothing started
    };

    // 3. Assemble: the typed deployment value (engine guest, mounts,
    //    resolver policy, argv) + resolution.json beneath
    //    <project-cache>/deployment/.
    let deployment = deployment::assemble(&project, &closure, &argv);

    // 4. Run: in-process Omnia command mode; argv and the engine
    //    guest's exit code pass through byte-for-byte.
    host::run(deployment)
}
```

Steps 1–3 project selectors from project/plan/typed args; step 4 receives only the typed deployment. The layer boundary from the decision above is the function boundary here.

Selector projection reuses the typed clap grammar in `crates/transport` (wasm-clean, already consumed by the native host): the launcher parses argv into the same `Args` the engine guest will parse, folds selectors out of the typed values, and discards the rest. A command whose `Args` carry no selectors contributes nothing to the closure, and a new selector-bearing argument is picked up by the closure projection from the shared grammar in the same change. The launcher's parse is a superset gate — argv that fails the shared grammar fails closed before `run`, nothing started.

The runtime configuration continues to carry only deployment concerns:

- the engine guest and its WIT interface link allow-list;
- resolver path globs and digest policy;
- project, cache, and store mounts;
- host backend configuration.

Workflow artifacts and lifecycle state remain invisible to Omnia.

### Diagnostic commands

Add a deployment-oriented read surface:

```bash
specify deployment show [--format json]
specify deployment doctor [--format json] [--all-slots]
```

The commands are top-level: the noun covers the engine guest, mounts, resolver policy, and the fingerprint. `show` projects the effective closure and generated configuration. `doctor` verifies:

- engine and adapter components exist;
- recorded digests match;
- every recorded adapter exports the expected axis world;
- metadata floors are compatible;
- project/cache/store mounts resolve;
- resolver path globs cover every recorded component path;
- the persisted resolution fingerprint matches its inputs.

Beyond the closure, `doctor` audits the resolver's reachable surface: a store entry reachable by the resolver path globs but missing its digest sidecar is reported (it would be a fail-closed refusal on dispatch), and an entry reachable by the globs but absent from every recorded closure is flagged as an orphan — the residue the trust model in [Supply-chain posture](#supply-chain-posture) asks operators to watch until per-guest mount scoping lands. `--all-slots` widens the audit from the active plan's closure to every materialized workspace slot target; the ordinary closure stays minimal (only targets reachable from the active plan — see [Required component closure](#required-component-closure)).

`show --format json` projects `resolution.json`. `doctor` re-verifies and reports through the ordinary finding currency, for example after a store entry has been tampered with:

```console
$ specify deployment doctor
engine  specify:engine@0.28.0   ok
target  omnia@0.5.0           ok
source  typescript@0.5.0      adapter-digest-mismatch: recorded sha256:5b0e… but recomputed sha256:99d1…
mounts  project cache store   ok
resolver paths                ok
fingerprint                   ok
$ echo $?
2
```

### Typed deployment value

`run` accepts a **typed deployment value** instead of today's `DeploymentBuilder` (or a manifest file path). That lands from the first cut. The ordinary Specify path writes no `omnia.toml`; the derived-state directory carries only `resolution.json` from day one, and the fingerprint guards reuse of the persisted *resolution*.

The generated Omnia `main` keeps loading a file/flag deployment for plain Omnia apps and examples; that path continues to accept a TOML path. The file format remains available for hand-authored deployments such as the composed example.

This also serves the dual deployment posture directly: steps 1–4 of the launcher are deployment-neutral, and a hosted runner composing the same closure can start the runtime in process from the typed value alone.

**Fallback staging.** If Omnia review declines to land the typed value in the same release as the miss-hook, the launcher temporarily materializes the derived deployment as `<project-cache>/deployment/omnia.toml` (fingerprint-guarded, safe to delete) and passes a builder/path into `run`; that file evaporates when the typed value lands. This is a fallback, not the plan of record.

### Contingency: static guest inventory

If the Omnia miss-hook cannot land with the first launcher cut, Specify may temporarily materialize today's full guest / route table and a release-versioned **first-party bootstrap profile** so `init` / `plan author` can dispatch `metadata` before plan bindings exist. That path is contingency, not the destination:

- the launcher enumerates every closure adapter as `[[guest]]` plus projected `[[route.http]]` rows;
- release packaging embeds a generated first-party inventory for bootstrap widening;
- typed argv preflight may project selectors from `init` / `plan author` into that inventory;
- third-party components need an explicit selector-bearing install or overlay until the miss-hook lands.

The bootstrap profile must be generated from the release inventory and tested against the first-party adapter index; it must not be a hand-maintained list in Rust. The contingency evaporates when the miss-hook ships — no permanent dual mode.

## Supply-chain posture

Deployment assembly must not turn model output into executable trust.

- Only exact package identities or explicitly supplied local components may be loaded.
- Package components must pass digest verification before compilation.
- Registry namespace and publisher policy are evaluated before hydration.
- Auto-selection may recommend an adapter, but installation requires the trust policy in [Adapter Descriptors and Registry Trust](rfc-71-discovery.md) and the approval surfaces in [Migration Intake and Source Selection](rfc-72-migration.md) and [Migration Programs](rfc-74-program.md).
- The generated deployment records origin and digest for every component.
- A component cannot gain a filesystem or network capability merely by declaring one; Omnia's configured host links remain authoritative.
- Resolver path globs bound where bytes may be read; they do not grant network fetch or widen trust policy.

The digest sidecar is written through the same writable store mount every guest in the deployment shares, so sidecar verification proves **integrity, not trust**: a guest could install an entry and a self-consistent sidecar. Two mitigations bound this. First, for every identity in the recorded closure the resolver verifies against the digest the launcher recorded in `resolution.json` — a host-held expectation no guest can rewrite. Second, identities outside the recorded closure (a mid-run `ensure_*` install) are accepted on fail-closed sidecar verification alone; this trusts the deployment's guests, which today are the engine plus adapters the operator explicitly selected. Per-guest mount scoping in Omnia (the store writable only to the engine guest) is the hardening path and is tracked as an [open question](#open-questions); until it lands, `deployment doctor` flags store entries reachable by resolver globs but absent from every recorded closure.

Local component selectors that resolve outside the project root require no additional trust flag: the operator typing an explicit component path is the approval act, consistent with how local components mirror today. The canonical origin path is recorded in `resolution.json`, and `doctor` surfaces out-of-root origins as informational findings. Revisit only if the trust policy in [Adapter Descriptors and Registry Trust](rfc-71-discovery.md) gives a flag something real to gate on.



## First delivery

The first cut must be something an in-house team can run daily: `specify …` with no authored Omnia config, first-party adapters hydrating into the store, and clear resolve failures. It does not need a polished diagnostics product or third-party trust theatre.

**In first delivery**

- Stages 0–1 below (miss-hook, typed `run`, nested launcher, closure hydrate, fail-closed sidecar verify).
- Release archive *or* registry hydrate for the engine and first-party pins the team actually uses.
- Kebab-case resolve/dispatch errors that name the missing or mismatched identity.

**Deferred until the team needs them**

| Capability | Pull in when |
| ---------- | ------------ |
| `resolution.json`, fingerprint reuse | Re-resolve cost or “what’s loaded?” support load |
| `deployment show\|doctor`, orphan audits | Store/digest failures are hard to diagnose from exit text alone |
| Host-held digests in `resolution.json` | Mid-run / third-party install makes guest-writable sidecars a real concern |
| MCP path→identity on the ordinary path | In-house workflows need adapter MCP references through the product binary |
| User-level `~/.specify/wasm-pkg.toml` | Mirrored registry installs outside project config |
| Fallback `omnia.toml` / static guest contingency | Omnia typed `run` or miss-hook slips a release |

Program sequencing: [RFC-74 §First delivery](rfc-74-program.md#first-delivery).

## Implementation stages

Stages below are the planned order for the coordinated Omnia + Specify cut. The [contingency](#contingency-static-guest-inventory) exists only if the miss-hook slips.

### Stage 0 — Omnia design note

Land the Omnia-side design note (*registry-miss guest resolution*) in `augentic/omnia`, covering the miss-hook deliverables below plus the typed-deployment argument to `run`, so the generic-runtime constraint is defended by Omnia's own review.

### Stage 1 — Omnia miss-hook, typed `run`, and direct invocation (first delivery)

Omnia:

1. Change `run` to take a typed deployment value; the file/flag path remains on the generated `main` for plain Omnia apps (see the [fallback staging](#typed-deployment-value) if the typed value must slip a release).
2. Define the deployment-side `GuestResolver` contract (opaque guest id + expected world → verified bytes) with registry late insertion: single-flight on concurrent misses, no negative caching of refusals.
3. Serve-at-resolve: a late-resolved guest exporting a linked interface gains its host-mediated serve side at resolve time.

Specify:

4. Implement the store-backed resolver behind path-glob + fail-closed sidecar digest policy (closure digests recorded in `resolution.json` wait for Stage 2).
5. Split the native binary: nest `runtime!` in a host submodule; crate-root launcher `main` owns closure + assemble + `host::run(…)`, projecting selectors through the shared `crates/transport` grammar.
6. Assemble the typed deployment value in memory — engine guest, mounts, resolver policy; no per-adapter guest / route table, no `omnia.toml`.
7. Hydrate closure components before `run` when missing from the store (release archive when present, else registry).
8. Forward all workflow arguments and exits unchanged; `--version` is the launcher's only native fast path.
9. Change the composed example to invoke `specify ...` directly.
10. Confirm mid-run `ensure_*` installs are dispatchable without regenerating runtime configuration.

### Stage 2 — Diagnostics and hardened verify

1. Persist `resolution.json` from project/plan/typed-argument closure; fingerprint reuse.
2. Check closure identities against `resolution.json`-recorded digests at resolve time.
3. Add `deployment show|doctor` (resolver path coverage, sidecar-less entries, closure-orphan store entries, `--all-slots`).
4. Deployment-supplied HTTP path→identity projection for the ordinary MCP path (hand-authored example routes remain until then).
5. User-level `~/.specify/wasm-pkg.toml` precedence for pre-project hydrate.



## Acceptance criteria

**First delivery (Stages 0–1)**

1. A Wasm installation runs `specify --help` and workflow commands without `run --config`.
2. The ordinary operator path involves no `omnia.toml` — authored or generated; the launcher passes a typed deployment value to `run`.
3. Adding a supported first-party source binding does not require editing runtime configuration.
4. Project and plan selectors remain the authoritative adapter bindings.
5. The engine guest remains the sole owner of command semantics and lifecycle transitions.
6. Omnia's guest-resolver contract carries no Specify vocabulary; Specify supplies the store-backed implementor and path-glob policy.
7. Specify nests `omnia::runtime!` in a host submodule and calls `run` from crate-root launcher `main` with the assembled typed deployment value.
8. A missing, incompatible, wrong-axis, sidecar-less, or digest-mismatched component fails before its operation runs (fail-closed sidecar verify).
9. The native host remains a separate deployment and does not become a fallback for missing Wasm components.
10. The composed Wasm example exercises the same operator invocation as the product binary.
11. A component installed by `ensure_*` is callable in the same process without regenerating a static guest list.
12. The ordinary derived deployment names no per-adapter guest or HTTP route entries.

**Later (Stage 2)**

13. Derived deployment state (`resolution.json`) lives outside the repository and is safe to delete.
14. Exact pins and component digests are visible in `specify deployment show --format json`.
15. On an archive install, first launch requires no network.
16. At least one third-party-style local component exercises the same operator invocation.



## Testing

- Closure computation and deployment assembly are crate-level integration tests over fixture stores and caches; no live registry access in CI. Fingerprinting and `deployment show|doctor` join when Stage 2 lands.
- Missing, digest-mismatched, wrong-axis, and floor-incompatible components are asserted at the CLI boundary through exit codes and the kebab-case error discriminants.
- The Omnia miss-hook, serve-at-resolve, and typed `run(deployment)` are covered by Omnia's own suite plus the operator-invoked composed wasm example (`cargo make wasm-run`), updated to invoke `specify ...` directly; there is no new per-push WASM gate. Path→identity projection joins with Stage 2.
- Launcher selector projection is asserted against the shared `crates/transport` grammar: a selector-bearing verb added to the grammar must surface in closure computation without launcher changes.



## Resolved questions

1. **Command placement** — `deployment show|doctor` are top-level: the noun covers the engine guest, mounts, resolver policy, and the fingerprint ([Diagnostic commands](#diagnostic-commands)).
2. **Workspace closure scope** — only targets reachable from the active plan; `deployment doctor --all-slots` covers whole-workspace audits ([Required component closure](#required-component-closure)).
3. **Out-of-root local components** — no trust flag; the explicit path is the approval act, the origin is recorded, and `doctor` surfaces out-of-root origins as informational findings ([Supply-chain posture](#supply-chain-posture)).
4. **Typed `run(deployment)` timing** — same Omnia release as the miss-hook; the ordinary path never writes `omnia.toml` ([Typed deployment value](#typed-deployment-value), with fallback staging if Omnia review splits the release).

## Open questions

1. Per-guest mount scoping in Omnia: should the writable store mount be grantable to the engine guest alone, closing the guest-writable-store residue in [Supply-chain posture](#supply-chain-posture)? Until it lands, `deployment doctor`'s closure-orphan finding is the compensating control.
