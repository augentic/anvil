# Discovery (step 3a) and greenfield registry bootstrap

## Step 3(a) — Discovery

Step 3(a) invokes the discovery brief bundled with this skill at `briefs/<capability>/discovery.md` (for Omnia, [`briefs/omnia/discovery.md`](briefs/omnia/discovery.md); for Vectis, [`briefs/vectis/discovery.md`](briefs/vectis/discovery.md); other capabilities ship their own variant alongside). Planning is orchestration, not capability-owned slice work, so planning briefs live with this skill rather than in `capability.yaml`. Discovery consumes the `from`, `against`, and `source` inputs, dispatching each input per its `kind` through `/change:analyze` (both `documentation` and `legacy-code` branches), and merges the results into a single neutral capability inventory at `.specify/plans/<change-name>/discovery.md`. The skill's job is to faithfully run the brief and pass inputs through; the algorithm (per-input handling, dedup rules, ordering) lives in the brief — see [`briefs/omnia/discovery.md`](briefs/omnia/discovery.md) for the authoritative contract.

Discovery is read-only with respect to `plan.yaml`. The output header is exactly `# Discovery — <change-name>` with no timestamps, run IDs, or working-directory paths, and re-running discovery on unchanged inputs MUST produce byte-equivalent output — the brief owns the ordering, the skill does not impose its own. An existing `discovery.md` is overwritten unless `extend` is set (see [SKILL.md](SKILL.md) §"Modes → `extend`"). The shape of a single-source inventory against a small pre-seeded source tree is pinned by `fixtures/discovery/expected-discovery.md` against `fixtures/discovery/legacy/`.

## Greenfield discovery → initial registry topology

When `/change:plan` runs **before** any `registry.yaml` exists and the discovery brief surfaces capabilities that cluster into more than one project (e.g. a TypeScript backend monolith with a separate Crux mobile shell, or two unrelated services in a single migration), the skill enters the **greenfield registry-bootstrap** flow between discovery (step 3(a)) and propose (step 3(c)). Single-project greenfield runs (`from <single-codebase>`, no clear cluster split) skip this flow and continue as a standard single-repo change.

**Detection.** The discovery brief decides whether the capability inventory implies more than one project. The brief writes a `## Proposed registry topology` section to `.specify/plans/<change-name>/discovery.md` whenever its clustering produces ≥ 2 candidate projects. Absence of that section (or a section with exactly one candidate) means single-repo — the bootstrap flow is skipped.

**Normative sequence**

1. Read the `## Proposed registry topology` block from `discovery.md`. The block lists each candidate project as `### <name>` with bullets for `url`, `capability`, and `description` — same shape `specify registry add` consumes.
2. Present the full topology table to the operator in a single batch review:

   ```markdown
   ## Proposed registry

   | # | Name | URL | Capability | Description |
   |---|---|---|---|---|
   | 1 | <name-a> | <url-a> | <capability-a> | <description-a> |
   | 2 | <name-b> | <url-b> | <capability-b> | <description-b> |
   ```

   The operator approves each row, edits any field, or rejects the entry. Rejection of a candidate drops it from the bootstrap; capabilities that would have routed to that project surface as `unresolved` during step 3(d) Assignment and re-enter the registry-proposal sub-step (3(d).1) one at a time.
3. For each accepted row, shell out **once**:

   ```text
   specify registry add <name> --url <url> --capability <capability> --description "<description>"
   ```

   Multi-project registries enforce the `description-missing-multi-repo` invariant — the description is required. Defer ordering to the discovery brief (alphabetical by `name` is the conservative default).
4. After the last accepted entry, run **once**:

   ```text
   specify workspace sync
   ```

   Materialise every new slot under `.specify/workspace/<name>/`. Then proceed to step 3(b) Sync workspace (which now has a multi-project registry to inventory) and step 3(c) Propose.
5. **Plan scaffold ordering.** Step 2 (`specify change plan create`) MUST run **before** the bootstrap flow — `plan.yaml` is independent of `registry.yaml`. Use the post-3.5 verb `specify change plan create <name>` (NOT the retired `specify change plan create` or the v1 `plan init`). The bootstrap flow runs between step 3(a) discovery and step 3(b) sync-workspace; it never re-creates the plan.

**`--dry-run`.** Render the `Proposed registry` table to stdout with a `Would create registry entries:` heading; do **not** shell `specify registry add` or `specify workspace sync`. The discovery brief still runs (so the clustering preview is real); only the writing path is suppressed.

**Single-project greenfield.** When the discovery brief writes no `## Proposed registry topology` section (or a section with exactly one entry), the skill proceeds as a single-repo change — no `registry.yaml` is created, no `specify workspace sync` is run, and step 3(d) Assignment is skipped per its existing single-project rule.
