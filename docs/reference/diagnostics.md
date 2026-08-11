# Diagnostics index

Every operator-facing diagnostic code in one lookup table: what the code means, and how to recover. Codes are kebab-case, grep-stable, and part of the public contract — they appear in the `error` field of a JSON failure envelope, as the `rule-id` of a validate finding, or as the stop reason `emery plan execute` renders.

Where a code fires:

- **Refusals and aborts** return the flat failure envelope (`error`, `message`, `exit-code`) — see [CLI output shapes](cli-output-shapes.md).
- **Validate findings** arrive inside a `DiagnosticReport`; open `critical`/`important` violations exit 2 — see [Interpret validate findings](../how-to/interpret-validate-findings.md).
- **Execute stops** render the `emery plan status` projection: the closed reason, a hint, and the literal resume command — see [`emery plan execute`](cli/plan.md#emery-plan-execute).

## Refine and execute stop reasons

Closed set rendered when `emery plan refine` halts (exit 2, `plan-refine-stopped`) or `emery plan execute` halts (exit 2, `plan-execute-stopped`). Re-running the stopped command after fixing the cause resumes the parked work.

| Code | Meaning | Recovery |
| ---- | ------- | -------- |
| `plan-refine-stopped` | The refinement drain halted on the first failed refinement (extract or synthesis); prior successful manifests stay. | Fix the source binding or amend the plan, then re-run `emery plan refine` — fresh manifests are skipped. |
| `refine-failed` | The awaited refinement last failed (extract or synthesis); the slice stays `refining`. | Fix the source binding or amend the plan, then re-run `emery plan refine`. |
| `refinement-required` | Execute reached an entry without a fresh refinement manifest (`plan-refinement-required` on the envelope) — execute never refines. | Run `emery plan refine`, then re-run `emery plan execute`. |
| `build-failed` | The target's build failed; the slice stays `refined`. | Read `failing-task` and `log-path` from the stop hint, fix, then re-run execute. |
| `merge-conflict` | The baseline drifted since the slice was defined; merge preflight refused. | Fix the conflicting inputs, re-run `emery plan refine` (the manifest is stale against the moved baseline), then re-run execute — see [Resolve spec conflicts](../how-to/resolve-spec-conflicts.md). |
| `merge-postflight-failed` | The target's postflight gate failed **after** the merge committed — the entry is already `done` and archived (non-rollback, sticky). | Inspect `.emery/archive/<date>-<slice>/merge/postflight.yaml`, repair the baseline, re-run execute to acknowledge. |
| `merge-incomplete` | The merge landed but the entry's `done` stamp is missing (torn stamp). | Re-run `emery plan execute` — it heals the stamp. |
| `slice-dropped` | The active entry's slice was dropped mid-plan. | Decide the entry's fate: re-plan it (`emery plan author --force` on a replaceable plan) or leave it dropped. |
| `stuck` | Pending entries remain but every one is blocked on unmet dependencies. | Run `emery plan validate` (first triage step) and fix the dependency structure. |

## Driver lock

| Code | Meaning | Recovery |
| ---- | ------- | -------- |
| `guest-marker-held` | A second driver session tried to start while `.emery/guest.lock` is held (exit 2). | Wait for the running session, or if the holder died, [recover from the stale lock](../how-to/recover-from-a-stale-guest-lock.md). |

## Plan authoring and amendment

| Code | Meaning | Recovery |
| ---- | ------- | -------- |
| `plan-already-exists` | `emery plan author` found an existing `plan.yaml`. | Re-run with `--force` to replace a pending plan wholesale (`/emery:plan` confirms first). |
| `duplicate-source-key` | `--add-source` named a key the entry already binds (a slice binds at most one lead per source). | Re-size instead: `emery plan amend <entry> --sources <key>=<other-lead>`. |
| `plan-amend-validation-failed` | A wholesale `--sources` replacement introduced an invalid binding set; the amend rolled back. | Fix the binding list (one lead per source key) and retry. |
| `plan-remove-plan-not-replaceable` | `emery plan remove` requires a fully pending plan (every entry `pending`). | Removal is a pre-execution action only; after execution starts, drop the entry's slice with `emery plan drop`. |
| `plan-drop-no-slice` | `emery plan drop` found no slice tree for the entry (never refined). | Curate the entry with `emery plan remove` instead. |
| `plan-remove-entry-referenced` | Another entry lists the removal target in `depends-on`. | Amend the dependent entry's `--depends-on` first. |
| `plan-has-outstanding-work` | `emery plan archive` refused: the plan still has non-terminal entries (exit 1). | Drain the plan (merge or drop every entry) before archiving. |

## Plan reconcile (inside `emery plan author`)

All exit 2. The reconcile leg validates the proposed `slices[]` grouping before writing it; see [lead reconciliation](cli/plan.md#lead-reconciliation-inside-emery-plan-author) for the full table.

| Code | Meaning |
| ---- | ------- |
| `proposal-schema` | The judgment response failed JSON-Schema validation. |
| `plan-reconcile-empty-catalog` | `discovery.md` surfaced no leads to reconcile. |
| `plan-reconcile-lead-orphan` | A cited `(source, lead)` is not in the surveyed catalog. |
| `lead-coverage-orphan` | A surveyed lead is referenced by no slice (coverage is at-least-once). |
| `plan-reconcile-slice-source-collision` | A slice names more than one lead from the same source. |
| `plan-reconcile-slice-name-invalid` | A slice `name` is not kebab-case. |
| `plan-reconcile-slice-name-collision` | Two slices resolve to the same name. |
| `plan-reconcile-depends-on-cycle` | The projected `depends-on` edges form a cycle. |
| `plan-reconcile-project-binding-required` | A slice omits `project` when more than one project exists. |
| `plan-reconcile-project-orphan` | A slice binds a `project` absent from the topology. |
| `plan-reconcile-plan-not-replaceable` | The plan carries a non-pending entry. |

## Plan validate findings

Findings from [`emery plan validate`](cli/plan.md#emery-plan-validate) — the first triage step when execute reports `stuck`.

| Code | Severity | Recovery |
| ---- | -------- | -------- |
| `duplicate-name` | important | Rename one of the colliding entries. |
| `cycle-in-depends-on` | important | Break the cycle: `emery plan amend <entry> --depends-on …`. |
| `orphan-source` | suggestion | A declared source no entry references — bind it or remove the declaration. |

## Slice validate findings

Findings from [`emery slice validate`](cli/slice.md#emery-slice-validate). The `slice-model-*` drift family shares one fix: re-run the synthesis that writes both `spec.md` and `model.yaml` — never hand-edit either.

| Code | Meaning | Recovery |
| ---- | ------- | -------- |
| `slice-spec-provenance-stale` | A kernel-rendered `ID:` / `Sources:` / `Status:` line was hand-edited. | Revert the edit; drive resolution through overrides and re-run `emery plan refine` — see [Resolve spec conflicts](../how-to/resolve-spec-conflicts.md). |
| `slice-model-schema` | `model.yaml` fails its typed schema. | Re-run `emery plan refine` (the drain re-refines the slice). |
| `slice-model-source-orphan` | `model.yaml` cites a source the plan no longer binds. | Re-run `emery plan refine` after the plan amendment. |
| `slice-model-target-drift` | The model's recorded target diverged from the bound project's. | Re-run `emery plan refine`. |
| `slice-model-cross-ref-orphan` | A model cross-reference points at a requirement that no longer exists. | Re-run `emery plan refine`. |
| `slice-model-claim-kind-mismatch` | A contributing claim's kind does not match its Evidence row. | Re-run `emery plan refine`. |
| `slice-model-id-grammar` | A requirement or claim id violates the id grammar. | Re-run `emery plan refine`. |
| `slice-refinement-missing` | Review advisory: the slice has no `refinement.yaml` manifest. | Run `emery plan refine` to generate and cover its specification bundle. |
| `slice-refinement-stale` | Review advisory: a recorded refinement input or bundle artifact no longer matches the live file (one finding per drifted identity). | Re-run `emery plan refine` — execute never refines, and it refuses stale manifests with `plan-refinement-required`. |
| `slice-baseline-conflict` | Review advisory: the baseline drifted under a built slice since it was defined. | Fix inputs and re-refine / re-run execute, or accept the merge-time conflict handling. |
| `slice-authority-override-orphan-source` | An authority override names a source key the slice does not bind. | Fix the override: `emery plan amend <entry> --authority-override <kind>=<source>`. |
| `slice-catalog-drift` | Evidence references a `component:` slug missing from (or rejected in) the Vectis catalog. | Review `.emery/design-system/components.yaml` — see [Component factoring](../explanation/components.md). |

## Build and merge

| Code | Meaning | Recovery |
| ---- | ------- | -------- |
| `target-build-input-missing` | A `required` adapter-declared build input is absent from the slice tree. | Supply the input file (e.g. Vectis `tokens.yaml`) and re-run execute. |
| `target-build-success-with-blocking-finding` | The target reported `status: success` but its report carries a blocking finding; the gate refuses. | Fix the finding the report names, then re-run execute. |
| `plan-refinement-required` | Execute reached an in-scope leaf without a fresh refinement manifest — checked before any epoch, workspace, or wave. | Run `emery plan refine`, then re-run `emery plan execute`. |
| `plan-gaps-unresolved` | Open `[conflict]` / unwaived `[unknown]` requirements block the gap gate before build. | Fix inputs and re-run `emery plan refine`, or defer named unknowns with `--waive <slice>/<req> --reason "<why>"` on execute. |
| `plan-epoch-stale` | A covered refinement digest drifted (or its manifest disappeared) after the epoch opened. | Re-run `emery plan refine`, then `emery plan execute`. |
| `merge-delta-headers-required` | A hand-authored flat requirement block was submitted against a non-empty baseline. | Use the delta format (`## ADDED / MODIFIED / REMOVED / RENAMED Requirements`) — see [Artifact format](artifact-format.md#delta-spec-format-modified-domain). |
| `plan-entry-not-found` | The merge phase found no plan entry matching the slice. | Add the entry (`emery plan add`) or check the slice name. |
| `slice-merge-entry-not-in-progress` | The plan entry exists but is not claimed. | Re-run `emery plan execute` — the loop claims entries itself. |

## Source sandbox

| Code | Meaning | Recovery |
| ---- | ------- | -------- |
| `source-survey-path-denied` / `source-extract-path-denied` | The adapter tried to read outside its preopened roots (symlinks escaping `$SOURCE_DIR` count); the slice stays `refining`. | Rebind the source via `emery plan amend` to include the needed root, or drop the source — see [Sandboxing](adapter-contract.md#sandboxing). |

## Adapter resolution and install

| Code | Meaning | Recovery |
| ---- | ------- | -------- |
| `adapter-cli-too-old` | The adapter's metadata declares a host-CLI floor newer than the running binary (exit 3). | Update the `emery` binary through its install channel. |
| `adapter-install-failed` | Pull-on-miss from the first-party registry failed (exit 1). | Check the name, seed locally with `emery adapter add`, or pin an explicit version. |
| `adapter-install-invalid` | The pulled artifact is malformed. | Retry; if it persists, pin a known-good version. |
| `adapter-digest-mismatch` | The store entry failed verify-on-read against its recorded digest. | Delete the store entry and re-resolve (it re-pulls and re-verifies). |
| `adapter-latest-failed` | `emery adapter upgrade` could not reach the registry. | Check network access and retry — see [Upgrade adapters](../how-to/upgrade-adapters.md). |
| `adapter-latest-none` | The adapter's registry repository has no exact-SemVer tags. | Check the adapter name; pin explicitly if the adapter is unpublished. |
| `adapter-github-uri-unsupported` | A GitHub URL was passed where an adapter identifier is expected. | Use a bare name, `emery:<name>@<semver>` pin, or a local `.wasm` path. |

## Init and versioning

| Code | Meaning | Recovery |
| ---- | ------- | -------- |
| `init-adapter-required` | `emery init` ran without an adapter positional (exit 2). | Name the target adapter (`emery init omnia`). |
| `project-platforms-required` | The resolved target requires `--platforms` and none was passed. | Re-run with the flag, e.g. `--platforms core,ios,android` (`core` is mandatory). |
| `emery-version-too-old` | The project's `emery-version` pin is newer than the running binary (exit 3). | Update the binary through its install channel; an older pin loads normally. |

## See also

- [Interpret validate findings](../how-to/interpret-validate-findings.md) — how severity and kind decide what blocks
- [CLI output shapes](cli-output-shapes.md) — the failure envelope these codes ride in
- [emery plan](cli/plan.md) / [emery slice](cli/slice.md) — the verbs that raise them
