# RFC-95: Publication Sets

> Status: Active product follow-on to RFC-88 in the [Services Delivery Programme](platform.md)
>
> Owns: one publication worktree per publication member, the shared branch and pull-request markers, landing order, archive-time publication observation, and the typed publication projection. Does not own the operator's Git commit, forge writes, atomic cross-repository submission, automated rollback, or parallel member preparation.
>
> Builds on implemented [RFC-88](rfc-88-detached-changes.md). Host Git and forge access use Omnia generic interfaces — [host surface](rfc-95-host-surface.md) — not new `emery:*` packages. [RFC-100](rfc-100-distributed-execution.md) may execute across nodes first. RFC-81 in `augentic/remedium` is the first external producer. Pre-implementation review: [rfc-95-review.md](rfc-95-review.md).

## Intent

Make the change, rather than the repository, the unit of publication.

A change such as `checkout-v2` may touch a payment API and the web frontend that consumes it. Execution finishes with one accepted CID per repository. Emery exports each final CID into a normal Git working tree. The operator reviews that diff, commits, pushes a branch and pull request for each member; both carry the same change identity and land in the plan's recorded dependency order.

That collection is the **publication set**. Emery derives it from the plan, materializes each publication worktree, and verifies the set at archive. The operator authors every Git commit and every forge write.

## Flow and terms

1. RFC-88 projects terminal conflict domains into plan entries that name participating targets and leaf `depends-on` edges.
2. When every entry for one target is merged, Emery materializes that target's final accepted CID as a publication worktree on `change/<plan>`: `HEAD` is the recorded parent revision, the index and worktree are the accepted CID, and nothing is committed.
3. The operator reviews with `git diff`, commits, pushes the branch, opens a pull request carrying `Emery-Change: <plan>`, and lands members in dependency order.
4. `emery plan archive` reads the forge, reconstructs the set, and verifies every member before archiving.

Nouns:

- **publication member** — a distinct adapter-bearing target used by at least one slice
- **publication worktree** — a normal Git checkout of that target: branch `change/<plan>`, `HEAD` at the recorded parent, tree equal to the final accepted CID, uncommitted
- **local commit** — the Git commit the operator writes in that worktree
- **publication set** — the change name, members, worktrees, local commits, branches, pull requests, and required landing order
- **publish** — create the remote branch and pull request
- **land** — merge that pull request on the forge

RFC-88's target-wave commit is the accepted-CID transition (initially one-leaf `emery slice merge`; RFC-96 later extends membership without changing the fact shape). A publication set is not an RFC-87 code patch, which relates one base CID to one result CID. The worktree is not an RFC-87 workspace: it is operator-owned publication surface, not disposable execution machinery.

## Worked example

```yaml
name: checkout-v2
targets:
  payments-api:
    adapter: emery:omnia@1.4.0
    locator: https://github.com/example/payments-api@0123456789abcdef0123456789abcdef01234567
    cid: sha256:…
  web-frontend:
    adapter: emery:vectis@1.4.0
    locator: https://github.com/example/web-frontend@89abcdef0123456789abcdef0123456789abcdef
    cid: sha256:…
slices:
  - name: expose-payment-api
    target: payments-api
  - name: adopt-payment-api
    target: web-frontend
    depends-on: [expose-payment-api]
```

Two targets → two members; `payments-api` must land first. Two slices on one target would still be one member and one publication worktree.

Each repository uses branch `change/checkout-v2` and a pull-request body containing:

```text
Emery-Change: checkout-v2
```

Archive projects plan, materialize facts, and forge state into one record. The JSON is shape, not schema (D7):

```json
{
  "change": "checkout-v2",
  "members": [
    {
      "project": "payments-api",
      "repository": "github.com/example/payments-api",
      "commit": "8e43c…",
      "branch": "change/checkout-v2",
      "pull-request": "https://github.com/example/payments-api/pull/412",
      "publication": "merged",
      "order": 1
    },
    {
      "project": "web-frontend",
      "repository": "github.com/example/web-frontend",
      "commit": "c71a9…",
      "branch": "change/checkout-v2",
      "pull-request": "https://github.com/example/web-frontend/pull/98",
      "publication": "open",
      "order": 2
    }
  ],
  "verification": "pending",
  "failures": [
    { "member": "web-frontend", "reason": "unmerged" }
  ]
}
```

Archive stops: the frontend pull request is still open. `publication` is `unpublished | open | merged | closed`. `verification` is `verified | pending | unverified`. `order` is present only because D4 ranks these two members; unrelated members omit it.

## Decisions

### D1 — The plan is the publication set

The plan name is the change identity. Publication members are the distinct adapter-bearing targets referenced by the terminal `slices[].target` projection. A target listed in `plan.yaml` but used only as read-only input, present only on an internal conflict domain, or unused is not a member.

There is no second lifecycle object or authored publication artifact. A single-repository plan is a one-member set.

Publication is incremental per member, not gated on draining the whole plan. Once the final terminal projection is fixed and every entry for a target is merged, that target may receive a publication worktree; the operator then commits, publishes, and lands while other targets continue, subject to D4. Archive remains a whole-set gate.

Progressive authoring, if [RFC-99](rfc-99-streaming-execution.md) later permits it, still cannot materialize until that target's complete entry set and covering execute epoch exist. This cut does not overlap survey, refine, or build with the publication worktree. Completing one leaf never creates a worktree or a commit; each target receives exactly one publication worktree after all of its entries merge. The operator authors the Git commit and every forge write. There is no `plan materialize` / `plan commit` / `plan publish` verb.

### D2 — Plan-backed publication records are derived

Each fact has one home:

- member identity: `slices[].target`
- repository location and initial revision: `plan.yaml.targets`
- publication worktree: D11's fact
- local commit id, remote branch, pull-request reference, and publication status: the forge

Emery joins them for the projection. It does not persist publication-set state in member repositories or restore a registry bridge. It does not record a local clone path on the target binding.

### D3 — Branches and pull requests carry a shared marker

Every member uses branch `change/<plan>` and carries `Emery-Change: <plan>` in the pull-request body.

Those markers reconstruct the set from the forge without the original change home. A forge label may mirror the trailer; the trailer is authoritative.

### D4 — Publication order comes from `depends-on`

Cross-target leaf `depends-on` edges are the publication order. RFC-88 has already compiled internal-domain dependencies into edges between entry and exit leaves; publication does not reread the decomposition. Same-target edges vanish when the leaf graph contracts onto targets. Unrelated targets stay unordered. There is no second ordering field.

Leaf acyclicity is not enough: contraction can yield `target-a → target-b` and `target-b → target-a` through different leaves with no leaf cycle. RFC-88 plan validation contracts the complete leaf graph onto distinct targets and rejects any strongly connected component or self-loop as `publication-target-cycle`. Archive repeats that validation before reading the forge. Only an acyclic contracted graph is a publication partial order.

### D5 — Emery observes publication but does not perform it

Emery never authors a Git commit, pushes a branch, opens a pull request, merges one, or reverts one. Those stay operator-owned under the [CLI contract](../docs/standards/cli-contract.md).

`emery plan archive` observes rather than confirming in prose. It reconstructs the members, reads their pull requests, and checks that:

1. every member has a pull request with the correct `Emery-Change` trailer;
2. every pull request is merged;
3. dependency-ordered members landed in the required order.

Success continues archive. Failure returns `publication-unverified` on exit 1 and names every failing member before changing archive state.

`--unverified` skips only these publication checks. It appends `plan.publication.unverified-archive`; it does not turn a red projection green. `--force` skips only the outstanding-work ladder check. It does not skip publication verification. The two flags compose.

### D6 — The guarantee is coordinated convergence, not atomicity

GitHub cannot atomically merge pull requests across repositories. Emery does not emulate Gerrit's `submitWholeTopic` with merge-all-or-revert.

Contract changes use expand/contract steps as ordered members. An out-of-order landing is a verification finding, not an automated rollback. Deployment compatibility stays with the operator and target adapters.

### D7 — Archive produces the typed publication projection

Before mutation, `emery plan archive` projects members, local commit ids (from the forge), branches, pull requests, publication states, derived order, and the verification verdict. Unchanged plan, facts, and forge state produce byte-stable output.

`publication` is `unpublished | open | merged | closed`. `verification` is `verified | pending | unverified`, plus a stable list of failing members and reasons. No free-text verdict. `order` is present only when D4 assigns a rank in the contracted DAG; unrelated members omit it. Ranks among comparable members are a stable topological numbering.

The worked example is shape, not schema. The schema is a schemars golden generated from the same Rust wire type the projector serializes, committed at `crates/project/answers/publication.schema.json` and parity-gated by `crates/project/tests/answers.rs` (`REGENERATE_GOLDENS=1`), the same path as `leads` / `evidence` / `report`. There is no second schema language.

The projector is an internal read surface, reused by archive and external-record validation. This RFC adds no publication subcommand.

### D8 — External producers use the same record shape

A non-plan system may emit the same publication-set record without acquiring Emery lifecycle authority. A Remedium alert spanning three repositories, for example, emits three members with the same trailer and ordering shape.

Plan-backed records stay derived under D2. External records are producer-authored inputs validated against that golden's Rust wire type; they do not create or mutate an Emery plan.

### D9 — Publication checks are serial and workspace-free

Archive reads resolved plan bindings and forge state serially. It does not prepare, capture, or inspect a product workspace, and it does not inspect the publication worktree.

That keeps verification deterministic and decoupled from checkout state. Parallel member preparation is RFC-96.

### D10 — Archive observes the forge over HTTP

Archive reconstructs publication state by reading the forge. The lookup contract is product policy; the transport is `wasi:http` plus `omnia:identity`, not a new `emery:forge` (or `emery:publication`) host package. See [host surface](rfc-95-host-surface.md).

- Find by `(repository, branch)` where `branch` is `change/<plan>`. Exactly one open-or-merged pull request succeeds; zero is `unpublished`; several fail closed.
- Read returns URL, body, `publication` (`unpublished | open | merged | closed`), and `merged-at` when merged.
- Trailer check is on the body (`Emery-Change: <plan>`). A forge label may exist; it is not the lookup key.
- Landing order is `merged-at` compared along D4's contracted partial order.

Those checks run in the engine over HTTP responses. There is no `gh` subprocess path, no `emery:*` forge import, and no dependency on RM-17. This RFC grants no forge write.

### D11 — One publication worktree per committable result

When every entry for a target is named by an RFC-88 committed target-wave chain and no postflight failure remains unacknowledged, Emery materializes one publication worktree as a side effect of `plan execute`:

```text
recorded initial Git revision + final accepted CID
    → worktree on change/<plan> (HEAD = parent, tree = CID, uncommitted)
    → plan.publication.materialized
```

Three slices may share one atomic wave or several serialized waves for `payments-api`; only the final accepted CID is exported. Workers do not write the worktree. Intermediate candidate or accepted CIDs do not receive branches.

The worktree is a normal Git checkout. `HEAD` is the locator's recorded Git revision (RFC-88 D5). The index and worktree contain the accepted CID. `git diff` and `git diff --cached` are the review surface. Emery does not create a Git commit; the operator does, with their identity, remotes, and credentials.

Placement uses RFC-88's in-place versus detached layouts. This RFC does not record a local clone path on the binding:

- **In-place, one publication member, product repo clean and at the recorded parent** — materialize on `change/<plan>` in that repository.
- **Otherwise** — `git worktree add -b change/<plan>` from an existing clone of the target, or a first-time clone the operator then owns (deployment layout, e.g. `$EMERY_HOME/publication/<target>/`).
- Never the current branch. Never a dirty tree (`publication-worktree-dirty`). Never an RFC-87 workspace, the change home, or an ambient CWD that fails the in-place rule.

RFC-88 already includes code and the merged `.emery/specs/` baseline in the accepted tree, so the worktree assembles no new content. The host provisions the Git checkout and mounts it; the guest writes the accepted CID as files onto that mount; the host stages the index. The engine does not encode Git objects and does not import a Git library. `.git` and nested change homes stay excluded. Empty directories are omitted — Git stores no empty tree without a child. Snapshot objects stay in the RFC-87 store; Git objects live only in the worktree's repository. The two digest schemes are not aliased. The accepted CID remains the deterministic identity. Implementation: [host surface](rfc-95-host-surface.md).

Idempotency: if the worktree still matches the CID, re-entry is a no-op. If the operator has edited and not committed, fail `publication-worktree-dirty`. If the operator has already committed, do not rewind.

The covering `plan.execute.started` epoch (plan digest plus per-leaf refinement digests, including every entry for that target) authorizes materialize. [RFC-102](rfc-102-policy-gated-autonomy.md), when reopened, may add a policy-gated alternative beside that epoch; it is not a prerequisite and grants no Git commit and no forge write. There is no `plan materialize` / `plan commit` / `plan publish` verb.

The worktree prepares publication. It does not create a remote ref, pull request, merge, or revert. The operator reviews, commits, and pushes with ordinary Git. RM-17 may later automate push and pull-request create; it does not invent the worktree.

The idempotent `plan.publication.materialized` fact records target, parent revision, final CID, worktree path, and branch — not a commit id. Archive appends `plan.publication.projected` before mutation (payload: canonical projection digest and `verification` verdict; an observation snapshot, not member authority) and one `plan.publication.member-landed` per member whose pull request is merged (payload: target, pull-request URL, merge commit, `merged-at`). Forge state remains authoritative under D2. `--unverified` appends `plan.publication.unverified-archive`.

## Implementation requirements

- Implement the idempotent publication worktree over the final accepted CID, local `change/<plan>` branch, D11 placement rules, and `plan.publication.materialized` fact, on the [host surface](rfc-95-host-surface.md) (mounted worktree + git-aware blobstore, one cut).
- Document the ordinary Git loop (`cd`, `git diff`, `git commit`, `git push`) and the `Emery-Change` trailer in operator guidance.
- Observe the forge over `wasi:http` + `omnia:identity` with D10's find/read contract. Do not add `emery:forge` or `emery:publication`.
- Implement one typed projector over terminal plan bindings, materialize facts, and forge state. Derive its partial order only from projected leaf `depends-on`; do not add a second decomposition reader.
- Share one target-contraction and cycle-validation kernel between RFC-88 plan validation and archive. Reject `publication-target-cycle` before materialize or forge reads.
- Render the projection before archive mutation; gate unverified publication with `publication-unverified`; journal the `--unverified` bypass.
- Append `plan.publication.projected` and `plan.publication.member-landed` through the existing per-writer fact logs.
- Add the publication-set wire type, generate `crates/project/answers/publication.schema.json` from it, and gate the golden in `crates/project/tests/answers.rs`. External records validate against that type.
- Exercise the WIT-breaking release order from [docs/release.md](../docs/release.md#three-release-shapes) across `augentic/emery` and `augentic/emery-adapters` as the in-house fixture. The first real release dogfoods the settled path; it does not gate RFC completion.

## Acceptance criteria

1. Draining a target materializes its final accepted CID as exactly one publication worktree on `change/<plan>`, with `HEAD` at the recorded initial revision and the tree uncommitted, and records an idempotent fact. It may run before unrelated targets drain, but never before the final terminal projection fixes that target's complete entry set and a covering `plan.execute.started` epoch exists. Completing one leaf creates no worktree and no commit. The worktree is not an RFC-87 workspace and is not a forge write. Re-entry is a no-op when the tree still matches the CID, fails `publication-worktree-dirty` when the operator has unpublished edits, and does not rewind an operator commit.
2. Before changing archive state, `emery plan archive` derives members and repository locations from RFC-88 plan bindings, worktrees from their facts, and branch, pull-request, and commit state from forge HTTP reads (D10).
3. Unchanged facts, plan, and forge state produce a byte-stable projection. A single-repository plan uses the same schema with one member and one publication worktree.
4. Publication order derives only from cross-target leaf `depends-on` edges, including RFC-88's projection of domain dependencies. Unrelated members carry no extra constraint; archive does not reread internal domains. A leaf-acyclic fixture whose target contraction contains a two-target cycle fails `publication-target-cycle` at plan validation and archive.
5. Archive verifies trailers, merged state, and landing order. Failures name every affected member. `--unverified` archives only after appending its fact.
6. External records validate against `crates/project/answers/publication.schema.json` (the schemars golden of the projector wire type) and project through the same read surface without acquiring plan lifecycle authority.
7. A WIT-breaking engine and adapter release fixture is represented and verified as a multi-member publication set.
8. `cargo make ci` passes with crate-level integration coverage for the publication worktree (in-place single-member, linked worktree, first-time clone, dirty refusal, already-committed no-rewind), one and many members, missing/open/closed/merged pull requests, ordering, trailer mismatch, HTTP/identity failure, external records, and the unverified bypass.

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
- **`emery:publication` / `emery:forge` host packages** — workflow nouns in WIT. Git and forge access use `wasi:blobstore`, a mounted `wasi:filesystem` preopen, and `wasi:http` + `omnia:identity` ([host surface](rfc-95-host-surface.md)).
- **A `wasi-git` WIT, or guest-visible `wasi:exec` as a Git placeholder** — Git stays in the backend and in host worktree provision.
- **Sequencing the git-aware blobstore and the mounted worktree as separate cuts** — one host-surface implementation.
