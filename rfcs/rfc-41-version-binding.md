# RFC-41:  `specify` Version Binding

> Status: Draft · Relates: [RFC-30 bootstrap/upgrade/migrate lifecycle](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#bootstrap-upgrade-and-migration-lifecycle-rfc-30) (the install/upgrade channels this RFC consumes) · Scope: build tooling for the `augentic/specify` repo, not a CLI behaviour change

## Abstract

The `augentic/specify` (framework) repo cannot lint or test itself without a checkout of `augentic/specify-cli`: `make lint` and CI build the `specify` binary from a sibling/nested `specify-cli/Cargo.toml`. This is a one-way, build-time coupling — editing the framework repo requires a second repo *and* a Rust toolchain. This RFC proposes a single `SPECIFY_VERSION` knob (`next | latest | X.Y.Z`, default `next`) that drives **how the framework repo obtains the `specify` binary**. The same value is both the binary selector and the cross-repo compatibility declaration, so day-to-day framework work can run against an installed/published `specify` while co-evolution of both repos stays a first-class, explicit mode.

## Motivation

Today the dependency is hard-wired to a source build:

- `Makefile` resolves `SPECIFY_MANIFEST` to `specify-cli/Cargo.toml` (CI layout) or `../specify-cli/Cargo.toml` (sibling), and both `lint` and `acceptance` run `cargo run --release --manifest-path $(SPECIFY_MANIFEST) --bin specify -- …`.
- `.cargo/config.toml` hard-codes the same path in the `fcheck` alias.
- CI (`.github/workflows/ci.yaml`) clones `augentic/specify-cli` (branch-matched, else `main`) and compiles it on every push.

So a contributor who only edits skills, rules, or docs still needs the CLI repo on disk and a full Rust build to run `make lint`. We want a steady state where **changing `specify` does not require checking out `specify-cli*`*, without losing the ability to co-develop both when a change spans the workflow contract.

The distribution infrastructure to support this already exists — `brew install augentic/tap/specify`, `cargo install specify`, `curl -sSfL https://specify.sh/install.sh | sh` (pinnable with `SPECIFY_VERSION=`), `specify --version`, and the `specify upgrade` release-resolution chain (`SPECIFY_RELEASE_TAG` → `gh release view` → `api.github.com`). The only missing piece is *how the framework repo selects which binary to run*. Notably the source workspace version is already ahead of the latest tag (`[workspace.package] version = "0.3.0"` vs the latest tag `v0.1.0`), so "the source tree carries the next, unreleased version" is already true in practice — which makes `next` a natural name for "build from source."

## Design: one knob

`SPECIFY_VERSION ?= next`, accepting:


| Value                  | Meaning                                                            | Binary comes from                                  |
| ---------------------- | ------------------------------------------------------------------ | -------------------------------------------------- |
| `next` (default)       | the unreleased/dev version living in the `specify-cli` source tree | build from a sibling/nested `specify-cli` checkout |
| `latest`               | the newest published release (floating)                            | resolve the latest tag, then acquire               |
| `X.Y.Z` (e.g. `0.3.0`) | one pinned published release                                       | that exact published binary                        |


The single value answers both "which version" and "where from," because `next` *is* "from source" by definition. This deliberately collapses what would otherwise be two separate concerns — a resolution precedence (installed vs. source) and a minimum-version floor — into one declaration.

## Resolution and acquisition

- `**next`** — `cargo run --release --manifest-path <checkout>/Cargo.toml --bin specify -- lint framework --framework-root .`. No version check: the binary *is* the source tree. If no checkout is present, fail with a clear message pointing at `latest`/`X.Y.Z` or at checking out `specify-cli`.
- `**latest` / `X.Y.Z`** — prefer an already-installed `specify` on `PATH` whose `--version` satisfies the request; otherwise acquire into a gitignored, repo-local `./.bin` (`cargo install specify --version X.Y.Z --root ./.bin`, or the `curl` installer with `SPECIFY_INSTALL_DIR=./.bin`). Never mutate the developer's global binary; `./.bin` is CI-cacheable. Verify `./.bin/specify --version` matches before running.

A small `scripts/ensure-specify.sh` should centralise resolution + acquisition so the `Makefile` and CI share one implementation. Sketch of the consuming `Makefile` shape:

```makefile
SPECIFY_VERSION ?= next
SPECIFY_MANIFEST := $(firstword $(wildcard specify-cli/Cargo.toml ../specify-cli/Cargo.toml))

ifeq ($(SPECIFY_VERSION),next)
  SPECIFY := cargo run --release --manifest-path $(SPECIFY_MANIFEST) --bin specify --
else
  SPECIFY := ./.bin/specify         # scripts/ensure-specify.sh resolves/installs the pinned version
endif

lint:
	$(SPECIFY) lint framework --framework-root .
```

## Version discipline (the contract that enables independence)

`next` only means something if the source tree reliably carries a version *ahead* of the latest release, and `latest`/`X.Y.Z` only mean something if releases are actually cut:

- **Release cadence.** On tagging `vX.Y.Z`, bump the `specify-cli` workspace version to the next. Today there is drift (source `0.3.0`, latest tag `v0.1.0`); for `latest`/pin to resolve, this RFC assumes the CLI repo tightens its release/bump loop.
- **Prerelease suffix (recommended).** Carry a `-dev` suffix on the source version (`0.4.0-dev`) so `next` is *self-identifying* — a suffixed version unambiguously means "source build, not a release," removing the tag-time window where source and release versions collide. Semver sorts `0.4.0-dev < 0.4.0`, and `cargo install specify --version 0.4.0-dev` correctly fails (because `next` never installs).
- **One declared compatible version.** The framework repo declares the CLI version it expects in one place; bumping it rides in the same PR as any framework-rule or schema change that needs newer engine support. This is the reviewable cross-repo compatibility contract — and is good hygiene independent of this RFC.

## CI

- **Default job:** a pinned `SPECIFY_VERSION=X.Y.Z` → install the release into `./.bin` → `make lint`. Deterministic, no cross-repo clone, no Rust build.
- **Opt-in cross-repo job:** `SPECIFY_VERSION=next` plus a branch-matched `specify-cli` checkout, triggered by a `workflow_dispatch` input or a `co-dev` PR label. The current branch-selection logic becomes the `next` path.

This removes the per-push clone/build cost (the cost `REVIEW.md` flags as the reason `make acceptance` is kept out of CI) while keeping co-evolution coverage available on demand.

## Migration

- **Phase A — additive, no behaviour change.** Introduce `SPECIFY_VERSION` with default `next`; add `scripts/ensure-specify.sh` and a `./.bin/` gitignore entry; update the `fcheck` alias and the build-from-sibling docs to mention the knob. Current co-dev contributors see no change.
- **Phase B — flip the default.** Once releases are cut reliably, change the default to a pin (or `latest`) and make `next` the explicit co-dev mode; switch the CI default to the pinned install. Independence becomes the steady state via a one-line default change.

## Non-Goals

- **No change to `specify-cli` distribution.** Brew / crates.io / `install.sh` already exist; this RFC only consumes them.
- **No removal of the source-build path.** Co-development stays first-class via `SPECIFY_VERSION=next`.
- **No new lifecycle or gate authority.** This is build tooling for one repo; it does not touch the workflow contract, lint authority, or any artifact.
- **No mutation of a developer's globally-installed `specify`.** Acquisition is repo-local (`./.bin`).

## Open Questions

1. **Default value.** Keep `next` (co-dev-first) initially, or flip to a pin sooner? What is the trigger condition for Phase B (e.g. "first `0.x` release cut from the tightened loop")?
2. `**next` strictness.** Should `next` ever fall back to an installed binary when no checkout is present, or always fail closed? (Proposed: fail closed — `next` means source.)
3. **Acquisition side effects.** Repo-local `./.bin` auto-install (turnkey, CI-cacheable) vs. check-and-instruct (zero-touch, no install). Which is the default `make lint` experience?
4. **Where the pinned version lives.** A `Makefile` variable, a `.specify-cli-version` file both repos can read, or derived from `specify-cli`'s published tags?
5. **Prerelease suffix adoption.** Does `specify-cli` adopt a `-dev`/`-pre` source suffix? This needs a decision (and a `docs/release.md` update) in the CLI repo.
6. **Version probe robustness.** Is `specify --version | awk '{print $NF}'` sufficient, or should `specify --version` gain a machine-readable form for the comparison?
7. **Escape hatch.** Is a `system`/`any` value ("use whatever `specify` is on `PATH`, no version enforcement") worth adding for casual contributors?

## References

- `[Makefile](../Makefile)` — current `SPECIFY_MANIFEST` resolution and the `lint` / `acceptance` targets.
- `[.cargo/config.toml](../.cargo/config.toml)` — the `fcheck` source-build alias.
- `[docs/contributing/checks.md](../docs/contributing/checks.md)` and `[docs/contributing/acceptance.md](../docs/contributing/acceptance.md)` — current build-from-sibling guidance.
- `[docs/orientation/prerequisites.md](../docs/orientation/prerequisites.md)` — install channels, `SPECIFY_VERSION` pin, and `specify upgrade` release resolution.
- `[specify-cli` `DECISIONS.md` §"Bootstrap, upgrade, and migration lifecycle"]([https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#bootstrap-upgrade-and-migration-lifecycle-rfc-30](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#bootstrap-upgrade-and-migration-lifecycle-rfc-30)) — the install/upgrade lifecycle this RFC builds on.
- `[specify-cli` `docs/release.md](https://github.com/augentic/specify-cli/blob/main/docs/release.md)` — the tag/publish pipeline that `latest` / `X.Y.Z` depend on.

