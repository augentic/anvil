# RFC-95: Publication Sets

> Status: Implemented product follow-on to RFC-88 in the [Services Delivery Programme](platform.md)
>
> Owns: one publication worktree per publication member, the shared branch and pull-request markers, landing order, archive-time publication observation, and the typed publication projection. Does not own the operator's Git commit, forge writes, atomic cross-repository submission, automated rollback, or parallel member preparation.
>
> Builds on implemented [RFC-88](rfc-88-detached-changes.md). Host Git and forge access use the VCS seam — [host surface](rfc-95-host-surface.md): one `emery:vcs` package (`trees` / `worktree` / `forge`), publication export as one host call, no workflow-noun host packages. RFC-81 in `augentic/remedium` is the first external producer.

## Intent

Make the change, rather than the repository, the unit of publication.

A change such as `checkout-v2` may touch a payment API and the web frontend that consumes it. Execution finishes with one accepted CID per repository. Emery exports each final CID into a normal Git working tree. The operator reviews that diff, commits, pushes a branch and pull request for each member; both carry the same change identity and land in the plan's recorded dependency order.

That collection is the **publication set**. Emery derives it from the plan, materializes each publication worktree, and verifies the set at archive. The operator authors every Git commit and every forge write.

## Flow and terms

1. RFC-88 projects terminal conflict domains into plan entries that name participating targets and leaf `depends-on` edges.
2. When every in-scope entry for one target is merged, Emery materializes that target's final accepted CID as a publication worktree on `change/<plan>`: `HEAD` is the recorded parent revision, the index and worktree are the accepted CID, and nothing is committed.
3. The operator reviews with `git diff`, commits, pushes the branch, opens a pull request carrying the D3 trailers (`Emery-Change: <plan>` plus the plan digest), and lands members in dependency order.
4. `emery plan archive` reads the forge, reconstructs the set, and verifies every member before archiving.

Nouns:

- **publication member** — a distinct adapter-bearing target used by at least one in-scope slice
- **publication worktree** — a normal Git checkout of that target: branch `change/<plan>`, `HEAD` at the recorded parent, tree equal to the final accepted CID, uncommitted
- **local commit** — the Git commit the operator writes in that worktree; Emery never records it
- **merge commit** — the commit the forge reports when the pull request lands; recorded per member at archive as `merge-commit`
- **publication set** — the change name, members, worktrees, local commits, branches, pull requests, and required landing order
- **publish** — create the remote branch and pull request
- **land** — merge that pull request on the forge

RFC-88's target-wave commit is the accepted-CID transition — initially a one-member wave committed by the merge phase of `emery plan execute` (`target.merge.wave-committed`); RFC-96 later extends membership without changing the fact shape. A publication set is not an RFC-87 code patch, which relates one base CID to one result CID. The worktree is not an RFC-87 workspace: it is operator-owned publication surface, not disposable execution machinery.

## Worked example

```yaml
name: checkout-v2
targets:
  payments-api:
    adapter: emery:omnia@1.4.0
    locator: https://github.com/example/payments-api@0123456789abcdef0123456789abcdef01234567
    cid: sha256:…
  checkout-service:
    adapter: emery:omnia@1.4.0
    locator: https://github.com/example/checkout-service@89abcdef0123456789abcdef0123456789abcdef
    cid: sha256:…
slices:
  - name: expose-payment-api
    target: payments-api
  - name: adopt-payment-api
    target: checkout-service
    depends-on: [expose-payment-api]
```

Two targets → two members; `payments-api` must land first. Two slices on one target would still be one member and one publication worktree.

Each repository uses branch `change/checkout-v2` and a pull-request body containing:

```text
Emery-Change: checkout-v2
Emery-Change-Digest: sha256:…
```

Archive projects plan, materialize facts, and forge state into one record. The JSON is shape, not schema (D7):

```json
{
  "change": "checkout-v2",
  "members": [
    {
      "target": "payments-api",
      "repository": "github.com/example/payments-api",
      "merge-commit": "8e43c…",
      "branch": "change/checkout-v2",
      "pull-request": "https://github.com/example/payments-api/pull/412",
      "base": "main",
      "publication": "merged",
      "order": 1
    },
    {
      "target": "checkout-service",
      "repository": "github.com/example/checkout-service",
      "merge-commit": null,
      "branch": "change/checkout-v2",
      "pull-request": "https://github.com/example/checkout-service/pull/98",
      "base": "main",
      "publication": "open",
      "order": 2
    }
  ],
  "verification": "pending",
  "failures": [
    { "member": "checkout-service", "reason": "unmerged" }
  ]
}
```

Archive stops: the checkout-service pull request is still open. `publication` is `unpublished | open | merged | closed`. `verification` is `verified | pending | unverified`. `merge-commit` is the forge's merge commit, present only when merged; the operator's local commit is never recorded. `base` is the observed pull-request base branch (recorded, not gated — D10). `order` is present only because D4 ranks these two members; unrelated members omit it.

## Decisions

### D1 — The plan is the publication set

The plan name is the change identity. Publication members are the distinct adapter-bearing targets referenced by the terminal `slices[].target` projection, counting **in-scope entries only** (the same `plan::in_scope` execute, refine, epoch, and gaps already use). A target listed in `plan.yaml` but used only as read-only input, present only on an internal conflict domain, unused, or referenced only by dropped entries is not a member; a dropped-only target does not block archive. Contraction for materialize and archive uses the in-scope graph; author-time cycle validation may keep the full graph — cycles cannot appear from drop alone.

There is no second lifecycle object or authored publication artifact. A single-repository plan is a one-member set.

Publication is incremental per member, not gated on draining the whole plan. Once the final terminal projection is fixed and every in-scope entry for a target is merged, that target may receive a publication worktree; the operator then commits, publishes, and lands while other targets continue, subject to D4. Archive remains a whole-set gate.

Progressive authoring, if [RFC-99](rfc-99-streaming-execution.md) later permits it, still cannot materialize until that target's complete in-scope entry set is fixed and D11's fact predicate holds. This cut does not overlap survey, refine, or build with the publication worktree. Completing one leaf never creates a worktree or a commit; each target receives exactly one publication worktree after all of its in-scope entries merge. The operator authors the Git commit and every forge write. There is no `plan materialize` / `plan commit` / `plan publish` verb.

### D2 — Plan-backed publication records are derived

Each fact has one home:

- member identity: `slices[].target`
- repository location and initial revision: `plan.yaml.targets`
- publication worktree: D11's fact
- merge commit, remote branch, pull-request reference, base branch, and publication status: the forge

Emery joins them for the projection. It does not persist publication-set state in member repositories or restore a registry bridge. It does not record a local clone path on the target binding.

### D3 — Branches and pull requests carry a shared marker

Every member uses branch `change/<plan>` and carries two trailer lines in the pull-request body: `Emery-Change: <plan>` and `Emery-Change-Digest: <covering plan digest>`. The digest disambiguates a reused plan name against the same repository over time — D10's lookup matches both, so a later change cannot false-verify against a previous change's merged pull request.

Those markers reconstruct the set from the forge without the original change home. A forge label may mirror the trailers; the trailers are authoritative.

### D4 — Publication order comes from `depends-on`

Cross-target leaf `depends-on` edges are the publication order. RFC-88 has already compiled internal-domain dependencies into edges between entry and exit leaves; publication does not reread the decomposition. Same-target edges vanish when the leaf graph contracts onto targets. Unrelated targets stay unordered. There is no second ordering field.

Leaf acyclicity is not enough: contraction can yield `target-a → target-b` and `target-b → target-a` through different leaves with no leaf cycle. RFC-88 plan validation contracts the complete leaf graph onto distinct targets and rejects any strongly connected component or self-loop as `publication-target-cycle`. Archive repeats that validation before reading the forge. Only an acyclic contracted graph is a publication partial order.

### D5 — Emery observes publication but does not perform it

Emery never authors a Git commit, pushes a branch, opens a pull request, merges one, or reverts one. Those stay operator-owned under the [CLI contract](../docs/standards/cli-contract.md).

`emery plan archive` observes rather than confirming in prose. It reconstructs the members, reads their pull requests, and checks that:

1. every member has a pull request with the correct `Emery-Change` trailer;
2. every pull request is merged;
3. dependency-ordered members landed in the required order.

Verification does **not** require the merge-commit tree to equal the accepted CID. An operator who amends the commit before push archives green: operator Git is authoritative, and the accepted CID is never aliased to a Git SHA. This is a recorded decision, not a hole (see rejected alternatives).

Success continues archive. Failure returns `publication-unverified` on exit 1 and names every failing member before changing archive state.

`--unverified` skips only these publication checks. It appends `plan.publication.unverified-archive`; it does not turn a red projection green. `--force` skips only the outstanding-work ladder check. It does not skip publication verification. The two flags compose.

### D6 — The guarantee is coordinated convergence, not atomicity

GitHub cannot atomically merge pull requests across repositories. Emery does not emulate Gerrit's `submitWholeTopic` with merge-all-or-revert.

Contract changes use expand/contract steps as ordered members. An out-of-order landing is a verification finding, not an automated rollback. Deployment compatibility stays with the operator and target adapters.

### D7 — Archive produces the typed publication projection

Before mutation, `emery plan archive` projects members, merge commits (from the forge), branches, pull requests, base branches, publication states, derived order, and the verification verdict. Unchanged plan, facts, and forge state produce byte-stable output.

`publication` is `unpublished | open | merged | closed`. `verification` is `verified | pending | unverified`, plus a stable list of failing members and reasons. No free-text verdict. `order` is present only when D4 assigns a rank in the contracted DAG; unrelated members omit it. Ranks are Kahn topological order over the contracted DAG with a **sorted** ready set — a closed algorithm, so unchanged plan, facts, and forge state remain byte-stable. An ordered pair with equal `merged-at` fails verification; there is no tie-break.

The worked example is shape, not schema. The schema is a schemars golden generated from the same Rust wire type the projector serializes, committed at `crates/project/answers/publication.schema.json` and parity-gated by `crates/project/tests/answers.rs` (`REGENERATE_GOLDENS=1`), the same path as `leads` / `evidence` / `report`. There is no second schema language.

The projector is an internal read surface, reused by archive and external-record validation. This RFC adds no publication subcommand.

### D8 — External producers use the same record shape

A non-plan system may emit the same publication-set record without acquiring Emery lifecycle authority. A Remedium alert spanning three repositories, for example, emits three members with the same trailer and ordering shape.

Plan-backed records stay derived under D2. External records are producer-authored inputs validated against that golden's Rust wire type; they do not create or mutate an Emery plan.

### D9 — Publication checks are serial and workspace-free

Archive reads resolved plan bindings and forge state serially. It does not prepare, capture, or inspect a product workspace, and it does not inspect the publication worktree.

That keeps verification deterministic and decoupled from checkout state. Parallel member preparation is RFC-96.

### D10 — Archive observes the forge over HTTP

Archive reconstructs publication state by reading the forge. The lookup contract is product policy; the transport is the VCS seam's `forge` interface — a host-side GitHub backend returning typed results, with the token supplied by backend configuration and never crossing into the guest. See [host surface](rfc-95-host-surface.md). GitHub is the only v1 forge; `https://github.com/org/repo@sha` locators map to the REST repository by stripping the scheme, host, and revision.

- Find by `(repository, branch)` where `branch` is `change/<plan>`, across **open, merged, and closed** pull requests, following pagination to exhaustion before applying the zero / one / several rule. A fork's head (`owner:branch`) does not match; only same-repository heads count.
- The trailer check is on the body: `Emery-Change: <plan>` **and** the covering plan digest (D3), so a later change reusing a plan name against the same repository cannot false-verify against the previous merged pull request. Exactly one matching pull request succeeds; zero is `unpublished`; several fail closed. A forge label may exist; it is not the lookup key.
- Read returns URL, body, `publication` (`unpublished | open | merged | closed`), the base branch, the merge commit, and `merged-at` when merged. Draft, squash, and rebase count as merged when the forge says merged. The projection records the base branch per member but does not gate on it — consistent with D5, the operator owns where the change lands; an unexpected base is visible at archive.
- Landing order is `merged-at` compared along D4's contracted partial order (equal `merged-at` on an ordered pair fails, D7).
- Transport or authentication failure is not `publication-unverified`: `pending`, `unverified`, and HTTP failure are three different outcomes.

Those checks run in the engine over the typed results. There is no `gh` subprocess path, no outgoing HTTP in the guest, and no dependency on RM-17. This RFC grants no forge write; RM-17 may later add write operations to the same `forge` interface behind an explicit grant.

### D11 — One publication worktree per committable result

Materialize is authorized from the fact union, not from one epoch payload: the current plan digest matches, every **in-scope** entry for the target is named by a committed target-wave chain (`target.merge.wave-committed`), each wave carries its own valid commit authorization, and no postflight failure remains unacknowledged. Epoch assembly deliberately excludes entries already projected `done`, so no single `plan.execute.started` payload covers a target that drained across several execute invocations; the epoch wire shape is not widened to compensate — the epoch stays coverage for what that run may execute. When the predicate holds, Emery materializes one publication worktree as a side effect of `plan execute`:

```text
recorded initial Git revision + final accepted CID
    → worktree on change/<plan> (HEAD = parent, tree = CID, uncommitted)
    → plan.publication.materialized
```

Three slices may share one atomic wave or several serialized waves for `payments-api`; only the final accepted CID is exported. Workers do not write the worktree. Intermediate candidate or accepted CIDs do not receive branches.

The worktree is a normal Git checkout. `HEAD` is the locator's recorded Git revision (RFC-88 D5). The index and worktree contain the accepted CID. `git diff` and `git diff --cached` are the review surface. Emery does not create a Git commit; the operator does, with their identity, remotes, and credentials.

Placement has exactly two cases. This RFC does not record a local clone path on the binding, and there is no "existing clone" search:

- **In-place, one publication member, product repo clean and at the recorded parent** — materialize on `change/<plan>` in that repository.
- **Otherwise** — reuse or create `$EMERY_HOME/publication/<plan>/<target>/`; the plan segment keeps concurrent changes touching the same target, or a reused plan name, from colliding on one checkout. The first-time clone root is that path, not an example.
- Never the current branch. Never a dirty tree (`publication-worktree-dirty`). Never an RFC-87 workspace, the change home, or an ambient CWD that fails the in-place rule.

RFC-88 already includes code and the merged `.emery/specs/` baseline in the accepted tree, so the worktree assembles no new content. Materialize is **one host call** on the VCS seam's `worktree` interface: the host provisions the checkout, creates the branch at the recorded parent, materializes the CID with the in-process workspace kernel (real permission bits — no `emery:exec-mode` widening), and stages the index so `git diff` and `git diff --cached` read against the parent. No publication mount, no guest file writes, no post-exit staging hook. The engine does not encode Git objects and does not import a Git library. `.git` and nested change homes stay excluded. Empty directories are omitted — Git stores no empty tree without a child. Snapshot objects stay in the RFC-87 store; Git objects live only in the worktree's repository. The two digest schemes are not aliased. The accepted CID remains the deterministic identity, and accepted CIDs **remain** snapshot GC roots after materialize — the publication worktree is not the store, and `plan archive` sweep policy does not change because a checkout now exists. Implementation: [host surface](rfc-95-host-surface.md).

The worktree and branch state table is closed:

- Tree still matches the CID → no-op.
- A later leaf or `plan amend` produces a **new** accepted CID on an **uncommitted** worktree → rematerialize.
- Operator has uncommitted edits → `publication-worktree-dirty`.
- No-rewind predicate: `HEAD` has moved off the recorded parent → the operator has committed; leave everything, including a subsequently dirtied tree — their commit is authoritative.
- Branch exists at the recorded parent and the destination is the expected worktree → reuse, then apply the rows above.
- Branch exists at any other commit → `publication-provision-failed` / `branch-diverged`; never delete or move operator state.
- Branch checked out in another linked worktree → `publication-provision-failed` / `branch-checked-out-elsewhere`.
- Destination exists but is not the expected worktree → `publication-provision-failed` / `destination-conflict`.
- Recorded parent absent from the clone → fetch once; still absent → `publication-provision-failed` / `parent-unavailable`.
- First-time clone or network failure → `publication-provision-failed` / `clone-failed`.
- Interrupted provision or write → the tree will not match the CID; re-entry rematerializes.
- An empty diff (accepted CID equals parent tree) still materializes, still requires the pull request, and verifies normally — membership derives from the plan, not from the diff.

Materialize failures are stop conditions, not hard errors: `publication-worktree-dirty` and `publication-provision-failed` (carrying the member and one of the closed provisioning reasons `branch-diverged | branch-checked-out-elsewhere | destination-conflict | parent-unavailable | clone-failed`) are stop reasons whose resume path is fixing the worktree and re-running `emery plan execute`. The reason set is closed — a new failure mode is an RFC edit, not a new string. The stop projects as the member's next action in the status milestone.

Materialize is part of the drain condition: a plan does not project `drained` until every publication member carries its `plan.publication.materialized` fact, and execute reconciles pending materializations (and only those) before its drained early-return, without opening a new epoch — authorization is the fact predicate above, not fresh coverage. A plan drained under an older binary therefore re-projects as not-drained after upgrade, and re-running `emery plan execute` materializes it. After `plan.publication.materialized`, topology edits that would add, remove, or rebind that target's in-scope entries are rejected until archive.

[RFC-102](rfc-102-policy-gated-autonomy.md), when reopened, may add a policy-gated alternative beside that predicate; it is not a prerequisite and grants no Git commit and no forge write. There is no `plan materialize` / `plan commit` / `plan publish` verb.

The worktree prepares publication. It does not create a remote ref, pull request, merge, or revert. The operator reviews, commits, and pushes with ordinary Git. RM-17 may later automate push and pull-request create; it does not invent the worktree.

Every `plan.publication.*` payload carries the plan name and the covering plan digest — the change events directory outlives one plan's archive and plan names recur, so target + branch is not a safe join key across successive changes. Match keys follow the `gap.deferred` precedent: `plan.publication.materialized` records target, parent revision, final CID, worktree path, and branch — not a commit id — and dedupes on `(target, accepted CID)`; a re-run that would restate an existing fact appends nothing. The worktree path is node-local observation, never portable authority — the projector must not treat it as resolvable off-node. Archive appends `plan.publication.projected` before mutation (payload: canonical projection digest and `verification` verdict; repeated observations are explicitly legal — each is a timestamped snapshot, not member authority) and one `plan.publication.member-landed` per member whose pull request is merged (payload: target, pull-request URL, merge commit, `merged-at`; dedupes on `(target, pull-request, merge commit)`). Forge state remains authoritative under D2. `--unverified` appends `plan.publication.unverified-archive`.

Archive mutation order is fixed: project → verify → journal `plan.publication.projected` / `plan.publication.member-landed` → mutate archive → sweep. A crash after journal and before mutation is resume-safe. `--unverified` still journals `plan.publication.unverified-archive` first; `--force` still skips only the outstanding-work ladder.

## Implementation requirements

- Implement the idempotent publication worktree over the final accepted CID, local `change/<plan>` branch, D11 placement rules and state table, and `plan.publication.materialized` fact, as the [host surface](rfc-95-host-surface.md)'s `emery:vcs` `worktree.export` host call — no publication mount, no git-aware blobstore.
- Land the host-surface prerequisite and fetch cuts first (WIT split; `trees` replacing `emery:origins` / `emery:ingest`) per the host surface's sequencing table.
- Document the ordinary Git loop (`cd`, `git diff`, `git commit`, `git push`) and both `Emery-Change` trailers in operator guidance.
- Grow `plan status` a publication milestone: per-member materialized / committed / pull request open / merged, plus the next operator Git step (`commit` / `push` / open PR / land). No publication verb. Materialize joins the drain condition (D11); the operator-owned steps never gate drain, and drain must not imply that `emery plan archive` / `/emery:finalize` will succeed.
- Observe the forge through the VCS seam's `forge` interface with D10's find/read contract. Do not add `emery:forge` or `emery:publication`, and put no outgoing HTTP in the guest.
- Implement one typed projector over terminal plan bindings, materialize facts, and forge state. Derive its partial order only from projected leaf `depends-on` over the in-scope graph; do not add a second decomposition reader.
- Share one target-contraction and cycle-validation kernel between RFC-88 plan validation and archive. Reject `publication-target-cycle` before materialize or forge reads.
- Render the projection before archive mutation; gate unverified publication with `publication-unverified`; journal the `--unverified` bypass; follow the D11 archive mutation order.
- Append `plan.publication.projected` and `plan.publication.member-landed` through the existing per-writer fact logs, with D11's payload identity and match keys.
- Add the publication-set wire type, generate `crates/project/answers/publication.schema.json` from it, and gate the golden in `crates/project/tests/answers.rs`. External records validate against that type in crate-level tests; this cut adds no publication subcommand and no `--record` flag.
- Update the same-change prose that asserts the operator checkout is never written (AGENTS.md, `docs/standards/workflow.md`, the RFC-87/88 sentences) for D11's in-place case, patch RFC-88 D7's retained merge-verb wording and RFC-91's archive sentence, and align `docs/standards/cli-contract.md`'s finalize prose with D5's forge observation.
- Exercise the WIT-breaking release order from [docs/release.md](../docs/release.md#three-release-shapes) across `augentic/emery` and `augentic/emery-adapters` as the in-house fixture. The first real release dogfoods the settled path; it does not gate RFC completion.

## Acceptance criteria

1. Draining a target's in-scope entries materializes its final accepted CID as exactly one publication worktree on `change/<plan>`, with `HEAD` at the recorded initial revision and the tree uncommitted, and records an idempotent fact keyed on `(target, accepted CID)`. It may run before unrelated targets drain, but never before the final terminal projection fixes that target's complete in-scope entry set and D11's fact predicate holds. Completing one leaf creates no worktree and no commit. The worktree is not an RFC-87 workspace and is not a forge write. Re-entry follows the D11 state table: no-op when the tree still matches the CID, rematerialize on a new accepted CID over an uncommitted tree, `publication-worktree-dirty` when the operator has unpublished edits, no rewind after an operator commit, `publication-provision-failed` on the refusal rows. A plan is not `drained` until every member is materialized; execute reconciles pending materializations on re-entry without opening a new epoch, including plans drained under a pre-RFC-95 binary.
2. Before changing archive state, `emery plan archive` derives members and repository locations from RFC-88 plan bindings, worktrees from their facts, and branch, pull-request, base, merge-commit, and landing state from the VCS seam's typed forge reads (D10).
3. Unchanged facts, plan, and forge state produce a byte-stable projection. A single-repository plan uses the same schema with one member and one publication worktree.
4. Publication order derives only from cross-target leaf `depends-on` edges, including RFC-88's projection of domain dependencies. Unrelated members carry no extra constraint; archive does not reread internal domains. A leaf-acyclic fixture whose target contraction contains a two-target cycle fails `publication-target-cycle` at plan validation and archive.
5. Archive verifies trailers, merged state, and landing order. Failures name every affected member. `--unverified` archives only after appending its fact.
6. External records validate against `crates/project/answers/publication.schema.json` (the schemars golden of the projector wire type) in crate-level tests and project through the same read surface without acquiring plan lifecycle authority. No publication subcommand and no `--record` flag exist.
7. A WIT-breaking engine and adapter release fixture is represented and verified as a multi-member publication set.
8. `cargo make ci` passes with crate-level integration coverage for the publication worktree (in-place single-member, `$EMERY_HOME` placement, first-time clone, dirty refusal, already-committed no-rewind, rematerialize, every `publication-provision-failed` row, stop-reason resume), one and many members, missing/open/closed/merged pull requests, ordering including the equal-`merged-at` failure, trailer and digest mismatch, transport failure as its own outcome, external records, and the unverified bypass. Launcher tests cover the export porcelain over temp Git repositories; a local HTTP fixture and the native provider's scripted forge double cover D10.

## Rejected alternatives

- **Monorepo consolidation** — a different question, and unavailable when repository boundaries mirror organisation boundaries. Coordination must work over the fleet that exists.
- **Meta-repository or submodules** — pin member SHAs but couple clone, CI, and review. The set needs a name and members, not a super-repository.
- **Multi-target slices** — breaks singular `slices[].target` and per-target synthesis headers. A plan already spans targets across slices.
- **A separate publication artifact or registry** — a second member authority beside `plan.yaml`. RFC-88 removed the detached registry.
- **`submitWholeTopic` emulation** — would have Emery merge and revert on the forge, which GitHub cannot do atomically across repositories. Ordered convergence is the honest guarantee.
- **An external system as record owner** — [Sourcegraph Batch Changes](https://sourcegraph.com/docs/batch-changes) has the right tracking shape but the wrong authority for plan-backed changes. External systems may emit D8 records; they do not own Emery workflow state.
- **Forge labels as the authoritative marker** — useful views, but forge-specific and easy to retag. The pull-request trailer is portable and travels with the review record.
- **Cross-checkout speculative CI in archive** — publication verification reads settled forge state. RFC-96's local domain gates own pre-merge composition; RFC-100 only transports that model.
- **Assuming leaf acyclicity implies publication acyclicity** — contracting several leaves onto one target can create a target cycle absent from the leaf graph. The contracted graph is validated explicitly.
- **A host-owned bare Git repository as the operator surface** — forces `--git-dir` push from a store with no remotes or credentials. The worktree is ordinary Git.
- **Emery-authored Git commits for cross-machine commit-id stability** — the accepted CID is already the deterministic identity. Git history is the operator's publication record.
- **Restoring `apply` onto ambient CWD** — a dirty checkout, a detached change with no product tree, and a two-target plan are why RFC-88 deleted it. The dedicated `change/<plan>` worktree is the git-native substitute.
- **Recording a local clone path on the target binding** — laptop-specific deployment state, not delivery-binding authority.
- **Building or verifying inside the publication worktree** — restores shared Git state during execute. RFC-87 workspaces stay private; this RFC exports the final CID once.
- **`emery:publication` / `emery:forge` host packages** — workflow nouns in WIT. Git and forge access use the VCS seam's domain interfaces ([host surface](rfc-95-host-surface.md)).
- **A git-aware `wasi:blobstore` backend as the Git seam** — a blob API on a tree domain, with product semantics smuggled into container names; rejected in the host surface.
- **A `wasi-git` WIT, or guest-visible `wasi:exec` as a Git placeholder** — Git stays in the host backend behind the VCS seam.
- **Verifying the pull-request head tree against the materialized tree** — operator Git is authoritative (D5); this stays available to a later cut without changing the fact shape if the `materialized` payload ever grows the staged tree SHA.
- **Verifying the pull-request base branch** — the operator owns where the change lands; the projection records the base so an unexpected one is visible at archive, without gating on it.
