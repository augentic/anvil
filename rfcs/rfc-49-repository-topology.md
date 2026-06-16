# RFC-49: Repository Topology — Core and CLI Consolidation

> Status: Draft · Execution order: **3rd of RFC-47 → RFC-48 → RFC-49**, executed to completion in numerical sequence. Runs *after* adapters have extracted ([RFC-48](rfc-48-adapter-packaging-transport.md)), folding the remaining `augentic/specify` + `augentic/specify-cli` into one lockstep platform repo — so the layout below (no `wasi-tools/` under `/cli/`) reflects a post-RFC-48 tree. · Sibling: [RFC-48: Adapter packaging and transport](rfc-48-adapter-packaging-transport.md) (extracts adapters into the *second* repo; this RFC consolidates the *first*) · Context: today's prose/runtime split — `augentic/specify` (plugins, docs, standards) and `augentic/specify-cli` (the Rust runtime) — coupled by the workflow contract (`docs/standards/workflow.md`, the embedded JSON Schemas, the error / journal taxonomies) and kept in step by a source pin (`Specify.toml`), a cargo-script lint shim (`scripts/specify.rs`), and a branch-matched CI checkout (`.github/workflows/ci.yaml`).

## Abstract

The Specify **platform** — the skill plugins, the docs and authoring standards, and the Rust runtime that executes the deterministic workflow — collapses into a **single repository**, `augentic/specify`, versioned and released in lockstep. Prose keeps the repo root (`plugins/`, `docs/`, `.cursor-plugin/`); the entire Cargo workspace moves wholesale under `/cli/`.

The split that exists today — `augentic/specify` for prose, `augentic/specify-cli` for the binary — buys nothing the two halves want. They change together, must be version-compatible at every commit, and are already wired across the repo boundary by a source pin, a cargo-script resolver, and a branch-matched CI checkout. Those are scaffolding that *simulates* one repo. This RFC deletes the scaffolding by making it one repo.

This is the **consolidating** half of a two-repo end-state. Its sibling, [RFC-48](rfc-48-adapter-packaging-transport.md), is the **extracting** half: adapters leave for `augentic/specify-adapters` as independently-versioned registry artifacts. Net result: **two** repos — one lockstep platform, one independent adapter ecosystem — not today's prose/runtime split.

## Motivation

The two repos are versioned on **separate, unrelated lines** while being functionally lockstep:

- The plugins ship at marketplace `version: 0.27.0` (`.cursor-plugin/marketplace.json`, and each `plugins/*/.cursor-plugin/plugin.json`).
- The runtime ships at workspace `version = "0.2.0"` (`specify-cli` `Cargo.toml`).

A skill at `0.27.0` shells out to a binary at `0.2.0`, and nothing in those numbers expresses the compatibility they actually require. The coupling is carried instead by machinery whose only job is to keep two repos in step:

- **A source pin.** `Specify.toml` pins the CLI by git / tag / path; co-development needs a gitignored `Specify.local.toml` `cli = { path = "../specify-cli" }`.
- **A cargo-script resolver.** `make lint` runs `cargo +nightly -Zscript scripts/specify.rs`, which reads that pin, *builds* the pinned CLI, and runs `lint framework`. `scripts/use-local-dev.rs` does the same for the plugin cache and WASI tools.
- **A branch-matched CI checkout.** `.github/workflows/ci.yaml` resolves a same-named `specify-cli` branch (falling back to `main`), checks it out, builds `--release`, and only then runs `specify lint framework --framework-root .`.
- **A cross-repo schema pin.** `marketplace.json`'s `$schema` points at `specify-cli/raw/main/schemas/authoring/marketplace.schema.json` — a `main`-pinned reference across the boundary.

Each of these exists to make two repos behave like one. Consolidation removes them outright rather than maintaining them.

## Background

### What each repo holds today

- **`augentic/specify`** — `plugins/` (skills + references for `spec`, `capture`, `client`), `docs/` (explanation, reference, contributing, authoring standards), `adapters/`, `rfcs/`, `evals/`, the `.cursor-plugin/` marketplace manifest, plus the `Specify.toml` pin and `scripts/` shims.
- **`augentic/specify-cli`** — a Cargo workspace: the root `specify` binary crate, `crates/{error,schema,diagnostics,model,tool-manifest,tool,standards,workflow}`, the embedded `schemas/`, `tests/`, `DECISIONS.md`, the Rust `docs/standards/`, and the separate `wasi-tools/` workspace (`contract`, `vectis`).

### What couples them, and what stays a contract

The binary and the prose meet at a **wire contract**: the CLI verbs skills invoke, the kebab-case `error` discriminants they branch on, the journal event taxonomy they emit, and the embedded JSON Schemas artifacts validate against — specified in `specify-cli` `docs/standards/workflow.md`. Consolidation does **not** dissolve that contract; it stops being a *cross-repo* contract between prose and binary and remains the contract between the platform and (a) downstream consumer projects (via `project.yaml.specify_version`) and (b) the extracted adapters repo (RFC-48). The seam moves; it does not vanish (see [Contract seam relocates, not dissolves (T5)](#contract-seam-relocates-not-dissolves-t5)).

### Why adapter co-location does not imply core co-location — and this does

[RFC-48](rfc-48-adapter-packaging-transport.md) co-locates an adapter's prose with its *leaf* tool wasm because they are one shippable, version-locked unit (one digest, one semver). That same reasoning *separates* the shared runtime from any single consumer — the engine is consumed by every skill, adapter, and downstream project, behind a stable contract. The deciding variable is **leaf-and-version-locked** vs **shared-engine-behind-a-contract**, not "prose vs Rust."

Consolidation is consistent with that test, not a violation of it. The platform's prose and its runtime are not an engine-vs-one-consumer relationship — they are two faces of one product that ship together, to the same audience, at the same version. They fail the *independent-cadence* test that keeps adapters separate. So the runtime co-locates with the prose, while adapters (independent cadence, RFC-47 semver, the RM-21 third-party trajectory) deliberately do not.

## Principles

- **One version per shippable platform.** A consumer pins one number and gets a binary and a plugin set known-compatible by construction.
- **Prose owns the root; the engine is quarantined.** The marketplace, skill, and docs paths stay stable; the Rust toolchain, `target/`, and Rust standards live under one subdir so prose contributors never trip over them.
- **The contract is explicit, not repo-enforced.** Compatibility is guaranteed by `workflow.md` + the embedded schemas + the version line — not by a repo boundary. The boundary was never what made it safe.
- **Delete scaffolding; do not maintain it.** The source pin, the cargo-script resolver, and the branch-matched CI exist only to fake one repo. Consolidation removes them.
- **Pre-1.0 major cut, no migration framework.** Consistent with [RFC-48](rfc-48-adapter-packaging-transport.md): the version-line unification and the repo move are a re-init cut, not a compatibility-aliased migration.

## Design

### Normative decisions

| ID | Decision | Implementation consequence |
| --- | --- | --- |
| **T1 Single platform repo** | `augentic/specify-cli` folds into `augentic/specify`. Runtime, plugins, docs, and standards share one repo, one history, one release, one PR / CI run. | Import the `specify-cli` history under `/cli/` (subtree, or a `--allow-unrelated-histories` merge); archive the `specify-cli` repo. The `resolve`-signature / cache-scope `rg`-sweep discipline that today spans two repos becomes intra-repo. |
| **T2 Layout: prose at root, runtime under `/cli/`** | `plugins/`, `docs/`, `.cursor-plugin/`, `rfcs/`, `evals/` keep the repo root; the whole Cargo workspace moves to `/cli/` (root binary, `crates/`, `schemas/`, `tests/`, the Rust `docs/standards/`, `Makefile.toml`). | One Cargo workspace root at `/cli/Cargo.toml`; `target/` and the toolchain are quarantined. `lint framework --framework-root .` runs from the repo root and ignores `/cli/`. Nested `AGENTS.md` (root = workflow, `cli/` = crate graph) auto-fences context. See [Repo layout (T2)](#repo-layout-t2). |
| **T3 Single version line** | One platform version replaces the plugin line (`0.27.0`) and the runtime line (`0.2.0`). The marketplace `version`, every `plugin.json` `version`, the Cargo workspace `version`, and the consumer `specify_version` move as one. | Pick the unified line (recommended: adopt `0.27.0`, the user-facing marketplace number; the internal `0.2.0` binary line retires at the pre-1.0 cut). A release tags all of it at once. See [Version unification (T3)](#version-unification-t3). |
| **T4 Tooling + CI collapse** | The source pin (`Specify.toml` / `Specify.local.toml`), the cargo-script shims (`scripts/specify.rs`, `scripts/use-local-dev.rs`), and the branch-matched CI checkout are deleted. `make lint` builds the in-tree binary; CI becomes one job. The marketplace `$schema` becomes a relative in-repo path. | `make lint` → an in-tree `cargo run -p specify -- lint framework --framework-root .` (wrapped by a top-level `make` target). `ci.yaml` drops the resolve-version / sibling-checkout / build-sibling steps. See [Tooling and CI collapse (T4)](#tooling-and-ci-collapse-t4). |
| **T5 Contract relocates, not dissolves** | The workflow contract (`workflow.md`, the embedded schemas, the error / journal taxonomies) stays the consumer- and adapter-facing contract. The only remaining cross-repo seam is platform ↔ `augentic/specify-adapters` (RFC-48). | The branch-matched-CI pattern relocates to the adapters repo (`specify-adapters` builds or fetches the platform binary). `workflow.md` and `DECISIONS.md` stay the durable spec, now intra-platform. See [Contract seam relocates, not dissolves (T5)](#contract-seam-relocates-not-dissolves-t5). |
| **T6 Adapters remain the second repo** | Consolidation does not absorb adapters. They extract to `augentic/specify-adapters` as independently-versioned artifacts per [RFC-48](rfc-48-adapter-packaging-transport.md) (D7 / D10 / D12). | The two RFCs are complementary halves of one two-repo end-state. `wasi-tools/{contract,vectis}` leave the consolidated runtime for the adapters repo (RFC-48 D10), so `/cli/` ships no `wasi-tools/` workspace. |

### Repo layout (T2)

Prose owns the root (every marketplace, skill, and docs path is unchanged); the Cargo workspace is quarantined under `/cli/`:

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

Why prose-at-root, engine-under-`/cli/` (and not a root-level Cargo workspace):

- **Zero churn to the prose surface.** `marketplace.json`'s `pluginRoot: "plugins"`, every `plugin.json` path, the skill cross-links, and `lint framework --framework-root .` keep working untouched.
- **The two `docs/standards/` do not collide.** Core authoring standards stay at `/docs/standards/`; Rust standards live at `/cli/docs/standards/`. No same-named-tree reconciliation.
- **Nested `AGENTS.md` context-fencing works in your favour.** Root `AGENTS.md` carries workflow / vocabulary; `cli/AGENTS.md` carries the crate graph and is applied only when working under `/cli/`.
- **The toolchain is quarantined.** One workspace root, one `target/`, one `rust-toolchain.toml`, one `Makefile.toml` — the "prose stays toolchain-free" property [RFC-48](rfc-48-adapter-packaging-transport.md) protects holds for the platform repo too.

(`/cli/` versus `/runtime/` is a cosmetic naming choice; `/cli/` matches the `specify-cli` lineage and contains `src/runtime/`.)

### Version unification (T3)

Today three version fields drift independently: marketplace `0.27.0`, each `plugin.json` `0.27.0`, and the Cargo workspace `0.2.0`. After consolidation they are **one number**, and the consumer `project.yaml.specify_version` pins that same number — so pinning the platform pins a binary *and* a known-compatible plugin set.

- **Which line wins.** Recommended: adopt `0.27.0` (the user-facing marketplace number) as the platform line and retire the internal `0.2.0` binary line. This preserves marketplace continuity for installed users; the binary number was never consumer-facing beyond `specify --version`. The choice is the one operator decision in this RFC and should be confirmed before the retag.
- **How it moves.** A release sets the marketplace `version`, every `plugin.json` `version`, and the Cargo `[workspace.package] version` to the same value in one tagged commit; there is no path where they diverge.
- **`$schema` continuity.** The marketplace manifest's `$schema` stops pointing at `specify-cli/raw/main/...` and resolves to the in-repo `cli/schemas/authoring/marketplace.schema.json` (relative or same-tag raw URL), so a tagged release validates against the schema it shipped with.

### Tooling and CI collapse (T4)

Consolidation deletes the cross-repo scaffolding rather than porting it:

- **Deleted.** `Specify.toml`, `Specify.local.toml`, `scripts/specify.rs`, `scripts/use-local-dev.rs` — the source pin and the cargo-script resolvers that build a pinned external CLI.
- **`make lint`** builds the in-tree binary (`cd cli && cargo run -q -p specify -- lint framework --framework-root ..`, wrapped by a top-level target) instead of resolving and building a pinned sibling. `nightly -Zscript` is no longer required.
- **CI** drops the *resolve-version → checkout-sibling → build-sibling* steps in `ci.yaml`; one job runs `cargo make ci` under `/cli/` and `specify lint framework --framework-root .` over the in-tree prose. The symlink-integrity check stays (or is subsumed by [RFC-48](rfc-48-adapter-packaging-transport.md) D12 once `spec-runtime` becomes a registry artifact).
- **`use-local-plugins` / `use-team-plugins`** keep working against the in-tree binary; the WASI-tool build path retires with `wasi-tools` (RFC-48 D7 / D11).

### Contract seam relocates, not dissolves (T5)

Merging the runtime into the prose repo does not remove the workflow contract — it relocates the only cross-repo seam to exactly one place:

- **Stays a contract.** `workflow.md`, the embedded JSON Schemas, the kebab-case `error` discriminants, and the journal taxonomy remain the surface that downstream consumer projects and the adapters repo depend on. Consolidation makes the prose↔binary half of that contract *intra-repo* (one PR, one CI run) without weakening the externally-facing half.
- **The seam moves.** The branch-matched-CI pattern that today couples `specify → specify-cli` relocates to `specify-adapters → specify`: the adapters repo builds or fetches the platform binary and runs `lint framework --framework-root .`, exactly as the platform CI does today against its sibling. The number of cross-repo seams stays at one; its endpoints change.

## Phasing

This is effectively **Phase 0** relative to [RFC-48](rfc-48-adapter-packaging-transport.md)'s packaging work — independent of the transport spike, landable first or in parallel:

1. **History import (T1).** Merge `specify-cli` into `augentic/specify` under `/cli/`, preserving history; archive `specify-cli`.
2. **Tooling + CI collapse (T4).** Delete the source pin and cargo-script shims; make CI one job; build the binary in-tree; flip the marketplace `$schema` to an in-repo reference.
3. **Version unification (T3).** Confirm the platform line, move all version fields together, retag.
4. **Seam relocation (T5).** Point the single remaining cross-repo CI pattern at `specify-adapters`.

The `wasi-tools` relocation ([RFC-48](rfc-48-adapter-packaging-transport.md) D10) is the hinge shared with the adapter extraction; sequence the two RFCs so that relocation happens once, not twice.

## Alternatives considered

- **Keep core and CLI in separate repos (status quo).** Rejected — dual version lines (`0.27.0` vs `0.2.0`) and a standing cross-repo seam (the `Specify.toml` pin, the cargo-script resolver, the branch-matched CI, the `main`-pinned `$schema`) for changes that are inherently lockstep. The boundary costs maintenance and buys no independent cadence the two halves actually use.
- **Root-level Cargo workspace (Rust at the repo root).** Rejected — drops `target/` beside the prose, collides the two `docs/standards/` trees and the two `AGENTS.md` files, and re-paths the marketplace / skill surface for no gain. Quarantining under `/cli/` preserves every prose path.
- **Merge adapters into the platform repo too (one mega-repo).** Rejected — adapters carry an independent RFC-47 semver cadence and an RM-21 third-party trajectory; [RFC-48](rfc-48-adapter-packaging-transport.md) extracts them deliberately. Consolidation is for the lockstep halves only.
- **A multi-language monorepo tool (nx / bazel / pants).** Rejected — overkill for one Cargo workspace plus a prose tree. `make` + `cargo make` already cover the combined build; a heavier orchestrator adds tooling no contributor asked for.
- **Record the consolidation inside RFC-48 (D7 rewrite + a new decision).** Rejected — the merge is a repo-topology decision broader than adapter packaging; folding it into RFC-48 muddies a focused RFC. RFC-48 D7 instead defers the platform half here and owns only the adapter extraction.

## Non-Goals

- **Adapter extraction mechanics** — owned by [RFC-48](rfc-48-adapter-packaging-transport.md) (D7 / D10 / D12). This RFC only states that adapters remain the second repo (T6).
- **A third-party / externally-contributable adapter ecosystem** — [RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model).
- **Changing the workflow contract surface** (schema shapes, CLI verbs, error / journal taxonomies) — unchanged; only its repo home moves.
- **Any migration framework** — pre-1.0 this is a re-init major cut, consistent with [RFC-48](rfc-48-adapter-packaging-transport.md).

## References

- `.github/workflows/ci.yaml` — the branch-matched `specify-cli` resolve / checkout / build that collapses to an in-tree build (T4).
- `Specify.toml`, `Makefile`, `scripts/specify.rs`, `scripts/use-local-dev.rs` — the source pin and cargo-script resolvers removed (T4).
- `.cursor-plugin/marketplace.json` and `plugins/*/.cursor-plugin/plugin.json` — the plugin version line and the cross-repo `$schema` (T3 / T4).
- `augentic/specify-cli` `Cargo.toml` — the workspace (root binary + `crates/`) that moves wholesale to `/cli/` (T2), and the `0.2.0` line that retires (T3).
- [`specify-cli` `docs/standards/workflow.md`](https://github.com/augentic/specify-cli/blob/main/docs/standards/workflow.md) and [`DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md) — the workflow contract that stays durable, now intra-platform (T5).
- [RFC-48: Adapter packaging and transport](rfc-48-adapter-packaging-transport.md) — the sibling extracting half; the two-repo end-state is the sum of both.
- [Roadmap RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model) — the adapter-ecosystem item the second repo serves.
