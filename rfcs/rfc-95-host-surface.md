# RFC-95 host surface

> Status: Active host-surface companion to [RFC-95](rfc-95-publication-sets.md) in the [Services Delivery Programme](platform.md). Not a new RFC. Implementation starts at cut 0 of the sequencing table.
>
> Product policy (worktree, markers, archive observation, facts, schema) lives in [RFC-95](rfc-95-publication-sets.md). This document decides how the host talks to Git and the forge. This revision supersedes the earlier git-aware-blobstore direction: the VCS seam below retires both bespoke fetch packages as the foundation RFC-95 lands on rather than as deferred cleanup. The pre-implementation review's open items are closed as decided text below — the WIT field lists and error taxonomies, the `emery:engine` package, the dedicated staging root, and the forge token order; git history holds the full findings.
>
> Decision: **one VCS seam** — a single `emery:vcs` WIT package with three interfaces (`trees`, `worktree`, `forge`) where credential and transport policy travel as typed data on each call, never as package identity. Publication export is one host call, not a mount. No `emery:publication` / `emery:forge`, no git-aware blobstore, no `augentic/omnia-backends` dependency. `emery:adapter` and `emery:exec-mode` stay.

## Why

Omnia Law 2's real content is threefold: no workflow nouns in WIT, no policy split across package identities, and the guest must not know which Git, credential, or HTTP identity satisfies a call. `emery:origins` / `emery:ingest` violated the second — one fetch domain split into two packages by credential policy alone. The previous revision of this document tried to satisfy Law 2 by treating Git as a `wasi:blobstore`; review (F13) showed the fit failure: a blob API carries no trees, revisions, or freshness channel, `repo@revision` container naming smuggles product semantics into a string convention, the single bound blobstore would need a launcher multiplexer, and RFC-88 D9 bounds would have to wrap a generic credentialed clone. The generic ecosystem also offers nothing to wait for — `wasi:blobstore` remains a Phase 1 WASI proposal and no `wasi-vcs` exists.

Every Git interaction Emery needs — RFC-88 bind, RFC-104 archaeology, RFC-95 publication export, archive observation — is one of four transformations between remote repository state and CID trees:

| Verb | Direction | Today |
| --- | --- | --- |
| resolve | mutable ref → exact revision | buried inside fetch (`ls-remote`) |
| fetch | locator → deployment-local tree (→ CID) | `emery:origins` + `emery:ingest` |
| export | CID → Git worktree on a branch at a parent revision | unbuilt |
| observe | (repository, branch) → pull-request state | unbuilt |

The trust boundary is constant across all four: Git execution, credential resolution, and network stay host-side; the guest deals only in CIDs, typed results, and facts. A single domain interface — version-controlled repository access — satisfies Law 2 honestly: backends are configuration (host `git` today; a Rust Git library later if wanted), forges are configuration (GitHub v1), and credentials are per-call typed data. It is the shape one would propose upstream as `wasi:vcs` if the ecosystem ever standardizes one; Jujutsu's `Backend` trait demonstrates the same abstraction in production.

## The VCS seam

One WIT package, owned by one host capability crate (`crates/wasi-vcs`), replacing `crates/wasi-origins` and `crates/wasi-ingest`. Decided shape — the records and error taxonomies are derived from the two retired packages, the `project::binding` wire types (D9 `Policy`, the `https-redirect-limit` / `https-body-limit` failures), and RFC-95 D10/D11; review confirmed no missing semantics:

```wit
package emery:vcs@0.1.0;

interface trees {
  /// Credential policy travels per call, never as package identity.
  enum credentials {
    /// Hardened credential-free fetch (RFC-88 bind): no hooks, no
    /// submodules, no LFS, no prompts.
    none,
    /// The operator's ambient host credentials (RFC-104 archaeology).
    ambient,
  }

  /// Transport-level bounds for one fetch, handed down from the
  /// engine's D9 policy (`https_body`, `https_redirects`, `time_ms`).
  /// Wave-level metering (bindings, trees, inspected bytes) stays
  /// engine-side.
  record limits { max-bytes: u64, max-redirects: u32, time-ms: u64 }

  /// One staged tree beneath the staging root.
  record fetched {
    /// Deployment-local root of the staged tree. The engine snapshots
    /// it (CID minting stays in the workspace kernel) and discards it.
    root: string,
    /// The commit the fetch reports, when the locator is Git. The
    /// moved-branch comparison against a recorded prior SHA is an
    /// engine check.
    revision: option<string>,
  }

  variant error {
    /// The locator is not a fetchable origin.
    invalid-request(string),
    /// The origin refused or could not be reached.
    access(string),
    /// A transport-level limit was exhausted (bytes, redirects, time).
    limit(string),
    internal(string),
  }

  fetch: async func(locator: string, credentials: credentials, limits: limits)
    -> result<fetched, error>;

  /// Discard a staged tree by its root. Best-effort and idempotent.
  discard: async func(root: string) -> result<_, error>;
}

interface worktree {
  /// One RFC-95 D11 materialize. `plan` and `target` name the
  /// `$EMERY_HOME/publication/<plan>/<target>/` slot; `branch` is
  /// `change/<plan>`; `allow-in-place` carries the engine's
  /// single-member in-place decision.
  record request {
    repository: string,
    parent-revision: string,
    branch: string,
    cid: string,
    plan: string,
    target: string,
    allow-in-place: bool,
  }

  /// Idempotency outcome per the D11 state table.
  enum state { created, matched, rematerialized }

  /// The D11 refusal rows. The engine maps `dirty` to
  /// `publication-worktree-dirty` and the closed provisioning reasons
  /// to `publication-provision-failed`.
  variant export-error {
    /// Operator has uncommitted edits on the publication worktree.
    dirty,
    branch-diverged,
    branch-checked-out-elsewhere,
    destination-conflict,
    parent-unavailable,
    clone-failed,
    invalid-request(string),
    internal(string),
  }

  /// Provision, materialize, and stage per RFC-95 D11; returns the
  /// host worktree path (node-local observation for the
  /// `plan.publication.materialized` fact) and the outcome.
  export: async func(req: request) -> result<tuple<string, state>, export-error>;
}

interface forge {
  /// Publication state as the forge reports it. `unpublished` is the
  /// engine's zero-match projection, not a forge state.
  enum pr-state { open, merged, closed }

  /// One pull request, carrying everything RFC-95 D10's read needs.
  record pull-request {
    url: string,
    body: string,
    state: pr-state,
    /// The observed base branch (recorded, not gated — D10).
    base: string,
    /// RFC 3339; present only when merged.
    merged-at: option<string>,
    /// The forge's merge commit; present only when merged.
    merge-commit: option<string>,
  }

  /// Authentication and transport failures are distinct outcomes;
  /// neither is `publication-unverified` (D10).
  variant error {
    invalid-request(string),
    auth(string),
    transport(string),
    internal(string),
  }

  /// Every open, merged, and closed pull request for
  /// `(repository, branch)`, pagination followed to exhaustion. The
  /// zero / one / several rule, trailer and covering-digest matching,
  /// and `merged-at` ordering are engine checks over these results.
  find: async func(repository: string, branch: string)
    -> result<list<pull-request>, error>;
}
```

### `trees` — locator fetch

Replaces `emery:origins` and `emery:ingest` with one operation. `credentials: ambient` is archaeology (RFC-104 system survey — host `git` with the operator's ambient credentials, discard after snapshot); `credentials: none` is delivery bind (RFC-88 — hardened credential-free checkout: no hooks, no submodules, no LFS, no prompts). The host stages the tree (exact-SHA Git checkout, or an HTTPS document as a one-file tree) and reports the resolved revision; it does not mint CIDs.

Staging lands under a dedicated staging root — `$EMERY_HOME/staging/`, mounted to the guest as its own preopen — not the workspaces root the two retired packages used. Fetch staging is never confused with RFC-87 workspace trees, and its lifecycle (discard after snapshot; sweep abandoned roots by age) stays independent of workspace GC policy.

Policy concentrates engine-side, making this seam dumber than the two packages it replaces:

- **CID minting stays in the workspace kernel.** The engine snapshots the staged tree for both archaeology and bind — one digest authority, uniform for both callers.
- **Recorded-CID skip is an engine pre-check.** `Session::ingest` already consults the store and intern cache before origin I/O; no seam channel is needed.
- **The moved-branch warning is an engine comparison** of the returned `revision` against the recorded prior SHA; no seam channel is needed.
- **RFC-88 D9 bounds stay Emery product policy.** The engine meters bindings, trees, and inspected bytes itself and hands only transport-level `limits` (bytes, redirects, time) to the backend per call.

### `worktree` — publication export

One host call performs the entire RFC-95 D11 materialize. The host owns both halves already: the launcher holds the snapshot object store at `$EMERY_HOME/snapshots/`, and the workspace kernel (`project::workspace`) is wasm-clean library code, so the host runs `Store::materialize` in-process onto a checkout it provisions itself:

1. Provision per RFC-95 D11 placement: when `allow-in-place` is set and the product repository is clean at the recorded parent, materialize on `change/<plan>` in that repository; otherwise reuse or create `$EMERY_HOME/publication/<plan>/<target>/`. Never the current branch, an RFC-87 workspace, or the change home.
2. Create or reuse branch `change/<plan>` at `parent-revision`, apply the D11 worktree state table (match / dirty / committed / branch-at-another-commit / checked-out-elsewhere / stale worktree metadata / parent absent → fetch once), and refuse with a typed `export-error` where the table says refuse.
3. Materialize the CID with real permission bits — no `emery:exec-mode` widening, because the host is not behind `wasi:filesystem`.
4. Stage the index (`git add -A`) so `git diff` and `git diff --cached` read against the parent. No commit — the operator authors every commit.
5. Return the host worktree path (node-local observation for the `plan.publication.materialized` fact) and the idempotency `state`.

The guest calls `export` from the execute loop whenever a target's last in-scope merge lands, and again on re-entry to reconcile pending materializations — no publication preopen, no mid-run mount, no post-exit index-staging hook, no launcher knowledge of workflow state. The mount-timing impossibility the review found (F2), the exec-bits and staging-hook questions (F11), and the unrecoverable drained plan (F17) all dissolve rather than needing answers.

### `forge` — pull-request observation

`emery plan archive` calls `find` / read operations; the host backend speaks GitHub REST (v1 is GitHub-only per RFC-95 D10). The token comes from backend configuration, resolved in a fixed order: `omnia:identity` first — confirmed present at the pinned Omnia version as `omnia:identity@0.1.0`, whose `credentials` interface resolves a named identity to a scoped OAuth2 access token — when the deployment configures an identity backend; else `GITHUB_TOKEN` from the launcher environment; else `gh auth token` when the CLI is present; else unauthenticated, sufficient for public repositories. The token lives only in the backend: never logged, never in a fact, projection, or error message, and it never crosses into the guest. Cut 3 does not block on `omnia:identity` — the environment path is sufficient for v1, and the sourcing order is backend configuration, invisible to the engine. The backend follows pagination to exhaustion and returns every open, merged, and closed pull request for `(repository, branch)`, including the base branch and merge commit; the zero / one / several rule, trailer and covering-digest matching, and `merged-at` ordering are engine checks over the typed results (RFC-95 D10). No outgoing HTTP in the guest, no `gh` subprocess, no forge writes — RM-17 may later add write operations to this same interface behind an explicit grant without structural change.

## Prerequisite: split the published WIT

The `workflow` world currently lives inside the published `emery:adapter` package, so any host-seam change drags the adapter release train (review F14). Before the seam lands: `emery:adapter` shrinks to the adapter contract alone (the `source` / `target` interfaces and worlds adapters vendor), and the engine `workflow` world moves to its own package, `emery:engine@0.1.0` — the name matches the engine-guest vocabulary already in use, no collision exists, and pre-1.0 the version starts fresh at `0.1.0`. After the split, every cut below is a host-only release shape per [docs/release.md](../docs/release.md); adapters are untouched — their operations never see fetch, export, or forge. The one-time adapter re-vendor is legal under the pre-1.0 hard-cut posture.

## Sequencing

Each cut is coherent and shippable; none patches a seam already known to be wrong:

| Cut | Ships | Depends on |
| --- | --- | --- |
| **0 — WIT split** | `emery:adapter` = adapter contract only; engine world in its own package; adapter re-vendor | — |
| **1 — `trees`** | Port RFC-88 bind and RFC-104 archaeology onto `trees.fetch`; delete `emery:origins` / `emery:ingest` and both host crates. Pure re-seaming of implemented behavior; orchestrations unchanged | Cut 0 |
| **2 — `worktree` + RFC-95 cut A** | Export host call, D11 placement and state table, `plan.publication.materialized`, drain condition, stop reasons, status milestone, operator Git docs | Cut 1 (crate layout), RFC-87 store |
| **3 — `forge` + RFC-95 cut B** | GitHub reads, projector golden, archive gate, `--unverified`, `projected` / `member-landed` facts | Cut 2 facts |

Snapshot store, private workspaces, accepted-CID identity, and RFC-96 are unchanged (RFC-87 / RFC-88 / RFC-90 / RFC-96). Accepted CIDs are never aliased to Git SHAs; a Git-object-database `Objects` backend remains a noted future optimization behind the existing seam, never an identity change.

## Testing seams

- `trees` / `worktree` backends: `crates/launcher/tests/` over temp Git repositories (the D11 state table, both credential modes, HTTPS one-file trees).
- `forge` backend: a local HTTP fixture speaking the D10 subset; engine checks covered in `crates/change/tests/` against the native provider's scripted forge double.
- The native provider implements the same three capability traits in-process (scripted doubles beside today's `Ingest` / `Origins` fakes).
- The operator-invoked wasm example gains one publication scenario, per the existing gate model.

## Rejected here

- New `emery:publication` or `emery:forge` WIT — workflow nouns in WIT.
- A git-aware `wasi:blobstore` backend in `augentic/omnia-backends` — blob API on a tree domain; policy in container names; multiplexer; third repository on the critical path (review F13).
- Splitting fetch by credential policy into separate packages — the inversion this seam exists to fix.
- Guest-side fetch over `wasi:http` (+ identity) — cannot carry the Git protocol or SSH; moves ambient-credential resolution into or behind the guest.
- The publication worktree as a guest mount with host index staging after exit — mounts are fixed at process start; the choreography is impossible as reviewed (F2) and needs `emery:exec-mode` widening the export call does not.
- Guest-visible `wasi:exec`, or Git porcelain in the engine guest — Git stays host-side.
- Encoding worktrees or locators as blobstore container names — rebuilds porcelain on the wrong API.
- Aliasing RFC-87 snapshot objects or accepted CIDs to Git objects / SHAs.
- Waiting for an upstream `wasi:vcs` — nothing exists; `wasi:blobstore` is Phase 1. `emery:vcs` is shaped (and named) to converge with such a standard if one appears.
- Staging `trees.fetch` under the workspaces root (today's layout) — fetch staging is not an RFC-87 workspace tree; a shared root confuses the two and couples their GC policies. The dedicated `$EMERY_HOME/staging/` root keeps them independent.
