# RFC-2 Review: Open Issues

> Companion to [RFC-2: Feature Manifests](rfc-2-manifests.md). Each issue is self-contained — resolve them in any order.

---

## ~~3. Extract-to-define handoff is undefined~~ — Resolved

**Resolution:** There is no separate EXTRACT step in the loop. `/spec:extract` is invoked *by* `/spec:define` when a change has `sources`. Define is always the entry point; extract is its source-analysis mechanism. Cross-stack define is a mode of define (sources in one language, target schema in another), not a separate skill. See updated loop diagram and Migration Mode section in [RFC-2](rfc-2-manifests.md).

---

## 6. Manifest mutation and crash safety

**Severity:** Medium — affects reliability of long-running initiatives.

The loop updates the manifest after merge (`status → done`). But:

- **Ownership.** Who writes the update — the merge skill, the orchestrator, or the CLI?
- **Crash recovery.** If the process dies after merge but before manifest update, the manifest is out of sync. Is the manifest the source of truth, or can `specify manifest status` reconstruct it from Specify state?
- **Concurrent access.** If two agents work on independent features in the same manifest, who serialises writes?

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