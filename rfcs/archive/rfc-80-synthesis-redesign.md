# Synthesis Redesign — Structural Cost and Shape of the Refine Phase

> Status: Superseded by [RFC-88 Concurrent Execution](../rfc-88-concurrent-execution.md) D7–D10, which merges this RFC's refine-time redesign with RFC-79's build-time swarm. Retained for the original measurement and sequencing detail.
>
> Owns: the engine-side synthesis judgment's structural shape beyond [RFC-78](rfc-78-prompt-budget.md) D8's mechanical cut: how the ~50 KB embedded playbook reaches the agent, where synthesized artifacts travel (answer channel vs lent tree), and the parallel survey / extract fan-outs at plan and refine time.
>
> Depends: [RFC-78](rfc-78-prompt-budget.md) D8 (path-first evidence — lands first, independently), [RFC-79](rfc-79-swarm-build.md) D4 (the backend concurrency substrate the fan-outs consume).
>
> Related: [RFC-55](rfc-55-working-tree.md) (distributed nodes — the same values-only constraints apply to refine-time operations), [RFC-60](../rfc-87-verify-profiles.md) (not consumed here — synthesis produces prose artifacts, not code to verify).

## Intent

Synthesis is the worst-observed wall-clock phase (11 and 54 minutes across the two `wasm-omnia-r9k` runs of 2026-07-30) and the least structurally examined: one judgment leg with a ~50 KB inlined system playbook, a user prompt that duplicates lent-tree evidence, artifacts that travel **back through the schema-gated JSON answer** instead of the lent tree, and serial fan-outs on either side of it. [RFC-78](rfc-78-prompt-budget.md) D8 fixes the evidence duplication mechanically; this RFC owns the three structural questions D8 deliberately left behind, plus the decomposition option that mirrors [RFC-79](rfc-79-swarm-build.md) at refine time.

## Current shape

1. **The playbook is pasted, not shelved.** `synthesize_system()` (`crates/slice/src/judgment/prose.rs`) joins `synthesize.md` plus seven playbook sections (~50 KB) into every synthesis system prompt. Engine judgment legs carry **no MCP grants** (`crates/project/src/judgment.rs` — "no MCP grants" is the kernel's posture), so unlike adapter references there is no lazy path today: laziness needs an engine-side references shelf and grant plumbing, not a prompt edit.
2. **Artifacts ride the answer channel.** The synthesis answer carries the full bodies of `proposal.md` / `spec.md` / `design.md` / `tasks.md` inside the schema-gated JSON; the deterministic tail (`persist_synthesized`) writes them to disk afterward. Every repair attempt therefore regenerates and re-transmits every artifact body, and the answer schema must gate prose documents rather than a compact outcome record. The build legs already use the opposite model — the agent writes the lent tree and answers with a summary — and the engine gates them post hoc (`validate-before-visible` via the validate sweep).
3. **Fan-outs are serial by design.** Survey iterates bindings one at a time (`crates/change/src/orchestrate/survey.rs` `survey_all`), extract likewise (`crates/slice/src/orchestrate/refine.rs`, "the skill's no-parallelism rule"). Leads and per-source extracts are independent; the serialization exists because the backend cannot isolate concurrent completions in one workspace — exactly what [RFC-79](rfc-79-swarm-build.md) D4 Stage B provides.
4. **Repairs hit the largest payload.** Synthesis runs under `repaired` (`MAX_REPAIRS = 2`), so the repair-growth problem (original user + failed answer + findings, re-sent per attempt) applies to the phase with the biggest user prompt and the biggest answer. RFC-78 D5's session-resume mitigates the transport cost; this RFC removes the payload itself.

## Decisions

### D1 — Engine references shelf: make the playbook lazy (engine + launcher)

Give engine judgment legs the same MCP-lazy posture adapter legs have. The engine guest already exports `wasi:http/incoming-handler` and the launcher already routes `/mcp/<axis>/<name>` per adapter (`launcher::mcp_route`); extend the route table with an engine shelf (e.g. `/mcp/engine/synthesis`) serving the embedded playbook corpus through the same `list_docs` / `read_doc` contract, and let `project::judgment::create` attach that grant. The synthesis system prompt shrinks to `synthesize.md` plus a references pointer — the same contract as `REFERENCES_POINTER` in the omnia adapter.

Keep inlined: the sections the leg *always* needs in full (candidate: `requirement-block.md`, the answer-shape contract). The split between always-inline and shelf is measured, not guessed — start from which sections the agent actually fetches when given the choice.

**Expected save:** most of ~50 KB per synthesis system prompt, ×1–3 under repair.

### D2 — Artifacts move to the lent tree; the answer becomes an outcome record (engine)

Adopt the build legs' write model for synthesis: the agent writes `proposal.md`, `specs/<domain>/spec.md`, `design.md`, `tasks.md` into a staging directory under the slice tree; the answer shrinks to a typed outcome record (artifact paths written, per-requirement provenance keys, conflict / divergence / unknown declarations). The deterministic tail then:

1. reads the staged artifacts from disk,
2. runs the existing projection kernel and validate sweep against them (the same `validate-before-visible` posture the build gate uses — staging keeps a bad answer from ever being the visible slice state),
3. promotes staging into place only on a clean gate, and
4. on a gate failure, issues the repair with **findings only** — the artifacts are already on disk; the agent edits them in place rather than regenerating and retransmitting them.

This is the structural fix for repair cost: a synthesis repair becomes an edit round, not a full regeneration. It also collapses the answer schema from "prose documents in JSON" to a compact record the host schema gate can actually enforce meaningfully.

**Sequencing note:** D2 changes the persist tail, the answer schema (`slice::answers`), and the repair semantics in one move — it is this RFC's largest item and should land behind the eval gate below, after D1.

### D3 — Parallel survey / extract fan-outs (engine, over RFC-79 Stage B)

Once the backend isolates concurrent completions ([RFC-79](rfc-79-swarm-build.md) D4 Stage B), lift the serial loops:

- `survey_all` dispatches all bound sources concurrently and merges into `discovery.md` in binding order after the joins (the merge is already order-deterministic; only the dispatch serializes today).
- Refine's extract fan-out likewise, with per-source evidence files as the natural disjoint write set (each extract owns `evidence/<source>.yaml` — the write-ownership property RFC-79 D3 formalizes, already true here for free).

Plan-time surveys are the cleanest first consumer: read-only over sources, independent by construction, and the phase the operator watches interactively.

### D4 — Decomposed synthesis (deferred decision, evaluated after D1–D3)

The RFC-79 pattern applied to refine: partition synthesis per domain (or per requirement cluster), run focused synthesis workers, and converge through the projection kernel and validate sweep as the gate. **Not committed here** — cross-domain reconciliation (conflict / divergence resolution, provenance coherence) is synthesis's core judgment, and partitioning it risks trading wall-clock for quality where quality is the product. Re-evaluate with eval evidence once D1–D3 have landed and the remaining synthesis cost is measured rather than estimated.

## Non-goals

- The mechanical evidence-inlining cut — [RFC-78](rfc-78-prompt-budget.md) D8 owns it and lands first.
- Target-build decomposition, convergence, and the concurrency substrate — [RFC-79](rfc-79-swarm-build.md).
- Verify profiles ([RFC-60](../rfc-87-verify-profiles.md)) — synthesis artifacts are validated by the engine's own kernel and sweep, not by toolchain verification.
- Changing the synthesis judgment's authority model, the `[conflict]` / `[divergence]` / `[unknown]` taxonomy, or the provenance contract — this RFC moves bytes and channels, not semantics.

## Ownership

| Decision | Repo |
| -------- | ---- |
| D1 engine references shelf + grant plumbing | `augentic/emery` (`crates/guest`, `crates/launcher`, `crates/project` judgment) |
| D2 staged artifacts + outcome-record answer | `augentic/emery` (`crates/slice` persist / answers / prompts) |
| D3 parallel fan-outs | `augentic/emery` (`crates/change`, `crates/slice`), gated on `augentic/backends` Stage B |
| D4 decomposed synthesis | decision deferred; would span `crates/slice` |

## Acceptance criteria

1. The synthesis system prompt carries `synthesize.md` plus the measured always-inline subset; the remaining playbook sections are served from the engine shelf and fetched lazily (observable in backend logs as MCP tool calls).
2. The synthesis answer is an outcome record; artifact bodies never cross the answer channel. A validate-gate failure produces a repair round that edits staged files in place, and the repair prompt carries findings only.
3. Staged artifacts are never visible as slice state before the gate passes; a failed synthesis leaves the previous slice state untouched.
4. With RFC-79 Stage B available, `plan author` over N sources dispatches surveys concurrently and `discovery.md` output is byte-identical to the serial run's.
5. Synthesis quality holds at the live eval rung: a workflow case (`omnia-r9k` / `orders-contracts`) shows no regression in `[unknown]` counts, provenance coherence, or spec completeness after each of D1 and D2 — evaluated separately, since they change different channels.
6. `cargo make ci` green; goldens (`crates/slice/answers/`) regenerated for the new outcome-record schema in the same change as D2.

## Risks and invariants

- **Quality is the product.** Synthesis authors the artifacts everything downstream consumes; every decision here ships behind the live eval gate, and D4 stays a decision — not a plan — until D1–D3 evidence exists.
- **Validate-before-visible is preserved, relocated.** D2 moves the gate from "parse the answer" to "gate the staged tree"; at no point may an ungated artifact become visible slice state.
- **The answer schema stays a real gate.** The outcome record must be strict enough that the host schema gate catches malformed answers — do not let "the artifacts are on disk now" erode the typed answer contract.
- **Fan-out determinism.** Concurrent surveys / extracts must produce byte-identical merged outputs to the serial order; concurrency is a dispatch property, never an output property.
- **No lead agent.** As in RFC-79, any decomposition (D4) is orchestrated by deterministic engine code; a model deciding synthesis partitioning re-creates the cost it was meant to remove.

## Open questions

- Which playbook sections are always-inline vs shelf? (Measure fetch patterns; the answer-shape contract is the likely inline floor.)
- Does the engine shelf serve from the engine guest's own `wasi:http` export, or from a host-native shelf in the launcher (adapters use in-guest serving; the engine guest already exports the handler — likely the former, but the launcher route table owns the decision)?
- D2 staging location: `.emery/slices/<slice>/.staging/` vs a tempdir under the slice tree — and how staging interacts with `emery slice validate` run standalone.
- Should the repair round after a D2 gate failure re-lend the same session (RFC-78 D5) so the agent retains its own authoring context? (Likely yes; it is the strongest case for session resume outside build legs.)
- Where does the fan-out concurrency cap live — engine policy, backend pool config, or per-phase?
