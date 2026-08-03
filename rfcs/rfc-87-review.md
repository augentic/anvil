# RFC-86 Review: Local Working Trees

> Reviewed: [rfc-86-working-trees.md](rfc-86-working-trees.md) (Draft), against the WIT contract (`wit/emery.wit`), the seam types (`crates/project/src/seam.rs`, `crates/adapter/src/seam.rs`), the `WorkingTree::live()` dispatch sites, the launcher mount policy (`crates/launcher/src/lib.rs`), the merge orchestration (`crates/slice/src/orchestrate/merge.rs`), and the sibling RFCs (platform.md, 87, 90, 91).
>
> Overall: the model is sound — values as the operation boundary, the closed cleanliness classification, and the phasing all fit the codebase's conventions, and the vocabulary (`lease recover`, changeset-vs-publication-set) is consistent with RFC-90/91. The findings below are mostly questions the first week of implementation will force; findings 1 and 2 should be resolved before Phase B starts.

## Findings

### 1. How a per-operation tree reaches an adapter guest is unspecified (the biggest gap)

The current deployment model fixes mounts at boot: `launcher::Policy` is evaluated once into a `OnceLock` and feeds `omnia::runtime!`'s mount expressions, and the WIT header is explicit that no directory handles cross the seam — "every guest opens its own `\".\"` preopen." The `working-tree` record's only routing mechanism is `subpath`, resolved beneath the shared mount by `Context::tree_root` in `crates/adapter/src/seam.rs`.

RFC-86 promises "immutable snapshot preopens" for sources and "leased writable-tree preopens" for targets, and `materialize` returning "a `wasi:filesystem` descriptor for deterministic guest code" — but never says how a tree that exists only after boot becomes guest-visible under a boot-time mount model. Three realistic mechanisms, with very different costs:

- **Mount a stable scratch *root* at boot and route via the existing `subpath`.** E.g. worktrees materialize under `.emery/scratch/` (already gitignored as "per-run working state" in `crates/project/src/registry/gitignore.rs`) or a dedicated mounted trees root, and `working-tree.subpath` names the lease. No WIT change, no Omnia change.
- **An Omnia capability for per-dispatch preopens** — a cross-repo dependency on `augentic/omnia` / `augentic/backends` that the RFC should name explicitly.
- **A WIT change** — which would contradict RFC-90's stated "the WIT seam does not change."

**Recommendation:** Add a decision choosing the mechanism — the scratch-root-plus-`subpath` route is the cheapest and preserves both the WIT contract and RFC-90's invariant — and add an **Ownership table** like RFC-90's. The work plausibly spans `augentic/emery` (launcher, seam, orchestration), `augentic/omnia` (only if dynamic preopens are chosen), and `augentic/emery-adapters` (every adapter's `tree_root` usage and the prompts that tell the spawned agent where to work). RFC-86 is where "the WIT seam does not change" gets proven or broken, so it should say which.

### 2. Uncommitted workflow state versus exact-base materialization

Slice artifacts (`spec.md`, `design.md`, `tasks.md`) are written by refine into `.emery/slices/<slice>/` and are typically uncommitted between operations. The build request ships project-relative `Payload::Path` values that the adapter resolves in its own preopen (`read_inputs` in `crates/slice/src/orchestrate/target.rs`). A worktree materialized from the *recorded base commit* will not contain those artifacts. The options:

- **Emery commits artifacts to `change/<plan>` before build** — Emery starts authoring Git commits, which needs a stated identity/message policy (nothing in the engine does this today; merge explicitly journals a *skipped* git leg — "the guest owns no git surface" in `crates/slice/src/orchestrate/merge.rs`).
- **Artifacts are copied into the leased tree out-of-band** — then `changes()` needs an explicit inclusion/exclusion policy so `.emery/` state does or doesn't ride the changeset.
- **`.emery/` stays outside the materialized tree** with separate routing — matches RFC-87's detached layout (change directory beside slots), but conflicts with today's in-place mode where `.emery/` lives inside the repo being built.

Related mechanical detail: `BuildReport::enforce_outputs_exist(layout.project_dir())` verifies declared outputs against the project dir; with a leased tree that check must anchor to the tree root instead.

**Recommendation:** Add a decision covering artifact injection. The out-of-band copy with an explicit `changes()` exclusion policy for `.emery/` paths is the least invasive and keeps the single-writer contract intact; if the commit route is chosen instead, the decision must also own the host-side Git identity (committer name/email) and message convention. Either way, list the `enforce_outputs_exist` re-anchoring in the Fixed implementation cut.

### 3. The in-place endgame: how merged work reaches the operator

The loop ends at `changes() → changeset → release worktree`, and D4 creates `change/<plan>` — but nothing states what *commits* to that branch, when, or with what identity. "Merge still owns folding the result into the baseline" leaves "the baseline" physically ambiguous post-RFC: the operator's checkout (which D3 forbids operating in), the mirror branch, or a fresh apply. Without this, "the serial loop wins today too" isn't demonstrable.

**Recommendation:** Spell out the serial merge choreography explicitly — presumably: materialize → apply the built changeset → preflight gate → deterministic commit → postflight gate → commit lands on `change/<plan>` in the mirror — and state how the operator sees the result before RFC-88 publication exists (does anything reach their checkout or origin, or does the operator inspect the mirror branch?). One paragraph under "The model" or a new decision would close it.

### 4. Non-Git target projects become impossible, silently

D7 covers the non-Git backend for *sources* only. D3 ("every writable tree is materialized" from a bare mirror) makes Git a hard requirement for any target project — a real behavior change; nothing in the engine requires the project to be a repository today.

**Recommendation:** State it as a hard cut with a named diagnostic (e.g. `working-tree-requires-git`, exit 2), consistent with the repo's no-compatibility-shim posture. Add it to the Fixed implementation cut and the acceptance criteria.

### 5. Closed surfaces left open (CLI verbs, diagnostics, journal events, `purpose`, lease ownership)

The repo's discipline is closed taxonomies everywhere (journal `EventKind`, kebab-case diagnostic codes, exact CLI grammars — RFC-87 gives its full command grammar). RFC-86 leaves several open:

- **CLI verbs.** `lease recover` is mentioned but not placed — `emery lease recover`? `emery tree …`? Which noun owns `inspect`, and what does it project? Acceptance criterion 1 says "through the public CLI/capability surface," so the surface should be named.
- **Diagnostic ids.** Only `plan-source-tree-overlap` gets a code. The three stopping classifications (`dirty-unaccounted`, `base-drifted`, `branch-diverged`) should map to named kebab-case codes and exit codes, plus `plan execute` stop reasons (parallel to the existing `merge-postflight-failed` convention).
- **Journal events.** D2 says "the journal carries the audit trail" — the events (lease acquired / released / recovered, materialized, changeset captured / applied) are new variants in the closed `EventKind` taxonomy and should be enumerated.
- **`ensure(project, requested-base, purpose)`** — `purpose` is never defined.
- **Lease owner identity.** For crash recovery to be meaningful, ownership must be a workflow identity, not a process handle. D2 says "the lease carries ownership" without saying what an owner *is*.

**Recommendation:** Add a "Surface" section (mirroring RFC-87's discovery-grammar section) that pins: the CLI noun and verbs (`ensure` is likely internal-only, `inspect` and `recover` operator-facing), the closed diagnostic-code set with exit codes, the new `EventKind` variants, the closed `purpose` enum (e.g. `build | merge | slot`), and the lease-owner identity as `(plan, slice)`.

### 6. Revision format for non-Git snapshots, and the stale WIT doc

The WIT `revision` comment currently describes the placeholder era ("names the state of the one live project tree… forward hook"), so it needs updating in this RFC's scope, along with deleting `WorkingTree::live()` and its three call sites (`crates/slice/src/handlers/build.rs`, `crates/change/src/plan/handlers/execute.rs`, `crates/slice/src/orchestrate/merge/gate.rs`). More substantively: for the non-Git directory-copy backend, what is the `revision` value? Changesets "carry the recorded base required for application," and Evidence provenance should be able to record the snapshot identity.

**Recommendation:** Pin the non-Git revision wire format as `sha256:<tree-digest>`, matching RFC-87's fingerprint convention and the existing `replay-digest: sha256:…` anchors. Add the WIT doc-comment update and the `WorkingTree::live()` removal to the Fixed implementation cut so they aren't missed.

### 7. Failure retention conflicts with unconditional worktree removal

`release(lease, outcome)` takes an `outcome`, but the Fixed implementation cut says releasing always removes the worktree. For a failed build, the scratch tree is often the only debugging evidence (the r9k failure mode RFC-90 cites).

**Recommendation:** Make retention explicit: a `failure` outcome retains the tree and its lease record for inspection (aligning with D5's "recovery keeps unaccounted changes intact"), a `success` outcome removes them, and retained trees fall under `emery archive prune`'s retention-policy GC. One line in the Fixed implementation cut plus an acceptance criterion.

### 8. Snapshot timing and retention for sources

When is a Git source pin resolved / a non-Git copy taken — at plan approval, per survey, or per extract? If per operation, evidence can drift between survey and extract, defeating the pinning; if per plan, the content-addressed scratch must be retained for the plan's life. The GC story for the read-only scratch copies is absent, and large non-Git source trees carry a real disk cost.

**Recommendation:** Pin snapshots once per plan (at plan authoring for in-place mode; RFC-87's approval already does this for detached mode) and retain the content-addressed scratch until `plan archive`. State that content addressing dedupes identical trees across plans, and put scratch GC under `emery archive prune`.

### 9. Phase A disjointness likely breaks a common today-binding

A `documentation` source bound to `docs/` *inside* the target repository is an overlap under any ancestor/descendant definition of "disjoint roots," and Phase A would reject that plan until Phase C lands — an interim regression for what is probably the most common multi-source shape.

**Recommendation:** Define overlap precisely (canonical-root equality or ancestry after symlink resolution) and scope Phase A's restriction to same-*repository* roots rather than same-path — a source subdirectory of the target repo can be snapshot-copied in Phase A without the full Phase C machinery. If the restriction must stay path-based, acknowledge the interim break explicitly and name the workaround.

### 10. Mirror-level concurrency and the Git implementation choice

Lease scope is per-tree, but the shared bare mirror itself needs a lock for concurrent fetch / `worktree add` — RFC-90 Stage C leans directly on this. Separately, the RFC should pin how Git is driven: shelling out to system `git` versus a Rust library (gix/git2). The engine currently has zero Git machinery, so this is a new dependency with consequences: credential handling for private-repo mirror sync (system credential helpers argue for shell-out; RFC-87's forge provider already uses operator credentials), `cargo vet` / `deny` supply-chain review for a library, and crate placement — it must live host-side (`launcher` / `native`) with the deployment-neutral capability trait in `project::seam` beside the existing ones, since the engine crates compile to wasm.

**Recommendation:** Add one sentence to the Fixed implementation cut giving the mirror its own advisory lock (independent of tree leases). Add a decision pinning the Git driver; shelling out to system `git` is the pragmatic choice for credential inheritance and supply-chain surface, and matches the host-owned-native-code posture already in D7. Name the crate placement: capability trait in `project::seam`, implementation in `launcher` or `native`.

### 11. Smaller notes

- **Live rung absent from acceptance.** Criterion 7 is all local fixtures — appropriate for the mechanics, but the RFC changes the environment the spawned agent runs in. **Recommendation:** add the operator-invoked wasm/eval rung as a completion gate, the way RFC-90's fixed cut does.
- **`inspect(project)` has no stated return shape.** **Recommendation:** sketch it — per-tree: lease owner, recorded base, branch, cleanliness class.
- **D6's escape hatch is vague.** "Pinned to the same commit unless its approved bindings say otherwise" is the only open clause in an otherwise closed policy. **Recommendation:** state exactly what an approved binding can override (presumably only a per-source ref), or drop the clause.

## What to keep as-is

- The temporary-index `changes()` design (untracked / empty / binary coverage).
- The three-proceed / three-stop cleanliness classification with explicit recovery.
- Values-as-boundary (D1) as the settled contract RFC-90/91 build on.
- The mirror-as-cache posture: revision + changeset carry continuity, the mirror is disposable.
