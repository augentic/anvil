# Shared scenario setup

Reusable environment setup for the platform eval scenarios. Individual scenarios link here for the common steps and inline only the delta that is specific to them (a different brief, a different adapter, an injected fault).

Run every scenario from the repo-local sandbox at `evals/.sandbox/<scenario>/` (gitignored). Pin it instead of an ad-hoc `mktemp` root so the tree is stable across runs, survives reboots, and is browsable in the IDE — add `evals/.sandbox/` as a second Cursor workspace folder to watch `.specify/`, `plan.yaml`, and `journal.jsonl` populate live. Isolation comes from recreating the directory at the start of each run, not from a unique suffix. Override the base with `SPECIFY_SANDBOX=/abs/path` if you want it outside the repo. These are throwaway projects, branches, and Specify state — never run a scenario from an important working tree. To inspect what a run produced, see [`inspect.md`](inspect.md).

## Prerequisites

- A `specify` binary on your PATH that is the build under test. The sweep needs a binary **built from the in-tree source**. `make install-cli` builds the binary and symlinks `target/release/specify` into `~/.local/bin` (overridable with `INSTALL_DIR=`), so the bare `specify` commands below resolve to this build with no further setup — it warns if that directory is not on your PATH. An agent driving the sweep self-heals this: if `specify --version` does not resolve to the build under test, prepend the symlink dir to PATH for its own shells (`export PATH="$HOME/.local/bin:$PATH"`) or call the absolute `target/release/specify` path. To build without `make`, run `cargo build --release --bin specify` and symlink `target/release/specify` into a PATH directory yourself. Confirm the right build with `specify --version` before starting. To test a different binary instead, put it earlier on your PATH.
- The adapters a scenario names (`omnia@1.0.0`, `vectis@1.0.0`, `contracts@1.0.0`) are resolvable. The versioned first-party shorthand (`specify init omnia@1.0.0`) installs the published component into the global adapter store through the wasm-pkg registry transport, so `init` needs network access on a cold store (`specify adapters sync` re-hydrates later; `--frozen` refuses to fetch). To run fully offline, pass a local `.wasm` component instead — `specify init <sibling>/target/wasm32-wasip2/release/omnia.wasm` against a release-built `augentic/specify-adapters` checkout (`cargo make release` there); init mirrors it into the project component cache. Bare names (`specify init omnia`) are the development shorthand resolving the sibling/in-repo release build. Pin the in-tree workflow guest as the core with `SPECIFY_CORE_PATH` so nothing hydrates `specify:core` from the registry.
- Source adapters a scenario binds (`documentation`, `typescript`, …) are components too: a pinned identity in `plan.yaml` resolves the global store entry; a bare name resolves the sibling/in-repo release build. `plan author` runs before `plan.yaml` binds sources, so dev runs give the deployment its source adapters through a project-root `omnia.toml` listing the release-built components (the drivers write one; see the checked-in repo-root [`omnia.toml`](../../omnia.toml) for the shape). Confirm a binding with `specify source resolve <name>`.
- Git is available for local branches and remotes. Scenarios that run `/spec:finalize` push the prepared `specify/<change>` branch to each routed project's `origin`, so every routed project needs a reachable `origin` remote before finalize. A **local bare repository** (`git init --bare`) reached via a `file://` URL satisfies this with no network or forge — see the cross-repo setup below.
- No forge client (`gh`) is required. `/spec:finalize` pushes branches and then archives the plan; it never creates, observes, or merges pull requests. Opening the PR and merging it is an operator action done entirely outside Specify.
- The replay drivers under [`drivers/`](../drivers/README.md) need `jq` on PATH (the only structured-data dependency). They are bash 3.2-compatible, so the stock macOS `/bin/bash` works — no Homebrew bash 4 required.

## Single-project setup

For scenarios that run against one initialized project (no registry):

```bash
SANDBOX="${SPECIFY_SANDBOX:-$(git rev-parse --show-toplevel)/evals/.sandbox}/<scenario>"
rm -rf "$SANDBOX" && mkdir -p "$SANDBOX" && cd "$SANDBOX"
specify init <adapter>     # e.g. omnia@1.0.0
```

Then create the scenario's brief (see its **Setup** section) and run its **Invocation**.

## Cross-repo workspace setup

For scenarios that coordinate work across multiple project repos from a registry-only workspace:

Create the disposable project directories under the pinned sandbox. Every project the plan routes a slice to must be a registered project with its own remote — the contract-routing scenario routes a contract slice to a dedicated `contracts` project, so it is set up alongside `backend` and `mobile`:

```text
evals/.sandbox/<scenario>/platform/    # registry-only workspace
evals/.sandbox/<scenario>/backend/     # omnia@1.0.0 project
evals/.sandbox/<scenario>/mobile/      # vectis@1.0.0 project
evals/.sandbox/<scenario>/contracts/   # contracts@1.0.0 project (contract-routing scenarios)
```

Initialize them:

```bash
SANDBOX="${SPECIFY_SANDBOX:-$(git rev-parse --show-toplevel)/evals/.sandbox}/<scenario>"
rm -rf "$SANDBOX" && mkdir -p "$SANDBOX"/{platform,backend,mobile,contracts} && cd "$SANDBOX"

cd platform
specify init --workspace

cd ../backend
specify init omnia@1.0.0

cd ../mobile
specify init vectis@1.0.0

cd ../contracts
specify init contracts@1.0.0
```

Give each routed project a local bare-repo `origin` so `specify workspace prepare` can resolve `origin/HEAD` and `/spec:finalize` has somewhere to push — no network, no forge. (Drop `contracts` from the list for scenarios that do not route a contract slice.)

```bash
cd "$SANDBOX"
for proj in backend mobile contracts; do
  git -C "$proj" init -b main -q
  git -C "$proj" add -A
  git -C "$proj" diff --cached --quiet || git -C "$proj" commit -q --no-gpg-sign -m "init $proj"
  git init --bare -q "$proj-origin.git"
  git -C "$proj" remote add origin "file://$SANDBOX/$proj-origin.git"
  git -C "$proj" push -q -u origin main
done
```

Return to the workspace and register the projects with descriptions that make routing unambiguous:

```bash
cd platform
specify registry add backend --url ../backend --adapter omnia --description "Omnia backend service for OAuth token exchange, sessions, and provider integration."
specify registry add mobile --url ../mobile --adapter vectis --description "Vectis mobile client for OAuth sign-in UI, callback handling, and API consumption."
specify registry add contracts --url ../contracts --adapter contracts --description "Shared OAuth login API contracts for cross-repo consumption."
specify registry validate
```

Then create the brief (below) and run the scenario's **Invocation**. After `/spec:finalize` pushes the `specify/<change>` branches to the bare repos, opening and merging the pull requests is an operator step done by hand outside Specify (for a local bare repo, that is a plain `git merge --no-ff` into the bare repo's default branch if you want to model the merge).

## OAuth login brief

The cross-repo and contract-routing scenarios use this brief. Create it in the workspace at `docs/oauth-login.md`:

```markdown
# OAuth Login

The platform needs OAuth login so mobile customers can sign in with an
external identity provider.

## Participants

- backend: owns token exchange and session creation
- mobile: owns the sign-in screen and callback handling
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

Capture each run with [`run-template.md`](run-template.md) as `evals/runs/<id>.<result>.md`, then update the scenario's status in the [catalog](../scenarios/README.md).
