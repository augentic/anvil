# Run: `target-shape` — **pass**

## Context

- **Scenario:** `target-shape`
- **Operator:** Cursor agent (Composer)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0`
- **Sandbox:**
  - `acceptance/.sandbox/target-shape-intent/`
  - `acceptance/.sandbox/target-shape-doc/`

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `spec-reflects-shape-idioms` | needs-human | Both specs carry provenance, handler-observable requirements, and scenarios with Config preconditions. Neither persisted spec includes the dedicated **Error conditions** table from `acceptance/fixtures/targets/omnia/expected/shape-evidence.md`; errors appear in scenarios + `design.md` Error mapping. Operator should confirm whether table-in-spec is mandatory. |
| `design-reflects-shape-idioms` | pass | |
| `intent-and-doc-fixtures-agree` | pass | |

**Negative expectations:** held (manual-by-design posture unchanged).

## Deviations

- `specify init omnia@v1` failed (`adapter-git-failed: Remote branch v1 not found`); both fixtures used local omnia adapter path.
- Symlinked `intent` and `documentation` source adapters into each sandbox.
- Execute stopped at refine (scenario `stages` do not include build/merge); refine driven via CLI, not `/spec:execute`.

## Notes

- Core shape injection into `design.md` demonstrated on intent and documentation fixtures with cross-fixture structural parity.
- `spec-reflects-shape-idioms` needs operator sign-off on Error conditions table placement.

## Evidence

- **Reproduce:** `scripts/snapshot.sh acceptance/.sandbox/target-shape-intent` (and doc twin)
- **Retained at:** `acceptance/.sandbox/target-shape-{intent,doc}/`
- **Key paths:** `.specify/slices/greeting/` (`design.md`, `specs/greeting/spec.md`, `evidence/{intent,brief}.yaml`)
