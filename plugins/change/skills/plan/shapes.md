# Change shapes (orchestration mode)

The three change shapes flow through the same seven-step sequence in [orchestration.md](orchestration.md). Only the inputs to step 3 (Plan) differ; steps 4–7 are shape-agnostic.

## Shape inference (when `--shape` is omitted)

When `--shape` is omitted under `--orchestrate`, the mode infers the shape from the CLI flags using a closed table:

| Flags supplied | Inferred shape | Notes |
|---|---|---|
| `--source <k>=<v>` (one or more) | `migrate-legacy` | `--from` may co-exist; `--against` may co-exist. |
| `--from <path>` (only) | `new-feature` | Documentation-driven greenfield/feature work. |
| `--against <path>` (only) | `new-feature` | Refactor-target without legacy migration sources. |
| neither `--source`, `--from`, nor `--against` | `update-existing` | Baseline-driven extension; depends on a populated change-brief `inputs` list (or, when absent, a non-empty registry whose baseline specs are the dominant signal). |

When `--shape` **is** explicitly supplied, validate the flags against the table:

| Explicit shape | Required | Forbidden |
|---|---|---|
| `migrate-legacy` | at least one `--source` | — |
| `new-feature` | at least one `--from` OR `--against` OR a populated change-brief `inputs` list | — |
| `update-existing` | — | `--from`, `--against`, `--source` (any of the three is a hard exit) |

A shape conflict is a hard exit before step 1 of the orchestration sequence; the diagnostic names the offending flag(s) so the operator can drop the flag or change the shape.

## Three-shape handling

### `migrate-legacy`

```text
/change:plan <name> orchestrate migrate-foo \
    shape migrate-legacy \
    source monolith=git@github.com:org/legacy-foo.git
```

Pre-flight asserts at least one `--source` (closed-enum kind defaults to `legacy-code`). Step 3 forwards `--source` to `/change:plan` (default mode), which clones the source into `.specify/plans/<name>/analyze/<key>/` (tier-1 workspace) for shallow inventory; deep `/spec:extract` runs at define time per slice. When the registry is empty, the discovery brief proposes a multi-project topology and the operator approves entries via the 2B greenfield path. Targets are existing or newly-minted registered projects.

Fixture: `fixtures/migrate-legacy/`.

### `new-feature`

```text
/change:plan <name> orchestrate dark-mode \
    shape new-feature \
    from ./docs/dark-mode-spec.md
```

Pre-flight asserts at least one of `--from`, `--against`, or a populated change-brief `inputs` list. Step 3 forwards the documentation inputs to `/change:plan` (default mode), which runs discovery against the docs, syncs peers (when the registry is multi-project), proposes slices, and assigns each slice to an existing project via the registry. New projects spawn at assignment time via the 2B registry-proposal sub-step when the operator's override names a project not yet in `registry.yaml`.

Fixture: `fixtures/new-feature/`.

### `update-existing`

```text
/change:plan <name> orchestrate polish-pass \
    shape update-existing
```

Pre-flight forbids `--from`, `--against`, and `--source`. Step 3 invokes `/change:plan` (default mode) with no input flags; the plan skill reads the change-brief `inputs` list (which may be empty) and falls back to baseline accumulation in `.specify/workspace/<peer>/specs/` — the dominant signal for a baseline-driven extension. No new registry entries are added; targets are exclusively existing registered projects.

Fixture: `fixtures/update-existing/`.
