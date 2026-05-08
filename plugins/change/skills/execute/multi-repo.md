# Multi-repo routing and cross-project contract check

For multi-repo changes the driver keeps the coordinator repo as the owner of `plan.yaml`, `.specify/plan.lock`, and terminal status transitions. For each plan entry with `project`, it resolves and prepares that project's materialised workspace slot, `chdir`s into the slot only for phase execution, then restores CWD before writing the terminal plan transition. After a successful merge, the driver commits any non-baseline residue before it can mark the entry `done`, then runs the non-fatal contract compatibility check against every consumer workspace.

These clones are the read-write **tier-2** workspace; they outlive the change and are pushed to remotes by `specify workspace push`. The read-only **tier-1** legacy-source clones used by `/spec:analyze` at plan time are a separate concern entirely. See [Workspace Tiers](../../../../docs/explanation/workspace-tiers.md) for the full contrast.

## Workspace routing and branch preparation (per-slice algorithm step 5a)

Read `project` from the `specify change plan next` response (step 4 of the per-slice algorithm). If `project` is non-null:

- Resolve the target project through `registry.yaml` using the same selector preflight as `specify workspace *`. Unknown names halt before filesystem, Git, forge, phase, or plan-status side effects.
- Save CWD (the initiating repo root).
- Resolve every key in the entry's `sources` list to an absolute filesystem path anchored to the initiating repo root. Git URLs pass through unchanged. These resolved paths are reused for `/spec:define` and for branch-preparation dirty-work classification.
- Check workspace state via `specify workspace status <project> --format json`. If the selected slot is `missing`, run `specify workspace sync <project>` and re-check only that project. Do **not** run broad `specify workspace sync` from `/change:execute`; selected execution materialises only the current plan entry's project unless the operator chose broader sync elsewhere.
- For mismatched materialisation (`other`, wrong origin, wrong symlink target, missing `.specify/project.yaml`, etc.), halt with the status diagnostic. Release the lock and exit non-zero; do not transition the plan entry.
- Prepare the worktree before any phase writes:
  ```bash
  specify workspace prepare-branch <project> \
      --change <change-name> \
      [source <absolute-source-path> ...] \
      [output <capability-owned-output-path> ...] \
      --format json
  ```
  The target branch is exactly `specify/<change-name>`. The helper fetches the remote-backed slot, resolves `origin/HEAD`, creates or reuses the local change branch, fast-forwards from `origin/specify/<change-name>` when appropriate, and classifies dirty work against the active slice boundary.
- On `prepared: true`, remember the returned `slot_path` and branch. After the plan entry transitions to `in-progress`, `chdir` into that prepared project root.
- Emit diagnostic: `Routing: <name> → <project> (<resolved-path>)`

If `project` is null, skip this step entirely (single-repo path).

### Branch-preparation failures

`workspace prepare-branch` failures are pre-phase failures, not phase outcomes. They never call `/spec:drop`, never write `.metadata.yaml:outcome`, and never transition the entry to `failed` or `blocked` automatically.

Stable diagnostic keys from the helper include:

| Key | Driver behaviour |
|---|---|
| `workspace-slot-missing` | Run the selected `specify workspace sync <project>` once, then retry status / branch preparation. If it is still missing, halt. |
| `origin-head-unresolved` | Halt before phase writes. Do not guess a default branch. |
| `dirty-unrelated-tracked` | Halt before checkout. Surface the blocked paths. |
| `dirty-branch-mismatch` | Halt before checkout. Resume-safe tracked work is allowed only when already on `specify/<change-name>`. |
| `origin-mismatch`, `workspace-slot-not-git`, `branch-pattern-mismatch`, `git-operation-failed` | Halt with the helper's diagnostic payload. |

When the branch-preparation failure occurs before the slice directory exists, there is no slice journal to write yet; the terminal output is the audit trail. When it occurs on a self-heal resume and `.specify/slices/<name>/journal.yaml` exists in the project slot, append a `failure` entry with summary `branch-preparation-failed: <diagnostic-key>` and the helper's JSON diagnostic in `--context`, then halt. In both cases the coordinator lock is released and the plan entry remains `pending` (fresh run) or `in-progress` (resume).

## Post-merge residue commit (per-slice algorithm step 9a)

For a routed project entry, `/spec:merge` success is not enough to mark the entry `done`. RFC-14 splits commit ownership:

1. `specify slice merge run` owns the merge-baseline commit and commits only `.specify/specs/` plus `.specify/archive/` with message `specify: merge <slice-name>`.
2. `/change:execute` owns any remaining project-output residue produced by define/build/merge, such as `crates/`, `contracts/`, `apps/`, generated tests, or other capability-owned files.

Immediately after reading `outcome: success` from `/spec:merge`, while still `chdir`ed into the project slot:

1. Check `.specify/specs/` and `.specify/archive/` for dirty tracked or untracked paths. If either tree is dirty, halt with diagnostic key `baseline-residue-after-merge`. Do not create a residue commit and do not transition the plan entry to `done`; the baseline commit boundary failed and requires operator triage.
2. Check the rest of the worktree, excluding `.specify/specs/` and `.specify/archive/`. If it is clean, emit `Residue: clean; no commit.` and continue.
3. If non-baseline residue exists, stage and commit only that residue:
   ```bash
   git add --all -- . ':!.specify/specs/**' ':!.specify/archive/**'
   git commit -m "specify: residue <slice-name>"
   ```
   On success, emit `Residue committed: specify: residue <slice-name>`.
4. If staging or committing fails, halt with diagnostic key `residue-commit-failed`. Leave the plan entry `in-progress`, release the lock, and tell the operator to inspect `git status` in the project slot. A later `/change:execute` run must pass self-heal's residue guard before it can transition the entry to `done`.

## CWD restore (per-slice algorithm step 9b)

If the CWD routing step (5c) changed the working directory, restore CWD to the saved initiating repo root. This ensures `specify change plan transition` (which reads `plan.yaml` in the initiating repo) runs from the correct directory. In `--loop` mode, the CWD routing and CWD restore steps bracket every iteration so that `specify change plan next` always runs from the initiating repo root.

## Cross-project contract check (RFC-9 §3B)

When step 10 transitions a slice to `done`, the driver runs a non-fatal contract compatibility check **only when** the merged slice satisfies all three conditions:

1. The merged slice has a non-null `project` field on its plan entry (multi-repo only — single-repo changes have no peer consumers to warn).
2. `registry.yaml` exists and the producer's project entry declares a non-empty `contracts.produces` list.
3. At least one project-output path changed by the completed slice under the producer slot's `contracts/` directory matches an entry in the producer's `produces` list (i.e. the routed workspace work actually touched a produced contract — most merges that just touch specs do nothing here).

When all three hold, walk the producer's `produces` list and find every consumer project (any registry entry whose `contracts.consumes` list contains the same path). RFC-12 collapsed the contract role set to `produces` and `consumes`; externally-authored contracts are encoded by the absence of any `produces` entry, not by a separate `imports` field. For each `(produced-contract, consumer)` pair, invoke the format-appropriate `/contract:*` skill in its verifier intent with the `cross-project` mode positional — pick the skill from the produced contract's category: `/contract:openapi` for HTTP / resource APIs (`contracts/http/*`), `/contract:asyncapi` for evented / pub-sub / streaming (`contracts/messages/*`), `/contract:json-schema` for shared payload schemas (`contracts/schemas/*`):

```bash
/contract:openapi \
    cross-project \
    producer-contract .specify/workspace/<producer>/<contract-path> \
    consumer-workspace .specify/workspace/<consumer>/
```

Both arguments are paths anchored at the **initiating repo root**. The producer contract path points into the producer's workspace slot because RFC-14 keeps project-output residue in that slot; journal entries and warning summaries still use the logical contract path (`contracts/...`) so operators do not see workspace implementation details in the contract identity.

The verifier emits a YAML report (see [shared report shape](../../../contract/references/report-shape.md#cross-project-mode-output-structured-yaml)). Parse `summary.total-findings`:

- Zero findings: do nothing — the consumer's view matches.
- One or more findings (any severity, including `info` for `consumer-has-no-baseline`): record each as a journal entry and render a warning block in the merge transcript.

### Recording the findings

For every finding, append one journal entry to **the merged slice** (not the consumer's slice) via:

```bash
specify slice journal append <merged-change-name> merge failure \
    --summary "cross-project-warning: <change-kind> in <consumer> for <contract>" \
    --context "$(cat <<'YAML'
contract: <contract-path>
consumer: <consumer-name>
workspace: .specify/workspace/<consumer>/
change-kind: <change-kind>
locator: <locator>
severity: <warning|info>
details: |
  <details prose verbatim from the validator>
YAML
)"
```

The journal entry uses the existing `failure` kind (per the journal contract in `specify-cli/crates/change/src/journal.rs` — `EntryKind::{Question, Failure, Recovery}`). The summary's `cross-project-warning:` prefix is the canonical marker that this entry is a §3B finding rather than an in-loop phase failure; the structured payload lives in `--context`. No new `EntryKind` variant is introduced; readers grep `summary` for the prefix when they need to filter.

The `--context` payload schema is stable and round-trippable:

| Key | Value |
|---|---|
| `contract` | The path of the produced contract (e.g. `contracts/http/user-api.yaml`). |
| `consumer` | The consumer project name (matches a `registry.yaml:projects[].name`). |
| `workspace` | The consumer's workspace clone path (`.specify/workspace/<consumer>/`). |
| `change-kind` | One of `removed-field`, `removed-endpoint`, `removed-channel`, `required-field-added`, `type-narrowed`, `status-code-removed`, `consumer-has-no-baseline`, `format-mismatch`. |
| `locator` | The dot-separated path into the contract document where the incompatibility was detected. |
| `severity` | `warning` for breaking changes; `info` for `consumer-has-no-baseline` and similar. |
| `details` | Free-form prose copied verbatim from the validator's `details` field. |

The append uses the verbatim journal-append shape — the `--context` is a multi-line YAML string the validator's prose maps into 1:1.

### Warning block in the merge transcript

When at least one finding is recorded, the success transcript (see [output-format.md](output-format.md) → Supervised / per-slice transcript → Success) renders a labelled warning block **immediately after** the `Status: done` line:

```text
⚠ Cross-project contract warnings
  Contract: contracts/http/user-api.yaml
  Consumers checked: 1 (mobile)

  mobile (.specify/workspace/mobile/):
    - removed-field at paths./users/{id}.get.responses.200.content.application/json.schema.properties.email
    - required-field-added at paths./users.post.requestBody.content.application/json.schema.required

  Recorded 2 finding(s) to .specify/slices/<name>/journal.yaml.
  Action needed: review the warning(s); the consumer change(s) may need a follow-up.
```

The block is omitted entirely when `summary.total-findings == 0`. Multiple consumers stack as additional indented blocks under the same `Contract:` line; multiple contracts produce repeated `Contract:` blocks.

### Non-fatal semantics

- **The execute loop never halts on cross-project warnings.** The merged slice has already transitioned to `done`. Findings are advisory output for the operator; the driver continues to the next iteration in `--loop` mode or exits normally in supervised mode.
- **The merged slice is not re-touched** beyond the journal append. No plan transition, no metadata edit, no follow-up phase invocation.
- **Verifier errors do not halt the driver.** If the format verifier (`/contract:openapi`, `/contract:asyncapi`, or `/contract:json-schema` running its verifier intent in `mode cross-project`) exits non-zero (read failure on a consumer workspace, malformed contract), record the failure as a single `failure`-kind journal entry on the merged slice with summary `cross-project-warning: validator-error in <consumer>` and continue. The driver does not retry the verifier.
- **The check is skipped under `dry-run`** end-to-end — dry-run never invokes phase skills (per the §Guardrails MUST-NOTs in the main SKILL.md) and the post-merge step inherits that prohibition.

### Self-heal interaction

Self-heal does **not** run the cross-project check on a reclaimed `success`-on-merge entry. The check is a one-shot side-effect of the live merge transition; on the next normal `/change:execute` startup, the merged slice has already been transitioned to `done` and no producer-side work remains. If a prior crash interrupted the cross-project check itself, the operator can re-trigger it manually by re-running the format-appropriate `/contract:*` skill (verifier intent, `mode cross-project`) against the same `(producer-contract, consumer-workspace)` pair — the verifier is idempotent and writes nothing to disk.
