# /spec:build

Implement tasks from a defined change.

## Synopsis

```text
/spec:build [change-name?]
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `change-name` | No | Name of the change to build. If omitted, uses the only active change or prompts for selection. |

## When to use

- A change is `defined` (all artifacts present) and you want to start or continue implementation.

## Artifacts produced

Source code changes in the project codebase (not under `.specify/`). Task checkboxes in `tasks.md` are flipped via `specify change task mark` as each task completes.

## Behavior

1. Validates that the change is in `defined` or `building` state.
2. Transitions the change from `defined` to `building` (if not already).
3. Reads the schema's build brief.
4. Runs pre-shell validation when `composition.yaml` is present (Vectis only): checks field coverage, event coverage, ViewModel mapping, overlay trigger consistency, and navigation graph consistency. Errors halt shell generation; warnings are logged.
5. Works through tasks sequentially:
   - Tasks with a **skill directive tag** (e.g. `<!-- skill: omnia:crate-writer -->`) are delegated to the named specialist skill.
   - Tasks without a skill tag are implemented via the schema's default build instruction.
6. Marks each task complete via `specify change task mark`.
7. On completion of all tasks, transitions to `complete`.
8. Writes phase outcome.

### Contract-only changes

Changes using the `contracts` schema have a different build behavior. The build brief delegates to the format-appropriate `/contract:*` skill -- `/contract:openapi` for HTTP / resource APIs, `/contract:asyncapi` for evented / pub-sub / streaming, `/contract:json-schema` for shared payload schemas -- rather than code-generation skills. It runs author or importer intent to produce change-local contract artifacts, then verifier intent for structural validation. A verify-repair loop runs up to 2 iterations: if the verifier reports failures, the same skill's producing intent makes targeted repairs, then the verifier re-checks. No implementation code is generated.

## Lifecycle transitions

`defined --> building --> complete`

## Error modes

| Error | Cause | Resolution |
|-------|-------|------------|
| Change not defined | Artifacts are incomplete | Run `/spec:define` first |
| Validation failure | Artifact does not conform to schema rules | Fix the artifact and retry |
| Specialist skill failure | A delegated skill encounters an error | Check the skill's output, fix, and re-run `/spec:build` |
| Build failure | Generated code does not compile or pass tests | The agent iterates on fixes within the build phase |

## Examples

```text
# Build the only active change
/spec:build

# Build a specific change by name
/spec:build add-auth
```

## See also

- [/spec:define](define.md) -- generate artifacts before building
- [/spec:merge](merge.md) -- next step after all tasks complete
- [Artifact Format](../artifact-format.md) -- skill directive tag syntax
- [Plugins](../plugins/index.md) -- specialist skills invoked during build
