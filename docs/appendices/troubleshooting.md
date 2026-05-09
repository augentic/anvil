# Troubleshooting

Common failure modes and their resolutions.

## Layout issues

### `legacy-layout` error from every CLI verb

**Symptom:** Any project-aware verb (`specify status`, `specify change plan ...`, `specify registry ...`, etc.) exits 1 with `error: legacy v1 layout detected; run \`specify migrate v2-layout\` to upgrade ([".specify/registry.yaml", ...])`. JSON callers see `error: "legacy-layout"`.

**Cause:** The CLI was upgraded past `0.2.0` (which moved operator-facing platform artifacts to the repo root) but the project still has v1-layout files under `.specify/`.

**Resolution:**

```bash
specify migrate v2-layout
```

The mover is idempotent and refuses to clobber existing destinations. The RFC-13 follow-on migration shims have been removed, so projects that still use `initiative.md` or `.specify/changes/` must be renamed to `change.md` and `.specify/slices/` manually before using current change or slice commands. See [Migrating to the v2 layout](../how-to/migrate-to-v2-layout.md) for the full walkthrough, including multi-repo platforms and collision recovery.

## Slice lifecycle issues

### "Slice not found"

**Symptom:** A skill reports that no slice exists or cannot find the specified slice.

**Cause:** The slice name is misspelled, or `/spec:init` has not been run.

**Resolution:**
1. Check active slices: `specify slice list`
2. Verify `.specify/` exists. If not, run `/spec:init`.

### "Slice not in expected state"

**Symptom:** A skill refuses to proceed because the slice is in the wrong lifecycle state (e.g. trying to build a slice that is not yet defined).

**Cause:** A previous phase did not complete, or the slice was manually transitioned.

**Resolution:**
1. Check the state: `specify slice status <name>`
2. Complete the missing phase (e.g. run `/spec:define`) or manually transition: `specify slice transition <name> <target>`

### Artifacts incomplete after define

**Symptom:** `/spec:build` reports missing artifacts even though `/spec:define` appeared to complete.

**Cause:** Define may have encountered an error mid-pipeline and not generated all artifacts.

**Resolution:**
1. Check which artifacts exist in `.specify/slices/<name>/`.
2. Re-run define to regenerate: `/spec:define <name>` or regenerate a specific artifact: `/spec:define <name> <artifact-id>`

## Merge issues

### Baseline conflict detected

**Symptom:** `/spec:merge` fails with a conflict-check error.

**Cause:** The baseline changed since the slice was defined (another slice was merged in between).

**Resolution:**
1. Review the conflict: `specify slice merge conflict-check <name>`
2. Options:
   - Re-run `/spec:define` to update specs against the current baseline.
   - Manually resolve conflicts in the spec files.
   - Drop and redefine: `/spec:drop`, then `/spec:define` with updated description.

### Coherence failure after merge

**Symptom:** `specify slice merge run` fails during coherence validation.

**Cause:** The merged baseline has structural issues (e.g. duplicate requirement IDs, broken references).

**Resolution:**
1. Review the error message for the specific coherence issue.
2. Fix the spec files in the slice directory.
3. Retry: `/spec:merge`

## Plan and execution issues

### Lock held by another process

**Symptom:** `/change:execute` reports that `.specify/plan.lock` is held.

**Cause:** Another `/change:execute` session is running, or a previous session crashed without releasing the lock.

**Resolution:**
1. Check the lock: `specify change plan lock status`
2. If the PID is not running, release it: `specify change plan lock release`
3. If another session is running, wait for it to finish.

### Self-heal on startup

**Symptom:** `/change:execute` reports a self-heal operation when starting.

**Cause:** A previous execution run crashed or was killed mid-slice, leaving an `in-progress` entry.

**Resolution:** Self-heal is automatic. The driver resolves the stale entry:
- If the slice completed successfully, it transitions the entry to `done`.
- If the slice is in a broken state, it transitions to `failed` or `blocked`.

For multi-repo entries with `project`, self-heal looks at `.specify/slices/<name>/.metadata.yaml` under the target project's workspace clone, not the initiating repo. If the workspace slot is missing, execution halts (see "Workspace slot missing" below).

If self-heal itself fails, manually resolve:
1. Check the stale slice: `specify slice status <name>`
2. Complete or drop it manually.
3. Transition the plan entry: `specify change plan transition <name> done|failed`

### Workspace slot missing

**Symptom:** `/change:execute` halts with a diagnostic pointing at `specify workspace sync` for a target project.

**Cause:** A plan entry has a `project` field targeting a registry project whose workspace slot is not materialised (`.specify/workspace/<project>/` does not exist or is incomplete).

**Resolution:**
1. Run `specify workspace sync <project>` to materialise the missing selected slot, or `specify workspace sync` to materialise all registry projects.
2. Re-run `/change:execute`.

### `origin-head-unresolved`

**Symptom:** `/change:execute` refuses before define/build/merge and reports `origin-head-unresolved` for a remote-backed workspace slot.

**Cause:** Branch preparation could not resolve `origin/HEAD` after fetching. Specify will not guess the default branch because `specify/<change-name>` must be prepared from the repository's remote default before any execution mutation.

**Resolution:**
1. Inspect the slot: `specify workspace status <project>`.
2. In the workspace slot, verify the remote default branch exists on the server and that `origin` points at the registry URL.
3. Fix the remote default branch in the forge, or repair the clone with `git remote set-head origin -a` after the remote is correct.
4. Re-run `/change:execute`.

### Dirty workspace slot before execution

**Symptom:** `/change:execute` refuses during branch preparation with a dirty-work diagnostic such as `dirty-unrelated-tracked` or `dirty-branch-mismatch`.

**Cause:** The target workspace slot has tracked modifications that are outside the active slice boundary, or resume-safe tracked modifications are present while the slot is not already on `specify/<change-name>`. The executor refuses to check out or mutate over unrelated work.

**Resolution:**
1. Inspect the slot: `specify workspace status <project>`.
2. Commit, stash, or discard unrelated local work in that slot.
3. If the work belongs to the active change, check out the exact `specify/<change-name>` branch first or let a clean `/change:execute` prepare it.
4. Re-run `/change:execute`.

### Execution stuck

**Symptom:** `/change:execute loop` exits with `stuck`.

**Cause:** No `pending` entry has all dependencies satisfied. Typically because a dependency is `failed` or `blocked`, or a structural problem in the plan (cycle, unreachable entry) is preventing progress.

**Resolution:**
1. **First triage step:** run `specify change plan doctor` -- it surfaces every structural problem (cycles, orphan sources, stale clones, unreachable entries) that `validate` would miss. See [Plan doctor diagnostics](#plan-doctor-diagnostics) below.
2. Check plan status: `specify change plan status`
3. Identify the blocking entries.
4. Options:
   - Fix and retry the failed entry: `specify change plan transition <name> pending` then `/change:execute`
   - Skip it: `specify change plan transition <name> skipped`
   - Remove the dependency: `specify change plan amend <downstream> --depends-on <updated-list>`

### Registry amendment required

**Symptom:** `/change:execute loop` halts with a `registry-amendment-required` outcome on the offending slice. The slice is transitioned to `blocked` and the proposal payload is written to its `journal.yaml`.

**Cause:** A phase skill (typically `/spec:extract` or a build brief) discovered that the slice targets a capability that does not fit any existing registry project, and proposed a new project. Introduced by RFC-9 Section 2B; the framework never auto-modifies the registry.

**Resolution:** Follow the canonical recovery sequence:

```bash
specify slice journal show <slice>             # read the proposal payload
specify registry add <proposed-name> \
    --url <proposed-url> \
    --schema <proposed-schema> \
    --description "<proposed-description>"
specify workspace sync                          # bootstrap the new slot
specify change plan amend <slice> --project <proposed-name>
specify change plan transition <slice> pending
# re-run /change:execute
```

For the full how-to, see [Recover from registry-amendment-required](../how-to/recover-from-registry-amendment.md).

### Phase failure during execution

**Symptom:** A plan entry transitions to `failed` during `/change:execute`.

**Cause:** The define, build, or merge phase failed for this slice.

**Resolution:**
1. The slice was automatically dropped by the driver.
2. Review the failure in the journal: check `.specify/slices/<name>/journal.yaml` (if it exists before archiving).
3. To retry: reset the plan entry to `pending` and re-run `/change:execute`.

## Contract issues

### `$ref` resolution failures

**Symptom:** A format verifier (`/contract:openapi`, `/contract:asyncapi`, or `/contract:json-schema` running its verifier intent) reports that a `$ref` pointer does not resolve.

**Cause:** A schema file referenced from an OpenAPI or AsyncAPI binding does not exist in either the slice's `contracts/schemas/` or the baseline `contracts/schemas/`.

**Resolution:**
1. Check the `$ref` path in the binding file.
2. Verify the referenced schema file exists and the filename matches (kebab-case, `.yaml` extension).
3. If the schema is new, ensure the corresponding `/contract:*` skill's author intent generated it (typically `/contract:json-schema` for shared payloads). If it is a baseline schema, ensure the baseline is up to date.

### Schema metadata incomplete

**Symptom:** `/contract:json-schema` (verifier intent) reports missing `$id`, `title`, or `description` on a JSON Schema file.

**Cause:** The schema file was created without the required Specify metadata, or an imported external schema was not fully normalised.

**Resolution:**
1. Add the missing fields. `$id` must be `urn:specify:schemas/<filename-without-extension>`.
2. For imported schemas, re-run the relevant `/contract:*` skill's importer intent (Layer 2) or add the metadata manually.

### Binding completeness failures

**Symptom:** A format verifier (`/contract:openapi` or `/contract:asyncapi` running its verifier intent) reports that a schema has no protocol binding.

**Cause:** A schema that appears as a top-level request/response body or message payload in a spec scenario has no corresponding OpenAPI path or AsyncAPI channel.

**Resolution:**
1. If the schema is a shared vocabulary type (e.g. `ErrorResponse`) used only via `$ref` from other schemas, it is exempt from this check -- verify the verifier is not misclassifying it.
2. If the schema should have a binding, ensure the relevant `/contract:*` skill's author intent (`/contract:openapi` for HTTP / resource APIs, `/contract:asyncapi` for evented / pub-sub / streaming) produced the corresponding binding file.

### Alignment warnings

**Symptom:** An `/contract:*` skill's author intent reports alignment warnings in the alignment report.

**Cause:** The slice's specs describe interactions that partially conflict with the baseline contracts -- e.g. a response schema missing a field that a spec scenario asserts, or a spec referencing a status code the baseline binding does not define.

**Resolution:**
1. Review each warning. The writer does not auto-resolve spec-vs-baseline conflicts.
2. If the spec is correct, update the baseline contract in a dedicated contract slice.
3. If the baseline is correct, update the spec to conform.

## Capability and init issues

### Capability resolution failure

**Symptom:** `/spec:init` fails to resolve the capability identifier.

**Cause:** Invalid identifier or URL, network error, or the `@ref` suffix does not exist.

**Resolution:**
1. Verify the identifier format: a bare name (e.g. `omnia`), an `https://github.com/augentic/specify/capabilities/<name>[@<ref>]` URL, or a `file:///…` URI.
2. Check network connectivity.
3. Try without a ref suffix to use the latest version.

### Cache stale after capability update

**Symptom:** Skills use outdated brief content.

**Cause:** The capability was updated upstream but the local cache was not refreshed.

**Resolution:** Re-run `/spec:init <capability>` to refresh the cache.

## Hub and registry issues

### `hub-cannot-be-project`

**Symptom:** `specify registry validate` (or `specify init --hub`) refuses with `hub-cannot-be-project: registry.yaml: projects[<idx>] (<name>).url is `.``.

**Cause:** A registry on a hub repo (`project.yaml: hub: true`) has an entry whose `url` is `.`. The hub topology forbids this -- the hub holds platform state and never appears in its own registry. Code projects always live in their own repos. Introduced by RFC-9 Section 1D.

**Resolution:** Two paths.

- **Stay on the hub:** remove the entry. `specify registry remove <name>`. Code projects must live in their own repos and be referenced via a remote URL.
- **Convert to platform-as-project:** if the operator actually wants the single-repo shape (the initiating repo is itself a code project), remove `.specify/` and re-run `specify init <capability>` without `--hub`. See [Platform repo topologies](../explanation/platform-repo.md).

### `description-missing-multi-repo`

**Symptom:** `specify registry add` or `specify registry validate` refuses with `description-missing-multi-repo` and names the offending entry.

**Cause:** A multi-project registry must declare a `description` on every entry (the description drives `/change:plan`'s assignment step; sparse descriptions force unresolved prompts during planning). The invariant fires when the addition produces a multi-project registry and any existing entry lacks a description, or when validate is run against an already-violating registry.

**Resolution:** Add the missing descriptions. Either re-run `specify registry add` for each existing entry with `--description "..."`, or hand-edit `registry.yaml` and re-run `specify registry validate` to confirm.

```bash
specify registry add <existing-name> \
    --url <existing-url> \
    --schema <existing-schema> \
    --description "..."
```

`registry add` refuses if the entry already exists; for already-declared entries the operator hand-edits `registry.yaml` and runs `specify registry validate` again.

## Plan doctor diagnostics

`specify change plan doctor` (RFC-9 Section 4B) is the first triage step when `/change:execute loop` reports `stuck`. It runs every check `validate` runs, then layers four health diagnostics.

### `cycle-in-depends-on`

**Symptom:** `specify change plan doctor` reports `cycle-in-depends-on` with the cycle path (e.g. `["a", "b", "a"]`).

**Cause:** Two or more plan entries form a `depends-on` cycle. `next_eligible` silently skips cycles at runtime, so the executor reports `stuck`; `doctor` is the only place where the cycle structure is surfaced.

**Resolution:** Break the cycle with `specify change plan amend <name> --depends-on <updated-list>` on one of the entries on the cycle path, then re-run doctor.

### `orphan-source-key`

**Symptom:** `specify change plan doctor` reports `orphan-source-key` (warning) for a key declared in the top-level `sources:` map but referenced by no entry.

**Cause:** A `--source <key>=<path>` was supplied at plan time but no proposed slice ended up using it (rejected during the propose loop, or scope changed).

**Resolution:** Either reference the key from an entry's `sources:` list (`specify change plan amend <name> --sources <key>`) or drop the declaration via a hand-edit of `plan.yaml`. Warnings are non-fatal; the loop will proceed.

### `stale-workspace-clone`

**Symptom:** `specify change plan doctor` reports `stale-workspace-clone` (warning) with reason `signature-changed` (URL or schema diverged) or `missing-sync-stamp` (no stamp file and no readable git remote).

**Cause:** The workspace clone's signature has drifted from the registry, typically because `registry.yaml` was edited after the clone was first materialised.

**Resolution:** `specify workspace sync` to refresh the clone. The verb is idempotent.

### `unreachable-entry`

**Symptom:** `specify change plan doctor` reports `unreachable-entry` for a pending entry whose dependency closure is rooted in a `failed` or `skipped` predecessor.

**Cause:** The entry's `depends-on` list (transitively) names an entry that can never become `done` (it is in a terminal non-success state).

**Resolution:** Two paths.

- **Reset the predecessor:** `specify change plan transition <pred> pending` (after fixing the underlying issue) and re-run `/change:execute`.
- **Drop the leaf:** `specify change plan transition <entry> skipped --reason "<reason>"` to remove the entry from the dependency frontier.

## Change landing issues

### `no-branch` from `workspace push`

**Symptom:** `specify workspace push <project>` reports `no-branch`.

**Cause:** The slot is not currently on exact `specify/<change-name>`, or the expected change branch resolves to the remote default branch. RFC-14 push is transport-only: it does not create or check out the change branch, and it never pushes `main`, `master`, or any default branch.

**Resolution:**
1. Check the branch and match state: `specify workspace status <project>`.
2. If execution has not run for this project, run `/change:execute` so branch preparation creates or reuses `specify/<change-name>` before mutation.
3. If you are recovering by hand, check out the exact `specify/<change-name>` branch in the slot and ensure it contains the intended commits.
4. Re-run `specify workspace push <project>`.

### Dirty slot from `workspace push` or `change finalize`

**Symptom:** `specify workspace push` reports status `failed` with a dirty-checkout message, or `specify change finalize` reports status `dirty`.

**Cause:** The workspace slot has uncommitted work. Push refuses dirty slots because it only transports committed state. Finalize refuses dirty slots, even without `--clean`, so no local work is lost during archive or cleanup.

**Resolution:**
1. Inspect the slot: `specify workspace status <project>`.
2. Commit and push intended work on `specify/<change-name>`, or stash/remove unrelated local edits.
3. Re-run `specify workspace push <project>` if the PR still needs publication.
4. After the PR is merged, re-run `specify change finalize`.

### `unmerged` from `change finalize`

**Symptom:** `specify change finalize` refuses with status `unmerged` for one or more projects.

**Cause:** A PR exists on `specify/<change-name>` but is still open. Finalize is read-only with respect to forges; it verifies that the operator already landed the PR and never invokes a merge API.

**Resolution:**
1. Open the PR shown in the finalize output.
2. Merge it through the forge UI, `gh pr merge`, or the repository's normal merge queue.
3. Re-run `specify change finalize` after the forge reports the PR as merged.

### `branch-pattern-mismatch`

**Symptom:** branch preparation or `specify change finalize` refuses on a project with status `branch-pattern-mismatch`.

**Cause:** The change branch or PR head is not exactly `specify/<change-name>`. The guard exists so Specify never prepares, publishes, or finalizes an unintended branch.

**Resolution:** Inspect the branch or PR by hand (`gh pr view <pr> -R <org/repo>`). If the PR is correct, recreate or rename it so the head branch is exactly `specify/<change-name>`. If it was created outside the Specify flow, close it, publish the exact change branch with `specify workspace push`, merge it through the forge, and re-run `specify change finalize`. The guard is never overridden.

### `plan-not-found` from `change finalize`

**Symptom:** `specify change finalize` exits non-zero with `plan-not-found`.

**Cause:** `plan.yaml` does not exist. This is the explicit "already finalized" signal -- a previous successful `finalize` run swept the plan into `.specify/archive/plans/<YYYYMMDD>-<name>/`.

**Resolution:** None needed -- the change is already closed. Inspect the archive to confirm: `ls .specify/archive/plans/`. If the plan was lost some other way (e.g. accidental `rm`), recover from version control.

### Breaking findings from `specify compatibility check`

**Symptom:** `specify compatibility check` exits validation-failed and reports `breaking`, `ambiguous`, or `unverifiable` findings.

**Cause:** RM-04 compatibility classification found producer-to-consumer contract risk, or it could not compare the current producer contract with a consumer workspace view.

**Resolution:** Read [Resolve cross-project compatibility findings](../how-to/resolve-cross-project-contract-warnings.md) for the triage checklist. Typical paths: spawn a follow-up consumer slice to track the producer's update, refresh the workspace clone if the finding is unverifiable, or accept the drift if the consumer is intentionally lagging.
