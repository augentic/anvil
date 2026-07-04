# RFC-61: The In-Place Migration — Specify and its Adapters as Omnia Guests

> Status: Draft · Supersedes: the S1–S4 staging (RFC-51–54, RFC-56–59, and the implementation-sequence note — archived under [archive/](archive/README.md)) · Defers: [RFC-55](future/rfc-55-working-tree.md) (distributed working trees), [RFC-60](future/rfc-60-verify-profiles.md) (verify profiles) · Owns: the end-to-end migration plan

## Abstract

Specify migrates **in place** from prose plus a Rust-native `specify` binary to a family of `wasm32-wasip2` components with the prose compiled in, hosted by the Omnia runtime. Control inverts: instead of a Cursor agent reading skill markdown and shelling out to `specify`, guest Rust code calls the `wasi-model` host (`omnia:model/completion.create`) and the bound cursor backend spawns `cursor-agent` against the mounted working tree; reference documents reach the spawned agent through an embedded MCP server each adapter guest serves over `wasi:http`. The specify-adapters ship as a collection of wasm components co-loaded by one Omnia deployment.

The earlier architecture RFCs (51–59) were drafted before the Omnia refactoring; much of what they specified now exists in a different, implemented shape. This RFC replaces their staging with a plan grounded in three anchors only: the migration intent above, what Omnia and the backends implement today, and [`wit/specify.wit`](../wit/specify.wit) as the adapter contract. Where an old RFC's mechanism was implemented, this RFC cites the implementation; where it was superseded, this RFC states the replacement; nothing here depends on unimplemented runtime capability except where explicitly called out as a constraint designed around.

## What the runtime already provides

The hosting side of the migration is largely built. The [`augentic/omnia`](https://github.com/augentic/omnia) and [`augentic/backends`](https://github.com/augentic/backends) repositories implement:

- **Multi-guest deployments.** One `omnia.toml` manifest declares `[[guest]]` entries (many wasm components on one `Engine` + shared `Linker`), `[[mount]]` filesystem preopens, `[[route.http]]` longest-prefix routing, and per-guest `link` allow-lists. Omnia's manifest tests already model a Specify-shaped layout (`workflow` + adapter guests with `link = ["augentic:specify/source", …]`).
- **Instance-per-call execution.** Every trigger, link dispatch, and host→guest callback instantiates a fresh instance from a pre-resolved `InstancePre` on a new `Store` and discards it.
- **Host-mediated guest-to-guest dispatch.** A guest imports an interface it does not satisfy; the manifest allow-lists it; the host polyfills the import over in-process wRPC and selects the target guest by the call's first string argument (`FirstArgSelector`) — exactly the `adapter-id`-as-data convention `specify.wit` assumes. Plain values only: resource handles cannot cross the seam.
- **The `wasi-model` host.** `omnia:model@0.1.0` exposes async `create(request) -> result<reply, error>` with chat-style messages, `format: text | json | schema(...)` (the host gate validates answers against the schema before the guest sees them), `tools` carrying MCP grants (`mcp { name, tools, url }`), and `grants` carrying `references` (a guest id for host→guest `resolve` dispatch), `workspace` (a lent `borrow<wasi:filesystem/descriptor>` the host resolves against the mount registry), and `verify` (accepted but stubbed).
- **The cursor model backend.** `omnia-cursor` implements `WasiModelCtx::complete`: it reads the host-resolved workspace `local_path`, merges MCP grants into `<workspace>/.cursor/mcp.json` (RAII guard, refcounted), spawns `cursor-agent --print --force --trust --output-format stream-json --workspace <path> [--approve-mcps] [--model <id>]`, parses the NDJSON stream, and retries once on an invalid answer. Each `create` is a fresh, context-free spawn; there is no session reuse.
- **The replay backend.** `ModelDefault` serves recorded answers from `MODEL_REPLAY_DIR`, giving deterministic CI without a live model or editor agent.
- **The MCP guest SDK.** `omnia_guest::mcp` provides the `McpServer` trait and a path-agnostic JSON-RPC/Streamable-HTTP router served through `wasi:http/incoming-handler` — the docs-server pattern.
- **The composed example.** [`backends/examples/cursor`](https://github.com/augentic/backends/tree/main/examples/cursor) runs the full inverted loop today: a single component exports both `wasi:cli/run` (builds a `Request` with an MCP grant naming its own HTTP route and a `grants.workspace` lend of its `"."` preopen, calls `create`) and `wasi:http/incoming-handler` (serves the embedded MCP docs the spawned agent fetches), hosted by a `runtime!({ mode: command, hosts: { WasiHttp: HttpDefault, WasiModel: Cursor, … } })` binary whose HTTP trigger runs in the background while the CLI command drives.

What the migration builds is therefore almost entirely **guest-side**: the adapter components, the workflow component, the prose embedding, and the Specify deployment binary. The runtime constraints that shape the design are catalogued in [Constraints designed around](#constraints-designed-around), not treated as blockers.

## The target shape

One deployment, one `runtime!`-generated host binary, N+1 guests:

- **The Specify runtime binary** — a new crate in `specify/engine`: `omnia::runtime!({ mode: command, hosts: { WasiHttp: HttpDefault, WasiModel: Cursor, WasiOtel: OtelDefault } })`. This **is** the next `specify` executable: command mode drives one CLI invocation to completion (exit code carried through) while the HTTP trigger serves MCP routes in the background. A sibling test binary binds `WasiModel: ModelDefault` (replay) over the same manifest so CI never needs a live `cursor-agent`.
- **The deployment manifest** — `omnia.toml` with one `[[guest]]` per adapter plus the workflow guest; `[[mount]]` of the operator's project directory as `"."`, writable; `[[route.http]]` prefix per adapter (`/mcp/vectis` → `vectis`, …). The workflow guest carries `link = ["augentic:specify/source", "augentic:specify/target"]`. Guests are referenced by path — the adapters repo already commits `adapter.wasm` artifacts.
- **The workflow guest** — the only guest exporting `wasi:cli/run`. It imports `source` / `target` and names the plan-bound `adapter-id` as each call's first argument, so one instance owns its own fan-out (survey over every bound source, the drained execute loop) in deterministic guest code.
- **Adapter guests** — each exports `augentic:specify/source` **or** `/target` plus `wasi:http/incoming-handler` wrapping `omnia_guest::mcp::router` over its compiled-in references. Each judgment operation assembles a prompt from its embedded brief, issues `create` with an MCP grant naming its own route URL and a `grants.workspace` lend of its `"."` preopen, and deserializes the schema-validated answer into the WIT record.

Prose ships inside the components. The adapters repo carries roughly 292 KB of briefs and 1.5 MB of references plus 258 KB of shared rules and runtime references — trivial as an `include_str!` payload next to the existing 6.5 MB vectis component. Briefs compile in as prompt bodies; references compile in as the MCP shelf the spawned agent fetches lazily, which is the inversion's point: handles cross the model boundary, not corpora.

## The contract revision (`specify.wit`)

Three changes to [`wit/specify.wit`](../wit/specify.wit) before bindings are generated, each forced by an implemented runtime behavior:

1. **No resources across the link seam.** Omnia's guest-to-guest dispatch carries plain values only, but `target.build` / `target.merge` take `working-tree { base, root: descriptor }`. Since every guest in the deployment shares the same `[[mount]]` preopens, the cross-guest signatures drop the descriptor: pass `base: revision` (plus a subpath string where needed) and let the adapter guest open its own `"."` preopen. The descriptor survives where it belongs — the adapter lending `grants.workspace` to `create`, a host call from its own instance, which works today. The unused `use wasi:filesystem` import in the `source` interface is removed.
2. **`references` becomes the MCP export.** Both adapter worlds export a `references` interface the file never defines. The implemented mechanism is MCP-over-HTTP fetched by the spawned agent, so the worlds export `wasi:http/incoming-handler` instead. A typed `references` interface returns only if a non-spawning backend (genai's host-dispatched `resolve` via `grants.references`) is ever bound; adding it later is additive.
3. **Schema-validated answers as a convention.** `survey` returns `list<lead>`, `extract` returns `evidence`, `build` / `merge` return `report`. Every judgment leg requests `format: schema(...)` so the `wasi-model` host gate rejects malformed answers before the guest sees them, and the guest deserializes straight into the WIT record. One convention, applied uniformly across all eight adapters.

Bindings are generated guest-side with `wit_bindgen::generate!` (the `omnia-guest` style) from a versioned `augentic:specify` package both repos consume. No host-side bindgen is needed: Omnia's link dispatch handles the `source` / `target` interfaces structurally.

## The migration steps

### Step 1 — Runtime binary and a walking skeleton

In `specify/engine`, add the `runtime!` host crate and an `omnia.toml`; add a trivial adapter guest (echo `survey`) and a trivial workflow guest that calls `survey("source:echo")` through the link import. This exercises every seam the migration depends on — manifest loading, link allow-lists, wRPC in-process dispatch, mount preopens, MCP routing, command mode with background HTTP — with zero Specify logic at risk. Wire the replay binary and one recorded fixture into CI here, first.

### Step 2 — First real adapter guest (the pattern-setter)

Migrate **contracts** first (smallest prose, already a `wasm32-wasip2` crate). Build the reusable adapter-guest scaffolding here, because the other seven stamp it out:

- **Prose registry via `build.rs`** — walk `briefs/` and `references/`, resolve the `references/spec-runtime` symlinks into `shared/references/runtime/` (symlinks do not survive compilation; the embed step inlines them), emit an embedded doc registry. This reuses the vectis `scaffold/templates/registry.rs` codegen precedent.
- **MCP shelf** — an `McpServer` implementation over that registry (`list_docs` / `read_doc`, `doc://` resources), served via `omnia_wasi_http::serve(mcp::router(...))` — the `examples/cursor` HTTP guest verbatim.
- **Operation template** — each `source` / `target` export: assemble prompt from embedded brief plus typed inputs, issue `create` with MCP grant + workspace lend + schema format, deserialize, then run the deterministic validate-before-visible checks **in guest Rust** after the answer lands. The old two-phase prepare/finalize handoff collapses here: what was "print envelope, exit, agent works, finalize validates" becomes one guest function whose middle is a `create` call and whose tail is the same validation code, compiled in.

This step also answers the migration's riskiest question: **sessions do not exist.** The cursor backend spawns a fresh, context-free agent per `create` with a two-attempt repair loop. Operations that today lean on editor-chat continuity must decompose into single-shot `create` calls with all state carried in the workspace files and the prompt. Prove the decomposition on contracts' build sub-flows before scaling to vectis.

### Step 3 — Roll out the remaining adapters

- **Agent-only sources** (intent, documentation, typescript, screenshots, captures): thin crates — no deterministic core, just brief-prompted `create` calls returning `list<lead>` / `evidence` through the schema gate.
- **Vectis and contracts extension tools**: the existing extension crates stop being separately-dispatched WASI tools and become plain library code inside the adapter guest — `validate` / `materialize` / `prepare` are ordinary Rust called before and after the model leg. `specify extension run` and the engine's registry-hosted Wasmtime runner become redundant.
- **Omnia target**: the largest reference shelf (roughly 700 KB across 65 files) — the strongest case for MCP-served references over prompt-pasting.
- **Workspace mechanics**: the specify-adapters Cargo workspace grows from 3 members to one crate per adapter; each adapter directory keeps its prose as the authoring source of truth, with the component as the build artifact. The `adapter.yaml` brief-path and `execution: agent` machinery becomes vestigial once nothing reads manifests for agent handoffs.

### Step 4 — The workflow guest

Port the plan/slice loop from the engine's `workflow` crate and CLI dispatch into the guest:

- **Deterministic sequencing compiles in**: `plan next`, transition legality, artifact-completion checks, journal writes, survey fan-out, the drained execute loop. `specify-workflow` never linked Wasmtime, so this is largely a retargeting exercise — swap native paths for the `"."` preopen and keep the validated formats identical.
- **Judgment stays model-driven**: lead reconciliation, synthesis, and refine-phase authoring become `create` calls with the relevant **skill** prose compiled in as system/prompt text — this is where `plugins/spec/**` markdown moves from "agent reads it" to "guest sends it".
- **Lifecycle authority** stays where it effectively is today — the same Rust validators, now inside the guest rather than behind a CLI subprocess. `.specify/` files remain the durable state, which is also what makes in-place coexistence safe.
- **Operator surface**: `specify plan …` / `specify slice …` argv maps onto the guest's `wasi:cli/run`; command mode's exit status carries through. Slash-command skills shrink to thin "invoke specify" front doors or retire; which orchestration prose survives as operator-facing skill text is decided per skill during this step.

### Step 5 — Retire the bespoke host

Cut over per operation (contracts build first, sources next, workflow last), then delete: the handoff-envelope machinery, `extension run` and the engine registry's Wasmtime host, and finally the native orchestration commands — leaving the `runtime!` binary as `specify`.

## Constraints designed around

Implemented-runtime behaviors the design accommodates rather than changes:

- **Single CLI exporter.** `[[route.cli]]` is not parsed; with multiple `wasi:cli/run` exporters startup routing fails. Convention: only the workflow guest exports the CLI world; adapters are reached exclusively through link dispatch and HTTP.
- **Compile-time backend binding.** One deployment binds one `WasiModelCtx`. Cursor for the real binary, replay for CI — two `runtime!` invocations over the same manifest. A per-call router is out of scope.
- **Guests ship by path.** OCI guest sources are parsed but rejected at load; the manifest references committed `adapter.wasm` files on disk, which suits an in-place migration.
- **MCP URLs are absolute.** The grant carries a full endpoint URL; guests read the host/port via `wasi:config` or environment rather than hardcoding, since `HTTP_ADDR` is deployment-configurable.
- **Session-less model calls.** Fresh spawn per `create`, two-attempt repair, no transcript reuse. Long adaptive operations decompose into single-shot calls with state in the working tree (Step 2 proves this).
- **Shared mounts, not materialized trees.** All guests see the same `[[mount]]` preopens, so `build` / `merge` run against the operator's live project tree exactly as the native CLI does today. Distributed, content-addressed working trees ([RFC-55](future/rfc-55-working-tree.md)) are deferred until a multi-node deployment exists; the `revision` / `changeset` types stay in the contract as their forward hook.
- **`verify` is stubbed.** The grant is accepted but unimplemented; generated-code verification keeps running through the adapters' existing native check flows until [RFC-60](future/rfc-60-verify-profiles.md) is revisited.

## Coexistence

The migration is in place because the two stacks never collide: `.specify/` artifacts, `plan.yaml`, and the journal are the only shared state, and both the native CLI and the runtime path read and write them through the same validated formats. Cutover is gated per operation, not per project, so each migrated guest earns trust on real work while everything else stays native.

## Acceptance criteria

1. The `runtime!` Specify binary hosts the workflow guest plus all eight adapter guests from one `omnia.toml`; command mode drives the operator CLI and background HTTP serves each adapter's MCP shelf.
2. `specify.wit` carries no resource types across guest-to-guest calls, and every adapter world's reference surface is the MCP export; guest bindings are generated from the published package in both repos.
3. Every adapter judgment operation runs as a schema-gated `create` against the cursor backend, with its brief compiled in and its references fetched over MCP by the spawned agent; the same operations replay deterministically in CI through `ModelDefault`.
4. The vectis and contracts extension tools run as in-guest library code; `specify extension run` is deleted.
5. The workflow guest owns plan/slice sequencing and fan-out; lifecycle validation runs in-guest over `.specify/` state; the bespoke handoff envelopes are deleted.
6. `make lint` and `cargo make ci` stay green at every step; the native and guest paths coexist on the same project state until the final cutover.

## Risks and invariants

- **Decomposition risk is front-loaded.** The plan's riskiest unknown — whether long, adaptive operations survive decomposition into session-less `create` calls — is answered in Step 2 on the smallest adapter, before the workflow port begins. Do not start Step 4 until contracts (and ideally one vectis build leg) has demonstrated the answer.
- **Prose stays authored as files.** Briefs and references remain markdown in the adapter trees; embedding is a build step. Authoring workflow, review, and `make lint` coverage of prose do not change.
- **Don't ossify the fluid.** Deterministic sequencing graduates into guest code; judgment stays behind `create`. A phase that needs the model to sequence stays model-driven.
- **The contract stays adapter- and model-agnostic.** No adapter name, taxonomy, or model id enters `augentic:specify`; the model id lives in the backend binding, never in a guest.
- **Omnia stays domain-free.** Everything Specify-specific lives in the guests, the deployment manifest, and the Specify runtime binary's backend selection — never in the Omnia runtime core.
