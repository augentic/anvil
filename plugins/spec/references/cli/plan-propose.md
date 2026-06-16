# specify plan propose

Reconcile surveyed `discovery.md` leads into the plan's `slices[]` grouping. Two modes; exactly one is required.

```bash
specify plan propose --dry-run [--format json]
specify plan propose --from <response.json> [--format json]
```

- `--dry-run` emits the **request envelope** — a flat catalog of raw `(source, lead)` leads read 1:1 from `discovery.md`, plus the project topology (always at least one project, each carrying its normalized `target` adapter). It writes no plan state and emits no journal event; its only filesystem effect is recreating the plan scratch lane (`.specify/scratch/plan/`) empty, so `--from` can never consume a stale response envelope from a prior run.
- `--from <response.json>` is the **only slice writer**. It schema-validates the raw response file (`proposal-schema`), re-reads `discovery.md`, rebuilds the lead catalog, validates the agent's `slices[]` grouping, enforces total lead coverage, validates explicit slice names, binds projects, atomically replaces `plan.yaml.slices[]`, then emits `plan.reconcile.completed`. It never trusts a prior dry-run snapshot.

**Platform bootstrap reconciliation (default-on).** When a bound project declares non-empty `project.yaml.platforms`, `--from` runs a deterministic post-pass before the write commits. For Vectis-bound projects the CLI calls `specify-vectis-shell-detect` in-process (the same heuristics behind `vectis verify --mode detect`; propose does not dispatch the vectis WASM tool) to find declared-but-absent shell trees (`core` → `shared/src/app.rs`, `ios` → `iOS/**/*.swift`, `android` → `Android/**/*.kt`). `Plan::reconcile_platforms` inserts bootstrap slices (`app-foundation` for greenfield, `bootstrap-<platform>` for incremental) and wires agent-proposed feature slices as `depends-on`. Non-Vectis targets skip shell detection. Omnia and other non-Vectis adapters are unaffected.

Passing neither mode fails with `plan-propose-mode-required`; passing both is rejected by the argument parser.

The response file's canonical location is `.specify/scratch/plan/propose-response.json` — the gitignored plan scratch lane that `--dry-run` just reset. Never write the envelope to the project root.

**Replaceable gate.** `--from` runs only while the plan is replaceable — `lifecycle: pending` and every entry `pending`; otherwise `plan-reconcile-plan-not-replaceable`.

Validation codes (all exit 2):

| Code | Meaning |
|------|---------|
| `plan-propose-mode-required` | Neither `--dry-run` nor `--from` was given. |
| `proposal-schema` | The `--from` response file failed JSON-Schema validation. |
| `plan-reconcile-empty-catalog` | `discovery.md` surfaced no leads to reconcile. |
| `plan-reconcile-lead-orphan` | A cited `(source, lead)` is not in the surveyed catalog. |
| `lead-coverage-orphan` | Grouped leads do not achieve total coverage — a surveyed lead is referenced by no slice. |
| `plan-reconcile-slice-source-collision` | A slice names more than one lead from the same source. |
| `plan-reconcile-slice-name-invalid` | A slice `name` is not kebab-case. |
| `plan-reconcile-slice-name-collision` | Two slices resolve to the same plan slice name. |
| `plan-reconcile-depends-on-cycle` | Projected `depends-on` edges form a cycle. |
| `plan-reconcile-project-binding-required` | A slice omits `project` when more than one project exists. |
| `plan-reconcile-project-orphan` | A slice binds a `project` absent from the request topology. |
| `plan-reconcile-plan-not-replaceable` | The plan is approved or carries a non-pending entry. |

Advisory findings (non-blocking; `--from` still succeeds, exit 0):

| Code | Meaning |
|------|---------|
| `lead-decision-topic-overlap` | A surveyed lead's `topics[]` overlaps an accepted decision's `topics[]` on the slice's bound project. Review nudge: confirm the slice aligns with that decision (or record a superseding one). Latent until both leads and decisions carry topics. |
| `slice-divergence-unrecorded` | A slice flags `divergence: likely`/`accepted` but records no adequate `disagreements[]` (≥2 distinct source values per field). |
| `slice-divergence-orphan-values` | A slice records `disagreements[]` without a `divergence` flag. |
| `greenfield-seed-shadowed` | A bound project still declares a `registry.yaml` `greenfield_seed` after acquiring a baseline (`.specify/specs/` exists); the derived `surface[]` supersedes the seed — remove it. |

Envelopes validate against `schemas/discovery/proposal.schema.json` (`kind: request` for `--dry-run`, `kind: response` for `--from`). Full CLI reference: [specify plan](https://specify.augentic.io/reference/cli/plan.html#specify-plan-propose).
