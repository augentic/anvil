# /spec:explore

A thinking partner for ideas, investigation, and requirements.

## Synopsis

```text
/spec:explore [change-name?]
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `change-name` | No | Explore within the context of an existing change. If omitted, explores freely. |

## When to use

- You want to think through a problem before committing to a change.
- You need to investigate the codebase, explore options, or clarify requirements.
- You want to discuss trade-offs before defining.
- You are mid-change and need to reconsider an approach.

## Artifacts produced

None by default. May update existing change artifacts (proposal, specs, design) if you ask it to capture a decision. Never writes application code.

## Behavior

There is no fixed workflow. Explore mode follows the conversation. It can:

- Read and analyse source code.
- Discuss architectural options and trade-offs.
- Help refine vague requirements into concrete specifications.
- Update artifacts in an existing change if asked.

When you are ready to proceed, transition to `/spec:define`.

## Lifecycle transitions

None. Explore does not create or transition changes.

## Error modes

None specific to explore -- it is conversational.

## Examples

```text
# Open-ended exploration
/spec:explore

# Explore within an existing change
/spec:explore add-auth
```

**Typical flow:**

```text
/spec:explore       (think through the problem)
    ...             (conversation, investigation)
/spec:define        (when ready, define the change)
```

## See also

- [/spec:define](define.md) -- the natural next step after exploration
