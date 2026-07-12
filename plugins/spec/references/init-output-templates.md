# Init output templates

Verbatim summaries `/spec:init` prints after a successful invocation. Pick the template that matches the resolved topology and `$WORKSPACE_MODE` / baseline-extraction outcome.

| Scenario | Template |
|---|---|
| Regular project, no codebase indicators or user declined extraction | [Greenfield](#greenfield) |
| Regular project, user opted into baseline extraction | [Brownfield](#brownfield) |
| Workspace init (`$WORKSPACE_MODE=true`) | [Workspace](#workspace) |

## Greenfield

```
## Specify Initialized

**Adapter**: $ADAPTER
**Config**: .specify/project.yaml
**Context**: AGENTS.md
**Context lock**: .specify/context.lock
**Slices**: .specify/slices/
**Baseline specs**: .specify/specs/

Next steps:
1. Edit `.specify/project.yaml` to describe your project
2. Run `/spec:plan <name>` to author your first change
```

## Brownfield

```
## Specify Initialized (Existing Codebase Detected)

**Adapter**: $ADAPTER
**Config**: .specify/project.yaml
**Context**: AGENTS.md
**Baseline change**: .specify/slices/initial-baseline/

Next steps:
1. Edit `.specify/project.yaml` to describe your project
2. Run `/spec:plan initial-baseline source typescript=.` (or whichever language source matches the codebase) to survey leads
3. Stamp Gate 1 with `specify plan transition initial-baseline approved`, then run `specify plan execute` to drive `refine -> build -> merge`
4. Run `/spec:plan <name> ...` for future changes
```

## Workspace

```
## Specify Initialized (Workspace Root)

**Topology**: registry-only workspace
**Config**: .specify/project.yaml (`workspace: true`; `adapter:` omitted)
**Context**: AGENTS.md
**Context lock**: .specify/context.lock
**Registry**: registry.yaml (`version: 1`, `projects: []`)

Next steps:
1. Add registered projects with `specify registry add`
2. Materialize `workspace/<project>/` slots through your normal repository tooling
3. Run `/spec:plan <name>` to author `change.md` + `plan.yaml` together
4. Stamp Gate 1 with `specify plan transition <name> approved`, then run `specify plan execute` to drive `refine -> build -> merge` per slice; publish through your normal repository workflow before `/spec:finalize <name>` archives the plan
```
