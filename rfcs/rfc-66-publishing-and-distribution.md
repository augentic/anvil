# RFC-66: Publishing and Distribution — One Transport, Two Axes

> Status: Proposed · Depends: [RFC-63](rfc-63-adapter-hydration.md) (hydration kernel, store root, `adapters.lock`), RFC-64 (one-component artifact, wasm-pkg publish — landed in `specify-adapters`), [RFC-65](rfc-65-standalone-deployment.md) (the `specify:` naming cut, the embedded core guest) · Owns: how every Specify artifact is published by developers and acquired by operators, across both repos

## Abstract

Every published wasm artifact — the WIT contract and the adapter components — travels over **one transport**: wasm-pkg/OCI, backed by GHCR behind a static well-known file at `augentic.io`. Everything native — the host binary, with the core guest embedded inside it (RFC-65 move 4) — travels over GitHub Releases and Homebrew. The developer axis collapses to *bump one `Cargo.toml` version, push a tag*: idempotent tag-driven workflows publish only what moved, authenticated by `GITHUB_TOKEN` alone. The operator axis collapses to *`brew install augentic/tap/specify`*: adapters hydrate through the RFC-63 kernel at init and sync, and the core guest rides the binary so upgrading the binary is the only version knob an operator turns. No custom registry service, no pack format, no crates.io, no dev-tool binary, no committed wasm.

## The artifact inventory

| Artifact | Identity | Published by | Acquired by |
| -------- | -------- | ------------ | ----------- |
| WIT contract | `specify:adapter@<ver>` (the RFC-65 rename of `augentic:specify`) | `specify` release workflow, on `package` version change | `specify-adapters` as a pinned `wkg get` into `wit/deps/` |
| Adapter components | `specify:<name>@<ver>` | `specify-adapters` release workflow (RFC-64, as landed) | `install_tofu` hydration into the RFC-63 store |
| Core guest | none — no registry identity | built from the tagged tree and embedded in the binary via the generic `runtime!` embed option (RFC-65 move 4) | rides inside the installed binary |
| Host binary | GitHub Release archives + brew formula | `specify` release pipeline (as landed) + automated tap bump | `brew install augentic/tap/specify`; archives and `cargo install --git` as fallbacks |

## The registry: a static file over GHCR

`augentic.io` stays the canonical first-party registry host — it is already hardcoded as `FIRST_PARTY_REGISTRY` in `crates/registry/src/package.rs` and written into the adapters publish workflow — but it is **not a registry service**. It serves exactly one static file:

```json
// https://augentic.io/.well-known/wasm-pkg/registry.json
{
  "preferredProtocol": "oci",
  "oci": { "registry": "ghcr.io", "namespacePrefix": "augentic/" }
}
```

The wasm-pkg client resolves registry metadata through the well-known path automatically, so `specify:omnia@1.0.0` becomes an anonymous pull from `ghcr.io/augentic/omnia:1.0.0` with no code change on the consume side beyond the RFC-65 namespace rename. Consequences:

- **Publish auth is `GITHUB_TOKEN`.** Both repos' workflows drop the `SPECIFY_REGISTRY_USERNAME` / `SPECIFY_REGISTRY_PASSWORD` secrets; `permissions: packages: write` plus a `wkg` login against `ghcr.io` is the whole credential story. Nothing to provision, nothing to rotate.
- **Consume auth is nothing.** First-party packages are public GHCR packages; TOFU pull works on a fresh laptop or CI runner with zero configuration, which is what makes RFC-63's "non-interactive, credentials from the environment" property trivially true.
- **Mirrors and air gaps keep their existing levers.** `.specify/wasm-pkg.toml` and `WKG_CONFIG` already override namespace routing per project and per invocation; nothing new is needed.

## Developer axis: bump the version, push the tag

### Adapters repo

`.github/workflows/release.yaml` keeps its landed RFC-64 shape — tag `v*` → `cargo make check` → `build-guests-release` → loop `wkg publish` — with one change: the publish loop becomes **idempotent**. Before pushing `specify:<name>@<version>`, the loop probes the registry for that exact identity and skips it when present. Store entries are immutable, so a version is published at most once, ever; a tag can carry a mixed bag of bumped and untouched adapters without failing or re-pushing. The whole developer story per adapter is: bump the guest crate's `Cargo.toml` `version` (the single identity declaration, per RFC-64), tag, done.

### Specify repo

The existing tag-driven pipeline (`release.yaml` → `publish.yaml` → `release-binaries.yaml`) changes twice on the same `v*` tag:

1. **The core guest is built, not published.** Each platform build compiles the core guest from the tagged tree to `wasm32-wasip2` and hands it to the generic `runtime!` embed option (RFC-65 move 4), so every binary carries its own core. This deletes the committed `crates/workflow-guest/guest.wasm`, its `.sha256` sidecar, the `tests/dist.rs` staleness gate, and the "refresh the embedded guest" release-checklist item — the guest is *built at build time* from source, never committed. (Under RFC-65's fallback — a published `specify:core@<binary version>` — this becomes one more idempotent `wkg publish` job in the same workflow; nothing else on this axis changes.)
2. **A WIT publish job** — `specify:adapter@<wit version>` via `wkg wit build` + `wkg publish` for `wit/specify.wit`, guarded by a version-changed probe so the job is a no-op on tags that did not bump the WIT `package` declaration.

Every publish leg lives in a `cargo make publish-*` task; the GitHub workflow is a thin caller. Local emergency publishing runs the same task with a developer's own token — one code path, two invocation surfaces.

### WIT ownership becomes real

Today both repos carry byte-identical `wit/specify.wit` copies, and the adapters repo's `check-pins` task does a sibling-checkout `cmp` with an RFC-64 migration-window carve-out. At the RFC-65 naming cut (the natural break point — every reference changes anyway), the relationship flips from *copy parity* to *publish and consume*:

- `specify` owns and publishes `specify:adapter`, as `wit/README.md` already declares.
- `specify-adapters` deletes its `wit/specify.wit` copy in favour of a `wkg get specify:adapter@<pin>` vendored into `wit/deps/`, pinned in exactly one place, refreshed by a `cargo make wit-vendor` task.
- The dev loop keeps a sibling-path override: while iterating on a contract change in `specify`, the adapters build can point at the sibling file before the new version is published.
- `check-pins`'s WIT arm becomes "the vendored file byte-matches the pinned published version" — deterministic in CI without a sibling checkout, and the migration-window carve-out dies.

## Operator axis: brew is the only door

The operator never learns the registry exists. The whole journey is two commands:

```bash
brew install augentic/tap/specify   # install (upgrade later via `specify upgrade` or `brew upgrade`)
cd my-project && specify init       # the guided front door — RFC-65 §"Operator onboarding"
```

Hydration first fires inside `specify init` (a native verb on RFC-65's provisioning surface); everything wasm arrives transparently behind it:

- **The core guest rides inside the binary.** Embedded at build time from the same tagged tree (RFC-65 move 4), so there is no core hydration, no core pin, and no skew surface at all: the operator has exactly one version knob, and upgrading the binary *is* upgrading the core — by construction, not by release discipline.
- **Adapters hydrate per RFC-63 as written** — `install_tofu` at the provisioning-surface triggers (init and `specify adapters sync`); the `$HOME/.specify/adapters` store; the committed `.specify/adapters.lock` digest pin; `--frozen` for reproducibility-strict CI; a typed `adapter-not-installed` error (never a guest-side fetch) on a plan-time or runtime store miss. This RFC changes nothing in that design; it supplies the registry backing that makes it work on a fresh machine.

### Tap automation

`upgrade.rs` already plans `brew upgrade augentic/tap/specify`; this RFC makes the formula real and self-updating. A `augentic/homebrew-tap` repo carries a templated formula (per-target archive URLs + `sha256` digests). One job at the tail of `release-binaries.yaml` regenerates the formula from the just-uploaded archives' checksums and commits it to the tap repo (fine-grained token or `repository_dispatch`). The loop closes unattended: tag → binaries → tap bump → `brew upgrade` finds it.

The GitHub Release archives with `.sha256` companions stay as the no-brew fallback (`InstallChannel::Binary` already handles them), and `cargo install --git` stays for Rust-native developers — but brew is documented as *the* path.

## What this RFC refuses to build

Each of these is a tempting complication the standing posture already argues against:

- **A custom registry service.** The well-known file plus GHCR is the whole backend. If GHCR ever becomes the wrong host, the static file is the migration lever — consumers re-resolve, identities never change.
- **Version-range resolution.** Exact pins everywhere (RFC-63's determinism boundary). "Latest" exists only at human decision points: `specify upgrade`'s release probe and `init` choosing a pin. RM-21 keeps ownership of ranges and floors.
- **A dev-tool binary for publishing.** The publish surface is `cargo make` tasks called by workflows (RFC-65's YAGNI posture). A bash loop over `wkg publish` is not a product.
- **Committed wasm.** The adapters repo got there at RFC-64; building the core guest at build time and embedding it through the macro's generic option gets this repo there too. Per the RFC-64 invariant: a slow dev loop is fixed with a path override or fetch-from-registry developer manifest, never a return to committed blobs.
- **crates.io publishing.** Unchanged from `docs/release.md`: the workspace rides `[patch.crates-io]` pins and has no external crate consumers.

## Scope

- The `augentic.io` well-known registry file and the GHCR package backing (public first-party packages under `ghcr.io/augentic/`).
- Publish-auth migration to `GITHUB_TOKEN` in both repos' release workflows; retirement of the registry username/password secrets.
- Idempotent publish loops (skip-if-present) in both repos, factored into `cargo make publish-*` tasks the workflows call.
- The release-build core-guest leg: build-from-tag into the `runtime!` embed option, and deletion of the committed workflow guest, its sidecar, the `tests/dist.rs` gate, and the release-checklist refresh item. (The published-`specify:core` fallback job, if RFC-65 move 4 falls back, slots into the same workflow.)
- The WIT publish job (`specify:adapter`), the adapters-side `wit/deps/` vendored consume with a single pin and a `wit-vendor` task, the sibling-path dev override, and the `check-pins` rewrite from sibling parity to pinned-version parity.
- The `augentic/homebrew-tap` repo, the templated formula, and the automated tap bump at the tail of `release-binaries.yaml`.
- Documentation: `docs/release.md` gains the core-guest and WIT publish legs and the tap bump; the install docs lead with brew.

## Out of scope

- **Hydration mechanics** — the kernel, store root, lock, and `--frozen` are RFC-63's, unchanged.
- **The artifact shape and adapter identity** — RFC-64's, unchanged.
- **The naming cut, the embed mechanism, and the core guest's existence** — RFC-65's; this RFC lands the surrounding publish/acquire plumbing and sequences after it.
- **Version-range resolution, floors, and the compatibility matrix** — RM-21.
- **Third-party adapter namespaces** — the first-party `specify:` posture is unchanged; the well-known mechanism is generic if third parties ever arrive.
- **Store garbage collection** — unchanged from RFC-63's deferral.
- **Private or authenticated consume** — first-party packages are public; a private-registry story (auth on pull) waits for a consumer who needs it.

## Acceptance criteria

1. On a machine with no wasm-pkg configuration, `wkg get specify:omnia@<published>` resolves through `https://augentic.io/.well-known/wasm-pkg/registry.json` to an anonymous GHCR pull, and `specify init` hydrates the same identity through `install_tofu` with no credentials.
2. Neither repo's release workflow references a registry username or password secret; both publish with `GITHUB_TOKEN` only.
3. Re-running either repo's publish workflow against an already-published tag succeeds and pushes nothing: every identity probe reports present, every leg skips.
4. A `specify` tag `v<x.y.z>` produces binaries embedding the core guest built from that same tag through the generic `runtime!` embed option; no `specify:core` registry package exists; the committed `crates/workflow-guest/guest.wasm`, its sidecar, and the `tests/dist.rs` gate are deleted.
5. The adapters repo builds with no sibling checkout: `wit/deps/` carries the pinned published `specify:adapter`, `check-pins` verifies the vendored bytes against the pin, and the RFC-64 migration-window carve-out is gone.
6. `brew install augentic/tap/specify` on a clean macOS machine yields a working binary whose `specify upgrade --dry-run` plans the brew channel; a subsequent release tag updates the tap without human action.
7. `cargo make ci` and `make lint` are green in both repos, and `docs/release.md` describes the full pipeline (binary, core guest, WIT, adapters, tap) without a manual refresh step for any committed artifact.

## Risks and invariants

- **The well-known file is load-bearing.** Every consume path resolves through `augentic.io`; the file must be served with high availability (a static host or CDN — it changes only on a backend migration). Outage degrades to the existing overrides: a project pins `.specify/wasm-pkg.toml` at `ghcr.io` directly and nothing else changes.
- **GHCR is an implementation detail, and must stay one.** No identity, no lockfile digest, and no prose outside this RFC and the workflows names `ghcr.io`; the `specify:` identities and `augentic.io` host are the stable surface. Migrating hosts is editing one JSON file and re-pushing packages — digests in `adapters.lock` verify content equivalence across the move.
- **Idempotency is the immutability enforcement.** Skip-if-present is what prevents a re-tag from mutating a published version. The probe must distinguish "absent" from "registry unreachable" — a network failure aborts the leg rather than treating the identity as unpublished.
- **The binary↔core-guest lockstep is structural.** The embed makes binary version = core version by construction; the release invariant reduces to "the platform builds compile the core guest from the tagged tree", which the build dependency enforces. The adapter `specify-floor` discipline remains the runtime backstop for adapter skew.
- **The tap bump must not become a second release process.** The formula is regenerated from release artifacts, never hand-edited; if the bump job fails, re-running it is safe (same archives, same digests).
- **Sequencing.** The registry backing, `GITHUB_TOKEN` migration, and idempotent loops land first — they are independent of RFC-65 and immediately useful. The core-guest embed rides RFC-65's Omnia dependency; if the embed option falls through, the fallback `specify:core` publish job slots into the same workflows without touching this RFC's other legs. The WIT ownership flip rides the RFC-65 naming cut. Tap automation is orthogonal and lands whenever.
