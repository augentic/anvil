# RFC-48: Adapter Packaging and Transport — OCI / wasm-pkg Distribution

> Status: Draft · Depends: [RFC-47: Adapter identity](rfc-47-adapter-identity.md) (the semver identity this RFC distributes), the wasm-pkg tool distribution precedent (`crates/tool/src/{package,resolver}.rs`, `crates/tool/src/cache/fetch.rs`, the `wasi-tools` job in `.github/workflows/release.yaml`), the adapter loader and install path (`crates/workflow/src/init/{adapter_uri,git,cache}.rs`), the per-project cache resolver (`crates/schema/src/cache.rs`) · Roadmap: the distribution portion of [RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model).

## Abstract

An adapter is published, fetched, verified, and cached as an **immutable registry artifact** over the same wasm-pkg / OCI plumbing first-party WASI tools already use. A single package carries both the adapter's **prose** (`adapter.yaml`, briefs, references) and its **wasm** (declared tools), so `omnia@1.0.0` is one pull, not a git sparse checkout plus N separate tool fetches. Identity ([RFC-47](rfc-47-adapter-identity.md)) is a semver; this RFC binds that semver to an immutable, content-addressed locator and proves it on read with the **registry's own content digest** rather than a bespoke downstream Merkle. Because the registry already gives immutability and content-addressing, the shared cache is an ordinary download-once-by-identity store.

## Motivation

[RFC-47](rfc-47-adapter-identity.md) fixes what an adapter is *named*. This RFC fixes how the bytes behind that name travel and how they are *proven*. Three facts make the registry the natural transport:

- **We already run the loop.** The `wasi-tools` job in `release.yaml` builds `wasm32-wasip2` components and publishes `specify:contract@${VERSION}` / `specify:vectis@${VERSION}` to `augentic.io` (GHCR-backed) via `wkg`; `crates/tool/src/{package,resolver}.rs` resolve them through layered wasm-pkg config (`.specify/wasm-pkg.toml` → `WKG_CONFIG` → embedded `specify -> augentic.io` fallback), stream the bytes, hash with `specify_schema::digest::Hasher`, sha256-verify, and atomically install. Adapter distribution is the same loop with a tree payload instead of one blob.
- **Adapters are trees, not single blobs.** The tool fetch path installs exactly one `module.wasm`; an adapter is a directory of prose plus optional wasm (see [Background](#background)). The gap this RFC closes is *one blob → a packed tree*.
- **The registry gives immutability and a digest for free.** Distribution today is a git sparse checkout from `github.com/augentic/specify/adapters/...@ref` (`init/git.rs`), copied into the per-project manifest cache by `init/cache.rs`. A git ref is a *moving* locator; proving immutability against it needs a bespoke publish-time Merkle plus a moved-tag backstop. An OCI content digest *is* immutable identity, so that machinery collapses to verifying the registry descriptor.

## Background

### What an adapter is, and isn't

A WASI tool is one wasm component — a single blob with a single content digest, which is exactly why the existing fetch path (`crates/tool/src/package.rs`) installs one `module.wasm` and is done. An adapter is not that shape. It is a **directory tree**:

- `adapter.yaml` — the prose manifest the loader validates.
- `briefs/*.md` — prose the *agent* reads and acts against; never run as code.
- `references/**` — supporting prose plus the vendored `references/spec-runtime/` bundle.
- optionally a `tools.yaml` sidecar that *declares* wasm tools, each currently fetched separately from the registry.

The loader (`SourceAdapter::resolve` / `TargetAdapter::resolve`) probes a *directory*, and distribution materialises that directory today by git sparse checkout (`init/git.rs`) into the per-project manifest cache (`init/cache.rs`). So the packaging problem is precisely *one blob → a tree of prose plus (optionally) wasm*, and the prose dominates: most of an adapter is markdown a human and an agent read.

That forces the framing point this RFC keeps returning to: **distribution is not execution.** Shipping an adapter as a registry artifact changes how its bytes travel and how its identity is proven — it does not turn briefs into executable wasm. Source adapters stay `execution: agent` (enforced by `source.schema.json`), and all eight first-party manifests remain agent-driven. "All adapters become wasi artifacts" is true only in the *distribution* sense — they ride the same registry, auth, and digest plumbing as tools — and false in the *execution* sense: there is no run-the-adapter-as-wasm step, and adding one (option C below) buys nothing at runtime. It also means packaging cannot *hide* the prose: the agent reads it as cleartext at point of use, so IP protection is an access-control and licensing concern, not a packaging one (see [Security / IP considerations](#security--ip-considerations)).

### Three ways to put a tree in the registry

The registry stores content-addressed blobs and (via wasm-pkg) wasm components; an adapter is neither the single blob the tool path expects nor a component. Three shapes close that gap, in rising order of how literally the adapter "becomes wasm":

- **(A) Packed-tree blob.** Pack the whole tree (`adapter.tar.zst`, sidecar wasm included) and stream it through the existing acquire-bytes path, then unpack. Greatest reuse — the stream / hash / size-cap / tempfile machinery in `package.rs` is untouched; only "persist one `module.wasm`" becomes "persist and unpack one tarball." Hinges on whether `wasm-pkg-client` will carry an opaque, non-component blob (the [Prerequisite spike](#prerequisite-spike)).
- **(B) OCI artifact with layers.** Push the prose tree as one OCI layer and each wasm tool as additional layers, under the same `augentic.io` namespace, fetched via an OCI client (`oci-client` / `oci-distribution`). The most natural fit for "prose + wasm in one package" and the registry's native model, at the cost of a fetch path parallel to the wasm-pkg component path. The fallback if (A) is infeasible.
- **(C) Wrap-as-component.** Compile a thin wasm component that embeds the tree as data and self-extracts on the consumer side. The only shape under which an adapter is *literally* a wasm artifact — and the heaviest: prose gains a build step, and the component is an elaborate self-extracting archive that does nothing at runtime, since execution stays agent-only.

All three reuse the asset that already exists — the registry, its auth, namespace routing, and content digests — and none requires prose to stop being prose. The choice is purely *how the tree is wrapped*, which is why it reduces to the single spike question and is recorded normatively as D1.

## Prerequisite spike

One library question sizes the whole effort and picks D1's packaging form: **can `wasm-pkg-client` push and pull an opaque, non-component blob (a packed tree), or must adapter transport use an OCI client (`oci-client` / `oci-distribution`) directly against the same registry?** wasm-pkg is component-oriented; if it rejects a non-component media type, adapter fetch becomes a parallel OCI path rather than a near-verbatim reuse of `crates/tool/src/package.rs`. Resolve this before authoring D1's mechanism — it decides "reuse the tool fetch" versus "add an OCI fetch path."

## Principles

- **Identity is fixed at publish, proven by the registry.** A published `name@X.Y.Z` is immutable: the registry content digest names exactly those bytes. Consumers *verify the digest*; they do not re-derive identity from whatever a checkout produced.
- **Artifacts are self-contained.** Everything an adapter needs at resolution time — spec-runtime, declared tools — is bundled at publish. Downstream resolution does no vendoring and dereferences no in-tree symlinks; the installed tree *is* the published tree.
- **The cache is boring.** A global store keyed by immutable `(name, version)` is download-once-by-identity with a temp-then-rename install. The integrity guarantee lives upstream at publish; downstream is a one-line verify.
- **Resolution stays project-local in semantics.** A shared *store* is storage, not a resolution fallback — what `name` resolves to is the project's pinned `(name, version)`, preserving [DECISIONS.md §"Resolution is project-local only"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#adapter-loader-axis-routing).
- **Pre-1.0 major cut, no migration framework.** This is a major bump: re-init, not migration. No compatibility aliases for git-ref pins or the `version: 1` manifest shape.

## Security / IP considerations

The prose *is* the IP — the briefs encode the methodology. The packaging layer cannot protect it, and this RFC says so explicitly so no later change reaches for obfuscation:

- **Prose is plaintext at the consumer at point of use.** The agent reads briefs directly from the per-project manifest cache (or the prepare-phase `$SCRATCH_DIR` lane); the bytes are in the clear on a machine the publisher does not control at the moment the model consumes them. No packaging shape changes this — tarball (A) untars, OCI layers (B) are pullable, and wasm-wrapped bytes (C) are `strings`-able; any "expand at point of use" step must still hand the model cleartext. Client-side obfuscation is a speed bump, not protection.
- **Access control is the real lever.** Whether the registry namespace is public or authenticated (D8) gates *who* obtains the bytes — far stronger than obfuscating bytes handed out freely, and a net improvement over today's public git checkout regardless of which packaging shape wins.
- **Licensing carries the rest.** Copyright and registry terms govern redistribution; reverse-engineering markdown is not the risk, redistribution is.
- **Sensitive logic belongs in the bundled wasm, not the prose.** Genuinely proprietary deterministic logic compiled into a declared tool (bundled by D3) is meaningfully better protected than markdown — compiled, not plaintext — while the briefs stay prose. That is the right home for IP-sensitive computation.

Per-licensee **watermarking** (attribution, not prevention) and **server-side prose expansion** (the only true-prevention path, at the cost of the self-contained / offline property) are recorded under [Non-Goals](#non-goals).

## Design

### Normative decisions

| ID | Decision | Implementation consequence |
| --- | --- | --- |
| **D1 Packaging format** | An adapter publishes as one immutable registry artifact carrying the packed prose tree plus bundled wasm. Preferred shape: a **packed-tree blob** (`adapter.tar.zst`) streamed through the existing fetch shape; fallback shape (if the spike rules out non-component blobs over wasm-pkg): an **OCI artifact with layers** (prose layer + wasm layers) fetched via an OCI client against the same `augentic.io` registry. | Spike-gated (see [Prerequisite spike](#prerequisite-spike)). Blob shape generalises `crates/tool/src/package.rs` to stream-and-unpack a tree; OCI shape adds a sibling fetch module reusing the registry/auth/namespace config. See [Packaging shapes (D1)](#packaging-shapes-d1). |
| **D2 Immutable fetch locator** | Fetch targets an **immutable, content-addressed locator** (OCI `@sha256:` digest, or an immutable tag whose digest is recorded), never a branch. | `init/adapter_uri.rs` gains a package-ref form (`specify:omnia@1.0.0`) alongside the local-path / GitHub-URL / shorthand forms, and does no branch-ref defaulting. The recorded digest (D4) is the backstop: a moved tag is caught as `adapter-digest-mismatch`. |
| **D3 Self-contained artifact** | Spec-runtime, and the wasm of declared tools, are bundled **at publish**. Downstream resolution does no vendoring and dereferences no in-tree symlinks; the installed tree *is* the published tree. | `vendor_spec_runtime` (`init/cache.rs`) runs at the publish step, not in the consumer's install. The `tools.yaml` declaration's `module.wasm` ships inside the artifact rather than being separately resolved (see [Bundled tools (D3)](#bundled-tools-d3)). |
| **D4 Identity via registry content digest, verified on read** | The artifact's identity is the registry content digest (`sha256:`). On install the consumer records it; on every read it re-hashes (or re-checks the descriptor) and refuses a mismatch. Re-publishing an existing `(name, version)` with different bytes is rejected **at publish**; a downstream mismatch is corruption, not routine. | Verification reuses the streaming `specify_schema::digest::Hasher` already in `package.rs`. The bespoke publish-time tree Merkle (the rejected stopgap below) is **not** built — the registry descriptor is the trust anchor. `ManifestMeta` (`init/cache.rs`) records the digest. |
| **D5 Trivial global store + projection** | A global store at `<adapters-root>/<name>@<version>/`, resolved `$SPECIFY_ADAPTER_CACHE` → `$XDG_CACHE_HOME/specify/adapters` → `$HOME/.cache/specify/adapters` → `<temp>/specify/adapters` — the `mirror_dir` precedent. Install = pull → temp → verify digest → atomic rename → `chmod` read-only. The per-project manifest cache is a **directory symlink** into the read-only entry, degrading to a recursive **copy** when symlink creation fails. | New resolver in `crates/schema/src/cache.rs` (sibling to `mirror_dir` / `project_cache_dir`); install path in `init/cache.rs` link-or-copies from the store; `locate_axis` and the `AdapterLocation::{Cached,Local}` labels are unchanged. See [Store layout and projection (D5)](#store-layout-and-projection-d5). |
| **D6 Publish tooling** | A publish step mirrors the `wasi-tools` release job for adapters: pack the tree (+ bundled wasm), push `specify:<name>@${VERSION}` to the registry, pull back and verify. | New job in `.github/workflows/release.yaml` (parent repo), reusing the `wkg` / GHCR / `specify -> augentic.io` namespace plumbing the tool job already exercises. |
| **D7 Repo split** | Fetch/unpack, store resolver, digest verification, and the package-ref parser live in `augentic/specify-cli`; packing, publish tooling, and brief/doc references in `augentic/specify`. | Per [AGENTS.md §"Note to the implementing agent"](https://github.com/augentic/specify-cli/blob/main/AGENTS.md), touching the adapter loader / cache scope — including the `resolve` signature shared with [RFC-47](rfc-47-adapter-identity.md) — requires the cross-repo `rg` sweep in the same PR. |
| **D8 Registry visibility and pull auth** | First-party adapter artifacts publish to an **authenticated** registry namespace; pulling requires credentials, gating *who* obtains the bytes. Visibility is the IP-bearing knob — packaging cannot obfuscate prose (see [Security / IP considerations](#security--ip-considerations)), so access control is the lever. A public namespace is a deliberate per-adapter opt-out, not the default. | Pull-side auth reuses the wasm-pkg / GHCR credential path the publish step (D6) already exercises — layered config in `crates/tool/src/package.rs::load_config` and `.specify/wasm-pkg.toml`. No new transport: the registry's native auth gates the pull. |

### Packaging shapes (D1)

The [design space](#three-ways-to-put-a-tree-in-the-registry) is three shapes (A / B / C in Background); the spike picks between (A) and (B), and (C) is recorded-but-rejected. The decision and its implementation consequence:

- **Preferred — (A) packed-tree blob**, if `wasm-pkg-client` carries a non-component blob: ~90% reuse of `crates/tool/src/package.rs` — stream-and-unpack one `adapter.tar.zst` instead of persisting one `module.wasm`.
- **Fallback — (B) OCI artifact with layers**, if the spike rules (A) out: a sibling fetch module over `oci-client`, reusing the same registry / auth / namespace config.
- **(C) wrap-as-component is not pursued** — it adds a prose build step and buys nothing at runtime.

### Bundled tools (D3)

An adapter that declares wasm tools via `tools.yaml` ships their `module.wasm` *inside* the artifact, so one pull is fully self-contained and one digest covers prose + wasm — consistent with the self-contained principle. The alternative (prose-only artifact; tools resolved separately as today) lets a tool bump avoid republishing the whole adapter but reintroduces N fetches and N digests per adapter. v1 bundles; a split-tool-channel is a deferred optimisation if tool churn outpaces adapter churn. The bundled wasm is also the right home for IP-sensitive logic: compiled tool bytes are better protected than plaintext prose (see [Security / IP considerations](#security--ip-considerations)).

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

- **The store is CLI-write-only.** Pristine artifact bytes are installed by the CLI fetch path; the agent interacts only with the per-project manifest cache, a read-only projection.
- **Install is pull → temp → verify digest → atomic rename → `chmod` read-only.** The temp dir lives under the store root so the rename is atomic on one filesystem. Because identity is immutable upstream, two concurrent installs of the same identity are idempotent: one wins the rename, the other verifies the matching digest and discards its temp. A flock around the rename (reusing the `File::try_lock` family from `plan_lock.rs`, and the staged-install precedent in `crates/tool/src/cache/fetch.rs`) is sufficient and unremarkable.
- **The per-project cache (`<project-cache>/manifests/{sources,targets}/<name>/`) is a directory symlink** into the read-only store entry, degrading to a recursive copy when symlink creation fails (Windows privilege, cross-device). `locate_axis` still finds a real directory at the same path; the `AdapterLocation::{Cached,Local}` labels are unchanged.

### CLI surface

No new top-level verbs. Identity ([RFC-47](rfc-47-adapter-identity.md)) flows through existing fetch/resolve paths; this RFC changes only what those paths fetch:

```bash
specify init omnia@1.0.0            # pulls the published artifact once, installs into the shared store
specify source survey <source>      # resolves the bound (name, version) from the shared store
specify slice build <slice>         # target resolution unchanged in shape
```

`specify archive prune` / a future `specify adapter gc` enumerates the store by `(name, version)`; cross-project reference-counting is a follow-on (see Non-Goals).

### Finding codes

| Code | Decision | Severity / kind | Raised when |
| --- | --- | --- | --- |
| `adapter-digest-mismatch` | D4 | violation (exit 2) | cached bytes (or a freshly fetched immutable locator) do not match the recorded content digest |

The `adapter-version-required` / `adapter-version-malformed` identity findings live in [RFC-47](rfc-47-adapter-identity.md).

### Test plan

- **D1** — a pack/unpack round-trip test (packed tree unpacks byte-identical); a fetch-uses-injected-fetcher test mirroring `package_source_uses_fetcher` in `resolver.rs`.
- **D2** — an `adapter_uri` test parsing the `specify:<name>@<semver>` package-ref form; a "fetch targets an immutable locator, never a branch" assertion.
- **D3** — a publish-fixture test that the artifact tree is self-contained (no dangling symlinks, spec-runtime present, declared tools' wasm bundled); a consumer test that install performs no vendoring (`repo_root_with_runtime` is never consulted downstream).
- **D4** — a verify-on-read test (corrupting a cached byte raises `adapter-digest-mismatch`); a moved-locator test (same version, different bytes → mismatch).
- **D5** — a `cache.rs` resolver test mirroring `distinct_projects_get_distinct_dirs` for the shared root; a "two projects, same identity ⇒ second is a link/copy, not a re-fetch" test; a "symlink-disabled falls back to copy" test; a "store entry is read-only after install" assertion; an "interrupted install leaves no visible entry" atomic-rename test.
- **D6** — a publish-then-pull-then-verify smoke job in `release.yaml`, mirroring the `wasi-tools` job's pull-back verification.

`cargo make ci` (`RUSTFLAGS=-Dwarnings`) gates the consumer half; the publish job gates in `release.yaml`.

## Phasing

1. **Spike — wasm-pkg blob feasibility.** Settles D1's mechanism. Smallest first step; everything downstream keys on it.
2. **D2 — package-ref form + immutable locator.** Teach `adapter_uri.rs` the `specify:<name>@<semver>` form and require an immutable fetch.
3. **D3 — self-contained artifact.** Bundle spec-runtime and declared tools at publish so the installed tree is standalone; required before any digest is stable or any store entry is shareable.
4. **D4 — verify on read.** Record the registry digest at install; re-verify on read. Tamper-evident now that the artifact is self-contained.
5. **D1 + D6 — packaging + publish tooling.** Pack the tree and stand up the publish job, in the shape the spike chose.
6. **D5 — shared store + projection.** The dedup/offline win, once identity is immutable and the install tree is byte-stable.

D5 must follow D2–D4 (sharing is only correct once identity is immutable and the install tree is self-contained). All of this keys on [RFC-47](rfc-47-adapter-identity.md)'s semver identity, which can land first and independently.

## Alternatives considered

- **Keep git transport; re-derive a canonical tree digest downstream and guard it with a bespoke atomic-publish protocol.** Rejected as steady state — it content-addresses the *symptom*. The downstream Merkle identity, the digest-after-vendoring dance, and the bespoke publish protocol exist only because a git ref is a *moving* locator. An immutable registry digest (D2/D4) collapses the lot to a one-line verify. Acceptable only as a stopgap while no immutable distribution exists — and the registry already exists.
- **Wrap each adapter as a wasm component (prose compiled in).** Rejected as default — adds a build step for prose and buys nothing at runtime, since execution stays agent-only. The packaging win is registry transport, not literal wasm execution. It also does not protect the prose as IP: embedded markdown is `strings`-able and must still reach the model as cleartext (see [Security / IP considerations](#security--ip-considerations)).
- **Client-side prose expansion — thin briefs that expand from bundled wasm at point of use.** Rejected — proposed as a way to keep prose out of flat `.md` files, it protects nothing: the expander emits cleartext the agent (and any operator running the same tool) reads, while the embedded source stays `strings`-able, so it combines both weak postures. It also collides with the *"CLI never reads brief bodies"* contract and breaks the `specify lint framework` checkers that parse brief prose (`links-registry`, `prose`, `brief-schema-link-resolve`). The salvageable kernels — per-licensee watermarking and context-computed prose — are recorded under [Non-Goals](#non-goals).
- **Prose-only artifact; resolve declared tools separately as today.** Deferred — bundling (D3) keeps one pull and one digest covering prose + wasm. Revisit only if tool churn meaningfully outpaces adapter churn.
- **Key the store by `(name, major)`.** Rejected — a major spans infinite commits; sharing it yields first-fetch-wins drift. The store keys on the full `(name, version)` ([RFC-47](rfc-47-adapter-identity.md) identity).
- **A global resolution fallback by name.** Rejected — reintroduces the ambient mutable-namespace footgun [DECISIONS.md §"Resolution is project-local only"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#adapter-loader-axis-routing) deliberately removed. The store is storage, not a resolution fallback.
- **Hardlink the per-project projection (D5).** Rejected as the default — hardlinks share inodes, so an accidental write through the per-project cache would mutate the shared store, and they break across filesystems. A read-only store entry plus a symlink (copy fallback) keeps the store immutable and the failure mode loud.

## Non-Goals

- **Adapter identity** — the semver `version` and the `AdapterRef` resolve signature are [RFC-47](rfc-47-adapter-identity.md).
- The hosted registry/publish *index* (discovery, search, a release feed), semver **range** resolution (`^1.0`, `~1.2`), and third-party namespacing (`org/name@req`) / `requires-cli` floors — RM-21. Note pull-side auth and visibility on the existing `augentic.io` namespace are **in scope** here (D8), not deferred.
- Cross-project reference counting and GC of the shared store beyond a simple `(name, version)` enumeration.
- **Per-licensee watermarking** of published / installed artifacts — deferred. Attribution and breach-traceability, not prevention; it applies at publish or install over any packaging shape (no thin-wrapper machinery required), so it rides on top of D1 unchanged if a business need lands.
- **Server-side / hosted prose expansion** — out of scope. It is the only mechanism that actually withholds prose from the consumer, but it contradicts the self-contained / offline principle this RFC is built on and is a hosted-product concern, not a packaging one.
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
- [RFC-47: Adapter identity](rfc-47-adapter-identity.md) — the semver identity this RFC distributes.
- [Roadmap RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model) — the ecosystem item both RFCs serve.
