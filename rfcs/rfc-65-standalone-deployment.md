# RFC-65: The Standalone Deployment — Specify as a Guest of a Generic Omnia Host

> Status: Proposed · Depends: [RFC-63](rfc-63-adapter-hydration.md) (generated deployment manifest, hydration kernel), RFC-64 (one-component adapter artifact, wasm-pkg transport), and an Omnia-side generic guest-embed option on `runtime!` (this RFC's one cross-repo ask; move 4 carries the fallback if it falls through) · Related: [RFC-66](rfc-66-publishing-and-distribution.md) (the publish/acquire plumbing for the adapters, the WIT contract, and the binary) · Amends: [RFC-61](rfc-61-omnia-migration.md) decisions D-bin (triage main) and D-dist (embedded workflow guest — the embed survives, re-mechanised as the macro's generic option) · Owns: the operational surface after the embedded host retires

## Abstract

Specify's operator surface becomes one shipped `specify` binary with **two strictly separated layers**: a thin native **provisioning surface** carrying the closed verb set the capability fence makes irreducibly native (`init`, `adapters sync`, `upgrade`, `plugins`), and a **generic, Specify-agnostic Omnia host layer** driven by the generated deployment manifest, through which every project-scoped verb runs in the core guest. The core guest is **embedded in the binary at build time through a generic `runtime!` embed option** — precompiled for instant instantiation, versioned by the binary itself, with no registry identity of its own — while adapters stay published, pinned, and hydrated (RFC-63/66). Anything that is not a provisioning verb forwards blindly — unparsed — to the guest's `wasi:cli/run`. The developer remainder (adapter build, publish) needs no product surface at all: RFC-66's tag-driven workflows own it. No workflow-verb triage, no hand-rolled embeds, no separately versioned core artifact: the operational shape matches the Omnia cursor example (`backends/examples/cursor`) — a macro-generated host, a manifest, and pulled adapter components — with the core guest riding inside the binary so the binary version *is* the core version.

## Operational model

- **Host layer** — a command-mode runtime from `omnia::runtime!({ mode: command, hosts: { WasiHttp: HttpDefault, WasiOtel: OtelDefault, WasiModel: Cursor } })`. The macro forwards argv to the guest's `wasi:cli/run`; backend binding stays compile-time (Omnia ships a blessed cursor-bound host layer; backend selection as deployment configuration is a later Omnia-side option). The macro gains this RFC's one Omnia-side ask: a **generic, domain-free embed option** — a build-time parameter naming a component to compile into the binary (optionally precompiled against the macro's own wasmtime version) and expose as a loadable guest. The host layer carries no Specify vocabulary; the embedded bytes are supplied by the consuming build.
- **Provisioning surface** — the native front of the shipped binary, with a closed verb set: `init`, `adapters sync`, `upgrade`, `plugins`. Each is native because it *cannot* run in-guest by construction, not because it is convenient: `init` and `adapters sync` are the RFC-63 hydration triggers — network pulls and global-store writes the runtime world deliberately lacks (RFC-63's guest-never-fetches fence); `upgrade` replaces the binary through its install channel — a guest cannot replace its own host; `plugins` probes and repairs the dev machine's Cursor plugin cache. The provisioning surface carries no workflow verbs and never runs during the workflow loop — it precedes it.
- **Manifest** — RFC-63's generated deployment manifest is the sole authority over the **adapter half** of the deployment: adapter guests by store path, the project mount, MCP routes, the link allow-list. The core guest needs no manifest entry — it ships inside the binary, wired by the host layer itself. Adapter discovery and wiring are resolved at manifest generation time, not per invocation.
- **Components** — the core guest (embedded; the workflow grammar's home; **no registry identity** — its version is the binary's) plus one guest per bound adapter, published, pinned, and hydrated identically: the generated manifest references each adapter by its single-file store entry path (`<store-root>/<name>@<version>.wasm`, RFC-64). The core guest defines no WIT package of its own — its world is an anonymous `export wasi:cli/run` plus the adapter-contract imports.
- **Naming** — namespaces follow product, not org. `omnia:` remains the runtime's interface namespace (`omnia:model/completion`); everything Specify-owned lives in the `specify:` namespace: `specify:adapter` is the WIT adapter contract (imports read `specify:adapter/source`, `specify:adapter/target` — renamed from `augentic:specify`), and `specify:<name>` is each adapter package (`specify:omnia@<semver>`, renamed from `augentic:omnia@<semver>` — the namespace disambiguates the omnia adapter from the Omnia runtime's own packages). Both namespaces route to the first-party registry in wasm-pkg config; `augentic:` is reserved for future org-wide contracts and is otherwise retired. The core guest takes no package name — it is not published.
- **Developer tooling** — adapter build and publish stay off the product surface entirely: `cargo make` tasks called by RFC-66's tag-driven release workflows. No dev-tool binary ships with this RFC.

Invocation form: `specify <verb …>`. A provisioning verb runs natively; any other argv forwards unparsed as `<host layer> run --config <generated manifest> -- <verb …>`, with the binary resolving the manifest path itself (it generated it — operators never type `--config`). The bare host form remains available for debugging and Omnia-native deployments.

### In-guest argument parsing

The core guest parses forwarded argv exactly as a Rust-native binary would — the Omnia CLI example (`omnia/examples/cli`) is the template. The host layer forwards argv to the guest's `wasi:cli/run` (`argv[0]` supplied by the host), and argv plus stdout/stderr arrive through the p2 `std` bridge Omnia links alongside p3 — so the existing clap grammar in `specify-dispatch` runs unmodified in-guest: `--help`, `--version`, and usage errors need no hand-rolling. One seam nuance the implementer must carry over: use `try_parse()` rather than `parse()`, and forward `clap::Error::exit_code()` through the p3 `wasi:cli/exit` (`wasip3::cli::exit::exit_with_code`). `parse()`'s internal `std::process::exit` lands on the *p2* exit, which carries only success/failure and would collapse clap's usage-error code `2` to `1`. Either way the host observes the exit as wasmtime's `I32Exit` and its generated `main` exits with it — the exit-code contract passes through verbatim. If the embedded core guest's size matters, the example's trims apply: clap with `default-features = false` (keeping `derive`, `error-context`, `help`, `std`, `usage`) drops the color and suggestion machinery.

### Operator onboarding

The full operator journey is two commands and no documentation:

```bash
brew install augentic/tap/specify   # RFC-66's one door
cd my-project && specify init
```

`specify init` is the front door onto RFC-63's hydration kernel plus the core guest's scaffold leg, in bootstrap order: hydrate the declared adapters (the core guest already rides inside the binary — one version knob by construction), generate the deployment manifest, then invoke the guest's scaffold leg through the normal host form — `project.yaml`, `registry.yaml` (workspace mode), and the `.specify/` tree are project-scoped state, so the guest writes them. Because the core is embedded, init's only network need is the adapters, and every workflow verb works offline thereafter. The ergonomic rules:

- **Product nouns only.** The operator's vocabulary during init is target, sources, platforms, project name — never store, manifest, component, or pin. Distribution is an output of init, not an input.
- **Flags are the substrate.** `specify init <target> --platforms core,ios --sources intent` is fully non-interactive with typed errors naming exactly the missing flag; CI, cloud agents, and skills use this form. Nothing below the elicitation layer ever prompts — RFC-63's non-interactive kernel property is untouched.
- **Guided layers elicit flags and call the same path.** In Cursor, `/spec:init` elicits conversationally (the house skill model: elicit missing arguments, invoke, relay). In a bare terminal, a minimal prompt mode — arrow-select target, platforms when the target requires them, sources defaulting to `intent` — engages only when stdin is a TTY and required arguments are missing. Both layers end by printing the equivalent flag invocation (teaching the non-interactive form) and the literal next command (the house pattern Gate 1 already follows).
- **Idempotent re-entry.** `specify init` on an initialized project detects it and offers `--upgrade`; rerunning the door is always safe, never an error.
- **Preflight folded into the report.** Init's postflight names what was provisioned and surfaces missing prerequisites (`cursor-agent` on `PATH`, registry reachability) with fix commands rather than letting the operator discover them three commands later.

## The five moves

1. **Widen the guest surface to everything project-scoped.** Route all pure workflow verbs through the guest alongside the orchestrators, so the core guest is the sole authority over `.specify/` state. `rules export`, `archive prune`, and `completions` (pure stdout from the wasm-clean clap grammar) join them. Cost: per-invocation component instantiation on trivial verbs (`plan status` pays a composed-runtime spin-up). The precompiled embed (move 4) is expected to defuse this; the latency measurement still gates the move, and an unacceptable result is fixed by making instantiation cheaper (host-side caching), not by re-splitting the grammar.
2. **Externalize the host.** Replace the hand-rolled embedded host with the macro-generated command-mode runtime described above. Everything Specify-specific stays in the guests, the generated manifest, and the provisioning surface.
3. **Persist the manifest.** The generated deployment manifest is the only deployment description the host layer reads for the adapter half — no transient assembly or per-invocation staging.
4. **Embed the core guest through the generic embed option.** The release build compiles the core guest from the tagged tree to `wasm32-wasip2` and hands it to the macro's embed parameter; the macro may precompile it (a serialized artifact keyed to the compiled-in wasmtime version — safe precisely because the bytes are the build's own) so per-verb instantiation is near-instant. Consequences: the binary↔core version lockstep holds by construction (no publish leg to fail, no skew surface); `specify init` fetches adapters only and workflow verbs run offline; RFC-66 carries no `specify:core` publish job; the committed `crates/workflow-guest/guest.wasm`, its `.sha256` sidecar, and the `tests/dist.rs` staleness gate are deleted — the guest is built from source at build time, never committed. A dev-deployment path override (manifest or env) loads the core from `target/wasm32-wasip2/` so core-guest iteration never rebuilds the host binary; the override is a development affordance, never a release mode. **Fallback:** if the Omnia embed option is declined or delayed, this move reverts to a published `specify:core@<binary version>` pulled through the RFC-63 kernel and published by an RFC-66 workflow job; the choice is isolated to this move — nothing else in this RFC changes.
5. **Shrink the native binary to the provisioning surface.** What remains native is the closed provisioning set — `init`, `adapters sync`, `upgrade`, `plugins` — plus blind forwarding of everything else. The RFC-61 triage main died because it compiled the full workflow grammar natively and served workflow verbs in-process; the provisioning front does neither: it parses no workflow argv and serves no workflow verb. The embedded core guest is the host layer's payload through the generic macro option, not a hand-rolled D-dist embed and not provisioning-front code. Developer build/publish concerns stay in RFC-66's scripted workflows; a dedicated dev-tool binary remains YAGNI.

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
| `init`          | Hydration is network pulls and global-store writes the runtime world deliberately lacks, and manifest generation must precede the first adapter-bound invocation. Hydrates adapters, generates the manifest, then invokes the guest scaffold leg. |
| `adapters sync` | The explicit RFC-63 hydration trigger — the same network and global-store capabilities as init's hydration leg, plus manifest regeneration.                                                                              |
| `upgrade`       | Replaces the binary through its install channel (brew / cargo / binary archive); a guest cannot replace its own host. Upgrading the binary upgrades the embedded core with it.                                            |
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

A ground-zero spike — macro host plus hand-written manifest over locally built components — validates acceptance criterion 1 and produces the per-verb instantiation latency numbers gating move 1; embedding a locally built core through a prototype of the embed option belongs in the same spike.

## Scope

- The generic host layer (Omnia-side coordination), removal of the embedded host surface and the workflow-verb triage main, and the provisioning front with its closed verb set.
- The Omnia-side generic embed option (coordination), the precompiled embed of the release-built core guest, the dev-deployment path override, and deletion of the committed workflow-guest artifact, its sidecar, and the `tests/dist.rs` gate.
- The naming cut: the WIT adapter-contract package rename from `augentic:specify` to `specify:adapter`, and the first-party adapter package-identity rename from `augentic:<name>` to `specify:<name>`, across both repos — the `wit/specify.wit` package declaration, `wit_bindgen` glue, manifest `link` allow-lists, the `adapter_uri.rs` package-reference parser and its first-party shorthand sugar, wasm-pkg namespace routing, and prose references. Pre-1.0 hard-cut posture applies: pinned projects re-init; no compatibility aliases for the old identities.
- The operator-onboarding surface: the flag-driven init substrate, the `/spec:init` elicitation layer, the TTY prompt mode, idempotent re-entry via `--upgrade`, and the postflight report with the literal next command.
- Guest-side landing of `registry`, `rules export`, `archive prune`, `completions`, `workspace sync/push` (including pinning the git transport mechanism), and init's scaffold leg.
- Latency measurement for the guest widening (the precompiled embed is the expected answer; host-side caching is the fallback lever).
- Skills' shell-out grammar migration to the `specify` binary's forwarding form.
- Removal of `lint project` from the operational surface.

## Out of scope

- **Omnia OCI guest sources, version ranges, third-party namespaces** — unchanged (RFC-63/64 postures hold).
- **Publishing the core guest as a registry artifact** — the fallback shape only (move 4); adopted only if the Omnia embed option falls through.
- **In-guest hydration** — the outbound-HTTP-plus-store-mount capability path stays open as a later *provisioning world* (a separate world from the runtime's, so the runtime world stays fetch-free and RFC-63's guest-never-fetches fence holds), but it is optional purity, not this RFC's work.
- **A dedicated dev-tool binary** — YAGNI. Developer concerns are RFC-66's scripted workflows until a product-shaped tool earns its way in; its crate split, name, and distribution channel are deferred with it.
- **Backend selection as deployment configuration** — the blessed cursor-bound host layer suffices; generalising `runtime!` backend binding is Omnia's own RFC.
- **Sandboxing and permission narrowing of the cursor backend** — phase after this cut, per RFC-61.
- **Multi-node or long-lived deployments** — the host layer remains one command-mode invocation per verb; RFC-55 stays deferred.

## Acceptance criteria

1. The host layer contains no Specify domain logic and no verb knowledge, and runs the full workflow loop from the generated manifest plus the embedded core: `specify plan execute` forwards to the guest and drains a plan with exit-code passthrough.
2. The core guest is embedded through the macro's generic option: no hand-rolled `include_bytes!` guest payload in Specify code, no committed `guest.wasm`, no `specify:core` registry package — and the binary version is the core version by construction.
3. Every project-scoped verb reaches the guest; the native verb set is exactly `{init, adapters sync, upgrade, plugins}`; nothing native carries a workflow verb, and non-provisioning argv forwards unparsed. Envelopes and exit codes are unchanged across the seam.
4. Only the generated manifest describes the adapter deployment; no transient assembly or workflow-verb triage.
5. On a fresh macOS machine, `brew install augentic/tap/specify` followed by `specify init` (guided or flag-driven) reaches an initialized, plan-ready project with no third command and no hand-edited YAML; init's only network fetches are adapters, and subsequent workflow verbs succeed offline.
6. Skills invoke the `specify` binary's forwarding form and the full plan-driven loop passes the composed integration suites and evals.
7. `make lint` and `cargo make ci` are green in both repos, and DECISIONS.md records the amendments to D-bin and D-dist.

## Risks and invariants

- **Version skew narrows to adapters.** The embed makes core skew impossible by construction; the adapter `specify-floor` discipline remains the runtime backstop, and skew must surface as a typed error at deployment build — never an Omnia load panic.
- **The Omnia embed option is the one cross-repo dependency.** It must arrive generic — a path parameter with no Specify vocabulary — or be refused; and if it is declined or late, move 4's fallback (a published `specify:core`) holds the cut's schedule without touching any other move.
- **Per-verb latency is a gate, not a footnote.** The precompiled embed is expected to answer it; the ground-zero spike still measures, and a failing result is fixed host-side (caching), never by re-splitting the operator grammar.
- **Dev/prod divergence on the path override.** Release binaries always run the embedded core; the path override exists for the development loop only and never ships enabled — graded evidence (suites, evals) comes from embedded builds, mirroring the RFC-62 prose-overlay posture.
- **The workspace git mechanism is the one open design item.** Multi-slot writes reduce to mount topology, but `workspace push` needs a git transport story from the guest (host capability, wasm-native git, or the model backend's agent); this RFC does not land until that mechanism is pinned, even if its implementation follows separately.
- **Omnia stays domain-free.** The host layer gains no Specify vocabulary; everything Specify-shaped lives in the guests, the generated manifest, and the provisioning front.
- **The provisioning surface must not regrow a runtime role.** The closed verb set is the fence, and the admission test is impossibility in-guest by construction — a "convenient" native fallback for one slow or awkward workflow verb re-creates the triage main. The acceptance criteria (exact native verb set, unparsed forwarding) are the ratchet.
- **Guided init must stay a veneer.** The elicitation layers (skill and TTY prompts) only gather flags for the one non-interactive substrate; the moment hydration, manifest generation, or scaffolding can prompt, RFC-63's non-interactive kernel property breaks and the cloud posture forks from the laptop posture.
- **Sequencing.** RFC-63 (hydration, generated manifest) is a prerequisite; the Omnia embed option can be prototyped inside the ground-zero spike (embed a locally built core). RFC-66's registry and tap work is orthogonal and may land first.
