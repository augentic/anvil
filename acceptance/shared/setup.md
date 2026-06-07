# Shared scenario setup

Reusable environment setup for the platform acceptance scenarios. Individual scenarios link here for the common steps and inline only the delta that is specific to them (a different brief, a different adapter, an injected fault).

Run every scenario from the repo-local sandbox at `acceptance/.sandbox/<scenario>/` (gitignored). Pin it instead of an ad-hoc `mktemp` root so the tree is stable across runs, survives reboots, and is browsable in the IDE — add `acceptance/.sandbox/` as a second Cursor workspace folder to watch `.specify/`, `plan.yaml`, and `journal.jsonl` populate live. Isolation comes from recreating the directory at the start of each run, not from a unique suffix. Override the base with `SPECIFY_SANDBOX=/abs/path` if you want it outside the repo. These are throwaway projects, branches, and Specify state — never run a scenario from an important working tree. To inspect what a run produced, see [`inspect.md`](inspect.md).

## Prerequisites

- A `specify` binary on your PATH that is the build under test. `make acceptance` builds one and symlinks it into `~/.local/bin` (override with `INSTALL_DIR=…`), so the bare `specify` commands below resolve to this build with no further setup — it warns if `~/.local/bin` is not on your PATH. An agent driving the sweep self-heals this: if `specify --version` does not resolve to the build under test, prepend the symlink dir to PATH for its own shells (`export PATH="$HOME/.local/bin:$PATH"`) or call the absolute `../specify-cli/target/release/specify` path. To build without `make`, run `cargo build --release --manifest-path ../specify-cli/Cargo.toml --bin specify` and symlink `../specify-cli/target/release/specify` into a PATH directory yourself. Confirm the right build with `specify --version` before starting. To test a different binary instead, put it earlier on your PATH.
- The adapters a scenario names (`omnia@v1`, `vectis@v1`, `contracts@v1`) are resolvable. The first-party shorthand (`specify init omnia@v1`) fetches the published adapter from GitHub, so `init` needs network access. To run fully offline, pass a local adapter path instead — `specify init ./adapters/targets/omnia` (or a `file://` URI) against a checkout of this framework repo. There is no `SPECIFY_ROOT`-style environment fallback; resolution after `init` reads the project-local manifest cache that `init` populates.
- Git is available for local branches and remotes. For scenarios that exercise PR/MR creation, the routed projects should be Git repositories with an `origin` remote configured before `/spec:finalize`.
- Do not add fake `gh` or fake forge behavior. `/spec:finalize` observes PR state via `gh pr list` and never merges PRs itself; merges happen through the operator's normal forge workflow.

## Single-project setup

For scenarios that run against one initialized project (no registry):

```bash
SANDBOX="${SPECIFY_SANDBOX:-$(git rev-parse --show-toplevel)/acceptance/.sandbox}/<scenario>"
rm -rf "$SANDBOX" && mkdir -p "$SANDBOX" && cd "$SANDBOX"
specify init <adapter>     # e.g. omnia@v1
```

Then create the scenario's brief (see its **Setup** section) and run its **Invocation**.

## Cross-repo workspace setup

For scenarios that coordinate work across multiple project repos from a registry-only workspace:

Create three disposable directories under the pinned sandbox:

```text
acceptance/.sandbox/<scenario>/shop-platform/   # registry-only workspace
acceptance/.sandbox/<scenario>/shop-backend/    # omnia@v1 project
acceptance/.sandbox/<scenario>/shop-mobile/     # vectis@v1 project
```

Initialize them:

```bash
SANDBOX="${SPECIFY_SANDBOX:-$(git rev-parse --show-toplevel)/acceptance/.sandbox}/<scenario>"
rm -rf "$SANDBOX" && mkdir -p "$SANDBOX"/{shop-platform,shop-backend,shop-mobile} && cd "$SANDBOX"

cd shop-platform
specify init --workspace

cd ../shop-backend
specify init omnia@v1

cd ../shop-mobile
specify init vectis@v1
```

Return to the workspace and register the implementation projects with descriptions that make routing unambiguous:

```bash
cd ../shop-platform
specify registry add shop-backend --url ../shop-backend --schema omnia@v1 --description "Omnia backend service for OAuth token exchange, sessions, and provider integration."
specify registry add shop-mobile --url ../shop-mobile --schema vectis@v1 --description "Vectis mobile client for OAuth sign-in UI, callback handling, and API consumption."
specify registry validate
```

Then create the brief (below) and run the scenario's **Invocation**.

## OAuth login brief

The cross-repo and contract-routing scenarios use this brief. Create it in the workspace at `docs/oauth-login.md`:

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

## Recording the run

Capture each run with [`run-summary-template.md`](run-summary-template.md), filed under [`acceptance/runs/`](../runs/README.md), then update the scenario's status in the [catalog](../scenarios/README.md).
