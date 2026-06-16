# RFC-48: Adapter Packaging and Transport — OCI / wasm-pkg Distribution

> Status: Draft · Execution order: **2nd of RFC-47 → RFC-48 → RFC-49**, executed to completion in numerical sequence. Runs after identity ([RFC-47](rfc-47-adapter-identity.md)) lands; self-contained against today's two-repo platform, which the consolidation half ([RFC-49](rfc-49-repository-topology.md)) folds into one repo *afterward* — RFC-49 is not a precondition of any step here. · Depends: [RFC-47: Adapter identity](rfc-47-adapter-identity.md) (the semver identity this RFC distributes), the wasm-pkg tool distribution precedent (`crates/tool/src/{package,resolver}.rs`, `crates/tool/src/cache/fetch.rs`, the `wasi-tools` job in `.github/workflows/release.yaml`), the adapter loader and install path (`crates/workflow/src/init/{adapter_uri,git,cache}.rs`), the per-project cache resolver (`crates/schema/src/cache.rs`) · Related: [RFC-49: Repository topology](rfc-49-repository-topology.md) (the platform-consolidation half of the same two-repo end-state) · Roadmap: the distribution portion of [RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model).

## Abstract

An adapter is published, fetched, verified, and cached as an **immutable registry artifact**, over the same wasm-pkg / OCI plumbing first-party WASI tools already use.

One package carries both the adapter's **prose** (`adapter.yaml`, briefs, references) and its **wasm** (declared tools). So `omnia@1.0.0` is one pull — not a git sparse checkout plus N separate tool fetches.

Identity ([RFC-47](rfc-47-adapter-identity.md)) is a semver. This RFC binds that semver to an immutable, content-addressed locator and proves it on read with the **registry's own content digest**, not a bespoke downstream Merkle. Because the registry already gives immutability and content-addressing, the shared cache is an ordinary download-once-by-identity store.

The authoring tree mirrors that unpacked artifact: an author writes prose into a structure shaped like the installed store entry, declares shared content (spec-runtime, shared rules) as versioned dependencies and any wasm tools inline in `adapter.yaml`, and runs `specify adapter vendor` (prose deps) / `specify adapter build` (tool wasm) so the local tree is byte-identical to what ships. Self-containment becomes a locally reproducible fact, not a publish-only transform (D9/D10).

## Motivation

[RFC-47](rfc-47-adapter-identity.md) fixes what an adapter is *named*. This RFC fixes how the bytes behind that name travel and how they are *proven*. Three facts make the registry the natural transport:

- **We already run the loop.** The `wasi-tools` job in `release.yaml` publishes `wasm32-wasip2` components to the `augentic.io` (GHCR-backed) registry via `wkg`. `crates/tool/src/{package,resolver}.rs` fetch them through layered wasm-pkg config, then stream, sha256-verify (`specify_schema::digest::Hasher`), and atomically install. Adapter distribution is the same loop with a tree payload instead of one blob.
- **Adapters are trees, not single blobs.** The tool fetch path installs exactly one `module.wasm`; an adapter is a directory of prose plus optional wasm (see [Background](#background)). The gap this RFC closes is *one blob → a packed tree*.
- **The registry gives immutability and a digest for free.** Distribution today is a git sparse checkout (`init/git.rs`) copied into the per-project manifest cache (`init/cache.rs`). A git ref is a *moving* locator: proving immutability against it needs a bespoke publish-time Merkle plus a moved-tag backstop. An OCI content digest *is* immutable identity, so that machinery collapses to verifying the registry descriptor.

## Background

### What an adapter is, and isn't

A WASI tool is one wasm component — a single blob with a single content digest, which is why the existing fetch path (`crates/tool/src/package.rs`) installs one `module.wasm` and is done. An adapter is a **directory tree**:

- `adapter.yaml` — the prose manifest the loader validates.
- `briefs/*.md` — prose the *agent* reads and acts against; never run as code.
- `references/**` — supporting prose plus the vendored `references/spec-runtime/` bundle.
- optionally wasm tools, declared inline in the manifest's `tools[]` block (their WASI permissions) and built from co-located crates (D10/D11); historically a separate `tools.yaml` sidecar resolved each tool separately.

The loader (`SourceAdapter::resolve` / `TargetAdapter::resolve`) probes a *directory*. So the packaging problem is precisely *one blob → a tree of prose plus (optionally) wasm*, and the prose dominates.

**Distribution is not execution.** Shipping an adapter as a registry artifact changes how its bytes travel and how identity is proven. It does not turn briefs into executable wasm: source adapters stay `execution: agent` (enforced by `source.schema.json`), and there is no run-the-adapter-as-wasm step.

Packaging also cannot *hide* the prose — the agent reads it as cleartext at point of use. IP protection is therefore an access-control and licensing concern, not a packaging one (see [Security / IP considerations](#security--ip-considerations)).

### Three ways to put a tree in the registry

The registry stores content-addressed blobs and (via wasm-pkg) wasm components; an adapter is neither. Three shapes close that gap, in rising order of how literally the adapter "becomes wasm":

- **(A) Packed-tree blob.** Pack the whole tree (`adapter.tar.zst`, sidecar wasm included), stream it through the existing acquire-bytes path, then unpack. Greatest reuse: only "persist one `module.wasm`" becomes "persist and unpack one tarball." Hinges on whether `wasm-pkg-client` will carry an opaque, non-component blob (the [Prerequisite spike](#prerequisite-spike)).
- **(B) OCI artifact with layers.** Push the prose tree as one OCI layer and each wasm tool as additional layers, fetched via an OCI client (`oci-client` / `oci-wasm`). The registry's native model for "prose + wasm in one package," at the cost of a fetch path parallel to the wasm-pkg one — but `oci-client` / `oci-wasm` are already transitive deps, so that cost is small. The RFC's working default (see [Packaging shapes (D1)](#packaging-shapes-d1)).
- **(C) Wrap-as-component.** Compile a thin wasm component that embeds the tree as data and self-extracts on the consumer side. The only *literal* wasm artifact, and the heaviest: prose gains a build step for an elaborate self-extracting archive that does nothing at runtime.

All three reuse what already exists — the registry, its auth, namespace routing, and content digests — and none requires prose to stop being prose. The choice is purely *how the tree is wrapped*, which is why it reduces to the single spike question and is recorded as D1.

## Prerequisite spike

One *transport* question sizes the whole effort and picks D1's packaging form. It is **not** a single library call — it spans publish, pull, *and* re-verify-on-read, because the existing loop publishes with `wkg` (`release.yaml`) and fetches with `wasm-pkg-client`:

> **Across `wkg publish` *and* `wasm-pkg-client` pull *and* re-verify-on-read, can the `augentic.io` (GHCR-backed) registry carry an opaque, non-component blob (a packed tree)? Or must adapter transport use an OCI artifact with layers (`oci-client` / `oci-wasm`) against the same registry?**

Two facts narrow the suspense:

- **The OCI client is already in the tree.** `wasm-pkg-client@0.15.0` depends transitively on `oci-client` and `oci-wasm` (workspace `Cargo.lock`), so shape (B) adds **no new dependency** — it exposes a client already compiled in. "Fallback B" is not a heavier path; it is a different call against the same crates.
- **The risk is asymmetric.** `wasm-pkg` is component-oriented on *both* ends, so a plausible outcome is "pull tolerates an opaque blob but `wkg publish` does not" — which a pull-only spike would miss. Probe publish first.

Resolve this before authoring D1's mechanism. The RFC's working assumption is **(B)** as the primary shape (see [Packaging shapes (D1)](#packaging-shapes-d1)); (A) is adopted only if the spike shows `wasm-pkg` carries an opaque blob cleanly in *both* directions and re-verification does not force a re-tar.

## Principles

- **Identity is fixed at publish, proven by the registry.** A published `name@X.Y.Z` is immutable: the registry content digest names exactly those bytes. Consumers *verify the digest*; they do not re-derive identity from whatever a checkout produced.
- **Artifacts are self-contained.** Everything an adapter needs at resolution time — spec-runtime, declared tools — is bundled at publish. Downstream resolution does no vendoring and dereferences no in-tree symlinks; the installed tree *is* the published tree.
- **Authoring mirrors the artifact.** The authoring tree is shaped like the unpacked store entry, so `AdapterLocation::Local` and `AdapterLocation::Cached` resolve through one code path and what an author lints and tests is what ships. Shared content is a declared, versioned dependency vendored into a reserved namespace by a local `vendor` step (D9), never a symlink into a framework checkout.
- **The cache is boring.** A global store keyed by immutable `(name, version)` is download-once-by-identity with a temp-then-rename install. The integrity guarantee lives upstream at publish; downstream is a one-line verify.
- **Resolution stays project-local in semantics.** A shared *store* is storage, not a resolution fallback — what `name` resolves to is the project's pinned `(name, version)`, preserving [DECISIONS.md §"Resolution is project-local only"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#adapter-loader-axis-routing).
- **Pre-1.0 major cut, no migration framework.** This is a major bump: re-init, not migration. No compatibility aliases for git-ref pins or the `version: 1` manifest shape.

## Security / IP considerations

The prose *is* the IP — the briefs encode the methodology. The packaging layer cannot protect it, and this RFC says so explicitly so no later change reaches for obfuscation:

- **Prose is plaintext at the consumer at point of use.** No packaging shape changes this — tarball (A) untars, OCI layers (B) are pullable, wasm-wrapped bytes (C) are `strings`-able, and any "expand at point of use" step must still hand the model cleartext. Client-side obfuscation is a speed bump, not protection.
- **Access control is the real lever.** Whether the registry namespace is public or authenticated (D8) gates *who* obtains the bytes — stronger than obfuscating freely-handed-out bytes, and a net improvement over today's public git checkout.
- **Licensing carries the rest.** Copyright and registry terms govern redistribution; the risk is redistribution, not reverse-engineering markdown.
- **Sensitive logic belongs in the bundled wasm, not the prose.** Proprietary deterministic logic compiled into a declared tool (bundled by D3) is better protected than plaintext markdown, while the briefs stay prose.

Per-licensee **watermarking** (attribution, not prevention) and **server-side prose expansion** (the only true-prevention path, at the cost of the self-contained / offline property) are recorded under [Non-Goals](#non-goals).

## Design

### Normative decisions

| ID | Decision | Implementation consequence |
| --- | --- | --- |
| **D1 Packaging format** | An adapter publishes as one immutable registry artifact carrying the packed prose tree plus bundled wasm. Working-default shape: an **OCI artifact with layers** (prose layer + wasm layers) fetched via the already-vendored `oci-client` / `oci-wasm`; optimisation shape (only if the spike clears an opaque blob in *both* directions): a **packed-tree blob** (`adapter.tar.zst`) streamed through the existing fetch shape. `specify adapter pack` is byte-deterministic either way (D4). | Spike-gated (see [Prerequisite spike](#prerequisite-spike)). OCI shape adds a sibling fetch module reusing the registry/auth/namespace config; blob shape generalises `crates/tool/src/package.rs` to stream-and-unpack a tree. See [Packaging shapes (D1)](#packaging-shapes-d1). |
| **D2 Immutable fetch locator** | Fetch targets an **immutable, content-addressed locator** (OCI `@sha256:` digest, or an immutable tag whose digest is recorded), never a branch. | `init/adapter_uri.rs` gains a package-ref form (`specify:omnia@1.0.0`) alongside the local-path / GitHub-URL / shorthand forms, and does no branch-ref defaulting. The recorded digest (D4) is the backstop: a moved tag is caught as `adapter-digest-mismatch`. |
| **D3 Self-contained artifact** | Spec-runtime, and the wasm of declared tools, are bundled **at publish**. Downstream resolution does no vendoring and dereferences no in-tree symlinks; the installed tree *is* the published tree. | The legacy `vendor_spec_runtime` ancestor-walk (`init/cache.rs`) retires; spec-runtime is resolved by identity at author time ([D12](#shared-content-dependencies-d12)) into its reserved tree and `pack` ships it. The manifest's `tools[]` `module.wasm` ships inside the artifact rather than being separately resolved (see [Bundled tools (D3)](#bundled-tools-d3)). Because `specify adapter vendor` / `build` (D9/D10) land the same bytes at author time, the local working tree is self-contained too. |
| **D4 Identity via registry content digest, verified on read** | The artifact's identity is the registry content digest (`sha256:`). On install the consumer records it; on every read it re-hashes (or re-checks the descriptor) and refuses a mismatch. Re-publishing an existing `(name, version)` with different bytes is rejected **at publish**; a downstream mismatch is corruption, not routine. | Verification reuses the streaming `specify_schema::digest::Hasher` already in `package.rs`. The bespoke publish-time tree Merkle (the rejected stopgap below) is **not** built — the registry descriptor is the trust anchor. `ManifestMeta` (`init/cache.rs`) records the digest. |
| **D5 Trivial global store + projection** | A global store at `<adapters-root>/<name>@<version>/`, resolved `$SPECIFY_ADAPTER_CACHE` → `$XDG_CACHE_HOME/specify/adapters` → `$HOME/.cache/specify/adapters` → `<temp>/specify/adapters` — the `mirror_dir` precedent. Install = pull → temp → verify digest → atomic rename → `chmod` read-only. The per-project manifest cache is a **directory symlink** into the read-only entry, degrading to a recursive **copy** when symlink creation fails. | New resolver in `crates/schema/src/cache.rs`; install path in `init/cache.rs` link-or-copies from the store; `locate_axis` and the `AdapterLocation::{Cached,Local}` labels are unchanged. See [Store layout and projection (D5)](#store-layout-and-projection-d5). |
| **D6 Publish tooling** | A publish step mirrors the `wasi-tools` release job for adapters: pack the tree (+ bundled wasm), push `specify:<name>@${VERSION}` to the registry, pull back and verify. | New job in `.github/workflows/release.yaml` (parent repo), reusing the `wkg` / GHCR / `specify -> augentic.io` namespace plumbing the tool job already exercises. |
| **D7 Adapter repo extraction** | This RFC owns one half of the two-repo end-state: **adapters — prose plus co-located tool crates — relocate to a dedicated `augentic/specify-adapters` repo**, extracted from today's `augentic/specify`, once [D12](#shared-content-dependencies-d12) severs the `adapters/shared/` coupling (Phasing step 8). The *other* half — folding `augentic/specify` + `augentic/specify-cli` into one lockstep platform repo — is [RFC-49](rfc-49-repository-topology.md), executed afterward and **not** a precondition of this extraction. **The first-party tool wasm transport relocates with the adapters:** contract/vectis ship inside the adapter artifact (D3/D6/D11), so the `wasi-tools` publish job (`release.yaml`) retires. | Per [AGENTS.md §"Note to the implementing agent"](https://github.com/augentic/specify-cli/blob/main/AGENTS.md), touching the adapter loader / cache scope — including the `resolve` signature shared with [RFC-47](rfc-47-adapter-identity.md) — requires the cross-repo `rg` sweep in the same PR; net of [RFC-49](rfc-49-repository-topology.md)'s consolidation that sweep is **two-way** (platform ↔ adapters), and the AGENTS.md / DECISIONS.md pointers move with it. Retiring `wasi-tools` (`release.yaml`) removes the standalone `specify:contract@x` / `specify:vectis@x` packages, the `first_party_permissions` catalog (`tool-manifest`), and the scalar first-party `tools:` form — each decommissioned in the same change, not aliased. |
| **D8 Registry visibility and pull auth** | First-party adapter artifacts publish to an **authenticated** registry namespace; pulling requires credentials, gating *who* obtains the bytes. Visibility is the IP-bearing knob — packaging cannot obfuscate prose (see [Security / IP considerations](#security--ip-considerations)), so access control is the lever. A public namespace is a deliberate per-adapter opt-out, not the default. | Pull-side auth reuses the wasm-pkg / GHCR credential path the publish step (D6) already exercises — layered config in `crates/tool/src/package.rs::load_config` and `.specify/wasm-pkg.toml`. No new transport: the registry's native auth gates the pull. |
| **D9 Authoring mirrors the unpacked artifact** | The authoring tree is shaped like the unpacked store entry: authored prose (`adapter.yaml`, `briefs/`, `references/`, `rules/`) plus a **reserved vendored namespace** (`references/spec-runtime/`, `references/agent-teams.md`, `rules/{core,universal}/`, `tools/*/module.wasm`) an author never hand-writes. Shared content is declared by identity in `adapter.yaml.requires`; declared-tool wasm builds from a co-located crate declared in `adapter.yaml.tools[]` (D11). `specify adapter vendor` resolves `adapter.lock` and writes real bytes to the `requires` reserved paths (declared-tool `module.wasm` is built by D10), so the post-vendor/build tree is byte-identical to the artifact; `package.include` is the pack manifest. | Adds optional `requires` / `package` blocks to `schemas/{source,target}.schema.json` and a new `adapter.lock` schema; adds author-time `specify adapter {vendor, build, pack}` verbs (`build` detailed in D10; the consumer fetch/resolve path is unchanged). `vendor` resolves the prose `requires` deps locally and idempotently; `pack` deterministically tars the `package.include` set and records the digest (D4). Hand-authoring a gitignored reserved path raises `adapter-authored-reserved-path`; lock drift raises `adapter-vendor-stale`. See [Authoring structure (D9)](#authoring-structure-d9). |
| **D10 Co-located tool source** | An adapter's declared-tool Rust crate lives **beside its prose** at `tools/<name>/` (`Cargo.toml`, `src/`). The built `tools/<name>/module.wasm` is the only shipped byte and is **committed + digest-pinned** (`adapter.lock`), mirroring the existing checked-in `wasi-tools/contract/dist/*.wasm` + drift-test precedent — so `pack`, consumer install, and the framework lint walk need no wasm toolchain; only *refreshing* the byte does. The crate is the tool's source by convention; a sparse Cargo workspace spans the per-adapter crates. Co-location is the prerequisite shape for the repo split, which is a **committed end-state** (D7): adapters relocate to `augentic/specify-adapters` once [D12](#shared-content-dependencies-d12) severs the `adapters/shared/` coupling (Phasing step 8), independent of RM-21 third-party demand. | A dedicated `specify adapter build` compiles the co-located crate to `wasm32-wasip2`, writes `tools/<name>/module.wasm`, and updates its `adapter.lock` digest — kept **separate from `vendor`** so prose-only adapters (6 of 8 today) never invoke cargo. `package.include`'s `tools/**/module.wasm` ships the artifact and excludes crate source. The Rust toolchain + `clippy`/`test` CI enters the adapters tree only for the tool-bearing crates, scoped like the existing `wasi-tools` carve-out. See [Co-located tool source (D10)](#co-located-tool-source-d10). |
| **D11 Tool declaration in the manifest** | There is no standalone `tools.yaml`. The **already-present** `adapter.yaml.tools[]` block becomes the single declaration, carrying only `name` and structured WASI `permissions`. This is a **breaking** change to that block, not an additive fold: per-tool `version` moves from *required* to *rejected* (the wasm rides the adapter's single semver ([RFC-47](rfc-47-adapter-identity.md)), covered by the content digest (D4)), and `permissions` moves from a flat string array to the `{read, write}` shape the WASI runner already speaks. `source` / `sha256` never appear — the wasm builds from the co-located crate at `tools/<name>/` (D10). | Rewrites the `toolDeclaration` `$def` in `schemas/{source,target}.schema.json` (`version` removed, `permissions` → `{read[], write[]}`). Unifies `AdapterToolDeclaration` (`adapter/core.rs`) with `specify_tool_manifest::ToolPermissions` while preserving the wasmtime-free loader boundary, retires the `tools.yaml` reader (`load::plugin_sidecar`), and **replaces** the now-moot `adapter-tool` cross-reference rule (`lint/eval/cross_reference.rs`, which today reconciles sidecar-vs-manifest `version`) with a co-located-crate / built-`module.wasm` presence check. The per-tool *build* digest is pinned in `adapter.lock`. `specify tool run <name>` resolves the tool from the installed adapter tree. |
| **D12 Shared-content dependency transport** | The `requires` targets (`spec-runtime`, `review-team-protocol`, `core-rules`) are themselves **versioned registry artifacts**, published and fetched over the same plumbing as adapters (D1/D6). `specify adapter vendor` resolves each `requires` entry by pulling that artifact at **author/publish time**; the consumer never resolves `requires`, because D3 bundles the resolved bytes into the adapter artifact. This is what makes D10's `adapters/shared/` severance real: the author depends on a published *identity*, not a sibling source tree. | Adds a prose-only sibling of the adapter publish job (D6) for shared-content artifacts, a registry-targeting `requires` resolver in `vendor`, and an `adapter.lock` digest pin per resolved dependency (a D4-style *input* pin). Retires the `adapters/shared/**` sparse checkout (`init/git.rs`) and the `vendor_spec_runtime` ancestor-walk (`init/cache.rs`). See [Shared-content dependencies (D12)](#shared-content-dependencies-d12). |

### Packaging shapes (D1)

The [design space](#three-ways-to-put-a-tree-in-the-registry) is three shapes (A / B / C in Background); the spike picks between (A) and (B), and (C) is recorded-but-rejected. The RFC's working assumption is **(B)**, for reasons the spike may overturn in favour of (A):

- **(B) OCI artifact with layers — working default.** A prose layer plus one layer per wasm tool, fetched over `oci-client` / `oci-wasm` — both already transitive deps ([Prerequisite spike](#prerequisite-spike)), so the marginal dependency cost is ~zero. It is the registry's native shape for "prose + N wasm," each layer is independently content-addressed (serving D3's multi-wasm bundle and D4's verify-on-read directly), and a tool-only bump touches one layer rather than the whole blob.
- **(A) packed-tree blob — optimisation if the spike clears it.** One `adapter.tar.zst` streamed through ~90% of `crates/tool/src/package.rs`. Cheaper *only if* `wasm-pkg` carries an opaque blob in both directions; otherwise (A)'s apparent simplicity is lost to a parallel publish path anyway.
- **(C) wrap-as-component is not pursued** — it adds a prose build step and buys nothing at runtime.

**Verify-on-read differs by shape (D4).** Under (B) the consumer re-checks the cached layer descriptors it already holds — no re-tar. Under (A) "re-hash on read" must name *what* is hashed: re-taring the unpacked tree requires a byte-deterministic pack (see below), and hashing a retained tarball means keeping it past install. (B) avoids both.

**Pack must be byte-deterministic.** D4 rejects re-publishing an existing `(name, version)` with different bytes, and D9's "post-`vendor` tree packs to the publish digest" equivalence depends on identical inputs producing an identical archive. `tar` + `zstd` are non-deterministic by default, so `specify adapter pack` normalises entry order, mtimes, uid/gid, and permission bits and pins compression parameters. Under (B) this requirement narrows to the single prose layer (wasm layers are already content-addressed bytes).

### Bundled tools (D3)

An adapter that declares wasm tools via its `adapter.yaml` `tools[]` block (D11) ships their `module.wasm` *inside* the artifact, so one pull is fully self-contained and one digest covers prose + wasm.

The alternative — a prose-only artifact, tools resolved separately as today — lets a tool bump avoid republishing the whole adapter, but reintroduces N fetches and N digests per adapter. v1 bundles; a split-tool-channel is a deferred optimisation if tool churn outpaces adapter churn.

### Store layout and projection (D5)

```text
$XDG_CACHE_HOME/specify/adapters/
  omnia@1.0.0/                  # immutable, digest-verified, read-only
    adapter.yaml
    briefs/…
    references/spec-runtime/…   # bundled at publish (D3)
    tools/<name>/module.wasm    # bundled at publish (D3)
  documentation@1.0.0/
```

- **The store is CLI-write-only.** The CLI fetch path installs pristine bytes; the agent interacts only with the per-project manifest cache, a read-only projection.
- **Install is pull → temp → verify digest → atomic rename → `chmod` read-only.** The temp dir lives under the store root so the rename is atomic on one filesystem. Because identity is immutable upstream, two concurrent installs are idempotent: one wins the rename, the other verifies the matching digest and discards its temp. A flock around the rename suffices — reusing the `File::try_lock` family from `plan_lock.rs` and the staged-install precedent in `crates/tool/src/cache/fetch.rs`.
- **The per-project cache (`<project-cache>/manifests/{sources,targets}/<name>/`) is a directory symlink** into the read-only store entry. It degrades to a recursive copy when symlink creation fails (Windows privilege, cross-device) — correct, but the copy fallback forfeits the dedup the store exists for, so the sharing win is POSIX-first. `locate_axis` still finds a real directory; the `AdapterLocation::{Cached,Local}` labels are unchanged.
- **Per-project provenance moves out of the linked tree.** `ManifestMeta` / `CodexMeta` (`init/cache.rs`) are stamped *inside* the manifest cache today; an immutable, cross-project store entry cannot hold a per-project stamp, so the stamp relocates to a sidecar beside the symlink (in the per-project cache root), not inside the linked entry. No post-install writer touches the linked tree — the store is CLI-write-only and the projection is read-through.
- **The projection is a second concurrency point.** The store rename is flock-guarded; the per-project symlink creation is a separate step two concurrent `specify init` in one project can race. It is idempotent (same target), but the test plan covers it (below) alongside the store rename.
- **The store is unbounded by design.** Cross-project reference-counting is deferred ([Non-Goals](#non-goals)), so every distinct `(name, version)` accumulates. Acceptable because entries are small, immutable, and content-addressed — `specify adapter gc` / `archive prune`-style retention is the eventual home, not a v1 blocker.

### Authoring structure (D9)

The authoring tree is shaped like the unpacked store entry, so the loader resolves a `Local` working tree and a `Cached` artifact through one code path, brief-relative links (`../references/spec-runtime/…`) resolve in place, and `specify lint framework` runs against the bytes that ship. The tree is the unpacked structure plus a thin source-only control layer, and it is a *complete* mirror only after `specify adapter vendor` populates the reserved namespace.

Two consequences for `specify lint framework`. First, today it **follows symlinks** (the §F1 walk in `index/framework.rs` records both endpoints); after D9 there are no symlinks under an adapter — the reserved `requires` trees are gitignored real files — so the walk simplifies to an ordinary recurse and the symlink-target-removal gotcha retires. Second, a fresh clone is not a complete mirror until `vendor` runs, so the lint bootstrap (`make lint`) chains `specify adapter vendor` before the framework checkers, exactly as `init --workspace` chains an initial sync. The committed `module.wasm` (D10) needs no bootstrap step.

```text
adapters/targets/omnia/            # authored tree == unpacked store entry (post-vendor)
  adapter.yaml                     # authored — manifest + requires + package.include
  adapter.lock                     # generated, committed — pinned dependency + tool digests
  briefs/{shape,build,merge}.md    # authored prose
  references/                      # authored prose you own …
    guardrails.md  providers/**  examples/**
    spec-runtime/                  # reserved — vendored from spec-runtime@1.2.0       (gitignored)
    agent-teams.md                 # reserved — vendored from review-team-protocol     (gitignored)
  rules/
    omnia.mdc  provider-only-host-access.md   # authored
    core/                          # reserved — vendored CORE-* rules                  (gitignored)
    universal/                     # reserved — vendored UNI-* rules                   (gitignored)
  tools/
    replay-validator/                # co-located Rust crate (authored, source-only)
    replay-validator/module.wasm     # reserved — built wasm, committed + digest-pinned (NOT gitignored)
```

**Reserved vendored namespace.** Vendored content only ever lands at a fixed, documented set of paths an author never hand-writes, so the authored ⊎ vendored union is unambiguous and the gitignore set is exact:

| Declared dependency | Reserved path | Posture |
| --- | --- | --- |
| `requires.spec-runtime` | `references/spec-runtime/` | gitignored, regenerated by `vendor` |
| `requires.review-team-protocol` | `references/agent-teams.md` | gitignored, regenerated by `vendor` |
| `requires.core-rules` | `rules/core/`, `rules/universal/` | gitignored, regenerated by `vendor` |
| `adapter.yaml.tools[]` entry `<name>` | `tools/<name>/module.wasm` | **committed**, built by `adapter build`, digest-pinned in `adapter.lock` |

The reserved namespace has two postures. The three **`requires` trees** (`spec-runtime`, `agent-teams.md`, `rules/{core,universal}`) are gitignored and regenerated by `vendor`; a committed (non-vendored) file under one raises `adapter-authored-reserved-path` (a cheap framework check beside the existing `CORE-011` / `CORE-021` link checks), and `specify adapter vendor --check` (CI) catches stale regenerated bytes as `adapter-vendor-stale`. The **built `tools/<name>/module.wasm`** is the exception: it is *committed* and digest-pinned in `adapter.lock`, mirroring the existing `wasi-tools/contract/dist` precedent, so `pack` / install / lint never need a wasm toolchain — `adapter-authored-reserved-path` exempts it, and `vendor --check` instead verifies its committed bytes against the lock digest (`adapter-digest-mismatch`).

Unlike the D5 store, the author tree is human-owned, so vendored `requires` bytes are written **writable**, not read-only: the `vendor --check` drift gate, not a filesystem bit, guards them — read-only files inside a working tree fight `git checkout` / `clean` and editor saves for no integrity gain the lock does not already provide.

**Manifest — `adapter.yaml`** gains a `requires` block (shared content declared by identity instead of symlinked) and a `package.include` allow-list (the pack manifest):

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

# Declared wasm tools (D11). version / source / fetch-digest are
# subsumed by the adapter's own identity (RFC-47) and content digest
# (D4) — only the tool name and its WASI permissions remain. The wasm
# builds from the co-located crate at tools/<name>/ (D10).
tools:
  - name: replay-validator
    permissions:
      read: ["$PROJECT_DIR/.specify"]
      write: []

# Shared content, declared by identity instead of symlinked into the
# framework monorepo. `specify adapter vendor` resolves each entry to
# real bytes under its reserved path; adapter.lock pins the digest.
requires:
  spec-runtime: "1.2.0"
  review-team-protocol: "1.0.0"
  core-rules: "3.0.0"

# The pack manifest: exactly what the published artifact contains.
# Source-only files (adapter.lock, the tool crate sources under
# tools/<name>/, dev fixtures) are excluded, so the artifact is a
# deterministic subset of the post-vendor tree.
package:
  include:
    - adapter.yaml
    - briefs/**
    - references/**
    - rules/**
    - tools/**/module.wasm
```

**Lockfile — `adapter.lock`** pins every resolved digest so the vendored tree is byte-reproducible and the artifact digest (D4) covers a deterministic union of authored + vendored bytes. The lock is not a re-introduction of the rejected downstream Merkle: it pins the *inputs* an author vendors (so authoring is reproducible), whereas the registry content digest (D4) proves the *published output*. The two are complementary — inputs pinned upstream of pack, output verified downstream of pull — and neither re-derives identity from a checkout:

```yaml
# adapters/targets/omnia/adapter.lock
# Generated by `specify adapter vendor`; committed. `vendor --check`
# fails with `adapter-vendor-stale` when the tree drifts from this lock.
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
# Tools carry no independent version — they ride the adapter's semver
# (RFC-47); the lock pins only the built wasm digest for reproducibility.
tools:
  replay-validator:
    digest: "sha256:1c3e…aa"
```

**Tool declarations live in `adapter.yaml.tools[]`, not a standalone `tools.yaml` (D11).** The `tools[]` block already exists (`AdapterToolDeclaration`), so D11 *narrows* it rather than inventing it — but the narrowing is breaking on two fields. (1) `version` goes from required to rejected: the wasm ships inside the adapter artifact (D3), is versioned by the adapter's single semver ([RFC-47](rfc-47-adapter-identity.md)), and is covered by the content digest (D4), so a per-tool version is redundant. (2) `permissions` goes from a flat string array to the `{read, write}` shape `specify_tool_manifest::ToolPermissions` already uses, so the manifest declaration and the WASI runner stop disagreeing. `source` / `sha256` never appear — the wasm builds from the co-located crate at `tools/<name>/` (D10), discovered by convention. The old `tools.yaml` reader (`load::plugin_sidecar`) and the `adapter-tool` cross-reference rule (which existed only to reconcile the two version fields) both retire; the latter is replaced by a co-located-crate / built-`module.wasm` presence check.

This also **relocates the wasm transport** (D7): contract and vectis stop being independently-published `specify:<name>@x` wasm-pkg packages and instead ride inside the framework-published adapter artifact (D3/D6). The CLI's `wasi-tools` release job, the `first_party_permissions` catalog, and the scalar first-party `tools:` form are decommissioned in the same change, not aliased.

**Pipeline.** `specify adapter vendor` resolves each `requires` entry against `adapter.lock`, verifies the pinned digest, and writes real bytes to the reserved trees — toolchain-free. `specify adapter build` (separate, toolchain) compiles each co-located tool crate to the committed, digest-pinned `tools/<name>/module.wasm`. `specify adapter pack` then makes a deterministic tar of the `package.include` set and records the artifact digest (D4); `pack --dry-run` shows the exact bytes before CI publishes them. Because both `vendor` and `build` land real bytes at their final paths, pack is a plain tar-and-digest with no symlink dereference. `AdapterLocation::Local` resolution runs (or requires) `vendor` first, so a local-path adapter is self-contained the same way a cached one is; `Local` stays digest-verify-exempt (you are editing it) while `Cached` keeps verify-on-read (D4).

### Shared-content dependencies (D12)

The `requires` block (D9) names shared content by versioned identity, not by a path into the framework monorepo. Each target — `spec-runtime`, `review-team-protocol`, `core-rules` — is published as its own immutable registry artifact over the D1/D6 plumbing (a prose-only artifact: no wasm, no `requires` of its own), so resolving one is the same fetch-and-verify shape as resolving an adapter.

The shared content is not free-floating. Today `spec-runtime` is the `adapters/shared/references/runtime/` hub, itself a set of symlinks into the **spec skill plugin** (`plugins/spec/references/`) and `docs/reference/` — so an adapter's `references/spec-runtime` reaches two hops deep into framework-owned prose. The `spec-runtime` artifact is therefore *built from and published by `augentic/specify`* (which owns the spec plugin), and *consumed by adapters* via `requires`. Post-split this fixes the dependency direction: `augentic/specify` publishes spec-runtime; `augentic/specify-adapters` resolves it by identity — no symlink crosses the repo boundary. That inversion is the concrete mechanism the [repo extraction](#co-located-tool-source-d10) is gated on.

Two properties follow, and together they discharge the [Co-located tool source (D10)](#co-located-tool-source-d10) repo-split prerequisite:

- **The consumer never resolves `requires`.** D3 bundles the resolved bytes *into* the adapter artifact at publish, so a downstream install is one self-contained pull with no shared-content fetch and no `adapters/shared/` back-reference. `requires` is purely an author/publish-time input.
- **The author depends on an identity, not a checkout.** `specify adapter vendor` pulls `spec-runtime@1.2.0` from the registry into its reserved path, exactly as a consumer pulls an adapter — so an adapters tree (in `augentic/specify` today, a dedicated repo later) needs no sibling `augentic/specify` source tree to vendor against. This is the severance D10 is gated on; without it, `requires` would only relocate the coupling from a build-time symlink to a vendor-time path lookup.

The alternative — resolve `requires` from the local framework checkout — is rejected: it leaves the `adapters/shared/` coupling intact under a new spelling and silently re-couples a future adapters repo to `augentic/specify`. Publishing shared content as artifacts is the only `requires` transport that makes D10's severance a fact rather than a rename.

### Co-located tool source (D10)

For adapters that declare wasm tools, the tool's Rust crate lives **beside the prose it serves**, not in a sibling workspace in another repo. Today the `contract` / `vectis` tool crates live in `augentic/specify-cli` under `wasi-tools/`, far from the `contracts` / `vectis` adapter prose in `augentic/specify`. Co-location makes each adapter self-describing — prose plus the source of its deterministic logic — and lets `specify adapter build` compile the tool from the same tree it packs.

```text
adapters/targets/contracts/
  adapter.yaml                     # declares the `contract` tool in tools[] (name + permissions)
  briefs/**  references/**  rules/**   # authored prose
  tools/
    contract/                      # co-located Rust crate (AUTHORED, source-only)
      Cargo.toml
      src/lib.rs
      tests/…
      module.wasm                  # reserved — BUILT output, committed + digest-pinned, the only shipped byte
```

- **The reserved path is the `module.wasm` file, not the `tools/<name>/` directory.** Crate source (`Cargo.toml`, `src/`, `tests/`) is authored and source-only; only `tools/<name>/module.wasm` is the shipped artifact. The split is one `package.include` line — `tools/**/module.wasm`. The built byte is **committed and digest-pinned** (D9), so `adapter-authored-reserved-path` exempts it (that guard polices the gitignored `requires` trees) and the crate source stays legitimate.
- **The tool is declared in `adapter.yaml.tools[]` (D11), built from the co-located crate by convention.** No `source` field is needed — the crate at `tools/<name>/` *is* the source. `specify adapter build` compiles it to `wasm32-wasip2` and writes the committed `module.wasm`; the tool version rides the adapter's RFC-47 semver and ships in one digest (D3). Build is its own verb, not part of `vendor`, so a prose-only adapter never needs the toolchain.
- **A sparse Cargo workspace spans the per-adapter crates.** A virtual root `Cargo.toml` (`members = ["adapters/*/*/tools/*"]`) carries the crates that exist; most adapters are prose-only and contribute none. This injects a Rust toolchain and `clippy`/`test` CI into the adapters tree — exactly the discipline the existing `wasi-tools` workspace carve-out keeps separate from the host CLI, now scoped to the adapters tree — but only `specify adapter build` and that CI invoke it; `vendor`, `pack`, install, and lint stay toolchain-free.

**Repo placement.** Co-location is the prerequisite shape; the repo split itself is now a **committed end-state**, not a deferred maybe — adapters relocate to a dedicated `augentic/specify-adapters` repo. Two clarifications keep the move honest:

- **The one-PR dev loop does not depend on the move.** What lets a developer edit a brief and its tool crate in one commit is [co-location in a single tree](#authoring-structure-d9) (D10) — true whether that tree lives in `augentic/specify` or `augentic/specify-adapters`. The dedicated repo is an **ownership, cadence, and contribution-model** decision (its own issue tracker, release rhythm, and the reference shape third parties copy), not a prerequisite for the authoring experience. The authoring win lands with co-location; the repo move banks the operating-model win on top.
- **The move is gated on one hard dependency: [D12](#shared-content-dependencies-d12).** Until shared content is published as versioned artifacts, every adapter's `spec-runtime` symlink resolves into `plugins/spec/references/` and `docs/reference/` (see [D12](#shared-content-dependencies-d12)); relocating `adapters/` before that either dangles those symlinks or drags spec-plugin prose into the wrong repo. RM-21 third-party demand is a *payoff* of the move, not a *gate* on it — first-party extraction proceeds regardless.

`specify lint framework --framework-root .` already parameterises the framework root, so pointing the framework checkers at the adapters repo is configuration, not new machinery — the repo-split discipline extends to the second repo (the platform repo becomes the consolidated core + CLI once [RFC-49](rfc-49-repository-topology.md) runs afterward).

**Migration is sequenced and designed.** The move is in scope (Phasing step 8), executed as a clean cut once [D12](#shared-content-dependencies-d12) lands — consistent with *"no migration framework — pre-1.0 this is a re-init major cut"*. The mechanics:

- **Shared content is published from `augentic/specify` first (D12).** `spec-runtime` / `core-rules` / `review-team-protocol` are published as versioned artifacts *before* the move, built from `plugins/spec/references/` + `docs/reference/`. This is the load-bearing step: it converts the `adapters/shared/` symlink hub into a registry identity the new repo resolves by `requires`, so there is something to relocate *cleanly* rather than a dangling back-reference.
- **The move carries prose + tool crates + the `vendor` / `build` / `pack` machinery and relocates the `release.yaml` publish jobs (D6) already standing in the platform repo.** Those adapter and shared-content publish jobs and the `lint framework` job relocate to — or split across — `augentic/specify-adapters`; the `--framework-root` seam makes the lint side configuration, the publish side a job move.
- **Namespace continuity holds.** The publish ref (`specify:<name>@<ver>`, D6/D8) names a registry namespace, not a source repo, so a published artifact's identity is unchanged by the source move — proven by a pull-back (D6) from the new origin. The cross-repo `rg` discipline and the AGENTS.md / DECISIONS.md pointers settle to two-way — platform ↔ adapters — once [RFC-49](rfc-49-repository-topology.md) consolidates the platform afterward; while this RFC runs the sweep additionally spans `augentic/specify` ↔ `augentic/specify-cli`, as it does today.
- **Timing of the physical cut.** The **earliest safe point** is immediately after severance and co-location (D12 + D10, Phasing steps 5–6) — the extraction's only hard dependencies. The numbered phasing defaults to the later cut (Phasing step 8, after the shared store D5), so the full transport + self-containment + store pipeline is proven once in `augentic/specify` (still pre-consolidation — that is [RFC-49](rfc-49-repository-topology.md)'s job, afterward) before a single relocation. A team may instead take the earliest-safe-point cut and scaffold `augentic/specify-adapters` early, migrating incrementally under transitional coupling (e.g. when separate owners or an external contribution cadence force the repo to exist sooner). Whichever variant is chosen should be recorded here when the work starts, so the transitional coupling is a justified decision rather than latent churn.

### CLI surface

The consumer fetch/resolve path adds **no new verbs**. Identity ([RFC-47](rfc-47-adapter-identity.md)) flows through existing fetch/resolve paths; this RFC changes only what those paths fetch:

```bash
specify init omnia@1.0.0            # pulls the published artifact once, installs into the shared store
specify source survey <source>      # resolves the bound (name, version) from the shared store
specify slice build <slice>         # target resolution unchanged in shape
```

Authoring adds an `adapter` verb group (D9/D10) for the author-time loop, alongside the anticipated `specify adapter gc`. `vendor` resolves prose `requires` only (no toolchain); `build` compiles the co-located tool crates (toolchain), kept separate so prose-only adapters never invoke cargo:

```bash
specify adapter vendor              # resolve prose requires → reserved trees (no Rust toolchain)
specify adapter vendor --check      # CI drift gate: adapter-vendor-stale if requires ≠ lock; adapter-digest-mismatch if module.wasm ≠ lock
specify adapter build               # compile co-located tool crates → committed tools/<name>/module.wasm + lock digest
specify adapter pack [--dry-run]    # deterministic tar of the package.include set + record the digest (D4)
```

`specify archive prune` / a future `specify adapter gc` enumerates the store by `(name, version)`; cross-project reference-counting is a follow-on (see Non-Goals).

### Finding codes

| Code | Decision | Severity / kind | Raised when |
| --- | --- | --- | --- |
| `adapter-digest-mismatch` | D4 | violation (exit 2) | cached bytes (or a freshly fetched immutable locator) do not match the recorded content digest; also raised by `vendor --check` when a committed `module.wasm` (D10) drifts from its `adapter.lock` digest |
| `adapter-vendor-stale` | D9/D12 | violation (exit 2) | `specify adapter vendor --check` finds a gitignored `requires` tree (or `adapter.lock`) out of sync with the declared `requires` |
| `adapter-authored-reserved-path` | D9 | violation (exit 2) | a committed (non-vendored) file occupies a gitignored reserved `requires` path (`references/spec-runtime/`, `rules/core/`, …); the committed `tools/<name>/module.wasm` (D10) is exempt |
| `adapter-tool-crate-missing` | D10/D11 | violation (exit 2) | an `adapter.yaml.tools[]` entry has no co-located crate at `tools/<name>/` or no committed `module.wasm` (the check that replaces the retired `adapter-tool` cross-reference rule) |

The `adapter-version-required` / `adapter-version-malformed` identity findings live in [RFC-47](rfc-47-adapter-identity.md).

### Test plan

- **D1** — a pack/unpack round-trip test (packed tree unpacks byte-identical); a **deterministic-pack** test (identical inputs produce byte-identical archive bytes across two runs); a fetch-uses-injected-fetcher test mirroring `package_source_uses_fetcher` in `resolver.rs`.
- **D2** — an `adapter_uri` test parsing the `specify:<name>@<semver>` package-ref form; a "fetch targets an immutable locator, never a branch" assertion.
- **D3** — a publish-fixture test that the artifact tree is self-contained (no dangling symlinks, spec-runtime present, declared tools' wasm bundled); a consumer test that install performs no vendoring (`repo_root_with_runtime` is never consulted downstream).
- **D4** — a verify-on-read test (corrupting a cached byte raises `adapter-digest-mismatch`); a moved-locator test (same version, different bytes → mismatch).
- **D5** — a `cache.rs` resolver test mirroring `distinct_projects_get_distinct_dirs` for the shared root; a "two projects, same identity ⇒ second is a link/copy, not a re-fetch" test; a "symlink-disabled falls back to copy" test; a "store entry is read-only after install" assertion; an "interrupted install leaves no visible entry" atomic-rename test; a "concurrent projection into one project is idempotent" race test; a "per-project provenance stamp lives beside the symlink, not inside the read-only entry" assertion.
- **D6** — a publish-then-pull-then-verify smoke job in `release.yaml`, mirroring the `wasi-tools` job's pull-back verification.
- **D9** — a `vendor` round-trip test (resolving `requires` writes byte-identical reserved trees matching `adapter.lock`); a `vendor --check` drift test asserting `adapter-vendor-stale`; a reserved-namespace guard test asserting `adapter-authored-reserved-path` for an authored file under a gitignored `requires` path **and** exempting the committed `tools/<name>/module.wasm`; a "post-`vendor` tree packs to the digest a fresh publish produces" equivalence test; a `Local`-resolves-post-`vendor` test asserting the same shape as `Cached`; a "`requires` reserved bytes are writable, not read-only" assertion.
- **D10** — a `specify adapter build` test that a co-located `tools/<name>/` crate compiles to a committed, digest-pinned `module.wasm`; a "`vendor` / `pack` / install / lint succeed with no `wasm32-wasip2` toolchain available" test (the toolchain-free path); a `package.include` test that crate source (`Cargo.toml`, `src/`) is excluded while `module.wasm` ships; a workspace-membership test that the sparse `members` glob resolves the per-adapter crates.
- **D11** — a manifest parse test that `adapter.yaml.tools[]` carries `name` + structured `{read, write}` `permissions` and **rejects** a `version` / `source` / `sha256` field; a "flat string-array permissions no longer parse" migration assertion; a "no `tools.yaml` is read" assertion; a "retired `adapter-tool` cross-reference rule does not fire" assertion; a `specify tool run <name>` resolution test against the installed adapter tree.
- **D12** — a `vendor` test that a `requires` entry resolves to a published shared-content artifact and writes byte-identical reserved bytes; a "consumer install performs no `requires` fetch" test (the bytes are pre-bundled by D3, so `repo_root_with_runtime` is never consulted downstream).

`cargo make ci` (`RUSTFLAGS=-Dwarnings`) gates the consumer half; the publish job gates in `release.yaml`.

## Phasing

The order is dependency-driven: identity first, then the **transport loop** every later step publishes onto, then the **self-containment / authoring** refactor that rides that transport, then the **shared store**, and finally the **repo extraction**. An earlier draft sequenced the self-containment cluster (D9 / D12) *ahead* of the transport (D1 / D6) it depends on; this order corrects that — shared content cannot be published as a versioned artifact (D12) before a deterministic pack (D1) and a publish job (D6) exist, and D12 explicitly forbids the local-checkout interim that would otherwise bridge the gap.

1. **Transport spike — publish + pull + verify.** Probes `wkg publish`, `wasm-pkg-client` pull, and re-verify-on-read for an opaque blob; confirms (B) as the working default or clears (A). Smallest first step; the transport loop (step 4) keys on it.
2. **D11 — manifest + tool unification.** Rewrite the `toolDeclaration` `$def` (drop `version`, structure `permissions`), unify `AdapterToolDeclaration` with `ToolPermissions`, retire the `tools.yaml` reader, and replace the `adapter-tool` cross-reference rule. Pure schema/type work; spike-independent and transport-independent, so it can land in parallel with step 1.
3. **D2 — package-ref form + immutable locator.** Teach `adapter_uri.rs` the `specify:<name>@<semver>` form and require an immutable fetch (never a branch). Identity-dependent, spike-independent — parallel with steps 1–2.
4. **D1 + D6 + D4 — the transport loop.** Pack the tree and stand up the publish→pull→verify job in the shape the spike chose (D1), mirror it as a `release.yaml` publish step in the platform repo (D6), and record-and-re-verify the registry digest on read (D4). Testable end-to-end via the pack/unpack round-trip and deterministic-pack tests before any adapter is self-contained. **This is the transport every later step publishes onto.**
5. **D12 + D9 — `requires` + `vendor` for prose deps.** Publish shared content (`spec-runtime`, `core-rules`, `review-team-protocol`) as versioned artifacts *over the step-4 transport*, teach `adapter.yaml.requires` + `adapter.lock`, and write the gitignored reserved trees with `specify adapter vendor` (no toolchain). This is the step that severs the `adapters/shared/` coupling — validate it before D10.
6. **D10 + D3 — co-located tool crates + `build` + bundling.** Move the contract/vectis crates beside their prose, add `specify adapter build` (the only toolchain step), commit the digest-pinned `module.wasm`, and bundle it at publish so one pull is self-contained (D3). Retires the CLI `wasi-tools` job (D7). Full adapters are now self-contained and genuinely publishable.
7. **D5 — shared store + projection.** The dedup/offline win, once identity is immutable (D4) and the install tree is byte-stable (steps 5–6). The one transport-side piece that legitimately lands late: author-time `vendor` (step 5) fetch-and-verifies without it.
8. **Extract adapters to `augentic/specify-adapters` (D7 / D10).** Prerequisites met by here: step 5 has severed the `adapters/shared/` coupling and step 6 has co-located the tool crates (the extraction's only hard gates — a team may cut at that earlier point, see [Co-located tool source (D10)](#co-located-tool-source-d10)). Relocate the adapter trees (prose + tool crates + the `vendor` / `build` / `pack` machinery and the `release.yaml` publish jobs) out of today's `augentic/specify`, stand up the platform ↔ adapters contract, and move the framework-lint job (`--framework-root`) and publish credentials to the new repo. This completes *this* RFC's half of the two-repo end-state; [RFC-49](rfc-49-repository-topology.md) then folds the remaining `augentic/specify` + `augentic/specify-cli` into the single lockstep platform repo. Mechanics under [Co-located tool source (D10)](#co-located-tool-source-d10).

The critical correction over the prior draft: the **transport loop (step 4) precedes the `requires` / `vendor` refactor (step 5)**, because D12 publishes shared content over D1 / D6. D5 (step 7) still follows D2–D4 — sharing is only correct once identity is immutable and the install tree is self-contained — and the repo extraction (step 8) still follows D12 (step 5) and co-location (step 6). All of this keys on [RFC-47](rfc-47-adapter-identity.md)'s semver identity, which lands first and independently. The platform consolidation ([RFC-49](rfc-49-repository-topology.md)) runs *after* this RFC completes — it is not a precondition for any step here, so the whole sequence executes one RFC at a time in numerical order.

## Alternatives considered

- **Keep git transport; re-derive a canonical tree digest downstream and guard it with a bespoke atomic-publish protocol.** Rejected — it content-addresses the *symptom*. The downstream Merkle, the digest-after-vendoring dance, and the bespoke publish protocol exist only because a git ref is a *moving* locator; an immutable registry digest (D2/D4) collapses the lot to a one-line verify.
- **Wrap each adapter as a wasm component (prose compiled in).** Rejected — adds a build step for prose and buys nothing at runtime, since execution stays agent-only. It also does not protect the prose: embedded markdown is `strings`-able and must still reach the model as cleartext.
- **Client-side prose expansion — thin briefs that expand from bundled wasm at point of use.** Rejected — it protects nothing (the expander emits cleartext, the embedded source stays `strings`-able), collides with the *"CLI never reads brief bodies"* contract, and breaks the `specify lint framework` checkers that parse brief prose (`links-registry`, `prose`, `brief-schema-link-resolve`). The salvageable kernels are recorded under [Non-Goals](#non-goals).
- **Prose-only artifact; resolve declared tools separately as today.** Deferred — bundling (D3) keeps one pull and one digest. Revisit only if tool churn meaningfully outpaces adapter churn.
- **Key the store by `(name, major)`.** Rejected — a major spans infinite commits; sharing it yields first-fetch-wins drift. The store keys on the full `(name, version)` ([RFC-47](rfc-47-adapter-identity.md) identity).
- **A global resolution fallback by name.** Rejected — reintroduces the ambient mutable-namespace footgun [DECISIONS.md §"Resolution is project-local only"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#adapter-loader-axis-routing) deliberately removed. The store is storage, not a resolution fallback.
- **Hardlink the per-project projection (D5).** Rejected — shared inodes mean an accidental write through the per-project cache would mutate the store, and they break across filesystems. A read-only store entry plus a symlink (copy fallback) keeps the store immutable and the failure mode loud.
- **A `src/` → `dist/` authoring split (separate authored inputs from an assembled output tree).** Rejected — it makes the authoring tree *not* resemble the runtime tree, so `Local` and `Cached` would need divergent resolution and authored brief links would not match what ships. D9 mirrors the unpacked structure in place, with vendored content in a reserved gitignored namespace.
- **Commit the vendored bytes into git (Go-vendor posture).** Rejected for the prose `requires` trees — committing them is noisy diffs and silent drift on hand-edit, so D9 gitignores them and pins digests in `adapter.lock` with `specify adapter vendor --check` as the CI drift gate. The one exception is the built `tools/<name>/module.wasm` (D10): a binary blob has no diff to be noisy, the existing `wasi-tools/contract/dist` precedent already commits one, and committing it keeps `pack` / install / lint toolchain-free — so it is committed and digest-pinned rather than gitignored.
- **Resolve `requires` from the local framework checkout instead of publishing shared content.** Rejected ([D12](#shared-content-dependencies-d12)) — it leaves the `adapters/shared/` coupling intact under a vendor-time path lookup and silently re-couples a future adapters repo to `augentic/specify`, defeating D10's split prerequisite. Shared content is published as versioned artifacts so the author depends on an identity, not a sibling tree.
- **One `vendor` verb that also builds tool wasm.** Rejected — folding the cargo build into `vendor` forces a `wasm32-wasip2` toolchain on prose-only adapters (6 of 8 today) that declare no tools. D10 splits `specify adapter build` (toolchain, occasional) from `specify adapter vendor` (prose `requires`, toolchain-free).
- **Keep tool source in the CLI's `wasi-tools/` workspace (status quo).** Rejected — the tool crate sits in a different repo from the prose it serves, so authoring spans two repos and `specify adapter vendor` cannot build the tool from the tree it packs. D10 co-locates the crate beside its adapter at `tools/<name>/`.
- **Extract adapters into a dedicated repo *before* D12.** Rejected — relocating `adapters/` while `spec-runtime` is still a symlink hub into `plugins/spec/references/` + `docs/reference/` ([D12](#shared-content-dependencies-d12)) either dangles those symlinks or drags spec-plugin prose into the wrong repo. The extraction is committed (D7/D10, Phasing step 8) but sequenced *after* D12 turns shared content into versioned `requires` artifacts the new repo resolves by identity.
- **Keep the standalone `tools.yaml` sidecar.** Rejected — once the wasm is bundled (D3), versioned by the adapter's semver ([RFC-47](rfc-47-adapter-identity.md)), and covered by the content digest (D4), a per-tool `version` / `source` / `sha256` is redundant. D11 narrows the existing `adapter.yaml.tools[]` block to (`name`, structured `permissions`) — a breaking shape change, not an additive fold (see [D11](#design)); prose and wasm move in lockstep under one identity.

## Non-Goals

- **Adapter identity** — the semver `version` and the `AdapterRef` resolve signature are [RFC-47](rfc-47-adapter-identity.md).
- The hosted registry/publish *index* (discovery, search, a release feed), semver **range** resolution (`^1.0`, `~1.2`), third-party namespacing (`org/name@req`), and *range-based* `requires-cli` policy — RM-21 (the exact-floor `requires-cli` guard is [RFC-47](rfc-47-adapter-identity.md) D4). Note pull-side auth and visibility on the existing `augentic.io` namespace are **in scope** here (D8), not deferred.
- Cross-project reference counting and GC of the shared store beyond a simple `(name, version)` enumeration.
- **Per-licensee watermarking** of published / installed artifacts — deferred. Attribution and breach-traceability, not prevention; it applies at publish or install over any packaging shape (no thin-wrapper machinery required), so it rides on top of D1 unchanged if a business need lands.
- **Server-side / hosted prose expansion** — out of scope. It is the only mechanism that actually withholds prose from the consumer, but it contradicts the self-contained / offline principle this RFC is built on and is a hosted-product concern, not a packaging one.
- **Opening the adapters repo to third-party authors** (external contribution, discovery/search, quality gates beyond first-party) — [RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model). The *first-party* extraction to `augentic/specify-adapters` **is** in scope (D7/D10, Phasing step 8); only outside authorship is deferred.
- Any migration framework — pre-1.0 this is a re-init major cut.

## References

- `crates/tool/src/package.rs` (`fetch`, `load_config`, `AcquiredBytes`) — the wasm-pkg fetch/verify path D1 reuses or parallels.
- `crates/tool/src/resolver.rs` and `crates/tool/src/cache/fetch.rs` (`stage_and_install`) — the stage-temp-then-atomic-install precedent D5 mirrors for adapters.
- `crates/tool-manifest/src/lib.rs` (`ToolSource`, `PackageRequest`, `DEFAULT_WASM_PKG_CONFIG`, `WASM_PKG_CONFIG_PATH`) — the package-ref shape and layered registry config D2 extends to adapters.
- `.github/workflows/release.yaml` (`wasi-tools` job) and [`docs/release.md`](https://github.com/augentic/specify-cli/blob/main/docs/release.md) — the publish loop D6 mirrors.
- `crates/workflow/src/init/{adapter_uri,git,cache}.rs` — the current git-sparse-checkout install path D1/D2/D3 replace, and the `vendor_spec_runtime` / `ManifestMeta` D3/D4 relocate to publish.
- `crates/schema/src/cache.rs` (`mirror_dir`, `project_cache_dir`) — the cache-root precedent D5's adapter store extends.
- `crates/workflow/src/plan_lock.rs` — the `File::try_lock` flock primitive reused for the install rename (D5).
- `crates/schema/src/digest.rs` (`Hasher`) — the incremental hasher D4 verifies with.
- `wasi-tools/{contract,vectis}/` (sibling workspace in `augentic/specify-cli`) — the current out-of-adapter tool-source location D10 co-locates beside its prose.
- [RFC-47: Adapter identity](rfc-47-adapter-identity.md) — the semver identity this RFC distributes.
- [Roadmap RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model) — the ecosystem item both RFCs serve.
