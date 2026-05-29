# /spec:refine

Refine a plan entry's slice — run extract per bound source, synthesize proposal, spec, design, and tasks, validate, transition to `refined`.

## Synopsis

```text
/spec:refine [slice-name]
```

## Arguments

| Argument | Required | Description |
| -------- | -------- | ----------- |
| `slice-name` | No | Slice to refine. When omitted, resolves the active `in-progress` entry from `specrun plan next`. |

## When to use

- An `in-progress` plan entry needs slice-time synthesis.
- Re-running extract after amending plan sources.
- As a manual breakout when `/spec:execute` parks before build.

Not for first-time change authoring without a plan — use [/spec:plan](../change-skills/plan.md).

## Artifacts produced

| Artifact | Location | Content |
| -------- | -------- | ------- |
| Slice metadata | `.specify/slices/<name>/.metadata.yaml` | Lifecycle `refining` → `refined` |
| Evidence | `.specify/slices/<name>/evidence/<source-key>.yaml` | Per-source extract output |
| Proposal | `.specify/slices/<name>/proposal.md` | Why the slice exists; scope |
| Spec | `.specify/slices/<name>/specs/<unit>/spec.md` | Behavioral requirements |
| Design | `.specify/slices/<name>/design.md` | Technical shape |
| Tasks | `.specify/slices/<name>/tasks.md` | Implementation sequence |
| Reconciliation index | `.specify/slices/<name>/reconciliation.yaml` | Audit-only reconciliation index |

## Behavior

1. **Resolve target and sources** — read `plan.yaml.slices[<slice>]` for `target:` and `sources[]`; cross-resolve against `discovery.md` lead inventory.
2. **Create slice directory** — `specrun slice create <name> --target <target>` stamps `refining`.
3. **Extract serially** — for each source binding, run the adapter's `extract` brief; persist Evidence YAML.
4. **Synthesize** — load target `shape` brief; write `proposal.md → spec.md → design.md → tasks.md` in fixed order.
5. **Write `reconciliation.yaml`** — atomic reconciliation index per `REQ-*` id.
6. **Validate** — `specrun slice validate`; on failure, slice stays `refining`.
7. **Transition** — `specrun slice transition <name> refined`.

Synthesis tags (`[unknown]`, `[conflict]`, `[divergence]`) never park the slice — refine still transitions to `refined`.

### Closing hints

On success:

```text
Slice <slice-name> refined. spec.md tags: <U> unknown, <C> conflict, <D> divergence. Review .specify/slices/<slice-name>/spec.md, then run /spec:build <slice-name> or resume /spec:execute.
```

On extract failure, the slice stays `refining` with amend-plan guidance. On validation failure, fix artifacts and re-validate before transitioning.

## Lifecycle transitions

`(none) → refining → refined` (or stays `refining` on extract/validation failure)

## Error modes

| Error | Cause | Resolution |
| ----- | ----- | ---------- |
| `refine-no-active-slice` | No `in-progress` entry and no slice argument | Run `/spec:execute` or pass slice name |
| `refine-binding-unresolved` | Source key or lead id not in plan/discovery | Fix plan bindings |
| Extract failure | Source path denied or brief error | Amend plan sources; re-run refine |
| Validation failure | Provenance or reconciliation drift | Fix `spec.md` or `reconciliation.yaml`; re-validate |

## Examples

```text
# Refine the active in-progress slice (inside /spec:execute)
/spec:refine

# Refine a specific slice by hand
/spec:refine fix-typo
```

## See also

- [Resolve spec conflicts](../../how-to/resolve-spec-conflicts.md) — `[conflict]` and `[divergence]` tags
- [/spec:build](build.md) — next phase after refine
- [Artifact format](../artifact-format.md) — requirement block shape
- [Lifecycle](../lifecycle.md) — slice state machine
