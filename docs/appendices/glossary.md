# Glossary

Canonical definitions for terms used throughout Specify.

## A

**Artifact**
One of the four structured documents that define a change: `proposal.md`, `spec.md`, `design.md`, `tasks.md`. Artifacts are the contract between human intent and agent execution.

**Archive**
The `.specify/archive/` directory where finalized changes (merged or dropped) and completed plans are stored for audit.

## B

**Baseline**
The accumulated set of merged specs at `.specify/specs/`. Represents the current known behavioral state of the system. Future changes produce deltas against the baseline.

**Brief**
A markdown prompt file provided by a schema that drives artifact generation. Briefs are organized into pipelines for each phase (define, build, merge).

**Brief pipeline**
An ordered sequence of briefs declared by a schema for a given phase. The define pipeline typically runs: proposal, specs, design, tasks.

## C

**Capability**
A discrete unit of system behavior that gets its own spec file. In the Omnia schema, capabilities typically correspond to crates. In the Vectis schema, they correspond to features.

**Change**
A unit of work in Specify, stored at `.specify/changes/<name>/`. Contains the four artifacts and a `.metadata.yaml` file tracking lifecycle state.

## D

**Define**
The first phase of the change lifecycle. Generates all artifacts from a description, optionally enriched by source code extraction.

**Delta spec**
A spec that describes modifications to an existing capability using `ADDED`, `MODIFIED`, `REMOVED`, and `RENAMED` sections. Delta specs merge into the baseline by matching on stable `REQ-XXX` IDs.

**Discovery**
The output of `/spec:analyze` during plan authoring. A `discovery.md` file containing capability summaries (name, description, source files, dependencies, confidence) derived from input analysis.

## E

**Execute**
The Layer 3 skill (`/spec:execute`) that automates the define-build-merge loop for each entry in a plan, in dependency order.

**Extract**
The process of deriving behavioral specs and design from existing source code, performed by `/spec:extract`. Produces language-agnostic artifacts.

## I

**Initiative**
A multi-change program coordinated through a plan. Examples: a migration, a greenfield build, a platform modernisation.

## L

**Layer 1 (CLI primitives)**
The `specify` CLI commands that handle all deterministic operations. The foundation that skills build on.

**Layer 2 (Change lifecycle)**
The `/spec:*` skills that operate on a single change: define, build, merge, drop, verify, explore, extract, status, init.

**Layer 3 (Initiative orchestration)**
The `/spec:*` skills that coordinate multi-change programs: plan, execute, analyze.

**Lifecycle state**
The current status of a change: `created`, `defined`, `building`, `complete`, `merged`, or `dropped`. Managed by the CLI via `.metadata.yaml`.

## M

**Merge**
The third phase of the change lifecycle. Applies spec deltas to the baseline and archives the change.

**Merge key**
The stable `ID: REQ-XXX` line in a spec requirement. Used to match delta spec operations to baseline requirements during merge.

## P

**Phase outcome**
A classification (`success`, `failure`, `deferred`) written to `.metadata.yaml` after a phase completes. Used by `/spec:execute` to determine whether to transition a plan entry to `done`, `failed`, or `blocked`.

**Plan**
An ordered, dependency-aware list of changes stored in `.specify/plan.yaml`. The initiative's table of contents.

**Plugin**
A Cursor marketplace package that provides skills, rules, and references for a specific domain (Specify, Omnia, Vectis, RT, Plan).

**Proposal**
The first artifact generated during define. Captures why the change exists, what is in scope, and which capabilities are affected.

## R

**Registry**
`.specify/registry.yaml` -- a platform catalogue declaring the repos in a multi-repo system. Each entry has a name, URL, schema, and domain description.

**Requirement ID**
A stable identifier (`REQ-001`, `REQ-002`, ...) assigned to each behavioral requirement in a spec. Serves as the merge key across delta specs.

## S

**Schema**
A configuration package that tells Specify how to generate artifacts and build code for a specific target platform. Contains brief pipelines and domain context.

**Skill**
An agent-driven orchestrator invoked with a slash-command prefix (e.g. `/spec:define`, `/omnia:crate-writer`). Skills delegate deterministic work to the CLI and use judgment for everything else.

**Skill directive tag**
An HTML comment in `tasks.md` (e.g. `<!-- skill: omnia:crate-writer -->`) that routes a task to a specific specialist skill during build.

**Spec**
A behavioral specification at `specs/<capability>/spec.md`. Contains requirements with stable IDs, scenarios (WHEN/THEN), error conditions, and optional metrics.

**Sync peers**
The phase during `/spec:plan` (multi-repo only) that clones registry projects into `.specify/workspace/` and inventories their baseline specs. Produces `workspace.md`.

## V

**Verify**
The read-only skill (`/spec:verify`) that compares code against baseline specs to detect drift. Classifies requirements as COVERED, DRIFTED, MISSING, or UNSPECIFIED.

## W

**Workspace**
`.specify/workspace/<project>/` -- read-only clones of registry projects, created during sync-peers for cross-repo planning.
