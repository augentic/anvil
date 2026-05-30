# RFC-29 Implementation Review

> Reviewer assessment of [`rfc-29-fan-in-fan-out.md`](rfc-29-fan-in-fan-out.md) against the live `augentic/specify` and `augentic/specify-cli` codebases. Scope: readiness, issues, and alternative approaches. Not a decision record — input for the RFC author.
>
> **Update (2026-05-30):** The RFC author applied the recommended fixes — draft/persisted model split (D3a), D2 slice-name derivation, kernel rendering into `spec.md`, schema hygiene, D13 read-path guarantee, M2a/M2b split, and companion [`rfc-29-m1-plan.md`](rfc-29-m1-plan.md) / [`rfc-29-m2-plan.md`](rfc-29-m2-plan.md). Sections below retain the original findings for audit context.

## Verdict

RFC-29 is an unusually thorough, well-grounded RFC. The decision table, error discriminants, journal taxonomy, milestone split, and acceptance proof are concrete, and the named dependencies (RFC-25/27/28/35) have genuinely landed. The codebase foundation is real: the axis-split adapter loader (`crates/workflow/src/adapter/core.rs`), the closed `EventKind` taxonomy (`crates/workflow/src/journal.rs`), the evidence/plan/provenance schemas, the `crates/workflow/src/change/plan/` writers, and the workflow-free `source preview` (`src/runtime/commands/source/preview.rs`) it wants to share with `survey`/`extract` all exist as described.

**It is not uniformly ready.** M1 is ready to implement; M2 contains two genuine internal contradictions in the drafted schemas plus an under-specified identity model that should be fixed *before* coding; M3 is intentionally not ready (its two schemas are deferred). None of the companion `rfc-29-m{1,2,3}-plan.md` PR-breakdown files exist yet.

---

## 1. Is the RFC ready for implementation?

### Per milestone

| Milestone | Decisions | Readiness |
| --- | --- | --- |
| **M1 — Executable source ops** | D1, D9-source, D12 | **Ready.** `survey`/`extract`, the `execution` enum, survey cache events, and `journal emit` are well-specified; the `source preview` reuse plan is sound (`preview.rs` already does the adapter-resolve + brief-path + `evidence/` scaffolding to factor out). |
| **M2 — Reconciliation + synthesis + typed model** | D2, D3, D4, D10, D11, D13, D5 | **Not ready as drafted.** Two schema contradictions + one identity-model gap (see §2). |
| **M3 — Target build envelope** | D6, D9-target, D7 | **Deferred by design.** The two build schemas are "authored during implementation (M3, Wave E)," so there is nothing to review yet. |

### Cross-cutting readiness gaps

- The companion milestone plan files the RFC defers the PR-sized breakdown to (`rfc-29-m1-plan.md`, etc.) **do not exist** — only the RFC and three draft schemas are present under `rfcs/rfc-29/`. Minor for M1; material for M2 given the surface size.
- M2 alone is very large for one milestone: two judgment-plus-kernel engines, five new verbs (`plan propose`, `slice synthesize`, `slice provenance`, `slice model show`, plus the kernel), two embedded schemas, seven drift validators, and the D13 evidence change. See §3 for a suggested split.

---

## 2. Issues with what is planned

### High — the synthesis response cannot validate against the schema it `$ref`s

`synthesis-envelope.schema.json` defines the response's `model` as a direct `$ref` to `model.schema.json`:

```json
"model": { "$ref": "https://github.com/augentic/specify-cli/schemas/slice/model.schema.json" }
```

But `model.schema.json` `modelRequirement` requires the kernel-owned fields on every requirement:

```json
"required": ["id", "title", "status", "sources", "statement", "claims"]
```

The RFC is emphatic that the agent must **not** author `id`, `status`, `sources`, or `winner` (§"Synthesis envelope"; error `slice-synthesize-kernel-field-usurped`). So a conforming agent response is **guaranteed to fail** validation against the very schema the response says to validate it with — `id`/`status`/`sources` are required but forbidden.

The same problem hits top-level `generated-at` and `generator` (required in `model.schema.json`), which are clearly CLI/kernel-stamped yet aren't in the RFC's "kernel-owned" list and aren't excused from the response. Who writes them?

**Fix:** a separate **draft-model** shape for the response (kernel fields optional or `not`-allowed) vs. the persisted `model.schema.json` (kernel fields required), or validate the response *after* projection. As written the contract is self-contradictory and an implementer hits it on day one of D3.

### High — the D2 group-id / slice-name identity model is internally inconsistent

`proposal.schema.json` says the response `group-id` *is* the slice name:

> "Proposed slice name. Validated against the slice-name grammar by the kernel; the agent proposes it, the kernel does not invent it."

…and simultaneously says the same group may appear under multiple targets:

> "Exactly one target per group … The same lead group may appear under more than one target."

These can't both hold: slice names are unique across a plan. The worked example (§"Reconciliation envelope") emits `group-id: identity-api` **twice** (for `contracts@v1` and `omnia@v1`), and its `depends-on: [identity-contracts]` references a slice name that is **not any group-id in the response**. The acceptance proof (§D7) then has the kernel write slices named `identity-contracts` and `identity-service`. So the example treats group-id as a *concept* and slice names as something derived per `(group, target)` — contradicting the schema's "group-id is the proposed slice name."

The RFC never pins the rule for deriving a unique slice name from `(group-id, target)`, nor how `depends-on` (expressed in slice names) is validated against groups (expressed in group-ids). `plan-reconcile-partition` is also defined *per target*, consistent with group-id-as-concept but not group-id-as-slice-name. This is the load-bearing data flow of D2 (response → `plan.yaml.slices[]`) and needs a single coherent identity model before implementation.

### Medium — `targetRef` pattern diverges across the new schemas

Canonical plan pattern (`plan.schema.json`): `^[a-z][a-z0-9-]*@v\d+$`. `model.schema.json` and `synthesis-envelope.schema.json` match it, but `proposal.schema.json` uses a stricter pattern `^[a-z0-9]+(-[a-z0-9]+)*@v[0-9]+$`. Since the D2 response's `target` is written verbatim into `plan.yaml.slices[].target`, a value could pass one gate and fail the other. Reference the plan pattern (or a shared `$def`) everywhere.

### Medium — D13 tightens a landed artifact schema; the migration claim needs verifying

D13 makes `claim-id` unconditionally required on **every** claim kind in `evidence.schema.json` (today required only on `requirement`/`criterion`/`example`). The migration section claims "Existing Evidence … keeps validating until re-extracted." That holds only if no read path re-validates persisted evidence against the tightened schema. The D1 `extract` path validates *before write* (fine), but `slice synthesize` reads `evidence/*.yaml` — if it validates on read against the embedded `EVIDENCE_JSON_SCHEMA`, every pre-RFC-29 slice with an id-less `decision`/`section`/etc. claim breaks mid-synthesis. The RFC should confirm the read path doesn't validate, or gate the stricter rule behind a version field. This is the one breaking change to an already-shipped schema and deserves an explicit non-revalidation guarantee.

### Medium — D8's target-neutrality can be silently defeated by an empty constraint array

The D8 wall depends on `forbidden-inputs-for-requirements-reconciliation` carrying `[target, shape-brief]`, but the schema only constrains the array's *item* enum, with no `minItems`/`const`/`contains`:

```json
"forbidden-inputs-for-requirements-reconciliation": {
  "type": "array", "uniqueItems": true,
  "items": { "enum": ["target", "shape-brief"] }
}
```

An empty array validates and silently disables the D8 contract. Pin it with `const: ["target","shape-brief"]` (or `contains` + `minItems`). Similarly, the model schema's prose "agreement required when >1 claim" is not encoded as a conditional `if/then` — it's schema-enforceable and currently isn't.

### Medium (architectural) — `model.yaml` multiplies the provenance surfaces

RFC-29 adds `model.yaml` as a third provenance-bearing artifact alongside `spec.md`'s `Sources:` lines and `provenance.yaml`, then needs **seven** drift validators to keep them coherent. The design reduces risk on one axis (provenance.yaml is now *projected* from `model.yaml` claims rather than hand-authored — a real improvement over the RFC-35 status quo). But `spec.md ↔ model.yaml` drift remains a maintained, validator-enforced invariant, and `spec.md` stays both human-editable and the merge input. Three artifacts encoding overlapping requirement/provenance state is a standing complexity cost worth calling out in the RFC's trade-offs (see §3 for an alternative).

### Minor

- The response `model` `$ref` is an absolute `$id` URL; the existing adapter loader deliberately **inlines** `$defs` "so the schema compiles without a registry lookup." Embedding `synthesis-envelope` + `model` requires registering both schemas together in `specify-schema` — a small but real implementation note that isn't mentioned.
- Per-target partition leaves the "a surveyed lead that no bound target needs" case unspecified: must every surveyed lead land in ≥1 group, or may leads be legitimately dropped? `plan-reconcile-partition` ("every surveyed lead lands in exactly one group, no orphan") reads as global, but the per-target framing makes that ambiguous.

---

## 3. Better ways to achieve RFC-29's goals

1. **Split the response shape from the persisted shape (fixes High #1).** Introduce a `synthesis-draft-model` schema (kernel-owned `id`/`sources`/`status`/`winner`/`generated-at`/`generator` absent or `not`-allowed) for the envelope response; keep `model.schema.json` as the post-projection persisted contract. `slice-synthesize-kernel-field-usurped` becomes partly a schema concern, and "validate the response" stops meaning "validate against a schema the response can't satisfy."

2. **Pin one D2 identity rule (fixes High #2).** Either (a) make `group-id` a pure *concept* id and have the kernel derive slice names deterministically — e.g. `slice = group-id` for single-target groups, `group-id-<target-slug>` when a concept fans out — and express `depends-on` in those derived names; or (b) make `group-id` literally the unique slice name and forbid reuse, expressing cross-target fan-in as distinct groups with explicit `depends-on`. Update the schema description, the worked example, and the `plan-reconcile-partition` wording to match whichever is chosen.

3. **Consider rendering `spec.md`/`provenance.yaml` *from* `model.yaml` rather than cross-checking three artifacts.** If `model.yaml` were the single machine source of truth that the kernel *renders* into `spec.md` and `provenance.yaml` (one-directional), most of the seven drift validators collapse into "did the operator hand-edit spec.md after synthesis?" — a single re-sync check — instead of bidirectional parity. This preserves `spec.md` as the human/merge surface while removing model↔spec drift by construction. The current "if they disagree, model.yaml wins for code, spec.md wins for behavior" rule is exactly the dual-source-of-truth the rest of the framework avoids. At minimum, document why three artifacts were chosen over render-from-model.

4. **Sequence M1 now and use it to de-risk the envelope/kernel pattern before committing M2.** M1 is ready, unblocks RM-05's durable proof, and exercises the "agent under a CLI-owned envelope + cache + journal + validate-before-visible" pattern on the smaller `survey`/`extract` surface. Landing it first gives a working reference for the much larger D2/D3 kernels.

5. **Split M2 along its natural seam.** D2 (reconciliation → writes `plan.yaml`) and D3/D4 (synthesis → writes `model.yaml`/artifacts) are independently testable and independently useful; they share no writer. Treating them as M2a/M2b lets the D2 identity-model fix land and bake before the synthesis kernel depends on its output shape.

6. **Author the companion `rfc-29-m1-plan.md` before starting**, and fold the two High issues into `rfc-29-m2-plan.md` as preconditions, so the schema fixes are tracked rather than discovered during coding.

---

## Suggested immediate next steps

- Fix the response/persisted model schema split and the D2 group-id/slice-name model in the RFC + draft schemas (blockers for M2, not M1).
- Unify the `targetRef` pattern and add the `const`/conditional constraints for `forbidden-inputs` and `agreement`.
- Confirm (and state in the RFC) that the evidence read path does not re-validate against the D13-tightened schema.
- Start M1 implementation in parallel — it's ready and doesn't depend on the unresolved M2 questions.

---

## Evidence reviewed

- RFC + draft schemas: `rfcs/rfc-29-fan-in-fan-out.md`, `rfcs/rfc-29/schemas/{discovery/proposal,slice/model,slice/synthesis-envelope}.schema.json`, `rfcs/roadmap.md`.
- Landed dependency: `rfcs/done/rfc-35-synthesis-determinism.md`.
- CLI (`augentic/specify-cli`): `crates/workflow/src/journal.rs`, `crates/workflow/src/adapter/{core,operation}.rs`, `crates/model/src/spec/provenance.rs`, `src/runtime/commands/source/preview.rs`, `schemas/{evidence,source,target,plan/plan,slice/provenance}.schema.json`, and the `crates/workflow/src/change/plan/` writer tree.
