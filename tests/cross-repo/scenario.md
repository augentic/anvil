---
id: cross-repo-contract-flow
owner: cross-repo
kind: suite
backend: manual
entrypoint: /spec:plan
stages: [plan, refine, build, merge]
isolation: fresh-project
assertions:
  - plan-exists
  - plan-validates
  - contract-slice-first
  - implementation-slices-routed
  - dependencies-contract-before-implementations
  - draft-stops-at-handoff
  - review-step-no-op
  - execute-loop-all-done
  - workspace-branches-prepared
  - finalize-halts-on-unmerged-prs
  - finalize-archives-plan
  - archived-plan-path-recorded
  - archived-change-md-present
  - merged-pr-list-recorded
  - rerun-finalize-plan-not-found
expected-artifacts:
  - plan.yaml
  - registry.yaml
  - .specify/workspace
  - .specify/archive/plans
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
---

# Cross-Repo Contract Flow

Scenario ID: `cross-repo-contract-flow`

Use this scenario to manually verify the simplest cross-repo happy path: a short
feature brief becomes one contract slice and two routed implementation slices,
the operator reviews the draft plan, the change executes, and the
`/spec:finalize` skill drives push, PR observation, and archive after the
project branches are merged externally.

The scenario exercises the three-skill `draft → review → execute → finalize`
lifecycle (RFC-23) end-to-end and pins the durable end-state outcome (merged
PRs, archived plan, archived `change.md`). It is deliberately manual. It does
not introduce a test runner, fake forge, recorded transcript, CI target, or
golden output comparison.

## Intent

Prove that the operator-facing Specify workflow can drive the cross-repo
contract-first path end to end through the three-skill change lifecycle, and
that it produces the same final state the retired `orchestrate` umbrella did:
identical archived plan path shape, identical merged-PR list (one PR per routed
project), identical archived `change.md` content next to the archived
`plan.yaml`.

```text
feature brief
  -> /spec:plan <name>
  -> contract slice
  -> routed backend and mobile implementation slices
  -> (operator review pause: specify plan status)
  -> /spec:execute loop
  -> /spec:finalize <name>      (halts on unmerged PRs)
  -> operator merges PRs externally
  -> /spec:finalize <name>      (archives the plan)
```

The scenario checks durable structure and state transitions only. It should not
fail because generated prose or implementation code differs from a previous
run.

## Workspace

- **Suite:** cross-repo.
- **Project shape:** one temporary registry-only hub plus two temporary
  registered projects.
- **Hub adapter:** none; initialize the hub with `specify init --hub`.
- **Backend project adapter:** `omnia@v1`.
- **Mobile project adapter:** `vectis@v1`.
- **Registry shape:** the hub registry contains exactly the backend and mobile
  projects for this run.
- **Isolation:** `fresh-project`. Use disposable directories and start with
  empty Specify state.
- **Backend:** `manual` - a human or agent follows this script and records
  results in the [run summary](run-summary-template.md).

Prerequisites:

- A current `specify` binary available on `PATH`, or `SPECIFY_BIN` documented in
  the run summary if the operator uses an explicit binary.
- The `contracts@v1`, `omnia@v1`, and `vectis@v1` adapters are resolvable in
  the local development environment.
- Git is available for local branches and remotes. The backend and mobile
  projects should be Git repositories with an `origin` remote configured before
  `/spec:finalize` if the operator wants to exercise PR/MR creation.
- Forge interaction is whatever the operator normally uses for this workspace.
  Do not add fake `gh` or fake forge behavior for this scenario. The
  `/spec:finalize` skill observes PR state via `gh pr list` and never merges
  PRs itself; merges happen through the operator's normal forge workflow
  between the first and second `/spec:finalize` invocations.

## Inputs

Create a short feature brief in the hub workspace at `docs/oauth-login.md`:

```markdown
# OAuth Login

The shop platform needs OAuth login so mobile customers can sign in with an
external identity provider.

## Participants

- shop-backend: owns token exchange and session creation
- shop-mobile: owns the sign-in screen and callback handling
- identity-provider: external OAuth provider

## Contract

Define a shared OAuth login contract before implementation begins.

HTTP endpoints:

1. POST /oauth/exchange
   - Request OAuthExchangeRequest:
     - provider: string, required, enum: apple, google
     - authorization_code: string, required
     - redirect_uri: string, required
     - code_verifier: string, required
   - 200 response OAuthSession:
     - access_token: string
     - refresh_token: string
     - expires_at: date-time
     - user_id: string
   - 400 ErrorResponse for invalid input
   - 401 ErrorResponse when the provider rejects the code

2. POST /oauth/refresh
   - Request OAuthRefreshRequest:
     - refresh_token: string, required
   - 200 response OAuthSession
   - 401 ErrorResponse when the refresh token is invalid or expired

## Backend implementation

The backend should validate requests, call the identity provider, create or
update the local user session, and return the shared response contract.

## Mobile implementation

The mobile client should present provider choices, launch the OAuth flow, handle
the callback, and call the backend exchange endpoint using the shared contract.
```

## Invocation

### 1. Prepare disposable projects

Create three disposable directories:

```text
cross-repo-shop-platform/
cross-repo-shop-backend/
cross-repo-shop-mobile/
```

Initialize them:

```bash
cd cross-repo-shop-platform
specify init --hub

cd ../cross-repo-shop-backend
specify init omnia@v1

cd ../cross-repo-shop-mobile
specify init vectis@v1
```

Return to the hub and register the implementation projects. Use descriptions
that make routing unambiguous:

```bash
cd ../cross-repo-shop-platform
specify registry add shop-backend --url ../cross-repo-shop-backend --schema omnia@v1 --description "Omnia backend service for OAuth token exchange, sessions, and provider integration."
specify registry add shop-mobile --url ../cross-repo-shop-mobile --schema vectis@v1 --description "Vectis mobile client for OAuth sign-in UI, callback handling, and API consumption."
specify registry validate
```

Create `docs/oauth-login.md` from the **Inputs** section.

### 2. Draft the change

Run `/spec:plan` from the hub:

```text
/spec:plan oauth-login source brief=docs/oauth-login.md

Draft a cross-repo OAuth login change from docs/oauth-login.md.

Expected shape:
- one contract slice that defines the shared OAuth HTTP and schema contracts
- one backend implementation slice routed to shop-backend
- one mobile implementation slice routed to shop-mobile
- both implementation slices depend on the contract slice

Keep the plan small and happy-path only.
```

The draft skill writes `change.md` and `plan.yaml`, runs the brief pipeline,
runs `specify plan validate`, and stops at the hand-off summary. It must not
proceed into execution. After the hand-off, the operator drives the next stage.

### 3. Review the draft (operator pause)

This is the explicit human seam introduced by RFC-23. Inspect the draft plan
without modifying it:

```bash
specify plan validate
specify plan status
```

The review step is a no-op for parity with the retired umbrella: the operator
observes `plan.yaml`, confirms the slice shape matches the draft hand-off
summary, and proceeds. If the operator needs to edit the plan, they would run
`specify plan amend` here; for the parity scenario the plan is accepted as
authored.

### 4. Execute the plan

Run the supervised execution loop from the hub:

```text
/spec:execute loop
```

The operator may answer ordinary clarification prompts if they are needed to
complete the slices. Do not change this scenario into an automated runner to
remove those prompts.

When the loop exits, inspect status:

```bash
specify plan status
specify workspace status
```

Every plan entry should be `done`. The execute loop exits because the plan is
complete (`all-done`), not because it is stuck, failed, or interrupted.

### 5. Finalize — first invocation (halts on unmerged PRs)

Run `/spec:finalize` from the hub:

```text
/spec:finalize oauth-login
```

The skill executes:

1. Pre-flight (`<change-name>` kebab-case, `plan.yaml` present).
2. Plan terminality (every entry `done`).
3. `specify workspace push` — pushes the prepared `specify/oauth-login`
   branches to backend and mobile remotes; surfaces the per-project status
   table verbatim.
4. `gh pr list --head specify/oauth-login --state all --json
   number,state,merged,headRefName,url` — observes PR state per project.

On the first invocation, the freshly opened PRs are not yet merged, so the
skill halts with `pr-not-merged`, naming each open PR with its URL. Record the
PR numbers and URLs in the run summary.

### 6. Merge externally

Merge the backend and mobile PRs using the normal operator workflow for the
environment under test. This can be a real forge merge, a local remote merge in
a disposable environment, or another documented operator action.

Do not add fake forge behavior to the repository as part of this scenario. The
`/spec:finalize` skill never merges PRs itself — operator merge between the
two finalize invocations is the design.

### 7. Finalize — second invocation (archives the plan)

Re-run `/spec:finalize` from the hub:

```text
/spec:finalize oauth-login
```

The second invocation re-runs every step. `specify workspace push` reports
`up-to-date` for both projects (idempotent re-entry). `gh pr list` reports
every PR as `MERGED`. The skill then runs `specify plan finalize`, which
archives `plan.yaml` and `change.md` together under
`.specify/archive/plans/oauth-login-<date>/` (or the equivalent dated archive
path the verb produces). The wrap-up summary prints the merged-PR list and the
archived plan path.

Run `/spec:finalize` a third time:

```text
/spec:finalize oauth-login
```

This re-entry should report `plan-not-found` from `specify plan finalize` and
exit 0 — the change is already archived.

## Expected Artifacts

The run should leave these artifacts or states for inspection:

- `registry.yaml` exists in the hub and contains `shop-backend` and
  `shop-mobile`.
- `plan.yaml` exists after `/spec:plan` and validates cleanly.
- The plan has exactly one contract slice and two implementation slices.
- The contract slice targets the contract adapter and has no routed
  implementation project.
- The backend implementation slice routes to `shop-backend`.
- The mobile implementation slice routes to `shop-mobile`.
- Both implementation slices depend on the contract slice.
- `.specify/workspace/shop-backend/` and `.specify/workspace/shop-mobile/`
  exist after sync or execution preparation.
- Prepared project branches use `specify/oauth-login`.
- The execute loop reaches `all-done`.
- The first `/spec:finalize oauth-login` invocation runs `specify workspace
  push` (creating or updating PRs/MRs for both routed projects, or the local
  equivalent documented by the operator) and halts with `pr-not-merged`.
- The second `/spec:finalize oauth-login` invocation, after external merges,
  archives the plan under `.specify/archive/plans/`.
- The archived directory contains both the archived `plan.yaml` and the
  archived `change.md` for the change.
- The wrap-up summary names every merged PR (one per routed project) with its
  URL.
- A third `/spec:finalize oauth-login` invocation reports `plan-not-found`.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `plan-validates`: `specify plan validate` exits cleanly after the draft
  hand-off and again during the operator review.
- `contract-slice-first`: the dependency graph makes the contract slice the
  first executable slice.
- `implementation-slices-routed`: exactly two implementation slices route to
  the expected projects, `shop-backend` and `shop-mobile`.
- `dependencies-contract-before-implementations`: each implementation slice
  depends on the contract slice.
- `draft-stops-at-handoff`: `/spec:plan` exits at the hand-off summary
  without invoking `/spec:execute`, pushing branches, or finalizing the
  change.
- `review-step-no-op`: `specify plan status` between draft and execute reports
  the plan as authored; the operator does not run `specify plan amend` for the
  parity scenario.
- `execute-loop-all-done`: `/spec:execute loop` exits because the plan is
  complete, not because it is stuck, failed, or interrupted.
- `workspace-branches-prepared`: routed project work happens on
  `specify/oauth-login` branches.
- `finalize-halts-on-unmerged-prs`: the first `/spec:finalize oauth-login`
  invocation runs push successfully and halts with `pr-not-merged` naming both
  PR URLs.
- `finalize-archives-plan`: after external merges, the second
  `/spec:finalize oauth-login` invocation archives the plan via
  `specify plan finalize`.
- `archived-plan-path-recorded`: the wrap-up summary names the archived plan
  path under `.specify/archive/plans/`, matching the umbrella's archive shape.
- `archived-change-md-present`: the archived directory next to the archived
  `plan.yaml` contains the archived `change.md` for the change.
- `merged-pr-list-recorded`: the wrap-up summary lists exactly two merged PRs,
  one for `shop-backend` and one for `shop-mobile`, with their numbers and
  URLs.
- `rerun-finalize-plan-not-found`: a third `/spec:finalize oauth-login`
  reports `plan-not-found` and exits 0.

## Negative Expectations

These are the guardrails for this first cross-repo pass:

- `automated-runner-added`: this scenario pack must not add a Deno, Rust,
  shell, Cursor SDK, or other automated test runner.
- `fake-forge-added`: this scenario pack must not add fake `gh`, fake GitHub,
  or fake forge behavior. PR merges between the two `/spec:finalize`
  invocations are operator actions through the normal forge workflow.
- `transcript-replay-added`: this scenario pack must not require recorded agent
  transcripts or replay fixtures.
- `ci-target-added`: this scenario pack must not add a CI job, `make` target,
  or required automated acceptance check.
- `golden-output-required`: this scenario pack must not require byte-for-byte
  generated prose, code, or transcript comparisons. Parity with the retired
  umbrella is asserted on durable structure (archive path shape, merged-PR
  count and project mapping, archive directory contents), not on byte-stable
  output.

## Cleanup

Use disposable directories and remove them when the run is complete unless a
failure needs investigation. Preserve these items on failure:

- completed [run summary](run-summary-template.md)
- `docs/oauth-login.md`
- `registry.yaml`
- `plan.yaml`, or the archived plan path if finalize succeeded
- `specify plan status` output (review step and post-execute)
- `specify workspace status` output
- first `/spec:finalize` output (push table + `pr-not-merged` halt)
- second `/spec:finalize` output (push idempotent + PR observation +
  finalize wrap-up summary)
- third `/spec:finalize` output (`plan-not-found` re-entry)
- relevant backend and mobile branch or PR/MR identifiers
