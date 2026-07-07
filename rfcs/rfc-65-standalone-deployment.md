# RFC-65: The Standalone Deployment — Specify as a Guest of a Generic Omnia Host

> Status: Proposed · Depends: RFC-61 (the in-place migration — landed; this RFC amends its decisions D-bin (triage main) and D-dist (embedded workflow guest)), RFC-64 (one-component adapter artifact, wasm-pkg transport — landed) · Related: [RFC-66](rfc-66-publishing-and-distribution.md) (the publish/acquire plumbing for the core guest, the adapters, the WIT contract, and the binary), and an Omnia-side generic guest-embed option on `runtime!` (an optional acceleration move 4 adopts when available — never a dependency of the cut) · Absorbs: RFC-63 (adapter hydration and the central store — now §"The provisioning half") · Owns: how a project's declared component identities become a fully provisioned, runnable deployment — locally and in the cloud — and the operational surface after the embedded host retires

## Abstract

Specify's operator surface becomes one shipped `specify` binary with **two strictly separated layers**: a thin native **provisioning surface** carrying the closed verb set the capability fence makes irreducibly native (`init`, `adapters sync`, `upgrade`, `plugins`), and a **generic, Specify-agnostic Omnia host layer** driven by the generated deployment manifest, through which every project-scoped verb runs in the core guest.

The provisioning surface owns the consumer path this repo lacks today (the only deployment manifest is the in-tree `omnia.toml` pointing at a sibling `specify-adapters` checkout — a developer posture, not a consumer one): it reads the project's declared component identities, downloads each missing package into the **central store at `$HOME/.specify/adapters/`**, and generates the deployment manifest from the resolved store entries. Hydration is one idempotent, concurrency-safe, non-interactive kernel with two triggers — init and an explicit sync verb — so the same mechanism provisions a laptop, a CI runner, and a cloud agent with only an environment variable between them. The guest never hydrates: a store miss at guest runtime is a typed error naming the identity and the sync command, never a network fetch.

The core guest is **versioned by the binary and resolved embedded-first**: the baseline is a published `specify:core@<binary version>` hydrated like an adapter (publish job in RFC-66), and when the build opts into Omnia's generic `runtime!` embed option the same component is instead compiled in, precompiled for instant instantiation — either way the binary version *is* the core version, one knob. Adapters stay published, pinned, and hydrated. Anything that is not a provisioning verb forwards blindly — unparsed — to the guest's `wasi:cli/run`. The developer remainder (adapter build, publish) needs no product surface at all: RFC-66's tag-driven workflows own it. No workflow-verb triage, no hand-rolled embeds, no independently versioned core: the operational shape matches the Omnia cursor example (`backends/examples/cursor`) — a macro-generated host, a manifest, and pulled components.

## Operational model

- **Host layer** — a command-mode runtime from `omnia::runtime!({ mode: command, hosts: { WasiHttp: HttpDefault, WasiOtel: OtelDefault, WasiModel: Cursor } })`. The macro forwards argv to the guest's `wasi:cli/run`; backend binding stays compile-time (Omnia ships a blessed cursor-bound host layer; backend selection as deployment configuration is a later Omnia-side option). This RFC's one (optional) Omnia-side ask is a **generic, domain-free embed option** on the macro — a build-time parameter naming a component to compile into the binary (optionally precompiled against the macro's own wasmtime version) and expose as a loadable guest; builds without it change nothing below. The host layer carries no Specify vocabulary; any embedded bytes are supplied by the consuming build.
- **Provisioning surface** — the native front of the shipped binary, with a closed verb set: `init`, `adapters sync`, `upgrade`, `plugins`. Each is native because it *cannot* run in-guest by construction, not because it is convenient: `init` and `adapters sync` are the hydration triggers (§"The hydration kernel") — network pulls and global-store writes the runtime world deliberately lacks; `upgrade` replaces the binary through its install channel — a guest cannot replace its own host; `plugins` probes and repairs the dev machine's Cursor plugin cache. The provisioning surface carries no workflow verbs and never runs during the workflow loop — it precedes it.
- **Manifest** — the generated deployment manifest (§"Deployment manifest generation") is the sole deployment authority the host layer reads: one guest entry per pulled component — every adapter, plus the core guest when the binary does not embed it — the project mount, MCP routes, the link allow-list. An embedded core needs no manifest entry; the host layer wires it itself. Discovery and wiring are resolved at manifest generation time, not per invocation.
- **Components** — the core guest (the workflow grammar's home; versioned by the binary in both modes — registry identity `specify:core@<binary version>` when pulled, no registry presence needed when embedded) plus one guest per bound adapter, published, pinned, and hydrated identically: the generated manifest references each pulled component by its single-file store entry path (`<store-root>/<name>@<version>.wasm`, RFC-64). The core guest defines no WIT package of its own — its world is an anonymous `export wasi:cli/run` plus the adapter-contract imports; `specify:core` is registry identity only.
- **Naming** — namespaces follow product, not org. `omnia:` remains the runtime's interface namespace (`omnia:model/completion`); everything Specify-owned lives in the `specify:` namespace: `specify:adapter` is the WIT adapter contract (imports read `specify:adapter/source`, `specify:adapter/target` — renamed from `augentic:specify`), `specify:core` is the core guest's registry identity (the pulled path), and `specify:<name>` is each adapter package (`specify:omnia@<semver>`, renamed from `augentic:omnia@<semver>` — the namespace disambiguates the omnia adapter from the Omnia runtime's own packages). Both namespaces route to the first-party registry in wasm-pkg config; `augentic:` is reserved for future org-wide contracts and is otherwise retired.
- **Developer tooling** — adapter build and publish stay off the product surface entirely: `cargo make` tasks called by RFC-66's tag-driven release workflows. No dev-tool binary ships with this RFC.

Invocation form: `specify <verb …>`. A provisioning verb runs natively; any other argv forwards unparsed as `<host layer> run --config <generated manifest> -- <verb …>`, with the binary resolving the manifest path itself (it generated it — operators never type `--config`). The bare host form remains available for debugging and Omnia-native deployments.

### In-guest argument parsing

The core guest parses forwarded argv exactly as a Rust-native binary would — the Omnia CLI example (`omnia/examples/cli`) is the template. The host layer forwards argv to the guest's `wasi:cli/run` (`argv[0]` supplied by the host), and argv plus stdout/stderr arrive through the p2 `std` bridge Omnia links alongside p3 — so the existing clap grammar in `specify-dispatch` runs unmodified in-guest: `--help`, `--version`, and usage errors need no hand-rolling. One seam nuance the implementer must carry over: use `try_parse()` rather than `parse()`, and forward `clap::Error::exit_code()` through the p3 `wasi:cli/exit` (`wasip3::cli::exit::exit_with_code`). `parse()`'s internal `std::process::exit` lands on the *p2* exit, which carries only success/failure and would collapse clap's usage-error code `2` to `1`. Either way the host observes the exit as wasmtime's `I32Exit` and its generated `main` exits with it — the exit-code contract passes through verbatim. If the core-guest component's size matters, the example's trims apply: clap with `default-features = false` (keeping `derive`, `error-context`, `help`, `std`, `usage`) drops the color and suggestion machinery.

## The provisioning half: hydration and the central store

### What already exists

Most of the machinery landed with RFC-48 and RFC-64; this RFC composes it rather than re-inventing it:

- **Identity.** An adapter is `name@<semver>` (`AdapterRef`), carried as a `specify:<name>@<semver>` package reference (the naming cut above); `project.yaml.adapter` records the target's pinned identity, and `plan.yaml.sources.<key>` carries an optional per-binding version pin.
- **Packaging and transport.** A published artifact is exactly one wasm component (RFC-64: no manifest, no tree pack), pulled through the wasm-pkg client (`crates/registry/src/package.rs` — the same funnel the `tools[]` resolver uses). Registry backing — the `augentic.io` well-known file over GHCR, anonymous consume, `GITHUB_TOKEN` publish — is RFC-66's.
- **The store.** `store::install_tofu` pulls a component once (trust-on-first-use), stages it under a sibling install lock, publishes it read-only by atomic rename at the single-file entry `<store-root>/<name>@<version>.wasm`, and records the component-byte digest sidecar (`<name>@<version>.meta`) that `verify_store_entry` re-checks on every resolve (`adapter-digest-mismatch` on drift).
- **Resolution order.** A pinned identity resolves the store entry; a bare name resolves the project component cache, then the sibling/in-repo development release build (`target/wasm32-wasip2/release/specify_<name>.wasm`). A miss on every probe is `adapter-not-found`. Resolution is project-local plus the store — no environment-variable fallback to a framework checkout.

The store root today resolves `$SPECIFY_ADAPTER_CACHE` → `$XDG_CACHE_HOME/specify/adapters` → `~/.cache/specify/adapters` (`crates/schema/src/cache.rs`). Install happens only when the operator hands `specify init` a `specify:<name>@<semver>` package ref; nothing reads `project.yaml` and hydrates, and nothing generates the deployment manifest the runtime needs.

### The store root moves to `$HOME/.specify/adapters`

The store is an **install store, not a disposable cache**: entries are immutable, digest-verified, and load-bearing at runtime (the deployment manifest references components inside it; evicting one bricks every project pinned to it). XDG cache semantics invite eviction, and cloud runners want one obvious directory to persist or mount. The root therefore becomes:

1. `$SPECIFY_ADAPTER_STORE` (renamed from today's `$SPECIFY_ADAPTER_CACHE` to match the store-not-cache framing — the relocation lever for sandboxes and tests),
2. else `$HOME/.specify/adapters`.

Layout is unchanged from RFC-64 as landed: single-file `<root>/<name>@<version>.wasm` entries with sibling `<name>@<version>.meta` digest sidecars and dot-prefixed install locks. `$HOME/.specify/` becomes Specify's per-user home (the store today; a natural later home for auth and channel config), distinct from the per-project `.specify/` system-of-record and the per-project derived cache under `$XDG_CACHE_HOME/specify/projects/`, which stay where they are.

### The hydration kernel

One kernel, `hydrate(refs) -> resolved set`: for each pinned `name@<semver>`, probe the store; on a miss, pull the component through the wasm-pkg transport and `install_tofu` it; verify the digest; return the resolved entry paths. Properties the kernel guarantees, all inherited from the RFC-48/64 substrate:

- **Idempotent.** A warm store makes hydration a no-op probe per identity — cheap enough to run on every provisioning invocation.
- **Concurrency-safe.** Parallel hydrators of one identity serialize behind the blocking install lock; losers find the entry materialized and move on.
- **Non-interactive.** No prompts, no credential dialogs: first-party packages pull anonymously (RFC-66), private mirrors authenticate from the environment, and any failure is a typed error naming the identity and the probe that failed.
- **Exact pins only.** Hydration never resolves a version range or a bare name over the network — an unpinned name keeps today's project-local resolution, and network version-resolution stays deferred (RM-21). Determinism in the cloud comes from pins, not from "latest".

Two triggers share the kernel, both native verbs on the provisioning surface (the kernel itself is surface-agnostic). Guided init elicitation (§"Operator onboarding") lives strictly above the kernel, gathering arguments — nothing at or below the kernel blocks on a TTY, so the non-interactive property holds on every machine shape:

- `specify init` hydrates the target identity recorded on `project.yaml.adapter`, plus every identity in the new optional `project.yaml.adapters:` prefetch list (both axes, pinned), plus `specify:core@<the binary's own version>` when the binary does not embed its core (move 4) — so a project that knows its source set up front provisions everything in one command. Init then generates the deployment manifest and hands off to the guest scaffold leg (§"Operator onboarding"). `specify init --upgrade` re-runs hydration against the (possibly re-pinned) declared set.
- `specify adapters sync` is the explicit verb: read `project.yaml` (and `plan.yaml` when present), hydrate every declared identity, regenerate the manifest, print the resolved set with per-identity store paths and digests. It is the one-line cloud bootstrap and the operator's cache-priming and diagnosis surface. `--frozen` turns any would-be fetch into a typed failure (`adapter-not-installed`) for offline and reproducibility-strict CI.

**The guest never hydrates.** Plan validation runs in the core guest, which holds no network or global-store capability — and per the fence in §"One binary, two fences", the provisioning surface must not regrow a runtime role, nor the runtime a fetching one. A plan binding a pinned source adapter absent from the store therefore surfaces as a typed `adapter-not-installed` error naming the identity and the literal sync command, and the operator (or the driving skill) runs the sync trigger. Runtime store misses behave as `--frozen` always would: fail loudly, fetch nothing.

### Deployment manifest generation

The runtime stops reading a hand-authored `omnia.toml` in consumer projects. After hydration, the provisioning surface **generates** the deployment manifest into the per-project derived cache (out-of-tree, per the cache-layout decision): one `[[guest]]` per resolved pulled component (every adapter, plus the core when not embedded) pointing at `<store>/<name>@<version>.wasm`, the `[[mount]]` of the project directory as writable `"."`, one `[[route.http]]` MCP prefix per adapter, and the core guest's link allow-list — exactly the shape the in-tree developer manifest models today. The manifest is a derived artifact: regenerated whenever the declared component set or pins change, never committed, never hand-edited.

**The core guest is resolved embedded-first.** A binary built with Omnia's generic `runtime!` embed option (move 4) carries its core and wires it itself — no hydration, no manifest entry. Otherwise the kernel hydrates `specify:core@<the binary's own version>` like any adapter and the manifest references its store entry. Both modes pin the core to the binary version, so there is no core pin surface either way. Today's committed workflow guest (RFC-61 D-dist) is the embed posture with a hand-rolled mechanism; this cut replaces it with the pulled baseline or the generic embed, changing nothing in the kernel or the manifest generator.

### Cloud posture

The cloud story is the local story with the knobs exposed, not a second mechanism:

- **Relocatable root.** `SPECIFY_ADAPTER_STORE` points the store at a mounted volume or a restored cache directory; nothing else changes. `$HOME/.specify/adapters` is the default that makes the unmounted case still work.
- **Cache priming.** CI restores the store directory keyed on the project's pinned identities (a digest over `project.yaml.adapter` + `adapters:` + plan source pins is a stable cache key); `specify adapters sync` after restore is a no-op probe on a hit and a fetch on a miss. `--frozen` converts "miss" into "fail loudly" where fetching is forbidden.
- **Cross-machine digest pinning.** RFC-48 left the trust model at TOFU per machine. Hydration closes the gap with a committed lock: `.specify/adapters.lock` records each identity's component-byte digest at first install, and every subsequent hydration — any machine — verifies the store entry against the committed digest before use (`adapter-digest-mismatch` on drift). The lock is written by the kernel, committed like any lockfile, and makes a cloud runner's install byte-equivalent to the laptop that authored the pin. (It is also RFC-66's content-equivalence lever across a registry-host migration.)
- **No interactive seams.** Anonymous first-party pulls, environment credentials for mirrors, typed errors on every failure path, exit codes carrying through — a cloud agent drives hydration exactly as an operator does.

## Operator onboarding

The full operator journey is two commands and no documentation:

```bash
brew install augentic/tap/specify   # RFC-66's one door
cd my-project && specify init
```

`specify init` is the front door onto the hydration kernel plus the core guest's scaffold leg, in bootstrap order: hydrate `specify:core@<the binary's own version>` when the binary does not embed its core, plus the declared adapters; generate the deployment manifest; then invoke the guest's scaffold leg through the normal host form — `project.yaml`, `registry.yaml` (workspace mode), and the `.specify/` tree are project-scoped state, so the guest writes them. Either way the operator holds one version knob — the binary's — and after init every workflow verb runs offline against the warm store. The ergonomic rules:

- **Product nouns only.** The operator's vocabulary during init is target, sources, platforms, project name — never store, manifest, component, or pin. Distribution is an output of init, not an input.
- **Flags are the substrate.** `specify init <target> --platforms core,ios --sources intent` is fully non-interactive with typed errors naming exactly the missing flag; CI, cloud agents, and skills use this form. Nothing below the elicitation layer ever prompts — the kernel's non-interactive property is untouched.
- **Guided layers elicit flags and call the same path.** In Cursor, `/spec:init` elicits conversationally (the house skill model: elicit missing arguments, invoke, relay). In a bare terminal, a minimal prompt mode — arrow-select target, platforms when the target requires them, sources defaulting to `intent` — engages only when stdin is a TTY and required arguments are missing. Both layers end by printing the equivalent flag invocation (teaching the non-interactive form) and the literal next command (the house pattern Gate 1 already follows).
- **Idempotent re-entry.** `specify init` on an initialized project detects it and offers `--upgrade`; rerunning the door is always safe, never an error.
- **Preflight folded into the report.** Init's postflight names what was provisioned and surfaces missing prerequisites (`cursor-agent` on `PATH`, registry reachability) with fix commands rather than letting the operator discover them three commands later.

## The five moves

1. **Widen the guest surface to everything project-scoped.** Route all pure workflow verbs through the guest alongside the orchestrators, so the core guest is the sole authority over `.specify/` state. `rules export`, `archive prune`, and `completions` (pure stdout from the wasm-clean clap grammar) join them. Cost: per-invocation component instantiation on trivial verbs (`plan status` pays a composed-runtime spin-up). The latency measurement gates the move; an unacceptable result is fixed by making instantiation cheaper — host-side caching generally, or move 4's precompiled embed where that mode is in play — never by re-splitting the grammar.
2. **Externalize the host.** Replace the hand-rolled embedded host with the macro-generated command-mode runtime described above. Everything Specify-specific stays in the guests, the generated manifest, and the provisioning surface.
3. **Provision from the store.** Land the store-root move, the hydration kernel with its two triggers, the committed `.specify/adapters.lock`, and manifest generation (§"The provisioning half"). The kernel is surface-agnostic and can land against today's binary shape ahead of the cut; the generated manifest is then the only deployment description the host layer reads — no transient assembly or per-invocation staging.
4. **Version the core guest by the binary; resolve it embedded-first.** The baseline mechanism needs nothing from Omnia: the release pipeline publishes `specify:core@<binary version>` (an RFC-66 job cut from the same tag), and the binary hydrates exactly that identity through the kernel at init — the pin is the binary's own version, so there is no second knob and no pin surface for a project to drift on. When Omnia's generic embed option is available and the build opts in, the same component is instead compiled into the binary — optionally precompiled (a serialized artifact keyed to the compiled-in wasmtime version, safe precisely because the bytes are the build's own) so per-verb instantiation is near-instant — and core hydration disappears along with the manifest's core entry. The two modes are interchangeable below the seam: same component, same version discipline, same operator experience. In both, the committed `crates/workflow-guest/guest.wasm`, its `.sha256` sidecar, and the `tests/dist.rs` staleness gate are deleted — the guest is built from the tagged source at publish or build time, never committed. A dev-deployment path override (manifest or env) loads the core from `target/wasm32-wasip2/` so core-guest iteration never republishes or rebuilds the host binary; the override is a development affordance, never a release mode.
5. **Shrink the native binary to the provisioning surface.** What remains native is the closed provisioning set — `init`, `adapters sync`, `upgrade`, `plugins` — plus blind forwarding of everything else. The RFC-61 triage main died because it compiled the full workflow grammar natively and served workflow verbs in-process; the provisioning front does neither: it parses no workflow argv and serves no workflow verb. A core guest embedded via the generic macro option is the host layer's payload, not a hand-rolled D-dist embed and not provisioning-front code. Developer build/publish concerns stay in RFC-66's scripted workflows; a dedicated dev-tool binary remains YAGNI.

## Command routing

### Guest — every project-scoped verb:

| Verb family           | Rationale                                                                                                                                                                                                                                                |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `registry`            | Validated edits to `registry.yaml` inside the mounted workspace root — workspace-scoped state like any plan or slice artifact; no network, no global store. Leaving its writer native would breach the guest's sole authority over project-scoped state. |
| `workspace sync/push` | Mounting the workspace root as `"."` makes multi-slot writes plain path resolution; the git transport mechanism from the guest is this RFC's one open design item.                                                                                       |
| `completions`         | Pure stdout from the wasm-clean clap grammar in `specify-dispatch`.  |
| init's scaffold leg   | `project.yaml`, `registry.yaml`, and the `.specify/` tree are project-scoped state; the provisioning front invokes this leg through the host form after hydration.  |

### Provisioning surface — native, closed set:

| Verb            | Why it cannot be in-guest                                                                                                                                                                                              |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `init`          | Hydration is network pulls and global-store writes the runtime world deliberately lacks, and manifest generation must precede the first guest-bound invocation. Hydrates the core (when not embedded) and the declared adapters, generates the manifest, then invokes the guest scaffold leg. |
| `adapters sync` | The explicit hydration trigger — the same network and global-store capabilities as init's hydration leg, plus manifest regeneration.                                                                              |
| `upgrade`       | Replaces the binary through its install channel (brew / cargo / binary archive); a guest cannot replace its own host. Upgrading the binary re-pins (or re-embeds) the core with it.                                       |
| `plugins`       | Dev-machine Cursor plugin-cache probing and repair — host-environment access by definition.                                                                                                                              |

### Developer scripts — no product surface:

| Concern                   | Rationale                                                                                                                                       |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `adapter build`           | Needs the host Rust toolchain for the `wasm32-wasip2` cross-build.                                                                                |
| `adapter publish`         | RFC-66's tag-driven workflows own publishing (idempotent `wkg publish` loops, `GITHUB_TOKEN` auth); locally, the same `cargo make publish-*` tasks. |

### Neither surface:

| Verb family      | Rationale                                                                                |
| ---------------- | ---------------------------------------------------------------------------------------- |
| `lint project`   | Out of scope (YAGNI). If it earns its way back, it returns as developer tooling.         |
| `lint framework` | Framework-repo CI tooling (`make lint`); not part of an operational or dev-tool surface. |

## One binary, two fences

- **Operational surface** — the whole workflow grammar lives guest-side in `specify-dispatch`; exit codes pass through `wasi:cli/exit` verbatim. Skills shell out to the `specify` binary, which forwards their argv unparsed.
- **Provisioning surface** — the closed verb set above, running only before or between workflow invocations, never inside the loop. The admission test for a new native verb is *impossibility in-guest by construction* (the capability fence on fetch, self-replacement, host-environment probing) — never "the guest form is slow or awkward".
- **Developer scripts** — off the operator's path entirely, owned by RFC-66, with no product lifecycle of their own.

A ground-zero spike — macro host plus hand-written manifest over locally built components — validates acceptance criterion 1 and produces the per-verb instantiation latency numbers gating move 1; prototyping the optional embed against a locally built core belongs in the same spike if the Omnia option exists by then.

## Scope

- The generic host layer (Omnia-side coordination), removal of the embedded host surface and the workflow-verb triage main, and the provisioning front with its closed verb set.
- The store-root relocation to `$HOME/.specify/adapters`, including the `$SPECIFY_ADAPTER_CACHE` → `$SPECIFY_ADAPTER_STORE` env-override rename.
- The hydration kernel and its two provisioning-surface triggers, including the `project.yaml.adapters:` prefetch list (additive, optional), `specify adapters sync` with `--frozen`, and the committed `.specify/adapters.lock` digest pin.
- The typed `adapter-not-installed` posture for plan-time and runtime store misses (the guest-never-hydrates fence).
- Deployment-manifest generation from resolved store entries (the core guest appears as one more resolved entry when pulled; an embedded core needs none — move 4).
- The binary-versioned core-guest path: the published `specify:core@<binary version>` baseline (publish job in RFC-66), opt-in adoption of the generic embed when Omnia lands it, the dev-deployment path override, and deletion of the committed workflow-guest artifact, its sidecar, and the `tests/dist.rs` gate.
- The naming cut: the WIT adapter-contract package rename from `augentic:specify` to `specify:adapter`, and the first-party package-identity rename from `augentic:<name>` to `specify:<name>` (adapters and the `specify:core` guest), across both repos — the `wit/specify.wit` package declaration, `wit_bindgen` glue, manifest `link` allow-lists, the `adapter_uri.rs` package-reference parser and its first-party shorthand sugar, wasm-pkg namespace routing, and prose references. Pre-1.0 hard-cut posture applies: pinned projects re-init; no compatibility aliases for the old identities.
- The operator-onboarding surface: the flag-driven init substrate, the `/spec:init` elicitation layer, the TTY prompt mode, idempotent re-entry via `--upgrade`, and the postflight report with the literal next command.
- Guest-side landing of `registry`, `rules export`, `archive prune`, `completions`, `workspace sync/push` (including pinning the git transport mechanism), and init's scaffold leg.
- Latency measurement for the guest widening (the precompiled embed is the expected answer; host-side caching is the fallback lever).
- Skills' shell-out grammar migration to the `specify` binary's forwarding form.
- Removal of `lint project` from the operational surface.

## Out of scope

- **Omnia OCI guest sources, version ranges, and a release index** — hydration requires exact pins; RM-21 owns ranges, floors, and the compatibility matrix.
- **Registry backing and publish workflows** — the well-known file, GHCR packages, idempotent publish loops, and the `specify:core` publish job are [RFC-66](rfc-66-publishing-and-distribution.md)'s.
- **Third-party adapter namespaces** — the first-party `specify:` posture is unchanged.
- **Store garbage collection** — entries are immutable and shared across projects; a retention policy over unreferenced identities is a follow-up, not a blocker (the store grows by one file per `(name, version)` ever used).
- **Requiring the Omnia embed option** — it is an opt-in acceleration; the cut lands and ships on the pulled `specify:core` baseline whether or not the option ever exists.
- **In-guest hydration** — the outbound-HTTP-plus-store-mount capability path stays open as a later *provisioning world* (a separate world from the runtime's, so the runtime world stays fetch-free and the guest-never-fetches fence holds), but it is optional purity, not this RFC's work.
- **A dedicated dev-tool binary** — YAGNI. Developer concerns are RFC-66's scripted workflows until a product-shaped tool earns its way in; its crate split, name, and distribution channel are deferred with it.
- **Backend selection as deployment configuration** — the blessed cursor-bound host layer suffices; generalising `runtime!` backend binding is Omnia's own RFC.
- **Sandboxing and permission narrowing of the cursor backend** — phase after this cut, per RFC-61.
- **Multi-node or long-lived deployments** — the host layer remains one command-mode invocation per verb; RFC-55 stays deferred.

## Acceptance criteria

1. The host layer contains no Specify domain logic and no verb knowledge, and runs the full workflow loop from the generated manifest plus the embedded core: `specify plan execute` forwards to the guest and drains a plan with exit-code passthrough.
2. The core guest carries the binary's version in both modes — pulled (`specify:core@<binary version>` hydrated at init) or embedded (via the macro's generic option, never hand-rolled) — with no committed `guest.wasm`, no `include_bytes!` guest payload in Specify code, and identical operator-visible behavior across the two modes.
3. Every project-scoped verb reaches the guest; the native verb set is exactly `{init, adapters sync, upgrade, plugins}`; nothing native carries a workflow verb, and non-provisioning argv forwards unparsed. Envelopes and exit codes are unchanged across the seam.
4. Only the generated manifest describes the pulled deployment; no transient assembly or workflow-verb triage.
5. On a fresh machine, `specify init` against a `project.yaml` with pinned identities downloads every declared component into `$HOME/.specify/adapters/<name>@<version>.wasm`, generates the deployment manifest, and leaves the project runnable — no sibling checkout, no vendored tree, no hand-authored `omnia.toml`. On a fresh macOS machine specifically, `brew install augentic/tap/specify` followed by `specify init` (guided or flag-driven) reaches an initialized, plan-ready project with no third command and no hand-edited YAML; init's network fetches are at most the core and the declared adapters, and subsequent workflow verbs succeed offline against the warm store.
6. A warm store makes init and sync no-op probes; two concurrent hydrations of one identity produce one immutable entry and two successes. `specify adapters sync --frozen` fails with a typed error naming any missing identity and fetches nothing.
7. A plan binding a pinned source adapter absent from the store fails validation with `adapter-not-installed`, naming the identity and the literal sync command; no guest code path performs a network fetch or a global-store write. An unpinned name resolves project-locally exactly as today.
8. Every hydrated entry is verified against `.specify/adapters.lock` when the lock carries its identity; drift aborts with `adapter-digest-mismatch` before any guest loads.
9. Relocating the store via `SPECIFY_ADAPTER_STORE` changes no behavior other than the root path — the generated manifest follows the resolved entries.
10. Skills invoke the `specify` binary's forwarding form and the full plan-driven loop passes the composed integration suites and evals.
11. `make lint` and `cargo make ci` are green in both repos, and DECISIONS.md records the amendments to D-bin and D-dist.

## Risks and invariants

- **Version skew narrows to adapters.** The core pin is the binary's own version — structural under the embed, procedural under the pull (where a release whose `specify:core` push fails must fail entirely, per RFC-66) — and the adapter `specify-floor` discipline remains the runtime backstop; skew must surface as a typed error at deployment build, never an Omnia load panic.
- **The store is load-bearing at runtime.** Deleting an entry breaks every project whose generated manifest references it; the manifest generator must verify entry presence at generation time and the runtime must fail with a typed error (not an Omnia load panic) on a dangling path. This is also why the root leaves `$XDG_CACHE_HOME`.
- **TOFU is only closed by the lock.** Without a committed `.specify/adapters.lock`, a first install on a new machine still trusts the registry; teams wanting supply-chain strictness commit the lock and run `--frozen` in CI.
- **Pins are the determinism boundary.** Nothing in hydration consults "latest"; a bare name never crosses the network. Loosening this requires RM-21's version-resolution design, not an ad-hoc default.
- **The guest never fetches.** The typed-error posture on store misses is a fence, not a stopgap: an in-guest hydration convenience would hand the runtime a network capability and re-blur the provisioning / runtime split. The fix for a friction-heavy miss is a better sync prompt, never a guest-side pull.
- **The generated manifest is derived, never authored.** Hand edits are lost on regeneration by design; deployment customization (if ever needed) enters through project configuration, not through the artifact.
- **Resolution order is unchanged.** Store-first for pins, then project-local (component cache, then the sibling development build) — hydration adds fetch-on-miss at the provisioning-surface triggers, not a new probe order, so the sibling-checkout development posture keeps working untouched.
- **The Omnia embed option is an acceleration, never a dependency.** The cut lands and ships on the pulled `specify:core` baseline. When the option arrives it must be generic — a path parameter with no Specify vocabulary — or be refused, and switching modes must change no operator-visible behavior (acceptance criterion 2 is the check).
- **Per-verb latency is a gate, not a footnote.** The ground-zero spike measures; a failing result is fixed host-side — instantiation caching generally, the precompiled embed where that mode is in play — never by re-splitting the operator grammar.
- **Dev/prod divergence on the path override.** Release binaries always run their release core (the published pin or the embed); the path override exists for the development loop only and never ships enabled — graded evidence (suites, evals) comes from release-mode builds, mirroring the RFC-62 prose-overlay posture.
- **The workspace git mechanism is the one open design item.** Multi-slot writes reduce to mount topology, but `workspace push` needs a git transport story from the guest (host capability, wasm-native git, or the model backend's agent); this RFC does not land until that mechanism is pinned, even if its implementation follows separately.
- **Omnia stays domain-free.** The host layer gains no Specify vocabulary; everything Specify-shaped lives in the guests, the generated manifest, and the provisioning front.
- **The provisioning surface must not regrow a runtime role.** The closed verb set is the fence, and the admission test is impossibility in-guest by construction — a "convenient" native fallback for one slow or awkward workflow verb re-creates the triage main. The acceptance criteria (exact native verb set, unparsed forwarding) are the ratchet.
- **Guided init must stay a veneer.** The elicitation layers (skill and TTY prompts) only gather flags for the one non-interactive substrate; the moment hydration, manifest generation, or scaffolding can prompt, the kernel's non-interactive property breaks and the cloud posture forks from the laptop posture.
- **Sequencing.** Move 3 (the provisioning half) is surface-agnostic and can land against today's binary shape ahead of the cut; RFC-66's `specify:core` publish job can land inert ahead of the cut, and its registry and tap work is orthogonal and may land first. The optional embed rides Omnia's timetable without blocking anything.
