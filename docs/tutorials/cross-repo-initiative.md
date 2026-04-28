# Cross-Repo Initiatives

Drive a feature spanning a backend and a mobile app from a single platform hub. This tutorial walks an end-to-end **platform-first** loop: bootstrap a hub, register two code projects, plan a feature that crosses both, execute the plan across workspace clones, and ship the result as PRs.

It exercises the full RFC-9 §1C critical path:

1. `specify init --hub` (RFC-9 §1D)
2. `specify registry add` (RFC-9 §2A)
3. `specify initiative create` (RFC-9 §1F)
4. `/spec:plan` with multi-repo sync-peers and assignment
5. `/spec:execute --loop` with CWD routing across two workspace clones
6. `specify workspace push` to publish branches and PRs
7. `specify workspace merge` to land PRs once CI is green (RFC-9 §4A)
8. `specify initiative finalize` to confirm landing and archive (RFC-9 §4C)

Every command below should run cleanly against the current `specify` CLI on a freshly-cloned hub. If a step fails, the gap is in the implementation, not the design — file an issue with the failing transcript.

**Prerequisites:**

- [`specify` CLI](../orientation/prerequisites.md) installed and on `PATH`.
- [`gh`](https://cli.github.com/) installed and authenticated against your GitHub org (`gh auth status`).
- A GitHub namespace you can create repos in. The walkthrough uses `org/` — substitute your real org or user.
- Two empty GitHub repos pre-created at `git@github.com:org/shop-backend.git` and `git@github.com:org/shop-mobile.git`. (Or skip pre-creation and let `specify workspace push` greenfield-bootstrap them in Step 7.)
- Familiarity with the [single-repo initiative tutorial](single-repo-initiative.md) — `/spec:plan`, `/spec:execute`, and the plan lifecycle.

## What you will build

A platform hub `shop-platform/` that drives the `oauth-login` initiative across two registered projects:

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
└── .specify/
    ├── project.yaml                        # { schema: hub, hub: true, name: shop-platform }
    ├── registry.yaml                       # version: 1, projects: [shop-backend, shop-mobile]
    ├── initiative.md                       # operator brief for `oauth-login`
    ├── plan.yaml                           # the plan authored by /spec:plan
    ├── plans/oauth-login/                  # discovery, workspace, proposal markdown
    ├── archive/                            # finalised initiatives (after Step 9)
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

specify init hub --schema-dir . --name shop-platform --hub
```

The first positional `hub` is a placeholder for the schema argument — `--hub` mode ignores it but the parser still requires *something*. `--name` must be kebab-case because the CLI bakes it into `.specify/initiative.md`'s frontmatter.

<details>
<summary>Expected output</summary>

```text
Initialized .specify/ as a registry-only platform hub
  schema: hub
  config: /…/shop-platform/.specify/project.yaml
  cache present: false
  directories created: /…/shop-platform/.specify
  specify_version: 0.x.y
```

</details>

The hub now has:

```text
.specify/
├── project.yaml      # schema: hub, hub: true
├── registry.yaml     # version: 1, projects: []
├── initiative.md     # canonical template, name: shop-platform
└── .gitignore        # upserts .specify/.cache/ and .specify/workspace/
```

`specify init --hub` refuses to run when `.specify/` already exists. To convert an existing single-repo project into a hub, remove `.specify/` first.

> **Why hub mode?** A hub gets `schema: hub` (the sentinel that disables phase pipelines on the hub itself) and `hub: true` (the validation flag that rejects any registry entry whose `url` is `.`). Together these pin the platform repo's identity unambiguously. See [Platform repo topologies](../explanation/platform-repo.md) for the full contract.

## 2. Register the two projects

Add the backend (Omnia schema):

```bash
specify registry add shop-backend \
    --url git@github.com:org/shop-backend.git \
    --schema omnia@v1 \
    --description "User registration, account management, and the authoritative implementation of the shop's HTTP API. Owns persistence, OAuth provider integration, token storage, and order processing."
```

Add the mobile app (Vectis schema):

```bash
specify registry add shop-mobile \
    --url git@github.com:org/shop-mobile.git \
    --schema vectis@v1 \
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

The descriptions matter beyond validation: the assignment step in `/spec:plan` (Step 4) infers project routing from registry descriptions. Rich, domain-specific descriptions land clean assignments; sparse descriptions force unresolved (`?`) prompts during planning.

## 3. Author the initiative brief

Scaffold the brief:

```bash
specify initiative create oauth-login
```

This rewrites `.specify/initiative.md` with a fresh template named after the initiative. Edit it to describe the feature and point the discovery brief at any supplementary documentation:

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

> **Initiative shape.** This walkthrough is the **new-feature** shape (sources are documentation only). The other two shapes — `migrate-legacy` (`--source <key>=<git-url>`) and `update-existing` (no flags) — flow through the same Steps 4–9 with different inputs. See [Initiative shapes](#initiative-shapes) at the bottom of this page.

## 4. Plan the initiative

Run the planning skill:

```text
/spec:plan oauth-login --from ./docs/oauth-login.md
```

`/spec:plan` runs the four-phase `pipeline.plan`:

| Phase | What happens | On-disk artefact |
|---|---|---|
| **Discovery** | Reads `initiative.md` and `./docs/oauth-login.md`; emits a neutral capability inventory. | `.specify/plans/oauth-login/discovery.md` |
| **Sync peers** *(multi-repo only)* | Runs `specify workspace sync` to materialise every registry project; inventories each peer slot. | `.specify/workspace/<peer>/`, `.specify/plans/oauth-login/workspace.md` |
| **Propose** | Decomposes the inventory into change slices via the accept / edit / reject loop; appends each accepted slice via `specify plan add`. | `.specify/plan.yaml` (entries without `project`), `.specify/plans/oauth-login/proposal.md` |
| **Assignment** *(multi-repo only)* | Infers `project` per entry from registry descriptions, baseline specs, and schema; writes via `specify plan amend --project`. | `.specify/plan.yaml` (entries gain `project:`) |

When the skill detects an API boundary between the two projects, it inserts a **contract change** before the implementation changes and populates `contracts.produces` / `contracts.consumes` on the relevant registry entries. The contract change carries `schema: contracts@v1` and no `project` — it runs against the hub itself.

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

The plan is now the single source of truth for what runs where. Run `cat .specify/plan.yaml` to see it in full — including the auto-populated `context:` lists that focus each implementation change on the contract paths it depends on.

## 5. Inspect the workspace

`/spec:plan` already ran `specify workspace sync` during the sync-peers phase. Verify the resulting clones:

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

`specify workspace sync` is idempotent — re-run it between initiatives to refresh clones. Greenfield projects (remote does not yet exist) are bootstrapped in place via `git init` + `specify init`.

> **Tier-2 only.** `.specify/workspace/<peer>/` clones are durable; they outlive any single initiative. The legacy-source clones under `.specify/plans/<initiative>/analyze/<key>/` (tier-1) are a separate concern — read-only and ephemeral. See [Workspace tiers](../explanation/workspace-tiers.md) for the full contrast.

## 6. Execute the plan

Drive every change in dependency order:

```text
/spec:execute --loop
```

The driver:

1. Acquires the plan lock at `.specify/plan.lock` (one driver at a time).
2. Picks the next eligible change via `specify plan next --format json`.
3. For multi-repo entries, resolves the `project` field against `registry.yaml` and `chdir`s into `.specify/workspace/<project>/`. The contract change has no `project` and runs against the hub itself.
4. Runs `/spec:define` → `/spec:build` → `/spec:merge` for the change.
5. Reads the phase outcome (`success`/`failure`/`deferred`) and transitions the plan entry to `done`/`failed`/`blocked`.
6. Restores CWD to the hub root.
7. After a successful merge, runs the [cross-project contract check](../../plugins/spec/skills/execute/SKILL.md#cross-project-contract-check-rfc-9-3b) (RFC-9 §3B): walks the producer's `contracts.produces` list, finds consumer projects via `contracts.consumes`, and runs `/contracts:validator --mode cross-project` against each consumer's workspace clone. Findings are recorded as `cross-project-warning:` entries on the merged change's `journal.yaml` and rendered in the merge transcript. **Warnings never halt the loop.**
8. Repeats from step 2 until `specify plan next` reports `all-done` or `stuck`.

<details>
<summary>Expected loop transcript (abbreviated)</summary>

```text
## /spec:execute — oauth-login

### Initiative: oauth-login
Progress: done 0, in-progress 0, pending 3, blocked 0, failed 0, skipped 0 (total 3)

---

Self-heal: no in-progress entries found.

# specify plan next --format json → { "next": "oauth-login-contract", "project": null, "description": "...", "sources": null }
# specify plan transition oauth-login-contract in-progress

### Processing: oauth-login-contract (greenfield)

Step 1/3: define
  Artifacts: proposal.md, contracts/, design.md, tasks.md ✓
Step 2/3: build
  Tasks: 2/2 complete ✓
Step 3/3: merge
  Baseline updated: .specify/contracts/http/oauth-login.yaml ✓
  Status: done

---

# specify plan next --format json → { "next": "add-oauth-tokens", "project": "shop-backend", ... }
# specify plan transition add-oauth-tokens in-progress
# specify workspace status shop-backend → materialised
# CWD saved: /…/shop-platform

Routing: add-oauth-tokens → shop-backend (.specify/workspace/shop-backend/)

### Processing: add-oauth-tokens (greenfield)

Step 1/3: define ✓
Step 2/3: build
  Tasks: 5/5 complete ✓
Step 3/3: merge
  specify: merge add-oauth-tokens
  Auto-commit: git add .specify/specs/ .specify/contracts/ .specify/archive/ \
      && git commit -m "specify: merge add-oauth-tokens"
  Baseline updated: .specify/specs/oauth-tokens/spec.md ✓

# CWD restored: /…/shop-platform
# specify plan transition add-oauth-tokens done
  Status: done

---

# specify plan next --format json → { "next": "add-oauth-screens", "project": "shop-mobile", ... }

Routing: add-oauth-screens → shop-mobile (.specify/workspace/shop-mobile/)

### Processing: add-oauth-screens (greenfield)

Step 1/3: define ✓
Step 2/3: build ✓
Step 3/3: merge ✓
  Status: done

---

## /spec:execute — oauth-login — terminated

### Final state
Progress: done 3, in-progress 0, pending 0, blocked 0, failed 0, skipped 0 (total 3)

Completion: all-done

Next action: Initiative complete — no further action needed.
```

</details>

Each implementation change auto-commits inside its workspace clone (`git add .specify/specs/ … && git commit -m "specify: merge <name>"`). This is what `specify workspace push` ships in Step 7.

> **Failure handling.** If a change fails mid-loop, `/spec:execute` invokes `/spec:drop`, transitions the entry to `failed` (verbatim `outcome.summary` as `--reason`), and continues. Subsequent changes that depend on the failed one stay `pending` until you `specify plan transition <pred> pending` to retry, or `specify plan transition <entry> skipped --reason …` to drop the dependency leaf. See `/spec:execute`'s [§Output format → Failure transcript](../../plugins/spec/skills/execute/SKILL.md) for the recovery prompt.

## 7. Push branches and PRs

After execution, each workspace clone has local commits ahead of `main`. Publish them:

```bash
specify workspace push
```

Per project, the verb:

1. Creates or updates the `specify/oauth-login` branch from the clone's HEAD.
2. Runs `git push --force-with-lease -u origin specify/oauth-login`.
3. For greenfield remotes, creates the repo via `gh repo create`.
4. Runs `gh pr create` if no PR exists for the branch.

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

## 8. Land the PRs (optional)

Once CI is green on each PR, squash-merge them in one shot:

```bash
specify workspace merge
```

Per project, the verb checks `gh pr checks` against the `specify/oauth-login` branch and, if every check is `pass` or `skipping`, runs `gh pr merge --squash`.

<details>
<summary>Expected output (all checks green)</summary>

```text
specify: workspace merge — oauth-login (specify/oauth-login)

  shop-backend     merged                    PR #41     https://github.com/org/shop-backend/pull/41
  shop-mobile      merged                    PR #18     https://github.com/org/shop-mobile/pull/18

2 merged, 0 would-merge, 0 pending-checks, 0 failed-checks, 0 closed, 0 no-branch, 0 branch-pattern-mismatch, 0 failed.
```

</details>

The verb refuses to operate on any PR whose branch is not `specify/oauth-login` exactly (the `branch-pattern-mismatch` guard). It never passes `--admin` or `--auto`, and it never overrides failing or pending checks. Failures on one project surface in their own row without aborting the others.

Use `--dry-run` to see the would-merge classification without invoking `gh pr merge`. See [`specify workspace merge`](../reference/cli/workspace.md#specify-workspace-merge) for the full status table and exit-code contract (any `pending-checks`, `failed-checks`, or `branch-pattern-mismatch` flips the exit code to `1` so CI loops can branch on it).

## 9. Finalize the initiative

Once every PR is merged, close the initiative with the canonical closure verb:

```bash
specify initiative finalize
```

`finalize` confirms the whole initiative is landed and atomically sweeps local plan state into the archive (RFC-9 §4C). It runs four guards in order before any move:

1. **Plan-presence:** `.specify/plan.yaml` exists.
2. **Plan terminal-state:** every entry is `done` / `failed` / `skipped`.
3. **Per-project PR-state:** every registered project's PR on `specify/oauth-login` is `MERGED` on its remote (or has no PR at all). Refuses on `unmerged` / `closed` / `branch-pattern-mismatch` / `failed`.
4. **Workspace-cleanliness:** `git status --porcelain` is empty for every workspace clone.

Any guard failure refuses with a per-project status table and leaves the on-disk state untouched. When all guards pass, `plan.yaml`, `.specify/initiative.md`, and `.specify/plans/oauth-login/` move atomically into `.specify/archive/plans/<YYYYMMDD>-oauth-login/`.

<details>
<summary>Expected output (all PRs merged, clean clones)</summary>

```text
specify: initiative finalize — oauth-login (specify/oauth-login)

  shop-backend         merged                   PR #41     https://github.com/org/shop-backend/pull/41
  shop-mobile          merged                   PR #18     https://github.com/org/shop-mobile/pull/18

2 merged, 0 unmerged, 0 closed, 0 no-branch, 0 branch-pattern-mismatch, 0 dirty, 0 failed.

Initiative `oauth-login` finalized.
  archived plan: /…/shop-platform/.specify/archive/plans/oauth-login-20260428.yaml
  archived dir:  /…/shop-platform/.specify/archive/plans/oauth-login-20260428
```

</details>

The two workspace clones stay on disk under `.specify/workspace/` — they are the staging area for the next initiative. To prune them at the same time:

```bash
specify initiative finalize --clean
```

`--clean` removes `.specify/workspace/<peer>/` for every non-symlink registered project after the archive completes. Refused when any clone has a dirty working tree; the diagnostic warns that `--clean` would drop the uncommitted changes.

Use `--dry-run` to preview the guard table without writing anything — useful for verifying readiness before you commit. `finalize` is **idempotent**: re-running it after manually clearing a refused guard (e.g. merging the last PR by hand) completes the archive on the second invocation. Re-running after a successful finalize returns `plan-not-found`, the explicit "already finalized" signal.

> **One-shot variant — `/spec:initiative` (RFC-9 §2C).** The Layer 4 umbrella skill composes Steps 1–9 into a single operator action: brief → registry validate → plan → execute → push → optional merge → finalize. The three subsections below show the umbrella driving each of the three initiative shapes against the same hub. See [`/spec:initiative` SKILL](../../plugins/spec/skills/initiative/SKILL.md) for the full algorithm, halt semantics, and re-entry rules.

## Troubleshooting

If `/spec:execute --loop` exits with `Completion: stuck` or any single invocation reports `reason: stuck`, the first triage step is `specify plan doctor`:

```bash
specify plan doctor
```

`doctor` is a strict superset of `specify plan validate` — it runs every check `validate` runs, then layers four health diagnostics on top:

| Code | Severity | Recovery |
|------|----------|----------|
| `cycle-in-depends-on` | error | Break the cycle: `specify plan amend <name> --depends-on …`. |
| `orphan-source-key` | warning | Reference the key from an entry's `sources:` (`specify plan amend <name> --sources …`) or remove it from the top-level map. |
| `stale-workspace-clone` | warning | Refresh: `specify workspace sync`. |
| `unreachable-entry` | error | `specify plan transition <pred> pending` after fixing the predecessor, or `specify plan transition <entry> skipped --reason "…"` to drop the leaf. |

See [`specify plan doctor`](../reference/cli/plan.md#specify-plan-doctor) for the full diagnostic table and JSON shape.

Other common issues:

- **`Error::DriverBusy { pid }`** — another `/spec:execute` is holding `.specify/plan.lock`. If it is dead, `specify plan lock release --pid <pid>` reclaims the stamp; otherwise wait for the live driver.
- **`hub-cannot-be-project`** — a registry entry has `url: .` on a hub. Either remove the entry (`specify registry remove <name>`) or convert the hub to a platform-as-project shape by removing `.specify/` and re-running `specify init <schema>` without `--hub`.
- **Cross-project contract warnings in the merge transcript** — see [`/spec:execute` §Cross-project contract check](../../plugins/spec/skills/execute/SKILL.md#cross-project-contract-check-rfc-9-3b). The merged change is still `done`; the warnings are advisory and recorded on the merged change's journal.

## Verification

A reviewer (or an operator stepping through this tutorial as an integration test) can grep these expected outputs at each step:

| After | Command | Expect |
|---|---|---|
| Step 1 | `cat .specify/project.yaml` | Lines containing `schema: hub` and `hub: true`. |
| Step 1 | `ls .specify/` | `project.yaml`, `registry.yaml`, `initiative.md`. **No** `changes/` or `specs/` (phase pipelines disabled). |
| Step 2 | `specify registry validate` | Exit 0; no diagnostics. |
| Step 2 | `specify registry show` | `version: 1` and two `projects[]` entries with descriptions. |
| Step 3 | `head -10 .specify/initiative.md` | Frontmatter `name: oauth-login` and the documentation `inputs:` entry. |
| Step 4 | `specify plan validate` | Exit 0; no error-level findings. |
| Step 4 | `specify plan status` | Three entries; the two implementation entries carry `project: shop-backend` / `project: shop-mobile`. |
| Step 5 | `specify workspace status` | Both projects show `git-clone` materialisation, `dirty: no`. |
| Step 6 | `specify plan status` | All three changes `done`; `Summary: 0 pending, 0 in-progress, 3 done`. |
| Step 7 | `gh pr list -R org/shop-backend --head specify/oauth-login` | Exactly one open PR. |
| Step 7 | `gh pr list -R org/shop-mobile --head specify/oauth-login` | Exactly one open PR. |
| Step 8 | `gh pr view <pr> -R org/shop-backend --json state,merged` | `{"state":"MERGED","merged":true}`. |
| Step 9 | `ls .specify/archive/plans/` | A `oauth-login-<YYYYMMDD>.yaml` plan file plus a `oauth-login-<YYYYMMDD>/` directory holding `initiative.md` and the `plans/oauth-login/` authoring trail. |
| Step 9 | `ls .specify/plan.yaml` | `No such file or directory` — the plan moved to the archive. |
| Step 9 | `specify initiative finalize` (re-run) | Exits `1` with `error: plan-not-found` — the canonical "already finalized" signal. |

Any deviation is a blocker. File the failing transcript against this tutorial; per RFC-9 §1C the gap is in the implementation, not the design.

## Initiative shapes

The platform-first loop above is shape-agnostic. The same Steps 1–9 drive three initiative shapes (RFC-9 §Motivation → *The three initiative shapes*); only the inputs to Step 4 (Plan) differ. Each shape is also drivable as a single command via the Layer 4 umbrella `/spec:initiative` (RFC-9 §2C). The transcripts below show each shape from the umbrella's perspective; the manual fallback for every step is the same Layer 1 verb the umbrella shells out to (see [§Drop down a layer](../how-to/drop-down-a-layer.md#from-layer-4-to-layer-3-skip-the-umbrella) for the exact verb sequence).

### Variant: migrate-legacy

Sources arrive via `--source <key>=<git-url-or-path>`. `/spec:analyze` clones each source into `.specify/plans/<initiative>/analyze/<key>/` (the [tier-1 workspace](../explanation/workspace-tiers.md#the-two-tiers)) for shallow capability inventory; deep `/spec:extract` runs at define time per change. Targets are existing or newly-minted registered projects.

Run against an empty hub:

```text
/spec:initiative create migrate-foo \
    --shape migrate-legacy \
    --source monolith=git@github.com:org/legacy-foo.git \
    --auto-merge
```

The umbrella runs all seven steps without halting:

1. **Brief.** `specify initiative create migrate-foo` scaffolds `.specify/initiative.md`; the operator confirms a default body listing the legacy monolith as a `legacy-code` input.
2. **Registry.** Empty + `--shape migrate-legacy` → hand off to the 2B greenfield path inside `/spec:plan`.
3. **Plan.** `/spec:plan` runs discovery against the cloned monolith, proposes a two-project topology (`foo-backend` + `foo-mobile`), shells `specify registry add` × 2 and `specify workspace sync` once, then propose decomposes into one cross-project contract change plus one implementation slice per project. Assignment routes the implementation slices.
4. **Execute.** `/spec:execute --loop` drives all three changes to `done` (contract change runs against the hub; the two implementation changes run inside their workspace clones).
5. **Push.** `specify workspace push` opens two PRs.
6. **Land.** `--auto-merge` → `specify workspace merge` waits for CI, sees both PRs green, squash-merges them.
7. **Finalize.** `specify initiative finalize` archives the plan and brief.

Verb sequence: `specify initiative create` → `specify registry validate` → `/spec:plan` → `specify plan create` → `specify registry add` × 2 → `specify workspace sync` → `specify plan add` × 3 → `specify plan amend --project` × 2 → `specify plan validate` → `/spec:execute --loop` → `specify workspace push` → `specify workspace merge` → `specify initiative finalize`. Full transcript and on-disk shapes: [`fixtures/migrate-legacy/`](../../plugins/spec/skills/initiative/fixtures/migrate-legacy/).

### Variant: new-feature

Sources arrive via `--from <docs>` only (or via `initiative.md:inputs`). Targets are existing registered projects, possibly with new ones spawned at assignment time via the registry-proposal sub-step (RFC-9 §2B).

Run against the populated hub from Steps 1–3 above (or your own equivalent):

```text
/spec:initiative create dark-mode \
    --shape new-feature \
    --from ./docs/dark-mode-spec.md
```

**The walkthrough at the top of this page is this shape.** The umbrella drives the same nine-step flow, with one wrinkle: without `--auto-merge`, Step 6 lists the open PRs and **stops**. The operator merges PRs by hand on the forge (or runs `specify workspace merge` directly), then re-runs the umbrella to land Step 7. Re-entry inspects on-disk state — brief present, plan terminal, every PR `MERGED` on remote — and skips straight to `specify initiative finalize`.

Verb sequence (run 1, halts at step 6): `specify initiative create` → `specify registry validate` → `/spec:plan --from ./docs/dark-mode-spec.md` → `specify plan create` → `specify workspace sync` → `specify plan add` × 3 → `specify plan amend --project` × 2 → `specify plan validate` → `/spec:execute --loop` → `specify workspace push` → `gh pr list` (read-only). No registry mutation — both projects exist before the run.

Verb sequence (run 2, after the operator merges PRs by hand): `specify registry validate` → `specify workspace push` (reports `up-to-date`) → `gh pr list` → `specify initiative finalize`.

Full transcript and on-disk shapes: [`fixtures/new-feature/`](../../plugins/spec/skills/initiative/fixtures/new-feature/).

### Variant: update-existing

No `--from` and no `--source` — sources are unused. Targets are existing registered projects; baseline accumulation in `.specify/workspace/<peer>/specs/` is the dominant signal during planning.

Run against the same populated hub:

```text
/spec:initiative create polish-pass \
    --shape update-existing \
    --auto-merge
```

Pre-flight forbids `--from`, `--against`, and `--source` under this shape; supplying any is a hard exit. The umbrella runs all seven steps without halting:

1. **Brief.** Scaffolded with `inputs: []`; the operator writes one paragraph naming the capabilities being polished.
2. **Registry.** Multi-project; descriptions complete. No mutation.
3. **Plan.** Discovery falls back to baseline accumulation in `.specify/workspace/<peer>/specs/` because the input set is empty. Propose surfaces two slices (one per project, **no contract change** — the polish does not change the API surface). Assignment routes each slice to its existing project. No registry mutation.
4. **Execute.** Both changes drive to `done`.
5. **Push.** Two PRs opened.
6. **Land.** `--auto-merge` → both PRs squash-merged.
7. **Finalize.** Archive completes.

Verb sequence: `specify initiative create` → `specify registry validate` → `/spec:plan` → `specify plan create` → `specify workspace sync` → `specify plan add` × 2 → `specify plan amend --project` × 2 → `specify plan validate` → `/spec:execute --loop` → `specify workspace push` → `specify workspace merge` → `specify initiative finalize`.

Full transcript and on-disk shapes: [`fixtures/update-existing/`](../../plugins/spec/skills/initiative/fixtures/update-existing/).

### Manual fallback parity

Each step in every shape above is a shell-out the umbrella runs verbatim. Operators can drop down a layer at any step — see [Drop down a layer](../how-to/drop-down-a-layer.md#from-layer-4-to-layer-3-skip-the-umbrella) for the canonical command sequence. The umbrella's value is single-command convenience plus idempotent re-entry; it adds no behaviour beyond the underlying skills and CLI verbs.

## What you learned

- The platform-hub topology (`specify init --hub`) is the canonical starting shape for multi-repo initiatives. The hub holds platform state and never carries code.
- `specify registry add` registers code projects with kebab-case names, schema identifiers, and domain descriptions. Descriptions drive automated assignment in `/spec:plan`.
- `specify initiative create` scaffolds the operator brief; the `inputs:` frontmatter feeds the discovery brief.
- `/spec:plan` runs discovery → sync-peers → propose → assignment, and finishes with `specify plan validate` as the gate. When it detects a cross-project API boundary it inserts a contract change before the implementation changes.
- `/spec:execute --loop` `chdir`s into each workspace clone, runs define-build-merge, transitions the plan entry, and routes back. Multi-repo CWD routing is invisible to the phase skills.
- `specify workspace push` ships local commits as PRs on `specify/<initiative-name>` branches; `specify workspace merge` lands them once CI is green (RFC-9 §4A).
- `specify initiative finalize` is the canonical closure verb (RFC-9 §4C): it confirms every per-project PR is merged on remote, refuses to archive on dirty workspace clones, and optionally prunes `.specify/workspace/<peer>/` via `--clean`.
- The same Steps 1–9 handle three initiative shapes — `migrate-legacy`, `new-feature`, `update-existing` — through a single uniform loop.

## Cross-links

- [Platform repo topologies](../explanation/platform-repo.md) — registry-only hub vs platform-as-project, the validation invariant, and the on-disk shape of each.
- [Workspace tiers](../explanation/workspace-tiers.md) — the legacy-source vs registered-project clone distinction the loop relies on.
- [The Layered Stack](../explanation/three-layer-stack.md) — where `/spec:plan`, `/spec:execute`, and `/spec:initiative` sit in the layered model.
- [`/spec:plan`](../../plugins/spec/skills/plan/SKILL.md) — Layer 3 plan authoring skill.
- [`/spec:execute`](../../plugins/spec/skills/execute/SKILL.md) — Layer 2 plan driver, including the cross-project contract check (RFC-9 §3B).
- [`specify init`](../reference/cli/init.md) — the `--hub` flag.
- [`specify registry`](../reference/cli/registry.md) — `add` / `remove` / `show` / `validate`.
- [`specify workspace`](../reference/cli/workspace.md) — `sync` / `status` / `push` / `merge`.
- [`specify plan`](../reference/cli/plan.md) — `create` / `add` / `amend` / `next` / `doctor` / `archive` / `lock`.
- [Migrating to CLI v1](../explanation/migrating-cli-v1.md) — rename map covering the v1.x `init`→`create` and `create`→`add` renames.

## Next

[Legacy Migration at Scale](legacy-migration-at-scale.md) — decompose a large monolith across multiple target repos using the analyze/extract split.
