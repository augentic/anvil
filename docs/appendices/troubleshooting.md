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
1. Review the conflict: `specify change merge conflict-check <name>`
2. Options:
   - Re-run `/spec:define` to update specs against the current baseline.
   - Manually resolve conflicts in the spec files.
   - Drop and redefine: `/spec:drop`, then `/spec:define` with updated description.

### Coherence failure after merge

**Symptom:** `specify change merge run` fails during coherence validation.

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

**Cause:** No `pending` entry has all dependencies satisfied. Typically because a dependency is `failed` or `blocked`, or a structural problem in the plan (cycle, unreachable entry) is preventing progress.

**Resolution:**
1. **First triage step:** run `specify plan doctor` -- it surfaces every structural problem (cycles, orphan sources, stale clones, unreachable entries) that `validate` would miss. See [Plan doctor diagnostics](#plan-doctor-diagnostics) below.
2. Check plan status: `specify plan status`
3. Identify the blocking entries.
4. Options:
   - Fix and retry the failed entry: `specify plan transition <name> pending` then `/spec:execute`
   - Skip it: `specify plan transition <name> skipped`
   - Remove the dependency: `specify plan amend <downstream> --depends-on <updated-list>`

### Registry amendment required

**Symptom:** `/spec:execute --loop` halts with a `registry-amendment-required` outcome on the offending change. The change is transitioned to `blocked` and the proposal payload is written to its `journal.yaml`.

**Cause:** A phase skill (typically `/spec:extract` or a build brief) discovered that the change targets a capability that does not fit any existing registry project, and proposed a new project. Introduced by RFC-9 Section 2B; the framework never auto-modifies the registry.

**Resolution:** Follow the canonical recovery sequence:

```bash
specify change journal show <change>             # read the proposal payload
specify registry add <proposed-name> \
    --url <proposed-url> \
    --schema <proposed-schema> \
    --description "<proposed-description>"
specify workspace sync                           # bootstrap the new slot
specify plan amend <change> --project <proposed-name>
specify plan transition <change> pending
# re-run /spec:execute
```

For the full how-to, see [Recover from registry-amendment-required](../how-to/recover-from-registry-amendment.md).

### Phase failure during execution

**Symptom:** A plan entry transitions to `failed` during `/spec:execute`.

**Cause:** The define, build, or merge phase failed for this change.

**Resolution:**
1. The change was automatically dropped by the driver.
2. Review the failure in the journal: check `.specify/changes/<name>/journal.yaml` (if it exists before archiving).
3. To retry: reset the plan entry to `pending` and re-run `/spec:execute`.

## Contract issues

### `$ref` resolution failures

**Symptom:** A format verifier (`/interfaces:openapi`, `/interfaces:asyncapi`, or `/interfaces:json-schema` running its verifier intent) reports that a `$ref` pointer does not resolve.

**Cause:** A schema file referenced from an OpenAPI or AsyncAPI binding does not exist in either the change's `contracts/schemas/` or the baseline `.specify/contracts/schemas/`.

**Resolution:**
1. Check the `$ref` path in the binding file.
2. Verify the referenced schema file exists and the filename matches (kebab-case, `.yaml` extension).
3. If the schema is new, ensure the corresponding `/interfaces:*` skill's author intent generated it (typically `/interfaces:json-schema` for shared payloads). If it is a baseline schema, ensure the baseline is up to date.

### Schema metadata incomplete

**Symptom:** `/interfaces:json-schema` (verifier intent) reports missing `$id`, `title`, or `description` on a JSON Schema file.

**Cause:** The schema file was created without the required Specify metadata, or an imported external schema was not fully normalised.

**Resolution:**
1. Add the missing fields. `$id` must be `urn:specify:schemas/<filename-without-extension>`.
2. For imported schemas, re-run the relevant `/interfaces:*` skill's importer intent (Layer 2) or add the metadata manually.

### Binding completeness failures

**Symptom:** A format verifier (`/interfaces:openapi` or `/interfaces:asyncapi` running its verifier intent) reports that a schema has no protocol binding.

**Cause:** A schema that appears as a top-level request/response body or message payload in a spec scenario has no corresponding OpenAPI path or AsyncAPI channel.

**Resolution:**
1. If the schema is a shared vocabulary type (e.g. `ErrorResponse`) used only via `$ref` from other schemas, it is exempt from this check -- verify the verifier is not misclassifying it.
2. If the schema should have a binding, ensure the relevant `/interfaces:*` skill's author intent (`/interfaces:openapi` for HTTP / resource APIs, `/interfaces:asyncapi` for evented / pub-sub / streaming) produced the corresponding binding file.

### Alignment warnings

**Symptom:** An `/interfaces:*` skill's author intent reports alignment warnings in the alignment report.

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

## Hub and registry issues

### `hub-cannot-be-project`

**Symptom:** `specify registry validate` (or `specify init --hub`) refuses with `hub-cannot-be-project: registry.yaml: projects[<idx>] (<name>).url is `.``.

**Cause:** A registry on a hub repo (`project.yaml: hub: true`) has an entry whose `url` is `.`. The hub topology forbids this -- the hub holds platform state and never appears in its own registry. Code projects always live in their own repos. Introduced by RFC-9 Section 1D.

**Resolution:** Two paths.

- **Stay on the hub:** remove the entry. `specify registry remove <name>`. Code projects must live in their own repos and be referenced via a remote URL.
- **Convert to platform-as-project:** if the operator actually wants the single-repo shape (the initiating repo is itself a code project), remove `.specify/` and re-run `specify init <schema>` without `--hub`. See [Platform repo topologies](../explanation/platform-repo.md).

### `description-missing-multi-repo`

**Symptom:** `specify registry add` or `specify registry validate` refuses with `description-missing-multi-repo` and names the offending entry.

**Cause:** A multi-project registry must declare a `description` on every entry (the description drives `/spec:plan`'s assignment step; sparse descriptions force unresolved prompts during planning). The invariant fires when the addition produces a multi-project registry and any existing entry lacks a description, or when validate is run against an already-violating registry.

**Resolution:** Add the missing descriptions. Either re-run `specify registry add` for each existing entry with `--description "..."`, or hand-edit `.specify/registry.yaml` and re-run `specify registry validate` to confirm.

```bash
specify registry add <existing-name> \
    --url <existing-url> \
    --schema <existing-schema> \
    --description "..."
```

`registry add` refuses if the entry already exists; for already-declared entries the operator hand-edits `registry.yaml` and runs `specify registry validate` again.

## Plan doctor diagnostics

`specify plan doctor` (RFC-9 Section 4B) is the first triage step when `/spec:execute --loop` reports `stuck`. It runs every check `validate` runs, then layers four health diagnostics.

### `cycle-in-depends-on`

**Symptom:** `specify plan doctor` reports `cycle-in-depends-on` with the cycle path (e.g. `["a", "b", "a"]`).

**Cause:** Two or more plan entries form a `depends-on` cycle. `next_eligible` silently skips cycles at runtime, so the executor reports `stuck`; `doctor` is the only place where the cycle structure is surfaced.

**Resolution:** Break the cycle with `specify plan amend <name> --depends-on <updated-list>` on one of the entries on the cycle path, then re-run doctor.

### `orphan-source-key`

**Symptom:** `specify plan doctor` reports `orphan-source-key` (warning) for a key declared in the top-level `sources:` map but referenced by no entry.

**Cause:** A `--source <key>=<path>` was supplied at plan time but no proposed slice ended up using it (rejected during the propose loop, or scope changed).

**Resolution:** Either reference the key from an entry's `sources:` list (`specify plan amend <name> --sources <key>`) or drop the declaration via a hand-edit of `.specify/plan.yaml`. Warnings are non-fatal; the loop will proceed.

### `stale-workspace-clone`

**Symptom:** `specify plan doctor` reports `stale-workspace-clone` (warning) with reason `signature-changed` (URL or schema diverged) or `missing-sync-stamp` (no stamp file and no readable git remote).

**Cause:** The workspace clone's signature has drifted from the registry, typically because `registry.yaml` was edited after the clone was first materialised.

**Resolution:** `specify workspace sync` to refresh the clone. The verb is idempotent.

### `unreachable-entry`

**Symptom:** `specify plan doctor` reports `unreachable-entry` for a pending entry whose dependency closure is rooted in a `failed` or `skipped` predecessor.

**Cause:** The entry's `depends-on` list (transitively) names an entry that can never become `done` (it is in a terminal non-success state).

**Resolution:** Two paths.

- **Reset the predecessor:** `specify plan transition <pred> pending` (after fixing the underlying issue) and re-run `/spec:execute`.
- **Drop the leaf:** `specify plan transition <entry> skipped --reason "<reason>"` to remove the entry from the dependency frontier.

## Initiative landing issues

### `branch-pattern-mismatch`

**Symptom:** `specify workspace merge` or `specify initiative finalize` refuses on a project with status `branch-pattern-mismatch`.

**Cause:** A PR exists on the workspace clone but its `headRefName` is not `specify/<initiative-name>` exactly. The verb refuses to operate on PRs created outside the Specify push flow -- the guard exists so the framework never accidentally squash-merges someone else's branch.

**Resolution:** Inspect the PR by hand (`gh pr view <pr> -R <org/repo>`). If the PR is correct, rename its branch to `specify/<initiative-name>`; if it was created outside the Specify flow, close it and re-run `specify workspace push`. The verbs never override the guard.

### `plan-not-found` from `initiative finalize`

**Symptom:** `specify initiative finalize` exits non-zero with `plan-not-found`.

**Cause:** `.specify/plan.yaml` does not exist. This is the explicit "already finalized" signal -- a previous successful `finalize` run swept the plan into `.specify/archive/plans/<YYYYMMDD>-<name>/`.

**Resolution:** None needed -- the initiative is already closed. Inspect the archive to confirm: `ls .specify/archive/plans/`. If the plan was lost some other way (e.g. accidental `rm`), recover from version control.

### Cross-project contract warnings on the merge transcript

**Symptom:** `/spec:execute --loop`'s merge transcript shows `cross-project-warning:` entries, and the merged change's `journal.yaml` carries the same warnings.

**Cause:** RFC-9 Section 3B post-merge cross-project contract validation. After a producer merges, the driver walks `contracts.produces`, finds consumer projects via `contracts.consumes`, and runs the format-appropriate `/interfaces:*` skill (verifier intent, with `--mode cross-project`) against each consumer's workspace clone — `/interfaces:openapi` for HTTP / resource APIs, `/interfaces:asyncapi` for evented / pub-sub / streaming, `/interfaces:json-schema` for shared payload schemas. Any incompatibilities surface as warnings. Warnings never halt the loop -- the operator triages.

**Resolution:** Read [Resolve cross-project contract warnings](../how-to/resolve-cross-project-contract-warnings.md) for the triage checklist. Typical paths: spawn a follow-up consumer change to track the producer's update, or accept the drift if the consumer is intentionally lagging.
