# Plan invocation reference

Full positional and option grammar for `/change:plan`. The SKILL.md body keeps the bare signature; this reference carries the per-positional contract.

## Signature

```text
/change:plan <change-name> \
    [from <path>...] \
    [against <path>] \
    [source <key>=<path-or-url>...] \
    [focus <area>] \
    [extend] \
    [dry-run] \
    [orchestrate] \
    [shape migrate-legacy|new-feature|update-existing]
```

## Positional arguments

- **`<change-name>`** — kebab-case identifier; becomes the plan's top-level `name` field. Validated with the same rules as slice names (regex `^[a-z][a-z0-9-]*$`) before any other work. An invalid name is a hard exit with a clear diagnostic — the skill never rewrites or "helps" the name.
- **`from <path>`** — artefact file(s) or directory describing the target shape for greenfield authoring. Repeatable. Consumed by the discovery brief. Kind defaults to `documentation`; override via `:<kind>` suffix (see SKILL.md §Kind defaults for positional inputs).
- **`against <path>`** — an existing codebase to delta against, used for refactor or modernisation changes. Consumed by the discovery brief. Kind defaults to `legacy-code`; override via `:<kind>` suffix.
- **`source <key>=<path-or-url>`** — a named source for migration. Repeatable. The `key` is a kebab-case identifier recorded in the plan's top-level `sources` map and referenced by individual plan entries via their `sources` list; the `value` is either a local filesystem path or a git URL. The skill forwards the tuple verbatim; cloning (if any) is the discovery brief's concern via `/change:analyze` (which inlines a guarded `git clone` snippet — see the *Cloning a source tree* subsection in [`../skills/analyze/SKILL.md`](../skills/analyze/SKILL.md)). Kind defaults to `legacy-code`; override via `:<kind>` suffix.
- **`focus <area>`** — optional scoping hint for the propose brief. Free-form string; the propose brief decides how to interpret it.
- **`extend`** — add to an existing `plan.yaml` instead of refusing. See SKILL.md §Modes → `extend` for the full contract.
- **`dry-run`** — emit the readiness report and the proposed plan to stdout; write nothing. See SKILL.md §Modes → `dry-run`.
- **`orchestrate`** — enable orchestration mode: run the cross-repo umbrella after the authoring loop. The umbrella pushes per-project PRs, stops for operator merge when PRs are still open, and later finalizes after `specify change finalize` verifies every PR is merged. See [orchestration.md](../skills/plan/orchestration.md). Required when `shape` is supplied.
- **`shape migrate-legacy|new-feature|update-existing`** — explicit shape override under `orchestrate`. Inferred from the supplied inputs when omitted. Rejected with a hard diagnostic when `orchestrate` is absent. See [shapes.md](../skills/plan/shapes.md).

## Input sufficiency

At least one of `from`, `against`, `source`, or a populated change-brief `inputs` list must be supplied. A bare `/change:plan <name>` with no slash inputs **and** no change brief (or a brief with empty `inputs`) is a hard exit — the skill cannot decide the change's shape without at least one input.

When the change-brief `inputs` list is the only source of inputs, the skill reads them via `specify change show --format json` before entering the core loop and treats each entry as if it had been supplied as slash-positionals: `kind: legacy-code` entries route through the same path as `source <k>=<path>:legacy-code`, and `kind: documentation` entries route through the `from` path. Both documentation and legacy-code dispatch are live via `/change:analyze`. Plan-time `/spec:extract` call sites have been fully retired; `/spec:extract` now runs only at `/spec:define` time with scope inferred from the slice's description.
