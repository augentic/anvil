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

## CWD-based routing for multi-repo execution

**Decision:** The execute driver routes each change to its target project by changing working directory to the workspace clone before invoking phase skills. Phase skills (`/spec:define`, `/spec:build`, `/spec:merge`) are completely unaware of multi-repo routing -- they run unmodified in whatever directory the driver places them in.

**Rationale:** The alternative (passing a `--project` flag through to every phase skill) would have required changes to every skill and every brief pipeline. CWD-based routing keeps the routing decision in one place (the driver) and preserves the invariant that phase skills operate on "the current project." Phase skills discover the schema via their normal `.specify/project.yaml` walk from CWD.

**Source:** [RFC-3b: Platform Changes, §Execution routing](https://github.com/augentic/specify/blob/main/rfcs/rfc-3b-platform.md)

## One change, one project

**Decision:** Each plan change targets exactly one registry project. Capabilities that span multiple repos are decomposed into separate plan entries (one per project) linked by `depends-on` edges.

**Rationale:** Allowing a single change to span repos would require the execution loop to manage multiple project roots, multiple schemas, and multiple baseline merge targets within one define-build-merge cycle. Decomposing cross-cutting capabilities into per-project entries keeps the loop simple and matches the existing baseline-accumulation model where each merge has a single target.

**Source:** [RFC-3b: Platform Changes, §One change, one project](https://github.com/augentic/specify/blob/main/rfcs/rfc-3b-platform.md)

## Project assignment is a framework concern

**Decision:** Inferring which registry project each plan entry targets (the assignment step) runs in the plan skill at the framework level, not inside schema-owned propose briefs. Propose creates entries without `--project`; the plan skill's assignment step writes the routing after propose completes.

**Rationale:** A multi-repo plan spans projects with different schemas, so assignment is inherently a cross-schema concern. Placing it in individual propose briefs would duplicate the logic across schemas and create an ordering problem (the brief would need to know about projects it does not own). Keeping it in the plan skill also means propose briefs are unchanged -- a single-repo propose brief works identically in a multi-repo plan.

**Source:** [RFC-3b: Platform Changes, §Assignment algorithm](https://github.com/augentic/specify/blob/main/rfcs/rfc-3b-platform.md)

## Workspace-centric execution with explicit push

**Decision:** All multi-repo execution happens inside workspace clones under the initiating repo's `.specify/workspace/`. Local commits from merge accumulate in the clones. Changes are published to remotes only when the operator explicitly runs `specify workspace push`.

**Rationale:** Automatic pushes during execution would make the driver non-idempotent and create a rollback problem -- a failed change that was already pushed cannot be cleanly undone. Keeping pushes explicit gives the operator a review gate between "execution produced artifacts" and "artifacts are published." The workspace is the staging area; `workspace push` is the release gate.

**Source:** [RFC-3b: Platform Changes, §Workspace-centric execution](https://github.com/augentic/specify/blob/main/rfcs/rfc-3b-platform.md)

## Stable requirement IDs as merge keys

**Decision:** Each behavioral requirement has a stable `ID: REQ-XXX` line that serves as the merge key across delta specs. Requirement titles may change; IDs must not.

**Rationale:** When specs evolve over multiple changes, the system needs a way to match "this modification applies to that requirement." Titles are human-facing and change frequently. Stable IDs give the merge engine a reliable key while keeping the spec format readable.

## Schema-agnostic lifecycle, schema-specific briefs

**Decision:** The lifecycle (states, transitions, four artifacts, baseline accumulation) is invariant across schemas. Schemas only control the *content* of brief pipelines and the specialist skills invoked during build.

**Rationale:** The workflow is the value -- define-build-merge, baseline accumulation, drift detection. Making this schema-agnostic means every project gets the same tooling regardless of target platform. Schemas customise the generation content without fragmenting the workflow.
