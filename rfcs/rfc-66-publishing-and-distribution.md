# RFC-66: Publishing and Distribution — One Transport, Two Axes

> Status: Proposed · Depends: [RFC-63](rfc-63-adapter-hydration.md) (hydration kernel, store root, `adapters.lock`), RFC-64 (one-component artifact, wasm-pkg publish — landed in `specify-adapters`), [RFC-65](rfc-65-standalone-deployment.md) (the `specify:` naming cut, the binary-versioned core guest) · Owns: how every Specify artifact is published by developers and acquired by operators, across both repos

## Abstract

Every wasm-shaped artifact — the WIT contract, the adapter components, the core guest — travels over **one transport**: wasm-pkg/OCI, backed by GHCR behind a static well-known file at `augentic.io`. Everything native — the host binary — travels over GitHub Releases and Homebrew, optionally carrying its core guest embedded (RFC-65 move 4). Prose rides the artifacts that already have identities: the shared codex compiles into the engine, adapter rule overlays into their components — closing the "prose distribution beyond the component" gap RFC-64 deferred (§"Codex ownership becomes real"). The developer axis collapses to *bump one `Cargo.toml` version, push a tag*: idempotent tag-driven workflows publish only what moved, authenticated by `GITHUB_TOKEN` alone. The operator axis collapses to *`brew install augentic/tap/specify`*: adapters hydrate through the RFC-63 kernel at init and sync, and the core guest carries the binary's own version — pulled at init or already embedded — so upgrading the binary is the only version knob an operator turns. No custom registry service, no pack format, no crates.io, no dev-tool binary, no committed wasm.

## The artifact inventory

| Artifact | Identity | Published by | Acquired by |
| -------- | -------- | ------------ | ----------- |
| WIT contract | `specify:adapter@<ver>` (the RFC-65 rename of `augentic:specify`) | `specify` release workflow, on `package` version change | `specify-adapters` as a pinned `wkg get` into `wit/deps/` |
| Adapter components | `specify:<name>@<ver>` | `specify-adapters` release workflow (RFC-64, as landed) | `install_tofu` hydration into the RFC-63 store |
| Core guest | `specify:core@<binary version>` | `specify` release workflow, same tag as the binary | hydrated at init — or already embedded when the binary opts into the generic `runtime!` embed (RFC-65 move 4) |
| Shared codex (`UNI-*` / `CORE-*` packs) | none — compiled into the engine (§"Codex ownership becomes real") | built from the tagged `specify` tree into the binary and `specify:core` | arrives with the binary / core guest; no separate fetch |
| Adapter rule overlays (`OMNIA-*` / `VECTIS-*` / `SRC-*`, …) | inside `specify:<name>@<ver>` | embedded by the adapter's prose registry at guest build | materialized into the per-project cache at init / `adapters sync` |
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

The existing tag-driven pipeline (`release.yaml` → `publish.yaml` → `release-binaries.yaml`) grows two publish jobs on the same `v*` tag:

1. **Publish `specify:core@<tag version>`** — build the core guest from the tagged tree to `wasm32-wasip2` and `wkg publish` it, idempotent like the adapters loop. Binaries built with Omnia's generic `runtime!` embed option additionally carry the same component compiled in (RFC-65 move 4) and skip core hydration; the publish job still runs regardless, keeping the pulled path universal. Either way this deletes the committed `crates/workflow-guest/guest.wasm`, its `.sha256` sidecar, the `tests/dist.rs` staleness gate, and the "refresh the embedded guest" release-checklist item — the guest is built from the tagged source at publish (or build) time, never committed.
2. **Publish the WIT** — `specify:adapter@<wit version>` via `wkg wit build` + `wkg publish` for `wit/specify.wit`, guarded by a version-changed probe so the job is a no-op on tags that did not bump the WIT `package` declaration.

Every publish leg lives in a `cargo make publish-*` task; the GitHub workflow is a thin caller. Local emergency publishing runs the same task with a developer's own token — one code path, two invocation surfaces.

### WIT ownership becomes real

Today both repos carry byte-identical `wit/specify.wit` copies, and the adapters repo's `check-pins` task does a sibling-checkout `cmp` with an RFC-64 migration-window carve-out. At the RFC-65 naming cut (the natural break point — every reference changes anyway), the relationship flips from *copy parity* to *publish and consume*:

- `specify` owns and publishes `specify:adapter`, as `wit/README.md` already declares.
- `specify-adapters` deletes its `wit/specify.wit` copy in favour of a `wkg get specify:adapter@<pin>` vendored into `wit/deps/`, pinned in exactly one place, refreshed by a `cargo make wit-vendor` task.
- The dev loop keeps a sibling-path override: while iterating on a contract change in `specify`, the adapters build can point at the sibling file before the new version is published.
- `check-pins`'s WIT arm becomes "the vendored file byte-matches the pinned published version" — deterministic in CI without a sibling checkout, and the migration-window carve-out dies.

## Codex ownership becomes real

The engineering-standards rules have the same two-copies disease the WIT had, plus a distribution gap RFC-64 explicitly deferred ("prose distribution beyond the component"). Today the shared codex (`codex/rules/{universal,core}/`) lives as a manually synced source-tree copy in **both** repos: `specify` is the authoring home (its own `make lint` resolves the monorepo probe against it), and `specify-adapters` carries the shipping fork because `cache_codex` / `sync_codex` distribute rules by walking the resolved adapter component's filesystem ancestors for a `codex/rules/universal/` tree — a probe only a *development sibling checkout* can satisfy. A registry-installed store entry is one wasm file with no source checkout, so codex distribution fail-softs to "nothing distributed": the first registry consumer gets silently degraded lint (the `UNI-022` / `UNI-023` synthetics skip per the graceful-degradation posture) and a `--rules-root` workaround as the default experience. The per-adapter overlays (`prose/rules/` — `OMNIA-*`, `VECTIS-*`, `SRC-*`) have the same gap one layer down: the prose registry embeds `prompts` and `references` into each component but not `rules`, and the resolver's manifest-cache overlay rung has no populator on the registry path.

This RFC closes both halves at the same cut, following the WIT flip's shape — one owner, everything else consumes:

- **The shared codex compiles into the engine.** The `universal/` and `core/` packs are built from the tagged `specify` tree into the binary (and therefore into `specify:core`, which is built from the same tree) — the same embed posture as the JSON Schemas in `specify-schema`, and a few dozen small markdown files. The codex is framework policy, so its version knob is the binary's, matching the RFC-65 discipline: upgrading the binary upgrades the rules, and there is no fourth registry identity to publish, pin, or hydrate.
- **Materialization replaces the ancestor walk.** At init and `adapters sync`, the provisioning surface writes the embedded packs into the existing out-of-tree cache layout (`<project-cache>/codex/codex/rules/{universal,core}/`). The resolver's probe order, `SHARED_REL` / `CORE_REL` constants, and derived-root semantics are byte-identical — only where the bytes come from changes. `CodexMeta` provenance re-pins from "adapter source/ref" to the binary version. `rules_root_for_component` (the ancestor walk), the fail-soft path, and the standalone `specify rules sync` verb all delete — re-materialization is `adapters sync`'s job, alongside the manifest regeneration it already owns.
- **`specify-adapters` deletes its `codex/rules/` copy.** The manual two-repo sync discipline dies with it (`codex/references/` — the prose overlay tree embedded into components — is unaffected and stays). Adapter prompts that today cite shared rules by repo-relative path (`build/review.md`'s links into `codex/rules/universal/`) re-point at the consume surfaces that exist in a consumer project: `specify rules export` and the materialized cache — the deterministic-review sections already model this. The adapters repo's own CI never ran `specify lint framework`, so nothing is lost gate-wise; `CORE-009`-family namespace enforcement continues where the rules now solely live.
- **Overlays travel inside the component.** The prose registry's walk gains the `rules` tree, so `specify:<name>@<ver>` carries its own overlay pack in the same compiled-in registry its references already live in; init and `adapters sync` extract it from the hydrated component into the manifest-cache rung the resolver already probes (`<project-cache>/manifests/{sources,targets}/<name>/prose/rules/`). Overlays are thereby pinned to the adapter version by construction; the project-local `adapters/{sources,targets}/<name>/prose/rules/` rung survives untouched as the operator-override layer.

Why not embed the universal pack per-adapter instead: a project binding two adapters would carry two potentially divergent copies of framework-owned policy, and every adapter release would re-ship rules it does not own. Ownership follows authorship, exactly as with the WIT: `specify` authors the shared codex, everything else consumes it — the only difference is that rules need no registry identity because they already travel inside artifacts that have one.

One skew consequence to name: shared rules are now pinned to the binary while overlays are pinned to the adapter, so an older adapter's prompts may cite a `UNI-*` id against a newer embedded pack. The codex file shape already absorbs this — ids are stable citation keys, never renumbered, and retired rules keep their files under a `deprecated:` block — so a cited id always resolves; the `specify-floor` discipline bounds how stale an adapter can be.

## Operator axis: brew is the only door

The operator never learns the registry exists. The whole journey is two commands:

```bash
brew install augentic/tap/specify   # install (upgrade later via `specify upgrade` or `brew upgrade`)
cd my-project && specify init       # the guided front door — RFC-65 §"Operator onboarding"
```

Hydration first fires inside `specify init` (a native verb on RFC-65's provisioning surface); everything wasm arrives transparently behind it:

- **The core guest carries the binary's version.** Pulled at init (`specify:core@<the binary's own version>`) or already embedded when the binary opts into the generic embed (RFC-65 move 4) — either way there is no core pin surface and no second knob: upgrading the binary is upgrading the core, and the two modes are indistinguishable to the operator.
- **Adapters hydrate per RFC-63 as written** — `install_tofu` at the provisioning-surface triggers (init and `specify adapters sync`); the `$HOME/.specify/adapters` store; the committed `.specify/adapters.lock` digest pin; `--frozen` for reproducibility-strict CI; a typed `adapter-not-installed` error (never a guest-side fetch) on a plan-time or runtime store miss. This RFC changes nothing in that design; it supplies the registry backing that makes it work on a fresh machine.

### Tap automation

`upgrade.rs` already plans `brew upgrade augentic/tap/specify`; this RFC makes the formula real and self-updating. A `augentic/homebrew-tap` repo carries a templated formula (per-target archive URLs + `sha256` digests). One job at the tail of `release-binaries.yaml` regenerates the formula from the just-uploaded archives' checksums and commits it to the tap repo (fine-grained token or `repository_dispatch`). The loop closes unattended: tag → binaries → tap bump → `brew upgrade` finds it.

The GitHub Release archives with `.sha256` companions stay as the no-brew fallback (`InstallChannel::Binary` already handles them), and `cargo install --git` stays for Rust-native developers — but brew is documented as *the* path.

## What this RFC refuses to build

Each of these is a tempting complication the standing posture already argues against:

- **A custom registry service.** The well-known file plus GHCR is the whole backend. If GHCR ever becomes the wrong host, the static file is the migration lever — consumers re-resolve, identities never change.
- **Version-range resolution.** Exact pins everywhere (RFC-63's determinism boundary). "Latest" exists only at human decision points: `specify upgrade`'s release probe and `init` choosing a pin. RM-21 keeps ownership of ranges and floors.
- **A dev-tool binary for publishing.** The publish surface is `cargo make` tasks called by workflows (RFC-65's YAGNI posture). A bash loop over `wkg publish` is not a product.
- **A registry identity for rules.** A published `specify:rules@<ver>` would add a pin surface and a skew axis for policy that is framework-owned and binary-versioned. The codex rides artifacts that already have identities — the binary and core guest for the shared packs, each adapter component for its overlay (§"Codex ownership becomes real").
- **Committed wasm.** The adapters repo got there at RFC-64; publishing (or optionally embedding) the release-built core guest gets this repo there too. Per the RFC-64 invariant: a slow dev loop is fixed with a path override or fetch-from-registry developer manifest, never a return to committed blobs.
- **crates.io publishing.** Unchanged from `docs/release.md`: the workspace rides `[patch.crates-io]` pins and has no external crate consumers.

## Scope

- The `augentic.io` well-known registry file and the GHCR package backing (public first-party packages under `ghcr.io/augentic/`).
- Publish-auth migration to `GITHUB_TOKEN` in both repos' release workflows; retirement of the registry username/password secrets.
- Idempotent publish loops (skip-if-present) in both repos, factored into `cargo make publish-*` tasks the workflows call.
- The `specify:core` publish job on the `specify` `v*` tag (with opt-in embed adoption per RFC-65 move 4), and deletion of the committed workflow guest, its sidecar, the `tests/dist.rs` gate, and the release-checklist refresh item.
- The WIT publish job (`specify:adapter`), the adapters-side `wit/deps/` vendored consume with a single pin and a `wit-vendor` task, the sibling-path dev override, and the `check-pins` rewrite from sibling parity to pinned-version parity.
- The codex ownership flip: embedding the shared `universal/` and `core/` packs into the engine build; materialization into the existing `<project-cache>/codex/` layout at init and `adapters sync`; deletion of the ancestor walk (`rules_root_for_component`), the fail-soft distribution path, the standalone `specify rules sync` verb, and the `specify-adapters` `codex/rules/` copy; the prose registry's `rules` tree for overlay embedding and its extraction into the manifest-cache overlay rung; adapter-prompt citation re-pointing.
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
4. A `specify` tag `v<x.y.z>` publishes `specify:core@<x.y.z>`; the binary built from that tag hydrates exactly that identity (or carries it embedded, when the build opted into the generic embed); the committed `crates/workflow-guest/guest.wasm`, its sidecar, and the `tests/dist.rs` gate are deleted.
5. The adapters repo builds with no sibling checkout: `wit/deps/` carries the pinned published `specify:adapter`, `check-pins` verifies the vendored bytes against the pin, and the RFC-64 migration-window carve-out is gone.
6. On a fresh machine with a registry-hydrated adapter (no sibling checkout, no `--rules-root`), `specify init` leaves `specify lint project` and `specify rules export` resolving the full rule set — shared `UNI-*` from the binary-materialized codex cache, overlays from the component-extracted manifest-cache rung — with `codex-meta.yaml` recording the binary version; `specify-adapters` carries no `codex/rules/` tree and no source in either repo walks component ancestors for one.
7. `brew install augentic/tap/specify` on a clean macOS machine yields a working binary whose `specify upgrade --dry-run` plans the brew channel; a subsequent release tag updates the tap without human action.
8. `cargo make ci` and `make lint` are green in both repos, and `docs/release.md` describes the full pipeline (binary, core guest, WIT, adapters, tap) without a manual refresh step for any committed artifact.

## Risks and invariants

- **The well-known file is load-bearing.** Every consume path resolves through `augentic.io`; the file must be served with high availability (a static host or CDN — it changes only on a backend migration). Outage degrades to the existing overrides: a project pins `.specify/wasm-pkg.toml` at `ghcr.io` directly and nothing else changes.
- **GHCR is an implementation detail, and must stay one.** No identity, no lockfile digest, and no prose outside this RFC and the workflows names `ghcr.io`; the `specify:` identities and `augentic.io` host are the stable surface. Migrating hosts is editing one JSON file and re-pushing packages — digests in `adapters.lock` verify content equivalence across the move.
- **Idempotency is the immutability enforcement.** Skip-if-present is what prevents a re-tag from mutating a published version. The probe must distinguish "absent" from "registry unreachable" — a network failure aborts the leg rather than treating the identity as unpublished.
- **The binary↔core-guest lockstep is a release invariant (structural under the embed).** On the pulled path, a binary version whose `specify:core` publish leg failed is a broken release: the workflow must fail the release when the core push fails, not ship the binary without it. Under the optional embed the lockstep holds by construction instead. The adapter `specify-floor` discipline remains the runtime backstop for adapter skew.
- **The tap bump must not become a second release process.** The formula is regenerated from release artifacts, never hand-edited; if the bump job fails, re-running it is safe (same archives, same digests).
- **The materialized codex must track the binary.** A cache written by an older binary is stale policy; provisioning invocations compare `codex-meta.yaml`'s recorded binary version against their own and re-materialize on mismatch — cheap, since the bytes are already in memory. The monorepo probe rung stays ahead of the cache rung, so both framework repos keep linting their working trees, not an embed.
- **Sequencing.** The registry backing, `GITHUB_TOKEN` migration, and idempotent loops land first — they are independent of RFC-65 and immediately useful. The codex ownership flip's shared-pack half is likewise independent and may land ahead of everything: it deletes the two-repo rules sync burden immediately, dev-checkout consumers see no behavior change (the materialized cache replaces the ancestor-walked one), and until RFC-63's `adapters sync` exists, re-materialization hangs off `specify init` and the existing `specify rules sync` (whose verb then retires when `adapters sync` absorbs it). The overlay half rides the next adapter releases. The `specify:core` publish job can land inert (published but unconsumed) to de-risk RFC-65; the optional embed rides Omnia's timetable without blocking anything. The WIT ownership flip rides the RFC-65 naming cut. Tap automation is orthogonal and lands whenever.
