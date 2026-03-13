# Improvement Plan

## Session 2: Abandon + requirement IDs

Builds on the stabilized format. Start with a quick win (abandon), then tackle the big design piece (requirement IDs).

### Add /spec:abandon for change cleanup

There's no way to cleanly discard a change. If someone starts a propose and decides to scrap it, or if apply hits an unrecoverable blocker, the change sits in `.specify/changes/` forever. Add an abandon action that moves the change to archive with `status: abandoned` and no spec merge. This is simple to implement and immediately useful.

### Add stable requirement IDs for traceability

Right now, requirements are identified by their heading text (`### Requirement: OAuth login`). This means:

- Renaming a requirement breaks all references
- There's no way to trace requirement → design → task → test → code
- Archive's RENAMED operation exists but is fragile

Add a stable ID scheme (e.g., `REQ-001` or a short hash) that persists through the lifecycle. The heading is the display name; the ID is the stable reference. Design sections can cite `REQ-001`, tests can annotate `#[test_for("REQ-001")]`, and archive can use IDs for matching instead of heading text matching. This is a big investment but it unlocks automated traceability checking — which is the killer feature of spec-driven development.

---

## Session 3: Validation + schema pinning

Additive safety nets. None changes the core workflow — they add checks around it. Lower risk, can be done in any order.

### Validate artifacts with structured checks

Add a lightweight review action:

- Reads all artifacts and runs all validate rules
- Optionally runs cross-artifact consistency checks (do all proposal crates have spec files? does design reference all spec requirements?)
- Produces a review summary
- Sets status to reviewed (a new state between proposed and applying)

It creates a natural place for the antagonist pattern from agent-teams.md to apply.

Introduce machine-checkable validators: convert validate strings to structured checks (regex/rules/functions) and run them in CI + skill runtime.

It should strictly enforce the `spec_format` defined in the schema. If a spec file doesn't match the expected structure (e.g., missing a `#### Scenario:` under a Requirement), the tool should reject it and ask the user (or agent) to fix the format before attempting a merge.

### Add post-archive baseline validation

After archive merges deltas into baseline, run a coherence check on the entire baseline:

- No duplicate requirement names within a spec file
- No orphaned references (requirement mentioned in design that doesn't exist in specs)
- Heading structure is valid per spec_format

This catches the case where two separately-valid changes produce an incoherent baseline when merged sequentially.

### Deterministic schema pinning

Schema pinning is not deterministic when a local schema folder exists. Resolution prefers local `schemas/<name>` before cache/remote, even when config uses URL+@ref. Good for local dev, bad for strict reproducibility across machines/branches.

---

## Deferred

Items that are valuable but not urgent. Revisit after sessions 1-3 are complete.

### Make archive merge deterministic (not LLM-interpreted)

Defer until after requirement IDs are in place — ID-based matching makes the merge much simpler to build. The archive merge algorithm permanently modifies baseline specs. A small script or tool that parses baseline and delta specs using the `spec_format` from `schema.yaml`, applies RENAMED → REMOVED → MODIFIED → ADDED deterministically, and reports structured errors would make this testable and reliable. The archive skill would invoke this tool rather than performing the merge itself.

### "Smart" Task Execution

Problem: `apply.md` is a static, monolithic instruction file. Solution: Make `tasks.md` more semantic. Instead of just text descriptions, tasks could include "directives" or tags that map to specific skills.

Current: `- [ ] Create the user crate`
Future: `- [ ] Create the user crate <!-- skill: omnia:crate-writer -->` The `/spec:apply` skill could then parse these tags and dynamically invoke the correct specialist skill, rather than relying on a long text prompt in `apply.md` to guide the process.

### Drift Detection (/spec:verify)

Problem: Over time, code may drift from the specs if changes are made without going through the workflow. Solution: Add a `/spec:verify` command (or skill) that uses the code-analyzer (from the rt plugin) to reverse-engineer the current code behavior and compare it against the baseline specs in `.specify/specs/`. This closes the loop, ensuring that the "Source of Truth" remains truthful.
