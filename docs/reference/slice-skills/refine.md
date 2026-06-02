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
| Evidence | `.specify/slices/<name>/evidence/<source>.yaml` | Per-source extract output |
| Proposal | `.specify/slices/<name>/proposal.md` | Why the slice exists; scope |
| Spec | `.specify/slices/<name>/specs/<unit>/spec.md` | Behavioral requirements |
| Design | `.specify/slices/<name>/design.md` | Technical shape |
| Tasks | `.specify/slices/<name>/tasks.md` | Implementation sequence |
| Slice model | `.specify/slices/<name>/model.yaml` | Single structured artifact; carries provenance inline (M2b synthesis kernel) |

## Behavior

The authoritative step-by-step (CLI choreography, handoff envelopes, guardrails) lives in the [`/spec:refine` skill body](../../../plugins/spec/skills/refine/SKILL.md); what the agent writes into the synthesis response is owned by the [synthesis playbook](../../../plugins/spec/references/synthesis/). The operator summary:

1. **Resolve target and sources** — take the resolved `target` from `specrun plan next` (the plan stores no per-slice `target`) and read `sources[]` from `plan.yaml.slices[<slice>]`; cross-resolve against `discovery.md` lead inventory.
2. **Create slice directory** — `specrun slice create <name> --target <target>` stamps `refining`.
3. **Extract serially** — for each source binding, run the adapter's `extract` brief; persist Evidence YAML.
4. **Synthesize** — drive the two-phase `specrun slice synthesize` verb: the agent authors per-requirement claims, an `agreement` verdict, and prose; the kernel owns `REQ`/`TASK` ids, status, winner markers, and rendered `Sources:` lists, persisting `proposal.md → specs/<unit>/spec.md → design.md → tasks.md → model.yaml` with provenance carried inline in `model.yaml`.
5. **Validate** — `specrun slice validate`; on failure, slice stays `refining`.
6. **Transition** — `specrun slice transition <name> refined`.

Synthesis tags (`[unknown]`, `[conflict]`, `[divergence]`) never park the slice — refine still transitions to `refined`.

### Closing hints

On success:

```text
Slice <slice-name> refined. spec tags: <U> unknown, <C> conflict, <D> divergence. Review .specify/slices/<slice-name>/specs/, then run /spec:build <slice-name> or resume /spec:execute.
```

On extract failure, the slice stays `refining` with amend-plan guidance. On a `slice.synthesize.failed` or post-persist validation failure, the prior artifacts stay intact; fix the synthesis response and re-run `specrun slice synthesize --from` before transitioning.

## Lifecycle transitions

`(none) → refining → refined` (or stays `refining` on extract/validation failure)

## Error modes

| Error | Cause | Resolution |
| ----- | ----- | ---------- |
| `refine-no-active-slice` | No `in-progress` entry and no slice argument | Run `/spec:execute` or pass slice name |
| `refine-binding-unresolved` | Source key or lead id not in plan/discovery | Fix plan bindings |
| Extract failure | Source path denied or brief error | Amend plan sources; re-run refine |
| Synthesis / validation failure | `slice.synthesize.failed` (orphan claim, schema gate) or a post-persist drift finding | Fix the synthesis response and re-run `specrun slice synthesize --from`; never hand-edit the kernel-rendered provenance lines |

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
