# Cross-Repo Changes

Drive a feature spanning a backend and a mobile app from a single platform hub. This tutorial walks the **bootstrap-to-PRs** half of an end-to-end platform-first loop: bootstrap a hub, register two code projects, plan a feature that crosses both, execute the plan across workspace clones, and ship the result as PRs.

It exercises Steps 1-7 of the RFC-9 §1C critical path:

1. `specify init --hub` (RFC-9 §1D)
2. `specify registry add` (RFC-9 §2A)
3. `specify change create` (RFC-9 §1F)
4. `/change:plan` with multi-repo sync-peers and assignment
5. (Inspect the workspace)
6. `/change:execute loop` with CWD routing across two workspace clones
7. `specify workspace push` to publish branches and PRs

The remaining steps -- merging the PRs and archiving the plan -- live in the follow-on tutorial [Landing a Change](landing-a-change.md). Between them, the two tutorials walk the full Steps 1-9 RFC-9 §1C path. The `/change:plan <name> orchestrate` umbrella variants and the three change shapes (migrate-legacy / new-feature / update-existing) also live there.

> **Choosing your topology.** This tutorial uses the platform-hub topology because the feature spans two registered projects -- the hub holds platform state and the code lives in registered repos. If your work is single-repo, the platform-as-project shape (initiating repo with `url: .` in the registry) is simpler; see [Platform repo topologies](../explanation/platform-repo.md) for the comparison and [A Multi-Slice Change](single-repo-change.md) for the single-repo flow.

Every command below should run cleanly against the current `specify` CLI on a freshly-cloned hub. If a step fails, the gap is in the implementation, not the design — file an issue with the failing transcript.

**Prerequisites:**

- [`specify` CLI](../orientation/prerequisites.md) installed and on `PATH`.
- [`gh`](https://cli.github.com/) installed and authenticated against your GitHub org (`gh auth status`).
- A GitHub namespace you can create repos in. The walkthrough uses `org/` — substitute your real org or user.
- Two empty GitHub repos pre-created at `git@github.com:org/shop-backend.git` and `git@github.com:org/shop-mobile.git`. (Or skip pre-creation and let `specify workspace push` greenfield-bootstrap them in Step 7.)
- Familiarity with the [single-repo change tutorial](single-repo-change.md) — `/change:plan`, `/change:execute`, and the plan lifecycle.

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
    schema: contracts@v1
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

This tutorial uses the [platform-hub topology](../explanation/platform-repo.md) codified in RFC-9 §1D. The hub holds platform state and never appears in its own registry. Code lives in registered project repos that materialise under `.specify/workspace/<name>/` (the [tier-2 workspace](../explanation/workspace-tiers.md#the-two-tiers)).

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

`specify init --hub` does not create `change.md` or `plan.yaml`; those are minted later by `specify change create` and `specify change plan create`. It refuses to run when `.specify/` already exists. To convert an existing single-repo project into a hub, remove `.specify/` first.

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
    schema: omnia@v1
    description: >
      User registration, account management, and the authoritative
      implementation of the shop's HTTP API. Owns persistence, OAuth
      provider integration, token storage, and order processing.
  - name: shop-mobile
    url: git@github.com:org/shop-mobile.git
    schema: vectis@v1
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

> **Change shape.** This walkthrough is the **new-feature** shape (sources are documentation only). The other two shapes — `migrate-legacy` (`--source <key>=<git-url>`) and `update-existing` (no flags) — flow through the same Steps 4–9 with different inputs. See [Change shapes](#change-shapes) at the bottom of this page.

## 4. Plan the change

Run the planning skill:

```text
/change:plan oauth-login from ./docs/oauth-login.md
```

`/change:plan` runs the four-phase planning pipeline (the briefs live alongside the skill at [`plugins/change/skills/plan/briefs/<capability>/`](../../plugins/change/skills/plan/briefs/) — RFC-13 §3.11 moved them out of the capability manifest):

| Phase | What happens | On-disk artefact |
|---|---|---|
| **Discovery** | Reads `change.md` and `./docs/oauth-login.md`; emits a neutral capability inventory. | `.specify/plans/oauth-login/discovery.md` |
| **Sync peers** *(multi-repo only)* | Runs `specify workspace sync` to materialise every registry project; inventories each peer slot. | `.specify/workspace/<peer>/`, `.specify/plans/oauth-login/workspace.md` |
| **Propose** | Decomposes the inventory into change slices via the accept / edit / reject loop; appends each accepted slice via `specify change plan add`. | `plan.yaml` (entries without `project`), `.specify/plans/oauth-login/proposal.md` |
| **Assignment** *(multi-repo only)* | Infers `project` per entry from registry descriptions, baseline specs, and schema; writes via `specify change plan amend --project`. | `plan.yaml` (entries gain `project:`) |

When the skill detects an API boundary between the two projects, it inserts a **contract change** before the implementation changes and populates `contracts.produces` / `contracts.consumes` on the relevant registry entries. The contract change carries `schema: contracts@v1` and no `project` — it runs against the hub itself.

`specify change plan validate` is the final gate; the skill exits non-zero on any error-level finding.

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
<summary>Expected <code>specify change plan status</code> output after planning</summary>

```text
oauth-login
  pending  oauth-login-contract                                   (depends-on: [])
  pending  add-oauth-tokens     project: shop-backend             (depends-on: [oauth-login-contract])
  pending  add-oauth-screens    project: shop-mobile              (depends-on: [oauth-login-contract])

Summary: 3 pending, 0 in-progress, 0 done
```

</details>

The plan is now the single source of truth for what runs where. Run `cat plan.yaml` to see it in full — including the auto-populated `context:` lists that focus each implementation change on the contract paths it depends on.

## 5. Inspect the workspace

`/change:plan` already ran `specify workspace sync` during the sync-peers phase. Verify the resulting clones:

```bash
specify workspace status
```

<details>
<summary>Expected output</summary>

```text
shop-backend     git-clone     <40-char sha>     dirty: no     specify-tree: project.yaml
shop-mobile      git-clone     <40-char sha>     dirty: no     specify-tree: project.yaml
```

</details>

`specify workspace sync` is idempotent — re-run it between changes to refresh clones. Greenfield projects (remote does not yet exist) are bootstrapped in place via `git init` + `specify init`.

> **Tier-2 only.** `.specify/workspace/<peer>/` clones are durable; they outlive any single change. The legacy-source clones under `.specify/plans/<change>/analyze/<key>/` (tier-1) are a separate concern — read-only and ephemeral. See [Workspace tiers](../explanation/workspace-tiers.md) for the full contrast.

## 6. Execute the plan

Drive every change in dependency order:

```text
/change:execute loop
```

The driver:

1. Acquires the plan lock at `.specify/plan.lock` (one driver at a time).
2. Picks the next eligible slice via `specify change plan next --format json`.
3. For multi-repo entries, resolves the `project` field against `registry.yaml`, materialises only the selected workspace slot if it is missing, and prepares `specify/oauth-login` before any phase writes. The contract slice has no `project` and runs against the hub itself.
4. Runs `/spec:define` -> `/spec:build` -> `/spec:merge` for the slice.
5. After a routed merge succeeds, verifies the `/spec:merge` baseline commit boundary (`.specify/specs/` plus `.specify/archive/`) and commits non-baseline residue as `specify: residue <slice-name>`.
6. Restores CWD to the hub root and transitions the plan entry to `done`/`failed`/`blocked`.
7. Repeats from step 2 until `specify change plan next` reports `all-done` or `stuck`.

After producer contracts change, run `specify compatibility report --change oauth-login` when you want a classified consumer-impact report against workspace views.

<details>
<summary>Expected loop transcript (abbreviated)</summary>

```text
## /change:execute — oauth-login

### Change: oauth-login
Progress: done 0, in-progress 0, pending 3, blocked 0, failed 0, skipped 0 (total 3)

---

Self-heal: no in-progress entries found.

# specify change plan next --format json → { "next": "oauth-login-contract", "project": null, "description": "...", "sources": null }
# specify change plan transition oauth-login-contract in-progress

### Processing: oauth-login-contract (greenfield)

Step 1/3: define
  Artifacts: proposal.md, contracts/, design.md, tasks.md ✓
Step 2/3: build
  Tasks: 2/2 complete ✓
Step 3/3: merge
  Baseline updated: contracts/http/oauth-login.yaml ✓
  Status: done

---

# specify change plan next --format json → { "next": "add-oauth-tokens", "project": "shop-backend", ... }
# registry selector: shop-backend → git@github.com:org/shop-backend.git
# specify workspace status shop-backend --format json → git-clone, branch=main, dirty=false
# specify workspace prepare-branch shop-backend --change oauth-login --format json
#   → prepared=true branch=specify/oauth-login local-branch=created remote-branch=absent
# CWD saved: /…/shop-platform
# specify change plan transition add-oauth-tokens in-progress

Routing: add-oauth-tokens → shop-backend (.specify/workspace/shop-backend/)
Workspace: shop-backend prepared on specify/oauth-login

### Processing: add-oauth-tokens (greenfield)

Step 1/3: define ✓
Step 2/3: build
  Tasks: 5/5 complete ✓
Step 3/3: merge
  specify: merge add-oauth-tokens
  Baseline committed: git add .specify/specs/ .specify/archive/ \
      && git commit -m "specify: merge add-oauth-tokens"
  Baseline updated: .specify/specs/oauth-tokens/spec.md ✓
  Residue committed: specify: residue add-oauth-tokens

# CWD restored: /…/shop-platform
# specify change plan transition add-oauth-tokens done
  Status: done

---

# specify change plan next --format json → { "next": "add-oauth-screens", "project": "shop-mobile", ... }

Routing: add-oauth-screens → shop-mobile (.specify/workspace/shop-mobile/)

### Processing: add-oauth-screens (greenfield)

Step 1/3: define ✓
Step 2/3: build ✓
Step 3/3: merge ✓
  Status: done

---

## /change:execute — oauth-login — terminated

### Final state
Progress: done 3, in-progress 0, pending 0, blocked 0, failed 0, skipped 0 (total 3)

Completion: all-done

Next action: Change complete. Run specify workspace push to publish prepared specify/oauth-login branches and create or update PRs. Merge those PRs through the forge UI or gh pr merge, then close out via specify change finalize.
```

</details>

Each implementation slice leaves two local commits in its workspace clone: `/spec:merge` commits only `.specify/specs/` and `.specify/archive/` as `specify: merge <slice-name>`, then `/change:execute` commits project-output residue as `specify: residue <slice-name>`. This is what `specify workspace push` ships in Step 7.

> **Failure handling.** If a change fails mid-loop, `/change:execute` invokes `/spec:drop`, transitions the entry to `failed` (verbatim `outcome.summary` as `reason`), and continues. Subsequent changes that depend on the failed one stay `pending` until you `specify change plan transition <pred> pending` to retry, or `specify change plan transition <entry> skipped reason …` to drop the dependency leaf. See `/change:execute`'s [§Output format → Failure transcript](../../plugins/change/skills/execute/SKILL.md) for the recovery prompt.

## 7. Push branches and PRs

After execution, each workspace clone is already on `specify/oauth-login` with local commits ahead of the remote branch. Publish them:

```bash
specify workspace push
```

Per project, the verb:

1. Verifies the clone is clean and already checked out to `specify/oauth-login`; any other checkout is reported as `no-branch`.
2. Runs `git push --force-with-lease -u origin specify/oauth-login`.
3. For greenfield remotes, creates the repo via `gh repo create`.
4. Creates or updates a PR for the branch via `gh pr create` when needed.

`workspace push` is transport-only: it does not create the change branch on the fly, does not create commits, does not push default branches, and never merges PRs.

<details>
<summary>Expected output</summary>

```text
specify: workspace push — oauth-login

  shop-backend   pushed       specify/oauth-login   PR #41
  shop-mobile    pushed       specify/oauth-login   PR #18

2 pushed, 0 created, 0 up-to-date. 0 failed.
```

</details>

<details>
<summary>JSON output (<code>--format json</code>)</summary>

```json
{
  "projects": [
    { "name": "shop-backend", "status": "pushed", "branch": "specify/oauth-login", "pr": 41 },
    { "name": "shop-mobile",  "status": "pushed", "branch": "specify/oauth-login", "pr": 18 }
  ]
}
```

</details>

For greenfield projects (remote did not exist before this run), the per-project status flips to `created` and `gh repo create` runs first. Use `--dry-run` to classify each clone's push status without performing any writes — the verb adds `would-` prefixes to the action statuses. See [`specify workspace push`](../reference/cli/workspace.md#specify-workspace-push) for the full status vocabulary.

## Pause point

Two PRs are now open against `org/shop-backend` and `org/shop-mobile`, both on the `specify/oauth-login` branch. The `oauth-login` plan still lives at `plan.yaml` with every entry `done`. The hub is in the canonical "ready to land" state.

[**Continue to Landing a Change**](landing-a-change.md) for Steps 8 (operator PR merge through the forge UI or `gh pr merge`) and 9 (archive with `specify change finalize`), the `/change:plan <name> orchestrate` umbrella variants, and the three change shapes (migrate-legacy / new-feature / update-existing).

If you stop here, the platform-first work is shipped but unmerged. The PRs sit on the forge until reviewed; nothing is blocking. You can resume landing at any time -- the umbrella is idempotent on re-entry, and the manual flow is to merge the PRs through the forge UI or `gh pr merge`, then run `specify change finalize`.

## Troubleshooting

If `/change:execute loop` exits with `Completion: stuck` or any single invocation reports `reason: stuck`, the first triage step is `specify change plan doctor`:

```bash
specify change plan doctor
```

`doctor` is a strict superset of `specify change plan validate` — it runs every check `validate` runs, then layers four health diagnostics on top:

| Code | Severity | Recovery |
|------|----------|----------|
| `cycle-in-depends-on` | error | Break the cycle: `specify change plan amend <name> --depends-on …`. |
| `orphan-source-key` | warning | Reference the key from an entry's `sources:` (`specify change plan amend <name> --sources …`) or remove it from the top-level map. |
| `stale-workspace-clone` | warning | Refresh: `specify workspace sync`. |
| `unreachable-entry` | error | `specify change plan transition <pred> pending` after fixing the predecessor, or `specify change plan transition <entry> skipped --reason "…"` to drop the leaf. |

See [`specify change plan doctor`](../reference/cli/plan.md#specify-plan-doctor) for the full diagnostic table and JSON shape.

Other common issues:

- **`Error::DriverBusy { pid }`** — another `/change:execute` is holding `.specify/plan.lock`. If it is dead, `specify change plan lock release --pid <pid>` reclaims the stamp; otherwise wait for the live driver.
- **`hub-cannot-be-project`** — a registry entry has `url: .` on a hub. Either remove the entry (`specify registry remove <name>`) or convert the hub to a platform-as-project shape by removing `.specify/` and re-running `specify init <capability>` without `--hub`.
- **Breaking compatibility findings** — run `specify compatibility report --change <name>` to inspect producer-to-consumer contract deltas, then see [Resolve Cross-Project Compatibility Findings](../how-to/resolve-cross-project-contract-warnings.md).

## Verification

A reviewer (or an operator stepping through this tutorial as an integration test) can grep these expected outputs at each step:

| After | Command | Expect |
|---|---|---|
| Step 1 | `cat .specify/project.yaml` | A line containing `hub: true` and **no** `capability:` line. |
| Step 1 | `ls .specify/` | `project.yaml`, `context.lock`. **No** `slices/`, `specs/`, or `.cache/` (phase pipelines disabled). |
| Step 1 | `test -f AGENTS.md && specify context check` | Exit 0. |
| Step 2 | `specify registry validate` | Exit 0; no diagnostics. |
| Step 2 | `specify registry show` | `version: 1` and two `projects[]` entries with descriptions. |
| Step 3 | `head -10 change.md` | Frontmatter `name: oauth-login` and the documentation `inputs:` entry. |
| Step 4 | `specify change plan validate` | Exit 0; no error-level findings. |
| Step 4 | `specify change plan status` | Three entries; the two implementation entries carry `project: shop-backend` / `project: shop-mobile`. |
| Step 5 | `specify workspace status` | Both projects show `git-clone` materialisation, `dirty: no`. |
| Step 6 | `specify change plan status` | All three changes `done`; `Summary: 0 pending, 0 in-progress, 3 done`. |
| Step 7 | `gh pr list -R org/shop-backend --head specify/oauth-login` | Exactly one open PR. |
| Step 7 | `gh pr list -R org/shop-mobile --head specify/oauth-login` | Exactly one open PR. |

Any deviation is a blocker. File the failing transcript against this tutorial; per RFC-9 §1C the gap is in the implementation, not the design. The Steps 8-9 verification (PR `MERGED` on remote, plan archived, re-run `plan-not-found`) lives in [Landing a Change](landing-a-change.md#verification).

## Change shapes (preview)

The platform-first loop above is shape-agnostic. The same Steps 1-7 drive three change shapes -- `migrate-legacy`, `new-feature`, `update-existing` -- through a single uniform sequence. The walkthrough at the top of this page is the **new-feature** shape (sources are documentation only); the other two arrive in [Landing a Change](landing-a-change.md#change-shapes), which also covers the `/change:plan <name> orchestrate` umbrella that drives each shape as a single operator action.

## What you learned

- The platform-hub topology (`specify init --hub`) is the canonical starting shape for multi-repo changes. The hub holds platform state and never carries code.
- `specify registry add` registers code projects with kebab-case names, capability identifiers, and domain descriptions. Descriptions drive automated assignment in `/change:plan`.
- `specify change create` scaffolds the operator brief; the `inputs:` frontmatter feeds the discovery brief.
- `/change:plan` runs discovery -> sync-peers -> propose -> assignment, and finishes with `specify change plan validate` as the gate. When it detects a cross-project API boundary it inserts a contract change before the implementation changes.
- `/change:execute loop` `chdir`s into each workspace clone, runs define-build-merge, transitions the plan entry, and routes back. Multi-repo CWD routing is invisible to the phase skills.
- `specify workspace push` ships prepared `specify/<change-name>` branches as PRs without creating branches, committing residue, pushing default branches, or merging PRs.

## Cross-links

- [Platform repo topologies](../explanation/platform-repo.md) -- registry-only hub vs platform-as-project, the validation invariant, and the on-disk shape of each.
- [Workspace tiers](../explanation/workspace-tiers.md) -- the legacy-source vs registered-project clone distinction the loop relies on.
- [The Layered Stack](../explanation/three-layer-stack.md) -- where `/change:plan` (default + `orchestrate` modes) and `/change:execute` sit in the layered model.
- [`/change:plan`](../reference/change-skills/plan.md) -- Layer 3 plan authoring skill.
- [`/change:execute`](../reference/change-skills/execute.md) -- Layer 2 plan driver.
- [`specify compatibility`](../reference/cli/compatibility.md) -- RM-04 consumer-impact contract report.
- [`specify init`](../reference/cli/init.md) -- the `--hub` flag.
- [`specify registry`](../reference/cli/registry.md) -- `add` / `remove` / `show` / `validate`.
- [`specify workspace`](../reference/cli/workspace.md) -- `sync` / `status` / `push`.
- [`specify change plan`](../reference/cli/plan.md) -- `create` / `add` / `amend` / `next` / `doctor` / `archive` / `lock`.

## Next

[Landing a Change](landing-a-change.md) -- merge the PRs you just pushed, finalize the change, and exercise the `/change:plan <name> orchestrate` umbrella shapes.
