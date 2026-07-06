# RFC-65: The Standalone Deployment — Specify as a Guest of an External Omnia Host

> Status: Proposed · Depends: [RFC-63](rfc-63-adapter-hydration.md) (generated deployment manifest, hydration), [RFC-64](rfc-64-adapter-artifact.md) (one-component adapter artifact, wasm-pkg transport) · Amends: [RFC-61](rfc-61-omnia-migration.md) decisions D-bin (triage main) and D-dist (embedded workflow guest) · Owns: the shape of the operational binary after the embedded host retires

## Abstract

RFC-61 delivered the inversion: `workflow.wasm` exists, owns every orchestration, and runs under the Omnia runtime. But the Omnia host is **embedded inside the native `specify` binary** — a triage main parses argv, routes seven orchestrator verbs through an in-process `omnia::run` over a transient manifest, and dispatches everything else natively. This RFC proposes the stronger cut: the workflow component is hosted by a **standalone, Specify-agnostic Omnia host binary** driven entirely by the generated deployment manifest, every project-scoped verb runs in-guest, and the native `specify` binary shrinks to the bootstrap residue that WASI cannot express. The deliverable of this RFC is primarily a decision: whether relocating the native surface (it cannot be deleted — see [The residue](#the-residue-what-stays-native-and-why)) buys enough architectural clarity to justify amending two settled decisions.

## Where the seam sits today

Decision D-bin (`DECISIONS.md` §"One `specify` binary") deliberately kept one native binary because the `runtime!` macro emits a private `main` that cannot wrap a triage layer, and because part of the CLI is inherently host-side. The current split after the S4 widening:

- **Guest-routed** — `plan execute`, `plan author`, `slice refine`, `slice build`, `slice merge run`, `source survey`, `source extract`: staged through `src/runtime/commands/guest.rs` (transient `omnia.toml`, embedded guest bytes, `specify_runtime::drive`).
- **Native but guest-capable** — every pure workflow verb (`plan create/add/amend/next/status/transition/…`, `slice create/validate/…`, `journal`, `source resolve`, `target resolve`): the wasm-clean `specify-dispatch` crate already gives the guest in-process handlers for all of them; the native binary simply doesn't route them there.
- **Native only** — the host-machine residue: `init`, `upgrade`, `plugins`, `lint`, `workspace`, `registry`, `adapter`, `rules`, `archive`, `completions`.

## The four moves

1. **Widen the guest surface to everything project-scoped.** Route the pure workflow verbs through the guest alongside the orchestrators, so the workflow component is the sole authority over `.specify/` state. The handlers already exist in-guest; the change is the triage predicate. Cost: per-invocation deployment assembly and component instantiation on trivial verbs (`plan status` pays a composed-runtime spin-up). If the latency is unacceptable, this move is severable — the standalone host can keep the D-bin triage set.
2. **Externalize the host.** A standalone command-mode host binary — `omnia run <manifest> -- <argv>` or a thin published `specify-host` — replaces the hand-rolled `CursorBundle` / `Hooks` embedding. This is Omnia-side work: backend binding is compile-time today (one `runtime!` = one backend bundle), so either backend selection becomes deployment configuration, or Omnia ships a blessed cursor-bound host binary. Everything Specify-specific must stay out of it (the RFC-61 invariant: Omnia stays domain-free).
3. **Persist the manifest; delete the transient assembly.** RFC-63's generated deployment manifest becomes the only deployment authority. The adapter-discovery and manifest-rendering logic in `src/runtime/commands/guest.rs` moves wholesale into manifest generation; the temp-dir staging path deletes.
4. **Publish the workflow guest as an artifact.** D-dist embeds the component via `include_bytes!`; RFC-64 explicitly scoped its distribution out ("it ships embedded in the `specify` binary"). A standalone host needs `workflow.wasm` pulled like any other component: published as a wasm-pkg package (`augentic:workflow@<semver>`), hydrated into the store, pinned by the project, referenced by path from the generated manifest. The staleness-sidecar discipline (`guest.wasm.sha256`, `cargo make dist-guest`) retires with the committed artifact.

## The residue: what stays native, and why

These verbs cannot run inside a WASI guest under Omnia's current capability set. They are the reason a native binary survives in any design, and the honest core of this RFC's trade-off:

| Verb family | Why it is host-side |
| ----------- | ------------------- |
| `init`, `registry`, `adapter publish` | Network OCI pulls; writes to the global store outside the `"."` mount |
| `adapter build` | Needs a host Rust toolchain for the `wasm32-wasip2` cross-build |
| `upgrade` | Replaces the host binary itself |
| `plugins` | Cursor's plugin cache on the host filesystem, outside any mount |
| `workspace sync/push` | Multi-slot writes and git; already a documented guest gap (D-workspace) |
| `lint project` | Hosts *other* WASI tools through wasmtime |
| `completions`, `lint framework` | Shell integration; repo-development tooling |

Two ways to house them, decided here:

- **Option A (recommended): a minimal native `specify` bootstrap CLI.** It keeps exactly the residue table plus one dispatch rule — any other verb is forwarded to the standalone host as `host <generated-manifest> -- <argv>` with the exit code passed through. The triage split survives, relocated and reduced: the bootstrap binary carries no workflow logic, no `specify-workflow` dependency, and no embedded component.
- **Option B: grow Omnia capabilities to absorb residue items** (outbound network grants, a store mount, host exec). Each item is its own Omnia-side RFC and none removes `upgrade` or `adapter build`; rejected here as sequencing, not as direction — Option A does not preclude later absorption.

## Scope

- The standalone host binary (Omnia-side coordination) and the deletion of `specify-runtime`'s embedded host surface (`CursorBundle`, `Hooks`, `drive`, `WORKFLOW_GUEST_WASM`).
- The workflow-guest publish/pin/hydrate path, amending RFC-64's out-of-scope carve-out.
- The bootstrap `specify` CLI (Option A): residue verbs plus forward-to-host dispatch.
- The triage-widening decision for pure workflow verbs, including the latency measurement that gates it.
- Deletion of the transient-manifest assembly in favor of the RFC-63 generated manifest.

## Out of scope

- **Omnia OCI guest sources, version ranges, third-party namespaces** — unchanged (RFC-63/64 postures hold).
- **Workspace slot routing in-guest** — D-workspace's gap keeps its own future RFC.
- **Sandboxing and permission narrowing of the cursor backend** — still the phase after the migration, per RFC-61.
- **Multi-node or long-lived deployments** — the host remains one command-mode invocation per verb; RFC-55 stays deferred.

## Acceptance criteria

1. One standalone host binary, containing no Specify domain logic, runs the full workflow loop from the generated manifest: `<host> <manifest> -- plan execute` drains a plan with exit-code passthrough.
2. `workflow.wasm` is published, pinned, and hydrated like an adapter component; no component bytes are embedded in any native binary, and `rg include_bytes!` finds no guest payload.
3. The bootstrap `specify` binary depends on neither `specify-workflow` nor `specify-runtime`; every non-residue verb reaches the guest, and the operator-visible argv grammar, envelopes, and exit codes are unchanged across the seam.
4. The transient-manifest assembly is deleted; the generated manifest is the only deployment description either binary reads.
5. `make lint` and `cargo make ci` are green in both repos, and DECISIONS.md records the amendments to D-bin and D-dist.

## Risks and invariants

- **This relocates the native surface; it does not delete it.** The residue table is irreducible under current WASI/Omnia capabilities. If the review concludes the embedded host already delivers the inversion's benefits (it does: orchestration in deterministic guest Rust, prose compiled in, one execution model), rejecting this RFC is a legitimate outcome — the amendments to D-bin/D-dist are the cost being weighed.
- **A second binary is a distribution surface.** Today one archive ships one self-contained executable; this cut ships a bootstrap CLI plus a host plus a pulled component, with version skew between them as a new failure class. The workflow-guest pin and the host version must both be recorded project-side and checked at dispatch, with typed errors — never an Omnia load panic.
- **Per-verb latency is a gate, not a footnote.** Routing `plan status` through component instantiation must be measured before the triage widening lands; if it regresses the operator loop, the pure verbs stay in the bootstrap binary (they are wasm-clean either way) and only the orchestrators cross the seam.
- **Omnia stays domain-free.** The host binary gains no Specify vocabulary; backend selection is configuration, and everything Specify-shaped lives in the guests, the generated manifest, and the bootstrap CLI.
- **Sequencing.** RFC-63 (hydration, generated manifest) and RFC-64 (single-component artifact) land first; both do this RFC's groundwork, and neither depends on it.
