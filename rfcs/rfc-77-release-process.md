# Release Coordination Across Emery and First-Party Adapters

> Status: Accepted — in force. Remaining: Phase B (`wkg` automation, CI no-repush + attestations); Phase C (consumer-ready crates.io, cadence/support windows if demand appears).
>
> Owns: the order in which host, WIT contract, and the first-party adapter train release when they move together, and the compatibility signals that let them move apart.
>
> Per-repo cut/patch/publish mechanics: shared `augentic/.github` workflows; operator steps in [`docs/release.md`](../docs/release.md).

## Problem

Emery ships from two repositories under three version identities:

- the host binary (`emery`, embedding `emery:engine@<host-semver>`),
- the adapter WIT contract (`emery:adapter@<wit-semver>`),
- the first-party adapter train (`emery:<name>@<train-semver>`, one lockstep workspace version).

The identities are deliberately independent — most releases touch only one of them. But nothing structural stops the failure cases:

1. **Adapters released against an unsettled seam.** A train built against an unpublished WIT revision, or an engine commit on `main` that changed the seam, produces components no released host can run.
2. **No machine-checkable link between a train and the host it needs.** Without a declared minimum host, an operator can pair any adapter version with any binary and discover the mismatch at dispatch time.
3. **No operator-readable record of what was tested together.** Independent SemVers mean equal numbers imply nothing; something else must say which pairings are known-good.

## Solution

Keep the three identities independent, and couple them only through four explicit mechanisms.

### 1. One release shape per release

Every release declares exactly one shape; the shape fixes the cross-repo order:

| Shape | Trigger | Order |
| ----- | ------- | ----- |
| **WIT-breaking** | `package emery:adapter@…` moves | 1) engine release branch + publish WIT 2) engine publish 3) adapters bump pin + train release 4) announce hard-cut / re-init when product policy requires it |
| **Host-only** | CLI / lifecycle / engine guest; WIT unchanged | engine cut → publish; raise adapter floor only when a train needs the newer host |
| **Adapter-only** | prompts, rules, target behavior; seam unchanged | adapters cut → publish |

The invariant behind all three: an adapter train releases only against a **published** WIT and a **released (or RC)** engine revision. A WIT-breaking landing is recorded as a publication set ([RFC-88](rfc-88-publication-sets.md)).

### 2. Adapter-train gates before publish

1. Tree builds against a published `emery:adapter` WIT pin.
2. CI green against a released (or RC) engine revision: engine crates pinned to a release tag (`tag = "vX.Y.Z"` in the adapters root `Cargo.toml`), sibling path `[patch]` commented out.
3. Every adapter's `emery-floor` names the minimum host that can run the train.
4. Each GHCR version tag is published once — bump the version for new bytes (checklist today; CI probe in Phase B).
5. Once source selection lands ([RFC-87](rfc-87-detached-changes.md#source-adapter-selection)): every identity named by the engine's first-party selector profiles is on the coordinated train.

The tag pin makes gate 2 structural: the adapters tree cannot silently build against a floating engine `main`.

### 3. `emery-floor` as the runtime compatibility check

Each adapter's WIT `metadata` export declares its minimum host version. The host enforces it at resolve time (`adapter-cli-too-old`, exit 3). This is the machine-checkable half of compatibility; exact pins in `project.yaml` / `plan.yaml` are the operator-chosen half.

### 4. Compatibility row in every release's notes

Every host and adapter `RELEASES.md` entry states what was tested together:

```text
engine 0.37.x  ↔  adapters 0.11.x  (WIT emery:adapter@0.1.0, floor ≥ 0.37.0)
```

One row per release. This is a record, not a solver.

## Supporting policy

- **Release lines** — both repos use the shared `augentic/.github` cut/patch/publish workflows on durable `release-X.Y.Z` branches; product jobs wrap publish (engine: binary archives + `crates.yaml`; adapters: GHCR via `adapters.yaml`; maintainer runs `wkg publish` for `emery:engine` / `emery:adapter`).
- **Cadence and support** — on-demand releases; support the latest released line, with optional N−1 for critical/security once external pins exist. Patches are bugfix/security: land on `main` first when applicable, then backport.
- **Pre-1.0 SemVer** — minor may be breaking; patches stay compatible within the line. Hard major-cut / re-init remains product policy, announced in release notes.
- **Cursor `/emery:*` plugin** — versions with `plugins/` content, on its own marketplace / `plugin.json` SemVer.
- **Registry immutability** — never re-publish different bytes under an existing version, on any registry.

## Remaining work

**Phase B** — CI no-repush probe and publish-time attestations on adapter GHCR (`actions/attest-build-provenance`); checklist-gate or automate `wkg publish` for engine + WIT.

**Phase C** (demand-driven) — consumer-ready crates.io publish of the `emery-*` SDK crates (waits on omnia pin hygiene — RM-21); calendar trains; explicit N/N−1 windows; per-adapter SemVer inside the first-party set.

## References

- [`docs/release.md`](../docs/release.md) — operator cut/publish flow
- `augentic/.github` — shared `release.yaml` / `patch.yaml` / `publish.yaml` / `crates.yaml`
- [RFC-76](archive/rfc-76-adapter-install.md) — GHCR publish/install loop, exact pins, lockstep train SemVer
- [RFC-88](rfc-88-publication-sets.md) — WIT-breaking landings as verifiable publication sets
- [RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model) — third-party ecosystem

Related: [RFC-76](archive/rfc-76-adapter-install.md) · [RFC-71](rfc-71-deployment.md) · [RFC-88](rfc-88-publication-sets.md) · [RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model) · [docs/release.md](../docs/release.md)
