# Run: `target-shape` — **pass**

## Context

- **Scenario:** `target-shape`
- **Operator:** Cursor agent (agent-as-operator, per the single-scenario runbook)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0` (built from the `Specify.toml` `cli` source)
- **Sandbox:** `evals/.sandbox/target-shape-intent/` (intent fixture), `evals/.sandbox/target-shape-docs/` (documentation fixture)

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `spec-reflects-shape-idioms` | pass | Intent: `.specify/slices/greeting/specs/greeting/spec.md` — per-requirement `ID:` / `Sources:` / `Status:` blocks; two handler-observable requirements (personalised response + empty-name rejection); `model.requirements[].scenarios[]` populated for validation; scenarios name inputs and observable 200/`BadRequest` outcomes. Docs fixture matches shape-derived requirement prose; `Sources: brief` only. |
| `design-reflects-shape-idioms` | pass | Both fixtures: `design.md` carries all eight Omnia `shape` sections in order — Domain model (newtypes), Provider trait dependencies (`Config` on `GreetingRequest`), Handler delegation (`Handler<P>`, `type Input = Vec<u8>`, no `Utc::now()` in `from_input`), External surfaces (`GET /greeting`, Axum brace syntax), Configuration (`GREETING_DEFAULT_NAME`), Error mapping (`thiserror` + `From<GreetingError> for omnia_sdk::Error` with `code`/`description`), Validation placement table (`from_input` vs `handle`), Observability (`monotonic_counter.*` metrics). |
| `intent-and-doc-fixtures-agree` | pass | `diff` on `design.md` is empty (byte-identical). `spec.md` differs only on kernel-rendered `Sources:` (`intent` vs `brief`); requirement statements, scenario outcomes, and REQ ids align. |

**Negative expectations:** held (manual-by-design posture unchanged; two fresh sandboxes driven interactively against the real CLI).

## Deviations

- Offline init via local omnia adapter path (`specify init <framework>/adapters/targets/omnia`) instead of `omnia@v1` network fetch.
- Symlinked `intent` and `documentation` source adapters per setup prerequisites.
- Two sandbox roots (`target-shape-intent`, `target-shape-docs`) instead of one directory — scenario allows sequential or parallel fresh projects.
- Gate 1 stamped with `--actor agent`; plan lock held for the session via `specify plan lock -- <cmd>`.
- Phase work driven by following `/spec:plan` and `/spec:refine` skill choreography via CLI verbs (survey/extract/synthesize two-phase handoffs).

## Notes

- `specify slice validate` returned two non-blocking `kind: review` suggestions (imperative proposal language, SHALL/MUST phrasing) on both fixtures — judged acceptable.
- Docs source key `brief` binds `documentation:docs/greeting.md`; intent source uses `intent:value:…` degenerate N=1 binding.

## Evidence

- **Reproduce:** `scripts/snapshot.sh evals/.sandbox/target-shape-intent` and `scripts/snapshot.sh evals/.sandbox/target-shape-docs`
- **Retained at:** `evals/.sandbox/target-shape-intent/`, `evals/.sandbox/target-shape-docs/`
- **Key paths:** `plan.yaml`, `.specify/slices/greeting/` (`specs/greeting/spec.md`, `design.md`, `model.yaml`, `evidence/`), `.specify/journal.jsonl`
- **Shape comparison:** `diff -u target-shape-intent/.specify/slices/greeting/design.md target-shape-docs/.specify/slices/greeting/design.md` (no output); spec diff shows only `Sources:` lines
