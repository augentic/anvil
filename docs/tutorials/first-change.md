# Your First Slice

This tutorial walks you through the complete Specify workflow: initialise a project, define a slice, build the implementation, and merge the result into the baseline. By the end you will understand the core define-build-merge loop that underpins everything else in Specify.

**Prerequisites:** [Cursor IDE, Augentic plugins, and the `specify` CLI installed](../orientation/prerequisites.md).

## 1. Initialise the project

Open your project in Cursor and type the following in the agent chat:

```text
/spec:init https://github.com/augentic/specify/capabilities/omnia
```

> Replace `omnia` with `vectis` if you are building a cross-platform Crux application.

<details>
<summary>Expected output</summary>

```text
Specify Initialized
  Capability: omnia@latest
  Project config: .specify/project.yaml
  Cache: .specify/.cache/omnia/
```

</details>

Specify creates the `.specify/` directory:

```
.specify/
├── project.yaml           # project configuration
├── .cache/omnia/           # cached capability and briefs
├── slices/                # will hold active slices
├── specs/                 # will hold merged baseline specs
└── archive/               # will hold finalized slices
```

Open `.specify/project.yaml` and review it. This is where you describe your project's domain, tech stack, and any constraints the agent should know about. Customise it to match your project.

## 2. Define a slice

Now describe what you want to build:

```text
/spec:define "Add a greeting endpoint that accepts a name and returns a personalised message"
```

<details>
<summary>Expected output</summary>

```text
Slice created: add-greeting-endpoint

Generating artifacts...
  ✓ proposal.md
  ✓ specs/greeting/spec.md
  ✓ design.md
  ✓ tasks.md

Slice defined (4 tasks).
```

</details>

Specify generates four artifacts (five for the Vectis capability, which adds `composition.yaml` for screen layout):

1. **`proposal.md`** -- captures the motivation and scope. It names the capabilities that will be affected.
2. **`specs/greeting/spec.md`** -- behavioral requirements with scenarios:
   - `REQ-001`: The system SHALL accept a name and return a greeting.
   - Scenario: WHEN a valid name is provided, THEN return "Hello, {name}!".
   - Scenario: WHEN the name is empty, THEN return an error.
3. **`design.md`** -- the technical shape: domain model, API contract, error handling.
4. **`tasks.md`** -- the implementation checklist with checkboxes.

Take a moment to read each artifact. Notice how they separate concerns: the proposal says *why*, the specs say *what*, the design says *how*, and the tasks say *in what order*.

## 3. Check status

At any point you can check where things stand:

```bash
specify status
```

<details>
<summary>Expected output</summary>

```text
Active slices:
  add-greeting-endpoint  defined  (4 tasks, 0 complete)
```

</details>

You will see the slice listed with status `defined`, all artifacts marked complete, and the task count.

## 4. Build the implementation

Now implement the slice:

```text
/spec:build
```

<details>
<summary>Expected output</summary>

```text
Building add-greeting-endpoint...
  ✓ 1.1 Generate the domain crate
  ✓ 1.2 Generate test suites
  ✓ 1.3 Verify and review output
  ✓ 1.4 Final verification

Implementation Complete
  Tasks: 4/4 done
```

</details>

The agent reads the build brief and works through the tasks in `tasks.md`. For each task:

- If the task has a **skill directive tag** (see [Glossary](../appendices/glossary.md)) (e.g. `<!-- skill: omnia:crate-writer -->`), the agent delegates to that specialist skill.
- If the task has no tag, the agent implements it using the capability's default build instruction.

As each task completes, the agent marks it done via `specify slice task mark`. You can watch the checkboxes flip in `tasks.md`.

When all tasks are complete, the slice transitions to `complete`.

## 5. Merge into the baseline

Finalise the slice:

```text
/spec:merge
```

<details>
<summary>Expected output</summary>

```text
Merge preview:
  + specs/greeting/spec.md (new capability)

Merge complete.
  Baseline updated: .specify/specs/greeting/spec.md
  Archived: .specify/archive/2026-04-27-add-greeting-endpoint/
```

</details>

The agent:

1. **Previews** the merge -- shows what will be added to the baseline.
2. **Checks for conflicts** -- verifies the baseline has not changed since you defined the slice.
3. **Asks for confirmation.**
4. **Merges** -- applies the spec deltas to `.specify/specs/` and archives the slice.

After merging, look at `.specify/specs/`:

```
.specify/specs/
└── greeting/
    └── spec.md    # your greeting spec is now part of the baseline
```

This baseline (see [Glossary](../appendices/glossary.md)) is permanent. Future slices will see it and build on it. The slice directory has been moved to the archive:

```
.specify/archive/
└── 2026-04-24-add-greeting-endpoint/
    ├── .metadata.yaml
    ├── proposal.md
    ├── design.md
    ├── tasks.md
    └── specs/greeting/spec.md
```

## 6. Aside: dropping a slice

If at any point you decide a slice should not be merged -- it was exploratory, superseded, or just wrong -- you can discard it:

```text
/spec:drop
```

<details>
<summary>Expected output</summary>

```text
Drop slice add-greeting-endpoint? (y/n): y

Slice dropped.
  Archived: .specify/archive/2026-04-27-add-greeting-endpoint/
  Baseline unchanged.
```

</details>

The slice is archived with status `dropped`. Baseline specs remain unchanged. You can provide a reason to skip the interactive confirmation:

```text
/spec:drop reason "Superseded by a different approach"
```

## What you learned

- **`/spec:init`** sets up the project once.
- **`/spec:define`** generates all artifacts from a description.
- **`/spec:build`** implements the tasks, delegating to specialist skills.
- **`/spec:merge`** applies specs to the baseline and archives the slice.
- **`/spec:drop`** discards a slice without affecting the baseline.
- Artifacts separate *why*, *what*, *how*, and *sequence*.
- The baseline accumulates over time, giving future slices context.

## Next

[Iterating on a Baseline](iterating-on-baseline.md) -- make a second slice that modifies an existing capability and learn about delta specs.
