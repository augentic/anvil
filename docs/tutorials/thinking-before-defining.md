# Thinking Before Defining

Not every change starts with a clear description. Sometimes you need to investigate, explore options, or clarify requirements before you are ready to define. That is what `/spec:explore` is for.

**Prerequisites:** Familiarity with the [define-build-merge loop](first-change.md).

## When to explore first

Explore mode is useful when:

- The requirement is vague ("make the app faster", "improve error handling").
- You need to investigate the codebase before deciding what to change.
- There are multiple approaches and you want to discuss trade-offs.
- You want to understand how existing capabilities work before modifying them.

## 1. Start exploring

```text
/spec:explore
```

There is no fixed workflow. The agent becomes a thinking partner. You can:

- Ask questions about the codebase: "How does the auth module handle token refresh?"
- Discuss options: "Should we use JWT or session tokens?"
- Investigate patterns: "Show me all the error handling in the payment service."
- Clarify requirements: "What would 'faster' mean in concrete terms?"

The conversation is open-ended. The agent reads code, analyses patterns, and helps you think through the problem.

## 2. Explore within a change

If you already have a change in progress and want to reconsider an approach:

```text
/spec:explore add-auth
```

This gives the agent context about the existing artifacts. You can ask it to reconsider a design decision or investigate an alternative.

## 3. Capture decisions

If the conversation reaches a conclusion you want to preserve, you can ask the agent to update the change's artifacts:

- "Update the design to reflect the JWT decision we just made."
- "Add a requirement for token refresh to the spec."

The agent will modify the artifact files in the change directory. It will not write application code -- that is `/spec:build`'s job.

## 4. Transition to define

When you have enough clarity, define the change:

```text
/spec:define "Add JWT-based authentication with token refresh"
```

The exploration informed your description. The agent may even reference decisions from the exploration when generating artifacts.

## The explore-define pattern

```text
/spec:explore          # think through the problem
    ...                # conversation, investigation, decisions
/spec:define           # when ready, commit to a change
/spec:build            # implement
/spec:merge            # merge
```

Explore is not a prerequisite -- you can skip straight to define if the requirement is clear. But when you need to think first, explore gives you a structured space to do it without committing to a change prematurely.

## What you learned

- `/spec:explore` is a thinking partner with no fixed workflow.
- You can explore freely or within the context of an existing change.
- Explore can update artifacts but never writes application code.
- Use explore when requirements are vague, options are unclear, or you need to investigate first.

## Next

[Brownfield Onboarding](brownfield-onboarding.md) -- bring an existing codebase into Specify by extracting specs from source code.
