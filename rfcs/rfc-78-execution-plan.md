# Execution Plan — RFC-78 Leg Budget, RFC-79 Stage A, RFC-80 D8

> Status: Active — companion to [RFC-78](rfc-78-prompt-budget.md), [RFC-79](rfc-79-swarm-build.md), [RFC-80](rfc-80-synthesis-redesign.md)
>
> Purpose: sequence the accepted decisions into agent-sized work packages with repo boundaries, dependencies, and verification gates. Each package is written to be handed to a fresh agent session with only this document and the named RFC sections as context.
>
> Timeline anchor: Emery graduates from PoC to trial use in ~4 weeks. WP1–WP5 are the trial-hardening set; WP6–WP7 (swarm Stage A) may land during or after trial start without blocking it.

## Pre-flight decisions (resolve before dispatching WP2/WP5)

These close RFC open questions so implementing agents do not re-litigate them:

1. **Batch every WIT change into one break.** WP2's `input` record and WP3's build-context (the slice's bound source names, for the replay skip) ship in the same `emery:adapter` package bump. One coordinated release, not two. *(Closes RFC-78's D6 open question: seam addition, not in-guest `plan.yaml` coupling.)*
2. **Inactivity timeout defaults (WP5):** kill after **120s** with no stream-json events; absolute cap from `CURSOR_TIMEOUT_SECS` unchanged (default 600s → recommend 1800s in examples). Tune with evidence later; do not block on the perfect number.
3. **Engine-kernel session resume waits.** WP5 implements resume for the backend's own two-attempt loop only; the `omnia:model/completion` continuation token is deferred until the backend proves the semantics (RFC-78 open question, resolved as "after").
4. **D8 ships behind the eval gate, independent of the train.** It is engine-internal (no WIT), so it does not need to ride the WP2 release.

## Package map

| WP | Title | Repo(s) | Depends on | RFC anchor |
| -- | ----- | ------- | ---------- | ---------- |
| 1 | Timeout hygiene + budget assertions | `emery-adapters` | — | RFC-78 D4.1, D7 |
| 2 | WIT `input` record + path-first inputs + guidance drop | `emery` + `emery-adapters` | WP1 (assertions exist to update) | RFC-78 D1, D2 |
| 3 | Replay skip + report absorption | `emery` (WIT context) + `emery-adapters` | WP2 (same WIT break) | RFC-78 D6 |
| 4 | Synthesis evidence path-first | `emery` | — | RFC-78 D8 |
| 5 | Backend: inactivity timeout, mismatch fail-fast, session resume | `backends` | — | RFC-78 D4.2–3, D5 |
| 6 | Verify profiles activation | `omnia` + `emery` | — (design), WP2 (ships with) | RFC-60 via RFC-79 D2 |
| 7 | Swarm Stage A: sequential focused workers in omnia | `emery-adapters` + `emery` SDK | WP2, WP3, WP6; WP5 desirable | RFC-79 D1, D3, D5 |

WP1, WP4, WP5, and WP6-design have no mutual dependencies — dispatch them in parallel. WP2+WP3 form the release-train package. WP7 is the convergence of everything.

## WP1 — Timeout hygiene + budget assertions

**Repo:** `emery-adapters`. **Size:** small; one session.

- Uncomment `CURSOR_TIMEOUT_SECS = "1800"` in `examples/Makefile.toml`; align `examples/wasm/README.md` and `.env.example` wording (RFC-78 D4.1).
- Add per-leg assembled byte-budget assertions in `targets/{omnia,vectis,contracts}/tests/operations.rs`, next to the existing leg-count locks. Budgets = current measured sizes + ~10% headroom (they tighten in WP2; the point is catching silent re-bloat). The assembles are pure functions over the embedded registry — no harness changes needed (RFC-78 D7).

**Verify:** `cargo make ci` in `emery-adapters`.

## WP2 — WIT `input` record + path-first inputs + guidance drop

**Repos:** `emery` and `emery-adapters` under the committed `[patch]` sibling override. **Size:** the largest package; one session per repo or one cross-repo session. **This is the release-train package** — see Coordination below.

`emery` side (RFC-78 D1):

- `wit/emery.wit`: `variant input` payloads become `variant payload { path(string), body(string) }` (labels stay the variant cases). Fold in WP3's build-context in the same edit (see WP3).
- Mirror in `crates/project/src/seam.rs` and the SDK seam (`crates/adapter/src/seam.rs`); update the guest provider mapping (`crates/guest/src/provider.rs`) and the native provider / `mock` crate.
- `read_inputs` (`crates/slice/src/orchestrate/target.rs`): emit project-relative paths (join `BuildRequest.inputs.root`-relative names against the root, normalized project-relative) as `payload.path`. Paths must resolve in the guest's `"."` preopen **and** in the lent agent workspace — never host-absolute.
- Add the path-form renderer to `crates/adapter/src/phase.rs` (`### input: <label> → <path>` + an explicit read-before-writing instruction).

`emery-adapters` side (RFC-78 D1 + D2):

- Switch omnia generation, vectis composition + core, and contracts' three sub-flows to the path-form renderer.
- Drop `prompts/guidance.md` from the omnia generation assemble; make `build/guest.md` conditional on create mode (the scaffold prelude already detects it deterministically). Optional one-line pointer in the generation user prompt per RFC-78 D2.
- Update `tests/operations.rs` assertions (path-form inputs, no guidance in generation, WP1 budgets tightened to the new sizes).

**Verify:** `cargo make ci` both repos; `cargo check --lib -p emery --examples --target wasm32-wasip2`; operator `cargo make wasm-omnia-r9k` — spilled generation `prompt_len` **≤ ~40 KB** (RFC-78 acceptance 1) and build quality parity.

## WP3 — Replay skip + report absorption

**Repos:** `emery` (the build-context WIT addition rides WP2's break) + `emery-adapters`. **Size:** medium.

- WIT/seam: the `build` call gains the slice's bound source names (a small `build-context` record or an extra parameter — implementer's choice, documented in the WIT doc comment). Engine populates it from the plan entry in `crates/slice/src/orchestrate/target.rs`.
- Omnia: dispatch the replay leg only when a `captures` binding is present — no model call to answer `applicable: false` (RFC-78 D6).
- Omnia: assemble the `BuildReport` in-guest from the typed `PhaseAnswer`s; fold the judgmental residue (tasks.md checkboxes, findings synthesis) into the review leg's answer schema. Happy path: 5 legs → 3–4. Update the leg-count lock.

**Verify:** `cargo make ci` both repos; leg-count test reflects ≤4 without captures; eval case confirms the report content is equivalent.

## WP4 — Synthesis evidence path-first (D8)

**Repo:** `emery` only. **Size:** medium. **Gate:** live eval before merge.

- `SynthesisInputs.sources[]`: `{ source, lead, claims }` → `{ source, lead, evidence-path }` (`crates/slice/src/synthesis/wire.rs`); bump `SYNTHESIS_VERSION`.
- `crates/slice/prompts/synthesize.md` (+ playbook sections as needed): instruct the agent to read each `evidence/<source>.yaml` from the lent tree, citing claim keys exactly as they appear there. Guidance brief and baseline projections stay inline (RFC-78 D8).
- Engine suites assert the path form; regenerate goldens if the inputs envelope is golden-locked.

**Verify:** `cargo make ci`; **merge gate:** one live workflow eval (`omnia-r9k` or `orders-contracts`) with no regression in `[unknown]` counts, provenance coherence, or spec completeness (RFC-78 acceptance 8).

## WP5 — Backend: inactivity timeout, mismatch fail-fast, session resume

**Repo:** `backends` (`crates/cursor`). **Size:** medium; independent of everything else.

- **Step 0 (load-bearing):** verify the headless resume surface against the installed `cursor-agent --help` (`--resume <chatId>` under `--print`). If unavailable, fall back to strict append-only repair prompts (byte-identical prefix for provider cache) and record the finding in RFC-78.
- Inactivity timeout: kill after 120s (pre-flight decision 2) of no stream-json events, absolute cap unchanged; the parser in `model.rs` already sees every event — thread a last-activity timestamp into the timeout select.
- Mismatch fail-fast: a distinct startup hint when the cursor timeout is lower than the guest timeout (the check lives where both values are visible — the example/composition layer if the backend cannot see `GUEST_TIMEOUT_MS`).
- Session resume: capture the session/chat id from the stream; attempt 2 of the two-attempt loop resumes with the failed answer's findings + format-repair instruction only (RFC-78 D5). Session scope: one `complete` call.

**Verify:** backend unit/integration tests for the parser and timeout paths; operator `cargo make wasm-omnia-r9k` completes standards review without a flat-timeout kill.

## WP6 — Verify profiles activation (RFC-60, promoted by RFC-79 D2)

**Repos:** `omnia` (`wasi-model` verify execution — currently stubbed) + `emery` (profile policy). **Size:** large; start design in parallel, land after WP2.

- Closed profile names (`fmt build clippy test doc vet deny ci`), host-owned argv, sandbox policy, normalized findings mapping — all per RFC-60 as written.
- First consumer: WP7's convergence gate. Second: removing cargo text from today's fat legs even pre-swarm (optional interim win).

**Verify:** RFC-60 acceptance criteria 1–5; `cargo make ci` in touched repos.

## WP7 — Swarm Stage A: sequential focused workers (omnia first)

**Repos:** `emery-adapters` + `emery` (SDK scaffolding). **Size:** large; the RFC-79 Stage A milestone. **Blockers:** WP2 (path inputs), WP3 (report fold pattern), WP6 (convergence gate). WP5 desirable (routed repairs as resume).

- In-guest deterministic orchestrator: partition (writer roles first) → dispatch → converge over verify profiles → fold report (RFC-79 D1).
- Write-ownership manifests per worker; overlap rejected pre-dispatch; out-of-manifest writes are blocking findings (RFC-79 D3).
- Review team becomes host-visible specialist workers with individual budgets and timeouts (RFC-79 D5).
- Sequential dispatch only — Stage B (concurrency, agent pool) and Stage C (RFC-55 trees, distributed workers) are separate follow-on packages, gated on the Omnia concurrent-`create` question (RFC-79 open question 1 — run that experiment during WP7, not after).

**Verify:** RFC-79 acceptance 1–4 + 7 (worker prompts ≤ ~15 KB, no cargo text, verify through profiles only, review observability, ownership checks); live eval parity.

## Coordination: the release train (WP2 + WP3)

The WIT break ships once:

1. Land WP2 + WP3 engine changes on `emery` main (adapters repo building against the sibling `[patch]` throughout).
2. Tag the engine release (`vX.Y.Z` per [docs/release.md](../docs/release.md)).
3. `emery-adapters`: bump the SDK git-tag pin, land the adapter-side WP2 + WP3 changes, bump the workspace train version.
4. Bump `FIRST_PARTY_ADAPTER_TRAIN` in `project::adapter` (release-checklist step) and publish the components to GHCR.
5. Operator smoke: `cargo make wasm-omnia-r9k` against the published train — measure `prompt_len`, leg count, and completion.

Pre-1.0 hard cut: no compatibility aliases; previously published adapter versions do not serve the new host.

## Suggested dispatch order

- **Week 1:** WP1, WP5, WP4 in parallel (three independent sessions/agents). Start WP6 design and the Omnia concurrent-`create` experiment.
- **Weeks 1–2:** WP2, then WP3 on the same branch pair; cut the train release.
- **Weeks 2–4:** WP6 implementation; WP7 Stage A once WP6 lands. Trial use proceeds on the WP1–WP5 posture regardless of WP7's exact landing date.

## Per-package agent handoff notes

Give each implementing agent: this document's package section, the named RFC decision sections, and the repo's `AGENTS.md`. Standing rules that apply to every package:

- `cargo make ci` before commit in each touched repo (nextest, not bare cargo test; `-Dwarnings`).
- Integration-first: no new `src` unit tests where crate-level tests reach the behavior; adapter behavior asserts in `targets/<name>/tests/`.
- Never hand-edit `.emery/` artifacts in sandboxes; live rungs (`wasm-*`, `eval`) are operator-invoked, not CI.
- When a symbol is removed or renamed, `rg` it across Rust **and** prose in both repos in the same change.
