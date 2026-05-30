# RFC-29 M1 implementation plan — Executable source operations

> Companion to [RFC-29](rfc-29-fan-in-fan-out.md). Milestone **M1** (Wave A). Status: ready to implement.

## Goal

Ship `specrun source survey`, `specrun source extract`, source-adapter `execution` enum (D9 source side), survey cache journal events, and `specrun journal emit` (D12). M1 does not depend on M2 schema or synthesis design.

## Preconditions

None beyond landed RFC-25/27/35 dependencies cited in RFC-29.

## PR breakdown

| PR | Scope | Acceptance |
| --- | --- | --- |
| **A1 — Shared source-operation prep** | Factor adapter resolve, `briefs-dir`, sandbox preopens, `evidence/` scaffolding from `source preview`; align lead-selection spelling. | `source preview` and new runners share one helper; existing preview tests pass. |
| **A2 — `source survey`** | Workflow-integrated survey: cache fingerprint, journal (`source.survey.cache-hit/miss`), `discovery.md` merge, validate-before-visible. | Golden: survey writes schema-valid leads; re-run replaces by canonical id. |
| **A3 — `source extract`** | Workflow-integrated extract per `(source-key, lead-id, slice)`; Evidence validate-before-write. | Golden: extract persists under `.specify/slices/<slice>/evidence/`; invalid Evidence leaves slice in `refining`. |
| **A4 — D9 source `execution`** | Closed enum on `source.schema.json`; `agent-fallback` → `cache: opt-out` + `source.execution.agent-fallback`. | First-party adapters declare `executable` before M1 ships; missing enum → `adapter-execution-mode-required`. |
| **A5 — D12 journal emitter** | `specrun journal emit <event-id> [--payload]`; closed taxonomy validation. | Unknown event → `journal-emit-unknown-event`; bad payload → `journal-emit-payload-schema`. |

## Out of scope (M2+)

- `specrun plan propose`, synthesis kernel, `model.yaml`, build envelope.
- Draft/persisted model schemas (M2b).

## Verification

- Unit + integration tests per PR.
- `cargo make ci` in `specify-cli` for Rust changes.
- Optional: wire `/spec:refine` extract sub-step to `source extract` behind feature flag (follow-up if not in A3).
