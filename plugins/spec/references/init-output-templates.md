# Init output templates

Verbatim summaries `/spec:init` prints after a successful invocation. Pick the template that matches the resolved topology and `$HUB_MODE` / baseline-extraction outcome.

| Scenario | Template |
|---|---|
| Regular project, no codebase indicators or user declined extraction | [Greenfield](#greenfield) |
| Regular project, user opted into baseline extraction | [Brownfield](#brownfield) |
| Hub init (`$HUB_MODE=true`) | [Hub](#hub) |

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
2. Run `/spec:plan initial-baseline source code-typescript=.` (or whichever `code-*` source matches the codebase) to enumerate candidates
3. Stamp Gate 1 with `specrun plan transition initial-baseline reviewed`, then run `/spec:execute` to drive `refine -> build -> merge`
4. Run `/spec:plan <name> ...` for future changes
```

## Hub

```
## Specify Initialized (Platform Hub)

**Topology**: registry-only hub
**Config**: .specify/project.yaml (`hub: true`; `adapter:` omitted)
**Context**: AGENTS.md
**Context lock**: .specify/context.lock
**Registry**: registry.yaml (`version: 1`, `projects: []`)

Next steps:
1. Add registered projects with `specrun registry add`
2. Run `/spec:plan <name>` to author `change.md` + `plan.yaml` together
3. Stamp Gate 1 with `specrun plan transition <name> reviewed`, then run `/spec:execute` to drive `refine -> build -> merge` per slice, and `/spec:finalize <name>` to push and archive
```
