# RFC-48: Adapter Packaging and Transport — OCI / wasm-pkg Distribution

> Status: Draft · Depends: [RFC-47: Adapter identity](rfc-47-adapter-identity.md) (the semver identity this RFC distributes), the wasm-pkg tool distribution precedent (`crates/tool/src/{package,resolver}.rs`, `crates/tool/src/cache/fetch.rs`, the `wasi-tools` job in `.github/workflows/release.yaml`), the adapter loader and install path (`crates/workflow/src/init/{adapter_uri,git,cache}.rs`), the per-project cache resolver (`crates/schema/src/cache.rs`) · Roadmap: the distribution portion of [RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model).

## Abstract

An adapter is published, fetched, verified, and cached as an **immutable registry artifact**, over the same wasm-pkg / OCI plumbing first-party WASI tools already use.

One package carries both the adapter's **prose** (`adapter.yaml`, briefs, references) and its **wasm** (declared tools). So `omnia@1.0.0` is one pull — not a git sparse checkout plus N separate tool fetches.

Identity ([RFC-47](rfc-47-adapter-identity.md)) is a semver. This RFC binds that semver to an immutable, content-addressed locator and proves it on read with the **registry's own content digest**, not a bespoke downstream Merkle. Because the registry already gives immutability and content-addressing, the shared cache is an ordinary download-once-by-identity store.

The authoring tree mirrors that unpacked artifact: an author writes prose into a structure shaped like the installed store entry, declares shared content (spec-runtime, shared rules) as versioned dependencies and any wasm tools inline in `adapter.yaml`, and runs one `specify adapter vendor` step so the local tree is byte-identical to what ships. Self-containment becomes a locally reproducible fact, not a publish-only transform (D9).

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
- **(B) OCI artifact with layers.** Push the prose tree as one OCI layer and each wasm tool as additional layers, fetched via an OCI client (`oci-client` / `oci-distribution`). The registry's native model for "prose + wasm in one package," at the cost of a fetch path parallel to the wasm-pkg one. The fallback if (A) is infeasible.
- **(C) Wrap-as-component.** Compile a thin wasm component that embeds the tree as data and self-extracts on the consumer side. The only *literal* wasm artifact, and the heaviest: prose gains a build step for an elaborate self-extracting archive that does nothing at runtime.

All three reuse what already exists — the registry, its auth, namespace routing, and content digests — and none requires prose to stop being prose. The choice is purely *how the tree is wrapped*, which is why it reduces to the single spike question and is recorded as D1.

## Prerequisite spike

One library question sizes the whole effort and picks D1's packaging form:

> **Can `wasm-pkg-client` push and pull an opaque, non-component blob (a packed tree)? Or must adapter transport use an OCI client (`oci-client` / `oci-distribution`) directly against the same registry?**

wasm-pkg is component-oriented. If it rejects a non-component media type, adapter fetch becomes a parallel OCI path (B) rather than a near-verbatim reuse of `crates/tool/src/package.rs` (A). Resolve this before authoring D1's mechanism.

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
| **D1 Packaging format** | An adapter publishes as one immutable registry artifact carrying the packed prose tree plus bundled wasm. Preferred shape: a **packed-tree blob** (`adapter.tar.zst`) streamed through the existing fetch shape; fallback shape (if the spike rules out non-component blobs over wasm-pkg): an **OCI artifact with layers** (prose layer + wasm layers) fetched via an OCI client against the same `augentic.io` registry. | Spike-gated (see [Prerequisite spike](#prerequisite-spike)). Blob shape generalises `crates/tool/src/package.rs` to stream-and-unpack a tree; OCI shape adds a sibling fetch module reusing the registry/auth/namespace config. See [Packaging shapes (D1)](#packaging-shapes-d1). |
| **D2 Immutable fetch locator** | Fetch targets an **immutable, content-addressed locator** (OCI `@sha256:` digest, or an immutable tag whose digest is recorded), never a branch. | `init/adapter_uri.rs` gains a package-ref form (`specify:omnia@1.0.0`) alongside the local-path / GitHub-URL / shorthand forms, and does no branch-ref defaulting. The recorded digest (D4) is the backstop: a moved tag is caught as `adapter-digest-mismatch`. |
| **D3 Self-contained artifact** | Spec-runtime, and the wasm of declared tools, are bundled **at publish**. Downstream resolution does no vendoring and dereferences no in-tree symlinks; the installed tree *is* the published tree. | `vendor_spec_runtime` (`init/cache.rs`) runs at the publish step, not in the consumer's install. The manifest's `tools[]` declaration's `module.wasm` ships inside the artifact rather than being separately resolved (see [Bundled tools (D3)](#bundled-tools-d3)). The same inline runs at author time via `specify adapter vendor` (D9), so the local working tree is self-contained too. |
| **D4 Identity via registry content digest, verified on read** | The artifact's identity is the registry content digest (`sha256:`). On install the consumer records it; on every read it re-hashes (or re-checks the descriptor) and refuses a mismatch. Re-publishing an existing `(name, version)` with different bytes is rejected **at publish**; a downstream mismatch is corruption, not routine. | Verification reuses the streaming `specify_schema::digest::Hasher` already in `package.rs`. The bespoke publish-time tree Merkle (the rejected stopgap below) is **not** built — the registry descriptor is the trust anchor. `ManifestMeta` (`init/cache.rs`) records the digest. |
| **D5 Trivial global store + projection** | A global store at `<adapters-root>/<name>@<version>/`, resolved `$SPECIFY_ADAPTER_CACHE` → `$XDG_CACHE_HOME/specify/adapters` → `$HOME/.cache/specify/adapters` → `<temp>/specify/adapters` — the `mirror_dir` precedent. Install = pull → temp → verify digest → atomic rename → `chmod` read-only. The per-project manifest cache is a **directory symlink** into the read-only entry, degrading to a recursive **copy** when symlink creation fails. | New resolver in `crates/schema/src/cache.rs`; install path in `init/cache.rs` link-or-copies from the store; `locate_axis` and the `AdapterLocation::{Cached,Local}` labels are unchanged. See [Store layout and projection (D5)](#store-layout-and-projection-d5). |
| **D6 Publish tooling** | A publish step mirrors the `wasi-tools` release job for adapters: pack the tree (+ bundled wasm), push `specify:<name>@${VERSION}` to the registry, pull back and verify. | New job in `.github/workflows/release.yaml` (parent repo), reusing the `wkg` / GHCR / `specify -> augentic.io` namespace plumbing the tool job already exercises. |
| **D7 Repo split** | Fetch/unpack, store resolver, digest verification, and the package-ref parser live in `augentic/specify-cli`; packing, publish tooling, and brief/doc references in `augentic/specify`. | Per [AGENTS.md §"Note to the implementing agent"](https://github.com/augentic/specify-cli/blob/main/AGENTS.md), touching the adapter loader / cache scope — including the `resolve` signature shared with [RFC-47](rfc-47-adapter-identity.md) — requires the cross-repo `rg` sweep in the same PR. |
| **D8 Registry visibility and pull auth** | First-party adapter artifacts publish to an **authenticated** registry namespace; pulling requires credentials, gating *who* obtains the bytes. Visibility is the IP-bearing knob — packaging cannot obfuscate prose (see [Security / IP considerations](#security--ip-considerations)), so access control is the lever. A public namespace is a deliberate per-adapter opt-out, not the default. | Pull-side auth reuses the wasm-pkg / GHCR credential path the publish step (D6) already exercises — layered config in `crates/tool/src/package.rs::load_config` and `.specify/wasm-pkg.toml`. No new transport: the registry's native auth gates the pull. |
| **D9 Authoring mirrors the unpacked artifact** | The authoring tree is shaped like the unpacked store entry: authored prose (`adapter.yaml`, `briefs/`, `references/`, `rules/`) plus a **reserved vendored namespace** (`references/spec-runtime/`, `references/agent-teams.md`, `rules/{core,universal}/`, `tools/*/module.wasm`) an author never hand-writes. Shared content is declared by identity in `adapter.yaml.requires`; declared-tool wasm builds from a co-located crate declared in `adapter.yaml.tools[]` (D11). `specify adapter vendor` resolves `adapter.lock` and writes real bytes to the reserved paths, so the post-`vendor` tree is byte-identical to the artifact; `package.include` is the pack manifest. | Adds optional `requires` / `package` blocks to `schemas/{source,target}.schema.json` and a new `adapter.lock` schema; adds author-time `specify adapter {vendor, pack}` verbs (the consumer fetch/resolve path is unchanged). `vendor` reuses the publish-time inline (D3) locally and idempotently; `pack` tars the `package.include` set and records the digest (D4). Reserved-path writes raise `adapter-authored-reserved-path`; lock drift raises `adapter-vendor-stale`. See [Authoring structure (D9)](#authoring-structure-d9). |
| **D10 Co-located tool source** | An adapter's declared-tool Rust crate lives **beside its prose** at `tools/<name>/` (`Cargo.toml`, `src/`); the built `tools/<name>/module.wasm` is the only shipped byte, so the reserved path (D9) is the *file*, not the directory. The crate is the tool's source by convention. A sparse Cargo workspace spans the per-adapter crates. Adapters stay in `augentic/specify` for now; extraction to a dedicated `augentic/specify-adapters` repo is **gated on D9** severing the `adapters/shared/` symlink coupling (and on RM-21 third-party demand). | `specify adapter vendor` builds the co-located crate to `wasm32-wasip2` and writes `tools/<name>/module.wasm`; `package.include`'s `tools/**/module.wasm` ships the artifact and excludes the crate source. A Rust toolchain + `clippy`/`test` CI enters the adapters tree, scoped the way the existing `wasi-tools` workspace carve-out is. See [Co-located tool source (D10)](#co-located-tool-source-d10). |
| **D11 Tool declaration in the manifest** | There is no standalone `tools.yaml`. Declared tools live as an `adapter.yaml.tools[]` block carrying only `name` and WASI `permissions`. A per-tool `version` / `source` / `sha256` is **subsumed**: the wasm rides the adapter's single semver ([RFC-47](rfc-47-adapter-identity.md)) and is covered by the artifact content digest (D4), so prose and wasm move in lockstep. The wasm is built by convention from the co-located crate at `tools/<name>/` (D10). | Folds the `tool.schema.json` sidecar's residual (`name`, `permissions`) into `schemas/{source,target}.schema.json` as `tools[]` and drops the `version` / `source` / `sha256` fields. The per-tool *build* digest is pinned in `adapter.lock`. `specify tool run <name>` resolves the tool from the installed adapter tree. |

### Packaging shapes (D1)

The [design space](#three-ways-to-put-a-tree-in-the-registry) is three shapes (A / B / C in Background); the spike picks between (A) and (B), and (C) is recorded-but-rejected. The decision and its implementation consequence:

- **Preferred — (A) packed-tree blob**, if `wasm-pkg-client` carries a non-component blob: ~90% reuse of `crates/tool/src/package.rs` — stream-and-unpack one `adapter.tar.zst` instead of persisting one `module.wasm`.
- **Fallback — (B) OCI artifact with layers**, if the spike rules (A) out: a sibling fetch module over `oci-client`, reusing the same registry / auth / namespace config.
- **(C) wrap-as-component is not pursued** — it adds a prose build step and buys nothing at runtime.

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
- **The per-project cache (`<project-cache>/manifests/{sources,targets}/<name>/`) is a directory symlink** into the read-only store entry. It degrades to a recursive copy when symlink creation fails (Windows privilege, cross-device). `locate_axis` still finds a real directory; the `AdapterLocation::{Cached,Local}` labels are unchanged.

### Authoring structure (D9)

The authoring tree is shaped like the unpacked store entry, so the loader resolves a `Local` working tree and a `Cached` artifact through one code path, brief-relative links (`../references/spec-runtime/…`) resolve in place, and `specify lint framework` runs against the bytes that ship. The tree is the unpacked structure plus a thin source-only control layer, and it is a *complete* mirror only after `specify adapter vendor` populates the reserved namespace.

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
    replay-validator/module.wasm   # reserved — built/pulled wasm                      (gitignored)
```

**Reserved vendored namespace.** Vendored content only ever lands at a fixed, documented set of paths an author never hand-writes, so the authored ⊎ vendored union is unambiguous and the gitignore set is exact:

| Declared dependency | Reserved vendored path |
| --- | --- |
| `requires.spec-runtime` | `references/spec-runtime/` |
| `requires.review-team-protocol` | `references/agent-teams.md` |
| `requires.core-rules` | `rules/core/`, `rules/universal/` |
| `adapter.yaml.tools[]` entry `<name>` | `tools/<name>/module.wasm` |

A committed (non-vendored) file under any reserved path raises `adapter-authored-reserved-path` (a cheap framework check beside the existing `CORE-011` / `CORE-021` link checks). Reserved paths are gitignored and regenerated, so editing one locally is a no-op against the artifact — `specify adapter vendor --check` (CI) catches drift as `adapter-vendor-stale`, and the vendored islands are written read-only to mirror the D5 store posture.

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

**Lockfile — `adapter.lock`** pins every resolved digest so the vendored tree is byte-reproducible and the artifact digest (D4) covers a deterministic union of authored + vendored bytes:

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

**Tool declarations live in `adapter.yaml.tools[]`, not a standalone `tools.yaml` (D11).** Because the wasm ships inside the adapter artifact (D3), is versioned by the adapter's single semver ([RFC-47](rfc-47-adapter-identity.md)), and is covered by the content digest (D4), a separate per-tool `version` / `source` / `sha256` declaration is redundant — prose and wasm move in lockstep. The residual that *isn't* versioning — the tool's `name` and its WASI `permissions` — folds into the `tools[]` block shown in the manifest above; the wasm builds from the co-located crate at `tools/<name>/` (D10), discovered by convention.

**Pipeline.** `specify adapter vendor` resolves each `requires` entry and `tools[]` declaration against `adapter.lock`, verifies the pinned digest, and writes real bytes to the reserved paths — the same inline the publish path runs (D3), now locally and idempotently. `specify adapter pack` then tars the `package.include` set and records the artifact digest (D4); `pack --dry-run` shows the exact bytes before CI publishes them. Because vendoring lands real bytes at their final paths, pack is a plain tar-and-digest with no symlink dereference. `AdapterLocation::Local` resolution runs (or requires) `vendor` first, so a local-path adapter is self-contained the same way a cached one is; `Local` stays digest-verify-exempt (you are editing it) while `Cached` keeps verify-on-read (D4).

### Co-located tool source (D10)

For adapters that declare wasm tools, the tool's Rust crate lives **beside the prose it serves**, not in a sibling workspace in another repo. Today the `contract` / `vectis` tool crates live in `augentic/specify-cli` under `wasi-tools/`, far from the `contracts` / `vectis` adapter prose in `augentic/specify`. Co-location makes each adapter self-describing — prose plus the source of its deterministic logic — and lets `specify adapter vendor` build the tool from the same tree it packs.

```text
adapters/targets/contracts/
  adapter.yaml                     # declares the `contract` tool in tools[] (name + permissions)
  briefs/**  references/**  rules/**   # authored prose
  tools/
    contract/                      # co-located Rust crate (AUTHORED, source-only)
      Cargo.toml
      src/lib.rs
      tests/…
      module.wasm                  # reserved — BUILT output, gitignored, the only shipped byte
```

- **The reserved path is the `module.wasm` file, not the `tools/<name>/` directory.** Crate source (`Cargo.toml`, `src/`, `tests/`) is authored and source-only; only `tools/<name>/module.wasm` is the vendored, shipped artifact. The split is one `package.include` line — `tools/**/module.wasm` — so the `adapter-authored-reserved-path` guard (D9) targets the built file and leaves the crate legitimate.
- **The tool is declared in `adapter.yaml.tools[]` (D11), built from the co-located crate by convention.** No `source` field is needed — the crate at `tools/<name>/` *is* the source. `vendor` compiles it to `wasm32-wasip2` and writes `module.wasm`; the tool version rides the adapter's RFC-47 semver and ships in one digest (D3).
- **A sparse Cargo workspace spans the per-adapter crates.** A virtual root `Cargo.toml` (`members = ["adapters/*/*/tools/*"]`) carries the crates that exist; most adapters are prose-only and contribute none. This injects a Rust toolchain and `clippy`/`test` CI into the adapters tree — exactly the discipline the existing `wasi-tools` workspace carve-out keeps separate from the host CLI, now scoped to the adapters tree.

**Repo placement.** Co-location is the prerequisite shape for a clean repo split, not the split itself. Adapters stay in `augentic/specify` for now; extraction to a dedicated `augentic/specify-adapters` repo is sequenced behind two triggers:

- **D9 severs the `adapters/shared/` coupling.** While shared content (spec-runtime, `CORE-*`/`UNI-*` rules, review-team-protocol) is symlinked into each adapter, a split would dangle those symlinks. Once D9 turns shared content into versioned `requires` dependencies, an adapters repo has no back-reference into `augentic/specify`.
- **RM-21 third-party demand.** A dedicated repo's main payoff — an independent, externally-contributable adapter ecosystem — is partly [RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model)-deferred (first-party-only per D8). The self-contained, co-located adapter is the reference shape third parties would copy.

`specify lint framework --framework-root .` already parameterises the framework root, so pointing the framework checkers at an adapters repo is configuration, not new machinery — D7's repo-split discipline extends to a third repo.

**Migration is deferred, not designed.** When the triggers above are met, the move is recorded as out of scope here (see [Non-Goals](#non-goals)); this RFC does not specify its mechanics. The open questions a follow-on must settle:

- **Namespace continuity.** The publish ref (`specify:<name>@<ver>`, D6/D8) names a registry namespace, not a source repo, so a published artifact's identity need not change when the *source* repo does — but the publish job's checkout and credentials relocate, and that must be proven by a pull-back (D6) from the new origin.
- **Cross-repo contract (D7) becomes three-way.** The `rg`-sweep discipline and the AGENTS.md / DECISIONS.md pointers that today say adapters live in `augentic/specify` must extend to the third repo in the same change.
- **CI relocation.** The `release.yaml` publish job (D6) and the framework-lint job move to (or are split across) the new repo; the `--framework-root` seam makes the lint side configuration, but the publish side is a job move.
- **Cut vs tooled move.** Consistent with *"no migration framework — pre-1.0 this is a re-init major cut"*, the expectation is a clean cut (re-point pins, re-init) rather than tooled migration; a follow-on confirms whether any consumer-facing redirect is warranted.

### CLI surface

The consumer fetch/resolve path adds **no new verbs**. Identity ([RFC-47](rfc-47-adapter-identity.md)) flows through existing fetch/resolve paths; this RFC changes only what those paths fetch:

```bash
specify init omnia@1.0.0            # pulls the published artifact once, installs into the shared store
specify source survey <source>      # resolves the bound (name, version) from the shared store
specify slice build <slice>         # target resolution unchanged in shape
```

Authoring adds an `adapter` verb group (D9) for the author-time loop, alongside the anticipated `specify adapter gc`:

```bash
specify adapter vendor              # resolve requires + build co-located tool crates → reserved paths
specify adapter vendor --check      # CI drift gate: fail (adapter-vendor-stale) if tree ≠ lock
specify adapter pack [--dry-run]    # tar the package.include set + record the digest (D4)
```

`specify archive prune` / a future `specify adapter gc` enumerates the store by `(name, version)`; cross-project reference-counting is a follow-on (see Non-Goals).

### Finding codes

| Code | Decision | Severity / kind | Raised when |
| --- | --- | --- | --- |
| `adapter-digest-mismatch` | D4 | violation (exit 2) | cached bytes (or a freshly fetched immutable locator) do not match the recorded content digest |
| `adapter-vendor-stale` | D9 | violation (exit 2) | `specify adapter vendor --check` finds the vendored tree (or `adapter.lock`) out of sync with the declared `requires` / `tools` |
| `adapter-authored-reserved-path` | D9 | violation (exit 2) | a committed (non-vendored) file occupies a reserved vendored path (`references/spec-runtime/`, `rules/core/`, `tools/`, …) |

The `adapter-version-required` / `adapter-version-malformed` identity findings live in [RFC-47](rfc-47-adapter-identity.md).

### Test plan

- **D1** — a pack/unpack round-trip test (packed tree unpacks byte-identical); a fetch-uses-injected-fetcher test mirroring `package_source_uses_fetcher` in `resolver.rs`.
- **D2** — an `adapter_uri` test parsing the `specify:<name>@<semver>` package-ref form; a "fetch targets an immutable locator, never a branch" assertion.
- **D3** — a publish-fixture test that the artifact tree is self-contained (no dangling symlinks, spec-runtime present, declared tools' wasm bundled); a consumer test that install performs no vendoring (`repo_root_with_runtime` is never consulted downstream).
- **D4** — a verify-on-read test (corrupting a cached byte raises `adapter-digest-mismatch`); a moved-locator test (same version, different bytes → mismatch).
- **D5** — a `cache.rs` resolver test mirroring `distinct_projects_get_distinct_dirs` for the shared root; a "two projects, same identity ⇒ second is a link/copy, not a re-fetch" test; a "symlink-disabled falls back to copy" test; a "store entry is read-only after install" assertion; an "interrupted install leaves no visible entry" atomic-rename test.
- **D6** — a publish-then-pull-then-verify smoke job in `release.yaml`, mirroring the `wasi-tools` job's pull-back verification.
- **D9** — a `vendor` round-trip test (resolving `requires` + `tools` writes byte-identical reserved paths matching `adapter.lock`); a `vendor --check` drift test asserting `adapter-vendor-stale`; a reserved-namespace guard test asserting `adapter-authored-reserved-path` when an authored file lands under a reserved path; a "post-`vendor` tree packs to the digest a fresh publish produces" equivalence test; a `Local`-resolves-post-`vendor` test asserting the same shape as `Cached`.
- **D10** — a `vendor` test that a co-located `tools/<name>/` crate builds to `module.wasm` at the reserved path; a `package.include` test that crate source (`Cargo.toml`, `src/`) is excluded from the packed artifact while `module.wasm` ships; a workspace-membership test that the sparse `members` glob resolves the per-adapter crates.
- **D11** — a manifest parse test that `adapter.yaml.tools[]` carries `name` + `permissions` and rejects a `version` / `source` / `sha256` field; a "no `tools.yaml` is read" assertion; a `specify tool run <name>` resolution test against the installed adapter tree.

`cargo make ci` (`RUSTFLAGS=-Dwarnings`) gates the consumer half; the publish job gates in `release.yaml`.

## Phasing

1. **Spike — wasm-pkg blob feasibility.** Settles D1's mechanism. Smallest first step; everything downstream keys on it.
2. **D2 — package-ref form + immutable locator.** Teach `adapter_uri.rs` the `specify:<name>@<semver>` form and require an immutable fetch.
3. **D3 + D9 + D10 + D11 — self-contained artifact + authoring structure + co-located tool source + manifest-embedded tool declaration.** Declare shared content as versioned `requires`, fold each declared tool's `name` + `permissions` into `adapter.yaml.tools[]` (no `tools.yaml`), co-locate its crate at `tools/<name>/`, and bundle the built wasm; `specify adapter vendor` writes them into the reserved namespace at author time (so the local tree is self-contained and byte-identical to the artifact) and the same inline runs at publish. Required before any digest is stable or any store entry is shareable. A dedicated adapters repo is **not** part of this phase — it is gated on D9 severing the `adapters/shared/` symlinks (see [Co-located tool source (D10)](#co-located-tool-source-d10) and [Non-Goals](#non-goals)).
4. **D4 — verify on read.** Record the registry digest at install; re-verify on read. Tamper-evident now that the artifact is self-contained.
5. **D1 + D6 — packaging + publish tooling.** Pack the tree and stand up the publish job, in the shape the spike chose.
6. **D5 — shared store + projection.** The dedup/offline win, once identity is immutable and the install tree is byte-stable.

D5 must follow D2–D4 (sharing is only correct once identity is immutable and the install tree is self-contained). All of this keys on [RFC-47](rfc-47-adapter-identity.md)'s semver identity, which can land first and independently.

## Alternatives considered

- **Keep git transport; re-derive a canonical tree digest downstream and guard it with a bespoke atomic-publish protocol.** Rejected — it content-addresses the *symptom*. The downstream Merkle, the digest-after-vendoring dance, and the bespoke publish protocol exist only because a git ref is a *moving* locator; an immutable registry digest (D2/D4) collapses the lot to a one-line verify.
- **Wrap each adapter as a wasm component (prose compiled in).** Rejected — adds a build step for prose and buys nothing at runtime, since execution stays agent-only. It also does not protect the prose: embedded markdown is `strings`-able and must still reach the model as cleartext.
- **Client-side prose expansion — thin briefs that expand from bundled wasm at point of use.** Rejected — it protects nothing (the expander emits cleartext, the embedded source stays `strings`-able), collides with the *"CLI never reads brief bodies"* contract, and breaks the `specify lint framework` checkers that parse brief prose (`links-registry`, `prose`, `brief-schema-link-resolve`). The salvageable kernels are recorded under [Non-Goals](#non-goals).
- **Prose-only artifact; resolve declared tools separately as today.** Deferred — bundling (D3) keeps one pull and one digest. Revisit only if tool churn meaningfully outpaces adapter churn.
- **Key the store by `(name, major)`.** Rejected — a major spans infinite commits; sharing it yields first-fetch-wins drift. The store keys on the full `(name, version)` ([RFC-47](rfc-47-adapter-identity.md) identity).
- **A global resolution fallback by name.** Rejected — reintroduces the ambient mutable-namespace footgun [DECISIONS.md §"Resolution is project-local only"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#adapter-loader-axis-routing) deliberately removed. The store is storage, not a resolution fallback.
- **Hardlink the per-project projection (D5).** Rejected — shared inodes mean an accidental write through the per-project cache would mutate the store, and they break across filesystems. A read-only store entry plus a symlink (copy fallback) keeps the store immutable and the failure mode loud.
- **A `src/` → `dist/` authoring split (separate authored inputs from an assembled output tree).** Rejected — it makes the authoring tree *not* resemble the runtime tree, so `Local` and `Cached` would need divergent resolution and authored brief links would not match what ships. D9 mirrors the unpacked structure in place, with vendored content in a reserved gitignored namespace.
- **Commit the vendored bytes into git (Go-vendor posture).** Rejected as the default — the git tree would literally be the artifact, but at the cost of noisy diffs and silent drift if a vendored file is hand-edited. D9 gitignores the reserved namespace and pins digests in `adapter.lock`, with `specify adapter vendor --check` as the CI drift gate.
- **Keep tool source in the CLI's `wasi-tools/` workspace (status quo).** Rejected — the tool crate sits in a different repo from the prose it serves, so authoring spans two repos and `specify adapter vendor` cannot build the tool from the tree it packs. D10 co-locates the crate beside its adapter at `tools/<name>/`.
- **Extract adapters into a dedicated repo now.** Deferred — the split is clean only after D9 turns the `adapters/shared/` symlinks into versioned `requires` dependencies; doing it first dangles those symlinks. Recorded under [Non-Goals](#non-goals); co-location (D10) is the prerequisite shape.
- **Keep the standalone `tools.yaml` sidecar.** Rejected — once the wasm is bundled (D3), versioned by the adapter's semver ([RFC-47](rfc-47-adapter-identity.md)), and covered by the content digest (D4), a per-tool `version` / `source` / `sha256` is redundant. D11 folds the residual (`name`, `permissions`) into `adapter.yaml.tools[]`; prose and wasm move in lockstep under one identity.

## Non-Goals

- **Adapter identity** — the semver `version` and the `AdapterRef` resolve signature are [RFC-47](rfc-47-adapter-identity.md).
- The hosted registry/publish *index* (discovery, search, a release feed), semver **range** resolution (`^1.0`, `~1.2`), and third-party namespacing (`org/name@req`) / `requires-cli` floors — RM-21. Note pull-side auth and visibility on the existing `augentic.io` namespace are **in scope** here (D8), not deferred.
- Cross-project reference counting and GC of the shared store beyond a simple `(name, version)` enumeration.
- **Per-licensee watermarking** of published / installed artifacts — deferred. Attribution and breach-traceability, not prevention; it applies at publish or install over any packaging shape (no thin-wrapper machinery required), so it rides on top of D1 unchanged if a business need lands.
- **Server-side / hosted prose expansion** — out of scope. It is the only mechanism that actually withholds prose from the consumer, but it contradicts the self-contained / offline principle this RFC is built on and is a hosted-product concern, not a packaging one.
- **Extracting adapters into a dedicated `augentic/specify-adapters` repo** — sequenced behind D9 (severing the `adapters/shared/` symlink coupling) and RM-21 third-party demand. Co-located tool source (D10) is the prerequisite shape; the repo move itself is not part of this RFC.
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
