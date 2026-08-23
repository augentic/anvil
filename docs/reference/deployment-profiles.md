# Deployment profiles

How the `emery` runtime binds engine storage, and how deployments other than the shipped local binary swap that binding without touching engine code. A profile is host policy: one `omnia::runtime!` invocation choosing what backs the `wasi:keyvalue` / `wasi:blobstore` capability imports. The engine ships fixed key and container formulas and never learns which backing it runs over.

## The seam

Engine state — the generation store, the `current` pointer, and the component cache — is reachable only through the storage capabilities (`omnia_guest::StateStore` / `BlobStore` on the guest side). The names the engine uses are flat, deployment-neutral formulas:

| Surface | Kind | Name |
| --- | --- | --- |
| Current-generation pointer | keyvalue key | `spec/current` |
| Generation documents | blobstore container `spec` | `generations/<id>/<doc>.md` |
| Component cache | blobstore container `adapters` | `<name>.wasm` |

The host side of the seam is a backend type implementing `omnia::Backend` (connection options from the environment) plus the host context traits `WasiKeyValueCtx` and `WasiBlobstoreCtx`. Bucket and container identifiers cross the seam exactly once — on `open_bucket` and the container methods — which is where a profile may rewrite them.

Package pins dispatch to statically admitted guests and do not imply a stored component. Dynamic resolution is deferred; its eventual artifact identity and integrity model must bind the resolved digest to the component the host executes rather than add an engine-owned mutable sidecar.

## The shipped profile: local filesystem

[`src/main.rs`](../../src/main.rs) binds both storage hosts to `omnia_filesystem::Client`: a durable, network-free store rooted at `FILESYSTEM_ROOT` (default `.omnia/storage` under the invocation directory). One invocation directory is one project; isolation between projects is the filesystem root itself. Generations survive restart, and nothing writes the working tree — the `.` mount is read-only.

## Project-id-keyed shared backings

A multi-project deployment can scope every bucket and container under a project id. This requires a genuinely shared backing: a command-mode process over in-memory defaults creates one fresh store and one project id, so it cannot demonstrate cross-project isolation or persistence.

The `multi_project_isolation` scenario in [`tests/specify.rs`](../../tests/specify.rs) exercises the invariant directly: two project-scoped views over one shared scripted store commit and show independent generations, and no unprefixed key is written. A concrete deployment supplies its shared backend clients and rewrites identifiers at the `WasiKeyValueCtx` and `WasiBlobstoreCtx` boundary; that host-specific configuration does not belong in the engine.

## Remote backings

`omnia-backends` ships host clients that drop into the same `hosts:` table:

| Backend | keyvalue | blobstore | Configuration |
| --- | --- | --- | --- |
| `omnia-filesystem` | yes | yes | `FILESYSTEM_ROOT` |
| `omnia-redis` | yes | — | `REDIS_URL` |
| `omnia-nats` | yes | yes | `NATS_ADDR` |
| `omnia-mongodb` | — | yes | `MONGODB_URL` |
| `omnia-azure-blob` | — | yes | `AZURE_BLOB_ENDPOINT` |

Credentials and endpoints live in the host binding's environment, never in engine state or operator files.

> [!WARNING]
> Identifier grammar is backend policy. The filesystem backend rejects `/` inside a bucket or container name (path-traversal fencing), so a project-id prefix targeting it needs a single-segment delimiter (for example `<project>--spec`) or per-project roots. The in-memory and remote backends accept `/`-separated identifiers.

Remote-binding performance is unmeasured: the numbers stay unconfirmed until a remote backing is deployed and wall-clocked (design/portable-storage.md, risk 4).

## See also

- [Architecture standards](../standards/architecture.md) — deployment policy and the workspace shape.
- [CLI architecture](../contributing/cli-architecture.md) — the `omnia::runtime!` invocation in detail.
