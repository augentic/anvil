# RFC-29 M2 implementation plan — Reconciliation + synthesis + typed model

> Companion to [RFC-29](rfc-29-fan-in-fan-out.md). Milestones **M2a** (Wave B) and **M2b** (Waves C–D). Status: ready after RFC schema/rendering revisions.

## Preconditions (blockers — must land in RFC + schemas before coding)

- [x] **D3a draft vs persisted model** — `synthesis-draft-model.schema.json`; envelope `$ref`s draft; persist pipeline validates draft then merged model ([`rfc-29/schemas/slice/`](rfc-29/schemas/slice/)).
- [x] **D2 slice-name derivation** — `group-id` as concept id; optional `slice-name`; `depends-on` in derived slice names (§"Slice-name derivation").
- [x] **Rendering pipeline** — agent prose-only Markdown; kernel renders `ID:` / `Sources:` / `Status:` into `spec.md` (§"Rendering").
- [x] **Schema hygiene** — unified `targetRef` pattern; `forbidden-inputs-for-requirements-reconciliation: const`; `agreement` required when `claims.length ≥ 2`.
- [x] **D13 read-path guarantee** — synthesize/provenance read Evidence without re-validating against tightened schema (§"Migration").

## M2a — Wave B: Lead reconciliation (D2)

| PR | Scope | Acceptance |
| --- | --- | --- |
| **B1 — Schemas + embed** | Copy `proposal.schema.json`; embed `PROPOSAL_JSON_SCHEMA`. | Request/response validate; `targetRef` matches plan pattern. |
| **B2 — Structural floor** | Deterministic floor pre-pass (exact id, alias, cross-reference); `--dry-run` envelope. | Golden floor matches RFC worked example. |
| **B3 — `plan propose --from` kernel** | Validate response; global partition; floor preservation; slice-name derivation; `depends-on` resolution; plan writers; `plan.reconcile.*` events. | Fixture: two slices (`identity-contracts`, `identity-service`); partition/floor/orphan errors fire as specified. |

**M2a exit gate:** `specrun plan propose` end-to-end on fixture leads without synthesis.

## M2b — Wave C–D: Synthesis kernel + typed model

| PR | Scope | Acceptance |
| --- | --- | --- |
| **C1 — Schemas + embed** | Copy `model.schema.json`, `synthesis-draft-model.schema.json`, `synthesis-envelope.schema.json`; register together in `specify-schema`. | Relative `$ref`s compile; draft rejects kernel fields. |
| **C2 — Projection kernel** | RFC-27 resolver; id/sources/status/winner derivation; `provenance.yaml` projection; shared with D11. | Golden: fixed draft → byte-identical kernel output (D11 determinism gate). |
| **C3 — `slice synthesize`** | Envelope dispatch; draft validate; usurp rejection; merge; persisted validate; **render provenance into `spec.md`**. | Golden synthesis path; usurp/orphan tests; `slice-spec-provenance-stale` on hand-edited provenance lines. |
| **C4 — `slice provenance` + drift** | D11 standalone verb; seven drift checks (incl. `slice-spec-provenance-stale` replacing requirement-drift). | Re-run provenance byte-stable; drift table from RFC. |
| **C5 — D10 + `/spec:refine` wire-up** | Synthesis `execution` enum; shell refine to `slice synthesize`. | Agent path default; executable path optional. |
| **D1 — D5 confirmation** | Reject stray `outputs[]` on plan slices; singular target binding regression. | Parser tests only. |

**M2b exit gate:** `specrun slice synthesize` + validate on fixture Evidence; kernel determinism + D8 envelope-construction probes from §D7 (slice-local subset).

## Dependency

M2b requires M1 extract output and M2a plan rows (or manual `plan add` equivalent in fixtures).

## Out of scope (M3)

- Build request/report schemas and target migrations (Wave E).
- Full D7 cross-repo fixture (Wave F — may start after M2b slice-local gates pass).
