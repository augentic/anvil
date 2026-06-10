# Run: `documentation-one-slice` — **pass**

## Context

- **Scenario:** `documentation-one-slice`
- **Operator:** Cursor agent (Composer)
- **CLI:** `/Users/andrewweston/github.com/augentic/specify-cli/target/release/specify` — `specify 0.2.0`
- **Sandbox:** `acceptance/.sandbox/documentation-one-slice/`

## Assertions

| Assertion | Verdict |
| --- | --- |
| `plan-exists` | pass |
| `plan-validates` | pass |
| `single-slice-from-doc` | pass |
| `sources-documentation-only` | pass |
| `execute-loop-all-done` | pass |

**Negative expectations:** held (manual-by-design posture unchanged).

## Deviations

- Used local `specify init <framework>/adapters/targets/omnia` (not `omnia@v1`).
- Symlinked `adapters/sources/documentation` (not vendored by `specify init`).
- Build: schema-valid `status: success` envelope only — full Omnia `build/crate.md` codegen and `cargo check` / `wasm32-wasip2` pre-merge gates not run.
- CLI installed via direct `cargo build --release` (`make install-cli` hung in agent shell).

## Notes

- Plan/refine/merge structural path is green; generated-output-correctness (separate release gate in `docs/contributing/acceptance.md`) was not exercised.
- Slice validation surfaced 2 suggestion-level review findings (`proposal.uses-imperative-language`, `specs.uses-normative-language`); operator may confirm prose quality.

## Evidence

- **Reproduce:** `scripts/snapshot.sh acceptance/.sandbox/documentation-one-slice`
- **Retained at:** `acceptance/.sandbox/documentation-one-slice/`
- **Key paths:** `plan.yaml`, `.specify/specs/user-profile/spec.md`, `.specify/archive/2026-06-10-user-profile-endpoint/`, `.specify/journal.jsonl`
