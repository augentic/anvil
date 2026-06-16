# RFC-49: Repository Topology — Core and CLI Consolidation

> Status: Draft · Execution order: **3rd of RFC-47 → RFC-48 → RFC-49**. Runs after adapters extract ([RFC-48](rfc-48-adapter-packaging-transport.md)), so the layout below reflects a post-RFC-48 tree (no `wasi-tools/` under `/cli/`). · Sibling: [RFC-48](rfc-48-adapter-packaging-transport.md) extracts adapters into the *second* repo; this RFC consolidates the *first*.

## Abstract

The Specify **platform** — skill plugins, docs and authoring standards, and the Rust runtime — collapses into a **single repository**, `augentic/specify`, versioned and released in lockstep. Prose keeps the repo root (`plugins/`, `docs/`, `.cursor-plugin/`); the Cargo workspace moves wholesale under `/cli/`.

Today's split — `augentic/specify` for prose, `augentic/specify-cli` for the binary — buys nothing the two halves use. They change together, must be version-compatible at every commit, and are already wired across the repo boundary by a source pin, a cargo-script resolver, and a branch-matched CI checkout: scaffolding that *simulates* one repo. This RFC deletes the scaffolding by making it one repo.

This is the **consolidating** half of a two-repo end-state. Its sibling [RFC-48](rfc-48-adapter-packaging-transport.md) is the **extracting** half: adapters leave for `augentic/specify-adapters` as independently-versioned registry artifacts. Net result is **two** repos — one lockstep platform, one independent adapter ecosystem.

## Motivation

The two repos sit on **separate version lines** while being functionally lockstep — plugins ship at marketplace `0.27.0`, the runtime at workspace `0.2.0` — and nothing in those numbers expresses the compatibility they require. The coupling is carried instead by machinery whose only job is to keep two repos in step:

- **A source pin** — `Specify.toml` pins the CLI by git / tag / path; co-development needs a gitignored `Specify.local.toml` `cli = { path = "../specify-cli" }`.
- **A cargo-script resolver** — `make lint` runs `cargo +nightly -Zscript scripts/specify.rs`, which reads that pin, *builds* the pinned CLI, and runs `lint framework`; `scripts/use-local-dev.rs` does the same for the plugin cache and WASI tools.
- **A branch-matched CI checkout** — `.github/workflows/ci.yaml` resolves a same-named `specify-cli` branch (falling back to `main`), builds it `--release`, then runs `specify lint framework --framework-root .`.
- **A cross-repo schema pin** — `marketplace.json`'s `$schema` points at `specify-cli/raw/main/schemas/authoring/marketplace.schema.json`.

Consolidation removes all four rather than maintaining them.

## Background

### What each repo holds today

- **`augentic/specify`** — `plugins/` (skills + references), `docs/`, `adapters/`, `rfcs/`, `evals/`, the `.cursor-plugin/` marketplace manifest, the `Specify.toml` pin, and `scripts/` shims.
- **`augentic/specify-cli`** — a Cargo workspace: the root `specify` binary, `crates/{error,schema,diagnostics,model,tool-manifest,tool,standards,workflow}`, the embedded `schemas/`, `tests/`, `DECISIONS.md`, the Rust `docs/standards/`, and the separate `wasi-tools/` workspace.

### The contract stays a contract

The binary and the prose meet at a **wire contract**: the CLI verbs skills invoke, the kebab-case `error` discriminants they branch on, the journal taxonomy they emit, and the embedded JSON Schemas artifacts validate against — specified in `workflow.md`. Consolidation does not dissolve that contract; it stops being a *cross-repo* contract between prose and binary and remains the contract between the platform and (a) downstream consumer projects (via `project.yaml.specify_version`) and (b) the extracted adapters repo. The seam moves; it does not vanish (T5).

### Why this consolidates but adapters extract

[RFC-48](rfc-48-adapter-packaging-transport.md) co-locates an adapter's prose with its leaf tool wasm because they are one version-locked unit. That same test *separates* anything **shared-engine-behind-a-contract** from any single consumer — but the platform's prose and runtime are not engine-and-consumer. They are two faces of one product, shipped together to the same audience at the same version, and they fail the *independent-cadence* test that keeps adapters separate. So the runtime co-locates with the prose; adapters (independent semver, RFC-47 identity) deliberately do not. The deciding variable is leaf-and-version-locked vs shared-engine-behind-a-contract, not "prose vs Rust."

## Principles

- **One version per shippable platform.** A consumer pins one number and gets a binary and plugin set known-compatible by construction.
- **Prose owns the root; the engine is quarantined.** Marketplace, skill, and docs paths stay stable; the Rust toolchain and `target/` live under one subdir so prose contributors never trip over them.
- **The contract is explicit, not repo-enforced.** Compatibility comes from `workflow.md` + the embedded schemas + the version line, never from a repo boundary.
- **Delete scaffolding; do not maintain it.** The source pin, cargo-script resolver, and branch-matched CI exist only to fake one repo.
- **Pre-1.0 major cut, no migration framework.** Consistent with [RFC-48](rfc-48-adapter-packaging-transport.md): a re-init cut, not a compatibility-aliased migration.

## Design

### Normative decisions

| ID | Decision | Consequence |
| --- | --- | --- |
| **T1 Single platform repo** | `augentic/specify-cli` folds into `augentic/specify`: one repo, one history, one release, one PR / CI run. | Import the `specify-cli` history under `/cli/` (subtree or `--allow-unrelated-histories` merge); archive `specify-cli`. The cross-repo `rg`-sweep discipline becomes intra-repo. |
| **T2 Layout: prose at root, runtime under `/cli/`** | `plugins/`, `docs/`, `.cursor-plugin/`, `rfcs/`, `evals/` keep the root; the whole Cargo workspace moves to `/cli/`. | One workspace root at `/cli/Cargo.toml`; `target/` and toolchain quarantined. `lint framework --framework-root .` runs from the root and ignores `/cli/`. Nested `AGENTS.md` auto-fences context. See [Repo layout (T2)](#repo-layout-t2). |
| **T3 Single version line** | One platform version replaces the plugin line (`0.27.0`) and the runtime line (`0.2.0`). | Adopt `0.27.0` (the user-facing marketplace number); the internal `0.2.0` line retires. One tag moves marketplace `version`, every `plugin.json` `version`, the Cargo workspace `version`, and consumer `specify_version` together. See [Version unification (T3)](#version-unification-t3). |
| **T4 Tooling + CI collapse** | Delete the source pin, the cargo-script shims, and the branch-matched CI checkout. `make lint` builds the in-tree binary; CI becomes one job; the marketplace `$schema` becomes a relative in-repo path. | `make lint` → `cargo run -p specify -- lint framework --framework-root .`. `ci.yaml` drops resolve-version / sibling-checkout / build-sibling. See [Tooling and CI collapse (T4)](#tooling-and-ci-collapse-t4). |
| **T5 Contract relocates, not dissolves** | The workflow contract stays the consumer- and adapter-facing surface; the only remaining cross-repo seam is platform ↔ `augentic/specify-adapters`. | The branch-matched-CI pattern relocates to the adapters repo. `workflow.md` and `DECISIONS.md` stay the durable spec, now intra-platform. See [Contract seam relocates, not dissolves (T5)](#contract-seam-relocates-not-dissolves-t5). |
| **T6 Adapters remain the second repo** | Consolidation does not absorb adapters; they extract per [RFC-48](rfc-48-adapter-packaging-transport.md) (D7 / D10 / D12). | `wasi-tools/{contract,vectis}` leave the runtime for the adapters repo (RFC-48 D10), so `/cli/` ships no `wasi-tools/` workspace. |

### Repo layout (T2)

Prose owns the root (every marketplace, skill, and docs path unchanged); the Cargo workspace is quarantined under `/cli/`:

```text
augentic/specify/                     # the platform — ONE version line, ONE release
├── .cursor-plugin/marketplace.json   # unchanged paths; $schema → relative in-repo path (T4)
├── .cursor/rules/
├── plugins/                          # PROSE — skills + references (spec, capture, client)
│   └── spec/.cursor-plugin/plugin.json
├── docs/                             # explanation / reference / contributing / authoring standards
├── rfcs/  branding/  evals/
├── cli/                              # THE ENGINE — self-contained Cargo workspace
│   ├── Cargo.toml                    # workspace root + `specify` binary crate
│   ├── Cargo.lock  rust-toolchain.toml  Makefile.toml  deny.toml
│   ├── src/runtime/                  # CLI dispatch
│   ├── crates/{error,schema,diagnostics,model,tool-manifest,tool,standards,workflow}
│   ├── schemas/                      # embedded JSON Schemas (the contract surface)
│   ├── tests/{…, rust_quality}
│   ├── DECISIONS.md
│   └── docs/standards/               # Rust standards (style, coding-standards, handler-shape, workflow.md)
├── AGENTS.md                         # platform + workflow context (root)
├── Makefile                          # `make ci` → (cd cli && cargo make ci) && specify lint framework
└── README.md

#  wasi-tools/{contract,vectis}  →  relocate to augentic/specify-adapters (RFC-48 D10)
```

Why prose-at-root, engine-under-`/cli/` (not a root-level workspace):

- **Zero churn to the prose surface.** `marketplace.json`'s `pluginRoot: "plugins"`, every `plugin.json` path, the skill cross-links, and `lint framework --framework-root .` keep working untouched.
- **The two `docs/standards/` do not collide.** Authoring standards stay at `/docs/standards/`; Rust standards at `/cli/docs/standards/`.
- **Nested `AGENTS.md` context-fencing works in your favour.** Root carries workflow / vocabulary; `cli/AGENTS.md` carries the crate graph and applies only under `/cli/`.
- **The toolchain is quarantined.** One workspace root, one `target/`, one `rust-toolchain.toml`, one `Makefile.toml`.

(`/cli/` versus `/runtime/` is cosmetic; `/cli/` matches the `specify-cli` lineage and contains `src/runtime/`.)

### Version unification (T3)

Three fields drift independently today: marketplace `0.27.0`, each `plugin.json` `0.27.0`, and the Cargo workspace `0.2.0`. After consolidation they are **one number**, which the consumer `project.yaml.specify_version` pins — so pinning the platform pins a binary *and* a known-compatible plugin set.

- **Which line wins.** Adopt `0.27.0` (the user-facing marketplace number) and retire the internal `0.2.0` line, preserving marketplace continuity for installed users. This is the one operator decision in this RFC; confirm it before the retag.
- **How it moves.** One tagged commit sets the marketplace `version`, every `plugin.json` `version`, and the Cargo `[workspace.package] version` to the same value; there is no path where they diverge.
- **`$schema` continuity.** The marketplace `$schema` stops pointing at `specify-cli/raw/main/...` and resolves to the in-repo `cli/schemas/authoring/marketplace.schema.json`, so a tagged release validates against the schema it shipped with.

### Tooling and CI collapse (T4)

Consolidation deletes the cross-repo scaffolding rather than porting it:

- **Deleted.** `Specify.toml`, `Specify.local.toml`, `scripts/specify.rs`, `scripts/use-local-dev.rs` — the source pin and cargo-script resolvers.
- **`make lint`** builds the in-tree binary (`cd cli && cargo run -q -p specify -- lint framework --framework-root ..`) instead of resolving a pinned sibling; `nightly -Zscript` is no longer required.
- **CI** drops the *resolve-version → checkout-sibling → build-sibling* steps; one job runs `cargo make ci` under `/cli/` and `specify lint framework --framework-root .` over the in-tree prose. The symlink-integrity check stays (or is subsumed by [RFC-48](rfc-48-adapter-packaging-transport.md) D12).
- **`use-local-plugins` / `use-team-plugins`** keep working against the in-tree binary; the WASI-tool build path retires with `wasi-tools` (RFC-48 D7 / D11).

### Contract seam relocates, not dissolves (T5)

Merging the runtime into the prose repo relocates the only cross-repo seam to exactly one place rather than removing the contract:

- **Stays a contract.** `workflow.md`, the embedded schemas, the `error` discriminants, and the journal taxonomy remain the surface downstream consumers and the adapters repo depend on. The prose↔binary half becomes *intra-repo* (one PR, one CI run) without weakening the externally-facing half.
- **The seam moves.** The branch-matched-CI pattern that today couples `specify → specify-cli` relocates to `specify-adapters → specify`: the adapters repo builds or fetches the platform binary and runs `lint framework --framework-root .`. One cross-repo seam, different endpoints.

## Phasing

Effectively **Phase 0** relative to [RFC-48](rfc-48-adapter-packaging-transport.md)'s packaging work — landable first or in parallel:

1. **History import (T1).** Merge `specify-cli` into `augentic/specify` under `/cli/`, preserving history; archive `specify-cli`.
2. **Tooling + CI collapse (T4).** Delete the source pin and cargo-script shims; make CI one job; build the binary in-tree; flip the marketplace `$schema` to an in-repo reference.
3. **Version unification (T3).** Confirm the platform line, move all version fields together, retag.
4. **Seam relocation (T5).** Point the single remaining cross-repo CI pattern at `specify-adapters`.

The `wasi-tools` relocation ([RFC-48](rfc-48-adapter-packaging-transport.md) D10) is the hinge shared with adapter extraction; sequence the two RFCs so it happens once, not twice.

## Alternatives considered

- **Keep core and CLI in separate repos (status quo).** Rejected — dual version lines and a standing cross-repo seam for changes that are inherently lockstep, buying no independent cadence the halves use.
- **Root-level Cargo workspace.** Rejected — drops `target/` beside the prose, collides the two `docs/standards/` trees and the two `AGENTS.md` files, and re-paths the marketplace / skill surface for no gain.
- **One mega-repo (adapters too).** Rejected — adapters carry an independent RFC-47 semver cadence; [RFC-48](rfc-48-adapter-packaging-transport.md) extracts them deliberately. Consolidation is for the lockstep halves only.
- **A monorepo orchestrator (nx / bazel / pants).** Rejected — overkill for one Cargo workspace plus a prose tree; `make` + `cargo make` already cover the combined build.
- **Record this inside RFC-48.** Rejected — repo topology is broader than adapter packaging; RFC-48 D7 defers the platform half here and owns only adapter extraction.

## Non-Goals

- **Adapter extraction mechanics** — owned by [RFC-48](rfc-48-adapter-packaging-transport.md) (D7 / D10 / D12); this RFC only states adapters remain the second repo (T6).
- **A third-party adapter ecosystem** — [RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model).
- **Changing the workflow contract surface** — unchanged; only its repo home moves.
- **Any migration framework** — pre-1.0 this is a re-init major cut.

## References

- `.github/workflows/ci.yaml` — the branch-matched `specify-cli` resolve / checkout / build that collapses to an in-tree build (T4).
- `Specify.toml`, `Makefile`, `scripts/specify.rs`, `scripts/use-local-dev.rs` — the source pin and cargo-script resolvers removed (T4).
- `.cursor-plugin/marketplace.json` and `plugins/*/.cursor-plugin/plugin.json` — the plugin version line and the cross-repo `$schema` (T3 / T4).
- `augentic/specify-cli` `Cargo.toml` — the workspace that moves wholesale to `/cli/` (T2) and the `0.2.0` line that retires (T3).
- [`specify-cli` `docs/standards/workflow.md`](https://github.com/augentic/specify-cli/blob/main/docs/standards/workflow.md) and [`DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md) — the workflow contract, now intra-platform (T5).
- [RFC-48](rfc-48-adapter-packaging-transport.md) — the sibling extracting half.
- [Roadmap RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model) — the adapter-ecosystem item the second repo serves.
