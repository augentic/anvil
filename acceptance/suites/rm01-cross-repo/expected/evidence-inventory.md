# Expected Evidence Inventory

> Per-stage list of files the runner is expected to write into the temp run
> directory for an `rm01-cross-repo` run. Owner: [`scenario.md`](../scenario.md)
> (Cleanup section + the per-stage Expected Artifacts subsections).

The shape comes directly from
[`acceptance/README.md` §Run Evidence Policy](../../../README.md#run-evidence-policy)
and [`acceptance/runner/README.md` §Run Directories And Evidence](../../../runner/README.md#run-directories-and-evidence).
What this file adds is the **RM-01-specific** evidence the runner must
capture beyond the shared `summary.md` / `assertions.json` shape.

Run output is **never** committed to this repo. Every path below is
relative to a temp run-root path under the runner's control (e.g.
`${TMPDIR}/specify-acceptance/rm01-cross-repo/<run-id>/`).

## Run-Root Layout

```text
<run-root>/
  summary.md                          # shared: human-readable verdict + fault domain
  scenario.md                         # shared: copy of the executed scenario.md
  assertions.json                     # shared: structured per-assertion results
  stdout.log                          # shared: aggregated stdout from CLI invocations
  stderr.log                          # shared: aggregated stderr from CLI invocations
  final-tree.txt                      # shared: recursive listing of <temp-root>
  transcript.md                       # backend-specific: agent backend only
  tool-calls.jsonl                    # backend-specific: recorded backend only
  artifacts/                          # shared: misc capture surface
  failures/                           # shared: per-failure detail when assertions fail

  # ── RM-01-specific evidence (this suite) ──────────────────────────────
  registry.yaml                       # snapshot from after C07 setup
  plan.yaml.before-finalize           # snapshot taken pre-`specify change finalize`
  workspace-status.json               # `specify --format json workspace status` output
  push-output.json                    # `specify --format json workspace push` output
  finalize-output.json                # `specify --format json change finalize` output
  finalize-output.second-call.json    # second-call output asserting plan-not-found
  git/
    hub.log                           # `git log --format=...` from shop-platform/
    shop-backend.log                  # `git log --format=...` from the backend clone
    shop-mobile.log                   # `git log --format=...` from the mobile clone
  fake-gh/
    prs.json                          # snapshot of fake gh's PR state across both repos
```

## Per-Stage Capture Order

Evidence is captured at the **end of each stage** so a partial-failure
run preserves the latest known-good state. The order below is the order
the runner should write files in.

### Stage: setup (C07)

After `specify init --hub`, `specify registry add` (×2), and the fixture
brief seed:

| File                                     | Source                                                            | Purpose                                                                                  |
| ---------------------------------------- | ----------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| `registry.yaml`                          | copy of `<temp-root>/shop-platform/registry.yaml`                 | Asserts the shape in [`registry.yaml.skeleton.md`](registry.yaml.skeleton.md).           |
| `git/hub.log`                            | `git log --format='%H %s' --all` from `<temp-root>/shop-platform` | Confirms hub init / registry-add commits or no-commit setup.                             |
| `git/shop-backend.log`                   | `git log --format='%H %s' --all` from `sources/shop-backend`      | Confirms project seed commit only — no premature workflow commits before the suite runs. |
| `git/shop-mobile.log`                    | as above for `sources/shop-mobile`                                | as above.                                                                                |

### Stage: plan (C09)

After `/change:plan oauth-login` returns:

| File                       | Source                                                                                       | Purpose                                                                       |
| -------------------------- | -------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `plan.yaml.before-finalize` | copy of `<temp-root>/shop-platform/plan.yaml`                                                | The plan the role-based assertions read; preserved for failure debugging.     |
| `workspace-status.json`    | `specify --format json workspace status` from `<temp-root>/shop-platform`                    | Confirms `.specify/workspace/<peer>/` slots materialised by sync-peers.       |
| `git/hub.log`              | refresh of the file from setup                                                               | Records any planner-driven commits in the hub (plan authoring trail, etc.).   |
| `transcript.md`            | runner-collected agent transcript when the agent backend ran                                 | Drift-debugging input only; never the test oracle.                            |

### Stage: execute (C10) — *captured by `scripted-execute`*

After the C10 deterministic loop driver reaches `all-done`:

| File                          | Source                                                                                                           | Purpose                                                                                              |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `git/shop-backend.log`        | `git log --decorate --oneline --all` from `<temp-root>/shop-platform/.specify/workspace/shop-backend`            | Asserts `specify: merge <slice>` and `specify: residue <slice>` commit pair on `specify/oauth-login`. |
| `git/shop-mobile.log`         | as above for the mobile clone                                                                                    | as above.                                                                                            |
| `workspace-status.json`       | refresh after execute completes                                                                                  | Asserts `dirty: false`, `current-branch: specify/oauth-login`, `branch-matches-change: true`.        |
| `plan.yaml.before-finalize`   | copy of `<temp-root>/shop-platform/plan.yaml` after the loop reaches `all-done`                                  | Snapshot for C11's finalize comparison; carries every entry at `status: done`.                       |
| `scripted-execute-loop.jsonl` | runner-emitted log of one record per loop iteration (slice → routed project → preparedBranch → stub action count) | Self-describes the C10 loop driver's CLI sequence.                                                   |
| `scripted-plan-actions.jsonl` | runner-emitted log of one record per `specify change plan {create, add, status}` invocation (plus `change plan next` probes)               | Self-describes the underlying CLI sequence the plan-creation phase executed.                         |
| `stub-actions.jsonl`          | runner-emitted log of one record per `specify` / `git` action issued by `StubBackend.driveSlice`                | Self-describes per-slice loop work (transitions, baseline/residue commits).                          |

### Stage: push (C11) — *captured by `scripted-finalize`*

After `specify workspace push` returns:

| File                              | Source                                                                                  | Purpose                                                                                      |
| --------------------------------- | --------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `push-output.json`                | `specify --format json workspace push` from `<temp-root>/shop-platform`                 | Asserts `projects[*].status == "pushed"`, `branch == specify/oauth-login`, numeric `pr` per project. Pinned by `push-output-json-shape-clean` so a JSON-shape regression is attributed to `cli-substrate`. |
| `fake-gh/prs.json`                | normalised dump of `<temp-root>/gh-state/*.pr`                                          | Asserts one PR per routed project on `specify/oauth-login` (state may be `OPEN` or `MERGED` depending on capture order; see `push-opens-pr-per-project`). |
| `scripted-finalize-actions.jsonl` | runner-emitted log of one record per push/mark-merged/finalize CLI step                 | Self-describes the C11 finalize-phase CLI sequence (workspace push → mark-prs-merged → first finalize → second finalize). |

### Stage: external merge simulation (C11) — *captured by `scripted-finalize`*

The backend mutates `<temp-root>/gh-state/*.pr` via `markPrMerged` to
flip every PR file to `MERGED` (preserving fields 1, 4, 5 — see the
load-bearing PR-state file format documented in
[`acceptance/runner/fake-gh.ts`](../../../runner/fake-gh.ts)). The
post-mutation state is preserved in `fake-gh/prs.json` (refreshed at
teardown by `collectEvidence`). No CLI call happens in this step.

### Stage: contracts-build (C13) — *captured by `contracts-build`*

When the suite is driven through the `contracts-build` backend
(`make acceptance-cross-repo-contracts-build-smoke`), the
contract slice's `ContractsBuildPhaseDriver` writes a deterministic
OpenAPI 3.1 + JSON Schema bundle into the hub's `contracts/` tree
(by-design realistic-but-stubbed fixture; see the `# STUB:` header
on every emitted file). The C13 contract-slice-* assertion family
then validates the bundle via the `contract` WASI tool.

| File                                  | Source                                                                                  | Purpose                                                                                      |
| ------------------------------------- | --------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `contracts/http/oauth-login.yaml`     | `ContractsBuildPhaseDriver` (deterministic body)                                        | OpenAPI 3.1 doc covering the OAuth start / exchange / refresh endpoints. Asserted by `contract-slice-emits-yaml-artifacts` and `contract-slice-includes-openapi-or-asyncapi`. |
| `contracts/schemas/oauth-token-request.yaml` | `ContractsBuildPhaseDriver` (deterministic body)                                 | JSON Schema for the token-exchange request body. Asserted by `contract-slice-includes-required-schemas`. |
| `contracts/schemas/oauth-token-response.yaml` | `ContractsBuildPhaseDriver` (deterministic body)                                | JSON Schema for the issued session tokens. Asserted by `contract-slice-includes-required-schemas`. |
| `contracts/schemas/error-response.yaml`       | `ContractsBuildPhaseDriver` (deterministic body)                                | JSON Schema for the structured error payload. Asserted by `contract-slice-includes-required-schemas`. |
| `contract-validator-scratch/`         | `runContractValidator` (`acceptance/assertions/verifier.ts`) staging dir                | Holds the synthesised `.specify/project.yaml` + `schemas/contracts/{capability,tools}.yaml` sidecar that lets `specify tool run contract` resolve the WASI tool. The captured stdout (`{"schema-version":2,"ok":true,"findings":[]}`) is the evidence pin for `contract-slice-yaml-validates-via-tool`. |

The contract bundle lives in the hub's baseline tree (the contract
slice is projectless), so the same files double as the
`contract-baseline-files-present` evidence after the merge step.

### Stage: omnia-build (C14a) — *captured by `omnia-build`*

When the suite is driven through the `omnia-build` backend
(`make acceptance-cross-repo-omnia-build-smoke`), the
`OmniaBuildPhaseDriver` writes a deterministic Rust crate skeleton
into the routed Omnia clone's `crates/<crate>/` tree (by-design
realistic-but-stubbed fixture; see the `# STUB:` / `// STUB:`
header on every emitted file). The C14a omnia-* assertion family
then validates that the skeleton landed correctly and the slice's
baseline files survived the merge. The contract slice still runs
through `ContractsBuildPhaseDriver` (Omnia builds need real
contract YAML to consume), so the C13 evidence above continues
to apply.

| File                                  | Source                                                                                  | Purpose                                                                                      |
| ------------------------------------- | --------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `.specify/workspace/shop-backend/crates/oauth_tokens/Cargo.toml`         | `OmniaBuildPhaseDriver` (deterministic body)                                            | Minimal `[package]`-bearing manifest with the dependency set scrubbed against `plugins/omnia/references/guardrails.md` §Forbidden Crates. Asserted by `omnia-slice-emits-cargo-toml`. |
| `.specify/workspace/shop-backend/crates/oauth_tokens/src/lib.rs`         | `OmniaBuildPhaseDriver` (deterministic body)                                            | Library entrypoint stub exposing `OauthToken` + `OauthTokenStore`; references the merged baseline `contracts/oauth-login.yaml` in a doc comment so the contract-first invariant remains visible. Asserted by `omnia-slice-emits-lib-rs`. |
| `.specify/workspace/shop-backend/crates/oauth_tokens/src/providers.rs`   | `OmniaBuildPhaseDriver` (deterministic body)                                            | Provider-trait stub (`TokenStore`) following the Omnia provider pattern (one trait per capability; host runtime injects an implementation). Folded into the residue commit via `git commit --amend --no-edit` so HEAD~1 stays the baseline merge commit. |

The crate tree lives inside the routed clone (the Omnia slice is
project-routed; nothing is written at the hub level), so the same
three files double as the `omnia-baseline-files-present` evidence
after the per-slice merge step. The `--amend --no-edit` step the
driver appends to the residue commit is logged as
`omnia-build-amend-residue` in `stub-actions.jsonl` (the per-slice
action log the driver shares with `StubPhaseDriver`).

### Stage: vectis-build (C14b) — *captured by `vectis-build`*

When the suite is driven through the `vectis-build` backend
(`make acceptance-cross-repo-vectis-build-smoke`), the
`VectisBuildPhaseDriver` writes a deterministic Vectis composition
+ SwiftUI shell skeleton into the routed mobile clone (by-design
realistic-but-stubbed fixture; see the `# STUB:` / `// STUB:`
header on every emitted file). The C14b vectis-* assertion family
then validates that the skeleton landed correctly and the slice's
baseline files survived the merge. The contract slice still runs
through `ContractsBuildPhaseDriver` (Vectis builds need real
contract YAML to consume), so the C13 evidence above continues
to apply.

| File                                  | Source                                                                                  | Purpose                                                                                      |
| ------------------------------------- | --------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `.specify/workspace/shop-mobile/composition.yaml`                  | `VectisBuildPhaseDriver` (deterministic body)                                           | Vectis composition with one `login` screen (header / body / footer regions, group containers, event-wired provider buttons). Validates against `capabilities/vectis/composition.schema.json`. Asserted by `vectis-slice-emits-composition-yaml`. Folded into the residue commit via `git commit --amend --no-edit` so HEAD~1 stays the baseline merge commit. |
| `.specify/workspace/shop-mobile/apps/mobile/login_screen.swift`    | `VectisBuildPhaseDriver` (deterministic body)                                           | Minimal SwiftUI shell (`struct LoginScreen: View`) referencing the merged baseline `contracts/oauth-login.yaml` in a doc comment so the contract-first invariant remains visible. Asserted by `vectis-slice-emits-screen-files`. |

The composition + shell tree lives inside the routed mobile clone
(the Vectis slice is project-routed; nothing is written at the
hub level), so the same files double as the
`vectis-baseline-files-present` evidence after the per-slice
merge step. The `--amend --no-edit` step the driver appends to
the residue commit is logged as `vectis-build-amend-residue` in
`stub-actions.jsonl` (the per-slice action log the driver shares
with `StubPhaseDriver` / `OmniaBuildPhaseDriver`).

### Stage: define (C12) — *captured by `agent`*

When the suite is driven through the `agent` backend (typically
`make acceptance-cross-repo-define-smoke OPERATOR_RESULTS=...`), the
backend records two extra files alongside the C10/C11 evidence so a
maintainer can replay or audit the operator-supplied define-stage
bodies. The deterministic `scripted-execute` / `scripted-finalize`
backends never write these files.

| File                                  | Source                                                                                  | Purpose                                                                                      |
| ------------------------------------- | --------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `operator-results.snapshot.json`      | copy of the `--operator-results <path>` JSON the runner consumed                        | Preserves the exact replay input for cross-referencing against `slice-has-*` failures. The runner takes the snapshot BEFORE driving the slice loop. Schema: [`.cursor/schemas/operator-results.schema.json`](../../../../.cursor/schemas/operator-results.schema.json). |
| `agent-actions.jsonl`                 | runner-emitted log of one record per `AgentPhaseDriver`-mediated artifact write or git step | Self-describes the per-slice define / build / merge work the agent backend did, mirroring `stub-actions.jsonl`'s shape so the same evidence-walker tools work across both backends. |
| `transcript.md`                       | when the (deferred) Cursor SDK driver path is wired up, this file carries the captured assistant transcript per slice | Drift-debugging input only; the assertion handlers never read this file as oracle. |

### Stage: recorded-replay (C15) — *captured by `recorded`*

When the suite is driven through the `recorded` backend
(`make acceptance-cross-repo-recorded-smoke`), the backend re-bootstraps
the hub via `prepareScriptedHub` and replays every `RecordedAction`
in the configured trace file (default
[`acceptance/recorded/rm01-cross-repo/baseline.jsonl`](../../../recorded/rm01-cross-repo/baseline.jsonl)).
The C10/C11 stages above do not run; the recorded backend only
re-issues the recorded `specify` argv set against the live binary
and pins exit codes. See
[`acceptance/runner/backends/README.md` §Recorded Backend (C15)](../../../runner/backends/README.md#recorded-backend-c15)
for the framing.

| File                                  | Source                                                                                  | Purpose                                                                                      |
| ------------------------------------- | --------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `replayed-actions.jsonl`              | runner-emitted log of one `replayed-actions-header` line + one record per recorded action | Self-describes the C15 replay outcome. Each record carries `{step, recorded, replayed, outcome, faultDomain?, note?}`. The header carries the source trace path, schema version, source-run header (when present), action count, replayed-command count, synthetic-skipped count, first mismatch, and the optional final-state declaration. |
| `registry.yaml`                       | snapshot from after `prepareScriptedHub`                                                | Same shape as the C07 setup capture; lets `recorded-trace-final-state-matches` confirm the post-replay tree includes the registry. |
| `git/hub.log`                         | runner-collected `git log` snapshot of `<temp-root>/shop-platform`                      | Confirms the replayed `change plan` argvs landed the same authoring trail as the source run. |

The replay never exercises `workspace push` / `change finalize` /
fake-`gh` mark-merged today, so `push-output.json`,
`finalize-output*.json`, `fake-gh/prs.json`, and the workspace clone
git logs are absent on a recorded-smoke run. A future trace that
records those argvs would resurrect the corresponding evidence files
without any backend changes.

### Stage: finalize (C11) — *captured by `scripted-finalize`*

After the optional pre-merge negative probe, the first-call
`specify change finalize`, and the second-call idempotency probe:

| File                                  | Source                                                                                  | Purpose                                                                                      |
| ------------------------------------- | --------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `finalize-output.pre-merge.json`      | optional pre-merge `specify --format json change finalize` probe                        | Asserts the `finalize-runs-before-prs-merged` negative expectation. The file carries `{ "exit-code": <n>, "output": <json> }`. Only written when the backend exercised the probe. |
| `finalize-output.json`                | first-call `specify --format json change finalize` after PRs are merged                 | Asserts `finalized: true`, all routed projects `merged`, `summary.merged` matches, and the archived plan path is real. Pinned by `finalize-output-json-shape-clean`. |
| `finalize-output.second-call.json`    | second-call `specify --format json change finalize`                                     | Asserts `finalize-second-call-returns-plan-not-found` (`error: plan-not-found`). The file carries `{ "exit-code": <n>, "output": <json> }`. |
| `git/hub.log`                         | refresh after finalize                                                                  | Records the archive commit and confirms `plan.yaml` is gone from the hub root.               |
| `plan.yaml.before-finalize`           | snapshot of `<temp-root>/shop-platform/plan.yaml` taken AFTER execute, BEFORE finalize  | Preserves the pre-finalize plan state for debugging — the live file is gone post-finalize.  |

## Retention

Per the
[Run Evidence Policy](../../../README.md#run-evidence-policy):

- **Pass:** the entire `<run-root>/` is discarded.
- **Failure:** the entire `<run-root>/` is preserved automatically so a
  maintainer can read every file above without re-running the suite.
- **`--preserve` opt-in:** preserves on pass too, for operator
  inspection.

## Pointers

- Shared evidence shape: [`acceptance/README.md` §Run Evidence Policy](../../../README.md#run-evidence-policy).
- Runner contract: [`acceptance/runner/README.md`](../../../runner/README.md).
- Layer 0 reference: `specify-cli/tests/cross_repo.rs` shows the same
  CLI surfaces that produce the JSON captured above.
