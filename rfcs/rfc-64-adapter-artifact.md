# RFC-64: The Adapter Artifact — One Component, No Manifest

> Status: Proposed · Depends: [RFC-61](rfc-61-omnia-migration.md) (guests with prose compiled in, the deployment manifest), RFC-47/48 (adapter identity, registry transport — landed; amended here) · Amends: [RFC-63](rfc-63-adapter-hydration.md) (the store holds components, not trees) · Owns: the deployable adapter artifact format and the retirement of `adapter.yaml`

## Abstract

Post-RFC-61 an adapter's entire behavior lives in its component: briefs compile in as prompt bodies, references ship as the embedded MCP shelf, and the former extension tools run as in-guest library code. Yet distribution still ships a directory tree — `adapter.yaml` beside a committed `guest.wasm` beside the prose sources — packed as a bespoke `tar+zstd` OCI layer and resolved through a manifest-first probe chain. This RFC finishes the collapse the migration started: **the deployable adapter artifact is exactly one wasm component**, published and pulled as a standard wasm-pkg package, and referenced by path from the deployment manifest exactly as Omnia's `guest-link` example references its guests. `adapter.yaml` is retired; every fact it carried moves into the component or the registry reference. The prose trees stay in the adapters repo as authoring sources, but they are build inputs, never shipped files.

## Where the manifest's facts go

`adapter.yaml` carries five facts the engine consumes before dispatching an operation. Each gets a wasm-native home:

| Fact | New home |
| ---- | -------- |
| `name`, `version` | The component's package identity (`augentic:<name>@<semver>`) and the wasm-pkg reference it publishes under — one identity, declared once, carried in the artifact itself. |
| `axis` | Introspected from the component's exports: a component exporting `augentic:specify/source` xor `/target` *is* that axis. The name-uniqueness-across-axes rule becomes "one identity, one world". |
| `description` | Registry package metadata; operator-facing listings read the registry, not a file. |
| Target `inputs[]`, `platforms` | A new `describe` operation on the `target` interface (below). |
| `specify` compatibility floor | The same `describe` record, on both axes. |

**The `describe` operation.** Each axis interface in `wit/specify.wit` gains one deterministic export, consistent in spirit with the existing `guidance` leg:

```wit
/// Deterministic self-description, read by the host at resolve time.
describe: func(id: adapter-id) -> manifest;
```

with `manifest` a typed record carrying the compatibility floor and, for targets, the declared `inputs[]` and platform capability. The host reads it through one instance-per-call dispatch at resolve time and caches the answer against the store entry's digest — the same execution model every other operation already uses. A custom wasm section was considered and rejected: it is stringly-typed, invisible to the WIT contract, and saves only one cheap instantiation.

## The artifact and its transport

- **Publish.** `cargo build --target wasm32-wasip2 --release` emits the component; the publish step pushes `target/wasm32-wasip2/release/specify_<name>.wasm` to the registry as a standard wasm-pkg package under `augentic:<name>@<semver>`. The RFC-48 tree-packing path — `pack_adapter`'s byte-deterministic tar, symlink dereferencing, `ADAPTER_LAYER_MEDIA_TYPE`, the raw-OCI-layer transport that existed because `wasm-pkg-client` rejected an opaque blob — retires with the tree it packed.
- **Store.** A hydrated entry is one file: `<store-root>/<name>@<version>.wasm`, with the digest sidecar now a plain SHA-256 of the component bytes. Verify-on-read, the install lock, atomic temp-then-rename, and `.specify/adapters.lock` all survive unchanged — they just hash one file instead of a tree.
- **Deploy.** The generated deployment manifest (RFC-63) points each `[[guest]] source.path` at the store file — byte-for-byte the shape of [`omnia/examples/guest-link/omnia.toml`](https://github.com/augentic/omnia/blob/main/examples/guest-link/omnia.toml): a guest id, a path to a component, a `link` allow-list on the workflow guest. Nothing about routing, mounts, or link dispatch changes.

## What each repo deletes

**`specify-adapters`:**

- The eight `adapter.yaml` files. Identity moves to each guest package's WIT/wasm-pkg declaration; nothing else read them.
- The committed `guest.wasm` artifacts and the `refresh-guests` copy step. The component is built where cargo puts it and published from there — the committed copy existed only to feed the tree pack and the sibling-checkout developer manifest. The runtime tests already build guests from source into the cargo target directory and are unaffected.

**`specify` (engine):**

- The manifest half of `crates/workflow/src/adapter/`: `ADAPTER_FILENAME`, `validate_manifest`, the per-axis schema validation, the manifest-cache probe leg, and the `adapter-manifest-*` / `adapter-schema-violation` error codes. `locate_axis` resolves an identity to one `.wasm` path (store for pins, in-repo build output for development); the axis check becomes an export check; version, floor, and target capability checks read the package identity and the cached `describe` answer.
- `schemas/{adapter,source,target}.schema.json` and their embedded constants in `specify-schema`, together with the lint `cross-reference` arms and `index/adapter.rs` extraction that policed manifests — the type system and the WIT contract now carry that consistency, per RFC-61's lint-shrinkage rule.
- The in-tree developer `omnia.toml` re-points at the sibling checkout's `target/wasm32-wasip2/release/` artifacts, with a documented one-time `cargo make build-guests` in the adapters checkout replacing the committed-artifact convenience.

Docs follow in the same change: the adapter vocabulary in `AGENTS.md`, `docs/explanation/adapter-anatomy.md`, `docs/reference/directory-layout.md`, the engine's `workflow.md`, and RFC-63's "prose and the built component together" packing description.

## Scope

- The `describe` operation and `manifest` record in `wit/specify.wit`, and its implementation in `specify-guest-kit` plus all eight guests.
- Retirement of `adapter.yaml`, its schemas, and the manifest resolver/validator surface in the engine.
- The single-component publish/pull path (wasm-pkg) replacing the tree pack and raw OCI layer, and the single-file store entry shape.
- Deployment-manifest generation and the developer `omnia.toml` re-pointed at component files.
- The committed-`guest.wasm` retirement and the `refresh-guests` deletion in the adapters repo.

## Out of scope

- **Version-range resolution, release index, third-party namespaces** — RM-21, unchanged.
- **Omnia OCI guest sources** — Omnia keeps loading guests by path; the store remains the path namespace.
- **The prose authoring surface** — briefs, references, and rules stay markdown files in the adapter trees, embedded at build time; authoring and review workflows do not change.
- **The workflow guest's distribution** — it ships embedded in the `specify` binary per RFC-61/63, not through the adapter registry.

## Acceptance criteria

1. Publishing an adapter is: release-build the guest package, push the emitted component to the registry under `augentic:<name>@<semver>`. No tree pack, no manifest, no committed wasm in either repo.
2. Hydration materializes `<store-root>/<name>@<version>.wasm`; the generated deployment manifest references those files directly, and a fresh machine runs `specify init && specify plan …` with nothing but the binary, the pins, and network access.
3. Neither repository contains an `adapter.yaml`, the per-axis manifest schemas, or any code path that reads them; `rg 'adapter\.yaml'` across both repos hits only historical DECISIONS entries.
4. Axis, identity, and compatibility all derive from the artifact: axis from the exported world, name/version from the package identity, floor and target capability from the cached `describe` answer — with typed errors for a floor violation, an axis mismatch against the binding, and a digest mismatch against `adapters.lock`.
5. The developer loop needs no committed artifacts: the engine's `omnia.toml` loads sibling `target/wasm32-wasip2/release/` components after one documented build command, and the RFC-62 prose overlay is unaffected.
6. `make lint` and `cargo make ci` are green in both repos with the manifest-policing lint arms deleted alongside the manifests they policed.

## Risks and invariants

- **`describe` must stay deterministic and effect-free.** It is metadata, not judgment: no model call, no filesystem write, answerable from compiled-in constants. A guest whose `describe` needs runtime state is a design error.
- **Resolve-time instantiation is a new host dependency.** Reading the floor and capabilities now requires loading the component. The cost is bounded (instance-per-call is the runtime's execution model and the answer caches against the entry digest), but a corrupt component now fails at describe-dispatch rather than at YAML parse — the typed error must name the identity and the artifact path, not surface an Omnia load panic.
- **Identity lives in one place.** The wasm-pkg reference is the sole identity authority; nothing may re-introduce a sidecar declaring name or version, or the two-sources-of-truth drift `adapter.yaml` created returns.
- **The committed-artifact discipline retires deliberately.** RFC-61 committed `guest.wasm` files to keep the native build graph flat; this RFC trades that for a one-command build step in the developer posture and standard registry pulls everywhere else. If the sibling-checkout loop proves too slow, the fix is a fetch-from-registry developer manifest, never a return to committed blobs.
- **Prose embedding is unchanged.** The RFC-61 inversion — briefs as prompt bodies, references as the MCP shelf — is untouched; this RFC changes how the component travels, not what is inside it.
