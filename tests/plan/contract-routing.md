---
id: contract-routing
owner: plan
kind: suite
backend: manual
entrypoint: /spec:plan
stages: [plan]
isolation: fresh-project
assertions:
  - plan-exists
  - plan-validates
  - contract-slice-present
  - implementation-slices-routed
  - dependencies-correct
  - routing-deterministic
expected-artifacts:
  - plan.yaml
  - registry.yaml
  - .specify/plans/oauth-login-plan/discovery.md
  - .specify/plans/oauth-login-plan/proposal.md
  - .specify/plans/oauth-login-plan/workspace.md
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
---

# Contract Routing Plan Generation

Scenario ID: `contract-routing`

Use this scenario to manually verify the plan-generation part of the cross-repo contract-first path: a short feature brief becomes one contract slice and routed implementation slices, without executing, pushing, or finalizing.

This scenario is deliberately manual. It does not introduce a test runner, fake forge, recorded transcript, CI target, or golden output comparison.

## Intent

Prove that `/spec:plan` can author a deterministic cross-repo plan from a registry-only workspace:

```text
feature brief
  -> /spec:plan
  -> contract slice
  -> routed backend and mobile implementation slices
  -> specrun plan validate
```

The scenario checks durable plan structure only. It should not fail because the generated proposal prose or slice descriptions differ from a previous run.

## Workspace

- **Suite:** plan.
- **Project shape:** one temporary registry-only workspace plus two temporary registered projects.
- **Hub adapter:** none; initialize the workspace with `specrun init --workspace`.
- **Backend project adapter:** `omnia@v1`.
- **Mobile project adapter:** `vectis@v1`.
- **Registry shape:** the workspace registry contains exactly the backend and mobile projects for this run.
- **Isolation:** `fresh-project`. Use disposable directories and start with empty Specify state.
- **Backend:** `manual` - a human or agent follows this script and records results in the [run summary](run-summary-template.md).

Prerequisites:

- A current `specify` binary available on `PATH`, or `SPECIFY_BIN` documented in the run summary if the operator uses an explicit binary.
- The `contracts@v1`, `omnia@v1`, and `vectis@v1` adapters are resolvable in the local development environment.
- Git is available if the local `specrun workspace sync` path needs repository metadata. Do not add fake `gh` or fake forge behavior for this scenario.

## Inputs

Create a short feature brief in the workspace workspace at `docs/oauth-login.md`:

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
plan-shop-platform/
plan-shop-backend/
plan-shop-mobile/
```

Initialize them:

```bash
cd plan-shop-platform
specrun init --workspace

cd ../plan-shop-backend
specrun init omnia@v1

cd ../plan-shop-mobile
specrun init vectis@v1
```

Return to the workspace and register the implementation projects. Use descriptions
that make routing unambiguous:

```bash
cd ../plan-shop-platform
specrun registry add shop-backend --url ../plan-shop-backend --schema omnia@v1 --description "Omnia backend service for OAuth token exchange, sessions, and provider integration."
specrun registry add shop-mobile --url ../plan-shop-mobile --schema vectis@v1 --description "Vectis mobile client for OAuth sign-in UI, callback handling, and API consumption."
specrun registry validate
```

Create `docs/oauth-login.md` from the **Inputs** section.

### 2. Plan the change

Run `/spec:plan` from the workspace:

```text
/spec:plan oauth-login-plan from docs/oauth-login.md

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
specrun plan validate
inspect plan.yaml
specrun registry validate
```

Do not run `/spec:execute`, `specrun workspace push`, or `specrun plan archive`. This scenario ends after plan validation and inspection.

## Expected Artifacts

The run should leave these artifacts or states for inspection:

- `registry.yaml` exists in the workspace and contains `shop-backend` and `shop-mobile` with clear descriptions.
- `plan.yaml` exists after `/spec:plan` and validates cleanly.
- `.specify/plans/oauth-login-plan/discovery.md` records the supplied documentation input.
- `.specify/plans/oauth-login-plan/workspace.md` records the synchronized peer context for routing.
- `.specify/plans/oauth-login-plan/proposal.md` records the proposed contract-first plan shape.
- The plan includes one contract slice and implementation slices for backend and mobile work.
- The backend implementation slice routes to `shop-backend`.
- The mobile implementation slice routes to `shop-mobile`.
- Both implementation slices depend on the contract slice.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `plan-validates`: `specrun plan validate` exits cleanly.
- `contract-slice-present`: the plan includes a contract slice before implementation work begins.
- `implementation-slices-routed`: implementation slices route to the expected projects, `shop-backend` and `shop-mobile`.
- `dependencies-correct`: each implementation slice depends on the contract slice.
- `routing-deterministic`: project assignments match the registry descriptions and do not depend on generated prose wording.

## Negative Expectations

These are the guardrails for this first plan-generation pass:

- `automated-runner-added`: this scenario pack must not add a Deno, Rust, shell, Cursor SDK, or other automated test runner.
- `fake-forge-added`: this scenario pack must not add fake `gh`, fake GitHub, or fake forge behavior.
- `transcript-replay-added`: this scenario pack must not require recorded agent transcripts or replay fixtures.
- `ci-target-added`: this scenario pack must not add a CI job, `make` target, or required automated acceptance check.
- `golden-output-required`: this scenario pack must not require byte-for-byte generated prose, code, plan text, or transcript comparisons.

## Cleanup

Use disposable directories and remove them when the run is complete unless a failure needs investigation. Preserve these items on failure:

- completed [run summary](run-summary-template.md)
- `docs/oauth-login.md`
- `registry.yaml`
- `plan.yaml`
- `.specify/plans/oauth-login-plan/`
- `specrun plan validate` output
- `inspect plan.yaml` output
- `specrun registry validate` output
