# Build phase output templates

Render these templates while implementing tasks, on completion, and on pause. The SKILL.md body keeps the algorithmic spine; the verbatim template surfaces live here so the agent can copy them without scrolling the body.

## Output during implementation

```text
## Implementing: <slice-name>

Working on task 3/7: <task description>
[...implementation happening...]
Task complete

Working on task 4/7: <task description>
[...implementation happening...]
Task complete
```

## Output on completion

```text
## Implementation Complete

**Slice:** <slice-name>
**Progress:** 7/7 tasks complete

### Completed This Session
- [x] Task 1
- [x] Task 2
...

All tasks complete! Ready to merge this slice.
Run `/spec:merge` to finalize.
```

## Output on pause (issue encountered)

```text
## Implementation Paused

**Slice:** <slice-name>
**Progress:** 4/7 tasks complete

### Issue Encountered
<description of the issue>

**Options:**
1. <option 1>
2. <option 2>
3. Other approach

What would you like to do?
```
