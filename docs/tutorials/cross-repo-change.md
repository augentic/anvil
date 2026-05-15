# Working across repos: planning

Drive a feature spanning a backend and a mobile app from a single platform hub. This tutorial walks the bootstrap-and-plan half of the cross-repo loop: bootstrap a hub, register two code projects, author the change brief, and produce a plan that decomposes the feature across both projects.

## Where you are in the cross-repo loop

The full loop is nine steps. This page covers steps **1-4**.

1. **Initialise the platform hub** (`specify init --hub`)
2. **Register code projects** (`specify registry add`)
3. **Write the change brief** (`specify change create`)
4. **Plan the change** (`/change:plan`)
5. Inspect the workspace
6. Execute the plan (`/change:execute loop`)
7. Push branches and open PRs (`specify workspace push`)
8. Operator merges the PRs
9. Finalize the change (`specify change finalize`)

Steps 5-7 live in the follow-on tutorial [Working across repos: executing](cross-repo-execute.md); Steps 8-9 in [Working across repos: landing](landing-a-change.md).

> **Choosing your topology.** This tutorial uses the platform-hub topology because the feature spans two registered projects -- the hub holds platform state and the code lives in registered repos. If your work is single-repo, the platform-as-project shape (initiating repo with `url: .` in the registry) is simpler; see [Platform repo topologies](../explanation/platform-repo.md) for the comparison and [A Multi-Slice Change](single-repo-change.md) for the single-repo flow.

Every command below should run cleanly against the current `specify` CLI on a freshly-cloned hub. If a step fails, the gap is in the implementation, not the design — file an issue with the failing transcript.

**Prerequisites:**

- [`specify` CLI](../orientation/prerequisites.md) installed and on `PATH`.
- [`gh`](https://cli.github.com/) installed and authenticated against your GitHub org (`gh auth status`).
- A GitHub namespace you can create repos in. The walkthrough uses `org/` — substitute your real org or user.
- Two empty GitHub repos pre-created at `git@github.com:org/shop-backend.git` and `git@github.com:org/shop-mobile.git`. (Or skip pre-creation and let `specify workspace push` greenfield-bootstrap them in Step 7.)
- Familiarity with the [single-repo change tutorial](single-repo-change.md) — `/change:plan`, `/change:execute`, and the plan lifecycle.

## Contents

- [What you will build](#what-you-will-build)
- [Topology — registry-only hub](#topology--registry-only-hub)
- [1. Bootstrap the platform hub](#1-bootstrap-the-platform-hub)
- [2. Register the two projects](#2-register-the-two-projects)
- [3. Author the change brief](#3-author-the-change-brief)
- [4. Plan the change](#4-plan-the-change)
- [Assignment](#assignment)
- [What you learned](#what-you-learned)
- [Next](#next)

## What you will build

A platform hub `shop-platform/` that drives the `oauth-login` change across two registered projects:

| Project | Schema | Domain |
|---|---|---|
| `shop-backend` | `omnia@v1` | User registration, account management, OAuth provider integration, token storage. |
| `shop-mobile` | `vectis@v1` | iOS and Android clients. Login screens, OAuth redirect handling, token refresh. |

The plan that lands has three changes:

```yaml
changes:
  - name: oauth-login-contract     # platform-level contract change
    capability: contracts@v1
    depends-on: []
  - name: add-oauth-tokens         # backend implementation
    project: shop-backend
    depends-on: [oauth-login-contract]
  - name: add-oauth-screens        # mobile implementation
    project: shop-mobile
    depends-on: [oauth-login-contract]
```

The contract change runs against the hub itself; the two implementation changes run inside the workspace clones.

## Topology — registry-only hub

This tutorial uses the [platform-hub topology](../explanation/platform-repo.md). The hub holds platform state and never appears in its own registry. Code lives in registered project repos that materialise under `.specify/workspace/<name>/` (the [tier-2 workspace](../explanation/workspace-tiers.md#the-two-tiers)).

```text
shop-platform/                              # the hub repo (this tutorial's working directory)
├── AGENTS.md                               # generated hub context
├── registry.yaml                           # version: 1, projects: [shop-backend, shop-mobile]
├── change.md                           # operator brief for `oauth-login`
├── plan.yaml                               # the plan authored by /change:plan
└── .specify/
    ├── project.yaml                        # { hub: true, name: shop-platform } (capability: omitted)
    ├── context.lock                        # context freshness fingerprint
    ├── plans/oauth-login/                  # discovery, workspace, proposal markdown
    ├── archive/                            # finalised changes (after Step 9)
    └── workspace/
        ├── shop-backend/                   # tier-2 clone of git@github.com:org/shop-backend.git
        └── shop-mobile/                    # tier-2 clone of git@github.com:org/shop-mobile.git
```

## 1. Bootstrap the platform hub

Create the hub directory, give it a remote, and run `specify init --hub`:

```bash
mkdir shop-platform && cd shop-platform
git init --quiet
git remote add origin git@github.com:org/shop-platform.git

specify init --hub --name shop-platform
```

In hub mode, **no positional** capability argument is passed — `--hub` is the discriminator. Combining a capability positional with `--hub` is rejected with `init-requires-capability-or-hub`. `--name` must be kebab-case because later change commands use it when scaffolding operator-facing artifacts.

<details>
<summary>Expected output</summary>

```text
Initialized .specify/ as a registry-only platform hub
  capability: (none — hub mode)
  config: /…/shop-platform/.specify/project.yaml
  cache present: false
  directories created: /…/shop-platform/.specify
  specify_version: 0.x.y
```

</details>

The hub now has:

```text
shop-platform/
├── AGENTS.md         # generated hub context
├── registry.yaml     # version: 1, projects: []
├── .gitignore        # upserts .specify/.cache/ and .specify/workspace/
└── .specify/
    ├── project.yaml  # hub: true (capability: omitted)
    └── context.lock  # context freshness fingerprint
```

`specify init --hub` does not create `change.md` or `plan.yaml`; those are minted later by `specify change create` (which scaffolds both files together). It refuses to run when `.specify/` already exists. To convert an existing single-repo project into a hub, remove `.specify/` first.

> **Why hub mode?** A hub gets `hub: true` (the validation flag that rejects any registry entry whose `url` is `.`) and **omits** `capability:` (the absence of which is what disables phase pipelines on the hub itself). Together these pin the platform repo's identity unambiguously. See [Platform repo topologies](../explanation/platform-repo.md) for the full contract.

## 2. Register the two projects

Add the backend (Omnia capability):

```bash
specify registry add shop-backend \
    --url git@github.com:org/shop-backend.git \
    --capability omnia@v1 \
    --description "User registration, account management, and the authoritative implementation of the shop's HTTP API. Owns persistence, OAuth provider integration, token storage, and order processing."
```

Add the mobile app (Vectis capability):

```bash
specify registry add shop-mobile \
    --url git@github.com:org/shop-mobile.git \
    --capability vectis@v1 \
    --description "iOS and Android mobile clients for the shop. Owns login and registration screens, the cart, checkout, and OAuth redirect handling. Calls the shop's HTTP API from the user-facing flows."
```

Verify the registry:

```bash
specify registry validate
specify registry show
```

<details>
<summary>Expected <code>specify registry show</code> output</summary>

```yaml
version: 1
projects:
  - name: shop-backend
    url: git@github.com:org/shop-backend.git
    capability: omnia@v1
    description: >
      User registration, account management, and the authoritative
      implementation of the shop's HTTP API. Owns persistence, OAuth
      provider integration, token storage, and order processing.
  - name: shop-mobile
    url: git@github.com:org/shop-mobile.git
    capability: vectis@v1
    description: >
      iOS and Android mobile clients for the shop. Owns login and
      registration screens, the cart, checkout, and OAuth redirect
      handling. Calls the shop's HTTP API from the user-facing flows.
```

</details>

Two invariants the validator just enforced:

- **`description-missing-multi-repo`**: a multi-project registry requires every entry to carry a `description`. Omitting one fails with a diagnostic naming the offending entry.
- **`hub-cannot-be-project`**: a hub repo (`hub: true` in `project.yaml`) rejects any registry entry whose `url` is `.`. A code project always lives in its own repo. See [Validation rules](../explanation/platform-repo.md#validation-rules).

The descriptions matter beyond validation: the assignment step in `/change:plan` (Step 4) infers project routing from registry descriptions. Rich, domain-specific descriptions land clean assignments; sparse descriptions force unresolved (`?`) prompts during planning.

## 3. Author the change brief

Scaffold the brief:

```bash
specify change create oauth-login
```

This rewrites `change.md` with a fresh template named after the change. Edit it to describe the feature and point the discovery brief at any supplementary documentation:

```markdown
---
name: oauth-login
inputs:
  - path: ./docs/oauth-login.md
    kind: documentation
---

Add OAuth-based login across the shop.

The backend needs Google and GitHub provider integrations, token
storage, and a refresh endpoint. The mobile app needs login and
registration screens, OAuth redirect handling, and token refresh
on app resume.

Both sides depend on a shared HTTP contract for the auth endpoints,
so the contract change must land before either implementation.
```

Then author `./docs/oauth-login.md` with the prose feature description — one paragraph per requirement is plenty. The discovery brief reads it as `kind: documentation` and folds the capabilities into the inventory.

> **Change shape.** This walkthrough is the **new-feature** shape (sources are documentation only). The other two shapes — `migrate-legacy` (`--source <key>=<git-url>`) and `update-existing` (no flags) — flow through the same Steps 4–9 with different inputs. See the change-shapes preview at the bottom of [Working across repos: executing](cross-repo-execute.md#change-shapes-preview).

## 4. Plan the change

Run the planning skill:

```text
/change:plan oauth-login from ./docs/oauth-login.md
```

`/change:plan` runs the four-phase planning pipeline (the briefs live alongside the skill at [`plugins/change/skills/plan/briefs/<capability>/`](../../plugins/change/skills/plan/briefs/)):

| Phase | What happens | On-disk artefact |
|---|---|---|
| **Discovery** | Reads `change.md` and `./docs/oauth-login.md`; emits a neutral capability inventory. | `.specify/plans/oauth-login/discovery.md` |
| **Sync workspace** *(multi-repo only)* | Runs `specify workspace sync` to materialise every registry project; inventories each project slot. | `.specify/workspace/<project>/`, `.specify/plans/oauth-login/workspace.md` |
| **Propose** | Decomposes the inventory into change slices via the accept / edit / reject loop; appends each accepted slice via `specify plan add`. | `plan.yaml` (entries without `project`), `.specify/plans/oauth-login/proposal.md` |
| **Assignment** *(multi-repo only)* | Infers `project` per entry from registry descriptions, baseline specs, and schema; writes via `specify plan amend --project`. | `plan.yaml` (entries gain `project:`) |

When the skill detects an API boundary between the two projects, it inserts a **contract change** before the implementation changes and populates `contracts.produces` / `contracts.consumes` on the relevant registry entries. The contract change carries `capability: contracts@v1` and no `project` — it runs against the hub itself.

`specify plan validate` is the final gate; the skill exits non-zero on any error-level finding.

<details>
<summary>Expected assignment table (interactive review)</summary>

```text
## Assignment

| # | Entry                | Project       | Rationale                                                   |
|---|----------------------|---------------|-------------------------------------------------------------|
| 1 | add-oauth-tokens     | shop-backend  | description overlap: OAuth providers, token storage         |
| 2 | add-oauth-screens    | shop-mobile   | description overlap: login screens, OAuth redirect handling |
```

The contract change is omitted from this table — only entries with a real `project` are routed.

</details>

<details>
<summary>Expected <code>specify plan status</code> output after planning</summary>

```text
oauth-login
  pending  oauth-login-contract                                   (depends-on: [])
  pending  add-oauth-tokens     project: shop-backend             (depends-on: [oauth-login-contract])
  pending  add-oauth-screens    project: shop-mobile              (depends-on: [oauth-login-contract])

Summary: 3 pending, 0 in-progress, 0 done
```

</details>

The plan is now the single source of truth for what runs where. Run `cat plan.yaml` to see it in full — including the auto-populated `context:` lists that focus each implementation change on the contract paths it depends on.

## What you learned

- The platform-hub topology (`specify init --hub`) is the canonical starting shape for multi-repo changes. The hub holds platform state and never carries code.
- `specify registry add` registers code projects with kebab-case names, capability identifiers, and domain descriptions. Descriptions drive automated assignment in `/change:plan`.
- `specify change create` scaffolds the operator brief; the `inputs:` frontmatter feeds the discovery brief.
- `/change:plan` runs discovery -> sync-workspace -> propose -> assignment, and finishes with `specify plan validate` as the gate. When it detects a cross-project API boundary it inserts a contract change before the implementation changes.

## Next

[Working across repos: executing](cross-repo-execute.md) -- inspect the workspace, run `/change:execute loop` across both projects, and publish PRs with `specify workspace push`.
