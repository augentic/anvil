# Scenario run summary

## Run header

- **Scenario id:** `target-shape-injection`
- **Scenario file:** `acceptance/scenarios/target-shape-injection.md`
- **Backend:** `manual`
- **Operator / agent:** Cursor agent (Composer)
- **Run id:** `2026-06-09T07:50:00Z`
- **Started at / finished at:** `2026-06-09T07:48:00Z` / `2026-06-09T07:51:00Z`
- **`specify` build:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0` (built via `make install-cli` from `Specify.toml` pin on branch `rfc-41`)
- **Workspace / project roots:**
  - Intent fixture: `acceptance/.sandbox/target-shape-injection-intent/`
  - Documentation fixture: `acceptance/.sandbox/target-shape-injection-doc/`

## Inputs created

- `acceptance/.sandbox/target-shape-injection-intent/` — created (fresh single-project, local `omnia` adapter)
- `acceptance/.sandbox/target-shape-injection-intent/adapters/sources/{intent,documentation}` — symlinked from framework repo (required because `specify init` does not vend source adapters)
- `acceptance/.sandbox/target-shape-injection-doc/` — created (fresh single-project, local `omnia` adapter)
- `acceptance/.sandbox/target-shape-injection-doc/docs/greeting.md` — created
- `acceptance/.sandbox/target-shape-injection-doc/adapters/sources/{intent,documentation}` — symlinked from framework repo

**Environment note:** `specify init omnia@v1` failed (`adapter-git-failed: Remote branch v1 not found`). Both fixtures used `specify init <repo>/adapters/targets/omnia` instead. Source adapters were symlinked into each sandbox so `intent` / `documentation` survey and extract could resolve.

## Invocation

### Plan

**Intent fixture**

```text
/spec:plan greeting "Add a GET /greeting endpoint …"  (driven via CLI equivalent)
```

CLI sequence: `specify plan create greeting --source intent=intent:value:…` → `specify source survey intent` (prepare/finalize) → `specify plan propose --from` → `specify plan validate` → `specify plan transition greeting approved`.

**Documentation fixture**

```text
/spec:plan greeting source brief=documentation:docs/greeting.md  (fresh project)
```

CLI sequence: `specify plan create greeting --source brief=documentation:docs/greeting.md` → documentation survey → propose → Gate 1 stamp → same refine path.

### Review (operator pause)

```bash
specify plan validate --format json   # both fixtures: 0 blocking findings
```

- **Gate 1 stamp:** `specify plan transition greeting approved` (both fixtures)
- **specify plan amend invocations:** none

### Execute

```text
n/a — scenario stages stop at refine; refine driven directly via specify slice create / extract / synthesize / validate / transition
```

### Finalize

```text
n/a
```

## Plan structure

| Role | Slice | Project | Depends on | Sources | Status |
| --- | --- | --- | --- | --- | --- |
| local | greeting | project | none | intent (intent fixture) / brief (doc fixture) | refined |

## Expected artifacts and state

| Item | Status | Notes |
| --- | --- | --- |
| `plan.yaml` | present | both fixtures |
| `.specify/slices/greeting/specs/greeting/spec.md` | present | Omnia unit layout (not root `spec.md`) |
| `.specify/slices/greeting/design.md` | present | both fixtures |

## Assertions

| Assertion id | Verdict | Evidence pointer |
| --- | --- | --- |
| `plan-exists` | pass | `acceptance/.sandbox/target-shape-injection-intent/plan.yaml` and doc twin |
| `spec-reflects-shape-idioms` | needs-human | Both specs carry provenance blocks, handler-observable requirements, scenarios with Config preconditions, and BadRequest codes in scenarios. Neither persisted spec includes the dedicated **Error conditions** table the shape-evidence checklist expects (`acceptance/fixtures/targets/omnia/expected/shape-evidence.md`); error mapping appears in `design.md` instead. Operator should confirm whether table-in-spec is mandatory for pass. |
| `design-reflects-shape-idioms` | pass | Both `design.md` files contain all eight Omnia shape sections in order: Domain model → Provider trait dependencies → Handler delegation → External surfaces → Configuration → Error mapping → Validation placement → Observability; provider DI (`Config`), `Handler<P>`, `from_input()` vs `handle()` split, `thiserror` + `From<…> for omnia_sdk::Error`, `monotonic_counter` metrics. |
| `intent-and-doc-fixtures-agree` | pass | `design.md` bodies are structurally identical across fixtures. Spec structure matches; only `Sources:` differs (`intent` vs `brief`) as expected. |

## Negative expectations

| Negative expectation | Verdict | Notes |
| --- | --- | --- |
| `automated-runner-added` | held | CLI-driven only |
| `fake-forge-added` | held | no forge interaction |
| `transcript-replay-added` | held | live synthesis |
| `ci-target-added` | held | local run |
| `golden-output-required` | held | structural checks only |

## Command output

- **Plan validation:** both fixtures — `summary.critical: 0`, empty findings
- **Slice validation:** both fixtures — 0 violations; 2 suggestion-level review findings each (`proposal.uses-imperative-language`, `specs.uses-normative-language`)
- **Execute loop:** n/a
- **Finalize invocations:** n/a

## Artefact snapshot

Intent fixture sandbox retained at `acceptance/.sandbox/target-shape-injection-intent/`. Key slice artifacts:

```text
.specify/slices/greeting/
├── design.md
├── specs/greeting/spec.md
├── proposal.md
├── tasks.md
├── model.yaml
└── evidence/intent.yaml
```

Documentation twin at `acceptance/.sandbox/target-shape-injection-doc/` with `evidence/brief.yaml` and `Sources: brief` on requirements.

Full snapshot: `scripts/snapshot.sh acceptance/.sandbox/target-shape-injection-intent` (tree + journal available in sandbox).

## Cleanup

- **Workspaces / projects:** retained for inspection
- **Branches:** n/a
- **Run evidence:** this file + both sandboxes under `acceptance/.sandbox/`

## Verdict

- **Result:** pass
- **Fault domain on failure:** n/a
- **Notes:** Core shape injection into `design.md` is demonstrated on both intent and documentation sources with cross-fixture structural parity. One checklist item (`Error conditions` table in `spec.md`) needs operator sign-off — synthesis response included it but the persisted spec omits the standalone table (errors covered in scenarios + `design.md` Error mapping). Remote `omnia@v1` init remains blocked on this network (`v1` branch missing); local adapter path was used.
