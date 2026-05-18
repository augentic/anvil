# Define skill — regenerate mode

`/spec:define <name> <artifact-id>` re-emits a single artifact in an existing
slice without re-running the full define flow or transitioning lifecycle
status. The SKILL.md body collapses the procedure to a single step pointer;
this reference owns the full algorithm.

## When to use

The operator passes an artifact ID as the third positional (e.g. `/spec:define
my-change design`). Regenerate mode is the only define-time path that touches
one artifact in isolation; every other invocation walks the full pipeline.

## Procedure

1. Run `specify slice status <name> --format json`. If the CLI returns
   `not_found`, the slice is missing; if `status` is some value other than
   `defined` / `building`, warn before proceeding.
2. Run `specify adapter pipeline define --change .specify/slices/<name>
   --format json` to resolve the brief for the target artifact ID. The
   returned `briefs[]` lists every define brief in topological order with each
   brief's `path`, `needs`, and `generates`.
3. For the brief matching the requested artifact ID, verify each entry in its
   `needs` is already `present` on the pipeline response.
4. Read the required dependency artifacts for context (paths come from each
   brief's `generates` joined to `.specify/slices/<name>/`).
5. Read the brief file from the returned `path`.
6. Regenerate ONLY the specified artifact, applying `domain` and effective
   rules as constraints.
7. Do NOT change `.metadata.yaml` status — there is no `specify slice
   transition` call in regenerate mode.
8. Render the output template below, then stop. Do not proceed to the full
   define flow.

## Output template

```markdown
## Artifact Regenerated

**Change:** <name>
**Artifact:** <generates> (regenerated)
**Dependencies read:** <list of needs artifacts>

The artifact has been updated. Other artifacts are unchanged.
```
