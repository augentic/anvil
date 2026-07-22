# Self-Assembling Wasm Deployment

> Status: Stage 1 landed (launcher and pre-run enumeration); Stages 2–3 remain draft
>
> Owns: the operator-facing `specify` executable, deployment assembly, and Specify's derived enumeration of the adapter store into Omnia's typed deployment value.
>
> Builds on: [Specify on Omnia](architecture.md) and [Native Specify](archive/native-deployment.md) (archived — implemented).
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

Adapters stay separate Wasm components selected by identity. The launcher **derives the guest set before `run`**: it computes the command's component closure from project, plan, and typed arguments, hydrates and verifies each component, and enumerates it as a guest entry in Omnia's typed deployment value. Store probe, filename mapping, and digest verification are launcher policy; Omnia never learns `source:` / `target:` vocabulary, plan bindings, or workflow semantics. Lazy resolve-on-miss is a deferred Omnia capability ([Dynamic Guest Registration §4.5](https://github.com/augentic/omnia/blob/main/rfcs/guest-resolution.md)), pulled in only when a command can dispatch an identity the launcher cannot derive pre-run.

## Motivation

Before Stage 1 landed, the composed Wasm example required three separate concerns in one operator-maintained file:

- the Specify engine component;
- one guest entry per source adapter and target adapter;
- runtime routes and mounts required by those guests.

The example then wrapped Omnia's generic invocation:

```bash
run() {
  cargo run -p specify -- run --config examples/wasm/omnia.toml -- "$@"
}
```

The wrapper had the desired command shape, but it did not own deployment closure. Adding a new adapter still required changing the Omnia configuration before the engine could dispatch to it.

The repository already has most of the required substrate:

- the shipped binary is an Omnia runtime;
- engine-to-adapter calls carry an axis-qualified adapter id;
- adapter selectors preserve bare, package, and local-component inputs;
- `Resolver::ensure_source` and `Resolver::ensure_target` hydrate or mirror components;
- the global adapter store and per-project cache have stable locations;
- the architecture defines the generated deployment manifest as derived state.

Stage 1 closed that gap: the launcher derives and enumerates the guest set itself (see [Pre-run guest enumeration](#pre-run-guest-enumeration)) and hands a typed deployment value to `run`. The lazy layer — resolve a guest on registry miss — remains specified as a **generic** Omnia seam ([Dynamic Guest Registration](https://github.com/augentic/omnia/blob/main/rfcs/guest-resolution.md)), deferred to Stage 3 until a command can dispatch an identity the launcher cannot derive pre-run.

## Goals

1. Make `specify <args...>` the ordinary Wasm operator invocation.
2. Preserve dynamically selected, independently versioned adapter components.
3. Derive the runtime deployment: engine guest, adapter guest entries, mounts, and effects — computed per invocation from project, plan, and typed arguments, never a hand-maintained table.
4. Keep the Omnia runtime generic and free of Specify domain vocabulary.
5. Preserve exact package pins, digest verification, host-CLI floors, and local development components.
6. Give operators a diagnostic view of the effective deployment without making its generated configuration an authored artifact.
7. Maintain the closure-superset invariant: every identity the engine guest can dispatch in a command is derivable by the launcher before `run` — and land Omnia's lazy resolution layer before any command breaks it.

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

1. **Deployment launcher** — resolves the project root, computes the required component closure, hydrates and verifies its components, assembles the typed Omnia deployment value (engine guest, adapter guest entries, mounts), and starts the runtime.
2. **Omnia runtime** — hosts the engine component, satisfies its effect imports, dispatches guest calls by identity, and forwards the command to the engine guest.

The launcher may understand adapter selectors and the derived deployment. It must not parse or implement plan, slice, validation, or lifecycle semantics.

The engine guest remains the only `wasi:cli/run` exporter. Command envelopes and exit codes pass through unchanged.

### Omnia `runtime!` composition

`omnia::runtime!` expands both a CLI `main` and a reusable `run` over the same host-wiring path. Specify nests the macro in a host submodule, owns crate-root `main`, and calls that module's `run` after the launcher steps. Plain Omnia apps keep invoking the macro at crate root and using the generated `main`.

Specify's shipped binary therefore splits:

1. a host submodule that invokes `runtime!` with the existing host set (`WasiHttp` / `WasiOtel` / `WasiModel` over Cursor) — still Specify-vocabulary-free;
2. a crate-root `main` that runs the launcher steps below and calls that module's `run` with the assembled typed deployment value (see [Typed deployment value](#typed-deployment-value)).

Omnia's composition deliverable for this program has landed: `run` takes a **typed deployment value** — a `DeploymentBuilder` carrying a programmatic `Manifest` — so every concern `omnia.toml` expresses (guests with per-guest `link` allow-lists, mounts, deployment-wide links, per-trigger routes, transport) is buildable in memory (see [Typed deployment value](#typed-deployment-value)). The file/flag path stays on the generated `main` for plain Omnia apps and hand-authored examples; Omnia's `examples/guest-link` exercises both paths over the same generated runtime wiring.

### Pre-run guest enumeration

The launcher derives the guest set before `run` — no resolver, no runtime hook. Everything the enumeration needs already exists: host-mediated dynamic linking treats adapter identity as *data* (the `adapter-id` call argument), the guest registry selects an `InstancePre` by identity on every call ([Specify on Omnia §Many guests, selected by identity](architecture.md#many-guests-selected-by-identity)), and the typed `Manifest` carries programmatic guest entries ([Typed deployment value](#typed-deployment-value)). For each closure component the launcher emits a `GuestEntry` — axis-qualified guest id (`source:typescript`, `target:omnia`), store or cache path, verified digest — beside the engine guest and its adapter-interface link allow-list. Guest-id → filename mapping (`<name>@<version>.wasm`) and axis qualification are launcher policy; Omnia sees opaque ids and local paths.

The guest set is *static per invocation, derived per invocation*. `specify` is a one-shot command-mode binary, so "regenerating the guest list" is an in-memory computation the launcher performs on every run from `project.yaml`, `plan.yaml`, and typed argv — never an authored or persisted table.

**The closure-superset invariant.** This model is sufficient exactly because of one invariant, which this RFC now states as a contract: *every identity the engine guest can ensure or dispatch during a command is derivable by the launcher before `run`.* It holds today — closure inputs cover the project target, plan-bound sources, reachable workspace-slot targets, and every selector in the command's typed arguments (the launcher parses argv through the shared `crates/transport` grammar as a superset gate). A mid-run `ensure_*` therefore degrades to a verify: the launcher already hydrated and enumerated the identity, so the same command dispatches it. The invariant is guarded in [Testing](#testing); any RFC that lets a command derive a dispatchable identity from state the launcher does not read — RFC-72/74 mid-run selection is the first candidate — must land Omnia's lazy layer first.

**The deferred lazy layer.** Omnia's [Dynamic Guest Registration](https://github.com/augentic/omnia/blob/main/rfcs/guest-resolution.md) note owns the runtime mechanism for growing the guest set after boot: an explicit `Runtime::register` primitive (§§4.1–4.4) with resolve-on-miss and trigger projection layered on top (§4.5). Specify's eventual consumer is §4.5 specifically — the mid-run installer here is the *engine guest* (the WASI provider's `ensure_*` runs guest-side over the store mount), which cannot call a host registration API, and the launcher is blocked inside `run`; only a dispatch-triggered resolve serves that scenario. When it lands, Specify supplies the resolver implementor (store probe, fail-closed sidecar verify, filename mapping — the same policy the launcher's preflight applies today) registered programmatically on the `DeploymentBuilder`. None of it is on the first-delivery critical path.

After Stage 1 (today):

- the derived deployment value names the engine guest, adapter guest entries, effects, and mounts;
- engine→adapter calls resolve the same axis-qualified guest ids the launcher enumerated;
- a component installed by `ensure_*` was already hydrated and enumerated pre-run (the superset invariant), so the same command dispatches it;
- third-party adapters follow the same path as first-party adapters: an explicit selector puts them in the closure.

### Derived deployment state

The deployment itself is a **typed value** handed to `run` in memory (see [Typed deployment value](#typed-deployment-value)); the ordinary path writes no `omnia.toml`. Stage 1 does not persist diagnostics. **Stage 2** writes deployment diagnostics beneath the out-of-tree per-project cache:

```text
<project-cache>/deployment/
└── resolution.json     # Specify diagnostics (Stage 2)
```

When Stage 2 lands, `resolution.json` records enough information for diagnostics:

- Specify binary and engine component versions;
- each adapter's axis, selector, resolved identity, component path, origin, and digest;
- store roots, mounts, and engine link allow-list;
- the project root and cache/store mount sources;
- the input fingerprint used to decide whether the persisted resolution can be reused.

For the `payments` project from the abstract — target `specify:omnia@0.5.0`, one plan-bound source `legacy=typescript@0.5.0:./legacy`, binary version `0.28.0` — the derived deployment value, rendered as TOML for illustration, is:

```toml
# The typed deployment value specify 0.28.0 assembles in memory,
# rendered as TOML for illustration. Every section is real Omnia
# manifest syntax; the guest list is derived per invocation from
# the closure, never authored or persisted.

[[guest]]
id = "specify"
source.path = "/home/op/.specify/store/engine@0.28.0.wasm"
link = ["specify:adapter/source@0.1.0", "specify:adapter/target@0.1.0"]

[[guest]]
id = "target:omnia"
source.path = "/home/op/.specify/store/omnia@0.5.0.wasm"

[[guest]]
id = "source:typescript"
source.path = "/home/op/.specify/store/typescript@0.5.0.wasm"

[[mount]]
name = "."
path = "/work/payments"
writable = true

[[mount]]
name = "/specify-cache"
path = "/home/op/.specify/cache/6b1f…"
writable = true

[[mount]]
name = "/specify-store"
path = "/home/op/.specify/store"
writable = true
```

Digest verification is a **fail-closed launcher preflight**: before assembling a `GuestEntry`, the launcher verifies each closure component against its store sidecar; an entry without a sidecar, or with a mismatched digest, is a preflight error (`adapter-sidecar-missing` / `adapter-digest-mismatch`) — nothing started. This is deliberately stricter than the engine's `verify_store_entry`, whose missing-sidecar fail-open remains for non-executable reads of legacy entries — anything reaching the executable-loading path meets the stricter bar, and `ensure_*` has always written sidecars. `specify deployment doctor` reports sidecar-less store entries so operators can re-hydrate them before they become preflight failures.

#### MCP route projection

MCP identity is a pure function of the request path.

| Input | Output |
| ----- | ------ |
| Request path `/mcp/<name>` (ordinary case) | Guest id `source:<name>` or `target:<name>`; only guests that export `wasi:http/incoming-handler` succeed |
| Request path `/mcp/<axis>/<name>` | Dual-axis fallback when the same name exists on both axes (unpublished fixtures such as `mock`) |

The formula is `/mcp/<name>`: the store already keys components by name with no axis segment, and first-party adapter names are unique across axes, so the axis need not appear in the URL. Guest ids stay axis-qualified in the launcher's enumeration; only the HTTP prefix drops the axis. On the ordinary path the launcher applies the formula itself, deriving one static `[[route.http]]` row (`Manifest::route_http`) per closure adapter — routes are derived state exactly like guest entries. Omnia's deployment-supplied path→identity projection hook ([Dynamic Guest Registration §4.5](https://github.com/augentic/omnia/blob/main/rfcs/guest-resolution.md)) is needed only when the lazy layer lands and a request may name an identity outside the enumerated closure. The engine guest is reached through CLI and host-mediated linking.

Doctor treats a prefix collision under the ordinary rule as a deployment error.

The composed example carries no hand-authored `omnia.toml`: the launcher derives its deployment per invocation. Derived MCP route rows land in Stage 2; until then the example's adapters are reached over the CLI seam only.

Stage 2's matching `resolution.json` carries the provenance the deployment value flattens away:

```json
{
  "specify": "0.28.0",
  "engine": {
    "package": "specify:engine@0.28.0",
    "component": "/home/op/.specify/store/engine@0.28.0.wasm",
    "origin": "registry",
    "digest": "sha256:9c41…"
  },
  "adapters": [
    {
      "axis": "source",
      "selector": "specify:typescript@0.5.0",
      "resolved": "typescript@0.5.0",
      "component": "/home/op/.specify/store/typescript@0.5.0.wasm",
      "origin": "store",
      "digest": "sha256:5b0e…"
    },
    {
      "axis": "target",
      "selector": "specify:omnia@0.5.0",
      "resolved": "omnia@0.5.0",
      "component": "/home/op/.specify/store/omnia@0.5.0.wasm",
      "origin": "store",
      "digest": "sha256:1d9c…"
    }
  ],
  "store": {
    "roots": [
      "/home/op/.specify/store",
      "/home/op/.specify/cache/6b1f…/components"
    ],
    "verify": "digest"
  },
  "project": {
    "root": "/work/payments",
    "cache": "/home/op/.specify/cache/6b1f…",
    "store": "/home/op/.specify/store"
  },
  "fingerprint": "sha256:d81f…"
}
```

`resolution.json` is safe to delete. The authored selectors remain in `project.yaml`, `plan.yaml`, and command inputs. Beyond diagnostics, Stage 2 uses the recorded digests as the launcher's host-held expectation for closure identities at preflight (see [Supply-chain posture](#supply-chain-posture)). Stage 1 preflight verifies against store sidecars only.

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

The closure includes only components that the command can reach. Because the closure is a superset of everything the guest can ensure or dispatch ([the closure-superset invariant](#pre-run-guest-enumeration)), a component the guest installs mid-command was already hydrated and enumerated by the launcher, and the same command dispatches it.

### First-launch engine installation

The engine guest reaches the store the same way every adapter does: on first launch the launcher installs `specify:engine@<binary-version>` into the global store — from the release archive beside the binary when present, else by registry hydration — and verifies the recorded digest. The architecture fixes the binary version as the engine version ([Specify on Omnia §CLI bootstrapping](architecture.md#cli-bootstrapping)), and one hydration path keeps the operator-local CLI and a hosted deployment identical apart from the bound store backend.

The release archive is the **primary** distribution: it ships the engine and first-party adapter components *beside* the binary, and first launch installs them into the store through the same verify-on-write path, recorded as `origin: release-archive`. Registry hydration covers everything else — a bare binary install, a pin the archive does not carry, a third-party component. Pre-project hydration resolves its registry from the user-level `~/.specify/wasm-pkg.toml` when present, else the compiled default; a project's `.specify/wasm-pkg.toml` overrides both once it exists (precedence: project → user → compiled default — mirrored/enterprise installs set the user file). The store entry is canonical in every case.

Help and version displays are answered host-side by the shared clap grammar — byte-identical to what the guest would print, so no deployment is assembled just to print usage. `adapter add` likewise completes host-side (the operator's component path may live outside any guest mount). Every other invocation forwards to the engine guest, which stays the sole owner of command semantics.

After first launch and one `init` plus `plan author` on the `payments` project, the global store carries the same entry-plus-sidecar pairs the existing hydration path writes today:

```text
~/.specify/store/
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

    // 3. Assemble: the typed deployment value — a programmatic
    //    Manifest (engine GuestEntry + link allow-list, one derived
    //    GuestEntry per closure adapter, Mounts) on a DeploymentBuilder
    //    carrying argv — + resolution.json beneath
    //    <project-cache>/deployment/.
    let manifest = deployment::assemble(&project, &closure);

    // 4. Run: in-process Omnia command mode (the nested runtime! stamps
    //    the mode); argv and the engine guest's exit code pass through
    //    byte-for-byte.
    match host::run(DeploymentBuilder::new().manifest(manifest).args(argv)) {
        Ok(status) => std::process::ExitCode::from(status.code_u8()),
        Err(err) => report_runtime(err),
    }
}
```

Steps 1–3 project selectors from project/plan/typed args; step 4 receives only the typed deployment. The layer boundary from the decision above is the function boundary here.

Selector projection reuses the typed clap grammar in `crates/transport` (wasm-clean, already consumed by the native host): the launcher parses argv into the same `Args` the engine guest will parse, folds selectors out of the typed values, and discards the rest. A command whose `Args` carry no selectors contributes nothing to the closure, and a new selector-bearing argument is picked up by the closure projection from the shared grammar in the same change. The launcher's parse is a superset gate — argv that fails the shared grammar fails closed before `run`, nothing started.

The runtime configuration continues to carry only deployment concerns:

- the engine guest and its WIT interface link allow-list;
- the derived adapter guest entries (and, with Stage 2, derived MCP route rows);
- project, cache, and store mounts;
- host backend configuration.

Workflow artifacts and lifecycle state remain invisible to Omnia.

### Diagnostic commands

Add a deployment-oriented read surface:

```bash
specify deployment show [--format json]
specify deployment doctor [--format json] [--all-slots]
```

The commands are top-level: the noun covers the engine guest, adapter guest entries, mounts, store roots, and the fingerprint. `show` projects the effective closure and generated configuration. `doctor` verifies:

- engine and adapter components exist;
- recorded digests match;
- every recorded adapter exports the expected axis world;
- metadata floors are compatible;
- project/cache/store mounts resolve;
- store roots cover every recorded component path;
- the persisted resolution fingerprint matches its inputs.

Beyond the closure, `doctor` audits the store's reachable surface: a store entry beneath the store roots but missing its digest sidecar is reported (it would be a fail-closed preflight error the moment a closure names it), and an entry beneath the roots but absent from every recorded closure is flagged as an orphan — the residue the trust model in [Supply-chain posture](#supply-chain-posture) asks operators to watch until per-guest mount scoping lands. `--all-slots` widens the audit from the active plan's closure to every materialized workspace slot target; the ordinary closure stays minimal (only targets reachable from the active plan — see [Required component closure](#required-component-closure)).

`show --format json` projects `resolution.json`. `doctor` re-verifies and reports through the ordinary finding currency, for example after a store entry has been tampered with:

```console
$ specify deployment doctor
engine  specify:engine@0.28.0   ok
target  omnia@0.5.0           ok
source  typescript@0.5.0      adapter-digest-mismatch: recorded sha256:5b0e… but recomputed sha256:99d1…
mounts  project cache store   ok
store roots                   ok
fingerprint                   ok
$ echo $?
2
```

### Typed deployment value

**Landed in Omnia.** `run` accepts a typed deployment value: a `DeploymentBuilder` carrying a programmatic `Manifest`. Everything `omnia.toml` expresses is buildable in memory — `[[guest]]` entries with per-guest `link` allow-lists (`GuestEntry::new(id, path).link(interface)`), `[[mount]]` entries (`Manifest::mounts`), deployment-wide `link` (`Manifest::links`), and per-trigger routes (`Manifest::route_http` / `route_messaging` / `route_websocket`). Omnia's `examples/guest-link` runs the same two-guest deployment from either the TOML file or the equivalent programmatic value:

```rust
fn main() -> anyhow::Result<()> {
    let artifacts =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/wasm32-wasip2/debug/examples");
    let manifest = Manifest::new()
        .guest(GuestEntry::new("responder", artifacts.join("guest_link_responder_wasm.wasm")))
        .guest(
            GuestEntry::new("router", artifacts.join("guest_link_router_wasm.wasm"))
                .link("omnia:link/echo"),
        );

    host::run(DeploymentBuilder::new().manifest(manifest))?;
    Ok(())
}
```

Specify's launcher assembles the same shape from the closure — the engine `GuestEntry` with the adapter-interface link allow-list, one derived `GuestEntry` per closure adapter, the project/cache/store `Mount`s — and forwards argv through `DeploymentBuilder::args`; the nested `runtime!` expansion stamps command mode onto the builder inside the generated `run`. The ordinary Specify path writes no `omnia.toml`. Stage 2 persists `resolution.json` under the per-project cache and uses its fingerprint to guard reuse of the recorded *resolution*.

The generated Omnia `main` keeps loading a file/flag deployment for plain Omnia apps and examples (when no manifest is supplied, `DeploymentBuilder::build` falls back to the `OMNIA_CONFIG` path). The file format remains available for hand-authored Omnia deployments; Specify's composed wasm example invokes bare `specify …` with a launcher-derived deployment.

This also serves the dual deployment posture directly: steps 1–4 of the launcher are deployment-neutral, and a hosted runner composing the same closure can start the runtime in process from the typed value alone.

Every closure component appears as an explicit derived `GuestEntry` — that is the plan of record, not a stopgap. The typed value carries no resolver seam and needs none for first delivery; a programmatic resolver registration joins `DeploymentBuilder` only with the lazy layer ([Dynamic Guest Registration §4.5](https://github.com/augentic/omnia/blob/main/rfcs/guest-resolution.md)), and resolution policy (store probe, digest verification) stays inside Specify's implementor when it does.

### Per-invocation compile cost

Pre-run enumeration makes every closure component a static guest that Omnia compiles at load — on **every** CLI invocation, because command mode is one-shot. Cranelift-compiling the engine plus two or three adapters per `specify` command is a real latency tax.

The mitigation is the install-time compile posture from [Dynamic Guest Registration §4.4](https://github.com/augentic/omnia/blob/main/rfcs/guest-resolution.md): hydration writes a settings-matched pre-compiled artifact (`omnia compile` output) beside each store entry, and the launcher's `GuestEntry` points at the pre-compiled artifact when present — Omnia's loader already prefers deserialization over JIT. The artifact is derived state keyed to the binary's compile-affecting settings; a settings or wasmtime bump invalidates it and the launcher recompiles on next hydrate. Whether this ships in first delivery or waits for the latency to hurt is an [open question](#open-questions).

## Supply-chain posture

Deployment assembly must not turn model output into executable trust.

- Only exact package identities or explicitly supplied local components may be loaded.
- Package components must pass digest verification before compilation.
- Registry namespace and publisher policy are evaluated before hydration.
- Auto-selection may recommend an adapter, but installation requires the trust policy in [Adapter Descriptors and Registry Trust](rfc-71-discovery.md) and the approval surfaces in [Migration Intake and Source Selection](rfc-72-migration.md) and [Migration Programs](rfc-74-program.md).
- The generated deployment records origin and digest for every component.
- A component cannot gain a filesystem or network capability merely by declaring one; Omnia's configured host links remain authoritative.
- Store roots bound where the launcher may read component bytes; they do not grant network fetch or widen trust policy.

The digest sidecar is written through the same writable store mount every guest in the deployment shares, so sidecar verification proves **integrity, not trust**: a guest could install an entry and a self-consistent sidecar. Pre-run enumeration bounds this well: nothing outside the launcher-enumerated closure is dispatchable in the current process, and Stage 1 preflight verifies every closure entry against its store sidecar. Stage 2 strengthens that with digests recorded in `resolution.json` — a host-held expectation no guest can rewrite. A store entry a guest writes mid-run becomes loadable only in a *later* invocation, whose preflight verifies it in turn (and, for hydrated identities, against registry-recorded digests). Per-guest mount scoping in Omnia (the store writable only to the engine guest) remains the hardening path and is tracked as an [open question](#open-questions); until it lands, Stage 2's `deployment doctor` flags store entries beneath the store roots but absent from every recorded closure.

Local component selectors that resolve outside the project root require no additional trust flag: the operator typing an explicit component path is the approval act, consistent with how local components mirror today. Stage 2 records the canonical origin path in `resolution.json`, and `doctor` surfaces out-of-root origins as informational findings. Revisit only if the trust policy in [Adapter Descriptors and Registry Trust](rfc-71-discovery.md) gives a flag something real to gate on.



## First delivery

The first cut is something an in-house team can run daily: `specify …` with no authored Omnia config, first-party adapters hydrating into the store, and clear resolve failures. It does not need a polished diagnostics product or third-party trust theatre — Stages 0–1 are landed; Stages 2–3 remain.

**In first delivery (landed)**

- Stages 0–1 below (typed `run` — already landed in Omnia, nested launcher, closure hydrate + pre-run guest enumeration, fail-closed preflight verify). No Omnia implementation work is on the critical path.
- Release archive *or* registry hydrate for the engine and first-party pins the team actually uses.
- Kebab-case preflight/dispatch errors that name the missing or mismatched identity.

**Deferred until the team needs them**

| Capability | Pull in when |
| ---------- | ------------ |
| `resolution.json`, fingerprint reuse | Re-resolve cost or “what’s loaded?” support load |
| `deployment show\|doctor`, orphan audits | Store/digest failures are hard to diagnose from exit text alone |
| Pre-compiled store artifacts ([Per-invocation compile cost](#per-invocation-compile-cost)) | Command startup latency hurts |
| Derived MCP route rows on the ordinary path | In-house workflows need adapter MCP references through the product binary |
| User-level `~/.specify/wasm-pkg.toml` | Mirrored registry installs outside project config |
| Omnia lazy layer ([Dynamic Guest Registration §4.5](https://github.com/augentic/omnia/blob/main/rfcs/guest-resolution.md)) + Specify's resolver implementor | A command can dispatch an identity the launcher cannot derive pre-run (RFC-72/74 mid-run selection), or a long-lived MCP surface outlives per-invocation closures |

Program sequencing: [RFC-74 §First delivery](rfc-74-program.md#first-delivery).

## Implementation stages

Stages below are the planned order. First delivery is Specify-only work over Omnia capabilities that have already landed; Omnia's lazy layer is a contingent later stage.

### Stage 0 — Omnia design note

Land [Dynamic Guest Registration](https://github.com/augentic/omnia/blob/main/rfcs/guest-resolution.md) through Omnia review, so the generic-runtime constraint is defended by Omnia's own review. The note owns the runtime mechanism (registration primitive, late insertion, serve-at-register, deferred resolve-on-miss and trigger projection); this RFC owns only Specify's closure derivation, enumeration, and — when the lazy layer lands — the resolver implementor. Nothing in the note's implementation blocks Stage 1.

### Stage 1 — Launcher and pre-run enumeration (first delivery)

**Landed.** The `crates/launcher` crate owns the pipeline (anchor → closure → hydrate → preflight → assemble); `transport::command::selectors` owns argv selector projection over the shared grammar.

Omnia:

1. ~~Change `run` to take a typed deployment value~~ **Landed**: `run` takes a `DeploymentBuilder` carrying a programmatic `Manifest`; the file/flag path remains on the generated `main` for plain Omnia apps ([Typed deployment value](#typed-deployment-value)).

Specify:

2. ~~Split the native binary~~ **Landed**: `runtime!` nests in `mod host` in `src/omnia.rs`; the crate-root launcher `main` owns closure + assemble + `host::run(…)`, projecting selectors through the shared `crates/transport` grammar.
3. ~~Compute the closure and hydrate missing components before `run`~~ **Landed** (registry hydration; the release-archive probe stays deferred): fail-closed preflight sidecar verify — `adapter-sidecar-missing` / `adapter-digest-mismatch` (closure digests recorded in `resolution.json` wait for Stage 2).
4. ~~Assemble the typed deployment value in memory~~ **Landed**: engine guest with the adapter link allow-list, one derived guest per closure adapter, the three well-known mounts; no authored table, no `omnia.toml`.
5. ~~Forward all workflow arguments and exits unchanged~~ **Landed**: only help/version displays and the deterministic `adapter add` seed complete host-side; everything else forwards byte-for-byte.
6. ~~Change the composed example to invoke `specify ...` directly~~ **Landed**: `cargo make wasm-run` seeds the sandboxed store and invokes the shipped binary bare.
7. ~~Assert the closure-superset invariant end to end~~ **Landed**: the example's `plan author --source …` dispatches the source the launcher hydrated and enumerated pre-run; the grammar-coverage guard in `crates/transport/tests/selectors.rs` keeps new selector-bearing verbs classified.

### Stage 2 — Diagnostics and hardened verify

1. Persist `resolution.json` from project/plan/typed-argument closure; fingerprint reuse.
2. Check closure identities against `resolution.json`-recorded digests at preflight.
3. Add `deployment show|doctor` (store-root coverage, sidecar-less entries, closure-orphan store entries, `--all-slots`).
4. Derived static MCP route rows (`/mcp/<name>`) for closure adapters on the ordinary path (hand-authored example routes remain until then).
5. User-level `~/.specify/wasm-pkg.toml` precedence for pre-project hydrate.
6. Pre-compiled store artifacts if command latency warrants ([Per-invocation compile cost](#per-invocation-compile-cost)).

### Stage 3 — Omnia lazy layer (contingent)

Triggered by the first command that can dispatch an identity the launcher cannot derive pre-run (RFC-72/74 mid-run selection), or by a long-lived MCP surface:

1. Omnia implements [Dynamic Guest Registration](https://github.com/augentic/omnia/blob/main/rfcs/guest-resolution.md) — the registration primitive (§§4.1–4.4) and the resolve-on-miss + trigger-projection layer (§4.5).
2. Specify implements the store-backed resolver (store probe, fail-closed sidecar verify, filename mapping) registered programmatically on the `DeploymentBuilder`.



## Acceptance criteria

**First delivery (Stages 0–1) — met**

1. ~~A Wasm installation runs `specify --help` and workflow commands without `run --config`.~~
2. ~~The ordinary operator path involves no `omnia.toml` — authored or generated; the launcher passes a typed deployment value to `run`.~~
3. ~~Adding a supported first-party source binding does not require editing runtime configuration; the next invocation's closure picks it up.~~
4. ~~Project and plan selectors remain the authoritative adapter bindings.~~
5. ~~The engine guest remains the sole owner of command semantics and lifecycle transitions.~~
6. ~~The derived enumeration carries no Specify vocabulary into Omnia: opaque guest ids, local component paths, and mounts only.~~
7. ~~Specify nests `omnia::runtime!` in a host submodule and calls `run` from crate-root launcher `main` with the assembled typed deployment value.~~
8. ~~A missing, incompatible, wrong-axis, sidecar-less, or digest-mismatched closure component fails at launcher preflight — before the runtime starts.~~
9. ~~The native host remains a separate deployment and does not become a fallback for missing Wasm components.~~
10. ~~The composed Wasm example exercises the same operator invocation as the product binary.~~
11. ~~The closure-superset invariant holds: every identity the engine guest ensures or dispatches during a command was hydrated and enumerated by the launcher pre-run, so an in-command `ensure_*` install is dispatchable by the same command.~~
12. ~~The guest list is derived per invocation from closure inputs; no authored or persisted guest table exists.~~

**Later (Stage 2)**

13. Derived deployment state (`resolution.json`) lives outside the repository and is safe to delete.
14. Exact pins and component digests are visible in `specify deployment show --format json`.
15. On an archive install, first launch requires no network.
16. At least one third-party-style local component exercises the same operator invocation.



## Testing

- Closure computation and deployment assembly are crate-level integration tests over fixture stores and caches; no live registry access in CI. Fingerprinting and `deployment show|doctor` join when Stage 2 lands.
- Missing, digest-mismatched, wrong-axis, and floor-incompatible components are asserted at the CLI boundary through exit codes and the kebab-case error discriminants (preflight failures — nothing started).
- The typed `run(deployment)` path is already covered on the Omnia side by `examples/guest-link` (the `guest-link-dynamic` host and the `guest_link` integration test build the registry from a programmatic `Manifest`). Registration and resolve-on-miss coverage is Omnia's when the lazy layer lands ([Dynamic Guest Registration §7](https://github.com/augentic/omnia/blob/main/rfcs/guest-resolution.md)); on the Specify side the seam is exercised by the operator-invoked composed wasm example (`cargo make wasm-run`), updated to invoke `specify ...` directly — there is no new per-push WASM gate.
- The closure-superset invariant is guarded twice: launcher selector projection is asserted against the shared `crates/transport` grammar (a selector-bearing verb added to the grammar must surface in closure computation without launcher changes), and the composed wasm example exercises ensure-then-dispatch within one command (`plan author --source …` surveying the source it just ensured).



## Resolved questions

1. **Command placement** — `deployment show|doctor` are top-level: the noun covers the engine guest, adapter guest entries, mounts, store roots, and the fingerprint ([Diagnostic commands](#diagnostic-commands)).
2. **Workspace closure scope** — only targets reachable from the active plan; `deployment doctor --all-slots` covers whole-workspace audits ([Required component closure](#required-component-closure)).
3. **Out-of-root local components** — no trust flag; the explicit path is the approval act, the origin is recorded, and `doctor` surfaces out-of-root origins as informational findings ([Supply-chain posture](#supply-chain-posture)).
4. **Typed `run(deployment)` timing** — landed in Omnia first: `run` takes a `DeploymentBuilder` carrying a programmatic `Manifest`, so the ordinary path never writes `omnia.toml` ([Typed deployment value](#typed-deployment-value)). No fallback staging is needed.
5. **Miss-hook vs pre-run enumeration** — resolved in favor of pre-run enumeration for first delivery, on the strength of the closure-superset invariant ([Pre-run guest enumeration](#pre-run-guest-enumeration)). Omnia's [Dynamic Guest Registration](https://github.com/augentic/omnia/blob/main/rfcs/guest-resolution.md) keeps registration as the runtime primitive and defers resolve-on-miss to its §4.5; Specify pulls that layer in only when the invariant breaks (Stage 3).

## Open questions

1. Per-guest mount scoping in Omnia: should the writable store mount be grantable to the engine guest alone, closing the guest-writable-store residue in [Supply-chain posture](#supply-chain-posture)? Until it lands, `deployment doctor`'s closure-orphan finding is the compensating control.
2. Pre-compiled store artifacts ([Per-invocation compile cost](#per-invocation-compile-cost)): Stage 2 item 6 — when does command latency warrant them? Measurement on the composed example with the real engine + two adapters should decide.
