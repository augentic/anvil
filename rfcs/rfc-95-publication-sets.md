# RFC-95: Publication Sets

> Status: Active product follow-on to RFC-88 in the [Services Delivery Programme](platform.md)
>
> Owns: one local project seal per publication member, the shared branch and pull-request markers, landing order, archive-time verification, and the typed publication projection. Does not own forge writes, atomic cross-repository submission, automated rollback, or parallel member preparation.
>
> Builds on implemented [RFC-88](rfc-88-detached-changes.md); extends its forge provider with publication reads. [RFC-100](rfc-100-distributed-execution.md) may execute across nodes first. RFC-81 in `augentic/remedium` is the first external producer.

## Intent

Make the change, rather than the repository, the unit of publication.

A change such as `checkout-v2` may touch a payment API and the web frontend that consumes it. Execution finishes with one committable result per repository. The operator then pushes a branch and pull request for each; both carry the same change identity and land in the plan's recorded dependency order.

That collection is the **publication set**. Emery derives it from the plan, seals each local commit, and verifies the set at archive. The operator owns every forge write.

## Flow and terms

1. RFC-88 projects terminal conflict domains into plan entries that name participating targets and leaf `depends-on` edges.
2. When every entry for one target is merged, Emery seals that target's final accepted CID into one local commit on `change/<plan>`.
3. The operator pushes the branch, opens a pull request carrying `Emery-Change: <plan>`, and lands members in dependency order.
4. `emery plan archive` reads the forge, reconstructs the set, and verifies every member before archiving.

Nouns:

- **publication member** — a distinct adapter-bearing target used by at least one slice
- **project seal** — that target's final accepted CID as one local Git commit
- **publication set** — the change name, members, sealed commits, branches, pull requests, and required landing order
- **publish** — create the remote branch and pull request
- **land** — merge that pull request on the forge

RFC-88's target-wave commit is the accepted-CID transition (initially one-leaf `emery slice merge`; RFC-96 later extends membership without changing the fact shape). A publication set is not an RFC-87 code patch, which relates one base CID to one result CID.

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

Two targets → two members; `payments-api` must land first. Two slices on one target would still be one member and one sealed commit.

Each repository uses branch `change/checkout-v2` and a pull-request body containing:

```text
Emery-Change: checkout-v2
```

Archive projects plan, seal facts, and forge state into one record (command envelope omitted):

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
  "verification": "pending — web-frontend unmerged"
}
```

Archive stops: the frontend pull request is still open. `publication` is `unpublished | open | merged | closed`.

## Decisions

### D1 — The plan is the publication set

The plan name is the change identity. Publication members are the distinct adapter-bearing targets referenced by the terminal `slices[].target` projection. A target listed in `plan.yaml` but used only as read-only input, present only on an internal conflict domain, or unused is not a member.

There is no second lifecycle object or authored publication artifact. A single-repository plan is a one-member set.

Publication is incremental per member, not gated on draining the whole plan. Once the final terminal projection is fixed and every entry for a target is merged, that target may seal, publish, and land while other targets continue, subject to D4. Archive remains a whole-set gate.

Progressive authoring does not weaken the seal. Survey, refine, and build may overlap, but no result advances an accepted CID until the final projection has exact manual or RFC-102 policy commit authorization. Completing one leaf never creates a commit; each target receives exactly one seal after all of its entries merge. Forge publication stays operator-owned.

### D2 — Plan-backed publication records are derived

Each fact has one home:

- member identity: `slices[].target`
- repository location and initial revision: `plan.yaml.targets`
- sealed commit: D11's fact
- remote branch, pull-request reference, and publication status: the forge

Emery joins them for the projection. It does not persist publication-set state in member repositories or restore a registry bridge.

### D3 — Branches and pull requests carry a shared marker

Every member uses branch `change/<plan>` and carries `Emery-Change: <plan>` in the pull-request body.

Those markers reconstruct the set from the forge without the original change home. A forge label may mirror the trailer; the trailer is authoritative.

### D4 — Publication order comes from `depends-on`

Cross-target leaf `depends-on` edges are the publication order. RFC-88 has already compiled internal-domain dependencies into edges between entry and exit leaves; publication does not reread the decomposition. Same-target edges vanish when the leaf graph contracts onto targets. Unrelated targets stay unordered. There is no second ordering field.

Leaf acyclicity is not enough: contraction can yield `target-a → target-b` and `target-b → target-a` through different leaves with no leaf cycle. RFC-88 plan validation contracts the complete leaf graph onto distinct targets and rejects any strongly connected component or self-loop as `publication-target-cycle`. Archive repeats that validation before reading the forge. Only an acyclic contracted graph is a publication partial order.

### D5 — Emery observes publication but does not perform it

Emery never pushes a branch, opens a pull request, merges one, or reverts one. Those stay operator-owned under the [CLI contract](../docs/standards/cli-contract.md).

`emery plan archive` observes rather than confirming in prose. It reconstructs the members, reads their pull requests, and checks that:

1. every member has a pull request with the correct `Emery-Change` trailer;
2. every pull request is merged;
3. dependency-ordered members landed in the required order.

Success continues archive. Failure returns `publication-unverified` on exit 1 and names every failing member before changing archive state.

`--unverified` lets the operator archive over a failed check. It appends `plan.publication.unverified-archive`; it does not turn a red projection green.

### D6 — The guarantee is coordinated convergence, not atomicity

GitHub cannot atomically merge pull requests across repositories. Emery does not emulate Gerrit's `submitWholeTopic` with merge-all-or-revert.

Contract changes use expand/contract steps as ordered members. An out-of-order landing is a verification finding, not an automated rollback. Deployment compatibility stays with the operator and target adapters.

### D7 — Archive produces the typed publication projection

Before mutation, `emery plan archive` projects members, sealed commit ids, branches, pull requests, publication states, derived order, and the verification verdict. Unchanged plan, facts, and forge state produce byte-stable output.

The projector is an internal read surface, reused by archive and external-record validation. This RFC adds no publication subcommand.

### D8 — External producers use the same record shape

A non-plan system may emit the same publication-set record without acquiring Emery lifecycle authority. A Remedium alert spanning three repositories, for example, emits three members with the same trailer and ordering shape.

Plan-backed records stay derived under D2. External records are producer-authored inputs validated against the shared schema; they do not create or mutate an Emery plan.

### D9 — Publication checks are serial and workspace-free

Archive reads resolved plan bindings and forge state serially. It does not prepare, capture, or inspect a product workspace.

That keeps verification deterministic and decoupled from checkout state. Parallel member preparation is RFC-96.

### D10 — The forge provider owns publication reads

RFC-88's host forge provider gains `find-pull-request` and `read-pull-request`. The shipped GitHub binding implements both.

There is no `gh` subprocess path and no dependency on RM-17. The provider gains no forge write capability.

### D11 — One project seal creates each committable result

When every entry for a target is named by an RFC-88 committed target-wave chain and no postflight failure remains unacknowledged, Emery creates one local Git commit:

```text
recorded initial Git revision + final accepted CID
    → one commit on change/<plan>
    → plan.publication.project-sealed
```

Three slices may share one atomic wave or several serialized waves for `payments-api`; only the final accepted CID becomes repository history. Workers do not commit. Intermediate candidate or accepted CIDs do not receive branches.

The seal writes the commit from the immutable tree in the host-owned Git object store. It does not materialize a workspace or touch the operator checkout. RFC-88 already includes code and the merged `.emery/specs/` baseline in the accepted tree, so the seal assembles no new content.

Commit author, message, and timestamp come from the plan and the closed-plan commit-authorization epoch covering its final wave. The same parent and final tree therefore produce the same commit id. The idempotent fact records project, parent revision, final CID, commit id, and branch.

The seal prepares publication. It does not create a remote ref, pull request, merge, or revert.

## Implementation requirements

- Implement the idempotent project seal over the final accepted CID, local `change/<plan>` ref, and `plan.publication.project-sealed` fact.
- Document the `change/<plan>` branch and `Emery-Change` trailer in operator guidance.
- Extend the forge provider and shipped GitHub binding with `find-pull-request` and `read-pull-request`.
- Implement one typed projector over terminal plan bindings, seal facts, and forge state. Derive its partial order only from projected leaf `depends-on`; do not add a second decomposition reader.
- Share one target-contraction and cycle-validation kernel between RFC-88 plan validation and archive. Reject `publication-target-cycle` before sealing or forge reads.
- Render the projection before archive mutation; gate unverified publication with `publication-unverified`; journal the `--unverified` bypass.
- Append `plan.publication.projected` and `plan.publication.member-landed` through the existing per-writer fact logs.
- Publish and validate the shared record schema for external producers.
- Exercise the WIT-breaking release order from [docs/release.md](../docs/release.md#three-release-shapes) across `augentic/emery` and `augentic/emery-adapters` as the in-house fixture. The first real release dogfoods the settled path; it does not gate RFC completion.

## Acceptance criteria

1. Draining a target seals its final accepted CID into exactly one local commit on `change/<plan>`, parented at the recorded initial revision, and records an idempotent fact. It may run before unrelated targets drain, but never before the final terminal projection fixes that target's complete entry set and exact manual or RFC-102 policy commit authorization exists. Completing one leaf creates no commit. The seal does not touch the operator checkout or forge.
2. Before changing archive state, `emery plan archive` derives members and repository locations from RFC-88 plan bindings, sealed commits from their facts, and branch and pull-request state from the forge provider.
3. Unchanged facts, plan, and forge state produce a byte-stable projection. A single-repository plan uses the same schema with one member and one sealed commit.
4. Publication order derives only from cross-target leaf `depends-on` edges, including RFC-88's projection of domain dependencies. Unrelated members carry no extra constraint; archive does not reread internal domains. A leaf-acyclic fixture whose target contraction contains a two-target cycle fails `publication-target-cycle` at plan validation and archive.
5. Archive verifies trailers, merged state, and landing order. Failures name every affected member. `--unverified` archives only after appending its fact.
6. External records validate against the published schema and project through the same read surface without acquiring plan lifecycle authority.
7. A WIT-breaking engine and adapter release fixture is represented and verified as a multi-member publication set.
8. `cargo make ci` passes with crate-level integration coverage for project sealing, one and many members, missing/open/closed/merged pull requests, ordering, trailer mismatch, provider failure, external records, and the unverified bypass.

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
