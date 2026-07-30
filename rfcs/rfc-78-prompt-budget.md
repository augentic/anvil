# Judgment Prompt Budget — Shrink Lent-Workspace Build Legs

> Status: Draft — implementation not started
>
> Owns: how engine-assembled slice inputs and adapter system prompts combine into the spilled `cursor-agent` prompt for judgment legs that already lend the working tree.
>
> Motivated by: operator-invoked `cargo make wasm-omnia-r9k` (2026-07-30) over the real component seam (`typescript` → `omnia`, slice `at-r9k-position-adapter`).
>
> Spans: `augentic/emery` (engine `read_inputs` / seam `Input` bodies), `augentic/emery-adapters` (omnia generation assemble), `augentic/backends` (`omnia-cursor` prompt spill + `CURSOR_TIMEOUT_SECS`).
>
> Related: [RFC-18](future/rfc-18-slm.md) (cheaper generation backend — orthogonal; this RFC shrinks the frontier-model prompt itself), [RFC-60](future/rfc-60-verify-profiles.md) (host-owned verify — would remove cargo loops from agent prompt text once activated).

## Intent

Cut the bytes and wall-clock of Omnia (and similarly shaped) target **build** judgment legs without weakening MCP-lazy references or the verify-repair contract. The sharpest observed waste is **duplicating lent-tree content into the prompt** and **inlining synthesis guidance on a build leg that already consumes refined artifacts**.

## Evidence (wasm-omnia-r9k, 2026-07-30)

Two sandboxed runs of `make wasm-omnia-r9k` (sibling `emery` binary + locally built adapter components). Logging: `RUST_LOG` includes `omnia_cursor=debug`. Times below are NZST wall clock from artifact mtimes and terminal timestamps (UTC = NZST − 12h). Journal timestamps for agent legs are not reliable wall clocks — several events share one stamp at orchestration bookends.

### Outcome

The second run reached crate generation and entered the standards-review leg, then failed:

```text
plan-execute-stopped: … stop build-failed (at-r9k-position-adapter):
  … cursor-agent timed out after 600s
wasi:cli/run exited guest=emery code=2
```

`GUEST_TIMEOUT_MS` defaults to one hour in the example Makefile; the **cursor** backend default remains **600s**. Review spent that budget on specialist `Task` subagents (Security / Quality / Correctness on `claude-sonnet-5-thinking-high` under a `cursor-grok-4.5-high-fast` lead) and never returned a phase answer before the host killed the agent.

### Phase wall clock (second run)

| Phase | Approx NZST | Notes |
| ----- | ----------- | ----- |
| Plan author (surveys + reconcile) | ~19:22–19:25 | Fast |
| Extract (intent + typescript) | ~19:26–19:27 | TypeScript evidence ~1–2 min |
| Synthesis | ~19:27–19:38 | ~11 min; eight `slice.synthesis.unknown` journal events (REQ-002…REQ-009) |
| Omnia prepare + scaffold | ~19:38–19:40 | Exemplar clone + deterministic prelude |
| Omnia generation | ~19:40–19:46 | Create-mode crate + tests + guest; verify-repair in-agent |
| Omnia standards review | ~19:47–19:57 | Timed out at 600s mid-antagonist after specialists reported |

An earlier attempt the same day spent ~54 minutes in synthesis alone before generation; synthesis cost is highly model/variance-dependent and is **out of scope** for the prompt-budget cuts below except as motivation to keep later legs shorter.

### Generation `prompt_len=64110`

`omnia-cursor` logs `prompt_len` on the spilled `.cursor/omnia-prompt-*.txt` after prepending the MCP hint and rendering `Request` (`system` + `user` + schema instruction). For the first run's generation leg that measured **64110** bytes. Composition:

| Piece | ~Bytes | Source |
| ----- | ------ | ------ |
| System assemble | ~43 200 | `build.md` + **`guidance.md`** + `crate.md` + `test.md` + `guest.md` (`targets/omnia/src/operations.rs`) |
| User (instructions + scaffold + **inlined artifacts**) | ~20 000 | Engine `read_inputs` bodies via `phase::render_inputs` |
| Schema instruction | ~900 | `PHASE_ANSWER_SCHEMA` |
| MCP hint | ~200 | `omnia-cursor` `mcp_hint` |

Second-run slice artifact bodies (inlined into generation user text):

| Artifact | Bytes |
| -------- | ----- |
| `proposal.md` | 963 |
| `design.md` | 6 210 |
| `tasks.md` | 1 430 |
| `specs/…/spec.md` | 8 445 |
| **Sum** | **~17 KB** (+ section headers) |

`prose/references/` (~210 KB on disk for omnia) stayed MCP-lazy and did **not** appear in `prompt_len` — that path is already correct.

The review leg's system assemble (`build.md` + `review.md`) is ~20 KB; a spilled review prompt observed mid-run was ~22 KB. Review cost was dominated by **nested specialist agents and remediation**, not by a 64 KB system prompt — but generation still paid the large assemble before review began.

### What is *not* the bottleneck

- WASM host dispatch and MCP `POST /mcp/target/omnia` (200s); occasional `GET` → 405 SSE noise noise is irrelevant to size.
- Exemplar prepare (~1 minute).
- Schema / MCP-hint wrappers (&lt;1.5 KB).

## Current shape (why the bytes land)

1. **Engine always loads artifact bodies** into seam `Input`s for every target build (`crates/slice/src/orchestrate/target.rs` `read_inputs`), regardless of `lend_workspace(true)`.
2. **Omnia generation** joins five prose documents into one system channel (so verify-repair can re-enter crate/test/guest writers) and appends `render_inputs(inputs)` to the user message — duplicating files already present under `.emery/slices/<slice>/` in the lent tree.
3. **`guidance.md` is synthesis-facing** (returned by `guidance`, consumed at refine). Build already assumes those idioms live in `design.md` / specs; generation still re-inlines the full guidance document as a “refresher” (~10 KB).
4. **Cursor timeout is independent of guest timeout.** Examples raise `GUEST_TIMEOUT_MS` to 1h but leave `CURSOR_TIMEOUT_SECS` at the backend default (600) unless the operator sets it — review teams can exceed that even when the guest would still be allowed to run.

## Proposal

### D1 — Path-first inputs on lent-workspace legs (engine + adapters)

When the model request lends the workspace, judgment user prompts should carry **paths and labels**, not full artifact bodies:

- Engine: either stop stuffing full bodies into `Input` for lend-workspace builds, or add a parallel path-manifest the adapter can render; prefer one seam shape.
- Adapters: `phase::render_inputs` (or a sibling) renders `### input: proposal → .emery/slices/…/proposal.md` (and the same for design / tasks / specs), with an explicit instruction to read those paths from the lent tree before writing code.

Bodies remain available for backends that do **not** lend a tree (future remote / pathless nodes — see [RFC-55](future/rfc-55-working-tree.md)). Gate the path-first form on `lend_workspace` / `local-path` presence so non-lent deployments keep inlined bodies.

**Expected save on this slice:** ~15–20 KB on every generation (and any other leg that currently dumps `render_inputs`).

### D2 — Drop `guidance.md` from the omnia generation assemble (adapters)

Remove `prompts/guidance.md` from the generation `assemble([...])` list. Keep it on the `guidance` operation and on refine. Update `targets/omnia/tests/operations.rs` assertions that require the “guidance refresher” in generation system text. Optionally leave a one-line pointer in the generation user prompt: “idioms were folded at refine; re-read `design.md` / specs, fetch `references/guardrails.md` via MCP if needed.”

**Expected save:** ~10 KB on generation system.

### D3 — Further thin writer / build preamble toward MCP (adapters, follow-on)

`build.md` / `crate.md` / `test.md` / `guest.md` are already under CONTRIBUTING line-count soft caps but still ~43 KB combined. Move tables and repair recipes that duplicate `references/*` into MCP-only docs; keep phase prompts as orchestrators. Do not split crate/test/guest into separate model calls in the first cut — that trades prompt size for round-trips and weakens the shared verify-repair channel (today’s intentional design).

### D4 — Align example / eval timeouts with review-team legs (examples + docs)

Document and default `CURSOR_TIMEOUT_SECS` for `wasm-omnia-r9k` / `eval omnia-r9k` to a value that covers standards review with specialist subagents (the Makefile already comments `1800`), or fail fast with a distinct hint when the backend timeout is lower than `GUEST_TIMEOUT_MS`. Treat “cursor timed out after 600s” during review as an operator-config defect until D4 lands — not as a silent guest bug.

### Non-goals

- Training or swapping the model backend ([RFC-18](future/rfc-18-slm.md)).
- Host-owned `verify` profiles ([RFC-60](future/rfc-60-verify-profiles.md)) — complementary later; this RFC does not move cargo out of the agent loop.
- Changing the MCP references shelf layout or the `REFERENCES_POINTER` contract.
- Shrinking synthesis prompts (separate investigation; journal `slice.synthesis.unknown` volume on this slice is noted only as context).

## Ownership

| Decision | Repo |
| -------- | ---- |
| D1 seam / `read_inputs` / lend-aware render | `augentic/emery` (+ adapter call sites) |
| D2 / D3 omnia assemble + prose | `augentic/emery-adapters` |
| D4 cursor timeout defaults / docs in examples | `augentic/emery-adapters` examples; timeout semantics owned by `augentic/backends` |

## Acceptance criteria

1. On a lend-workspace omnia generation call, spilled `prompt_len` for a slice the size of `at-r9k-position-adapter` is **≤ ~40 KB** (generation system without guidance + path-only inputs + schema/MCP wrappers), measured the same way `omnia-cursor` logs `prompt_len` today.
2. Generation still instructs the agent to read proposal / design / tasks / specs from the lent tree and still runs the verify-repair loop in-agent; `targets/omnia/tests/operations.rs` asserts path-form inputs (not inlined bodies) when the harness lends a workspace.
3. `guidance.md` is absent from the generation system assemble; refine still receives full guidance via the `guidance` operation.
4. `cargo make wasm-omnia-r9k` docs (and/or Makefile env) set or document a cursor timeout that can finish standards review, or surface a clear mismatch with `GUEST_TIMEOUT_MS`.
5. `cargo make ci` green in both repos for the touched suites; no new unit tests where crate-level integration already reaches the assemble.

## Risks and invariants

- **Path-only inputs require a real `local-path`.** Never ship path-only prompts to a backend that cannot read the tree.
- **Do not inline references to “save” MCP round-trips.** The 64 KB problem was system + artifact duplication; the references shelf must stay lazy.
- **Verify-repair stays one generation leg** until a deliberate multi-leg redesign; D3 must not regress that without a separate RFC.
- **Timeouts are part of the operator contract.** Raising limits without shrinking prompts only masks cost; D1–D2 are the primary levers, D4 is hygiene.

## Open questions

- Should path-first inputs be an engine-wide rule for every lend-workspace judgment (sources included), or an opt-in on the target `build` path only?
- Does the WIT / seam `Input` type need a `path` variant, or is “empty body + label convention” enough for one release?
- After D1–D2, is a soft `prompt_len` budget asserted in omnia (or probe) integration tests to prevent silent re-bloat?

## Appendix — log anchors

- Terminal: `make wasm-omnia-r9k` under `emery-adapters`; final failure `cursor-agent timed out after 600s` during standards review (specialists completed; antagonist / remediation not finished).
- Journal (second run): `slice.build.started` for `at-r9k-position-adapter`; no `slice.build.succeeded` / `.failed` payload beyond execute stop (guest exit 2).
- Sandbox: `sandbox/wasm-omnia-r9k/project/` — crate at `crates/at_r9k_position_adapter/`, slice still `metadata.status: refined`, plan entry `in-progress`.
- First-run generation spill: `prompt_len=64110`, `schema_name=generation`, `mcp_servers=["omnia-references"]`.
