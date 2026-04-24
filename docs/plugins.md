# Plugins

> **This page has moved.** See the Specify Operator Guide:
>
> - [Plugins Reference](reference/plugins/index.md) -- all five plugins and their skills
> - [Schemas Reference](reference/schemas/index.md) -- how schemas compose with plugins

<!-- inventory: generated from plugins/ SKILL.md frontmatter -->

## Specify (`plugins/spec/`)

- **analyze** — Plan-time capability inference for both legacy code and documentation inputs. Emits capability summaries into discovery.md — not full specs. Branches internally on --kind; per-kind clustering / extraction prompts are schema-owned. Use when the plan-time discovery brief needs a capability-level inventory of a source before propose slices it.
- **build** — Implement tasks from a Specify change. Use when the user wants to start implementing, continue implementation, or work through tasks.
- **define** — Define a new change with all artifacts generated in one step. Use when the user wants to quickly describe what they want to build and get a complete proposal with design, specs, and tasks ready for implementation.
- **drop** — Drop a change without merging specs into the baseline. Use when the user wants to discard a change that should not be merged normally.
- **execute** — Drive an initiative through its plan.yaml: read the plan, pick the next eligible change, run define → build → merge, and update status. Layer 2 automation over the Layer 1 specify plan CLI.
- **explore** — Enter explore mode - a thinking partner for exploring ideas, investigating problems, and clarifying requirements. Use when the user wants to think through something before or during a change.
- **extract** — Extract Specify artifacts (specs + design.md) from existing source code. Produces reconstruction-grade, language-agnostic artifacts capturing domain-level business logic. Supports optional `--include` / `--exclude` / `--manifest` filters that scope which source files are read for business-logic extraction without changing the artifact output shape.
- **init** — Initialize Specify in a project. Populates `.specify/.cache/` and invokes `specify init` to scaffold `.specify/` and write `project.yaml`. Use when setting up a new project for spec-driven development.
- **merge** — Merge a completed change. Merges delta specs into baseline and moves the change to the archive. Use when the user wants to finalize a change after implementation is complete.
- **plan** — Author the initial .specify/plan.yaml for an initiative via the pipeline.plan brief pipeline. Layer 3 counterpart to /spec:execute: /spec:plan writes the plan, /spec:execute runs it. When `.specify/registry.yaml` declares more than one project, runs the sync-peers phase (`specify workspace sync`) before propose and emits `workspace.md` for cross-repo planning.
- **status** — Show the current state of Specify changes. Invokes `specify status` and renders active changes, artifact completion, and task progress. Use when the user wants to check where they are.
- **verify** — Compare current code against baseline specs to detect drift. Use when the user wants to check whether the codebase still matches the merged specifications.

## Omnia (`plugins/omnia/`)

- **code-reviewer** — AI-powered code review for generated Rust crates, catching security issues and quality problems
- **crate-writer** — Write Rust WASM crates from Specify artifacts -- greenfield creation or incremental updates -- following Omnia SDK patterns with provider-based dependency injection.
- **guest-writer** — Generate a Rust project that exposes HTTP endpoints, subscribes to message topics, and handles WebSocket events in order to surface business logic via the Omnia WASI runtime.
- **test-writer** — Generate or update test suites for Omnia Rust WASM crates from Specify artifacts -- MockProvider setup, integration tests, spec-to-test mapping, and drift detection.

## Vectis (`plugins/vectis/`)

- **android-reviewer** — Review generated Android shell (Kotlin/Jetpack Compose) code for structural issues, integration correctness, and quality problems. Use when reviewing a Crux app's Android shell after generation, or when the user mentions android-reviewer.
- **android-writer** — Generate or update a Kotlin/Jetpack Compose Android shell for a Crux application. Use when the user wants to create an Android shell, scaffold Android UI, or generate Compose views for a Crux app, or mentions android-writer.
- **core-reviewer** — Review generated Crux core (Rust shared crate) code for structural issues, logic bugs, and quality problems. Use when reviewing a Crux app's core after generation, or when the user mentions core-reviewer.
- **core-writer** — Generate or update a Rust Crux shared crate from Specify artifacts. Use when implementing core tasks from a Specify change, or when the user mentions core-writer.
- **design-system-writer** — Generate or update the platform-specific design system implementation from tokens.yaml for iOS (Swift Package) and Android (Jetpack Compose Material 3 library). Use when implementing design-system tasks from a Specify change, or when the user mentions design-system-writer.
- **ios-reviewer** — Review generated iOS shell (SwiftUI) code for structural issues, integration correctness, and quality problems. Use when reviewing a Crux app's iOS shell after generation, or when the user mentions ios-reviewer.
- **ios-writer** — Generate or update a SwiftUI iOS shell for a Crux application from Specify artifacts. Use when implementing iOS shell tasks from a Specify change, or when the user mentions ios-writer.
- **template-updater** — Fix Vectis CLI templates and version pins when upstream crate or tooling bumps break a freshly scaffolded project. Use when `specify vectis update-versions --verify` reports a failing cap-matrix combo, when a Crux/uniffi/Gradle release has introduced template drift, or when the user mentions template-updater.
- **test-writer** — Generate or update test suites for Crux shared crates from Specify artifacts -- spec-to-test mapping, traceability, drift detection, and synchronous Crux testing patterns.

## Runtime Toolkit (`plugins/rt/`)

- **git-cloner** — Clone git repositories autonomously with validation, error handling, and flexible options.
- **replay-writer** — Add tests from real-life JSON fixtures in tests/data/replay/, run tests, and review code so tests pass. For crates already generated by the Specify workflow.
- **wiretapper** — Add wiretap code to a cloned legacy TypeScript repo to capture request/response and side-effect data as fixture JSON; detect patterns, generate adapters, wire entrypoint, verify compile.

## Plan (`plugins/plan/`)

- **sow-writer** — Generate a Statement of Work (SoW) document from Specify artifacts and project context.
