# Assignment and registry proposal (step 3d, multi-repo only)

After the propose brief completes step 3(c) and all accepted entries have been written to `plan.yaml` (without `project`), the plan skill runs the assignment pass when `workspace.md` is present and contains more than one project section. Single-project registries skip this step entirely.

## Step 3(d) — Assignment

**Normative sequence**

1. Read all entries created by the propose brief — the entries with `status: pending` and no `project` field.
2. For each entry, infer a project assignment using the following signal priority:
   - **Description match.** Compare the entry's `description` against each project's `Description` bullet in `workspace.md`. Domain-term overlap is the primary signal.
   - **Baseline spec affinity.** If a peer already has baseline specs whose names or domains overlap with the entry, that peer is a strong candidate. This signal is only available for brownfield (materialised workspace with existing specs listed in the `Specify tree` bullet).
   - **Schema compatibility.** If the entry's nature (e.g. UI vs backend logic) aligns with only one schema type in the registry (via the `Schema` bullet), use that as a tiebreaker.
   - **Ambiguity → human.** When no signal clearly differentiates, or when confidence is low, surface the assignment as "unresolved" and require operator input. Never silently assign a low-confidence match.
3. Present the full assignment table to the operator in a batch review:

   ```markdown
   ## Assignment

   | # | Entry | Project | Rationale |
   |---|---|---|---|
   | 1 | ingest-pipeline | traffic | description overlap: ingestion, Kafka |
   | 2 | operator-dashboard | command-centre | baseline spec: user-alerts exists |
   | 3 | shared-types | ? | ambiguous: matches both projects |
   ```

   The operator reviews the table and can override any assignment. For **unresolved** assignments (`?`), the operator must assign a project before the step can proceed. The `project` prompt is a pick-from-list field — the legal values are the project names from `workspace.md`. Invalid input re-prompts.

4. For each entry, shell out to:
   ```text
   specify change plan amend <name> --project <project>
   ```
5. Append the assignment table (with final assignments and rationale) to `proposal.md` so the proposal reconstructs the full decision trail — decomposition (from the propose brief) followed by routing (from the assignment step).

When the registry is absent or single-project, step 3(d) is skipped entirely. No `--project` is written to plan entries.

## Step 3(d).1 — Registry proposal sub-step (RFC-9 §2B)

Step 3(d) routes entries to **existing** registry projects. When the operator's response in step 3(d).3 names a project that does **not** exist in `registry.yaml`, the registry-proposal sub-step is invoked **before** continuing assignment. This sub-step is the only place `/change:plan` ever calls `specify registry add`.

**Trigger.** An unresolved (`?`) row in the assignment table where the operator types a project name that is not present in `registry.yaml:projects[].name` (case-sensitive, exact match).

**Normative sequence**

1. **Confirm.** Prompt the operator with the exact line:

   ```text
   Project `<name>` does not exist in registry.yaml. Create it now? [y/N]
   ```

   Default is **N** (decline). On decline, surface a follow-up prompt asking the operator either to (a) name an existing project from the registry, or (b) drop the entry via `specify change plan transition <name> skipped --reason "<reason>"` outside the loop. The skill never auto-skips a low-confidence assignment.

2. **Gather defaults.** On accept:
   - **`--url`.** Default to `git@github.com:<org>/<name>.git` where `<org>` is inferred from the longest common `<host>:<org>/` prefix across `registry.yaml:projects[].url` entries (when the registry already has at least one entry with that prefix). If no prefix can be inferred (e.g. greenfield registry, or every existing entry uses a different host or path layout), prompt the operator to supply the URL by hand. The operator can always override the suggested default.
   - **`--schema`.** Default to the schema used by the **majority** of existing registry entries; on a tie, prompt with the legal candidates. If the registry is empty, prompt with the canonical schema list (`omnia@v1`, `vectis@v1`, `contracts@v1`, `hub` — the closed enum from `Registry::validate_shape`). Bail with a hard exit on an empty response or an unknown value.
   - **`--description`.** Required when the addition produces a multi-project registry (`description-missing-multi-repo` invariant from RFC-3b). The operator supplies free-form prose; the skill never paraphrases.

3. **Apply.** Shell out **in this exact order**:

   ```text
   specify registry add <name> \
       --url <url> \
       --schema <schema> \
       [--description "<description>"]

   specify workspace sync
   ```

   Both calls are blocking. Surface a non-zero exit from `specify registry add` (e.g. `description-missing-multi-repo`) verbatim and abort the sub-step — the operator may retry with `--description "<...>"` or decline. Treat a non-zero `specify workspace sync` (e.g. clone failure on the new slot) as a hard failure that leaves the registry edit in place (atomicity is at the verb level — `specify registry remove <name>` can roll it back if needed).

4. **Continue assignment.** Re-render the assignment table with the freshly-added project highlighted in the legal-values list, then re-prompt for any remaining unresolved rows. The accepted row's project field is set to `<name>` and the skill shells out:

   ```text
   specify change plan amend <name> --project <name>
   ```

   The skill never bundles the registry-add and the plan-amend into one step — the registry is the producer of legal project names; the plan is the consumer. Two writes, two verbs, in that order.

5. **Audit trail.** Append a `Registry amendments` block to `.specify/plans/<slice-name>/proposal.md` listing every `(name, url, schema, description)` tuple created during the run, plus the rationale (the entry that triggered the proposal). The block sits after the assignment table.

**`--dry-run`.** Do **not** shell `specify registry add` or `specify workspace sync`; do **not** invoke `specify change plan amend`. The dry-run output emits a `Would propose registry amendments:` block listing the same `(name, url, schema, description)` tuples the writing path would have created, plus the rationale per entry. The block sits after the assignment preview.

**`--extend`.** Same as default, with one wrinkle: an `--extend` run can land additional entries that route to a project added in an earlier `/change:plan` run. When the assignment table names a project that **does** exist in `registry.yaml`, no proposal sub-step runs (the project is already legal). The proposal sub-step only fires for genuinely new project names.

## Contract role population

When `/change:plan` inserts a contract slice for an API boundary between projects, it populates the `contracts` block on the relevant registry project entries:

1. **Producer project**: Add the contract file paths to `contracts.produces` on the project that implements the API.
2. **Consumer project**: Add the contract file paths to `contracts.consumes` on the project that calls the API.

RFC-12 collapsed the role set to these two. A contract that no project lists under `contracts.produces` is — by definition — externally authored; do not invent a separate field to mark it. Consumer projects that integrate with an external system still appear under `contracts.consumes`, exactly as they would for an internally-produced contract.

Use `specify registry validate` to verify the invariants after populating roles. The validation is advisory — the operator can adjust role assignments.
