# RFC-95 cut readiness

> Status: Editing brief for [RFC-95](rfc-95-publication-sets.md). Not a new RFC. Fold the decided items into RFC-95 (and the RFC-88 wording patch), then delete this file.
>
> Verdict: **direction-ready, not freeze-ready.** D1–D11 and the eight acceptance criteria are a complete product policy. RFC-88 is implemented and this is the next critical-path delivery gap. A faithful first PR would stall on where the operator pushes from, how a CID becomes a Git commit, and a forge-read capability that does not yet exist.

## How to use this

Each finding is one RFC-95 hole plus a recommended edit. Accept, rewrite, or reject the recommendation in RFC-95 itself. Do not implement against this file.

Suggested fold order: F1 and F2 first — they make the seal implementable. Then F3 (forge reads). Then F4 and F5 (projection and journal wire). Then F6 and F7 (orchestration and authorization). Then F8 (RFC-88 wording) and F9 (progressive-authoring fence).

## Already decided

Do not reopen these in the fold:

- Members are distinct adapter-bearing targets referenced by terminal `slices[].target`. Unused, read-only, and internal-domain-only targets are not members. A single-repository plan is a one-member set.
- One seal per target after all of its entries merge. Completing one leaf creates no commit. Archive remains a whole-set gate.
- Emery never pushes, opens, merges, or reverts. `--unverified` journals without greening the projection.
- Order is contracted cross-target `depends-on`. `publication-target-cycle` already exists in RFC-88 plan validation; archive reuses that kernel.
- Records are derived. No publication artifact, registry, or member-repo state.
- RFC-96, RFC-100, RFC-102, and [RM-17](roadmap.md#rm-17-forge-publication-providers) are not prerequisites. The WIT-breaking engine/adapters fixture (AC7) dogfoods the path; it does not gate RFC completion.

## Recommended first cut

Staff one internal cut on the implemented RFC-88 substrate, not a second RFC and not a wait for RFC-96.

- Idempotent project seal after a target's last merged entry, writing one local Git commit and `plan.publication.project-sealed`.
- Host-owned Git object store plus a documented operator push path (F1, F2).
- New read-only host forge capability with shipped GitHub `find-pull-request` / `read-pull-request` (F3).
- Typed archive projection, `publication-unverified`, and `--unverified` (F4, F5, F6).
- Shared contraction kernel at plan validation, seal, and archive.

Do not implement overlapping survey/refine/build, forge writes, atomic cross-repository submission, or parallel member preparation.

---

## F1 — The operator has no local act that pushes

**Where:** Intent; Flow step 3; D5; D11; AC1; [RM-17](roadmap.md#rm-17-forge-publication-providers).

**Finding.** The seal writes a commit in a host-owned Git object store and must not touch the operator checkout. RM-17 (operator-triggered branch transport) starts only after RFC-95 lands and manual handoff is a measured bottleneck. The RFC never names a store path, ref layout, bundle, or command the operator can push from. “The operator still owns every forge write” has no local gesture.

**Recommend.** Name the host-owned store and the operator act:

- One bare Git repository per bound target under the host store (deployment layout, e.g. `$EMERY_HOME/git/…`; not the checkout, not a product path in the change home).
- The seal updates `refs/heads/change/<plan>` in that repository.
- On seal, Emery prints the git-dir, branch, and commit id. Operator guidance is `git --git-dir <store> push <origin> change/<plan>`.
- No working tree, no checkout write, no remote create. RM-17 later automates that push; it does not invent the store.

---

## F2 — CID → Git commit encoding is unspecified

**Where:** D11; AC1. Substrate: RFC-87 snapshot store (`sha256:` tree manifests in blobstore); no Git object library in the workspace.

**Finding.** Accepted CIDs are not Git trees. D11 requires a deterministic commit from recorded initial Git revision + final accepted CID, with author, message, and timestamp derived from the plan and the covering closed-plan epoch, and “the same parent and final tree therefore produce the same commit id.” Tree encoding, exec bits, empty directories, object-store identity, and the exact author/message/timestamp mapping are all unstated. An implementer cannot keep commit ids stable across machines without freezing those.

**Recommend.** Add a short encoding contract to D11 (or a D12):

- Build the Git tree from the accepted snapshot: blobs by file content, trees with Git’s sorted-name encoding, executable bits from the `ExecBits` seam, `.git` and nested change homes already excluded by RFC-88.
- Parent is the locator’s recorded Git revision (RFC-88 D5).
- Author and committer are a closed identity derived from the covering `plan.execute.started` writer; timestamp is that event’s timestamp, not wall-clock at seal.
- Commit message is a closed template carrying the plan name and final CID.
- Git objects live in F1’s bare repository. Snapshot objects stay in the RFC-87 store; the seal does not alias one digest scheme to the other.

---

## F3 — There is no forge provider to extend

**Where:** Preamble “extends its forge provider”; D10; IR “Extend the forge provider”; AC2, AC8.

**Finding.** RFC-88 has ingest/origins (clone and HTTPS fetch), not a GitHub pull-request reader. No `forge` type, WIT, or crate exists. D10’s `find-pull-request` / `read-pull-request` are new infrastructure. Lookup key, multiple matches, and how landing order is observed are unspecified. RM-17 is write-side follow-on and must stay out.

**Recommend.** This RFC *creates* the read-only host forge capability; it does not extend an RFC-88 type. Keep it host-side (launcher / `emery:…` import on the origins/ingest shape), not an adapter-axis WIT.

- `find-pull-request(repository, branch)` where `branch` is `change/<plan>`. Exactly one open-or-merged pull request succeeds; zero is `unpublished`; several fail closed.
- `read-pull-request` returns URL, body, `publication` (`unpublished | open | merged | closed`), and `merged-at` when merged.
- Trailer check is on the body (`Emery-Change: <plan>`). A forge label may exist; it is not the lookup key.
- Landing order is `merged-at` compared along D4’s contracted partial order.
- No `gh` subprocess. No forge writes. Preamble “extends its forge provider” becomes “adds a read-only host forge capability.”

---

## F4 — The projection example is not a schema

**Where:** Worked example JSON; D7; D8; AC3, AC6; IR “Publish and validate the shared record schema.”

**Finding.** `verification` is prose (`pending — web-frontend unmerged`), not a closed verdict. Unrelated members are unordered, yet the example assigns `order: 1` / `2`. D7 requires byte-stable output. D8’s external producers cannot validate against an illustration. Engine load gates are typed serde plus schemars goldens, not a second schema language.

**Recommend.** Close the wire in D7 and label the JSON as shape, not schema:

- `publication`: `unpublished | open | merged | closed` (already stated).
- `verification`: `verified | pending | unverified`, plus a stable list of failing members and reasons. No free-text verdict.
- `order`: present only when D4 assigns a rank in the contracted DAG. Unrelated members omit it (or use `null`); do not invent a total order. Ranks among comparable members are a stable topological numbering.
- Publish the schema as a schemars golden from the Rust wire type, same path as `crates/project/answers/`. External records validate against that type.

---

## F5 — Journal facts have no payload or emission moment

**Where:** D2; D11; IR `plan.publication.projected` and `plan.publication.member-landed`; AC5.

**Finding.** D2 says records are derived and must not be stored as a second authority. The IR still requires two journal kinds whose payloads and writers are unnamed. `project-sealed` is described; `projected` and `member-landed` are not. An implementer could persist the set into the fact log and contradict D2.

**Recommend.**

- `plan.publication.project-sealed` — execute, once per target, idempotent. Payload: project, parent revision, final CID, commit id, branch (already in D11).
- `plan.publication.projected` — archive, before mutation. Payload: canonical projection digest and `verification` verdict. The fact is an observation snapshot; it is not member authority.
- `plan.publication.member-landed` — archive, one fact per member whose pull request is merged. Payload: project, pull-request URL, merge commit, `merged-at`. Forge state remains authoritative under D2; the fact records what archive observed.
- `plan.publication.unverified-archive` — already specified on `--unverified`.

---

## F6 — Seal timing and archive flags are implied, not stated

**Where:** D1 incremental seal; Flow step 2; D5; current `plan archive --force` (outstanding work).

**Finding.** Incremental seal after a target’s last merge is a side effect of `plan execute`, but no sentence says there is no new verb. Today’s `--force` archives over non-`done` entries. `--unverified` is a different gate. Without an explicit split, `--force` will be used to skip publication verification, or a `plan seal` verb will appear.

**Recommend.** State both in D1/D5/D11:

- Seal runs inside `plan execute` when every in-scope entry for that target is named by a committed target-wave chain, no postflight failure remains unacknowledged, and F7’s authorization holds. There is no `plan seal` / `plan publish` verb (D7 already refuses a publication subcommand).
- `--unverified` skips only the D5 publication checks.
- `--force` skips only the outstanding-work ladder check. It does not skip publication verification. The two flags compose.

---

## F7 — RFC-102 in AC1 reads as a dependency

**Where:** D1; AC1; [RFC-102](rfc-102-policy-gated-autonomy.md) parked; [platform.md](platform.md) parked programme.

**Finding.** “Exact manual or RFC-102 policy commit authorization” can be read as waiting on parked RFC-102. The intended manual path is the existing `plan.execute.started` closed-plan epoch. RFC-102 later extends the seal gate; it does not define the first cut’s authority.

**Recommend.** Replace with: the covering `plan.execute.started` epoch (plan digest plus per-leaf refinement digests, including every entry for that target) is the manual authorization. RFC-102, when reopened, may add a policy-gated alternative beside that epoch; it is not a prerequisite and grants no forge write.

---

## F8 — RFC-88 still says RFC-95 owns publication writes

**Where:** [RFC-88](rfc-88-detached-changes.md) D8 (“RFC-95 owns push, branch, pull-request, merge, and other publication writes”); RFC-88 AC11; this RFC D5; [platform.md](platform.md) “Forge writes remain operator-owned.”

**Finding.** RFC-88’s wording contradicts this RFC and the programme spine. An implementer reading RFC-88 first will add forge writes.

**Recommend.** Patch RFC-88 in the same edit:

```text
RFC-95 owns the local project seal and archive-time publication observation. Forge writes remain operator-owned.
```

Keep RFC-88 AC11’s “execution ends with one accepted CID per touched target and no commit or branch” until this RFC lands; after it lands, that sentence becomes “no forge write.”

---

## F9 — Progressive authoring is a future fence, not this cut

**Where:** D1 last paragraph; RFC-88 complete-tree publication; parked [RFC-99](rfc-99-streaming-execution.md).

**Finding.** “Survey, refine, and build may overlap” is a constraint on a later streaming cut. RFC-88 already closes the plan at execute. Implementing overlap here would pre-build RFC-99.

**Recommend.** Keep the seal-boundary rules (final terminal projection fixed, one seal after all entries, no leaf-completion commit). Reword the overlap sentence as a fence: progressive authoring, if RFC-99 later permits it, still cannot seal until that target’s complete entry set and covering execute epoch exist. This cut does not overlap survey, refine, or build with seal.

---

## RFC-95 section checklist

Fold into the RFC, then delete this file.

- [ ] Preamble: create a read-only host forge capability (not “extend RFC-88’s”); RFC-102 not a predecessor.
- [ ] Flow: execute seals; operator pushes from the documented git-dir; archive observes.
- [ ] Worked example JSON labelled as shape; closed `verification`; `order` omitted when unordered.
- [ ] D1: no new verb; `--force` vs `--unverified`; RFC-99 fence only.
- [ ] D5: forge lookup and `merged-at` order; flag split.
- [ ] D7 / D8: closed projection wire; schemars golden.
- [ ] D10: new host capability, lookup contract, no RM-17.
- [ ] D11 / new encoding paragraph: F1 store, F2 Git encoding, F7 epoch authorization, F6 execute-side-effect.
- [ ] Journal payloads for `project-sealed`, `projected`, `member-landed`.
- [ ] Operator guidance: git-dir push line.
- [ ] Implementation requirements and AC1/AC2/AC3 aligned with the above.
- [ ] RFC-88 D8 / AC11 wording patch (F8).
