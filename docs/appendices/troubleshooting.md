# Troubleshooting

Common failure modes and their resolutions.

## Change lifecycle issues

### "Change not found"

**Symptom:** A skill reports that no change exists or cannot find the specified change.

**Cause:** The change name is misspelled, or `/spec:init` has not been run.

**Resolution:**
1. Check active changes: `specify change list`
2. Verify `.specify/` exists. If not, run `/spec:init`.

### "Change not in expected state"

**Symptom:** A skill refuses to proceed because the change is in the wrong lifecycle state (e.g. trying to build a change that is not yet defined).

**Cause:** A previous phase did not complete, or the change was manually transitioned.

**Resolution:**
1. Check the state: `specify change status <name>`
2. Complete the missing phase (e.g. run `/spec:define`) or manually transition: `specify change transition <name> <target>`

### Artifacts incomplete after define

**Symptom:** `/spec:build` reports missing artifacts even though `/spec:define` appeared to complete.

**Cause:** Define may have encountered an error mid-pipeline and not generated all artifacts.

**Resolution:**
1. Check which artifacts exist in `.specify/changes/<name>/`.
2. Re-run define to regenerate: `/spec:define <name>` or regenerate a specific artifact: `/spec:define <name> <artifact-id>`

## Merge issues

### Baseline conflict detected

**Symptom:** `/spec:merge` fails with a conflict-check error.

**Cause:** The baseline changed since the change was defined (another change was merged in between).

**Resolution:**
1. Review the conflict: `specify spec conflict-check <change-dir>`
2. Options:
   - Re-run `/spec:define` to update specs against the current baseline.
   - Manually resolve conflicts in the spec files.
   - Drop and redefine: `/spec:drop`, then `/spec:define` with updated description.

### Coherence failure after merge

**Symptom:** `specify merge` fails during coherence validation.

**Cause:** The merged baseline has structural issues (e.g. duplicate requirement IDs, broken references).

**Resolution:**
1. Review the error message for the specific coherence issue.
2. Fix the spec files in the change directory.
3. Retry: `/spec:merge`

## Plan and execution issues

### Lock held by another process

**Symptom:** `/spec:execute` reports that `.specify/plan.lock` is held.

**Cause:** Another `/spec:execute` session is running, or a previous session crashed without releasing the lock.

**Resolution:**
1. Check the lock: `specify plan lock status`
2. If the PID is not running, release it: `specify plan lock release`
3. If another session is running, wait for it to finish.

### Self-heal on startup

**Symptom:** `/spec:execute` reports a self-heal operation when starting.

**Cause:** A previous execution run crashed or was killed mid-change, leaving an `in-progress` entry.

**Resolution:** Self-heal is automatic. The driver resolves the stale entry:
- If the change completed successfully, it transitions the entry to `done`.
- If the change is in a broken state, it transitions to `failed` or `blocked`.

For multi-repo entries with `project`, self-heal looks at `.specify/changes/<name>/.metadata.yaml` under the target project's workspace clone, not the initiating repo. If the workspace slot is missing, execution halts (see "Workspace slot missing" below).

If self-heal itself fails, manually resolve:
1. Check the stale change: `specify change status <name>`
2. Complete or drop it manually.
3. Transition the plan entry: `specify plan transition <name> done|failed`

### Workspace slot missing

**Symptom:** `/spec:execute` halts with a diagnostic pointing at `specify workspace sync` for a target project.

**Cause:** A plan entry has a `project` field targeting a registry project whose workspace slot is not materialised (`.specify/workspace/<project>/` does not exist or is incomplete).

**Resolution:**
1. Run `specify workspace sync` to materialise all registry projects.
2. Re-run `/spec:execute`.

### Execution stuck

**Symptom:** `/spec:execute --loop` exits with `stuck`.

**Cause:** No `pending` entry has all dependencies satisfied. Typically because a dependency is `failed` or `blocked`.

**Resolution:**
1. Check plan status: `specify plan status`
2. Identify the blocking entries.
3. Options:
   - Fix and retry the failed entry: `specify plan transition <name> pending` then `/spec:execute`
   - Skip it: `specify plan transition <name> skipped`
   - Remove the dependency: `specify plan amend <downstream> --depends-on <updated-list>`

### Phase failure during execution

**Symptom:** A plan entry transitions to `failed` during `/spec:execute`.

**Cause:** The define, build, or merge phase failed for this change.

**Resolution:**
1. The change was automatically dropped by the driver.
2. Review the failure in the journal: check `.specify/changes/<name>/journal.yaml` (if it exists before archiving).
3. To retry: reset the plan entry to `pending` and re-run `/spec:execute`.

## Contract issues

### `$ref` resolution failures

**Symptom:** `/contracts:validator` reports that a `$ref` pointer does not resolve.

**Cause:** A schema file referenced from an OpenAPI or AsyncAPI binding does not exist in either the change's `contracts/schemas/` or the baseline `.specify/contracts/schemas/`.

**Resolution:**
1. Check the `$ref` path in the binding file.
2. Verify the referenced schema file exists and the filename matches (kebab-case, `.yaml` extension).
3. If the schema is new, ensure `/contracts:writer` generated it. If it is a baseline schema, ensure the baseline is up to date.

### Schema metadata incomplete

**Symptom:** `/contracts:validator` reports missing `$id`, `title`, or `description` on a JSON Schema file.

**Cause:** The schema file was created without the required Specify metadata, or an imported external schema was not fully normalised.

**Resolution:**
1. Add the missing fields. `$id` must be `urn:specify:schemas/<filename-without-extension>`.
2. For imported schemas, run `/contracts:importer` (Layer 2) or add the metadata manually.

### Binding completeness failures

**Symptom:** `/contracts:validator` reports that a schema has no protocol binding.

**Cause:** A schema that appears as a top-level request/response body or message payload in a spec scenario has no corresponding OpenAPI path or AsyncAPI channel.

**Resolution:**
1. If the schema is a shared vocabulary type (e.g. `ErrorResponse`) used only via `$ref` from other schemas, it is exempt from this check -- verify the validator is not misclassifying it.
2. If the schema should have a binding, ensure `/contracts:writer` produced the corresponding OpenAPI or AsyncAPI file.

### Alignment warnings

**Symptom:** `/contracts:writer` reports alignment warnings in the alignment report.

**Cause:** The change's specs describe interactions that partially conflict with the baseline contracts -- e.g. a response schema missing a field that a spec scenario asserts, or a spec referencing a status code the baseline binding does not define.

**Resolution:**
1. Review each warning. The writer does not auto-resolve spec-vs-baseline conflicts.
2. If the spec is correct, update the baseline contract in a dedicated contract change.
3. If the baseline is correct, update the spec to conform.

## Schema and init issues

### Schema resolution failure

**Symptom:** `/spec:init` fails to resolve the schema URL.

**Cause:** Invalid URL, network error, or the `@ref` suffix does not exist.

**Resolution:**
1. Verify the URL format: `https://github.com/augentic/specify/schemas/<name>[@<ref>]`
2. Check network connectivity.
3. Try without a ref suffix to use the latest version.

### Cache stale after schema update

**Symptom:** Skills use outdated brief content.

**Cause:** The schema was updated upstream but the local cache was not refreshed.

**Resolution:** Re-run `/spec:init` with the schema URL to refresh the cache.

## Verify issues

### No baseline specs

**Symptom:** `/spec:verify` reports no baseline specs to verify against.

**Cause:** No changes have been merged yet.

**Resolution:** Complete and merge at least one change, or run the brownfield onboarding flow to establish a baseline from existing code.

### Source not found for capability

**Symptom:** `/spec:verify` cannot locate the implementation for a baseline spec.

**Cause:** The capability naming or project structure does not match what verify expects.

**Resolution:** Ensure the capability name in `.specify/specs/<name>/` corresponds to the actual source location in your project.
