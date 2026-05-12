# Baseline detection — existing codebase vs greenfield

Step 6 of the `/spec:init` Critical Path. Only runs for **regular projects** — skip entirely in hub mode and render the hub output template directly.

## Heuristics

Check whether the project root contains an active codebase by looking for either:

- **Manifest files**: `Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`, `pom.xml`, `*.csproj`, `build.gradle`, `Gemfile`.
- **Source directories**: `src/`, `lib/`, `app/`, `cmd/`.

If **none** of these are found, treat the project as **greenfield**: render the Greenfield template from [init-output-templates.md](init-output-templates.md) and stop.

## Brownfield prompt

If at least one indicator is found, use the **AskQuestion tool**:

> "I've detected an existing codebase (found `<indicator>`). Would you like me to analyze it and generate baseline specs that capture its current behavior? This uses `/spec:extract`."

Options:

- **Yes, generate baseline specs** — proceed to create the slice.
- **No, skip for now** — render the Greenfield template and stop (the user can run `/spec:extract` manually later).

## Creating the baseline slice

If the user chooses **yes**, create the slice via the CLI:

```bash
specify slice create initial-baseline --format json
```

The CLI validates the name, creates `.specify/slices/initial-baseline/specs/`, and writes the initial `.metadata.yaml` (status `defining`, `created_at` timestamp). Then render the Brownfield template from [init-output-templates.md](init-output-templates.md) and stop.

Init does NOT run `/spec:extract` itself — baseline-extraction is delegated. The brownfield template tells the operator the next command to run.
