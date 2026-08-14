# RFC-95 host surface

> Status: Design. Edit this file, then implement against it. Not a new RFC.
>
> Product policy (worktree, markers, archive observation, facts, schema) lives in [RFC-95](rfc-95-publication-sets.md). This document decides how the host talks to Git and the forge.
>
> Decision: **Omnia generic interfaces**, shipped as **one implementation** — git-aware blobstore and mounted publication worktree together. No new `emery:*` host packages.

## Why

Omnia Law 2: guests import a generic interface; which Git, credential, or HTTP identity satisfies it is backend configuration. Emery inverted that: `emery:origins` and `emery:ingest` are the same fetch domain, split only by credential policy, and RFC-95 D10/D11 previously would have added `emery:forge` / `emery:publication` the same way.

`emery:adapter` stays. It is the product contract, not a host capability. `emery:exec-mode` stays: `wasi:filesystem` has no permission bits.

## Layering

| Need | Interface the guest sees | Where Git / HTTP lives |
| --- | --- | --- |
| Survey / bind locator → tree | `wasi:blobstore` | New git-aware blobstore backend in [`augentic/omnia-backends`](https://github.com/augentic/omnia-backends). Container ≈ `repo@revision`; object name ≈ path. Replaces `emery:origins` / `emery:ingest`. |
| Snapshot objects (RFC-87) | `wasi:blobstore` | Today's `omnia-filesystem`. Optional git-backed objects later. Do not alias accepted CID ↔ Git SHA. |
| Publication worktree | `wasi:filesystem` preopen | Host provisions a real Git checkout (clone / `git worktree add` / in-place branch). Launcher mounts that path. Guest writes the accepted CID as files. Operator `cd`s the host path. |
| Forge PR reads | `wasi:http` + `omnia:identity` | REST. No `emery:forge`. Lookup contract (branch → one PR, trailer, `merged-at`) stays product policy in RFC-95 D10. |

`omnia-filesystem` is already a **blobstore** backend over a directory, not a `wasi:filesystem` backend. Preopens are wasmtime mounts. The git-aware backend is another `WasiBlobstoreCtx`, next to azure-blob / nats / filesystem.

## Publication worktree

RFC-95 D11's placement rules stay product policy. Implementation:

1. **Launcher / deployment** decides the host path (in-place one-member clean-at-parent; else `git worktree add -b change/<plan>` from an existing clone; else a first-time clone the operator owns, e.g. `$EMERY_HOME/publication/<target>/`). Creates the branch at the recorded parent. Never the current branch, a dirty tree, an RFC-87 workspace, or the change home.
2. **Mount** that path as a guest preopen for the execute invocation that materializes.
3. **Guest** reads the accepted CID from the snapshot blobstore and writes the tree onto the mount. It does not encode Git objects, run `git`, or see `wasi:exec`.
4. **Host** stages the index (`git add`) so `git diff` and `git diff --cached` match the CID. No commit.
5. **Fact** `plan.publication.materialized` records target, parent, CID, host worktree path, and branch.

Idempotency, dirty refusal, and no-rewind stay RFC-95 D11. Git porcelain (clone, worktree, index) never enters the engine guest.

## Locator fetch

Survey and bind open a blobstore container for the locator's exact revision and read paths. Clone, credentials, SSH, and pack unpack stay in the backend.

Credential policy is backend configuration, not a second WIT package:

- archaeology (today `emery:origins`): ambient credentials; discard after snapshot
- delivery bind (today `emery:ingest`): credential-free, RFC-88 D9 bounds, delivery CID

Once the guest talks blobstore, delete `emery:origins` and `emery:ingest`. Keep `emery:exec-mode`.

Non-git HTTPS locators remain one GET; the backend can satisfy those without a `.git`.

## Forge reads

`emery plan archive` issues outgoing HTTP for `find-pull-request` / `read-pull-request` semantics (RFC-95 D10). Identity supplies the token. Zero / one / several, trailer-as-key, and `merged-at` order are engine checks over the HTTP responses, not host functions.

No `gh` subprocess. No forge writes. RM-17 stays out.

## What this work ships together

Do not sequence "blobstore first, worktree later." One design, one implementation cut:

- `omnia-backends`: git-aware `WasiBlobstoreCtx`
- Emery launcher: worktree provision + mount; bind the git blobstore for locator containers; keep the filesystem blobstore for snapshot objects
- Engine guest: write CID onto the publication mount; journal the materialize fact; archive over `wasi:http`
- Retire `emery:origins` / `emery:ingest` in the same cut

Snapshot store, private workspaces, and accepted-CID identity are unchanged (RFC-87 / RFC-88 / RFC-90).

## Rejected here

- New `emery:publication` or `emery:forge` WIT
- A `wasi-git` package unless refusing both guest-git and exec
- Guest-visible `wasi:exec` as a Git placeholder
- Encoding the publication worktree as blobstore container names (rebuilds porcelain on the wrong API)
- Aliasing RFC-87 blobstore objects to Git objects
- Host-owned bare repo + `git --git-dir` as the operator surface (already rejected in RFC-95)

## Edit points

Not locked by the product RFC — change these here before coding:

- Container naming for the git blobstore (`repo@rev` vs opaque id + metadata)
- Whether index staging is a launcher hook after the guest returns, or part of worktree provision before unmount
- Whether archive's HTTP client lives in `change` (guest) or a small host-side projector that still uses `wasi:http` + identity rather than a custom WIT
- Exact first-time clone root (`$EMERY_HOME/publication/<target>/` is the RFC-95 example, not a schema)
