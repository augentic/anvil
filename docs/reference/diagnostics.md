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
| `merge-postflight-failed` | The target's postflight gate failed **after** the merge committed — the entry is already `done` and archived (non-rollback, sticky). | Inspect `.emery/change/archive/<date>-<slice>/merge/postflight.yaml`, repair the baseline, re-run execute to acknowledge. |
| `merge-incomplete` | The merge landed but the entry's `done` stamp is missing (torn stamp). | Re-run `emery plan execute` — it heals the stamp. |
| `slice-dropped` | The active entry's slice was dropped mid-plan. | Decide the entry's fate: re-plan it (`emery plan author --force` on a replaceable plan) or leave it dropped. |
| `stuck` | Pending entries remain but every one is blocked on unmet dependencies. | Run `emery plan validate` (first triage step) and fix the dependency structure. |
| `boundary-escalation` | Refinement wrote an inert boundary proposal at `planning/proposals/<digest>.yaml`; the leaf is parked (`slice.refinement.parked`). | Apply with `emery plan amend --proposal <digest>` after quiescing affected work, then re-run `emery plan refine` on the new children. Re-running refine on this leaf does not re-synthesize. |
| `refine-budget-exhausted` | Focused resurvey or nearest-domain re-decomposition exhausted its compiled budget. | Adjust sources or the bound profile, then re-run `emery plan refine`. |
| `domain-frontier-failed` | Domain-level verification failed over a multi-member wave's composed candidate (RFC-96 D8); the wave is parked — no prefix commits. | Repair the members (re-refine or apply an amendment) — staling the frozen bindings retracts the wave — then re-run `emery plan execute`. |
| `domain-complete-failed` | Domain-level verification failed (or has not passed) over the accepted tree (RFC-96 D8); dependants, drain, and publication materialization are blocked. | Land an authorized repair (a follow-up slice via `/emery:plan`), then re-run `emery plan execute`. |

## Driver lock

| Code | Meaning | Recovery |
| ---- | ------- | -------- |
| `guest-marker-held` | A second driver session tried to start while `.emery/change/guest.lock` is held (exit 2). | Wait for the running session, or if the holder died, [recover from the stale lock](../how-to/recover-from-a-stale-guest-lock.md). |

## Plan authoring and amendment

| Code | Meaning | Recovery |
| ---- | ------- | -------- |
| `plan-already-exists` | `emery plan author` found an existing plan under a *different name* (re-entry under the same name resumes or no-ops). | Re-run with `--force` to replace a pending plan wholesale (`/emery:plan` confirms first). |
| `plan-author-stopped` | Authoring parked one or more domains after failed cuts (exit 2); the partial tree persists and closed leaves project into `plan.entries`. | Re-run `emery plan author` — re-entry resumes only the open and parked domains. |
| `plan-author-incomplete` | A topology verb (`add` / `amend` / `remove` / `gaps`) ran on a bound-not-authored change home. | Re-run `emery plan author` to finish decomposition first. |
| `duplicate-source-key` | `--add-source` named a key the entry already binds (a slice binds at most one lead per source). | Re-size instead: `emery plan amend <entry> --sources <key>=<other-lead>`. |
| `plan-amend-validation-failed` | A wholesale `--sources` replacement introduced an invalid binding set; the amend rolled back. | Fix the binding list (one lead per source key) and retry. |
| `plan-remove-plan-not-replaceable` | `emery plan remove` requires a fully pending plan (every entry `pending`). | Removal is a pre-execution action only; after execution starts, drop the entry's slice with `emery plan drop`. |
| `plan-drop-no-slice` | `emery plan drop` found no slice tree for the entry (never refined). | Curate the entry with `emery plan remove` instead. |
| `plan-remove-entry-referenced` | Another entry lists the removal target in `depends-on`. | Amend the dependent entry's `--depends-on` first. |
| `plan-has-outstanding-work` | `emery plan archive` refused: the plan still has non-terminal entries (exit 1). | Drain the plan (merge or drop every entry) before archiving. |
| `plan-proposal-stale` | `emery plan amend --proposal` compare-and-set found a drifted frontier. | Quiesce affected claims and waves, then re-run against a fresh proposal. |
| `plan-proposal-live` | The proposal's affected claims are still live (an affected *open wave* no longer refuses — the applied amendment retracts it through refinement staleness, RFC-96 D7). | Quiesce the affected claims, then re-run `emery plan amend --proposal <digest>`. |
| `plan-proposal-preserve` | Applying the proposal would drop a preservation-required node. | Author a new proposal from the live decomposition. |
| `plan-proposal-kind` | The named document is an envelope or definition-revision — not an amendment. | Revise the reviewed handoff or wait for RFC-106. |
| `plan-proposal-cycle` | The candidate tree is cyclic. | Author a new proposal from the live decomposition. |
| `plan-proposal-not-found` | No file at `planning/proposals/<digest>.yaml`. | Check the digest from the stop card. |
| `plan-proposal-malformed` | The retained proposal failed its typed parse. | Re-emit the proposal (re-run refine) rather than hand-editing it. |
| `plan-mutation-ambiguous` | Direct `plan add` / `amend` / `remove` cannot uniquely reproject through `decomposition.yaml`. | Re-run `emery plan author --force` for a hierarchy edit, or apply a retained proposal. |
| `plan-ownership-overlap` | Merge found overlapping ownership; an inert ownership proposal is waiting. | Quiesce affected work, then apply with `emery plan amend --proposal <digest>`. |

## Plan decompose (inside `emery plan author`)

All exit 2. Decomposition validates the projected `slices[]` grouping before writing it; see [decomposition](cli/plan.md#decomposition-inside-emery-plan-author) for the full table.

| Code | Meaning |
| ---- | ------- |
| `proposal-schema` | The judgment response failed JSON-Schema validation. |
| `plan-reconcile-empty-catalog` | `leads.md` surfaced no leads to decompose. |
| `plan-reconcile-lead-orphan` | A cited `(source, lead)` is not in the surveyed catalog. |
| `lead-coverage-orphan` | A surveyed lead is referenced by no slice (coverage is at-least-once). |
| `plan-reconcile-slice-source-collision` | A slice names more than one lead from the same source. |
| `plan-reconcile-slice-name-invalid` | A slice `name` is not kebab-case. |
| `plan-reconcile-slice-name-collision` | Two slices resolve to the same name. |
| `plan-reconcile-depends-on-cycle` | The projected `depends-on` edges form a cycle. |
| `plan-reconcile-target-unknown` | A slice names a `target` absent from `plan.yaml.targets`. |
| `plan-reconcile-plan-not-replaceable` | The plan carries a non-pending entry. |

## Slice synthesis (inside `emery plan refine`)

All exit 2. Synthesis lends the agent a staged copy of the slice bundle (RFC-96 D10); the deterministic tail validates the staged tree before anything is promoted, and a tail failure re-prompts the same agent over the same stage. These codes surface on the envelope when the repair budget is exhausted; the slice artifacts stay untouched.

| Code | Meaning | Recovery |
| ---- | ------- | -------- |
| `slice-synthesize-stage-prepare-failed` | Snapshotting the slice bundle, materializing the writable stage, or seeding a dependency root failed before the agent ran. | Check the workspace store and slice tree, then re-run `emery plan refine`. |
| `slice-synthesize-stage-missing` | A required bundle file (`model.yaml`, `proposal.md`, `design.md`, `tasks.md`, or every `specs/<domain>/spec.md`) is absent or empty in the staged tree after the repair budget. | Re-run `emery plan refine`; persistent misses usually mean the source Evidence is too thin to synthesize from. |
| `slice-synthesize-stage-model` | The staged `model.yaml` failed the typed slice-model parse. | Re-run `emery plan refine`. |
| `slice-synthesize-stage-decision` | A staged `decisions/<slug>.md` is not a well-formed Decision Record (front-matter, `# <title>`, Nygard sections). | Re-run `emery plan refine`. |

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
| `slice-model-target-drift` | The model's recorded target diverged from the bound `plan.yaml.slices[].target`. | Re-run `emery plan refine`. |
| `slice-model-cross-ref-orphan` | A model cross-reference points at a requirement that no longer exists. | Re-run `emery plan refine`. |
| `slice-model-claim-kind-mismatch` | A contributing claim's kind does not match its Evidence row. | Re-run `emery plan refine`. |
| `slice-model-id-grammar` | A requirement or claim id violates the id grammar. | Re-run `emery plan refine`. |
| `slice-refinement-missing` | Review advisory: the slice has no `refinement.yaml` manifest. | Run `emery plan refine` to generate and cover its specification bundle. |
| `slice-refinement-stale` | Review advisory: a recorded refinement input or bundle artifact no longer matches the live file (one finding per drifted identity). | Re-run `emery plan refine` — execute never refines, and it refuses stale manifests with `plan-refinement-required`. |
| `slice-refinement-input-missing` | Manifest assembly found a canonical bundle artifact (`proposal.md`, `design.md`, `tasks.md`, or a per-domain spec) absent; the drain stops on the slice. | Fix the refinement failure the stop detail names, then re-run `emery plan refine`. |
| `slice-refinement-pin-missing` | A bound source has no closed content pin to record in the manifest. | Re-run `emery plan refine` — the drain closes pins during extraction; a persistent miss means the source binding is broken. |
| `slice-refinement-source-unbound` | The slice binds a source key absent from `plan.yaml.sources`. | Fix the binding (`emery plan amend`) or bind the source, then re-run `emery plan refine`. |
| `plan-projection-source-unbound` | A planning projection could not resolve an entry's source binding against `plan.yaml.sources`. | Fix the binding (`emery plan amend`), then re-run `emery plan refine`. |
| `slice-baseline-conflict` | Review advisory: the baseline drifted under a built slice since it was defined. | Fix inputs and re-refine / re-run execute, or accept the merge-time conflict handling. |
| `slice-disposition-drifted` | Review advisory: the deferred set a built slice's record consumed no longer matches the live dispositions (a deferral lapsed or was added after the build). | No manual action — the next `emery plan execute` re-builds the slice under the current dispositions. |
| `slice-wave-record-missing` | Review advisory: the slice's newest opened wave has no build record — the re-build it authorized failed. | No manual action — the next `emery plan execute` re-builds the slice before merge. |
| `slice-authority-override-orphan-source` | An authority override names a source key the slice does not bind. | Fix the override: `emery plan amend <entry> --authority-override <kind>=<source>`. |
| `slice-catalog-drift` | Evidence references a `component:` slug missing from (or rejected in) the Vectis catalog. | Review `.emery/design-system/components.yaml` — see [Component factoring](../explanation/components.md). |

## Build and merge

| Code | Meaning | Recovery |
| ---- | ------- | -------- |
| `plan-gap-digest-missing` | An open `[unknown]` / `[conflict]` row carries no requirement digest (a legacy `spec.md`-fallback inventory), so no deferral fact can take it out of build scope; the gap gate refuses rather than building over it. | Re-run `emery plan refine` — refinement rewrites `model.yaml` and mints the digests deferrals match on, then re-run `emery plan execute`. |
| `target-build-input-missing` | A `required` adapter-declared build input is absent from the slice tree. | Supply the input file (e.g. Vectis `tokens.yaml`) and re-run execute. |
| `target-build-success-with-blocking-finding` | The target reported `status: success` but its report carries a blocking finding; the gate refuses. | Fix the finding the report names, then re-run execute. |
| `target-build-deferred-covered` | The build phase report's `covered[]` claims a requirement the request's `deferred[]` excluded from build scope; the phase machine halts the attempt before verification. | Deferred requirements are out of the build's obligations — fix the target build so it neither implements nor claims them, then re-run execute. |
| `plan-refinement-required` | Execute reached an in-scope leaf without a fresh refinement manifest — checked before any epoch, workspace, or wave. | Run `emery plan refine`, then re-run `emery plan execute`. |
| `target-build-refinement-missing` | The build phase found no refinement manifest for the claimed slice (a hard refusal, unlike the `slice-refinement-missing` review advisory). | Run `emery plan refine`, then re-run `emery plan execute`. |
| `target-base-freeze-failed` | Freezing the product tree as the wave base at wave open failed. | Check the product tree is readable and re-run `emery plan execute`. |
| `plan-epoch-stale` | A covered refinement digest drifted (or its manifest disappeared) after the epoch opened. | Re-run `emery plan refine`, then `emery plan execute`. |
| `merge-delta-headers-required` | A hand-authored flat requirement block was submitted against a non-empty baseline. | Use the delta format (`## ADDED / MODIFIED / REMOVED / RENAMED Requirements`) — see [Artifact format](artifact-format.md#delta-spec-format-modified-domain). |
| `plan-entry-not-found` | The merge phase found no plan entry matching the slice. | Add the entry (`emery plan add`) or check the slice name. |
| `slice-merge-entry-not-in-progress` | The plan entry exists but is not claimed. | Re-run `emery plan execute` — the loop claims entries itself. |
| `target-wave-not-opened` | The merge phase found no `target.wave.opened` fact naming the slice. | Re-run `emery plan execute` — the build phase opens the wave merge resolves its record through. |
| `slice-build-record-missing` | A built slice has no `builds/<digest>.yaml`, or none records the merge's authorized wave. | Re-run `emery plan execute` — the loop re-builds and mints the record. |
| `slice-build-record-ambiguous` | More than one build record names the merge's authorized wave. | Remove the stale `builds/<digest>.yaml` duplicates, then re-run execute. |
| `target-wave-member-stale` | A frozen member's live refinement manifest no longer matches its wave binding — the whole uncommitted wave is retracted; no prefix commits (RFC-96 D7). | Re-run `emery plan execute` — the scheduler rebuilds the members under a fresh wave. |
| `target-merge-compose-failed` | Composing the frozen member patches into the wave's merge base failed (base mismatch or touched-path overlap surfaces here as `workspace-compose-*`). | Inspect the named conflict; overlapping members must land through separate waves — amend the plan topology, then re-run execute. |
| `workspace-compose-base-mismatch` / `workspace-compose-overlap` | `compose(base, patches)` refused: a patch starts from another base, or two patches touch the same path — there is no textual merge (RFC-96 D6). | Rebuild the stale member so every patch shares the wave base, or split overlapping work across waves. |
| `domain-frontier-compose-failed` / `domain-verify-prepare-failed` / `domain-verify-dispatch-failed` | A domain round's mechanics failed before any verdict: composing the frontier candidate, preparing the verification workspace, or the `target.verify` dispatch itself (RFC-96 D8). A dispatch failure is not a verdict — no round is recorded. | Fix the named failure (adapter binding, workspace store), then re-run `emery plan execute`. |

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
