# Multi-repo routing and cross-project contract check

For multi-repo initiatives the driver `chdir`s into a registered project clone under `.specify/workspace/<project>/` before invoking the phase skills, then restores CWD before the terminal plan transition. After a successful merge in a multi-repo change, the driver runs a non-fatal contract compatibility check against every consumer workspace.

These clones are the read-write **tier-2** workspace; they outlive the initiative and are pushed to remotes by `specify workspace push`. The read-only **tier-1** legacy-source clones used by `/spec:analyze` at plan time are a separate concern entirely. See [Workspace Tiers](../../../../docs/explanation/workspace-tiers.md) for the full contrast.

## CWD routing (per-change algorithm step 5a)

Read `project` from the `specify plan next` response (step 4 of the per-change algorithm). If `project` is non-null:

- Resolve the target directory from `registry.yaml`: relative-path `url` → resolved filesystem path; remote `url` → `.specify/workspace/<name>/`.
- Check workspace freshness via `specify workspace status` for that slot. If `missing`, halt with a diagnostic pointing the operator at `specify workspace sync`. Release the lock and exit non-zero.
- Save CWD (the initiating repo root).
- Resolve every key in the entry's `sources` list to an absolute filesystem path anchored to the initiating repo root. Git URLs pass through unchanged.
- `chdir` into the target project root.
- Emit diagnostic: `Routing: <name> → <project> (<resolved-path>)`

If `project` is null, skip this step entirely (single-repo path).

## CWD restore (per-change algorithm step 9a)

If the CWD routing step (5a) changed the working directory, restore CWD to the saved initiating repo root. This ensures `specify plan transition` (which reads `plan.yaml` in the initiating repo) runs from the correct directory. In `--loop` mode, the CWD routing and CWD restore steps bracket every iteration so that `specify plan next` always runs from the initiating repo root.

## Cross-project contract check (RFC-9 §3B)

When step 10 transitions a change to `done`, the driver runs a non-fatal contract compatibility check **only when** the merged change satisfies all three conditions:

1. The merged change has a non-null `project` field on its plan entry (multi-repo only — single-repo initiatives have no peer consumers to warn).
2. `.specify/registry.yaml` exists and the producer's project entry declares a non-empty `contracts.produces` list.
3. At least one merged file path under the producer's `contracts/` directory matches an entry in the producer's `produces` list (i.e. the merge actually touched a produced contract — most merges that just touch specs do nothing here).

When all three hold, walk the producer's `produces` list and find every consumer project (any registry entry whose `contracts.consumes` or `contracts.imports` list contains the same path). For each `(produced-contract, consumer)` pair, invoke the contracts validator in cross-project mode:

```bash
/contracts:validator \
    --mode cross-project \
    --producer-contract <merged-contract-path> \
    --consumer-workspace .specify/workspace/<consumer>/
```

Both arguments are paths anchored at the **initiating repo root**, not the producer's workspace clone — the post-merge check runs after the CWD restore (step 9a) so the driver sits in the initiating repo where `.specify/registry.yaml` and the central contracts live.

The validator emits a YAML report (see [`/contracts:validator` → §Output Format — Cross-Project](../../../contracts/skills/validator/SKILL.md#output-format--cross-project)). Parse `summary.total-findings`:

- Zero findings: do nothing — the consumer's view matches.
- One or more findings (any severity, including `info` for `consumer-has-no-baseline`): record each as a journal entry and render a warning block in the merge transcript.

### Recording the findings

For every finding, append one journal entry to **the merged change** (not the consumer's change) via:

```bash
specify change journal append <merged-change-name> merge failure \
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

When at least one finding is recorded, the success transcript (see [output-format.md](output-format.md) → Supervised / per-change transcript → Success) renders a labelled warning block **immediately after** the `Status: done` line:

```text
⚠ Cross-project contract warnings
  Contract: contracts/http/user-api.yaml
  Consumers checked: 1 (mobile)

  mobile (.specify/workspace/mobile/):
    - removed-field at paths./users/{id}.get.responses.200.content.application/json.schema.properties.email
    - required-field-added at paths./users.post.requestBody.content.application/json.schema.required

  Recorded 2 finding(s) to .specify/changes/<name>/journal.yaml.
  Action needed: review the warning(s); the consumer change(s) may need a follow-up.
```

The block is omitted entirely when `summary.total-findings == 0`. Multiple consumers stack as additional indented blocks under the same `Contract:` line; multiple contracts produce repeated `Contract:` blocks.

### Non-fatal semantics

- **The execute loop never halts on cross-project warnings.** The merged change has already transitioned to `done`. Findings are advisory output for the operator; the driver continues to the next iteration in `--loop` mode or exits normally in supervised mode.
- **The merged change is not re-touched** beyond the journal append. No plan transition, no metadata edit, no follow-up phase invocation.
- **Validator errors do not halt the driver.** If `/contracts:validator` exits non-zero (read failure on a consumer workspace, malformed contract), record the failure as a single `failure`-kind journal entry on the merged change with `--summary "cross-project-warning: validator-error in <consumer>"` and continue. The driver does not retry the validator.
- **The check is skipped under `--dry-run`** end-to-end — dry-run never invokes phase skills (per the §Guardrails MUST-NOTs in the main SKILL.md) and the post-merge step inherits that prohibition.

### Self-heal interaction

Self-heal does **not** run the cross-project check on a reclaimed `success`-on-merge entry. The check is a one-shot side-effect of the live merge transition; on the next normal `/spec:execute` startup, the merged change has already been transitioned to `done` and no producer-side work remains. If a prior crash interrupted the cross-project check itself, the operator can re-trigger it manually by re-running `/contracts:validator --mode cross-project` against the same `(producer-contract, consumer-workspace)` pair — the validator is idempotent and writes nothing to disk.
