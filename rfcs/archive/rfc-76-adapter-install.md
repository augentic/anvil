# Adapter Publish and Install — Initial Internal Cut

> Status: Implemented (archived). Phases A–D complete; Actions GHCR publish landed; CI no-repush + attestations remain under [RFC-77](../rfc-77-release-process.md) Phase B.
>
> Owns: the minimum first-party publish/install loop, automatic adapter download, and a working `cargo install --git` bootstrap for internal developers.
>
> Builds on: [RFC-71](../rfc-71-deployment.md) Stages 1+3 (landed). First-party slice of [RM-21](../roadmap.md#rm-21-adapter-ecosystem-operating-model).
>
> Defers: GitHub Actions publish automation (attestation, CI no-repush), production supply-chain hardening, third-party registries and trust, discovery ([RFC-87](../rfc-87-detached-changes.md#source-adapter-selection)), private-registry credentials, semver ranges, and polished public installers.

## Intent

Close one small internal loop:

```text
cargo install Emery
    → manually publish an adapter
    → reference emery:<name>@<version>
    → Emery downloads it on first use
    → later runs reuse the local install
```

The initial users are Augentic developers, the publisher and registry are controlled by Augentic, and exact package versions are chosen explicitly. This cut should establish the right transport and ownership boundaries without building the eventual public ecosystem in advance.

Prove that loop with **operator-driven** publication (`cargo make adapter` / `cargo make publish`) and **automatic** install. Do not automate releases in GitHub Actions until the manual publish path and pull-on-miss install are satisfactory — that automation is the next cut, not this one.

The sharpest day-to-day payoff is universal pull-on-miss: today a changed target pin fails at build or merge until the operator re-runs init; after this cut every routed package resolution can install.

Local adapter development remains `build → adapter add → bare-name resolve` and does not require publication.

The design below is the internal cut. Scope exclusions, trust assumptions, and follow-on hardening are at the end under [Scope boundaries](#scope-boundaries).

## Current gaps

The existing implementation is close but does not form one dependable loop:

- `build.rs` embeds an empty engine placeholder when the wasm32 engine was not built first, so plain `cargo install --git …` produces a binary that cannot boot;
- adapter release CI and docs reference `cargo make adapter` and `cargo make publish`, but those tasks are absent in the current adapters checkout (this cut restores the Make tasks for local use; repairing and running the Actions publish workflow is next);
- publication is described through wasm-pkg/OCI while hydration uses a custom guest-side HTTP path;
- the engine guest owns download and global-store writes while the native launcher only verifies and loads;
- target adapters hydrate during init, but later build/execute paths resolve only, so a changed pin can fail until re-init;
- project-local wasm-pkg configuration can redirect the first-party namespace.

## Decisions

| # | Decision | Initial consequence |
| - | -------- | ------------------- |
| D1 | Make `cargo install --git https://github.com/augentic/emery --locked` build and embed the engine guest. | Internal developers get the requested one-command source install; prebuilt public installers remain later work. |
| D2 | Publish adapters as standard Wasm OCI artifacts in public GHCR. | Publish and consume one standard protocol; no custom CDN or raw-file layout. Warg is archived — OCI is the ecosystem's consolidated direction, not a stopgap. |
| D3 | Pull missing package adapters in the native launcher resolver. | OCI and global-store mutation leave the engine guest; every routed package resolution can install. |
| D4 | Retain the current identity-named global store and digest sidecar; record the OCI repository and manifest digest on every install. | No CAS migration in the initial cut; the sidecar's existing optional digest slot carries the registry provenance. |
| D5 | Hard-code the first-party `emery:` mapping to Augentic GHCR. | A project cannot redirect trusted first-party executable bytes. |
| D6 | Keep exact SemVer selectors and lockstep `emery-adapters` `[workspace.package]` versioning. | Every first-party adapter publishes under that shared Cargo SemVer. No ranges, channels, independent-release redesign, or solver. That package version is independent of the WIT `emery:adapter@…` contract version and of the host `emery` binary / project pin. |
| D7 | Restore thin `cargo make adapter` and `cargo make publish` tasks in `emery-adapters` for **local** operator use. | Publication stays close to adapter source; no engine publish verbs. The dangling Actions workflow that invokes a missing `publish` task is left alone until the automate-releases follow-on. |
| D8 | Keep `adapter add` and component selectors unchanged. | The no-registry development loop and wasm example continue to work. |
| D9 | Defer GitHub Actions publish automation (workflow repair, CI no-repush, publish-time attestation generation). | Refine manual publish + automatic install first; automate releases only after that loop is satisfactory. Attestation generation cannot be backfilled, so it lands with the Actions cut — not later than that — but it is not a gate on proving install. |

## Binary bootstrap

The initial audience already has Rust and can tolerate a source build. Make the requested command correct:

```bash
cargo install --git https://github.com/augentic/emery --locked
```

The native build must produce the wasm32 engine before `include_bytes!` runs. The minimum stable-Cargo implementation is:

1. have `build.rs` invoke the same Cargo executable for `--lib --target wasm32-wasip2` (a release/CI env override pointing at a prebuilt engine existed in the initial cut and was later removed as unused — release legs run the same child build);
2. use an isolated target directory under `OUT_DIR` to avoid the parent Cargo target lock — set `CARGO_TARGET_DIR` explicitly for the child, never merely unset it, which still deadlocks for users with `build.target-dir` in their Cargo config;
3. sanitize the inherited Cargo environment for the child (`CARGO_*`, `RUSTFLAGS`, `RUSTUP_TOOLCHAIN`) and propagate `--locked` so the install command's promise holds through the recursion;
4. guard the wasm32 child build from recursively building another engine;
5. embed the resulting non-empty component;
6. fail with a direct instruction (`rustup target add wasm32-wasip2`) if the target is unavailable or the child build fails.

These constraints are the accumulated scar tissue of Substrate's `wasm-builder`, the longest-lived implementation of this pattern; deviate from them knowingly or not at all.

This is intentionally an internal bootstrap, not the final public installer. It adds build time and requires the wasm target. The existing release workflow remains the preferred way to produce distributable platform binaries.

Do not check a generated engine component into git, silently retain the empty placeholder in an installed binary, or download the engine at first launch. Replacing the Emery binary must continue to replace its engine atomically.

When the audience expands beyond Rust-equipped developers, add a small installer or package-manager formula over the existing GitHub Release archives rather than extending this source-build path. `cargo-binstall` metadata in `Cargo.toml` over those same archives is the cheapest first rung — metadata only, no new pipeline — ahead of a curl installer or Homebrew formula.

## Adapter publication

### Identity and OCI mapping

The logical identity remains:

```text
emery:<name>@<exact-semver>
```

The launcher maps it to one fixed first-party OCI location:

```text
emery:omnia@1.2.0
    → ghcr.io/augentic/emery-adapters/omnia:1.2.0
```

The exact repository prefix is a compiled launcher constant. `.emery/wasm-pkg.toml` is not consulted for the `emery:` namespace in this cut.

The OCI manifest is only a transport envelope. Adapter metadata remains the component's WIT `metadata` export; no adapter manifest is added.

### Artifact format

Use the standard Wasm OCI artifact layout:

- OCI image manifest;
- `application/vnd.wasm.config.v0+json` config;
- one `application/wasm` component layer;
- `architecture: wasm`;
- `os: wasip2`.

Use `wkg oci push` for publication and Bytecode Alliance `oci-wasm`/`oci-client` for native pulls. Do not shell out to `wkg` from Emery.

`wasm-pkg-client` — the library `wkg` itself is built on — was considered for the pull side: it would make the `wkg oci pull` parity criterion true by construction. It loses to `oci-wasm` because it carries the config-file and well-known-URI registry-discovery machinery that D5 exists to exclude; the round-trip acceptance test carries the parity guarantee instead. Revisit if third-party namespaces ever need configurable registry mapping.

### Manual publish (this cut)

Restore these **local** surfaces in `augentic/emery-adapters`:

| Surface | Role |
| ------- | ---- |
| `cargo make adapter <name>` | Build one release component. |
| `cargo make release` | Build the current release set and run existing checks. |
| `cargo make publish <name>` | Push one built component to its exact GHCR SemVer tag (operator machine, after `gh auth` / `docker login` to GHCR as documented). |

The helper derives the name and version from the adapter crate (`CARGO_PKG_VERSION`, which is the shared `[workspace.package]` SemVer), logs into GHCR, and invokes `wkg oci push`. It refuses to replace an existing version tag. If idempotent same-byte detection is cheap with the selected OCI client, a rerun may skip; otherwise “tag already exists” is an acceptable initial failure.

Three version axes stay distinct (D6):

| Axis | Example role | Used for |
| ---- | ------------ | -------- |
| Adapters workspace Cargo SemVer | `emery-adapters` `[workspace.package]` (lockstep across all first-party adapters) | Package selector, OCI tag, store identity (`emery:<name>@…`) |
| WIT contract | `package emery:adapter@…` in `emery.wit` | SDK / WIT package pin consumed by adapter crates |
| Host CLI | `emery` binary / `project.yaml` pin | Operator runtime; optional `emery-floor` in adapter WIT metadata is the only link to a minimum host version |

Independently versioned per-adapter releases are useful later but unnecessary for proving install.

Document the operator steps (auth, build, publish, `wkg oci pull` round-trip). Do not repair or rely on the GitHub Actions publish workflow in this cut — leave the dangling `cargo make publish` invocation alone until [Phase E](#phase-e--automate-releases-next).

No SBOM, signature scheme, publish-time attestation, or runtime attestation verification is required in this cut. Publication credentials remain operator concerns for the manual path and are never committed.

## Automatic install

### Launcher pull-on-miss

The existing asynchronous Omnia `GuestResolver` becomes the single package installation seam.

For `target:omnia@1.2.0` or `source:intent@1.2.0`, the launcher:

1. parses the routed axis, name, and exact version;
2. checks `$EMERY_HOME/store/<name>@<version>.wasm`;
3. verifies the existing digest sidecar when present;
4. on a miss, maps the package to the fixed GHCR OCI reference;
5. anonymously pulls the manifest and one component layer;
6. verifies the OCI and layer SHA-256 digests;
7. rejects an unsupported manifest, empty/oversized layer, malformed component, or wrong source/target export;
8. atomically writes the existing store entry and digest sidecar;
9. returns the verified component bytes to Omnia.

The sidecar must additionally record the OCI repository and resolved manifest digest (D4). The existing sidecar already carries an optional second digest slot, so this extends the current schema rather than introducing a receipt subsystem. Without that record the link between local bytes and what the registry served is unrecoverable; with it, later tag-drift detection and per-project digest pinning become possible. A separate receipt hierarchy is deferred.

If the store entry is valid, resolution makes no registry request and works offline. A cold miss without network access fails with the package identity and OCI reference.

### Engine changes

Package resolution must dispatch before requiring a guest-visible store file:

1. the engine calls the adapter's WIT `metadata` export by routed id;
2. Omnia asks the launcher resolver for the component;
3. the launcher installs on a miss and returns the bytes;
4. the engine applies existing metadata and CLI-floor checks.

For package selectors, `ensure_*` and `resolve_*` therefore share the same host pull-on-miss behavior. This removes the init-only target hydration assumption: init, source survey/extract, target build, and target merge all work from a cold store.

Delete the package-download branch from guest `project::adapter::ensure` and remove the custom `/adapters/<namespace>/<name>@<version>.wasm` fetch protocol. Stop scaffolding `.emery/wasm-pkg.toml` at init in the same change — a hard cut, not an ignored file that still looks authoritative. Keep local component mirroring in `ensure` for `adapter add` and component selectors.

The launcher's fail-closed sidecar posture (`adapter-sidecar-missing`) is the surviving one: the engine's fail-open tolerance for a missing sidecar in `verify_store_entry` goes with the guest hydration path it served.

The global store becomes host-owned and no longer needs a writable guest mount. The per-project component cache remains guest-visible for the existing local seed path.

The native lab provider remains a static catalog and performs no OCI I/O.

## Local development

The unpublished adapter loop remains:

```bash
cargo make adapter <name>
emery adapter add <path/to/component.wasm>
emery init <name>
```

Bare names and local component selectors resolve only through the project component cache. Package selectors resolve only through the global store/OCI path. No sibling checkout, Cargo `target/`, or fallback probe is added.

Native eval (`cargo make eval` / `cargo make lab`) stays on the static catalog and performs no OCI or store install; release publish and launcher pull-on-miss must not become a prerequisite of that loop. The operator-invoked wasm example continues to load locally built components the same way.

## Update model

This cut has no updater. Operators move forward with the install and pin surfaces already defined.

| Surface | How it advances |
| ------- | --------------- |
| Emery binary | Out of band: re-run `cargo install --git … --locked`, or replace a release binary. The engine is embedded, so replacing the binary replaces the engine atomically. There is no `emery self-update`. |
| Adapters | Change the project's exact SemVer pin. The new `name@version` is a cold miss and installs via pull-on-miss. A verifying store entry for the same pin is never re-fetched from the registry. |
| Compatibility | Existing floors stay as-is: a project `emery` pin or adapter CLI floor newer than the running binary fails with exit 3; the operator updates the binary through its install channel. `emery init --upgrade` bumps the project pin and re-scaffolds; it does not update the installed CLI. |

Old store entries for superseded pins may linger; retention and garbage collection are deferred. Ranges, channels, solvers, and automatic upgrades are out of scope (see [Scope boundaries](#scope-boundaries)).

## Ownership

| Surface | Owns |
| ------- | ---- |
| Engine guest | Selectors, routed ids, metadata/CLI-floor checks, workflow semantics, and local component mirroring |
| Native launcher | Fixed first-party OCI mapping, pull-on-miss, digest verification, global store writes, and component loading |
| `emery-adapters` | Component build, package version, manual OCI publication, and publication credentials |
| GHCR | Public first-party OCI manifests and component blobs |
| Cursor skills | Nothing new; continue to invoke one CLI verb and relay output |

Workspace `registry.yaml` remains membership/topology only. It never indexes or distributes adapter bytes.

## Delivery

### Phase A — Working internal bootstrap

- Make native `cargo install --git … --locked` build and embed the engine.
- Fail rather than embed an unusable placeholder.
- Verify the command from a clean checkout with the documented Rust toolchain.

Exit: the installed binary boots and reports its version without a separately installed engine.

### Phase B — Manual publish one adapter

- Restore `cargo make adapter <name>` and `cargo make publish <name>` for local operator use.
- Document GHCR login and the publish steps in adapters `README` / `AGENTS.md`.
- Publish one adapter manually to public GHCR using the Wasm OCI layout.
- Confirm `wkg oci pull` round-trips the component bytes.

Exit: one exact package version is retrievable anonymously through standard OCI tooling. No Actions workflow required.

### Phase C — Automatic host install

- Add `oci-wasm`/`oci-client` to the native launcher.
- Pull and atomically populate the existing store on a resolver miss.
- Reorder package metadata resolution to dispatch through the host first.
- Delete guest package hydration and its custom HTTP layout.

Exit: a clean store installs the Phase B adapter through a normal Emery command, and a second offline invocation reuses it.

### Phase D — Apply to the first-party set

- Publish the remaining adapters with the same manual Make path.
- Align engine and adapters docs with the implemented commands.
- Add integration coverage for source and target cold misses, local corruption, wrong-axis components, and offline reuse.

Exit: every documented first-party exact pin follows the same automatic install path. Stop here until the manual publish + pull-on-miss loop is satisfactory.

### Phase E — Automate releases (next)

Out of this cut's exit criteria; start only after Phases A–D are proven in daily use:

- Repair the release workflow's dangling `cargo make publish` invocation; wire manual workflow dispatch to the same Make tasks.
- Enforce no-repush in CI (manifest-existence check before push), not only in the local helper.
- Generate SLSA build-provenance with `actions/attest-build-provenance` on each successful push (cannot be backfilled; lands here, not later).
- Confirm `gh attestation verify` accepts published versions.

Exit: an internal developer can publish via Actions with immutable tags and attestations, using the same Make tasks refined in Phase B.

## Acceptance criteria

| Criterion | Observable |
| --------- | ---------- |
| Internal binary install | `cargo install --git … --locked` yields one runnable binary with the engine embedded. |
| Manual publication | An internal developer can build and publish one exact adapter version with documented Make commands. |
| Standard transport | `wkg oci pull` and Emery retrieve the same component from GHCR. |
| Automatic install | A clean store resolves `emery:<name>@<version>` without `adapter add` or re-init. |
| Universal behavior | Source operations and target build/merge all install on a cold miss. |
| Integrity | OCI digest mismatch, local store modification, malformed Wasm, and wrong-axis components fail closed. |
| Offline reuse | A valid installed package resolves with the registry unavailable. |
| Local loop | `cargo make adapter <name> && emery adapter add …` continues to support bare-name development. |
| Native eval | `cargo make eval` / `cargo make lab` still resolve through the static catalog with no registry or store warm-up required. |
| Fixed first-party route | Project configuration cannot redirect the `emery:` namespace, and init no longer scaffolds `.emery/wasm-pkg.toml`. |
| One resolver | No raw-file, sibling-checkout, build-tree, or alternate download fallback remains. |
| Digest record | Every installed store entry's sidecar records the OCI repository and resolved manifest digest. |
| Helper no-repush | A second local `cargo make publish` of an existing version tag fails (or no-ops on identical bytes). |

Phase E (next cut) adds: CI no-repush, publish-time attestation generation, and `gh attestation verify` on published versions.

## Expected touch points

### `augentic/emery`

- `build.rs` — build the wasm32 engine for native source installs;
- `crates/launcher/` — OCI pull-on-miss and existing-store installation;
- `crates/project/src/adapter/` — remove package hydration and dispatch before guest store lookup;
- `src/main.rs` — remove the writable global-store guest mount;
- init scaffold/docs — delete the scaffolded `.emery/wasm-pkg.toml` (`DEFAULT_WASM_PKG_CONFIG`) rather than leaving dead config that looks authoritative;
- integration tests — source and target cold-miss behavior.

### `augentic/emery-adapters`

- `Makefile.toml` and a small helper — restore one-adapter build/publish for local operator use;
- README and `AGENTS.md` — document the manual GHCR publish path and the local seed loop;
- release workflow — **Phase E only**: repair the dangling `cargo make publish` call, CI no-repush, attestation step (`id-token: write`, `attestations: write`).

## Scope boundaries

This RFC deliberately does **not** solve the complete public distribution problem.

### In this cut

- one first-party namespace;
- one public OCI registry;
- anonymous reads;
- **manual** publication via local Make tasks;
- exact SemVer pins;
- SHA-256 integrity;
- automatic install on a cold miss;
- one existing global store layout;
- a source install suitable for Rust-equipped internal developers.

### Not in this cut

- GitHub Actions publish automation (workflow repair, CI no-repush, publish-time attestation generation) — [Phase E](#phase-e--automate-releases-next);
- Sigstore/TUF infrastructure of its own, attestation **verification**, or key rotation;
- content-addressed blob/reference storage;
- mirrors, private registries, or registry credentials;
- third-party namespace delegation;
- per-project registry configuration;
- channels, ranges, dependency solving, or automatic upgrades;
- an eager `adapter install` command;
- a curl installer, Homebrew formula, or `cargo-binstall` metadata;
- a new descriptor or adapter manifest.

### Alternatives considered

**Embed first-party adapters in the binary (rejected).** With lockstep versioning (D6) and a first-party-only set it would close the internal loop with no registry at all, but it couples every adapter fix to an engine release, inflates each platform binary, and — decisively — proves nothing about the transport and ownership seam this cut exists to establish for [RM-21](../roadmap.md#rm-21-adapter-ecosystem-operating-model).

**Cursor Teams as full bootstrap (closed).** Distribute `/emery:*` via Cursor Teams or the marketplace, and have `/emery:init` download the Emery binary and the adapters it needs when missing. Closed for this cut: skill distribution is orthogonal and already exists (`plugins/emery/`); adapter install is host pull-on-miss once the binary exists; binary install from a skill would either violate the ultrathin skill contract or duplicate the deferred public-installer channel (GitHub Release archives → curl / `cargo-binstall` / Homebrew). Revisit only as a Cursor-side *caller* of that shared installer, not as a second distribution root for adapters or engine bytes. Today's init skill may confirm `cargo binstall --git … emery@<version>` when `emery` is missing; that soft bootstrap stays — it does not grow into a release fetcher.

### Trust assumption

The internal trust assumption must be explicit: the runtime trusts that Augentic controls the configured GHCR repositories and does not overwrite released version tags. Local digest verification protects against corruption and post-install modification; it does not authenticate the publisher. Publisher authentication is required before opening the registry to third parties or treating adapters as a public execution surface.

GHCR offers no registry-native tag immutability (unlike ECR or Quay). In this cut the compensating controls are the local publish helper's refuse-to-repush check and every install recording the resolved OCI manifest digest in the store sidecar — the only durable link between local bytes and what the registry served, and the prerequisite for any later tag-drift detection or per-project digest pinning. CI-enforced no-repush and publish-time attestations land in Phase E once the manual path is proven (wasmCloud applies the same immutable-tag + attestation pattern to its first-party GHCR packages).

### Explicitly deferred hardening

Before external publishers or a broad public audience, revisit:

- GitHub Actions publish automation — Phase E (workflow, CI no-repush, attestation **generation**; generation cannot be backfilled, so do not slip it past the first automated publish);
- attestation and publisher identity **verification** (generation lands with Phase E; only checking waits for third-party hardening);
- registry-native immutable-tag enforcement (GHCR has none; helper no-repush now, CI no-repush in Phase E, plus recorded manifest digests);
- per-project digest pinning — trust-on-first-use over the manifest digests the sidecar now records;
- content-addressed blob storage and package receipts;
- concurrent-install locking and crash recovery beyond the existing atomic writer;
- trusted mirrors, private registries, and credential discovery;
- third-party namespace roots and dependency-confusion policy;
- size/resource policy informed by real adapter artifacts;
- SBOMs, vulnerability scanning, retention, and store garbage collection;
- prebuilt installer scripts, Homebrew, and other package-manager channels;
- per-adapter release/version automation;
- eager prefetch and diagnostic commands.

These are expected follow-ons, not hidden requirements for the internal cut.

## References

- [Wasm OCI Artifact layout](https://tag-runtime.cncf.io/wgs/wasm/deliverables/wasm-oci-artifact/)
- [Component Model distribution with wasm-pkg-tools](https://component-model.bytecodealliance.org/composing-and-distributing/distributing.html)
- [Warg deprecation — development moved to wasm-pkg-tools/OCI](https://github.com/bytecodealliance/registry)
- [GitHub Container Registry](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry)
- [OCI references and digest behavior](https://oras.land/docs/concepts/reference/)
- [GitHub Artifact Attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations)
- [wasmCloud first-party GHCR publication (workflow-immutable tags + attestations)](https://wasmcloud.com/docs/wash/registries/)

## Review ask

Confirm the deliberately narrow D1–D9 cut: internal source install, one public first-party GHCR mapping, **manual** publication via Make tasks, exact pins, launcher pull-on-miss with mandatory manifest-digest recording, and the existing store. GitHub Actions publish automation (CI no-repush, attestations) is Phase E — start only after Phases A–D are satisfactory. Attestation verification, digest pinning, CAS, public installers, third-party trust, and independent release sophistication remain later still.

Related: [RFC-71](../rfc-71-deployment.md) · [RFC-71](rfc-71-discovery.md) · [RFC-77](../rfc-77-release-process.md) (host/adapter release lines and coordination) · [RM-21](../roadmap.md#rm-21-adapter-ecosystem-operating-model) · [RFC-75](rfc-75-artifact-locations.md) (archive, locations).
