# Decision Log

Key architectural decisions in Specify, distilled from the design RFCs. Each entry explains the *why* behind a design choice. For full context, follow the links to the original RFCs.

## CLI owns correctness, agent owns judgment

**Decision:** All deterministic operations (validation, lifecycle transitions, spec merging, task parsing, plan management) run through the `specify` CLI. Skills never hand-edit `.metadata.yaml` or manipulate the `.specify/` directory directly.

**Rationale:** LLM-interpreted prose rules for structured operations (validation, task parsing, directory manipulation) produced unreliable results. A binary that returns structured JSON and exit codes gives deterministic correctness where it matters, while the agent retains judgment for semantic decisions.

**Litmus test:** "Would this operation need to understand `.specify/` directory structure or spec format?" If yes, it belongs in the CLI. If no (like running `cargo test`), it stays with the agent.

**Source:** [RFC-1: specify CLI](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-1-cli.md)

## Pass/Fail/Deferred validation

**Decision:** The validation engine classifies checks into three outcomes: Pass (check passed), Fail (must fix), Deferred (requires semantic judgment, flagged for agent review).

**Rationale:** Some checks are purely structural (file exists, format correct) and can be answered definitively by the CLI. Others require understanding context ("is this design adequate?"). The three-way classification lets the CLI handle what it can and explicitly flags what needs agent judgment, rather than pretending everything is binary.

**Source:** [RFC-1a: Deferred Validation](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-1a-validation.md)

## Three independently useful layers

**Decision:** The system is structured in three layers (CLI primitives, change lifecycle, initiative orchestration), each independently useful. Higher layers invoke lower layers but lower layers are unaware of what sits above them.

**Rationale:** Not every use case needs automation. A single change needs only Layer 2. A small initiative can be driven manually with Layer 1 CLI commands. Full automation (Layer 3) composes on top without requiring the lower layers to change. This means you can always drop down a layer when automation fails.

**Source:** [RFC-2: Execution](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-2-execution.md)

## Plan as a data file, not a configuration

**Decision:** The plan (`plan.yaml`) is an ordered list of changes with status, not a pipeline configuration. There is no planning configuration file. The internal flow of `/spec:plan` is fixed.

**Rationale:** Configurability adds a debugging surface ("why did step X run?") before the system is well-understood. A fixed flow with no config is easier to reason about, and configurability can be added later without migration.

**Source:** [RFC-3a: Monolith Migration Planning](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-3a-monoliths.md)

## Analyze/extract split

**Decision:** Plan-time capability discovery (`/spec:analyze`) is separate from define-time deep extraction (`/spec:extract`). Analyze scans the whole source cheaply; extract runs deeply against a per-change slice.

**Rationale:** A large monolith cannot be fully extracted in one pass -- it would be too slow and expensive. The two-skill split makes large migrations tractable: cheap scanning builds the inventory, deep extraction happens per-change where it is focused and affordable.

**Source:** [RFC-3a: Monolith Migration Planning, Large-Monolith Decomposition](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-3a-monoliths.md)

## Registry-driven multi-repo planning

**Decision:** Multi-repo coordination uses a `registry.yaml` platform catalogue and an automatic sync-peers phase, not a configuration DSL or federation protocol.

**Rationale:** The same `/spec:plan <name>` command should work unchanged from one repo to 100+. The registry adds the minimum information needed (what repos exist, what schema they use, what domain they own). Sync-peers runs automatically when the registry has multiple projects, and not at all for single-repo work. No new user-facing concepts for the common case.

**Source:** [RFC-3a: Monolith Migration Planning](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-3a-monoliths.md), [RFC-3b: Platform Changes](https://github.com/augentic/specify/blob/main/rfcs/rfc-3b-platform.md)

## Stable requirement IDs as merge keys

**Decision:** Each behavioral requirement has a stable `ID: REQ-XXX` line that serves as the merge key across delta specs. Requirement titles may change; IDs must not.

**Rationale:** When specs evolve over multiple changes, the system needs a way to match "this modification applies to that requirement." Titles are human-facing and change frequently. Stable IDs give the merge engine a reliable key while keeping the spec format readable.

## Schema-agnostic lifecycle, schema-specific briefs

**Decision:** The lifecycle (states, transitions, four artifacts, baseline accumulation) is invariant across schemas. Schemas only control the *content* of brief pipelines and the specialist skills invoked during build.

**Rationale:** The workflow is the value -- define-build-merge, baseline accumulation, drift detection. Making this schema-agnostic means every project gets the same tooling regardless of target platform. Schemas customise the generation content without fragmenting the workflow.
