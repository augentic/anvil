# Judgment Leg Budget — Shrink Lent-Workspace Build Legs

> Status: Implemented (archived). D1–D3 and D6–D8 landed across `emery` / `emery-adapters`; D4 (inactivity timeout) and D5 (session-resume repairs) landed in `backends`. Residue: the D4.3 model-mismatch fail-fast and engine-kernel session resume remain follow-ons in `backends` / `emery` (pre-flight decision 3 resolved them as "after"); the swarm decomposition continues under [RFC-79](rfc-79-swarm-build.md).
>
> Owns: how engine-assembled slice inputs and adapter system prompts combine into the spilled `cursor-agent` prompt for judgment legs that already lend the working tree (target build legs, plus the synthesis leg's evidence inputs); how many host judgment legs one target build dispatches; and the timeout / session semantics of the cursor backend those legs run on.
>
> Motivated by: operator-invoked `cargo make wasm-omnia-r9k` (2026-07-30) over the real component seam (`typescript` → `omnia`, slice `at-r9k-position-adapter`).
>
> Spans: `augentic/emery` (engine `read_inputs` / seam `Input` / the adapter SDK's `phase` renderers), `augentic/emery-adapters` (omnia / vectis / contracts assembles), `augentic/backends` (`omnia-cursor` spawn model, `CURSOR_TIMEOUT_SECS`).
>
> Related: [RFC-79](rfc-79-swarm-build.md) (the swarm build this RFC is the enabling layer for — focused convergent build requests replace the fat legs), [RFC-80](rfc-80-synthesis-redesign.md) (the deeper synthesis redesign D8 defers to), [RFC-18](../future/rfc-18-slm.md) (cheaper generation backend — orthogonal; this RFC shrinks the frontier-model cost itself), [RFC-60](../rfc-87-verify-profiles.md) (host-owned verify — RFC-79 promotes it to the swarm's convergence gate), [RFC-55](rfc-55-working-tree.md) (materialized working trees — per-worker isolation and the deployment that re-activates bodies-on-the-wire).

## Intent

Cut the bytes **and** the wall-clock of Omnia (and similarly shaped) target **build** judgment legs without weakening MCP-lazy references or the verify-repair contract. Two levers, in order of expected value:

1. **The leg / process model** (D4–D6): every judgment leg is a cold `cursor-agent` spawn with no session continuity; repairs re-spawn with the full prompt re-appended; one leg exists only to answer `applicable: false`; and the flat per-spawn timeout killed a review leg that was actively working. Prompt bytes cost seconds per leg — the observed failure and the wall-clock table are dominated by spawn count, repair re-spawns, and timeout semantics.
2. **Prompt bytes** (D1–D3, D8): the sharpest byte waste is **duplicating lent-tree content into the prompt** — slice artifacts on the build legs, evidence documents on the synthesis leg — and **inlining synthesis guidance on a build leg that already consumes refined artifacts**.

## Evidence (wasm-omnia-r9k, 2026-07-30)

Two sandboxed runs of `make wasm-omnia-r9k` (sibling `emery` binary + locally built adapter components). Logging: `RUST_LOG` includes `omnia_cursor=debug`. Times below are NZST wall clock from artifact mtimes and terminal timestamps (UTC = NZST − 12h). Journal timestamps for agent legs are not reliable wall clocks — several events share one stamp at orchestration bookends.

### Outcome

The second run reached crate generation and entered the standards-review leg, then failed:

```text
plan-execute-stopped: … stop build-failed (at-r9k-position-adapter):
  … cursor-agent timed out after 600s
wasi:cli/run exited guest=emery code=2
```

The **cursor** backend default remains **600s** (`examples/Makefile.toml` carries `CURSOR_TIMEOUT_SECS = "1800"` commented out). Review spent that budget on specialist `Task` subagents (Security / Quality / Correctness on `claude-sonnet-5-thinking-high` under a `cursor-grok-4.5-high-fast` lead) and never returned a phase answer before the host killed the agent — the timeout is flat wall-clock per spawn, with no credit for a stream that is still making progress.

### Phase wall clock (second run)

| Phase | Approx NZST | Notes |
| ----- | ----------- | ----- |
| Plan author (surveys + reconcile) | ~19:22–19:25 | Fast |
| Extract (intent + typescript) | ~19:26–19:27 | TypeScript evidence ~1–2 min |
| Synthesis | ~19:27–19:38 | ~11 min |
| Omnia prepare + scaffold | ~19:38–19:40 | Exemplar clone + deterministic prelude |
| Omnia generation | ~19:40–19:46 | Create-mode crate + tests + guest; verify-repair in-agent |
| Omnia standards review | ~19:47–19:57 | Timed out at 600s mid-antagonist after specialists reported |

An earlier attempt the same day spent ~54 minutes in synthesis alone before generation; synthesis cost is highly model/variance-dependent. D8 takes the mechanical cut this RFC can make there (path-first evidence inputs); the deeper synthesis redesign stays deferred (see Non-goals).

Correction to earlier drafts: the eight `slice.synthesis.unknown` journal events (REQ-002…REQ-009) are **not** synthesis rounds. They are one event per `[unknown]`-tagged requirement, emitted once by the post-synthesis validate sweep (`crates/slice/src/validate.rs`) — a spec-gap quality signal, not model cost. Synthesis ran as one judgment leg.

### Leg count and spawn model

One omnia build dispatches **five host judgment legs** — preparation, generation, review, replay, report — locked by `targets/omnia/tests/operations.rs`, plus an optional sixth when the deterministic report gate re-prompts. Each leg is a **cold `cursor-agent` spawn**: the backend (`backends/crates/cursor/src/model.rs`) spills the prompt to a file, spawns `cursor-agent --print`, and discards the session. There is no resume, so no provider-side prompt-cache reuse across legs, and the shared `build.md` preamble (14.5 KB) is re-sent on all five.

Repairs compound across two layers, each re-spawning with the **full original prompt plus the failed answer appended**:

- Backend: up to 2 attempts per completion (`take_answer` → `append_repair`).
- Engine / SDK kernel: `MAX_REPAIRS = 2` (up to 3 `create`s) on `repaired` legs (synthesis, propose, source extract/survey). Target build phase legs are one-shot `judgment` calls, so they pay only the backend layer.

Worst case for a `repaired` leg: six full agent spawns, the later ones carrying ~1–3× the original payload. A schema-shape miss on a 10-minute generation leg doubles its cost today.

### Generation `prompt_len=64110`

`omnia-cursor` logs `prompt_len` on the spilled `.cursor/omnia-prompt-*.txt` after prepending the MCP hint and rendering `Request` (`system` + `user` + schema instruction). For the first run's generation leg that measured **64110** bytes. Composition:

| Piece | ~Bytes | Source |
| ----- | ------ | ------ |
| System assemble | ~43 200 | `build.md` + **`guidance.md`** + `crate.md` + `test.md` + `guest.md` (`targets/omnia/src/operations.rs`) |
| User (instructions + scaffold + **inlined artifacts**) | ~20 000 | Engine `read_inputs` bodies via `phase::render_inputs` |
| Schema instruction | ~900 | `PHASE_ANSWER_SCHEMA` |
| MCP hint | ~200 | `omnia-cursor` `mcp_hint` |

Second-run slice artifact bodies (inlined into generation user text):

| Artifact | Bytes |
| -------- | ----- |
| `proposal.md` | 963 |
| `design.md` | 6 210 |
| `tasks.md` | 1 430 |
| `specs/…/spec.md` | 8 445 |
| **Sum** | **~17 KB** (+ section headers) |

`prose/references/` (~210 KB on disk for omnia) stayed MCP-lazy and did **not** appear in `prompt_len` — that path is already correct.

The review leg's system assemble (`build.md` + `review.md`) is ~20 KB; a spilled review prompt observed mid-run was ~22 KB. Review cost was dominated by **nested specialist agents and remediation**, not by a 64 KB system prompt — but generation still paid the large assemble before review began.

The same shape repeats in the sibling targets, larger: vectis inlines `render_inputs` on **two** legs (composition and core) and its composition assemble alone is ~62 KB (`build.md` there is 31.7 KB); contracts re-inlines the same inputs block **three times**, once per format sub-flow. Whatever D1–D3 land as must land at the SDK level so all three targets inherit it.

### What is *not* the bottleneck

- WASM host dispatch and MCP `POST /mcp/target/omnia` (200s); occasional `GET` → 405 SSE noise is irrelevant to size.
- Exemplar prepare (~1 minute).
- Schema / MCP-hint wrappers (&lt;1.5 KB).
- Reference laziness, adapter-metadata caching (SHA-256 sidecar), and compile-time prose embedding — all already correct.

## Current shape (why the bytes and spawns land)

1. **Engine always loads artifact bodies** into seam `Input`s for every target build (`crates/slice/src/orchestrate/target.rs` `read_inputs`). The persisted `build/request.yaml` is path-only; `read_inputs` inflates every path to a body and **drops the paths at the seam** — `Input` (Rust `crates/project/src/seam.rs` and WIT `wit/emery.wit`) carries a bare body string with no path field.
2. **Lending is invariant and invisible at render time.** `lend_workspace(true)` is hardcoded inside `create` in both judgment kernels (`crates/adapter/src/call.rs`, `crates/project/src/judgment.rs`), *after* the adapter has assembled its prompt; the adapter `Context` carries no lend signal. A runtime "gate on lend" therefore cannot be implemented as a render-time branch today — path-first must be the default, with bodies-on-the-wire returning only when a non-lending deployment ([RFC-55](rfc-55-working-tree.md)) exists.
3. **Omnia generation** joins five prose documents into one system channel (so verify-repair can re-enter crate/test/guest writers) and appends `render_inputs(inputs)` to the user message — duplicating files already present under `.emery/slices/<slice>/` in the lent tree.
4. **`guidance.md` is synthesis-facing** (returned by `guidance`, consumed at refine). Build already assumes those idioms live in `design.md` / specs; generation still re-inlines the full guidance document as a “refresher” (~10 KB). `guest.md` (~5 KB) is likewise assembled even in update mode, which skips the guest writer.
5. **Two legs are cheaper than a spawn.** The replay leg exists to answer `applicable: false` whenever the slice has no `captures` binding — a fact knowable without a model (the plan entry's `sources[]`, readable from the lent tree in-guest). The report leg is a fifth spawn over phase outcomes the adapter already holds as typed `PhaseAnswer`s, paying the full `build.md` system plus the ~18 KB report answer schema.
6. **Cursor timeout is flat wall-clock.** Examples leave `CURSOR_TIMEOUT_SECS` at the backend default (600) unless the operator sets it. The backend already parses the stream-json event-by-event, so it has a free progress signal it does not use — review teams get killed mid-work even when the stream shows steady activity.
7. **The synthesis leg duplicates lent-tree evidence the same way the build legs duplicate artifacts.** `SynthesisInputs` embeds every bound source's claims verbatim from `evidence/<source>.yaml` — files that sit in the lent tree and that the deterministic tail reads from disk anyway — and the leg runs under `repaired`, so repairs re-embed that largest payload up to twice more.

## Proposal

### D1 — Path-first inputs on lent-workspace legs (WIT `input` record, now)

When the judgment leg lends the workspace — which today is every leg — user prompts should carry **paths and labels**, not full artifact bodies. Change the WIT `input` variant payload to an exclusive `variant payload { path(string), body(string) }` (mirrored in `crates/project/src/seam.rs` and the SDK seam), and add a path-form `phase::render_inputs` to the SDK that renders `### input: proposal → .emery/slices/<slice>/proposal.md` (and so on) with an explicit instruction to read those files from the lent tree before writing code.

Nothing blocks doing the WIT change immediately:

- **The break is sanctioned.** The package is `emery:adapter@0.1.0`; pre-1.0 a contract change is a hard cut per repo policy — no compatibility alias, no migration shim. First-party adapters are the whole ecosystem today.
- **The paths already exist and travel well.** `build_request` assembles project-relative paths under the slice tree; joined against `inputs.root` (itself under the guest's `"."` preopen) the same strings resolve for the adapter guest **and** for the spawned `cursor-agent` whose workspace is the same lent tree. `read_inputs` sends `payload.path` while every deployment lends; `payload.body` returns for [RFC-55](rfc-55-working-tree.md) pathless nodes.
- **Co-development needs no release.** The committed `[patch]` block in the adapters repo resolves the SDK from the sibling `emery` checkout, so both repos' changes build and test together before any tag exists.

The shipping cost is one coordinated release — engine tag, adapter SDK pin bump, `FIRST_PARTY_ADAPTER_TRAIN` bump, republish of the first-party components — which is the existing release-checklist process, not new machinery. In exchange, no interim convention ever exists: adapters render paths from typed inputs, never re-derive `.emery/slices/<slice>/…` from prose or point at a manifest file, and the path-form assertion in tests is against the seam type, not a string convention.

Apply the path-form renderer in **all three targets** (omnia generation; vectis composition + core; contracts' three format sub-flows) in the same change — the SDK owns the shape, the adapters just call it.

**Expected save on this slice:** ~15–20 KB on every generation (and ~3× that across a contracts build).

### D2 — Drop `guidance.md` from the omnia generation assemble; make `guest.md` mode-conditional (adapters)

Remove `prompts/guidance.md` from the generation `assemble([...])` list. Keep it on the `guidance` operation and on refine (synthesis already receives it through `seam.guidance`). Update `targets/omnia/tests/operations.rs` assertions that require the “guidance refresher” in generation system text. Optionally leave a one-line pointer in the generation user prompt: “idioms were folded at refine; re-read `design.md` / specs, fetch `references/guardrails.md` via MCP if needed.”

Additionally, detect create-vs-update mode deterministically in-guest (the scaffold prelude already walks the lent tree; the guest crate's presence is the discriminant) and include `build/guest.md` in the assemble only in create mode.

**Expected save:** ~10 KB on every generation, plus ~5 KB on update-mode generations.

### D3 — Thin `build.md` first; it is paid five times per build (adapters, follow-on)

`build.md` (14.5 KB) rides every leg's system channel — preparation, generation, review, replay, report — so ~73 KB of the same preamble crosses per build; thinning it pays a 5× multiplier that the single-leg sizes hide. Vectis is worse (its `build.md` is 31.7 KB). Move tables and repair recipes that duplicate `references/*` into MCP-only docs; keep phase prompts as orchestrators. Under the planned swarm model ([RFC-79](rfc-79-swarm-build.md)) the fixed per-request tax becomes a per-**worker** multiplier, so this thinning is a swarm precondition, not cleanup.

Do not split crate/test/guest into separate model calls **in this RFC** — an ad hoc split trades prompt size for round-trips and abandons the shared verify-repair channel without replacing it. The split is [RFC-79](rfc-79-swarm-build.md)'s job, and it arrives there together with the convergence gate that supersedes the channel.

### D4 — Timeout semantics: inactivity-based kill (backends + examples)

Two parts, in order:

1. **Hygiene (immediate):** uncomment `CURSOR_TIMEOUT_SECS = "1800"` in `examples/Makefile.toml` and document it for `wasm-omnia-r9k` / `eval omnia-r9k`. Until the rest of D4 lands, treat “cursor timed out after 600s” during review as an operator-config defect, not a silent guest bug.
2. **Inactivity timeout (backends):** the backend already parses `cursor-agent`'s stream-json events as they arrive; replace the single flat deadline with an **inactivity timeout** (kill after N seconds with no stream events) plus a generous absolute cap. A stalled agent dies fast; a review team that is still streaming specialist output survives. Raising a flat limit alone only masks cost.

### D5 — Session reuse for repair attempts (backends)

Both repair layers currently re-spawn a fresh agent with the full prompt plus the failed answer appended. `cursor-agent` supports resuming sessions: keep the session id from the first attempt and drive backend-level repairs (and, once plumbed through the model seam, engine-kernel repairs) as **resume + findings delta** instead of cold re-spawns. This keeps the provider's prompt cache warm, cuts repair payloads from ~1–3× the original prompt to the findings alone, and removes the worst-case six-spawn compounding on `repaired` legs. Scope: session lifetime is one judgment leg's repair chain — never across legs or slices (a fresh leg starts a fresh session, preserving the stateless-leg contract).

### D6 — Eliminate deterministic legs: replay skip and report absorption (adapters, + engine assist)

- **Replay skip:** whether the slice has a `captures` source binding is deterministic. Let the omnia core read the plan entry's `sources[]` from the lent tree in-guest (or receive the binding set on the build request, engine-side) and dispatch the replay leg **only when bound** — no agent spawn to answer `applicable: false`.
- **Report absorption:** the adapter already holds four typed `PhaseAnswer`s and `gate_report` re-checks declared outputs deterministically. Fold the judgmental residue of the report leg (marking `tasks.md` checkboxes, findings synthesis) into the review leg's answer schema, and assemble the `BuildReport` in-guest from the typed outcomes. Happy path drops from 5 host spawns to 3–4.

Both changes keep the WIT contract untouched; the report answer schema move is adapter-internal.

### D7 — Prompt-budget regression assertions (adapters)

The assembles are pure functions over the embedded registry, so byte budgets are nearly free to lock: assert per-leg assembled sizes (with headroom) in each adapter's `tests/operations.rs`, next to the existing leg-count lock. This is the guard that would have caught `guidance.md` creeping into the generation assemble, and it answers this RFC's earlier open question in the affirmative.

### D8 — Path-first evidence on the synthesis leg (engine)

The same D1 lever, applied to the worst-wall-clock phase, scoped to the one mechanical piece that carries no redesign risk. Today `SynthesisInputs` (`crates/slice/src/synthesis/wire.rs`) embeds every bound source's `claims[]` **verbatim** from the already-parsed `evidence/<source>.yaml` — full claim bodies pretty-printed into the user prompt as JSON — while the same files sit in the lent tree at `.emery/slices/<slice>/evidence/<source>.yaml`, and the deterministic tail (the `Kernel`'s `evidence_claims`, authority resolution, provenance projection) already reads them from disk independently of the prompt.

Change the `sources[]` entry shape from `{ source, lead, claims }` to `{ source, lead, evidence-path }` (bump `SYNTHESIS_VERSION`; the envelope is engine-internal — no WIT change) and have `synthesize.md` instruct the agent to read each evidence document from the lent tree before reconciling, citing claim keys exactly as they appear there. Orphan claim references stay caught downstream by the existing validate sweep; under `repaired`, a repair prompt no longer re-embeds the claim dump, which is the leg's largest payload.

Keep inline: the guidance brief (it comes from the adapter component, not the tree), and the baseline `Surface` / `DomainDetail` / `Decision` projections (small computed facts, not verbatim file dumps).

**Risk posture:** synthesis authors the product artifacts, so this is the one decision here that can move output quality, not just cost. Gate it on the live eval rung — a workflow case (`omnia-r9k` / `orders-contracts`) must produce equivalent-quality `spec.md` / `model.yaml` (no new `[unknown]` / orphan-provenance regressions) before it ships.

**Expected save:** the bulk of the synthesis user payload on every refine, ×1–3 under repair.

### Non-goals

- Training or swapping the model backend ([RFC-18](../future/rfc-18-slm.md)).
- Host-owned `verify` profiles ([RFC-60](../rfc-87-verify-profiles.md)) — complementary later; this RFC does not move cargo out of the agent loop.
- Changing the MCP references shelf layout or the `REFERENCES_POINTER` contract.
- Splitting the generation leg's shared verify-repair channel (see D3).
- **The swarm build itself — owned by [RFC-79](rfc-79-swarm-build.md).** Decomposing the fat legs into focused convergent requests, the verify convergence gate (promoting [RFC-60](../rfc-87-verify-profiles.md)), backend support for concurrent completions and per-worker workspace policy, and agent-pool lifecycle management all live there. This RFC is its enabling layer: small requests only work when the per-request fixed overhead (D1–D3), timeout semantics (D4), and session model (D5) are already right.
- **The deeper synthesis redesign — owned by [RFC-80](rfc-80-synthesis-redesign.md).** D8 takes the mechanical evidence-inlining cut; the structural questions move there: making the ~50 KB embedded synthesis playbook lazy (engine judgment legs carry no MCP grants — `crates/project/src/judgment.rs` — so laziness needs an engine-side references shelf and grant plumbing, not a prompt edit); moving synthesized artifacts off the answer channel (today artifact bodies ride the schema-gated JSON answer and are persisted by the tail — writing to the lent tree instead would rework the validate-before-visible contract); and parallelising the serial survey / extract fan-outs (`crates/slice/src/orchestrate/refine.rs`, `crates/change/src/orchestrate/survey.rs`) over RFC-79's concurrency substrate.

## Ownership

| Decision | Repo |
| -------- | ---- |
| D1 WIT `input` record + `read_inputs` + path-form renderer + adapter call sites | `augentic/emery` (`wit/emery.wit`, seam types, `crates/adapter/src/phase.rs`) + `augentic/emery-adapters`; ships on one coordinated release (engine tag + SDK pin + adapter-train bump) |
| D2 / D3 omnia (and vectis / contracts) assemble + prose | `augentic/emery-adapters` |
| D4 timeout hygiene / docs in examples | `augentic/emery-adapters` examples; inactivity + mismatch semantics owned by `augentic/backends` |
| D5 session resume for repairs | `augentic/backends` (engine-kernel plumbing later in `augentic/emery`) |
| D6 replay skip + report absorption | `augentic/emery-adapters` (optional engine assist for the binding set) |
| D7 budget assertions | `augentic/emery-adapters` |
| D8 path-first synthesis evidence | `augentic/emery` (`crates/slice` wire + prompts), eval-gated in `augentic/emery-adapters` |

## Acceptance criteria

1. On a lend-workspace omnia generation call, spilled `prompt_len` for a slice the size of `at-r9k-position-adapter` is **≤ ~40 KB** (generation system without guidance + path-form inputs + schema/MCP wrappers), measured the same way `omnia-cursor` logs `prompt_len` today.
2. Generation still instructs the agent to read proposal / design / tasks / specs from the lent tree and still runs the verify-repair loop in-agent; `targets/omnia/tests/operations.rs` asserts path-form inputs (not inlined bodies), and vectis / contracts assert the same on their inlining legs.
3. `guidance.md` is absent from the generation system assemble; refine still receives full guidance via the `guidance` operation; update-mode generations omit `guest.md`.
4. The cursor backend kills on inactivity rather than flat wall-clock (or, until that lands, `wasm-omnia-r9k` docs / Makefile set a cursor timeout that can finish standards review).
5. A slice with no `captures` binding completes an omnia build in **≤ 4 host judgment legs** with no replay spawn; the build report is assembled from typed phase outcomes.
6. Backend repair attempts resume the leg's session and send only the failed answer + findings, not the full original prompt.
7. Per-leg assemble byte budgets are asserted in each first-party target's `tests/operations.rs`.
8. Synthesis user prompts carry `evidence-path` entries, not inlined `claims[]` (`SYNTHESIS_VERSION` bumped; engine suites assert the path form), and one live workflow eval case (`omnia-r9k` or `orders-contracts`) shows no synthesis-quality regression (no new `[unknown]` tags or orphan-provenance findings) before D8 merges.
9. `cargo make ci` green in both repos for the touched suites; no new unit tests where crate-level integration already reaches the assemble.

## Risks and invariants

- **Path-only inputs require a real `local-path`.** Lending is invariant today (both kernels hardcode `lend_workspace(true)`), so path-first is safe as the default — but the exclusive `payload.body` case is the contract that keeps non-lent deployments ([RFC-55](rfc-55-working-tree.md)) expressible. Never ship path-only prompts to a backend that cannot read the tree.
- **Adapters must not hardcode the engine's slice layout.** Paths arrive typed on the seam `input` record; adapters render them verbatim and never re-derive `.emery/slices/<slice>/…` from prose conventions. Paths crossing the seam stay project-relative — a host-absolute path is meaningless in the guest's `"."` preopen and to the lent agent.
- **Do not inline references to “save” MCP round-trips.** The 64 KB problem was system + artifact duplication; the references shelf must stay lazy.
- **Verify-repair stays one generation leg — transitionally.** The fat leg is today's only shared verify-repair channel; D3 must not weaken it ad hoc. Its deliberate replacement is [RFC-79](rfc-79-swarm-build.md)'s convergence gate — until that lands, the channel is load-bearing.
- **Session reuse never crosses legs.** D5 resumes only within one judgment leg's repair chain; every leg still starts a fresh session, preserving the stateless-leg contract and slice isolation.
- **Timeouts are part of the operator contract.** Inactivity-based semantics make the contract about progress, not about guessing a wall-clock number; D1–D2 and D5–D6 remain the primary cost levers, D4 is correctness.

## Open questions

- Should the engine judgment kernel (`crates/project/src/judgment.rs`) adopt D5 session-resume for its `repaired` loop in the same change as the backend, or after the backend proves the semantics on target legs?
- For D6's replay skip: read `plan.yaml` in-guest (no seam change, couples the adapter to the plan file shape) or carry the binding set on the build request (seam addition)?
- What inactivity window default (D4) balances slow-model thinking pauses against genuinely hung agents?
- For D8: should the guidance brief also move path-first (persist it under the slice tree at refine and reference it), or is a component-sourced ~10 KB inline brief below the threshold worth churning?

## Appendix — log anchors

- Terminal: `make wasm-omnia-r9k` under `emery-adapters`; final failure `cursor-agent timed out after 600s` during standards review (specialists completed; antagonist / remediation not finished).
- Journal (second run): `slice.build.started` for `at-r9k-position-adapter`; no `slice.build.succeeded` / `.failed` payload beyond execute stop (guest exit 2).
- Sandbox: `sandbox/wasm-omnia-r9k/project/` — crate at `crates/at_r9k_position_adapter/`, slice still `metadata.status: refined`, plan entry `in-progress`.
- First-run generation spill: `prompt_len=64110`, `schema_name=generation`, `mcp_servers=["omnia-references"]`.
