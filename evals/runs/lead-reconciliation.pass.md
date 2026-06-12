# Run: `lead-reconciliation` — **pass**

## Context

- **Scenario:** `lead-reconciliation`
- **Operator:** Cursor agent (Fable)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0` (built from local `../specify-cli` via `make install-cli`)
- **Sandbox:** `acceptance/.sandbox/lead-reconciliation/`

## Assertions

| Assertion | Verdict |
| --- | --- |
| `plan-exists` | pass |
| `plan-validates` | pass |
| `merged-slice-combines-sources` | pass |
| `tentative-merge-surfaced` | pass |
| `amend-overrides-merge` | pass |
| `extract-runs-per-contributing-source` | pass |

**Negative expectations:** held (manual-by-design posture unchanged).

## Deviations

- Used local `specify init <framework>/adapters/targets/omnia` (`omnia@v1` remote fetch failed: `Remote branch v1 not found in upstream origin` — same failure as prior runs).
- Symlinked `adapters/sources/{documentation,typescript}` (not vendored by `specify init`).
- Drove the plan and refine lifecycles via the CLI equivalents of `/spec:plan` / `/spec:refine` (`plan create`, `source survey`, `plan propose --from`, `slice create`, `source extract`, `slice synthesize`, `slice validate`, `slice transition`) rather than the slash commands in Cursor chat.
- `legacy-auth` extract was finalized twice: the first Evidence omitted optional claim `id`s, which the synthesis model claims require to cite `(source, id, kind)`; re-ran prepare/finalize with ids added. Both finalizes appear in the journal.

## Notes

- Sources bound: `security-design-notes` (documentation, `./design-notes/security`) and `legacy-auth` (typescript, `./vendor/legacy-auth`), both describing the same account-lockout behaviour under different lead slugs (`account-lockout` vs `user-login`).
- Propose merged the two leads into one slice on synopsis content (matching numerals: 5 failures / 15-minute window / 30-minute lock / notification email); `plan.yaml.slices[account-lockout].sources` lists both `{source, lead}` bindings.
- The uncertain merge (different slugs) is surfaced in `change.md` under `## Tentative merges` with the confirm-or-split instruction, mirroring the `cross-source-identity-revamp` fixture.
- Amend override exercised both directions at Gate 1: split via `specify plan amend account-lockout --remove-source legacy-auth` + `specify plan add user-login --sources legacy-auth=user-login` (plan validated clean), then restored via `specify plan remove user-login` + `specify plan amend account-lockout --add-source legacy-auth=user-login` (validated clean again). Note: `specify plan add --project` fails with `plan-project-no-registry` in a single regular project; omitting `--project` works (sole project auto-binds).
- Gate 1 stamped by the operator-side `specify plan transition account-lockout approved`; `/spec:plan` exited at `pending` with the literal transition hint.
- Refine: extract ran once per contributing source in declaration order (journal: `source.execution.agent` + `slice.extract.cache-miss` for `security-design-notes`/documentation then `legacy-auth`/typescript); Evidence persisted at `evidence/security-design-notes.yaml` and `evidence/legacy-auth.yaml`.
- Synthesis projected 5 requirements, all `Status: agreed` with `Sources: security-design-notes, legacy-auth` rendered on every block; `specify slice validate` exited 0 (2 non-blocking `kind: review` suggestions); slice transitioned to `refined`.

## Evidence

- **Reproduce:** `scripts/snapshot.sh acceptance/.sandbox/lead-reconciliation`
- **Retained at:** `acceptance/.sandbox/lead-reconciliation/`
- **Key paths:** `plan.yaml`, `change.md`, `discovery.md`, `.specify/journal.jsonl`, `.specify/slices/account-lockout/{model.yaml,evidence/,specs/account-lockout/spec.md}`
