# Release Process for Emery and First-Party Adapters

> Status: Accepted — Phase A implemented (Omnia-shaped `release-X.Y.Z` lines in both repos via the shared `augentic/.github` workflows, tag-pinned engine deps, real `emery-floor` values; GHCR / `wkg` publish stays manual); Phases B and C deferred
>
> Owns: how `augentic/emery` and `augentic/emery-adapters` cut, publish, and patch releases; the three version axes (host, WIT contract, adapter train); coordination order when those axes move together.
>
> Builds on: [RFC-76](rfc-76-adapter-install.md) (publish/install loop, exact pins, lockstep first-party adapter SemVer, deferred Actions automation). Complements [RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model) (ecosystem operating model) and the dual-repo seam note in [roadmap.md](roadmap.md#cross-repo-coordination).
>
> Defers: Wasmtime-style calendar trains and LTS windows; multi-line long-term support; per-adapter independent SemVer inside the first-party set; SDK crate publication to crates.io (until third-party SDK consumers exist — RM-21); third-party registry release policy; curl installer / automated Homebrew formula bump; attestation and publisher-identity verification (RFC-76 Phase E and later).

## Intent

Give Emery a release process that matches how the product is already designed — a host binary, an independently versioned WIT contract, and OCI-published adapter components — without inventing a second process family next to Omnia, and without forcing a single lockstep version across both repositories.

One-line summary:

```text
Omnia-shaped release branches for the host;
VS Code / Terraform-shaped independent trains for adapters;
WIT as the protocol version that couples them when it must.
```

## Background

### Versioned surfaces today

| Surface | Owner | Identity | Ship path |
| ------- | ----- | -------- | --------- |
| CLI + embedded engine guest | `augentic/emery` | workspace SemVer / `v*` tag | GitHub Release platform archives |
| Engine guest package | `augentic/emery` | `emery:engine@<host-semver>` | manual `wkg publish` |
| Adapter WIT contract | `augentic/emery` | `emery:adapter@<wit-semver>` | manual `wkg publish`; versions independently of the binary |
| First-party adapters | `augentic/emery-adapters` | shared `[workspace.package]` SemVer → `emery:<name>@<semver>` | manual GHCR via `cargo make publish` |
| Cursor `/emery:*` plugin | `augentic/emery` `plugins/` | co-bumped with host | marketplace with the engine release |

Compatibility today is exact adapter pins plus an optional adapter `emery-floor` (host-CLI minimum). Floors are declared but unused in practice (`None` on every first-party adapter). Immutable registry identities are policy: never re-publish different bytes under an existing version.

Product policy already states that crossing a major is a hard cut (no silent aliases, no migration framework); pre-1.0, a hard cut means re-init. That remains a product rule, not a git-branch rule.

### Process today

- **Omnia** uses shared `augentic/.github` workflows: cut `release-X.Y.Z` from `main` → bump `main` → stabilize and backport on the branch → publish/tag from the branch → patch on the same branch. Manual dispatch; no calendar train; `RELEASES.md` lives per release line.
- **Emery** uses a PR-from-`main` path (`release/v*` → merge → tag). `patch.yaml` expects `release/*` branches, but `publish.yaml` tags only when a `release/v*` PR merges to `main`. That is not a durable release-line model, so honest patches are awkward.
- **Adapters** have no release-branch ritual yet; publication is local and manual (RFC-76 Phase E defers Actions automation).

### Prior art considered

| Model | Shape | Verdict for Emery |
| ----- | ----- | ------------------- |
| [Wasmtime](https://docs.wasmtime.dev/contributing-release-process.html) | Calendar cut → stabilize → publish; patches from `release-*`; later LTS | Mechanics excellent; monthly train + multi-line LTS too heavy pre-1.0 |
| Omnia / shared org workflows | Wasmtime shape without calendar or LTS | Best direct template for the host repo |
| Kubernetes | Long-lived `release-X.Y`, cherry-pick bureaucracy, ~1y support | Overkill until external users stuck on old pins |
| wasmCloud train | Biweekly cut from `main`, build-once-promote, co-versioned monorepo | Cadence idea later; wrong as the only model for dual-repo OCI packages |
| VS Code extensions | Host + independently versioned plugins; `engines.vscode` floor | Closest *compatibility* model for adapters (`emery-floor`) |
| Terraform providers | Protocol version ≠ CLI version ≠ provider version | Closest *axis* model for WIT + host + adapters |
| Lockstep CLI+plugin (some small CLIs) | One SemVer for every artifact | Fights exact pins, independent GHCR tags, and independent WIT |

## Current gaps

- Emery’s patch path does not match Omnia’s durable `release-X.Y.Z` lines, so the two Augentic runtimes do not share operational muscle memory.
- Host, WIT, and adapter trains can move independently, but there is no written coordination order for the three release shapes (WIT-breaking, host-only, adapter-only).
- Adapter releases are not gated on a published WIT pin or a released (or RC) engine revision.
- `emery-adapters` consumes the engine crates (`adapter`, `native`, `probe`, `prose`) as unpinned git dependencies on emery’s default branch, held only by the committed `Cargo.lock` — the seam gate in D9 has no structural enforcement in `Cargo.toml`.
- `emery-floor` is not used as a real compatibility signal.
- There is no short compatibility table operators can read when choosing pins.
- RFC-76 correctly defers Actions publish automation; this RFC owns the *process* that automation must eventually encode.

## Decisions

| # | Decision | Consequence |
| - | -------- | ----------- |
| D1 | Treat three version axes as independent identities: host (`emery` / `emery:engine`), WIT (`emery:adapter`), adapter train (`emery:<name>@…`). | Never force `emery` and `emery-adapters` to share a SemVer. Process verbs align; version numbers do not. |
| D2 | Adopt Omnia’s release-branch model for `augentic/emery`: cut `release-X.Y.Z` → stabilize → publish from the branch → patch from the branch. | Replaces tag-from-`main` hybrid; enables honest patches; reuses org workflows where practical. |
| D3 | Keep first-party adapters on lockstep `[workspace.package]` SemVer (RFC-76 D6), released as an independent train with the same branch verbs. | One operator-facing first-party train version; no per-adapter SemVer redesign in this cut. |
| D4 | Pre-1.0 cadence is on-demand, not calendar. | No Wasmtime 5th/20th train until demand justifies it. |
| D5 | Support window starts at **latest released line only**; optional N−1 for critical/security once external pins exist. | No LTS, no Kubernetes-length multi-branch support. |
| D6 | Patches are bugfix and security only: land on `main` first when applicable, backport, no features, no WIT breaks, no CLI wire breaks. | Matches Omnia/Wasmtime patch discipline without their support matrix. |
| D7 | Cursor `/emery:*` plugins co-version with the host release. | Plugins are host distribution surface, not adapter packages. |
| D8 | Every adapter train release declares a real `emery-floor` when it depends on host behavior; stop shipping meaningful releases with floor `None` once support matters. | Floors become the VS Code `engines` analogue. |
| D9 | Gate every adapter train release on a published WIT pin and CI against a released (or RC) engine revision — not only a sibling `main`. | Prevents adapters that only build against unpublished seam changes. |
| D10 | Document three release shapes and their order (below). | Humans pick a shape; automation later encodes the same checklist. |
| D11 | Keep hard major-cut / re-init product policy unchanged. | Release branches handle compatible maintenance; hard cuts remain product events, not endless backport obligations. |
| D12 | Defer Actions automation of adapter GHCR publish and `wkg publish` until the branch ritual is written and used manually (aligns with RFC-76 Phase E). | Process first; CI encodes process second. |
| D13 | Once durable engine release lines exist, `emery-adapters` pins its engine git dependencies by release tag (`tag = "vX.Y.Z"`), not a floating default branch. | D9 becomes enforceable in `Cargo.toml`, not just checklist discipline; the committed sibling `[patch]` block remains the co-development escape hatch. Works against a private repo too — tag pins do not depend on repo visibility. |

## Three version axes

```text
emery:adapter@WIT     ← protocol (rare; breaking = hard cut)
emery binary / engine ← host product line
emery:<name>@semver   ← first-party plugin train
         └── metadata.emery-floor → minimum host
```

- **Host SemVer** is the operator-facing `emery` version (`project.yaml` pin, GitHub Release, embedded engine, `emery:engine@…`).
- **WIT SemVer** is the `package emery:adapter@…` declaration. It versions the component contract, not the CLI.
- **Adapter train SemVer** is the shared adapters workspace version published to `ghcr.io/augentic/emery-adapters/<name>:<version>`.

Compatibility between host and adapters is declared (`emery-floor` + exact pins), not implied by equal numbers.

## Engine process (`augentic/emery`)

Reuse Omnia’s shape. Prefer shared `augentic/.github` release/patch/publish workflows over a Emery-only fork when the inputs fit.

### Cut

1. From `main`, create durable branch `release-X.Y.Z` at the snapshot to ship.
2. Open a PR that bumps `main` to the next unreleased version and resets `RELEASES.md` for the next line (Omnia pattern).
3. Edit release notes on the release branch, not on `main`.

### Stabilize

On the release branch only:

- omnia pin hygiene (`cargo build --locked` on a clean runner; no sibling path patches);
- operator rungs (`cargo make wasm-run`, `cargo make eval`) when the change warrants them;
- backports from `main` for fixes that must ship on this line.

All fixes land on `main` first when applicable, then backport.

### Publish

From the release branch:

1. Date the release notes and tag `vX.Y.Z`.
2. Create the GitHub Release; binary matrix attaches as today.
3. Publish `emery:engine@X.Y.Z` with `wkg publish` (manual until automated).
4. If WIT moved on this line, publish `emery:adapter@<wit>` before or with the engine publish — never after adapters that need it have already shipped.

### Patch

On the same `release-X.Y.Z` line:

1. Land the fix on `main` when applicable.
2. Backport to the release branch.
3. Bump patch (`X.Y.Z` → `X.Y.Z+1`) on the branch.
4. Publish as above.

Do not invent a new line from a floating tag. Do not merge patch release PRs to `main` as the publish trigger.

### Semver on `0.x`

Follow Omnia’s pre-1.0 convention: **minor may be breaking**. Patches remain compatible within the line. Hard-cut product policy (re-init, no silent aliases) still applies when the change warrants it — that is called out in release notes, not smuggled into a patch.

## Adapter process (`augentic/emery-adapters`)

Same verbs, independent cadence and SemVer.

### Train

- One shared workspace SemVer for all first-party adapters (RFC-76 D6).
- Cut `release-X.Y.Z` in the adapters repo when the train should ship.
- Publish every first-party component for that version to immutable GHCR tags (`cargo make publish <name>` until Phase E automation).
- Patch on that adapters release line the same way as the host.

### Gates before publish

1. Tree builds against a **published** `emery:adapter` WIT pin.
2. Native CI (and wasm-run when seam-relevant) against a **released or RC** engine revision.
3. Engine git dependencies pinned by release tag (D13) — no floating-branch resolution at publish time, and no active sibling `[patch]` block.
4. `emery-floor` set to the minimum host that can run this train.
5. Existing no-repush probe: refuse to replace an existing GHCR version tag.
6. Once adapter descriptors land ([RFC-71](rfc-71-discovery.md) D3): every published component has a descriptor in the projected index, and the restated fields agree with its `metadata` export.

### What stays independent

Adapter-only prompt, rule, or target-behavior changes ship an adapter train without an engine bump. Host-only lifecycle/CLI changes ship an engine release without an adapter train unless the floor must rise.

## Coordination: three release shapes

Every release chooses exactly one shape:

| Shape | Trigger | Order |
| ----- | ------- | ----- |
| **WIT-breaking** | `package emery:adapter@…` moves | 1) engine release branch + publish WIT 2) engine publish 3) adapters bump pin + train release 4) announce hard-cut / re-init when product policy requires it |
| **Host-only** | CLI / lifecycle / engine guest; WIT unchanged | engine cut → publish; adapters unchanged unless floor must rise |
| **Adapter-only** | prompts, rules, target behavior; seam unchanged | adapters cut → publish; engine unchanged |

Never release adapters against an unpublished WIT or an unreleased engine commit that changed the seam.

### Compatibility table

Each host and adapter release notes entry includes a short row:

```text
engine 0.28.x  ↔  adapters 0.5.x  (WIT emery:adapter@0.1.0, floor ≥ 0.28.0)
```

Keep the table short. Do not build a solver.

## Phased delivery

### Phase A — Align process (this RFC’s first cut)

1. Move `emery` onto Omnia-shaped `release-X.Y.Z` + publish-from-branch + patch-from-branch; retire or rewrite the tag-from-`main` / broken patch hybrid in `.github/workflows/{release,publish,patch}.yaml`.
2. Document the three axes and three shapes in [`docs/release.md`](../docs/release.md) (operator-facing summary; this RFC remains the design home).
3. Add the same release-branch ritual to `emery-adapters` (publish may stay manual).
4. Switch the `emery-adapters` engine git dependencies from the floating default branch to `tag = "vX.Y.Z"` pins on the new release lines (D13).
5. Start setting `emery-floor` on adapter releases that depend on host behavior.
6. Add `RELEASES.md` (or equivalent) per line if adopting Omnia’s notes layout.

### Phase B — Reduce toil

1. Automate adapter GHCR publish on tag (RFC-76 Phase E: repair workflow, CI no-repush, attestations).
2. Checklist-gate or automate `wkg publish` for engine + WIT.
3. Publish the compatibility row with every release as a required notes section.

### Phase C — Only if users need it

1. Calendar minor trains (biweekly or monthly).
2. Explicit N / N−1 support windows.
3. Optional LTS — only after someone is stuck on an old pin for real.
4. Per-adapter independent SemVer inside first-party — only if lockstep train cost exceeds benefit (third-party adapters already version independently by nature).
5. Publish the SDK-facing crates (`adapter`, plus `native` / `probe` for the test harness) to crates.io under `emery-*` names — only when a third-party adapter author needs them (RM-21). Prerequisite: omnia pin hygiene, since published crates cannot carry a `[patch]` on a git dependency.

## Rejected alternatives

**Full Wasmtime monthly + LTS.** Support-matrix cost dominates a small pre-1.0 team. Revisit when external consumers need guaranteed backports.

**Kubernetes multi-branch cherry-pick theater.** Same cost problem; no evidence of that user base yet.

**Single lockstep SemVer across `emery` and `emery-adapters`.** Contradicts exact pins, independent OCI tags, independent WIT, and the roadmap’s “never a lockstep release” dual-repo note. Would either over-release adapters or block engine ships.

**Tag-only from `main` (current Emery path).** Makes honest patches hard; Omnia already solved the durable-line problem.

**Per-adapter independent SemVer for first-party now.** Useful later for third-party and for shipping one hot adapter; unnecessary while the first-party set is small and co-developed (RFC-76 D6).

**Calendar train before the branch ritual.** Automating cadence on a broken patch path amplifies confusion. Process shape first.

## Relationship to RFC-76

RFC-76 owns *how bytes get to GHCR and into the store* (manual publish, pull-on-miss, exact pins, lockstep first-party SemVer). This RFC owns *when and from which git line those bytes are cut*, and how host / WIT / adapter trains coordinate.

RFC-76 Phase E (Actions publish automation) should implement the adapter half of Phase B here — not invent a third release model.

## Non-goals

- Changing adapter identity, OCI mapping, or pull-on-miss (RFC-76).
- Introducing semver ranges or a version solver.
- Publishing workspace crates to crates.io in this cut — deferred until third-party SDK consumers exist (RM-21, Phase C), not rejected. Generic crate names (`adapter`, `native`, `probe`, `prose`) would need `emery-*` renames first.
- Multi-year support commitments.
- Third-party publisher release policy (RM-21 / RFC-71 later).
- Replacing the hard major-cut / re-init product rule with a migration framework.

## References

- [Wasmtime release process](https://docs.wasmtime.dev/contributing-release-process.html)
- [Wasmtime stability / LTS](https://docs.wasmtime.dev/stability-release.html)
- Omnia shared workflows: `augentic/.github` `release.yaml` / `patch.yaml` / `publish.yaml`
- [Emery `docs/release.md`](../docs/release.md) (current operator flow; to be updated when Phase A lands)
- [RFC-76 Adapter Publish and Install](rfc-76-adapter-install.md)
- [RM-21 Adapter ecosystem operating model](roadmap.md#rm-21-adapter-ecosystem-operating-model)
- VS Code `engines.vscode` compatibility model
- Terraform plugin protocol versioning (protocol ≠ CLI ≠ provider)

## Review ask

Confirm D1–D13: three independent version axes; Omnia-shaped durable release branches for the host; independent lockstep adapter trains with the same verbs; on-demand cadence; latest-line (optional N−1) support; three explicit coordination shapes; floors and published-WIT gates before adapter publish; tag-pinned engine git dependencies in the adapters tree; Actions automation only after the ritual is used manually.

Related: [RFC-76](rfc-76-adapter-install.md) · [RFC-70](rfc-70-deployment.md) · [RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model) · [docs/release.md](../docs/release.md)
