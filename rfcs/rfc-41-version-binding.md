# RFC-41:  `specify` Version Binding

> Status: Draft · Relates: [RFC-30 bootstrap/upgrade/migrate lifecycle](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#bootstrap-upgrade-and-migration-lifecycle-rfc-30) (the install/upgrade channels this RFC consumes) · Scope: build tooling for the `augentic/specify` repo, not a CLI behaviour change

## Abstract

The `augentic/specify` (framework) repo cannot lint itself or run tests without a checkout of `augentic/specify-cli`: `make lint` and CI build the `specify` binary from a sibling/nested `specify-cli/Cargo.toml`. This is a one-way, build-time coupling — editing the framework repo requires a second repo *and* a Rust toolchain.

This RFC proposes a single `SPECIFY_VERSION` knob (`next | latest | X.Y.Z`, default `next`) that drives **how the framework repo binds to a** `specify` **binary**, adopted in one shot. The default `next` prefers the `specify-cli` source tree and **falls back to the latest published release when no checkout is present**, so co-development stays the default for contributors who have both repos on disk while skills/rules/docs-only contributors get a working `make lint` with no Rust toolchain. The version value is both the binary selector and a cross-repo compatibility declaration.

## Motivation

Today the dependency is hard-wired to a source build:

- `Makefile` resolves `SPECIFY_MANIFEST` to `specify-cli/Cargo.toml` (CI layout) or `../specify-cli/Cargo.toml` (sibling), and both `lint` and `acceptance` run `cargo run --release --manifest-path $(SPECIFY_MANIFEST) --bin specify -- …`.
- A former `cargo fcheck` shortcut (via `.cargo/config.toml`) hard-coded the same manifest path; that directory is removed because the framework repo is not Cargo-managed.
- CI (`.github/workflows/ci.yaml`) clones `augentic/specify-cli` (branch-matched, else `main`) and compiles it on every push.

A contributor who only edits skills, rules, or docs still needs the CLI repo on disk and a full Rust build to run `make lint`. We want a steady state where:

- changing `specify` does not require checking out `specify-cli`
- it is easy to co-develop both when a change spans the workflow contract.

The distribution infrastructure to support this already exists — `brew install augentic/tap/specify`, `cargo install specify`, `curl -sSfL https://specify.sh/install.sh | sh` (pinnable with `SPECIFY_VERSION=`), `specify --version`, and the `specify upgrade` release-resolution chain (`SPECIFY_RELEASE_TAG` → `gh release view` → `api.github.com`).

The missing piece is *how the framework repo selects which binary to run*. We introduce the knob in one change: the default `next` keeps the source build as the primary path but degrades gracefully — when no `specify-cli` checkout can be found, it acquires and runs the latest published release instead of failing. Notably the source workspace version is already ahead of the latest tag (`[workspace.package] version = "0.2.0"` vs the latest tag `v0.1.0`), so "the source tree carries the next, unreleased version" is already true in practice — which makes `next` a natural name for "build from source."

## Design: one knob

`SPECIFY_VERSION ?= next`, accepting:


| Value                  | Meaning                                                            | Binary comes from                                                                               |
| ---------------------- | ------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------- |
| `next` (default)       | the unreleased/dev version living in the `specify-cli` source tree | a sibling/nested `specify-cli` checkout, **falling back to `latest*`* when no checkout is found |
| `latest`               | the newest published release (floating)                            | resolve the latest tag, then acquire                                                            |
| `X.Y.Z` (e.g. `0.2.0`) | one pinned published release                                       | that exact published binary                                                                     |
| `system`               | whatever `specify` is already on `PATH`; no version enforcement    | the developer's existing install — no resolution, no acquisition                                |


The single value answers both "which version" and "where from," because `next` *is* "from source" by definition. This deliberately collapses what would otherwise be two separate concerns — a resolution precedence (installed vs. source) and a minimum-version floor — into one declaration. The default favours the source build for co-development but never blocks a contributor who lacks the checkout: it transparently falls back to the latest published release. Explicit `latest`/`X.Y.Z` force the published binary regardless of any checkout on disk.

## Resolution and acquisition

- `**next`** (default) — if a sibling/nested `specify-cli` checkout is found, run `cargo run --release --manifest-path <checkout>/Cargo.toml --bin specify -- lint framework --framework-root .`. No version check: the binary *is* the source tree. **If no checkout is present, fall back to the acquisition path below, acquiring the version pinned in `.specify-version*`* (with a one-line notice so the contributor knows they are on a published binary, not source). The fallback is deterministic — it resolves the declared pin, not floating `latest`.
- `**latest` / `X.Y.Z`** — prefer an already-installed `specify` on `PATH` whose `--version` satisfies the request; otherwise **auto-acquire** into a gitignored, repo-local `./.bin` (`cargo install specify --version X.Y.Z --root ./.bin`, or the `curl` installer with `SPECIFY_INSTALL_DIR=./.bin`). Acquisition is the default, turnkey path so a docs/skills/rules contributor with no Rust toolchain gets a working `make lint` with zero manual setup; it prints a one-line notice naming the version and `./.bin` target so the install is loud, not silent. Never mutate the developer's global binary; `./.bin` is CI-cacheable. Verify `./.bin/specify --version` matches (parsed with `awk '{print $NF}'`) before running.
- `**system`** — skip resolution and acquisition entirely and run whatever `specify` is on `PATH`, with no version enforcement and no compatibility check. The opt-out from the auto-install default, intended for contributors who already have `specify` installed (e.g. via `brew`), offline/air-gapped environments, and CI debugging. Disabling the compatibility contract is the explicit trade-off.

A small `scripts/specify.sh` centralises resolution, acquisition, and invocation so the `Makefile`, contributor shortcuts, and CI share one implementation. The script resolves the binary per `SPECIFY_VERSION`, `cd`s to the framework repo root, and `exec`s the resolved `specify` command. A `fcheck` shorthand runs `lint framework --framework-root .` — the direct replacement for the removed `cargo fcheck` alias, invocable from any subdirectory. Sketch of the consuming `Makefile` shape:

```makefile
SPECIFY_VERSION ?= next

lint:
	SPECIFY_VERSION=$(SPECIFY_VERSION) ./scripts/specify.sh fcheck
```

## Version discipline (the contract that enables independence)

`next` only means something if the source tree reliably carries a version *ahead* of the latest release, and `latest`/`X.Y.Z` only mean something if releases are actually cut:

- **Release cadence.** Releases are cut and published manually by maintainers.
- **One declared compatible version.** The framework repo declares the CLI version it expects in a single-line `.specify-version` file at the repo root — the one place `make lint`, `scripts/specify.sh`, and CI read the pinned published version. This is the reviewable cross-repo compatibility contract, and is good hygiene independent of this RFC. A plain file beats the alternatives: it is readable by `make`, the script, CI, and a human without parsing Make, and a release script can bump it without touching build logic. A `Makefile` variable was rejected because it couples the compatibility declaration to the build system; deriving the pin from `specify-cli`'s published tags was rejected because it would require a network call on every `make lint` and couple the declaration to GitHub availability. The runtime `SPECIFY_VERSION` knob still overrides the file (`next` / `latest` / `X.Y.Z` / `system`); the file is only read when acquisition needs an explicit version — the `next` fallback and the CI baseline job — so the resolved pin is deterministic rather than floating `latest`.
- **No prerelease suffix.** `next` reads `specify-cli`'s `[workspace.package] version` from `Cargo.toml` as-is — it does **not** require a `-dev`/`-pre` suffix. The "source is ahead of latest" property already holds in practice (`0.2.0` source vs the latest `v0.1.0` tag), and how `specify-cli` is versioned is the maintainer's call; RFC-41 makes no demand on it. The `awk` probe (below) must therefore tolerate whatever version string the maintainer chooses.
- **Version probe.** `specify --version | awk '{print $NF}'` (the last whitespace token) is the comparison mechanism for `latest`/`X.Y.Z`. No machine-readable `--version` form is added now; if one ever lands in the CLI repo it can replace the probe without changing this RFC.

## CI

CI pins explicitly rather than relying on the `next` fallback, so a job is never silently a source build one run and a published binary the next:

- **Baseline job:** acquire the version pinned in `.specify-version` into `./.bin` → `make lint`. Deterministic (the declared pin, not floating `latest`), no cross-repo clone, no Rust build.
- **Cross-repo job:** `SPECIFY_VERSION=next` plus a branch-matched `specify-cli` checkout, exercising the source build for changes that span the workflow contract. The current branch-selection logic moves to this `next` job.

Local `make lint` uses the default `next` with its fallback; CI prefers the explicit pins above for reproducibility.

## Migration

This is a single, non-staged change. The source build stays the default, so existing co-dev contributors see no behaviour change; the new behaviour is purely additive — a graceful fallback plus the `latest`/`X.Y.Z` modes.

- Add the single-line `.specify-version` file at the repo root carrying the declared-compatible published version; `scripts/specify.sh` reads it as the default acquisition target.
- Add `scripts/specify.sh` (centralised resolution, acquisition, and invocation) with a `fcheck` shorthand and passthrough for any `specify` subcommand; add the `./.bin/` gitignore entry.
- Set `SPECIFY_VERSION ?= next` in the `Makefile` and delegate `lint` to `./scripts/specify.sh fcheck` so a missing `specify-cli` checkout resolves to `./.bin/specify` instead of erroring.
- Remove `.cargo/config.toml` (the framework repo is not Cargo-managed; `cargo fcheck` is replaced by `./scripts/specify.sh fcheck`).
- Add the explicit-pin CI baseline job (acquiring `.specify-version`) and move the existing branch-matched clone+build into the `next` CI job.
- Update `docs/contributing/checks.md`, `docs/contributing/acceptance.md`, and `docs/orientation/prerequisites.md` to document the default source build, its fallback, `make lint` / `./scripts/specify.sh fcheck`, and the `latest`/`X.Y.Z` pins.

The hard failure when no checkout exists is the only behaviour removed; everything else is additive.

## Non-Goals

- **No change to `specify-cli` distribution.** Brew / crates.io / `install.sh` already exist; this RFC only consumes them.
- **The source-build path stays the default.** `next` remains the default and the primary path; the change only adds a graceful fallback to `latest` when no checkout is present, plus the explicit published-binary modes.
- **No new lifecycle or gate authority.** This is build tooling for one repo; it does not touch the workflow contract, lint authority, or any artifact.
- **No mutation of a developer's globally-installed `specify`.** Acquisition is repo-local (`./.bin`).

## References

- `[.specify-version](../.specify-version)` — the single-line declared-compatible published CLI version (to be added by this RFC's migration).
- `[Makefile](../Makefile)` — current `SPECIFY_MANIFEST` resolution and the `lint` / `acceptance` targets.
- `[scripts/specify.sh](../scripts/specify.sh)` — centralised resolver, acquirer, and runner (to be added by this RFC's migration); `fcheck` shorthand replaces the removed `cargo fcheck` alias.
- `[docs/contributing/checks.md](../docs/contributing/checks.md)` and `[docs/contributing/acceptance.md](../docs/contributing/acceptance.md)` — current build-from-sibling guidance.
- `[docs/orientation/prerequisites.md](../docs/orientation/prerequisites.md)` — install channels, `SPECIFY_VERSION` pin, and `specify upgrade` release resolution.
- `[specify-cli` `DECISIONS.md` §"Bootstrap, upgrade, and migration lifecycle"]([https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#bootstrap-upgrade-and-migration-lifecycle-rfc-30](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#bootstrap-upgrade-and-migration-lifecycle-rfc-30)) — the install/upgrade lifecycle this RFC builds on.
- `[specify-cli` `docs/release.md](https://github.com/augentic/specify-cli/blob/main/docs/release.md)` — the tag/publish pipeline that `latest` / `X.Y.Z` depend on.

