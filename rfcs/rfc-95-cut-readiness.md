# RFC-95 cut readiness

> Status: Editing brief for [RFC-95](rfc-95-publication-sets.md). Not a new RFC. Product policy is folded. Delete this file when the first implementation PR lands or the remaining stalls are accepted as follow-on.
>
> Verdict: **direction-ready, not freeze-ready.** D1–D11 and the eight acceptance criteria are a complete product policy. A faithful first PR still stalls on two implementation gaps: there is no Git object library, and no forge-read capability.

Folded and removed: operator worktree surface (F1); RFC-102 is not a prerequisite (F7); RFC-88 / RFC-87 wording (F8); RFC-99 fence (F9); execute-side-effect and no new verb (F6); `plan.publication.materialized` (F5); schemars golden contract (F4). Their remaining sentences are now in RFC-95 D1, D5, D7, D8, D10, D11, and AC6.

---

## F2 — No Git object library for the D11 tree encoding

**Where:** D11; AC1. Substrate: RFC-87 snapshot store (`sha256:` tree manifests in blobstore).

**Finding.** D11 states the tree contract (blobs, Git sorted-name trees, `ExecBits`, omitted empty directories, parent = recorded revision, no digest aliasing). The workspace has no Git object library. An implementer cannot fill a worktree from a CID without choosing one.

**Recommend.** Add a host-side Git encoding path (launcher or a small host crate) that implements D11 against the worktree's repository. Do not put Git in the engine guest. Do not alias RFC-87 blobstore objects to Git objects.

---

## F3 — The D10 forge-read capability does not exist

**Where:** D10; IR; AC2, AC8.

**Finding.** D10 now specifies `find-pull-request` / `read-pull-request`, the zero/one/several lookup, trailer-as-key, and `merged-at` order. RFC-88 has ingest/origins only. No `forge` type, WIT, or crate exists.

**Recommend.** Create the read-only host capability as specified in D10. RM-17 stays out.
