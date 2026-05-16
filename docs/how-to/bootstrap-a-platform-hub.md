# Bootstrap a Platform Hub

A **platform hub** is a registry-only Specify repo that holds platform state -- `registry.yaml`, `change.md`, `plan.yaml`, `workspace/` -- but is never itself a code project. It is the canonical starting shape for any cross-repo Specify change.

This how-to gets you from an empty directory to a registered hub with two code projects in under five minutes. For the full conceptual treatment see [Platform repo topologies](../explanation/platform-repo.md).

## Prerequisites

- The `specify` CLI (>= 0.24.2) installed and on `PATH`.
- A namespace on a git forge -- the example uses GitHub via `gh`.
- The remote URLs for the code projects you intend to register. Greenfield is fine: `specify workspace sync` will bootstrap them later.

## 1. Create the hub directory

```bash
mkdir shop-platform && cd shop-platform
git init --quiet
git remote add origin git@github.com:org/shop-platform.git
```

The hub itself is a git repo so the platform state (registry, change briefs, archived plans) is version-controlled alongside any other operator-owned material.

## 2. Run `specify init --hub`

```bash
specify init --hub --name shop-platform
```

In hub mode, **no positional** capability argument is passed -- `--hub` is the discriminator. Combining a capability positional with `--hub` is rejected with `init-requires-capability-or-hub`. `--name` must be kebab-case because later change commands use it when scaffolding operator-facing artifacts.

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

The hub now contains:

```text
shop-platform/
├── AGENTS.md         # generated hub context
├── registry.yaml     # version: 1, projects: []
├── .gitignore        # upserts .specify/.cache/ and .specify/workspace/
└── .specify/
    ├── project.yaml  # hub: true (capability: omitted)
    └── context.lock  # context freshness fingerprint
```

`specify init --hub` does not create `change.md` or `plan.yaml`; `specify change draft` (typically invoked through `/change:draft`) mints those operator artifacts together when a specific change begins. It refuses to run when `.specify/` already exists -- the guard prevents accidentally clobbering an existing single-project setup. Remove `.specify/` first if you genuinely want to convert.

## 3. Register code projects

Add at least two projects (descriptions are mandatory once the registry has more than one entry):

```bash
specify registry add shop-backend \
    --url git@github.com:org/shop-backend.git \
    --capability omnia@v1 \
    --description "User registration, account management, and the authoritative implementation of the shop's HTTP API."

specify registry add shop-mobile \
    --url git@github.com:org/shop-mobile.git \
    --capability vectis@v1 \
    --description "iOS and Android mobile clients. Owns login screens, the cart, checkout, and OAuth redirect handling."
```

For greenfield projects whose remote does not yet exist, register them anyway -- `specify workspace push` will run `gh repo create` later.

## 4. Sanity-check

```bash
specify registry validate
specify registry show
```

`validate` confirms three invariants the hub topology relies on:

- Every entry has a kebab-case `name`, a well-formed `url`, and a non-empty `capability` capability value.
- Every entry has a `description` (the `description-missing-multi-repo` invariant fires above one project).
- No entry has `url: .` (the `hub-cannot-be-project` invariant fires when `project.yaml: hub: true`).

`show` renders the registry as parsed YAML so you can spot-check the descriptions before they feed into `/change:draft`'s assignment step.

## 5. Commit the platform state

```bash
git add AGENTS.md .specify/ registry.yaml .gitignore
git commit -m "Bootstrap shop-platform hub with two registered projects"
```

The hub is now ready to drive a change. The recommended next step is the cross-repo tutorial.

## Verification

| Check | Command | Expect |
|-------|---------|--------|
| Hub markers in place | `cat .specify/project.yaml` | A line containing `hub: true` and **no** `capability:` line. |
| Phase pipelines disabled | `ls .specify/` | `project.yaml` and `context.lock` only. **No** `slices/`, `specs/`, or `.cache/`. |
| Context generated | `test -f AGENTS.md && specify context check` | Exit 0. |
| Registry validates | `specify registry validate` | Exit 0, no diagnostics. |
| Both projects listed | `specify registry show` | `version: 1` and two `projects[]` entries with descriptions. |

## See also

- [Cross-Repo Changes](../tutorials/cross-repo-change.md) -- end-to-end tutorial driving the first change through this hub.
- [Manage registry projects](manage-registry-projects.md) -- add and remove projects after the first bootstrap.
- [Platform repo topologies](../explanation/platform-repo.md) -- when to choose hub vs platform-as-project.
- [`specify init`](../reference/cli/init.md) -- CLI reference for the `--hub` flag.
- [`specify registry`](../reference/cli/registry.md) -- CLI reference for `add` / `remove` / `show` / `validate`.
