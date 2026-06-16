# RFC-48: Adapter Packaging and Transport — OCI / wasm-pkg Distribution

> Status: Draft · Execution order: **2nd of RFC-47 → RFC-48 → RFC-49**, executed to completion in numerical sequence. Runs after identity ([RFC-47](rfc-47-adapter-identity.md)) lands; self-contained against today's two-repo platform. The consolidation half ([RFC-49](rfc-49-repository-topology.md)) runs *afterward* and is not a precondition of any step here. · Depends: [RFC-47: Adapter identity](rfc-47-adapter-identity.md) (the semver identity this RFC distributes), the wasm-pkg extension-distribution precedent (`crates/registry/src/{package,resolver}.rs`, `crates/registry/src/cache/fetch.rs`, the `extensions` job in `.github/workflows/release.yaml`), the adapter loader and install path (`crates/workflow/src/init/{adapter_uri,git,cache}.rs`), the per-project cache resolver (`crates/schema/src/cache.rs`) · Related: [RFC-49: Repository topology](rfc-49-repository-topology.md) · Roadmap: the distribution portion of [RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model).

## Abstract

An adapter is published, fetched, verified, and cached as an **immutable registry artifact**, over the same wasm-pkg / OCI plumbing first-party WASI extensions already use. One package carries both the adapter's **prose** (`adapter.yaml`, briefs, references) and its **wasm** (an adapter ships at most one declared extension), so `omnia@1.0.0` is one pull — not a git sparse checkout plus a separate extension fetch.

Identity ([RFC-47](rfc-47-adapter-identity.md)) is a semver; this RFC binds it to an immutable, content-addressed locator and proves it on read with the **registry's own content digest**, not a bespoke downstream Merkle. Because the registry already supplies immutability and content-addressing, the shared cache is an ordinary download-once-by-identity store.

The authoring tree mirrors that unpacked artifact: an author writes prose into a structure shaped like the installed store entry, declares which shared bundles it needs (the `spec` bundle, shared rules) and its wasm extension inline in `adapter.yaml`, and runs `specify adapter build` — a single step that copies those bundles in from the repo's own `shared/` tree, compiles any declared extension, and packs the deterministic artifact (the way `cargo build` compiles *and* links) — so the local tree is byte-identical to what ships (D9/D10). Self-containment becomes a locally reproducible fact, not a publish-only transform.

## Motivation

[RFC-47](rfc-47-adapter-identity.md) fixes what an adapter is *named*; this RFC fixes how the bytes behind that name travel and how they are *proven*. The registry is the natural transport:

- **We already run the loop.** The `extensions` job in `release.yaml` publishes `wasm32-wasip2` components to the `augentic.io` (GHCR-backed) registry via `wkg`; `crates/registry/src/{package,resolver}.rs` fetch, stream, sha256-verify (`specify_schema::digest::Hasher`), and atomically install them. Adapter distribution is the same loop with a tree payload instead of one blob.
- **Adapters are trees, not blobs.** The extension path installs exactly one `adapter.wasm`; an adapter is a directory of prose plus optional wasm (see [Background](#background)). The gap this RFC closes is *one blob → a packed tree*.
- **The registry gives immutability and a digest for free.** Today's transport is a git sparse checkout (`init/git.rs`) copied into the per-project cache (`init/cache.rs`). A git ref is a *moving* locator — proving immutability against it needs a bespoke publish-time Merkle plus a moved-tag backstop. An OCI content digest *is* immutable identity, so that machinery collapses to verifying the registry descriptor.

## Background

### What an adapter is, and isn't

A WASI extension is one wasm component — a single blob with a single digest, which is why the existing fetch path (`crates/registry/src/package.rs`) installs one `adapter.wasm` and is done. An adapter is a **directory tree**:

- `adapter.yaml` — the prose manifest the loader validates.
- `briefs/*.md` — prose the *agent* reads and acts against; never run as code.
- `references/`** — supporting prose plus the copied-in `references/spec/` bundle.
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
- **Artifacts are self-contained.** Everything an adapter needs at resolution time — the `spec` bundle, the declared extension — is bundled at publish. Downstream resolution does no vendoring and dereferences no in-tree symlinks; the installed tree *is* the published tree.
- **Authoring mirrors the artifact.** The authoring tree is shaped like the unpacked store entry, so `AdapterLocation::Local` and `AdapterLocation::Cached` resolve through one code path and what an author lints and tests is what ships. Shared content is a declared, in-repo bundle copied in by a local `build` step (D9), never a symlink into a framework checkout.
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


| ID                                                            | Decision                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Implementation consequence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **D1 Packaging format**                                       | An adapter publishes as one immutable registry artifact carrying the packed prose tree plus bundled wasm. Working-default shape: an **OCI artifact with layers** (prose layer + a wasm layer) via the already-vendored `oci-client` / `oci-wasm`; optimisation shape (only if the spike clears an opaque blob in *both* directions): a **packed-tree blob** (`adapter.tar.zst`). `specify adapter build`'s pack stage is byte-deterministic either way.                                                                                                                                                                                                                                                                | Spike-gated (see [Prerequisite spike](#prerequisite-spike)). OCI adds a sibling fetch module reusing registry/auth/namespace config; the blob shape generalises `crates/registry/src/package.rs` to stream-and-unpack a tree. See [Packaging shapes (D1)](#packaging-shapes-d1).                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| **D2 Immutable fetch locator**                                | Fetch targets an **immutable, content-addressed locator** (OCI `@sha256:` digest, or an immutable tag whose digest is recorded), never a branch.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | `init/adapter_uri.rs` gains a package-ref form (`specify:omnia@1.0.0`) alongside the local-path / GitHub-URL / shorthand forms, with no branch-ref defaulting. The recorded digest (D4) backstops a moved tag as `adapter-digest-mismatch`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| **D3 Self-contained artifact**                                | The `spec` bundle and the wasm of the declared extension are bundled **at publish**; downstream resolution does no vendoring and dereferences no symlinks.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | The legacy `vendor_spec_runtime` ancestor-walk (`init/cache.rs`) retires; the `spec` bundle is copied from the repo's own `shared` tree at author time ([D12](#shared-content-dependencies-d12)) and `build` packs it into the artifact. The declared extension's `adapter.wasm` ships inside the artifact. Because `build` (D9/D10) lands the same bytes at author time, the local working tree is self-contained too. See [Bundled extension (D3)](#bundled-extension-d3).                                                                                                                                                                                                                                                           |
| **D4 Identity via registry content digest, verified on read** | The artifact's identity is the registry content digest (`sha256:`). The consumer records it on install and re-checks it on every read, refusing a mismatch. Re-publishing an existing `(name, version)` with different bytes is rejected **at publish**.                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Verification reuses the streaming `specify_schema::digest::Hasher` in `package.rs`. The bespoke publish-time tree Merkle is **not** built — the registry descriptor is the trust anchor. The digest is recorded in the store entry's install metadata (the `ManifestMeta` role, now keyed to the immutable entry rather than stamped per-project).                                                                                                                                                                                                                                                                                                                                                                 |
| **D5 Trivial global store, resolved in place**                | A global store at `<adapters-root>/<name>@<version>/`, resolved `$SPECIFY_ADAPTER_CACHE` → `$XDG_CACHE_HOME/specify/adapters` → `$HOME/.cache/specify/adapters` → `<temp>/specify/adapters` (the `mirror_dir` precedent). Install = pull → temp → verify → atomic rename → `chmod` read-only. A `Cached` adapter resolves **directly to its store entry** by pinned `(name, version)` — the Cargo `~/.cargo/registry` model — with **no per-project symlink or copy**.                                                                                                                                                                                                                                                 | New store resolver in `crates/schema/src/cache.rs`; `locate_axis` threads the pinned version and returns `<store>/<name>@<version>/` for `AdapterLocation::Cached(PathBuf)` (the `Local` in-tree probe is unchanged). Resolution stays project-local: `name` resolves to the project's *pinned* identity, never a global-by-name latest. See [Store layout and resolution (D5)](#store-layout-and-resolution-d5).                                                                                                                                                                                                                                                                                                  |
| **D6 Publish tooling**                                        | A publish step mirrors the `extensions` release job: pack the tree (+ bundled wasm), push `specify:<name>@${VERSION}`, pull back and verify.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | New job in `.github/workflows/release.yaml`, reusing the `wkg` / GHCR / `specify -> augentic.io` namespace plumbing the extension job exercises.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| **D7 Adapter repo extraction**                                | **Adapters — prose plus co-located extension crates — relocate to a dedicated `augentic/specify-adapters` repo**, extracted from `augentic/specify` once the shared content forks into the adapters repo ([D12](#shared-content-dependencies-d12); a one-time clean copy at the split — Phasing step 8). The *other* half — folding `augentic/specify` + `augentic/specify-cli` into one platform repo — is [RFC-49](rfc-49-repository-topology.md), executed afterward and **not** a precondition. First-party extension wasm relocates with the adapters: contract/vectis ship inside the adapter artifact (D3/D6/D11), so the `extensions` publish job retires.                                                                                                    | Touching the adapter loader / cache scope requires the cross-repo `rg` sweep per [AGENTS.md §"Note to the implementing agent"](https://github.com/augentic/specify-cli/blob/main/AGENTS.md); net of [RFC-49](rfc-49-repository-topology.md) the sweep is **two-way** (platform ↔ adapters). Retiring `extensions` removes the standalone `specify:contract@x` / `specify:vectis@x` packages, the `first_party_permissions` catalog, and the scalar first-party `tools:` form — decommissioned, not aliased.                                                                                                                                                                                                        |
| **D8 Registry visibility and pull auth**                      | First-party adapter artifacts publish to an **authenticated** namespace; pulling requires credentials, gating *who* obtains the bytes. A public namespace is a deliberate per-adapter opt-out, not the default.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Pull-side auth reuses the wasm-pkg / GHCR credential path the publish step (D6) exercises — layered config in `crates/registry/src/package.rs::load_config` and `.specify/wasm-pkg.toml`. No new transport.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| **D9 Authoring mirrors the unpacked artifact**                | The authoring tree is shaped like the unpacked store entry: authored prose (`adapter.yaml`, `briefs/`, `references/`, `rules/`) plus a **reserved vendored namespace** an author never hand-writes. Shared content is declared by name in `adapter.yaml.requires` (a versionless selector); the declared-extension wasm builds from a co-located crate declared in `adapter.yaml.extension` (D11). `specify adapter build` copies each named `requires` bundle from the repo's `shared` tree into the reserved paths, compiles any declared extension, and packs the artifact in one step; the pack set is the adapter directory minus the declared `extension/` source and a built-in dev/VCS exclude set.                                                                                                                                      | Adds an optional `requires` block to `schemas/{source,target}.schema.json`; adds the author-time `specify adapter build` verb (with `--check` / `--dry-run`). Hand-authoring a gitignored reserved path raises `adapter-authored-reserved-path`; a reserved tree out of sync with the repo's `shared` tree raises `adapter-vendor-stale`. See [Authoring structure (D9)](#authoring-structure-d9).                                                                                                                                                                                                                                                                                                                      |
| **D10 Co-located extension source**                           | An adapter's declared-extension Rust crate lives **beside its prose** at `extension/`. The built `adapter.wasm` sits at the adapter root (not under `extension/`); it is the only shipped byte and is **committed** (git content-addresses the blob), mirroring the committed-byte half of the existing `extensions/contract/dist/*.wasm` precedent — so packing, install, and lint need no wasm toolchain; only *refreshing* the byte does. A sparse Cargo workspace spans the per-adapter crates.                                                                                                                                                                                                                                  | `specify adapter build` compiles the crate to `wasm32-wasip2` and writes the root `adapter.wasm` — but **only when an extension is declared and the committed wasm is absent (or an explicit `--refresh-extension` is requested)**, so prose-only adapters (6 of 8 today) share the verb and never invoke cargo. the root `adapter.wasm` ships the artifact while the entire `extension/` crate source is excluded from the pack by convention. The Rust toolchain + `clippy`/`test` CI enters the adapters tree only for extension-bearing crates, scoped like the `extensions` carve-out. See [Co-located extension source (D10)](#co-located-extension-source-d10).                                                 |
| **D11 Extension declaration in the manifest**                 | There is no standalone `tools.yaml`, and an adapter ships **at most one binary**. The plural `adapter.yaml.tools[]` array collapses to a singular `adapter.yaml.extension` object carrying an optional `name` (the run handle, defaulting to the adapter name) and structured WASI `permissions`. **Breaking** change: the array→object collapse, plus the per-extension `version` moving from *required* to *rejected* (the wasm rides the adapter's semver ([RFC-47](rfc-47-adapter-identity.md)), covered by D4) and `permissions` moving from a flat string array to the `{read, write}` shape the WASI runner already speaks. `source` / `sha256` never appear — the wasm builds from the co-located crate (D10). | Replaces the `tools[]` array (and its `toolDeclaration` `$def`) with a singular `extension` object (an `extensionDeclaration` `$def`) in `schemas/{source,target}.schema.json`. Renames `AdapterToolDeclaration` → `AdapterExtensionDeclaration` (`adapter/core.rs`), collapses it to an `Option<…>`, and unifies it with `specify_extension_manifest::ExtensionPermissions` while preserving the wasmtime-free loader boundary, retires the `tools.yaml` reader (`load::plugin_sidecar`), and **replaces** the now-moot `adapter-tool` cross-reference rule with a co-located-crate / built-`adapter.wasm` presence check. `specify extension run <name>` resolves the extension from the installed adapter tree. |
| **D12 In-repo shared content**                                | The `requires` targets (`spec`, `review-team-protocol`, `core-rules`) are **bundles that live in the adapters repo's own tree**, not external artifacts. `specify adapter build` copies each named bundle from the repo's canonical `shared` location into the reserved path at **author/build time**; the consumer never resolves `requires`, because D3 packs the copied bytes. No registry transport, no version pins, and no reference outside the repo.                                                                                                                                                                                                                                                 | Replaces the `adapters/shared/` symlink hub (`init/git.rs`) and the `vendor_spec_runtime` ancestor-walk (`init/cache.rs`) with an in-repo copy step in `build`. At the repo split the adapter-needed subset is clean-copied into `specify-adapters/shared/` once; each repo owns its copy thereafter. See [Shared-content dependencies (D12)](#shared-content-dependencies-d12).                                                                                                                                                                                                                                                                                                                               |


### Packaging shapes (D1)

The [design space](#three-ways-to-put-a-tree-in-the-registry) is three shapes (A / B / C); the spike picks between (A) and (B), and (C) is recorded-but-rejected. The working assumption is **(B)**, which the spike may overturn in favour of (A):

- **(B) OCI artifact with layers — working default.** A prose layer plus a single wasm layer, over the already-transitive `oci-client` / `oci-wasm`. Each is independently content-addressed (serving D3's bundle and D4's verify-on-read directly), and an extension-only rebuild re-pushes just the wasm layer, leaving the prose layer untouched.
- **(A) packed-tree blob — optimisation if the spike clears it.** One `adapter.tar.zst` streamed through ~90% of `crates/registry/src/package.rs`. Cheaper *only if* `wasm-pkg` carries an opaque blob in both directions; otherwise its apparent simplicity is lost to a parallel publish path.
- **(C) wrap-as-component is not pursued** — a prose build step that buys nothing at runtime.

**Verify-on-read differs by shape (D4).** Under (B) the consumer re-checks the cached layer descriptors it already holds — no re-tar. Under (A) re-hash-on-read must either re-tar (requiring a byte-deterministic pack) or retain the tarball past install. (B) avoids both.

**Pack must be byte-deterministic.** D4 rejects re-publishing the same `(name, version)` with different bytes, and D9's "post-`build` tree packs to the publish digest" equivalence depends on identical inputs producing an identical archive. `tar` + `zstd` are non-deterministic by default, so `build`'s pack stage normalises entry order, mtimes, uid/gid, and permission bits and pins compression parameters. Under (B) this narrows to the single prose layer.

### Bundled extension (D3)

An adapter that declares a wasm extension (D11) ships its `adapter.wasm` *inside* the artifact, so one pull is self-contained and one digest covers prose + wasm. The alternative — a prose-only artifact with the extension resolved separately — lets an extension bump avoid republishing the adapter, but reintroduces a second fetch and a second digest. v1 bundles; a split-extension-channel is a deferred optimisation if extension churn outpaces adapter churn.

### Store layout and resolution (D5)

```text
$XDG_CACHE_HOME/specify/adapters/
  omnia@1.0.0/                  # immutable, digest-verified, read-only
    adapter.yaml
    adapter.wasm                # bundled at publish (D3)
    briefs/…
    references/spec/…           # bundled at publish (D3)
  documentation@1.0.0/
```

- **The store is CLI-write-only.** The fetch path installs pristine bytes; the agent only ever *reads* the resolved store entry, which is `chmod`-read-only after install.
- **Install is pull → temp → verify → atomic rename → `chmod` read-only.** The temp dir lives under the store root so the rename is atomic on one filesystem. Because identity is immutable, concurrent installs are idempotent: one wins the rename, the other verifies the matching digest and discards its temp. A flock around the rename suffices, reusing the `File::try_lock` family from `plan_lock.rs` and the staged-install precedent in `crates/registry/src/cache/fetch.rs`. The rename is the *only* concurrency point — there is no second per-project step to race.
- **A `Cached` adapter resolves directly to its store entry.** `locate_axis` computes `<store>/<name>@<version>/` from the project's pinned identity and reads it in place — the Cargo `~/.cargo/registry` model — so there is **no per-project symlink, no recursive copy, and no Windows-symlink-privilege / cross-device failure mode** to fall back from. `AdapterLocation::Cached(PathBuf)` simply points at the store entry; the `Local` in-tree probe (`adapters/<axis>/<name>/`) is unchanged, so authoring and consuming still read through one `load_validated` path.
- **The recorded digest is a property of the entry, not the project.** D4's verify-on-read re-hashes the store entry against the digest recorded *at install* in the entry's own install metadata — identical bytes for every project, so it lives once with the entry rather than as a per-project provenance stamp. Which `(name, version)` a project uses already lives in its `sources.yaml` / `targets.yaml` / `plan.yaml` pins.
- **The store is unbounded by design.** Cross-project reference-counting is deferred ([Non-Goals](#non-goals)). Acceptable because entries are small, immutable, and content-addressed — `specify adapter gc` / `archive prune`-style retention is the eventual home, not a v1 blocker.

### Authoring structure (D9)

The authoring tree is shaped like the unpacked store entry, so the loader resolves a `Local` working tree and a `Cached` artifact through one code path, brief-relative links resolve in place, and `specify lint framework` runs against the bytes that ship. It is a *complete* mirror only after `specify adapter build` populates the reserved namespace.

Two consequences for `specify lint framework`. First, after D9 there are no symlinks under an adapter — the reserved `requires` trees are gitignored real files — so the §F1 walk in `index/framework.rs` simplifies to an ordinary recurse and the symlink-target-removal gotcha retires. Second, a fresh clone is not a complete mirror until `build` runs, so the lint bootstrap (`make lint`) chains `specify adapter build` first, exactly as `init --workspace` chains an initial sync. Because the committed `adapter.wasm` (D10) is already present on a fresh clone, that bootstrap `build` recompiles nothing — it copies the `requires` bundles from the repo's `shared/` tree and stays toolchain-free.

```text
adapters/targets/omnia/            # authored tree == unpacked store entry (post-build)
  adapter.yaml                     # authored — manifest + requires
  adapter.wasm                     # reserved — built wasm, committed; git content-addresses it  (NOT gitignored)
  briefs/{shape,build,merge}.md    # authored prose
  references/                      # authored prose you own …
    guardrails.md  providers/**  examples/**
    spec/                          # reserved — copied from shared/spec               (gitignored)
    agent-teams.md                 # reserved — copied from shared/agent-teams.md     (gitignored)
  rules/
    omnia.mdc  provider-only-host-access.md   # authored
    core/                          # reserved — vendored CORE-* rules                  (gitignored)
    universal/                     # reserved — vendored UNI-* rules                   (gitignored)
  extension/                       # co-located Rust crate (authored, source-only — excluded from pack)
    Cargo.toml  src/               # authored — extension.name may differ from the adapter name
```

**Reserved vendored namespace.** Vendored content only ever lands at a fixed, documented set of paths an author never hand-writes, so the authored ⊎ vendored union is unambiguous and the gitignore set is exact:


| Declared dependency             | Reserved path                     | Posture                                                                  |
| ------------------------------- | --------------------------------- | ------------------------------------------------------------------------ |
| `requires.spec`                 | `references/spec/`                | gitignored, regenerated by `build`                                       |
| `requires.review-team-protocol` | `references/agent-teams.md`       | gitignored, regenerated by `build`                                       |
| `requires.core-rules`           | `rules/core/`, `rules/universal/` | gitignored, regenerated by `build`                                       |
| `adapter.yaml.extension`        | `adapter.wasm` (adapter root)     | **committed**, built by `adapter build`, content-addressed by git        |


The namespace has two postures. The three `**requires` trees** are gitignored and regenerated by `build`; a committed file under one raises `adapter-authored-reserved-path`, and `build --check` (CI) catches stale regenerated bytes as `adapter-vendor-stale`. The **built root `adapter.wasm`** is the exception: it is *committed* (git content-addresses the blob), mirroring the committed-byte half of the `extensions/contract/dist` precedent so packing / install / lint never need a wasm toolchain — `adapter-authored-reserved-path` exempts it. With no lock to drift against, `build --check` makes no claim about the committed wasm; its integrity rides git review and history.

Unlike the D5 store, the author tree is human-owned, so vendored `requires` bytes are written **writable**: the `build --check` re-copy, not a filesystem bit, guards them — read-only files inside a working tree fight `git checkout` / `clean` and editor saves for no integrity gain over re-copying from the repo's `shared/` tree.

**Manifest — `adapter.yaml*`* gains a `requires` block (shared bundles named by a versionless selector); there is no pack manifest — the pack set is convention-derived (see below):

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

# Shared bundles this adapter includes, copied from the repo's own shared/
# tree by `specify adapter build` instead of symlinked into framework prose.
# No versions: the shared bundles and the adapters live in one repo and move
# together at a single commit (D12).
requires:
  - spec
  - review-team-protocol
  - core-rules

# There is no `package:` field. `specify adapter build` packs the whole
# adapter directory, minus the co-located extension/ crate source (declared
# above) and a built-in dev/VCS exclude set (.git, target/, editor scratch).
```

**Reproducibility needs no lockfile.** There is deliberately no `adapter.lock`. The shared bundles are copied from the repo's own `shared/` tree, and the committed root `adapter.wasm` is content-addressed by git — so a single commit of the adapters repo fully determines every byte the adapter ships. There is no external version to pin and no resolution to record: `requires` names in-repo bundles, not versioned artifacts. `build --check` diffs the reserved trees against `shared/` (`adapter-vendor-stale`); it consults no lock.

**Pipeline.** `specify adapter build` is one step with three stages, mirroring how `cargo build` compiles *and* links rather than handing linking back to the caller. First it copies each named `requires` bundle from the repo's `shared/` tree into the reserved paths — a plain in-repo file copy, toolchain-free. Then, *only when an extension is declared and the committed `adapter.wasm` is absent (or an explicit `--refresh-extension` is requested)*, it compiles the co-located crate to `wasm32-wasip2` and rewrites the committed byte — the one cargo-bearing stage, skipped whenever the wasm is already present (and always for prose-only adapters). Finally it makes a deterministic tar of the pack set — the adapter directory minus the declared `extension/` source and a built-in dev/VCS exclude set — and records the artifact digest (D4); `build --dry-run` shows the resolved tree and the would-be pack bytes without writing the artifact. Because the earlier stages land real bytes at their final paths, the pack stage is a plain tar-and-digest with no symlink dereference. `AdapterLocation::Local` resolution runs (or requires) `build` first, so a local-path adapter is self-contained like a cached one; `Local` stays digest-verify-exempt (you are editing it) while `Cached` keeps verify-on-read (D4). Publishing — the registry push — stays a separate step, the `cargo publish` to this `cargo build`.

### Shared-content dependencies (D12)

The `requires` block (D9) names shared content by **bundle name** — a versionless selector, because the shared bundles and the adapters live in **one repo** and move together at a single commit. `specify adapter build` resolves each name to the repo's own copy of that bundle and copies it into the reserved path; there is no registry artifact, no version pin, and no reference to anything outside the repo.

The shared content has a single in-repo home. Today the `spec` bundle is the `adapters/shared/references/runtime/` hub — symlinks into the **spec skill plugin** (`plugins/spec/references/`) and `docs/reference/` — so an adapter's `references/spec` reaches into framework-owned prose. `build` replaces those symlinks with a copy from the repo's canonical locations into the gitignored reserved path. Because the copy is in-repo, there is no cross-repo coupling to engineer: the dependency is a file copy within one tree.

- **The consumer never resolves `requires`.** D3 packs the copied bytes *into* the adapter artifact at publish, so a downstream install is one self-contained pull. `requires` is purely an author/build-time selector.
- **Single source within the repo, no committed duplication.** The canonical bundle exists once; each adapter's reserved copy is gitignored and regenerated by `build`. Editing the canonical and rebuilding fans the change out — `build --check` catches an adapter copy that drifts from the canonical (`adapter-vendor-stale`).

**At the repo split, the shared content forks once.** When adapters relocate to `augentic/specify-adapters`, the adapter-needed subset is **clean-copied** from `plugins/spec/references/` + `docs/reference/` into `specify-adapters/shared/`, and `build` re-points at `shared/`. After the fork each repo owns its copy with no ongoing sync — the platform's copy serves the spec workflow, the adapters' copy serves adapter briefs, and they are free to diverge. This is the severance D10's repo split is gated on: not a registry identity, but an in-repo `shared/` tree that needs no sibling `augentic/specify` checkout. Scope the fork to exactly what adapter briefs link to, to keep the duplication minimal.

Publishing shared content as versioned registry artifacts was considered and rejected ([Alternatives](#alternatives-considered)): it builds cross-repo dependency transport — a prose-only publish job, a registry resolver, version pins, a drift lock — for a dependency that never crosses a repo boundary once the `shared/` tree lives in the adapters repo.

### Co-located extension source (D10)

For an adapter that declares a wasm extension, the extension's Rust crate lives **beside the prose it serves**. Today the `contract` / `vectis` crates live in `augentic/specify-cli` under `extensions/`, far from the `contracts` / `vectis` adapter prose in `augentic/specify`. Co-location makes each adapter self-describing and lets `specify adapter build` compile the extension from the same tree it packs.

```text
adapters/targets/contracts/
  adapter.yaml                     # declares the `contract` extension in `extension` (name + permissions)
  adapter.wasm                     # reserved — BUILT, committed (git content-addressed), the only shipped byte
  briefs/**  references/**  rules/**   # authored prose
  extension/                          # co-located Rust crate (AUTHORED, source-only — excluded from pack)
    Cargo.toml
    src/lib.rs
    tests/…
```

- **The shipped wasm sits at the adapter root (`adapter.wasm`); `extension/` is source-only.** Crate source is authored and never ships, so the *entire* `extension/` directory is excluded from the pack (the declared source dir is one of the convention's two exclusions, alongside the dev/VCS set) while the root `adapter.wasm` ships — no reaching into a source directory for one file, and no need to ignore the crate's `target/` while committing a byte beside it. The built `adapter.wasm` is committed (D9; git content-addresses it), so `adapter-authored-reserved-path` exempts it and the crate source stays legitimate.
- **The extension is declared in `adapter.yaml.extension` (D11), built from the co-located crate by convention.** No `source` field — the crate at `extension/` *is* the source; its run handle is `extension.name` (defaulting to the adapter name), so `contracts` can still expose `contract`. `specify adapter build` compiles it to `wasm32-wasip2`; the extension version rides the adapter's RFC-47 semver and ships in one digest (D3). `build` compiles only when an extension is declared and its committed wasm is absent (or an explicit `--refresh-extension` is requested), so a prose-only adapter — or any adapter whose committed wasm is already present — runs the same verb without ever needing the toolchain.
- **A sparse Cargo workspace spans the per-adapter crates.** A virtual root `Cargo.toml` (`members = ["adapters/*/*/extension"]`) carries the crates that exist; most adapters are prose-only and contribute none. This injects a Rust toolchain and `clippy`/`test` CI into the adapters tree — the discipline the `extensions` carve-out keeps separate — but only a `specify adapter build` that must actually compile (an extension is declared and its committed wasm is absent or a refresh is forced), plus that CI, invoke it; resolving `requires`, the pack stage, install, and lint stay toolchain-free.

**Repo placement.** Co-location is the prerequisite shape; the repo split itself is a **committed end-state** — adapters relocate to `augentic/specify-adapters`. Two clarifications:

- **The one-PR dev loop does not depend on the move.** Editing a brief and its extension crate in one commit is enabled by co-location in a single tree (D10), true whether that tree lives in `augentic/specify` or `augentic/specify-adapters`. The dedicated repo is an **ownership, cadence, and contribution-model** decision, not a prerequisite for the authoring experience.
- **The move is gated on one hard dependency: [D12](#shared-content-dependencies-d12).** Until the shared content forks into the adapters repo, every adapter's `references/spec` symlink resolves into `plugins/spec/references/` and `docs/reference/`; relocating `adapters/` before clean-copying that subset into `specify-adapters/shared/` either dangles those symlinks or drags spec-plugin prose into the wrong repo. RM-21 third-party demand is a *payoff* of the move, not a *gate* — first-party extraction proceeds regardless.

`specify lint framework --framework-root .` already parameterises the framework root, so pointing the checkers at the adapters repo is configuration, not new machinery.

**Migration is sequenced and designed** — a clean cut once [D12](#shared-content-dependencies-d12) lands, consistent with *"no migration framework — pre-1.0 this is a re-init major cut"*:

- **Shared content forks into the adapters repo at the split (D12).** The adapter-needed subset of `plugins/spec/references/` + `docs/reference/` is clean-copied into `specify-adapters/shared/` — the load-bearing step that converts the `adapters/shared/` symlink hub into an in-repo `shared/` tree the new repo copies from by `requires`. Each repo owns its copy thereafter, with no ongoing sync.
- **The move carries prose + extension crates + the `shared/` tree + the `build` machinery (copy / compile / pack) and relocates the `release.yaml` publish jobs (D6).** The `--framework-root` seam makes the lint side configuration, the publish side a job move.
- **Namespace continuity holds.** The publish ref (`specify:<name>@<ver>`, D6/D8) names a registry namespace, not a source repo, so a published artifact's identity is unchanged by the source move — proven by a pull-back (D6) from the new origin.
- **Timing of the physical cut.** The **earliest safe point** is immediately after copy-on-build and co-location (D12 + D10, Phasing steps 5–6), with the shared fork-copy performed as part of whichever cut is taken. The numbered phasing defaults to the later cut (step 8, after the shared store D5), so the full pipeline is proven once in `augentic/specify` before any relocation. A team may instead take the earliest-safe-point cut under transitional coupling (e.g. when separate owners force the repo to exist sooner); whichever variant is chosen should be recorded here when the work starts.

### CLI surface

The consumer fetch/resolve path adds **no new verbs**. Identity ([RFC-47](rfc-47-adapter-identity.md)) flows through existing paths; this RFC changes only what they fetch:

```bash
specify init omnia@1.0.0            # pulls the published artifact once, installs into the shared store
specify source survey <source>      # resolves the bound (name, version) from the shared store
specify slice build <slice>         # target resolution unchanged in shape
```

Authoring adds a single `specify adapter build` verb (D9/D10) that does all three author-time stages — copy the shared `requires` bundles in, compile any declared extension, and pack the deterministic artifact — just as `cargo build` compiles and links in one invocation. Cargo is invoked only for the compile stage and only when the committed `adapter.wasm` is absent, so prose-only adapters — and any adapter whose wasm is already committed — run `build` without a toolchain. A source edit needs an explicit `specify adapter build --refresh-extension` (recompilation is never triggered by source-mtime heuristics, mirroring the manual `cargo make contract-wasm` refresh of the `extensions/contract/dist` precedent). Publishing (the registry push in the release job) stays separate, the `cargo publish` to this `cargo build`:

```bash
specify adapter build               # one step: copy shared requires bundles → reserved trees; compile the declared extension when its committed wasm is absent → committed root adapter.wasm; deterministic pack of the adapter dir (minus extension/ + dev set) → artifact + recorded digest (D4)
specify adapter build --check       # CI drift gate (read-only, no toolchain): adapter-vendor-stale if the reserved requires trees differ from the repo's shared tree
specify adapter build --dry-run     # show the copied requires bundles + the resolved pack set + would-be digest, without writing the artifact
```

`specify archive prune` / a future `specify adapter gc` enumerates the store by `(name, version)`; cross-project reference-counting is a follow-on (see Non-Goals).

### Finding codes


| Code                              | Decision | Severity / kind    | Raised when                                                                                                                                                                                                |
| --------------------------------- | -------- | ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `adapter-digest-mismatch`         | D4       | violation (exit 2) | cached bytes (or a freshly fetched immutable locator) do not match the recorded content digest                                                                                                            |
| `adapter-vendor-stale`            | D9/D12   | violation (exit 2) | `specify adapter build --check` finds a reserved `requires` tree out of sync with the repo's `shared` tree                                                                                                |
| `adapter-authored-reserved-path`  | D9       | violation (exit 2) | a committed (non-vendored) file occupies a gitignored reserved `requires` path; the committed root `adapter.wasm` (D10) is exempt                                                                          |
| `adapter-extension-crate-missing` | D10/D11  | violation (exit 2) | `adapter.yaml.extension` is declared but there is no co-located crate at `extension/` or no committed `adapter.wasm` (replaces the retired `adapter-tool` cross-reference rule)                            |


The `adapter-version-required` / `adapter-version-malformed` identity findings live in [RFC-47](rfc-47-adapter-identity.md).

### Test plan

- **D1** — a pack/unpack round-trip test; a **deterministic-pack** test (identical inputs → byte-identical archive across two runs); a fetch-uses-injected-fetcher test mirroring `package_source_uses_fetcher` in `resolver.rs`.
- **D2** — an `adapter_uri` test parsing the `specify:<name>@<semver>` form; a "fetch targets an immutable locator, never a branch" assertion.
- **D3** — a publish-fixture test that the artifact tree is self-contained (no dangling symlinks, the `spec` bundle present, the declared extension's wasm bundled); a consumer test that install performs no vendoring (`repo_root_with_runtime` never consulted downstream).
- **D4** — a verify-on-read test (corrupting a cached byte raises `adapter-digest-mismatch`); a moved-locator test (same version, different bytes → mismatch).
- **D5** — a `cache.rs` store-resolver test that a pinned `(name, version)` maps to `<store>/<name>@<version>/`; a "two projects, same identity ⇒ both resolve the one store entry, no re-fetch" test; a "store entry is read-only after install" assertion; an "interrupted install leaves no visible entry" atomic-rename test; a "concurrent installs of one identity are idempotent under the rename flock" race test; a `locate_axis` test that a pinned `Cached` resolves to the store path while `Local` resolves the in-tree dir; a verify-on-read test that the digest is read from the entry's install metadata, not a per-project stamp.
- **D6** — a publish-then-pull-then-verify smoke job in `release.yaml`, mirroring the `extensions` pull-back verification.
- **D9** — a `build` round-trip test (copying the declared `requires` bundles from `shared/` writes byte-identical reserved trees across two runs); a `build --check` drift test asserting `adapter-vendor-stale` when the `shared/` tree is edited without re-running `build`; a reserved-namespace guard test asserting `adapter-authored-reserved-path` for an authored file under a gitignored `requires` path **and** exempting the committed `adapter.wasm`; a "post-`build` tree packs to a fresh publish's digest" equivalence test; a `Local`-resolves-post-`build` test; a "`requires` reserved bytes are writable" assertion.
- **D10** — a `specify adapter build` test that a declared-extension crate compiles to a committed `adapter.wasm`; a "`build` (including its pack stage) on a prose-only adapter and on an extension adapter whose committed wasm is already present, plus install and lint, succeed with no `wasm32-wasip2` toolchain" test; a "`build` recompiles only when the committed `adapter.wasm` is absent (or `--refresh-extension` is passed), and skips cargo when it is present" test; a pack-set test that the `extension/` crate source (and the dev/VCS exclude set) are excluded while the root `adapter.wasm` and the prose ship; a workspace-membership test that the sparse `members` glob resolves the per-adapter crates.
- **D11** — a manifest parse test that the singular `extension` object carries an optional `name` + structured `{read, write}` `permissions` and **rejects** `version` / `source` / `sha256`; a "the plural `tools[]` array no longer parses" collapse assertion; a "flat string-array permissions no longer parse" migration assertion; an "omitted `extension.name` defaults to the adapter name" assertion; a "no `tools.yaml` is read" assertion; a "retired `adapter-tool` cross-reference rule does not fire" assertion; a `specify extension run <name>` resolution test against the installed adapter tree.
- **D12** — a `build` test that a `requires` bundle is copied from the in-repo `shared/` tree into the reserved path (and no registry fetch occurs for shared content); a "consumer install performs no `requires` resolution" test.

`cargo make ci` (`RUSTFLAGS=-Dwarnings`) gates the consumer half; the publish job gates in `release.yaml`.

## Phasing

The order is dependency-driven: identity first, then the **transport loop** the adapter artifact publishes onto, then the **self-containment / authoring** refactor, then the **shared store**, and finally the **repo extraction**. Shared content itself rides no transport — it is an in-repo copy (D12) — so the `requires` copy step (step 5) has no dependency on the publish loop; only the adapter-artifact stages do.

1. **Transport spike — publish + pull + verify.** Probes `wkg publish`, `wasm-pkg-client` pull, and re-verify-on-read for an opaque blob; confirms (B) or clears (A). The transport loop (step 4) keys on it.
2. **D11 — manifest + extension unification.** Collapse the `tools[]` array to a singular `extension` object, replace the `toolDeclaration` `$def` with `extensionDeclaration`, rename `AdapterToolDeclaration` → `AdapterExtensionDeclaration` and unify it with `ExtensionPermissions`, retire the `tools.yaml` reader, replace the `adapter-tool` rule. Spike- and transport-independent — lands in parallel with step 1.
3. **D2 — package-ref form + immutable locator.** Teach `adapter_uri.rs` the `specify:<name>@<semver>` form and require an immutable fetch. Identity-dependent, spike-independent — parallel with steps 1–2.
4. **D1 + D6 + D4 — the transport loop.** Pack the tree, stand up the publish→pull→verify job in the spike-chosen shape (D1), mirror it as a `release.yaml` step (D6), and record-and-re-verify the digest on read (D4). **This is the transport every later step publishes onto.**
5. **D12 + D9 — `requires` copy for prose bundles.** Teach `adapter.yaml.requires` (a versionless bundle selector) and have `specify adapter build` copy each named bundle from the repo's canonical `shared` locations into the gitignored reserved trees — an in-repo file copy, toolchain-free (no extension declared yet, so no cargo). This replaces the `adapters/shared/` symlink hub with copy-on-build — validate it before D10. Spike- and transport-independent (no publish loop needed for an in-repo copy), so it can land alongside steps 1–3.
6. **D10 + D3 — co-located extension crates + extension compile + bundling.** Move the contract/vectis crates beside their prose, extend `specify adapter build` to compile the declared extension when its committed wasm is absent, commit the `adapter.wasm`, and bundle it at publish (D3). Retires the CLI `extensions` job (D7). Full adapters are now self-contained and publishable.
7. **D5 — shared store, resolved in place.** The dedup/offline win, once identity is immutable (D4) and the install tree is byte-stable (steps 5–6). `Cached` adapters resolve straight from `<store>/<name>@<version>/`, no per-project projection. The one transport-side piece that legitimately lands late: author-time `build` (step 5) fetch-and-verifies the `requires` deps without it.
8. **Extract adapters to `augentic/specify-adapters` (D7 / D10).** Prerequisites met (step 5 replaced the symlink hub with copy-on-build, step 6 co-located the crates — a team may cut at that earlier point). Relocate the adapter trees, clean-copy the adapter-needed shared subset into `specify-adapters/shared/` (the one-time fork, D12), carry the `build` machinery (copy / compile / pack), and relocate the `release.yaml` publish jobs out of `augentic/specify`; move the framework-lint job (`--framework-root`) and publish credentials. This completes *this* RFC's half of the two-repo end-state; [RFC-49](rfc-49-repository-topology.md) then folds `augentic/specify` + `augentic/specify-cli` into one platform repo afterward.

## Alternatives considered

- **Keep git transport; re-derive a canonical tree digest downstream guarded by a bespoke atomic-publish protocol.** Rejected — it content-addresses the *symptom*. The downstream Merkle, the digest-after-vendoring dance, and the bespoke protocol exist only because a git ref is a *moving* locator; an immutable registry digest (D2/D4) collapses the lot to a one-line verify.
- **Wrap each adapter as a wasm component.** Rejected — adds a build step for prose and buys nothing at runtime (execution stays agent-only), and does not protect the prose (`strings`-able, must still reach the model as cleartext).
- **Client-side prose expansion — thin briefs that expand from bundled wasm at point of use.** Rejected — protects nothing, collides with the *"CLI never reads brief bodies"* contract, and breaks the `lint framework` checkers that parse brief prose (`links-registry`, `prose`, `brief-schema-link-resolve`). Salvageable kernels are under [Non-Goals](#non-goals).
- **Prose-only artifact; resolve the declared extension separately.** Deferred — bundling (D3) keeps one pull and one digest. Revisit only if extension churn meaningfully outpaces adapter churn.
- **Key the store by `(name, major)`.** Rejected — a major spans infinite commits; sharing it yields first-fetch-wins drift. The store keys on the full `(name, version)`.
- **A global resolution fallback by name.** Rejected — reintroduces the ambient mutable-namespace footgun [DECISIONS.md §"Resolution is project-local only"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#adapter-loader-axis-routing) deliberately removed. The store is storage, not a fallback.
- **A per-project projection of the store entry (symlink, hardlink, or copy) (D5).** Rejected — it buys nothing over resolving the store path directly, and each variant adds cost: a symlink needs a Windows-privilege / cross-device copy fallback that forfeits dedup; a hardlink shares inodes (an accidental write mutates the store) and breaks across filesystems; a copy forfeits dedup outright. All exist only to keep `locate_axis` probing a project-local path. Threading the pinned `(name, version)` into `locate_axis` and reading `<store>/<name>@<version>/` in place (the Cargo model) is simpler, cross-platform, has a single concurrency point (the install rename), and stays project-local.
- **A `src/` → `dist/` authoring split.** Rejected — it makes the authoring tree *not* resemble the runtime tree, so `Local` and `Cached` would need divergent resolution and authored brief links would not match what ships. D9 mirrors the unpacked structure in place.
- **Commit the per-adapter copies into git (Go-vendor posture).** Rejected for the prose `requires` trees — it duplicates the in-repo `shared/` bytes across every adapter, with noisy diffs and silent drift on hand-edit, so D9 gitignores them and re-copies from `shared/` with `build --check` as the CI gate. The one exception is the built root `adapter.wasm` (D10): a binary blob has no diff to be noisy, the `extensions/contract/dist` precedent already commits one, and committing it keeps packing / install / lint toolchain-free.
- **An explicit `package.include` allow-list in the manifest.** Rejected — at dir-glob granularity it just re-enumerates the tree D9 already says *is* the artifact, so it collapses to "ship everything except the declared `extension/` source" — which `build` derives for free. The per-adapter list was near-identical boilerplate and a drift mode (a new top-level dir silently dropped when unlisted). The pack set is instead convention: the adapter directory minus the declared `extension/` crate and a built-in dev/VCS exclude set. A `package.exclude` escape hatch can be added if a real per-adapter need appears.
- **An `adapter.lock` pinning resolved `requires` digests and the built `adapter.wasm`.** Rejected — a lock records a resolver's choices, but `requires` names **in-repo** bundles with no versions at all, so the adapters repo's HEAD already fixes their bytes, and git content-addresses the committed wasm. A lock would have nothing to pin: it would restate what one commit already determines, for a schema, a committed file, and a drift surface of pure cost.
- **Publish shared content as versioned registry artifacts (an earlier D12).** Rejected — it engineers cross-repo dependency transport (a prose-only publish job, a registry resolver, version pins, a drift lock) for a dependency that never crosses a repo boundary once the `shared/` tree lives in the adapters repo. In-repo copy-on-build needs none of it; the one-time split-time clean copy ([D12](#shared-content-dependencies-d12)) handles the fork, and each repo then owns its copy.
- **A separate `vendor` verb beside `build`.** Rejected — the toolchain concern that would motivate a split is better handled *inside* `build`, which invokes cargo only when an extension is declared and its committed wasm is absent. Prose-only adapters (6 of 8) and adapters whose wasm is already committed run the same `build` and never touch the toolchain, so a second verb buys no isolation and only fragments the authoring loop. `build` copies the `requires` bundles, (conditionally) compiles the extension, and packs the artifact in one step; publishing (the release job's registry push) stays separate.
- **Keep extension source in the CLI's `extensions/` workspace (status quo).** Rejected — the crate sits in a different repo from the prose it serves, so authoring spans two repos and `build` cannot compile the extension from the tree it packs. D10 co-locates the crate at `extension/`.
- **Extract adapters into a dedicated repo *before* the shared fork.** Rejected — relocating `adapters/` while the `spec` bundle is still a symlink hub into `plugins/spec/references/` + `docs/reference/` either dangles those symlinks or drags spec-plugin prose into the wrong repo. The extraction is committed (D7/D10, step 8); the shared subset is clean-copied into `specify-adapters/shared/` *as part of* that cut (D12).
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
- `.github/workflows/release.yaml` (`extensions` job) and `[docs/release.md](https://github.com/augentic/specify-cli/blob/main/docs/release.md)` — the publish loop D6 mirrors.
- `crates/workflow/src/init/{adapter_uri,git,cache}.rs` — the current git-sparse-checkout install path D1/D2/D3 replace, and the `vendor_spec_runtime` / `ManifestMeta` D3/D4 relocate to publish.
- `crates/schema/src/cache.rs` (`mirror_dir`, `project_cache_dir`) — the cache-root precedent D5 extends.
- `crates/workflow/src/plan_lock.rs` — the `File::try_lock` flock primitive reused for the install rename (D5).
- `crates/schema/src/digest.rs` (`Hasher`) — the incremental hasher D4 verifies with.
- `extensions/{contract,vectis}/` (sibling workspace in `augentic/specify-cli`) — the current out-of-adapter extension-source location D10 co-locates.
- [RFC-47: Adapter identity](rfc-47-adapter-identity.md) — the semver identity this RFC distributes.
- [Roadmap RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model) — the ecosystem item both RFCs serve.

