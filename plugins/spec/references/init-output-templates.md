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

**Capability**: $CAPABILITY
**Config**: .specify/project.yaml
**Context**: AGENTS.md
**Context lock**: .specify/context.lock
**Slices**: .specify/slices/
**Baseline specs**: .specify/specs/

Next steps:
1. Edit `.specify/project.yaml` to describe your project
2. Run `/spec:define` to create your first change
```

## Brownfield

```
## Specify Initialized (Existing Codebase Detected)

**Capability**: $CAPABILITY
**Config**: .specify/project.yaml
**Context**: AGENTS.md
**Baseline change**: .specify/slices/initial-baseline/

Next steps:
1. Edit `.specify/project.yaml` to describe your project
2. Run `/spec:extract . .specify/slices/initial-baseline/` to analyze the codebase
3. After extraction, run `/spec:merge initial-baseline` to promote specs to baseline
4. Then run `/spec:define` for future changes
```

## Hub

```
## Specify Initialized (Platform Hub)

**Topology**: registry-only hub
**Config**: .specify/project.yaml (`hub: true`; `capability:` omitted)
**Context**: AGENTS.md
**Context lock**: .specify/context.lock
**Registry**: registry.yaml (`version: 1`, `projects: []`)

Next steps:
1. Add registered projects with `specify registry add`
2. Run `specify change draft <name>` to scaffold the change brief and plan together
3. Run `/change:draft <name>` to author the plan, `/change:execute loop` to drive it, then `/change:finalize <name>` to push and archive
```
