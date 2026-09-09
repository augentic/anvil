# Deployment profiles

How the `emery` runtime binds engine storage, and how deployments other than the shipped local binary swap that binding without touching engine code. A profile is host policy: one `omnia::runtime!` invocation choosing what backs the `wasi:keyvalue` / `wasi:blobstore` capability imports. The engine ships fixed key and container formulas and never learns which backing it runs over.

## The storage boundary

Engine state — the revision store and its current revision id — is reachable only through the storage capabilities (`omnia_guest::StateStore` / `BlobStore` on the guest side). The names the engine uses are flat, deployment-neutral formulas:

| Surface             | Kind                            | Name                |
| ------------------- | ------------------------------- | ------------------- |
| Current revision id | keyvalue key                    | `current-revision`  |
| Revision documents  | blobstore container `revisions` | `<id>/<doc>.md`     |

The host side of the boundary is a backend type implementing `omnia::Backend` (connection options compiled into the `hosts:` row, or loaded from the environment when the row carries none) plus the host context traits `WasiKeyValueCtx` and `WasiBlobstoreCtx`. Bucket and container identifiers cross the boundary exactly once — on `open_bucket` and the container methods — which is where a profile may rewrite them.

Loaded components are not engine state: a path- or package-shaped adapter loads through the deployment's `omnia:plugins/loader` capability, whose acquisition policy is the shipped profile's declarative `locations:` list — the `.` path root first, reading a local component fresh on every run, then the `omnia.host` registry endpoint, whose fetches are likewise read fresh on every run — the shipped profile declares no project cache, so no fetched component persists between runs and resolution is always fresh-release-preferred. Earlier cache trees (`.omnia/cache/wasm-pkg`, `.omnia/storage/plugins/`) are orphaned — nothing reads them; delete them freely. Integrity binds the resolved sha256 digest to the exact bytes the host executes, verified against the binding's optional pin, never an engine-owned mutable sidecar. Bare names dispatch to statically admitted guests and do not imply a stored component.

## The shipped profile: local filesystem

The `deployment!` macro in the native arm of [`src/lib.rs`](../../src/lib.rs) binds both storage hosts to `omnia_filesystem::Client` with the root compiled into the invocation: a durable, network-free store at `.omnia/storage` under the invocation directory (`blobstore/` and `keyvalue/`). The root is deployment policy, not an environment tunable — retargeting it means shipping a different profile, never setting `FILESYSTEM_ROOT`. One invocation directory is one project; isolation between projects is the filesystem root itself. Revisions survive restart, and nothing writes the working tree — the `.` mount is read-only.

## Project-id-keyed shared backings

A multi-project deployment can scope every bucket and container under a project id. This requires a genuinely shared backing: a command-mode process over in-memory defaults creates one fresh store and one project id, so it cannot demonstrate cross-project isolation or persistence.

The `multi_project_isolation` scenario in [`tests/specify.rs`](../../tests/specify.rs) exercises the invariant directly: two project-scoped views over one shared scripted store commit and show independent revisions, and no unprefixed key is written. A concrete deployment supplies its shared backend clients and rewrites identifiers at the `WasiKeyValueCtx` and `WasiBlobstoreCtx` boundary; that host-specific configuration does not belong in the engine.

## Remote backings

`omnia-backends` ships host clients that drop into the same `hosts:` table:

| Backend            | keyvalue | blobstore | Environment configuration |
| ------------------ | -------- | --------- | ------------------------- |
| `omnia-filesystem` | yes      | yes       | `FILESYSTEM_ROOT`         |
| `omnia-redis`      | yes      | —         | `REDIS_URL`               |
| `omnia-nats`       | yes      | yes       | `NATS_ADDR`               |
| `omnia-mongodb`    | —        | yes       | `MONGODB_URL`             |
| `omnia-azure-blob` | —        | yes       | `AZURE_BLOB_ENDPOINT`     |

The environment variables apply to a bare `hosts:` row; a row carrying compiled-in connect options (`Backend(options)`, as the shipped profile does for its filesystem root) ignores them. Credentials and endpoints live in the host binding's environment, never in engine state or operator files.

> [!WARNING]
> Identifier grammar is backend policy. The filesystem backend rejects `/` inside a bucket or container name (path-traversal fencing), so a project-id prefix targeting it needs a single-segment delimiter (for example `<project>--revisions`) or per-project roots. The in-memory and remote backends accept `/`-separated identifiers.

Remote-binding performance is unmeasured: the numbers stay unconfirmed until a remote backing is deployed and wall-clocked (design/portable-storage.md, risk 4).

## See also

- [Architecture standards](../standards/architecture.md) — deployment policy and the workspace shape.
- [CLI architecture](../contributing/cli-architecture.md) — the `omnia::runtime!` invocation in detail.
