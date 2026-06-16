# RFC-48: Adapter Packaging and Transport — OCI / wasm-pkg Distribution

> Status: Draft · Execution order: **2nd of RFC-47 → RFC-48 → RFC-49**, executed to completion in numerical sequence. Runs after identity ([RFC-47](rfc-47-adapter-identity.md)) lands; self-contained against today's two-repo platform. The consolidation half ([RFC-49](rfc-49-repository-topology.md)) runs *afterward* and is not a precondition of any step here. · Depends: [RFC-47: Adapter identity](rfc-47-adapter-identity.md) (the semver identity this RFC distributes), the wasm-pkg extension-distribution precedent (`crates/registry/src/{package,resolver}.rs`, `crates/registry/src/cache/fetch.rs`, the `extensions` job in `.github/workflows/release.yaml`), the adapter loader and install path (`crates/workflow/src/init/{adapter_uri,git,cache}.rs`), the per-project cache resolver (`crates/schema/src/cache.rs`) · Related: [RFC-49: Repository topology](rfc-49-repository-topology.md) · Roadmap: the distribution portion of [RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model).

## Abstract

An adapter is published, fetched, verified, and cached as an **immutable registry artifact**, over the same wasm-pkg / OCI plumbing first-party WASI extensions already use. One package carries both the adapter's **prose** (`adapter.yaml`, briefs, references) and its **wasm** (an adapter ships at most one declared extension), so `omnia@1.0.0` is one pull — not a git sparse checkout plus a separate extension fetch.

Identity ([RFC-47](rfc-47-adapter-identity.md)) is a semver; this RFC binds it to an immutable, content-addressed locator and proves it on read with the **registry's own content digest**, not a bespoke downstream Merkle. Because the registry already supplies immutability and content-addressing, the shared cache is an ordinary download-once-by-identity store.

The authoring tree mirrors that unpacked artifact: an author writes prose into a structure shaped like the installed store entry, declares shared content (spec-runtime, shared rules) as versioned dependencies and its wasm extension inline in `adapter.yaml`, and runs `specify adapter build` — a single step that resolves the prose deps, compiles any declared extension, and packs the deterministic artifact (the way `cargo build` compiles *and* links) — so the local tree is byte-identical to what ships (D9/D10). Self-containment becomes a locally reproducible fact, not a publish-only transform.

## Motivation

[RFC-47](rfc-47-adapter-identity.md) fixes what an adapter is *named*; this RFC fixes how the bytes behind that name travel and how they are *proven*. The registry is the natural transport:

- **We already run the loop.** The `extensions` job in `release.yaml` publishes `wasm32-wasip2` components to the `augentic.io` (GHCR-backed) registry via `wkg`; `crates/registry/src/{package,resolver}.rs` fetch, stream, sha256-verify (`specify_schema::digest::Hasher`), and atomically install them. Adapter distribution is the same loop with a tree payload instead of one blob.
- **Adapters are trees, not blobs.** The extension path installs exactly one `module.wasm`; an adapter is a directory of prose plus optional wasm (see [Background](#background)). The gap this RFC closes is *one blob → a packed tree*.
- **The registry gives immutability and a digest for free.** Today's transport is a git sparse checkout (`init/git.rs`) copied into the per-project cache (`init/cache.rs`). A git ref is a *moving* locator — proving immutability against it needs a bespoke publish-time Merkle plus a moved-tag backstop. An OCI content digest *is* immutable identity, so that machinery collapses to verifying the registry descriptor.

## Background

### What an adapter is, and isn't

A WASI extension is one wasm component — a single blob with a single digest, which is why the existing fetch path (`crates/registry/src/package.rs`) installs one `module.wasm` and is done. An adapter is a **directory tree**:

- `adapter.yaml` — the prose manifest the loader validates.
- `briefs/*.md` — prose the *agent* reads and acts against; never run as code.
- `references/**` — supporting prose plus the vendored `references/spec-runtime/` bundle.
- optionally a single wasm extension, declared inline in the manifest's `extension` block and built from a co-located crate (D10/D11).

The loader (`SourceAdapter::resolve` / `TargetAdapter::resolve`) probes a *directory*, so the packaging problem is *one blob → a tree of prose plus (optionally) wasm*, with the prose dominating.

**Distribution is not execution.** Shipping an adapter as a registry artifact changes how its bytes travel and how identity is proven; it does not turn briefs into executable wasm. Source adapters stay `execution: agent` (enforced by `source.schema.json`). Packaging also cannot *hide* the prose — the agent reads it as cleartext at point of use — so IP protection is an access-control and licensing concern, not a packaging one (see [Security / IP considerations](#security--ip-considerations)).

### Three ways to put a tree in the registry

The registry stores content-addressed blobs and (via wasm-pkg) wasm components; an adapter is neither. Three shapes close that gap, in rising order of how literally the adapter "becomes wasm":

- **(A) Packed-tree blob.** Pack the whole tree (`adapter.tar.zst`, sidecar wasm included), stream it through the existing acquire-bytes path, then unpack. Greatest reuse, but hinges on whether `wasm-pkg-client` will carry an opaque, non-component blob (the [Prerequisite spike](#prerequisite-spike)).
- **(B) OCI artifact with layers.** Push the prose tree as one OCI layer and the wasm extension as a second layer, fetched via `oci-client` / `oci-wasm`. The registry's native model for "prose + wasm in one package," at the cost of a sibling fetch path — but those crates are already transitive deps, so the cost is small. The RFC's working default (see [Packaging shapes (D1)](#packaging-shapes-d1)).
- **(C) Wrap-as-component.** Compile a thin wasm component that embeds the tree as data and self-extracts. The only *literal* wasm artifact and the heaviest: prose gains a build step that does nothing at runtime.

All three reuse the existing registry, auth, namespace routing, and content digests, and none requires prose to stop being prose. The choice is purely *how the tree is wrapped*, which reduces to the spike question and is recorded as D1.

## Prerequisite spike

One *transport* question sizes the effort and picks D1's packaging shape. It spans publish, pull, *and* re-verify-on-read, because the existing loop publishes with `wkg` (`release.yaml`) and fetches with `wasm-pkg-client`:

> **Across `wkg publish`, `wasm-pkg-client` pull, *and* re-verify-on-read, can the `augentic.io` (GHCR-backed) registry carry an opaque, non-component blob (a packed tree)? Or must adapter transport use an OCI artifact with layers (`oci-client` / `oci-wasm`) against the same registry?**

Two facts narrow the suspense:

- **The OCI client is already in the tree.** `wasm-pkg-client@0.15.0` depends transitively on `oci-client` and `oci-wasm` (workspace `Cargo.lock`), so shape (B) adds **no new dependency** — it is a different call against crates already compiled in.
- **The risk is asymmetric.** `wasm-pkg` is component-oriented on *both* ends, so a plausible outcome is "pull tolerates an opaque blob but `wkg publish` does not" — which a pull-only spike would miss. Probe publish first.

Resolve this before authoring D1's mechanism. The working assumption is **(B)**; (A) is adopted only if the spike shows `wasm-pkg` carries an opaque blob cleanly in *both* directions and re-verification does not force a re-tar.

## Principles

- **Identity is fixed at publish, proven by the registry.** A published `name@X.Y.Z` is immutable: the registry content digest names exactly those bytes. Consumers *verify the digest*; they do not re-derive identity from a checkout.
- **Artifacts are self-contained.** Everything an adapter needs at resolution time — spec-runtime, the declared extension — is bundled at publish. Downstream resolution does no vendoring and dereferences no in-tree symlinks; the installed tree *is* the published tree.
- **Authoring mirrors the artifact.** The authoring tree is shaped like the unpacked store entry, so `AdapterLocation::Local` and `AdapterLocation::Cached` resolve through one code path and what an author lints and tests is what ships. Shared content is a declared, versioned dependency resolved by a local `build` step (D9), never a symlink into a framework checkout.
- **The cache is boring.** A global store keyed by immutable `(name, version)` is download-once-by-identity with a temp-then-rename install. Integrity lives upstream at publish; downstream is a one-line verify.
- **Resolution stays project-local.** A shared *store* is storage, not a resolution fallback — what `name` resolves to is the project's pinned `(name, version)`, preserving [DECISIONS.md §"Resolution is project-local only"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#adapter-loader-axis-routing).
- **Pre-1.0 major cut, no migration framework.** Re-init, not migration. No compatibility aliases for git-ref pins or the `version: 1` manifest shape.

## Security / IP considerations

The prose *is* the IP — the briefs encode the methodology — and the packaging layer cannot protect it. This RFC says so explicitly so no later change reaches for obfuscation:

- **Prose is plaintext at the consumer at point of use.** No packaging shape changes this: tarball (A) untars, OCI layers (B) are pullable, wasm-wrapped bytes (C) are `strings`-able, and any "expand at point of use" step must still hand the model cleartext. Client-side obfuscation is a speed bump, not protection.
- **Access control is the real lever.** Whether the registry namespace is public or authenticated (D8) gates *who* obtains the bytes — stronger than obfuscating freely-handed-out bytes, and a net improvement over today's public git checkout.
- **Licensing carries the rest.** Copyright and registry terms govern redistribution; the risk is redistribution, not reverse-engineering markdown.
- **Sensitive logic belongs in the bundled wasm, not the prose.** Proprietary deterministic logic compiled into a declared extension (bundled by D3) is better protected than plaintext markdown.

Per-licensee **watermarking** (attribution, not prevention) and **server-side prose expansion** (true prevention, at the cost of the offline / self-contained property) are recorded under [Non-Goals](#non-goals).

## Design

### Normative decisions

| ID | Decision | Implementation consequence |
| --- | --- | --- |
| **D1 Packaging format** | An adapter publishes as one immutable registry artifact carrying the packed prose tree plus bundled wasm. Working-default shape: an **OCI artifact with layers** (prose layer + a wasm layer) via the already-vendored `oci-client` / `oci-wasm`; optimisation shape (only if the spike clears an opaque blob in *both* directions): a **packed-tree blob** (`adapter.tar.zst`). `specify adapter build`'s pack stage is byte-deterministic either way. | Spike-gated (see [Prerequisite spike](#prerequisite-spike)). OCI adds a sibling fetch module reusing registry/auth/namespace config; the blob shape generalises `crates/registry/src/package.rs` to stream-and-unpack a tree. See [Packaging shapes (D1)](#packaging-shapes-d1). |
| **D2 Immutable fetch locator** | Fetch targets an **immutable, content-addressed locator** (OCI `@sha256:` digest, or an immutable tag whose digest is recorded), never a branch. | `init/adapter_uri.rs` gains a package-ref form (`specify:omnia@1.0.0`) alongside the local-path / GitHub-URL / shorthand forms, with no branch-ref defaulting. The recorded digest (D4) backstops a moved tag as `adapter-digest-mismatch`. |
| **D3 Self-contained artifact** | Spec-runtime and the wasm of the declared extension are bundled **at publish**; downstream resolution does no vendoring and dereferences no symlinks. | The legacy `vendor_spec_runtime` ancestor-walk (`init/cache.rs`) retires; spec-runtime is resolved by identity at author time ([D12](#shared-content-dependencies-d12)) and `build` packs it into the artifact. The declared extension's `module.wasm` ships inside the artifact. Because `build` (D9/D10) lands the same bytes at author time, the local working tree is self-contained too. See [Bundled extension (D3)](#bundled-extension-d3). |
| **D4 Identity via registry content digest, verified on read** | The artifact's identity is the registry content digest (`sha256:`). The consumer records it on install and re-checks it on every read, refusing a mismatch. Re-publishing an existing `(name, version)` with different bytes is rejected **at publish**. | Verification reuses the streaming `specify_schema::digest::Hasher` in `package.rs`. The bespoke publish-time tree Merkle is **not** built — the registry descriptor is the trust anchor. `ManifestMeta` (`init/cache.rs`) records the digest. |
| **D5 Trivial global store + projection** | A global store at `<adapters-root>/<name>@<version>/`, resolved `$SPECIFY_ADAPTER_CACHE` → `$XDG_CACHE_HOME/specify/adapters` → `$HOME/.cache/specify/adapters` → `<temp>/specify/adapters` (the `mirror_dir` precedent). Install = pull → temp → verify → atomic rename → `chmod` read-only. The per-project cache is a **directory symlink** into the entry, degrading to a recursive **copy** when symlink creation fails. | New resolver in `crates/schema/src/cache.rs`; install path in `init/cache.rs` link-or-copies from the store; `locate_axis` and the `AdapterLocation::{Cached,Local}` labels are unchanged. See [Store layout and projection (D5)](#store-layout-and-projection-d5). |
| **D6 Publish tooling** | A publish step mirrors the `extensions` release job: pack the tree (+ bundled wasm), push `specify:<name>@${VERSION}`, pull back and verify. | New job in `.github/workflows/release.yaml`, reusing the `wkg` / GHCR / `specify -> augentic.io` namespace plumbing the extension job exercises. |
| **D7 Adapter repo extraction** | **Adapters — prose plus co-located extension crates — relocate to a dedicated `augentic/specify-adapters` repo**, extracted from `augentic/specify` once [D12](#shared-content-dependencies-d12) severs the `adapters/shared/` coupling (Phasing step 8). The *other* half — folding `augentic/specify` + `augentic/specify-cli` into one platform repo — is [RFC-49](rfc-49-repository-topology.md), executed afterward and **not** a precondition. First-party extension wasm relocates with the adapters: contract/vectis ship inside the adapter artifact (D3/D6/D11), so the `extensions` publish job retires. | Touching the adapter loader / cache scope requires the cross-repo `rg` sweep per [AGENTS.md §"Note to the implementing agent"](https://github.com/augentic/specify-cli/blob/main/AGENTS.md); net of [RFC-49](rfc-49-repository-topology.md) the sweep is **two-way** (platform ↔ adapters). Retiring `extensions` removes the standalone `specify:contract@x` / `specify:vectis@x` packages, the `first_party_permissions` catalog, and the scalar first-party `tools:` form — decommissioned, not aliased. |
| **D8 Registry visibility and pull auth** | First-party adapter artifacts publish to an **authenticated** namespace; pulling requires credentials, gating *who* obtains the bytes. A public namespace is a deliberate per-adapter opt-out, not the default. | Pull-side auth reuses the wasm-pkg / GHCR credential path the publish step (D6) exercises — layered config in `crates/registry/src/package.rs::load_config` and `.specify/wasm-pkg.toml`. No new transport. |
| **D9 Authoring mirrors the unpacked artifact** | The authoring tree is shaped like the unpacked store entry: authored prose (`adapter.yaml`, `briefs/`, `references/`, `rules/`) plus a **reserved vendored namespace** an author never hand-writes. Shared content is declared by identity in `adapter.yaml.requires`; the declared-extension wasm builds from a co-located crate declared in `adapter.yaml.extension` (D11). `specify adapter build` resolves `adapter.lock`, writes the `requires` reserved paths, compiles any declared extension, and packs the artifact in one step; `package.include` is the pack manifest. | Adds optional `requires` / `package` blocks to `schemas/{source,target}.schema.json` and a new `adapter.lock` schema; adds the author-time `specify adapter build` verb (with `--check` / `--dry-run`). Hand-authoring a gitignored reserved path raises `adapter-authored-reserved-path`; lock drift raises `adapter-vendor-stale`. See [Authoring structure (D9)](#authoring-structure-d9). |
| **D10 Co-located extension source** | An adapter's declared-extension Rust crate lives **beside its prose** at `extension/`. The built `extension/module.wasm` is the only shipped byte and is **committed + digest-pinned** (`adapter.lock`), mirroring the existing `extensions/contract/dist/*.wasm` + drift-test precedent — so packing, install, and lint need no wasm toolchain; only *refreshing* the byte does. A sparse Cargo workspace spans the per-adapter crates. | `specify adapter build` compiles the crate to `wasm32-wasip2`, writes `module.wasm`, and updates its `adapter.lock` digest — but **only when an extension is declared and the committed wasm no longer matches its lock digest**, so prose-only adapters (6 of 8 today) share the verb and never invoke cargo. `package.include`'s `extension/module.wasm` ships the artifact and excludes crate source. The Rust toolchain + `clippy`/`test` CI enters the adapters tree only for extension-bearing crates, scoped like the `extensions` carve-out. See [Co-located extension source (D10)](#co-located-extension-source-d10). |
| **D11 Extension declaration in the manifest** | There is no standalone `tools.yaml`, and an adapter ships **at most one binary**. The plural `adapter.yaml.tools[]` array collapses to a singular `adapter.yaml.extension` object carrying an optional `name` (the run handle, defaulting to the adapter name) and structured WASI `permissions`. **Breaking** change: the array→object collapse, plus the per-extension `version` moving from *required* to *rejected* (the wasm rides the adapter's semver ([RFC-47](rfc-47-adapter-identity.md)), covered by D4) and `permissions` moving from a flat string array to the `{read, write}` shape the WASI runner already speaks. `source` / `sha256` never appear — the wasm builds from the co-located crate (D10). | Replaces the `tools[]` array (and its `toolDeclaration` `$def`) with a singular `extension` object (an `extensionDeclaration` `$def`) in `schemas/{source,target}.schema.json`. Renames `AdapterToolDeclaration` → `AdapterExtensionDeclaration` (`adapter/core.rs`), collapses it to an `Option<…>`, and unifies it with `specify_extension_manifest::ExtensionPermissions` while preserving the wasmtime-free loader boundary, retires the `tools.yaml` reader (`load::plugin_sidecar`), and **replaces** the now-moot `adapter-tool` cross-reference rule with a co-located-crate / built-`module.wasm` presence check. `specify extension run <name>` resolves the extension from the installed adapter tree. |
| **D12 Shared-content dependency transport** | The `requires` targets (`spec-runtime`, `review-team-protocol`, `core-rules`) are themselves **versioned registry artifacts**, published and fetched over the same plumbing as adapters (D1/D6). `specify adapter build` resolves each at **author/publish time**; the consumer never resolves `requires`, because D3 bundles the resolved bytes. This makes D10's `adapters/shared/` severance real: the author depends on a published *identity*, not a sibling source tree. | Adds a prose-only sibling of the adapter publish job (D6), a registry-targeting `requires` resolver in `build`, and an `adapter.lock` digest pin per resolved dependency. Retires the `adapters/shared/**` sparse checkout (`init/git.rs`) and the `vendor_spec_runtime` ancestor-walk (`init/cache.rs`). See [Shared-content dependencies (D12)](#shared-content-dependencies-d12). |

### Packaging shapes (D1)

The [design space](#three-ways-to-put-a-tree-in-the-registry) is three shapes (A / B / C); the spike picks between (A) and (B), and (C) is recorded-but-rejected. The working assumption is **(B)**, which the spike may overturn in favour of (A):

- **(B) OCI artifact with layers — working default.** A prose layer plus a single wasm layer, over the already-transitive `oci-client` / `oci-wasm`. Each is independently content-addressed (serving D3's bundle and D4's verify-on-read directly), and an extension-only rebuild re-pushes just the wasm layer, leaving the prose layer untouched.
- **(A) packed-tree blob — optimisation if the spike clears it.** One `adapter.tar.zst` streamed through ~90% of `crates/registry/src/package.rs`. Cheaper *only if* `wasm-pkg` carries an opaque blob in both directions; otherwise its apparent simplicity is lost to a parallel publish path.
- **(C) wrap-as-component is not pursued** — a prose build step that buys nothing at runtime.

**Verify-on-read differs by shape (D4).** Under (B) the consumer re-checks the cached layer descriptors it already holds — no re-tar. Under (A) re-hash-on-read must either re-tar (requiring a byte-deterministic pack) or retain the tarball past install. (B) avoids both.

**Pack must be byte-deterministic.** D4 rejects re-publishing the same `(name, version)` with different bytes, and D9's "post-`build` tree packs to the publish digest" equivalence depends on identical inputs producing an identical archive. `tar` + `zstd` are non-deterministic by default, so `build`'s pack stage normalises entry order, mtimes, uid/gid, and permission bits and pins compression parameters. Under (B) this narrows to the single prose layer.

### Bundled extension (D3)

An adapter that declares a wasm extension (D11) ships its `module.wasm` *inside* the artifact, so one pull is self-contained and one digest covers prose + wasm. The alternative — a prose-only artifact with the extension resolved separately — lets an extension bump avoid republishing the adapter, but reintroduces a second fetch and a second digest. v1 bundles; a split-extension-channel is a deferred optimisation if extension churn outpaces adapter churn.

### Store layout and projection (D5)

```text
$XDG_CACHE_HOME/specify/adapters/
  omnia@1.0.0/                  # immutable, digest-verified, read-only
    adapter.yaml
    briefs/…
    references/spec-runtime/…   # bundled at publish (D3)
    extension/module.wasm          # bundled at publish (D3)
  documentation@1.0.0/
```

- **The store is CLI-write-only.** The fetch path installs pristine bytes; the agent interacts only with the per-project cache, a read-only projection.
- **Install is pull → temp → verify → atomic rename → `chmod` read-only.** The temp dir lives under the store root so the rename is atomic on one filesystem. Because identity is immutable, concurrent installs are idempotent: one wins the rename, the other verifies the matching digest and discards its temp. A flock around the rename suffices, reusing the `File::try_lock` family from `plan_lock.rs` and the staged-install precedent in `crates/registry/src/cache/fetch.rs`.
- **The per-project cache is a directory symlink** into the store entry, degrading to a recursive copy on failure (Windows privilege, cross-device) — correct, but the copy forfeits dedup, so the sharing win is POSIX-first. `locate_axis` still finds a real directory; the `AdapterLocation` labels are unchanged.
- **Per-project provenance moves out of the linked tree.** `ManifestMeta` / `CodexMeta` are stamped *inside* the cache today; an immutable, cross-project store entry cannot hold a per-project stamp, so it relocates to a sidecar beside the symlink, not inside the linked entry.
- **The projection is a second concurrency point.** The store rename is flock-guarded; the per-project symlink creation is a separate step two concurrent `specify init` can race. It is idempotent (same target), but the test plan covers it.
- **The store is unbounded by design.** Cross-project reference-counting is deferred ([Non-Goals](#non-goals)). Acceptable because entries are small, immutable, and content-addressed — `specify adapter gc` / `archive prune`-style retention is the eventual home, not a v1 blocker.

### Authoring structure (D9)

The authoring tree is shaped like the unpacked store entry, so the loader resolves a `Local` working tree and a `Cached` artifact through one code path, brief-relative links resolve in place, and `specify lint framework` runs against the bytes that ship. It is a *complete* mirror only after `specify adapter build` populates the reserved namespace.

Two consequences for `specify lint framework`. First, after D9 there are no symlinks under an adapter — the reserved `requires` trees are gitignored real files — so the §F1 walk in `index/framework.rs` simplifies to an ordinary recurse and the symlink-target-removal gotcha retires. Second, a fresh clone is not a complete mirror until `build` runs, so the lint bootstrap (`make lint`) chains `specify adapter build` first, exactly as `init --workspace` chains an initial sync. Because the committed `module.wasm` (D10) already matches its lock digest on a fresh clone, that bootstrap `build` recompiles nothing — it resolves `requires` and stays toolchain-free.

```text
adapters/targets/omnia/            # authored tree == unpacked store entry (post-build)
  adapter.yaml                     # authored — manifest + requires + package.include
  adapter.lock                     # generated, committed — pinned dependency + extension digest
  briefs/{shape,build,merge}.md    # authored prose
  references/                      # authored prose you own …
    guardrails.md  providers/**  examples/**
    spec-runtime/                  # reserved — vendored from spec-runtime@1.2.0       (gitignored)
    agent-teams.md                 # reserved — vendored from review-team-protocol     (gitignored)
  rules/
    omnia.mdc  provider-only-host-access.md   # authored
    core/                          # reserved — vendored CORE-* rules                  (gitignored)
    universal/                     # reserved — vendored UNI-* rules                   (gitignored)
  extension/                          # co-located Rust crate (authored, source-only)
    Cargo.toml  src/               # authored — extension.name may differ from the adapter name
    module.wasm                    # reserved — built wasm, committed + digest-pinned  (NOT gitignored)
```

**Reserved vendored namespace.** Vendored content only ever lands at a fixed, documented set of paths an author never hand-writes, so the authored ⊎ vendored union is unambiguous and the gitignore set is exact:

| Declared dependency | Reserved path | Posture |
| --- | --- | --- |
| `requires.spec-runtime` | `references/spec-runtime/` | gitignored, regenerated by `build` |
| `requires.review-team-protocol` | `references/agent-teams.md` | gitignored, regenerated by `build` |
| `requires.core-rules` | `rules/core/`, `rules/universal/` | gitignored, regenerated by `build` |
| `adapter.yaml.extension` | `extension/module.wasm` | **committed**, built by `adapter build`, digest-pinned in `adapter.lock` |

The namespace has two postures. The three **`requires` trees** are gitignored and regenerated by `build`; a committed file under one raises `adapter-authored-reserved-path`, and `build --check` (CI) catches stale regenerated bytes as `adapter-vendor-stale`. The **built `extension/module.wasm`** is the exception: it is *committed* and digest-pinned, mirroring the `extensions/contract/dist` precedent so packing / install / lint never need a wasm toolchain — `adapter-authored-reserved-path` exempts it, and `build --check` instead verifies its bytes against the lock digest (`adapter-digest-mismatch`).

Unlike the D5 store, the author tree is human-owned, so vendored `requires` bytes are written **writable**: the `build --check` drift gate, not a filesystem bit, guards them — read-only files inside a working tree fight `git checkout` / `clean` and editor saves for no integrity gain the lock does not already provide.

**Manifest — `adapter.yaml`** gains a `requires` block (shared content declared by identity) and a `package.include` allow-list (the pack manifest):

```yaml
# adapters/targets/omnia/adapter.yaml
# yaml-language-server: $schema=https://github.com/augentic/specify-cli/raw/main/schemas/target.schema.json
name: omnia
version: "1.0.0"            # RFC-47 semver identity
axis: target
execution: agent
description: Omnia Rust WASM target adapter…
briefs:
  shape: briefs/shape.md
  build: briefs/build.md
  merge: briefs/merge.md

# Declared wasm extension (D11). An adapter ships at most one binary, so this is
# a singular object, not an array. version / source / fetch-digest are
# subsumed by the adapter's own identity (RFC-47) and content digest (D4);
# `name` is the optional run handle (defaults to the adapter name). The wasm
# builds from the co-located crate at extension/ (D10).
extension:
  name: replay-validator     # optional; omit to default to the adapter name
  permissions:
    read: ["$PROJECT_DIR/.specify"]
    write: []

# Shared content, declared by identity instead of symlinked into the
# framework monorepo. `specify adapter build` resolves each entry to real
# bytes under its reserved path; adapter.lock pins the digest.
requires:
  spec-runtime: "1.2.0"
  review-team-protocol: "1.0.0"
  core-rules: "3.0.0"

# The pack manifest: exactly what the published artifact contains. Source-
# only files (adapter.lock, extension crate sources, dev fixtures) are excluded.
package:
  include:
    - adapter.yaml
    - briefs/**
    - references/**
    - rules/**
    - extension/module.wasm
```

**Lockfile — `adapter.lock`** pins every resolved digest so the vendored tree is byte-reproducible. It is not the rejected downstream Merkle: it pins the *inputs* an author vendors (so authoring is reproducible), whereas the registry content digest (D4) proves the *published output* — inputs pinned upstream of pack, output verified downstream of pull, neither re-deriving identity from a checkout:

```yaml
# adapters/targets/omnia/adapter.lock
# Generated by `specify adapter build`; committed. `build --check` fails
# with `adapter-vendor-stale` when the tree drifts from this lock.
version: 1
requires:
  spec-runtime:
    version: "1.2.0"
    digest: "sha256:9f2b…c1"
  review-team-protocol:
    version: "1.0.0"
    digest: "sha256:4ad0…7e"
  core-rules:
    version: "3.0.0"
    digest: "sha256:bb71…02"
# The extension carries no independent version — it rides the adapter's semver
# (RFC-47); the lock pins only the built wasm digest for reproducibility.
extension:
  digest: "sha256:1c3e…aa"
```

**Pipeline.** `specify adapter build` is one step with three stages, mirroring how `cargo build` compiles *and* links rather than handing linking back to the caller. First it resolves each `requires` entry against `adapter.lock`, verifies the pinned digest, and writes the reserved trees — toolchain-free. Then, *only when an extension is declared and the committed `module.wasm` no longer matches its lock digest*, it compiles the co-located crate to `wasm32-wasip2` and refreshes the digest — the one cargo-bearing stage, skipped for prose-only or in-sync adapters. Finally it makes a deterministic tar of the `package.include` set and records the artifact digest (D4); `build --dry-run` shows the resolved tree and the would-be pack bytes without writing the artifact. Because the earlier stages land real bytes at their final paths, the pack stage is a plain tar-and-digest with no symlink dereference. `AdapterLocation::Local` resolution runs (or requires) `build` first, so a local-path adapter is self-contained like a cached one; `Local` stays digest-verify-exempt (you are editing it) while `Cached` keeps verify-on-read (D4). Publishing — the registry push — stays a separate step, the `cargo publish` to this `cargo build`.

### Shared-content dependencies (D12)

The `requires` block (D9) names shared content by versioned identity, not by a path into the framework monorepo. Each target — `spec-runtime`, `review-team-protocol`, `core-rules` — is published as its own immutable registry artifact over the D1/D6 plumbing (prose-only: no wasm, no `requires` of its own), so resolving one is the same fetch-and-verify shape as resolving an adapter.

The shared content is not free-floating. Today `spec-runtime` is the `adapters/shared/references/runtime/` hub, itself a set of symlinks into the **spec skill plugin** (`plugins/spec/references/`) and `docs/reference/` — so an adapter's `references/spec-runtime` reaches two hops into framework-owned prose. The `spec-runtime` artifact is therefore *built and published by `augentic/specify`* (which owns the spec plugin) and *consumed by adapters* via `requires`. Post-split this fixes the dependency direction: `augentic/specify` publishes spec-runtime; `augentic/specify-adapters` resolves it by identity, with no symlink crossing the repo boundary.

Two properties follow, and together they discharge the [Co-located extension source (D10)](#co-located-extension-source-d10) repo-split prerequisite:

- **The consumer never resolves `requires`.** D3 bundles the resolved bytes *into* the adapter artifact at publish, so a downstream install is one self-contained pull. `requires` is purely an author/publish-time input.
- **The author depends on an identity, not a checkout.** `specify adapter build` pulls `spec-runtime@1.2.0` from the registry into its reserved path, so an adapters tree needs no sibling `augentic/specify` source tree to resolve against. This is the severance D10 is gated on; without it, `requires` would only relocate the coupling from a build-time symlink to a vendor-time path lookup.

The alternative — resolve `requires` from the local framework checkout — is rejected: it leaves the `adapters/shared/` coupling intact under a new spelling and silently re-couples a future adapters repo to `augentic/specify`. Publishing shared content as artifacts is the only `requires` transport that makes D10's severance a fact rather than a rename.

### Co-located extension source (D10)

For an adapter that declares a wasm extension, the extension's Rust crate lives **beside the prose it serves**. Today the `contract` / `vectis` crates live in `augentic/specify-cli` under `extensions/`, far from the `contracts` / `vectis` adapter prose in `augentic/specify`. Co-location makes each adapter self-describing and lets `specify adapter build` compile the extension from the same tree it packs.

```text
adapters/targets/contracts/
  adapter.yaml                     # declares the `contract` extension in `extension` (name + permissions)
  briefs/**  references/**  rules/**   # authored prose
  extension/                          # co-located Rust crate (AUTHORED, source-only)
    Cargo.toml
    src/lib.rs
    tests/…
    module.wasm                    # reserved — BUILT, committed + digest-pinned, the only shipped byte
```

- **The reserved path is the `module.wasm` file, not the `extension/` directory.** Crate source is authored and source-only; only `module.wasm` ships. The split is one `package.include` line — `extension/module.wasm`. The built byte is committed and digest-pinned (D9), so `adapter-authored-reserved-path` exempts it and the crate source stays legitimate.
- **The extension is declared in `adapter.yaml.extension` (D11), built from the co-located crate by convention.** No `source` field — the crate at `extension/` *is* the source; its run handle is `extension.name` (defaulting to the adapter name), so `contracts` can still expose `contract`. `specify adapter build` compiles it to `wasm32-wasip2`; the extension version rides the adapter's RFC-47 semver and ships in one digest (D3). `build` compiles only when an extension is declared and its committed wasm is stale, so a prose-only adapter runs the same verb without ever needing the toolchain.
- **A sparse Cargo workspace spans the per-adapter crates.** A virtual root `Cargo.toml` (`members = ["adapters/*/*/extension"]`) carries the crates that exist; most adapters are prose-only and contribute none. This injects a Rust toolchain and `clippy`/`test` CI into the adapters tree — the discipline the `extensions` carve-out keeps separate — but only an extension-bearing, out-of-sync `specify adapter build` and that CI invoke it; resolving `requires`, the pack stage, install, and lint stay toolchain-free.

**Repo placement.** Co-location is the prerequisite shape; the repo split itself is a **committed end-state** — adapters relocate to `augentic/specify-adapters`. Two clarifications:

- **The one-PR dev loop does not depend on the move.** Editing a brief and its extension crate in one commit is enabled by co-location in a single tree (D10), true whether that tree lives in `augentic/specify` or `augentic/specify-adapters`. The dedicated repo is an **ownership, cadence, and contribution-model** decision, not a prerequisite for the authoring experience.
- **The move is gated on one hard dependency: [D12](#shared-content-dependencies-d12).** Until shared content is published as versioned artifacts, every adapter's `spec-runtime` symlink resolves into `plugins/spec/references/` and `docs/reference/`; relocating `adapters/` before that either dangles those symlinks or drags spec-plugin prose into the wrong repo. RM-21 third-party demand is a *payoff* of the move, not a *gate* — first-party extraction proceeds regardless.

`specify lint framework --framework-root .` already parameterises the framework root, so pointing the checkers at the adapters repo is configuration, not new machinery.

**Migration is sequenced and designed** — a clean cut once [D12](#shared-content-dependencies-d12) lands, consistent with *"no migration framework — pre-1.0 this is a re-init major cut"*:

- **Shared content is published from `augentic/specify` first (D12).** `spec-runtime` / `core-rules` / `review-team-protocol` are published as versioned artifacts *before* the move, built from `plugins/spec/references/` + `docs/reference/` — the load-bearing step that converts the `adapters/shared/` symlink hub into a registry identity the new repo resolves by `requires`.
- **The move carries prose + extension crates + the `build` machinery (resolve / compile / pack) and relocates the `release.yaml` publish jobs (D6).** The `--framework-root` seam makes the lint side configuration, the publish side a job move.
- **Namespace continuity holds.** The publish ref (`specify:<name>@<ver>`, D6/D8) names a registry namespace, not a source repo, so a published artifact's identity is unchanged by the source move — proven by a pull-back (D6) from the new origin.
- **Timing of the physical cut.** The **earliest safe point** is immediately after severance and co-location (D12 + D10, Phasing steps 5–6). The numbered phasing defaults to the later cut (step 8, after the shared store D5), so the full pipeline is proven once in `augentic/specify` before any relocation. A team may instead take the earliest-safe-point cut under transitional coupling (e.g. when separate owners force the repo to exist sooner); whichever variant is chosen should be recorded here when the work starts.

### CLI surface

The consumer fetch/resolve path adds **no new verbs**. Identity ([RFC-47](rfc-47-adapter-identity.md)) flows through existing paths; this RFC changes only what they fetch:

```bash
specify init omnia@1.0.0            # pulls the published artifact once, installs into the shared store
specify source survey <source>      # resolves the bound (name, version) from the shared store
specify slice build <slice>         # target resolution unchanged in shape
```

Authoring adds a single `specify adapter build` verb (D9/D10) that does all three author-time stages — resolve prose `requires`, compile any declared extension, and pack the deterministic artifact — just as `cargo build` compiles and links in one invocation. Cargo is invoked only for the compile stage and only when needed, so prose-only adapters run `build` without a toolchain. Publishing (the registry push in the release job) stays separate, the `cargo publish` to this `cargo build`:

```bash
specify adapter build               # one step: resolve prose requires → reserved trees; compile the declared extension when stale → committed extension/module.wasm + lock digest; deterministic pack of package.include → artifact + recorded digest (D4)
specify adapter build --check       # CI drift gate (read-only, no toolchain): adapter-vendor-stale if requires ≠ lock; adapter-digest-mismatch if module.wasm ≠ lock
specify adapter build --dry-run     # show the resolved requires + pack manifest (package.include + would-be digest) without writing the artifact
```

`specify archive prune` / a future `specify adapter gc` enumerates the store by `(name, version)`; cross-project reference-counting is a follow-on (see Non-Goals).

### Finding codes

| Code | Decision | Severity / kind | Raised when |
| --- | --- | --- | --- |
| `adapter-digest-mismatch` | D4 | violation (exit 2) | cached bytes (or a freshly fetched immutable locator) do not match the recorded content digest; also raised by `build --check` when a committed `module.wasm` (D10) drifts from its `adapter.lock` digest |
| `adapter-vendor-stale` | D9/D12 | violation (exit 2) | `specify adapter build --check` finds a gitignored `requires` tree (or `adapter.lock`) out of sync with the declared `requires` |
| `adapter-authored-reserved-path` | D9 | violation (exit 2) | a committed (non-vendored) file occupies a gitignored reserved `requires` path; the committed `extension/module.wasm` (D10) is exempt |
| `adapter-extension-crate-missing` | D10/D11 | violation (exit 2) | `adapter.yaml.extension` is declared but there is no co-located crate at `extension/` or no committed `module.wasm` (replaces the retired `adapter-tool` cross-reference rule) |

The `adapter-version-required` / `adapter-version-malformed` identity findings live in [RFC-47](rfc-47-adapter-identity.md).

### Test plan

- **D1** — a pack/unpack round-trip test; a **deterministic-pack** test (identical inputs → byte-identical archive across two runs); a fetch-uses-injected-fetcher test mirroring `package_source_uses_fetcher` in `resolver.rs`.
- **D2** — an `adapter_uri` test parsing the `specify:<name>@<semver>` form; a "fetch targets an immutable locator, never a branch" assertion.
- **D3** — a publish-fixture test that the artifact tree is self-contained (no dangling symlinks, spec-runtime present, the declared extension's wasm bundled); a consumer test that install performs no vendoring (`repo_root_with_runtime` never consulted downstream).
- **D4** — a verify-on-read test (corrupting a cached byte raises `adapter-digest-mismatch`); a moved-locator test (same version, different bytes → mismatch).
- **D5** — a `cache.rs` resolver test mirroring `distinct_projects_get_distinct_dirs`; a "two projects, same identity ⇒ second is a link/copy, not a re-fetch" test; a "symlink-disabled falls back to copy" test; a "store entry is read-only after install" assertion; an "interrupted install leaves no visible entry" atomic-rename test; a "concurrent projection into one project is idempotent" race test; a "per-project provenance stamp lives beside the symlink" assertion.
- **D6** — a publish-then-pull-then-verify smoke job in `release.yaml`, mirroring the `extensions` pull-back verification.
- **D9** — a `build` round-trip test (resolving `requires` writes byte-identical reserved trees matching `adapter.lock`); a `build --check` drift test asserting `adapter-vendor-stale`; a reserved-namespace guard test asserting `adapter-authored-reserved-path` for an authored file under a gitignored `requires` path **and** exempting the committed `module.wasm`; a "post-`build` tree packs to a fresh publish's digest" equivalence test; a `Local`-resolves-post-`build` test; a "`requires` reserved bytes are writable" assertion.
- **D10** — a `specify adapter build` test that a declared-extension crate compiles to a committed, digest-pinned `module.wasm`; a "`build` (including its pack stage) on a prose-only adapter and on an in-sync extension adapter, plus install and lint, succeed with no `wasm32-wasip2` toolchain" test; a "`build` recompiles only when the committed `module.wasm` drifts from its lock digest" staleness test; a `package.include` test that crate source is excluded while `module.wasm` ships; a workspace-membership test that the sparse `members` glob resolves the per-adapter crates.
- **D11** — a manifest parse test that the singular `extension` object carries an optional `name` + structured `{read, write}` `permissions` and **rejects** `version` / `source` / `sha256`; a "the plural `tools[]` array no longer parses" collapse assertion; a "flat string-array permissions no longer parse" migration assertion; an "omitted `extension.name` defaults to the adapter name" assertion; a "no `tools.yaml` is read" assertion; a "retired `adapter-tool` cross-reference rule does not fire" assertion; a `specify extension run <name>` resolution test against the installed adapter tree.
- **D12** — a `build` test that a `requires` entry resolves to a published shared-content artifact and writes byte-identical reserved bytes; a "consumer install performs no `requires` fetch" test.

`cargo make ci` (`RUSTFLAGS=-Dwarnings`) gates the consumer half; the publish job gates in `release.yaml`.

## Phasing

The order is dependency-driven: identity first, then the **transport loop** every later step publishes onto, then the **self-containment / authoring** refactor, then the **shared store**, and finally the **repo extraction**. Self-containment follows the transport it depends on — shared content cannot be published as a versioned artifact (step 5) before a deterministic pack and a publish job (step 4) exist.

1. **Transport spike — publish + pull + verify.** Probes `wkg publish`, `wasm-pkg-client` pull, and re-verify-on-read for an opaque blob; confirms (B) or clears (A). The transport loop (step 4) keys on it.
2. **D11 — manifest + extension unification.** Collapse the `tools[]` array to a singular `extension` object, replace the `toolDeclaration` `$def` with `extensionDeclaration`, rename `AdapterToolDeclaration` → `AdapterExtensionDeclaration` and unify it with `ExtensionPermissions`, retire the `tools.yaml` reader, replace the `adapter-tool` rule. Spike- and transport-independent — lands in parallel with step 1.
3. **D2 — package-ref form + immutable locator.** Teach `adapter_uri.rs` the `specify:<name>@<semver>` form and require an immutable fetch. Identity-dependent, spike-independent — parallel with steps 1–2.
4. **D1 + D6 + D4 — the transport loop.** Pack the tree, stand up the publish→pull→verify job in the spike-chosen shape (D1), mirror it as a `release.yaml` step (D6), and record-and-re-verify the digest on read (D4). **This is the transport every later step publishes onto.**
5. **D12 + D9 — `requires` resolution for prose deps.** Publish shared content as versioned artifacts *over the step-4 transport*, teach `adapter.yaml.requires` + `adapter.lock`, and write the gitignored reserved trees via the toolchain-free path of `specify adapter build` (no extension declared yet, so no cargo). This severs the `adapters/shared/` coupling — validate it before D10.
6. **D10 + D3 — co-located extension crates + extension compile + bundling.** Move the contract/vectis crates beside their prose, extend `specify adapter build` to compile the declared extension when its committed wasm is stale, commit the digest-pinned `module.wasm`, and bundle it at publish (D3). Retires the CLI `extensions` job (D7). Full adapters are now self-contained and publishable.
7. **D5 — shared store + projection.** The dedup/offline win, once identity is immutable (D4) and the install tree is byte-stable (steps 5–6). The one transport-side piece that legitimately lands late: author-time `build` (step 5) fetch-and-verifies the `requires` deps without it.
8. **Extract adapters to `augentic/specify-adapters` (D7 / D10).** Prerequisites met (step 5 severed the coupling, step 6 co-located the crates — a team may cut at that earlier point). Relocate the adapter trees, the `build` machinery (resolve / compile / pack), and the `release.yaml` publish jobs out of `augentic/specify`; move the framework-lint job (`--framework-root`) and publish credentials. This completes *this* RFC's half of the two-repo end-state; [RFC-49](rfc-49-repository-topology.md) then folds `augentic/specify` + `augentic/specify-cli` into one platform repo afterward.

## Alternatives considered

- **Keep git transport; re-derive a canonical tree digest downstream guarded by a bespoke atomic-publish protocol.** Rejected — it content-addresses the *symptom*. The downstream Merkle, the digest-after-vendoring dance, and the bespoke protocol exist only because a git ref is a *moving* locator; an immutable registry digest (D2/D4) collapses the lot to a one-line verify.
- **Wrap each adapter as a wasm component.** Rejected — adds a build step for prose and buys nothing at runtime (execution stays agent-only), and does not protect the prose (`strings`-able, must still reach the model as cleartext).
- **Client-side prose expansion — thin briefs that expand from bundled wasm at point of use.** Rejected — protects nothing, collides with the *"CLI never reads brief bodies"* contract, and breaks the `lint framework` checkers that parse brief prose (`links-registry`, `prose`, `brief-schema-link-resolve`). Salvageable kernels are under [Non-Goals](#non-goals).
- **Prose-only artifact; resolve the declared extension separately.** Deferred — bundling (D3) keeps one pull and one digest. Revisit only if extension churn meaningfully outpaces adapter churn.
- **Key the store by `(name, major)`.** Rejected — a major spans infinite commits; sharing it yields first-fetch-wins drift. The store keys on the full `(name, version)`.
- **A global resolution fallback by name.** Rejected — reintroduces the ambient mutable-namespace footgun [DECISIONS.md §"Resolution is project-local only"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#adapter-loader-axis-routing) deliberately removed. The store is storage, not a fallback.
- **Hardlink the per-project projection (D5).** Rejected — shared inodes mean an accidental write through the cache would mutate the store, and they break across filesystems. A read-only store entry plus a symlink (copy fallback) keeps the store immutable and the failure mode loud.
- **A `src/` → `dist/` authoring split.** Rejected — it makes the authoring tree *not* resemble the runtime tree, so `Local` and `Cached` would need divergent resolution and authored brief links would not match what ships. D9 mirrors the unpacked structure in place.
- **Commit the vendored bytes into git (Go-vendor posture).** Rejected for the prose `requires` trees — noisy diffs and silent drift on hand-edit, so D9 gitignores them and pins digests with `build --check` as the CI gate. The one exception is the built `extension/module.wasm` (D10): a binary blob has no diff to be noisy, the `extensions/contract/dist` precedent already commits one, and committing it keeps packing / install / lint toolchain-free.
- **Resolve `requires` from the local framework checkout.** Rejected ([D12](#shared-content-dependencies-d12)) — it leaves the `adapters/shared/` coupling intact under a vendor-time path lookup and silently re-couples a future adapters repo to `augentic/specify`.
- **A separate `vendor` verb beside `build`.** Rejected — the toolchain concern that would motivate a split is better handled *inside* `build`, which invokes cargo only when an extension is declared and its committed wasm is stale. Prose-only adapters (6 of 8) and in-sync extension adapters run the same `build` and never touch the toolchain, so a second verb buys no isolation and only fragments the authoring loop. `build` resolves `requires`, (conditionally) compiles the extension, and packs the artifact in one step; publishing (the release job's registry push) stays separate.
- **Keep extension source in the CLI's `extensions/` workspace (status quo).** Rejected — the crate sits in a different repo from the prose it serves, so authoring spans two repos and `build` cannot compile the extension from the tree it packs. D10 co-locates the crate at `extension/`.
- **Extract adapters into a dedicated repo *before* D12.** Rejected — relocating `adapters/` while `spec-runtime` is still a symlink hub into `plugins/spec/references/` + `docs/reference/` either dangles those symlinks or drags spec-plugin prose into the wrong repo. The extraction is committed (D7/D10, step 8) but sequenced *after* D12.
- **Keep the standalone `tools.yaml` sidecar (or the plural `tools[]` array).** Rejected — once the wasm is bundled (D3), versioned by the adapter's semver ([RFC-47](rfc-47-adapter-identity.md)), and covered by the content digest (D4), a per-extension `version` / `source` / `sha256` is redundant, and no adapter has ever declared more than one extension. D11 collapses the array to a singular `extension` object (optional `name`, structured `permissions`) — a breaking shape change (see [D11](#design)).

## Non-Goals

- **Adapter identity** — the semver `version` and the `AdapterRef` resolve signature are [RFC-47](rfc-47-adapter-identity.md).
- The hosted registry/publish *index* (discovery, search, release feed), semver **range** resolution (`^1.0`, `~1.2`), third-party namespacing (`org/name@req`), and *range-based* `specify`-floor policy — RM-21 (the exact-floor `specify` guard is [RFC-47](rfc-47-adapter-identity.md) D3). Pull-side auth and visibility on the existing `augentic.io` namespace are **in scope** here (D8).
- Cross-project reference counting and GC of the shared store beyond a simple `(name, version)` enumeration.
- **Per-licensee watermarking** — deferred. Attribution and breach-traceability, not prevention; it rides on top of D1 unchanged if a business need lands.
- **Server-side / hosted prose expansion** — out of scope. The only mechanism that actually withholds prose from the consumer, but it contradicts the self-contained / offline principle and is a hosted-product concern.
- **Opening the adapters repo to third-party authors** — [RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model). The *first-party* extraction (D7/D10, step 8) **is** in scope; only outside authorship is deferred.
- Any migration framework — pre-1.0 this is a re-init major cut.

## References

- `crates/registry/src/package.rs` (`fetch`, `load_config`, `AcquiredBytes`) — the wasm-pkg fetch/verify path D1 reuses or parallels.
- `crates/registry/src/resolver.rs` and `crates/registry/src/cache/fetch.rs` (`stage_and_install`) — the stage-temp-then-atomic-install precedent D5 mirrors.
- `crates/extension-manifest/src/lib.rs` (`ExtensionSource`, `PackageRequest`, `DEFAULT_WASM_PKG_CONFIG`, `WASM_PKG_CONFIG_PATH`) — the package-ref shape and layered registry config D2 extends.
- `.github/workflows/release.yaml` (`extensions` job) and [`docs/release.md`](https://github.com/augentic/specify-cli/blob/main/docs/release.md) — the publish loop D6 mirrors.
- `crates/workflow/src/init/{adapter_uri,git,cache}.rs` — the current git-sparse-checkout install path D1/D2/D3 replace, and the `vendor_spec_runtime` / `ManifestMeta` D3/D4 relocate to publish.
- `crates/schema/src/cache.rs` (`mirror_dir`, `project_cache_dir`) — the cache-root precedent D5 extends.
- `crates/workflow/src/plan_lock.rs` — the `File::try_lock` flock primitive reused for the install rename (D5).
- `crates/schema/src/digest.rs` (`Hasher`) — the incremental hasher D4 verifies with.
- `extensions/{contract,vectis}/` (sibling workspace in `augentic/specify-cli`) — the current out-of-adapter extension-source location D10 co-locates.
- [RFC-47: Adapter identity](rfc-47-adapter-identity.md) — the semver identity this RFC distributes.
- [Roadmap RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model) — the ecosystem item both RFCs serve.
