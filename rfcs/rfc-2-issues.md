# RFC-2 Review: Open Issues

> Companion to [RFC-2: Feature Manifests](rfc-2-manifests.md). Each issue is self-contained — resolve them in any order.

---

## 1. Manifest filename inconsistency

**Severity:** Trivial — but blocks implementation until resolved.

The RFC uses two different names for the same file:

- RFC-2 line 28: `# .specify/features.yaml`
- RFC-2 New Capabilities table: `Feature manifest (manifest.yaml)`
- Roadmap: `manifest.yaml`

**Resolution needed:** Pick one name and use it consistently across RFC-2, the roadmap, and RFC-1 Phase 2.

---

## 2. The orchestrator skill has no design

**Severity:** High — this is the most complex new component.

The "Manifest orchestrator" appears in the New Capabilities table as: *"Reads the manifest, selects the next pending feature, wires the loop."* But it's the piece that ties everything together and has several unresolved mechanics.

### 2a. Interactive skill problem

The existing skills (define, build, merge) are designed for human interaction. They use `AskQuestion` to confirm change names, warn about incomplete artifacts, preview merge operations, and ask whether to proceed. An orchestrator looping through features would need to either:

- Run non-interactive variants of each skill
- Pre-answer the confirmations programmatically
- Accept that the loop is semi-automated (human confirms each step)

The RFC doesn't say which model it envisions.

### 2b. Failure and resumption

If a feature fails mid-build (tests don't pass, design turns out wrong), what happens?

- Does the feature go back to `pending`?
- Is there a `failed` or `blocked` status?
- Can the orchestrator skip it and advance to the next independent feature?
- What happens to the half-created Specify change — is it dropped?

### 2c. Context threading

How does the orchestrator pass context between extract, define, build, and merge for a single feature? Today the user manually invokes each skill in sequence. The orchestrator needs to know the change directory, the resolved schema, which spec files were created, etc.

---

## 3. Extract-to-define handoff is undefined

**Severity:** High — required for migration mode.

The loop shows `EXTRACT → DEFINE → BUILD → MERGE`, but the extract→define transition mechanism is missing.

- The extract skill writes language-agnostic artifacts to a change directory. The define skill creates a *new* change from a human description. Which model applies when the manifest drives the loop?
- Does extract create the change, and define operates on the *same* change? Or does extract produce intermediate output that define consumes?
- The "Cross-stack define" is listed in New Capabilities as a separate Skill: *"Extract from one stack (e.g. TypeScript) and define against another (e.g. Omnia/Rust)."* Is this a new skill, or a mode of the existing define skill? What are the inputs, outputs, and how does it differ from a normal define?

---

## 4. Feature status vocabulary has gaps

**Severity:** Medium — affects state machine correctness.

The manifest status values are `migrated | in-progress | pending | skipped`. Issues:

- **`migrated` is migration-biased.** For greenfield features that were never extracted, "migrated" is semantically wrong. Consider `done` or `complete`.
- **No failure state.** If a feature fails during build, there's no `failed` or `blocked` status.
- **`skipped` is unexplained.** Appears in the YAML example but the text never describes when or why a feature would be skipped, or how to un-skip it.
- **`in-progress` has no entry/exit rules.** When does `pending` → `in-progress` happen? When the change is created? When extract starts? When define starts?

**Resolution needed:** Define the complete feature state machine with transitions, including failure and skip paths.

---

## 5. Feature-to-change mapping is implicit

**Severity:** Medium — affects both the orchestrator and status tracking.

The manifest operates on *features*; Specify operates on *changes*. The RFC implies 1:1 but doesn't specify:

- How the change name is derived from the feature name (same? prefixed?)
- Whether a feature can span multiple changes if it's too large
- Whether a feature can produce multiple capabilities within a single change
- How to correlate manifest feature status (`migrated`) with Specify change lifecycle status (`merged`) — they use different vocabularies

---

## 6. Manifest mutation and crash safety

**Severity:** Medium — affects reliability of long-running initiatives.

The loop updates the manifest after merge (`status → migrated`). But:

- **Ownership.** Who writes the update — the merge skill, the orchestrator, or the CLI?
- **Crash recovery.** If the process dies after merge but before manifest update, the manifest is out of sync. Is the manifest the source of truth, or can `specify manifest status` reconstruct it from Specify state?
- **Concurrent access.** If two agents work on independent features in the same manifest, who serialises writes?

---

## 7. No per-feature schema override

**Severity:** Low — may be intentional.

The manifest has `target-schema: omnia@v1` at the top level. Real initiatives might have features targeting different schemas (e.g., an Omnia crate and a Vectis app). There's no per-feature `schema` field.

**Resolution needed:** Decide whether one manifest = one target schema (document this constraint) or add an optional per-feature override.

---

## 8. Behavioural diff — listed but not designed

**Severity:** Low — doesn't block the core loop.

"Behavioural diff" appears in New Capabilities: *"Compare legacy fixture output against new implementation output (migration mode)."* No design exists for:

- How it finds fixtures (wiretapper output? manually placed?)
- Comparison semantics (exact match? structural? fuzzy?)
- Handling expected differences (field ordering, timestamps, UUIDs)
- Whether it integrates with replay-writer tests or is a separate concern

---

## 9. `specify manifest init` — practical limitations

**Severity:** Low — the RFC already positions this as a draft-quality tool.

The structural discovery design is clear and well-scoped, but worth noting:

- **Regex-based import parsing** will miss dynamic imports, barrel files, dependency injection, and runtime-resolved modules. The RFC says *"handles the common cases"* but doesn't describe behaviour when the graph is incomplete.
- **Infrastructure classification** heuristic ("imported by most, imports none") will misclassify in codebases with bidirectional utility dependencies.
- **Language detection** isn't specified for multi-language repos (common in migration: TypeScript frontend + Go backend in the same monorepo).

None block implementation — the tool produces a draft for human refinement — but expectations should be set.

---

## 10. RFC-1 dependency

**Severity:** Low — pragmatic path exists.

RFC-1 has zero implementation (no `Cargo.toml`, no `crates/`). The manifest CLI commands are designed as RFC-1 subcommands.

However, the manifest *format*, the *orchestrator skill*, and even *manifest parsing* could be prototyped as skills or scripts before the CLI exists. The pragmatic path: define the YAML schema, build the orchestrator skill, prototype CLI commands as scripts, and port to the CLI when RFC-1 lands.

The risk is throwaway work if RFC-1's module structure changes, but the manifest logic (YAML parsing, topological sort, status tracking) is self-contained enough to survive a CLI refactor.

---

## Not flagged (considered fine)

- **Progressive baseline accumulation** — well-described; the merge skill already does this.
- **Dependency-aware ordering** — topological sort on `depends-on` is straightforward.
- **The loop itself** — extract → define → build → merge is proven for individual features.
- **Multi-source support** — the `sources` map is a simple name→path lookup.
- **Layer 2 feature recommender** — explicitly optional, clearly scoped.
- **Integration with RFC-3** — correctly deferred; the vague interface is appropriate for a draft.
