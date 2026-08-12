# RFC-95: Publication Sets

> Status: Active product follow-on to RFC-88 in the [Services Delivery Programme](platform.md)
>
> Owns: one local project seal per publication member; the branch, pull-request marker, and landing-order conventions that bind those members into one change; archive-time verification; and the shared machine-readable publication projection.
>
> Builds on completed [RFC-88](rfc-88-detached-changes.md), which records the targets, persists recursive conflict-domain decomposition, projects its buildable leaves into the plan, and supplies the forge provider extended here with publication reads. [RFC-100](rfc-100-distributed-execution.md) coordinates execution across nodes before publication. RFC-81 in `augentic/remedium` is the first external producer.
>
> Defers: pushing branches, opening or merging pull requests, atomic cross-repository submission, automated rollback, cross-checkout CI, and parallel member preparation.

## Intent

Make the change, rather than the repository, the unit of publication.

Suppose `checkout-v2` changes a payment API and the web frontend that consumes it. Emery should finish execution with one committable result for each repository. The operator then pushes two branches and completes two pull requests. Both pull requests carry the same change identity, and their landing order follows the dependency already recorded in the plan.

That collection is the **publication set**. Emery derives it from the plan, seals its local commits, and verifies it at archive. The operator still owns every forge write.

## Flow and terms

1. RFC-88 projects terminal conflict domains into plan entries identifying the participating targets and their leaf `depends-on` relationships.
2. When all entries for one target are merged, Emery seals its final accepted CID into one local commit on `change/<plan>`.
3. The operator pushes that branch, opens a pull request carrying `Emery-Change: <plan>`, and lands the members in dependency order.
4. `emery plan archive` reads the forge, reconstructs the set, and verifies every member before archiving.

A **publication member** is a distinct adapter-bearing target used by at least one slice. A **project seal** turns that target's final accepted CID into its one local Git commit. A **publication set** binds the change name, members, sealed commits, branches, pull requests, and required landing order.

RFC-88's target-wave commit is the accepted-CID transition; its initial one-leaf form is `emery slice merge`, and RFC-96 later extends membership without changing the fact shape. In this RFC, **publish** means creating the remote branch and pull request, while **land** means merging that pull request on the forge. A publication set is also distinct from an RFC-87 code patch, which relates one base CID to one result CID.

## Worked example

The relevant part of `checkout-v2` is:

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

The two distinct target bindings make two publication members. The cross-target dependency means `payments-api` must land before `web-frontend`. If both slices targeted `payments-api`, the set would still have only one member and one sealed commit.

Each repository gets its own branch named `change/checkout-v2`. Its pull-request body contains:

```text
Emery-Change: checkout-v2
```

Before archive, Emery projects the plan, seal facts, and forge state into one record. The standard command envelope is omitted here:

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

Here archive stops because the frontend pull request is still open. `publication` is a closed enum: `unpublished | open | merged | closed`.

## Decisions

### D1 — The plan is the publication set

The plan name is the change identity. Its publication members are the distinct adapter-bearing targets referenced by the terminal `slices[].target` projection. A target listed in `plan.yaml` but used only as read-only input, present only on an internal conflict domain, or not used at all is not a member.

There is no second lifecycle object or authored publication artifact. A single-repository plan is simply a one-member publication set.

Publication is incremental by member, not gated on draining the whole plan. Once the final terminal projection is fixed and every entry for one target is merged, that target may seal, publish, and land while entries for other targets continue, subject to D4's dependency order. Archive remains a whole-set gate. Progressive authoring does not weaken the seal boundary: survey, refine, and build may overlap, but no result advances an accepted CID until the final projection has exact manual or RFC-102 policy commit authorization, and forge publication remains operator-owned. Completing one leaf never creates a commit; each target receives exactly one seal after all of its entries merge.

### D2 — Plan-backed publication records are derived

Each fact has one authoritative home:

- member identity comes from `slices[].target`;
- repository location and initial revision come from `plan.yaml.targets`;
- the sealed commit comes from D11's fact;
- remote branch, pull-request reference, and publication status come from the forge.

Emery joins those values when it needs the projection. It does not commit publication-set state into member repositories or restore a registry bridge.

### D3 — Branches and pull requests carry a shared marker

Every member uses `change/<plan>` as its branch name and carries `Emery-Change: <plan>` in the pull-request body. For `checkout-v2`, both repositories therefore use `change/checkout-v2` and `Emery-Change: checkout-v2`.

Those markers let an operator reconstruct the set from the forge without the original Emery change home. A forge label may mirror the trailer, but the trailer is authoritative.

### D4 — Publication order comes from `depends-on`

Cross-target leaf `depends-on` edges form the publication order. RFC-88 has already compiled dependencies between internal domains into edges between their entry and exit leaves; publication does not interpret the decomposition again. Same-target edges disappear when the leaf graph is projected onto targets, while unrelated targets remain unordered.

In the worked example, `adopt-payment-api` depends on `expose-payment-api`, so the payment API must land first. If a documentation target had no dependency path to either member, it could land at any time. Emery does not invent a second ordering field for the operator to maintain.

An acyclic leaf graph is not sufficient: contraction can expose `target-a → target-b` and `target-b → target-a` through different leaves without creating a leaf cycle. RFC-88 plan validation therefore contracts the complete leaf graph to distinct targets and rejects any strongly connected component or self-loop as `publication-target-cycle`. Archive repeats the same pure validation before reading forge state. Only an acyclic contracted graph is a publication partial order.

### D5 — Emery observes publication but does not perform it

Emery never pushes a branch, opens a pull request, merges one, or reverts one. Those remain operator-owned actions under the [CLI contract](../docs/standards/cli-contract.md).

`emery plan archive` replaces prose confirmation with observation. It reconstructs the members, reads their pull requests, and verifies that:

1. every member has a pull request with the correct `Emery-Change` trailer;
2. every pull request is merged;
3. dependency-ordered members landed in the required order.

On success, archive proceeds. On failure, it returns `publication-unverified` on exit 1 and names every failing member before changing archive state.

The explicit `--unverified` escape hatch lets the operator archive over a failed check. That action appends `plan.publication.unverified-archive`; it does not turn a red projection green.

### D6 — The guarantee is coordinated convergence, not atomicity

GitHub cannot atomically merge pull requests across repositories. Emery does not imitate Gerrit's `submitWholeTopic` with merge-all-or-revert behavior.

Contract changes instead use expand/contract steps expressed through ordered members. An out-of-order landing becomes a verification finding, not an automated rollback. Deployment compatibility remains the responsibility of the operator and target adapters.

### D7 — Archive produces the typed publication projection

Before mutation, `emery plan archive` projects members, sealed commit ids, branches, pull requests, publication states, derived order, and the verification verdict. Unchanged plan data, facts, and forge state produce byte-stable output.

The projector is an internal read surface reused by archive and external-record validation. This RFC adds no publication subcommand.

### D8 — External producers use the same record shape

A non-plan system may emit the same publication-set record without acquiring Emery lifecycle authority. For example, one Remedium alert remediation spanning three repositories can emit three members with the same trailer and ordering shape.

Plan-backed records remain derived under D2. External records are producer-authored inputs validated against the shared schema; they do not create or mutate an Emery plan.

### D9 — Publication checks are serial and workspace-free

Archive reads the resolved plan bindings and forge state serially. It does not prepare, capture, or inspect a product workspace.

That keeps verification deterministic and avoids coupling a read-only publication check to checkout state. Parallel member preparation remains RFC-96 work.

### D10 — The forge provider owns publication reads

RFC-88's host forge provider gains `find-pull-request` and `read-pull-request`. The shipped GitHub binding implements both in this RFC.

There is no temporary `gh` subprocess path and no dependency on RM-17. The provider gains no forge write capability.

### D11 — One project seal creates each committable result

When every entry for a target is named by an RFC-88 committed target-wave chain and no postflight failure remains unacknowledged, Emery creates one local Git commit:

```text
recorded initial Git revision + final accepted CID
    → one commit on change/<plan>
    → plan.publication.project-sealed
```

For example, three slices may enter one atomic wave or several serialized waves for `payments-api`; only the final accepted CID becomes repository history. Workers do not commit, and intermediate candidate or accepted CIDs do not receive branches.

The seal writes the commit directly from the immutable tree in the host-owned Git object store. It does not materialize a workspace or touch the operator's checkout. RFC-88 already includes code and the merged `.emery/specs/` baseline in the accepted tree, so the seal assembles no new content.

Commit author, message, and timestamp derive from the plan and the closed-plan commit-authorization epoch covering its final wave. The same parent and final tree therefore produce the same commit id. The idempotent seal records the project, parent revision, final CID, commit id, and branch in `plan.publication.project-sealed`.

The seal prepares publication; it does not create a remote ref, pull request, merge, or revert.

## Implementation requirements

- Implement the idempotent project seal over the final accepted CID, local `change/<plan>` ref, and `plan.publication.project-sealed` fact.
- Document the `change/<plan>` branch and `Emery-Change` trailer conventions in operator guidance.
- Extend the forge provider and shipped GitHub binding with `find-pull-request` and `read-pull-request`.
- Implement one typed projector over terminal plan bindings, seal facts, and forge state. Derive its partial order only from projected leaf `depends-on`; do not create a second decomposition reader in publication.
- Share one target-contraction and cycle-validation kernel between RFC-88 plan validation and archive. Reject `publication-target-cycle` before sealing or forge reads.
- Render the projection before archive mutation and gate unverified publication with `publication-unverified`; journal the `--unverified` bypass.
- Append `plan.publication.projected` and `plan.publication.member-landed` through the existing per-writer fact logs.
- Publish and validate the shared record schema for external producers.
- Exercise the WIT-breaking release order from [docs/release.md](../docs/release.md#three-release-shapes) across `augentic/emery` and `augentic/emery-adapters` as the in-house fixture. The first real release dogfoods the settled path but does not gate RFC completion.

## Acceptance criteria

1. Draining a target seals its final accepted CID into exactly one local commit whose parent is the recorded initial revision. It may do so before unrelated targets drain, but never before the final terminal projection fixes that target's complete entry set and exact manual or RFC-102 policy commit authorization exists; individual leaf completion creates no commit. The seal updates local `change/<plan>` and records an idempotent fact without touching the operator checkout or forge.
2. `emery plan archive` derives every member and repository location from RFC-88 plan bindings, every sealed commit from its fact, and branch and pull-request state through the forge provider before changing archive state.
3. The projection is byte-stable for unchanged facts, plan, and forge state. A single-repository plan produces the same schema with one member and one sealed commit.
4. Publication order derives only from cross-target leaf `depends-on` edges, including RFC-88's deterministic projection of domain dependencies. Unrelated members carry no invented constraint, and archive does not reinterpret internal domains. A leaf-acyclic fixture whose target contraction contains a two-target cycle fails `publication-target-cycle` at plan validation and archive.
5. Archive verifies trailers, merged state, and landing order. Failures name every affected member; `--unverified` archives only after appending its fact.
6. External records validate against the published schema and project through the same read surface without acquiring plan lifecycle authority.
7. A WIT-breaking engine and adapter release fixture is represented and verified as a multi-member publication set.
8. `cargo make ci` passes with crate-level integration coverage for project sealing, one and many members, missing/open/closed/merged pull requests, ordering, trailer mismatch, provider failure, external records, and the unverified bypass.

## Rejected alternatives

- **Monorepo consolidation** — answers a different question and is unavailable when repository boundaries mirror organisation boundaries. Coordination must work over the fleet that exists.
- **Meta-repository or submodules** — pins member SHAs to solve an identity problem while coupling clone, CI, and review mechanics. The set needs a name and members, not a super-repository.
- **Multi-target slices** — breaks singular `slices[].target` and the synthesis kernel's per-target header derivation for no gain. A plan already spans targets across slices.
- **A separate publication artifact or registry** — creates a second member authority beside `plan.yaml`. RFC-88 deliberately removes the detached registry.
- **`submitWholeTopic` emulation** — requires Emery to merge and revert on the forge even though GitHub provides no atomic cross-repository operation. Ordered convergence is the honest guarantee.
- **An external system as record owner** — [Sourcegraph Batch Changes](https://sourcegraph.com/docs/batch-changes) has the right tracking shape but the wrong authority for plan-backed changes. External systems may emit D8 records; they do not own Emery workflow state.
- **Forge labels as the authoritative marker** — labels are useful views but are forge-specific and easy to retag. The pull-request trailer is portable and travels with the review record.
- **Cross-checkout speculative CI in archive** — publication verification reads settled forge state. RFC-96's local domain gates own pre-merge composition; RFC-100 only transports that model.
- **Assuming leaf acyclicity implies publication acyclicity** — contracting several leaves onto one target can create a target cycle absent from the leaf graph. The contracted graph is validated explicitly.
