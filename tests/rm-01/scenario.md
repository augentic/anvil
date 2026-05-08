---
id: rm-01-cross-repo-contract-flow
owner: rm-01
kind: suite
backend: manual
entrypoint: /change:plan
stages: [define, build, merge]
isolation: fresh-project
assertions:
  - plan-exists
  - plan-validates
  - contract-slice-first
  - implementation-slices-routed
  - dependencies-contract-before-implementations
  - execute-loop-all-done
  - workspace-branches-prepared
  - push-created-prs
  - finalize-archives-plan
  - rerun-finalize-plan-not-found
expected-artifacts:
  - plan.yaml
  - registry.yaml
  - .specify/workspace
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
---

# RM-01 Cross-Repo Contract Flow

Scenario ID: `rm-01-cross-repo-contract-flow`

Use this scenario to manually verify the simplest RM-01 happy path: a short
feature brief becomes one contract slice and two routed implementation slices,
then the change executes, pushes project branches, and finalizes after the
project branches are merged.

This scenario is deliberately manual. It does not introduce a test runner, fake
forge, recorded transcript, CI target, or golden output comparison.

## Intent

Prove that the operator-facing Specify workflow can drive the cross-repo
contract-first path end to end:

```text
feature brief
  -> /change:plan
  -> contract slice
  -> routed backend and mobile implementation slices
  -> /change:execute loop
  -> workspace push
  -> operator merge
  -> specify change finalize
```

The scenario checks durable structure and state transitions only. It should not
fail because generated prose or implementation code differs from a previous
run.

## Workspace

- **Suite:** RM-01.
- **Project shape:** one temporary registry-only hub plus two temporary
  registered projects.
- **Hub capability:** none; initialize the hub with `specify init --hub`.
- **Backend project capability:** `omnia@v1`.
- **Mobile project capability:** `vectis@v1`.
- **Registry shape:** the hub registry contains exactly the backend and mobile
  projects for this run.
- **Isolation:** `fresh-project`. Use disposable directories and start with
  empty Specify state.
- **Backend:** `manual` - a human or agent follows this script and records
  results in the [run summary](run-summary-template.md).

Prerequisites:

- A current `specify` binary available on `PATH`, or `SPECIFY_BIN` documented in
  the run summary if the operator uses an explicit binary.
- The `contracts@v1`, `omnia@v1`, and `vectis@v1` capabilities are resolvable in
  the local development environment.
- Git is available for local branches and remotes. The backend and mobile
  projects should be Git repositories with an `origin` remote configured before
  `specify workspace push` if the operator wants to exercise PR/MR creation.
- Forge interaction is whatever the operator normally uses for this workspace.
  Do not add fake `gh` or fake forge behavior for this scenario.

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
rm01-shop-platform/
rm01-shop-backend/
rm01-shop-mobile/
```

Initialize them:

```bash
cd rm01-shop-platform
specify init --hub

cd ../rm01-shop-backend
specify init omnia@v1

cd ../rm01-shop-mobile
specify init vectis@v1
```

Return to the hub and register the implementation projects. Use descriptions
that make routing unambiguous:

```bash
cd ../rm01-shop-platform
specify registry add shop-backend --url ../rm01-shop-backend --schema omnia@v1 --description "Omnia backend service for OAuth token exchange, sessions, and provider integration."
specify registry add shop-mobile --url ../rm01-shop-mobile --schema vectis@v1 --description "Vectis mobile client for OAuth sign-in UI, callback handling, and API consumption."
specify registry validate
```

Create `docs/oauth-login.md` from the **Inputs** section.

### 2. Plan the change

Run `/change:plan` from the hub:

```text
/change:plan oauth-login source brief=docs/oauth-login.md

Plan a cross-repo OAuth login change from docs/oauth-login.md.

Expected shape:
- one contract slice that defines the shared OAuth HTTP and schema contracts
- one backend implementation slice routed to shop-backend
- one mobile implementation slice routed to shop-mobile
- both implementation slices depend on the contract slice

Keep the plan small and happy-path only.
```

After planning, validate and inspect the plan:

```bash
specify change plan validate
specify change plan status
```

### 3. Execute the plan

Run the supervised execution loop from the hub:

```text
/change:execute loop
```

The operator may answer ordinary clarification prompts if they are needed to
complete the slices. Do not change this scenario into an automated runner to
remove those prompts.

When the loop exits, inspect status:

```bash
specify change plan status
specify workspace status
```

### 4. Push project branches

Push prepared workspace branches from the hub:

```bash
specify workspace push
```

Record the branch or PR/MR identifiers in the run summary.

### 5. Merge externally

Merge the backend and mobile branches using the normal operator workflow for
the environment under test. This can be a real forge merge, a local remote
merge in a disposable environment, or another documented operator action.

Do not add fake forge behavior to the repository as part of this scenario.

### 6. Finalize

Return to the hub and finalize:

```bash
specify change finalize
```

Run finalize a second time:

```bash
specify change finalize
```

The second run should report `plan-not-found`.

## Expected Artifacts

The run should leave these artifacts or states for inspection:

- `registry.yaml` exists in the hub and contains `shop-backend` and
  `shop-mobile`.
- `plan.yaml` exists after `/change:plan` and validates cleanly.
- The plan has exactly one contract slice and two implementation slices.
- The contract slice targets the contract capability and has no routed
  implementation project.
- The backend implementation slice routes to `shop-backend`.
- The mobile implementation slice routes to `shop-mobile`.
- Both implementation slices depend on the contract slice.
- `.specify/workspace/shop-backend/` and `.specify/workspace/shop-mobile/`
  exist after sync or execution preparation.
- Prepared project branches use `specify/oauth-login`.
- The execute loop reaches `all-done`.
- `specify workspace push` creates or updates project PRs/MRs, or the local
  equivalent documented by the operator.
- `specify change finalize` archives the completed plan after the project
  branches are merged.
- A second `specify change finalize` reports `plan-not-found`.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/change:plan`.
- `plan-validates`: `specify change plan validate` exits cleanly.
- `contract-slice-first`: the dependency graph makes the contract slice the
  first executable slice.
- `implementation-slices-routed`: exactly two implementation slices route to
  the expected projects, `shop-backend` and `shop-mobile`.
- `dependencies-contract-before-implementations`: each implementation slice
  depends on the contract slice.
- `execute-loop-all-done`: `/change:execute loop` exits because the plan is
  complete, not because it is stuck, failed, or interrupted.
- `workspace-branches-prepared`: routed project work happens on
  `specify/oauth-login` branches.
- `push-created-prs`: `specify workspace push` creates or updates the handoff
  targets for both routed projects.
- `finalize-archives-plan`: after external merges, `specify change finalize`
  archives the plan.
- `rerun-finalize-plan-not-found`: a second finalize reports `plan-not-found`.

## Negative Expectations

These are the guardrails for this first RM-01 pass:

- `automated-runner-added`: this scenario pack must not add a Deno, Rust,
  shell, Cursor SDK, or other automated test runner.
- `fake-forge-added`: this scenario pack must not add fake `gh`, fake GitHub,
  or fake forge behavior.
- `transcript-replay-added`: this scenario pack must not require recorded agent
  transcripts or replay fixtures.
- `ci-target-added`: this scenario pack must not add a CI job, `make` target,
  or required automated acceptance check.
- `golden-output-required`: this scenario pack must not require byte-for-byte
  generated prose, code, or transcript comparisons.

## Cleanup

Use disposable directories and remove them when the run is complete unless a
failure needs investigation. Preserve these items on failure:

- completed [run summary](run-summary-template.md)
- `docs/oauth-login.md`
- `registry.yaml`
- `plan.yaml`, or the archived plan path if finalize succeeded
- `specify change plan status` output
- `specify workspace status` output
- `specify workspace push` output
- `specify change finalize` output
- relevant backend and mobile branch or PR/MR identifiers
