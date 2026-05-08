DENO := $(or $(shell command -v deno 2>/dev/null),$(wildcard $(HOME)/.deno/bin/deno))

.PHONY: checks
checks:
	@$(DENO) run --allow-read scripts/checks.ts

# C07 setup-only smoke for the RM-01 cross-repo suite. Drives the C07
# helper modules (hub + projects + fake gh + workspace sync) end-to-end
# and asserts the four setup-* invariants. Skips with exit 0 when
# `specify` is not on PATH (and SPECIFY_BIN is unset). Broader
# acceptance-cross-repo target is C16's responsibility.
.PHONY: acceptance-cross-repo-setup-smoke
acceptance-cross-repo-setup-smoke:
	@$(DENO) run \
		--allow-read --allow-write --allow-env --allow-run --allow-net=none \
		acceptance/runner/smoke-cross-repo-setup.ts

# C09 plan-level smoke for the RM-01 cross-repo suite. Drives the
# scripted-plan backend end-to-end (setup → 3-entry plan via
# `specify change plan {create, add}` → role-based assertions). Skips
# with exit 0 when `specify` is missing or pre-RFC-9. The scripted-plan
# backend is a deterministic stand-in for /change:plan, NOT a proof
# that /change:plan is correct — that requires the reserved `agent`
# backend. Broader acceptance-cross-repo target is C16's responsibility.
.PHONY: acceptance-cross-repo-plan-smoke
acceptance-cross-repo-plan-smoke:
	@$(DENO) run \
		--allow-read --allow-write --allow-env --allow-run --allow-net=none \
		acceptance/runner/smoke-cross-repo-plan.ts

# C10 execute-level smoke for the RM-01 cross-repo suite. Drives the
# scripted-execute backend (setup → plan → deterministic loop driver
# equivalent of `/change:execute loop` → setup-* + plan-* + execute-*
# assertions). Skips with exit 0 when `specify` is missing or predates
# the RFC-9 surface (`change plan next`, `workspace prepare-branch`).
# The scripted-execute backend is a deterministic stand-in for
# /change:execute loop, NOT a proof that the loop skill itself is
# correct — that requires the reserved `agent` backend.
.PHONY: acceptance-cross-repo-execute-smoke
acceptance-cross-repo-execute-smoke:
	@$(DENO) run \
		--allow-read --allow-write --allow-env --allow-run --allow-net=none \
		acceptance/runner/smoke-cross-repo-execute.ts

# C11 finalize-level smoke for the RM-01 cross-repo suite. Drives the
# scripted-finalize backend (setup → plan → execute loop → workspace
# push → fake-gh mark-merged → change finalize → idempotency probe →
# setup-* + plan-* + execute-* + push-* + finalize-* assertions).
# Skips with exit 0 when `specify` is missing or predates the RFC-9
# surface (`init --hub`, `change plan {next, transition}`,
# `workspace {prepare-branch, push}`, `change finalize`). The
# scripted-finalize backend is a deterministic stand-in for the
# post-execute landing path, NOT a proof that any orchestration skill
# is correct — that requires the reserved `agent` backend.
.PHONY: acceptance-cross-repo-finalize-smoke
acceptance-cross-repo-finalize-smoke:
	@$(DENO) run \
		--allow-read --allow-write --allow-env --allow-run --allow-net=none \
		acceptance/runner/smoke-cross-repo-finalize.ts

# C12 define-level smoke for the RM-01 cross-repo suite. Drives the
# `agent` backend with the example operator-results JSON
# (`acceptance/suites/rm01-cross-repo/operator-results.example.json`)
# so the new define-* assertions exercise end-to-end against
# operator-supplied artifact bodies. Override the operator-results
# path with `OPERATOR_RESULTS=/path/to/results.json`.
#
# Skips with exit 0 when:
#   * `specify` is missing or pre-RFC-9 (same policy as C09/C10/C11);
#   * the operator-results file does not exist;
#   * neither --operator-results nor --cursor-sdk is supplied.
#
# The agent backend is C12's hand-off for real `/spec:define`
# execution. Two driver shapes are documented in
# `backends/README.md` §Agent Backend: option (B) operator-manual
# (this smoke) and option (A) Cursor SDK programmatic invocation
# (deferred to a future amendment).
.PHONY: acceptance-cross-repo-define-smoke
acceptance-cross-repo-define-smoke:
	@$(DENO) run \
		--allow-read --allow-write --allow-env --allow-run --allow-net=none \
		acceptance/runner/smoke-cross-repo-define.ts

# C13 contracts-build smoke for the RM-01 cross-repo suite. Drives the
# `contracts-build` backend (setup → plan → execute loop with per-slice
# dispatch: contract slice → ContractsBuildPhaseDriver, impl slices →
# StubPhaseDriver) so the contract slice emits a realistic-but-
# deterministic OpenAPI 3.1 + JSON Schema bundle the contracts WASI
# tool can validate. Asserts the C13 contract-slice-* family on top of
# the C09/C10/C12 setup-/plan-/execute-/define-* families.
#
# Skips with exit 0 when:
#   * `specify` is missing or pre-RFC-9 (same policy as C09/C10/C11);
#   * the resolved binary does not expose `specify tool run` (pre-
#     RFC-15 release);
#   * the contract WASM cannot be located at assertion time
#     (downgrades the validator handler to `cli-substrate` skip).
#
# The contracts-build backend is execute-only by design (boundary
# documented in `backends/contracts-build.ts`); push/finalize coverage
# stays on `scripted-finalize` / `agent`. C14a/C14b extend through
# finalize once Omnia / Vectis builds are deterministic.
.PHONY: acceptance-cross-repo-contracts-build-smoke
acceptance-cross-repo-contracts-build-smoke:
	@$(DENO) run \
		--allow-read --allow-write --allow-env --allow-run --allow-net=none \
		acceptance/runner/smoke-cross-repo-contracts-build.ts

# C14a omnia-build smoke for the RM-01 cross-repo suite. Drives the
# `omnia-build` backend (setup → plan → execute loop with per-slice
# dispatch: contract slice → ContractsBuildPhaseDriver, omnia slices
# → OmniaBuildPhaseDriver, other slices → StubPhaseDriver) so the
# backend slice emits a deterministic-but-realistic Rust crate
# skeleton (Cargo.toml + src/lib.rs + src/providers.rs) into the
# routed clone's `crates/<crate>/` tree. Asserts the C14a omnia-* /
# baseline-files-present family on top of the C09/C10/C12/C13
# setup-/plan-/execute-/define-/contract-build-* families.
#
# Skips with exit 0 when:
#   * `specify` is missing or pre-RFC-9 (same policy as C09/C10/C11);
#   * the resolved binary does not expose `specify tool run` (pre-
#     RFC-15 release; the omnia-build backend reuses C13's contracts-
#     build driver for the contract slice).
#
# The omnia-build backend is execute-only by design (boundary
# documented in `backends/omnia-build.ts`); push/finalize coverage
# stays on `scripted-finalize` / `agent`. The Vectis equivalent
# lands in C14b.
.PHONY: acceptance-cross-repo-omnia-build-smoke
acceptance-cross-repo-omnia-build-smoke:
	@$(DENO) run \
		--allow-read --allow-write --allow-env --allow-run --allow-net=none \
		acceptance/runner/smoke-cross-repo-omnia-build.ts

# C14b vectis-build smoke for the RM-01 cross-repo suite. Drives the
# `vectis-build` backend (setup → plan → execute loop with per-slice
# dispatch: contract slice → ContractsBuildPhaseDriver, vectis slices
# → VectisBuildPhaseDriver, other slices → StubPhaseDriver) so the
# mobile slice emits a deterministic-but-realistic Vectis composition
# + SwiftUI shell (`composition.yaml` at the project root +
# `apps/mobile/login_screen.swift` residue) into the routed mobile
# clone. Asserts the C14b vectis-* / baseline-files-present family on
# top of the C09/C10/C12/C13 setup-/plan-/execute-/define-/contract-
# build-* families.
#
# Skips with exit 0 when:
#   * `specify` is missing or pre-RFC-9 (same policy as C09/C10/C11);
#   * the resolved binary does not expose `specify tool run` (pre-
#     RFC-15 release; the vectis-build backend reuses C13's contracts-
#     build driver for the contract slice).
#
# The vectis-build backend is execute-only by design (boundary
# documented in `backends/vectis-build.ts`); push/finalize coverage
# stays on `scripted-finalize` / `agent`. The Omnia counterpart
# lives in C14a (parallel coverage; both backends run independently).
.PHONY: acceptance-cross-repo-vectis-build-smoke
acceptance-cross-repo-vectis-build-smoke:
	@$(DENO) run \
		--allow-read --allow-write --allow-env --allow-run --allow-net=none \
		acceptance/runner/smoke-cross-repo-vectis-build.ts

# C15 recorded-replay smoke for the RM-01 cross-repo suite. Drives the
# `recorded` backend against the checked-in baseline trace at
# `acceptance/recorded/rm01-cross-repo/baseline.jsonl` (override with
# RECORDED_TRACE=/path/to/trace.jsonl). The recorded backend re-runs
# every recorded `specify` argv against the live binary and pins exit
# codes; a regression that diverges from the trace becomes a
# `cli-substrate` (recorded 0 → live non-zero) or
# `live-agent-nondeterminism` (any other delta) failure.
#
# Skips with exit 0 when:
#   * the baseline trace file is missing (fresh checkout, regenerate path);
#   * `specify` is missing or pre-RFC-9 (same policy as C09/C10/C11);
#   * the resolved binary does not expose `change plan {create, next}`.
#
# The recorded backend is *complementary* coverage — cheap regression
# pinning of the CLI substrate the scripted backends compose. It does
# NOT replace `make acceptance-cross-repo-execute-smoke`; periodic
# live runs are still the source of truth for the trace's correctness.
# See `acceptance/runner/backends/README.md` §Recorded Backend (C15).
.PHONY: acceptance-cross-repo-recorded-smoke
acceptance-cross-repo-recorded-smoke:
	@$(DENO) run \
		--allow-read --allow-write --allow-env --allow-run --allow-net=none \
		acceptance/runner/smoke-cross-repo-recorded.ts

.PHONY: acceptance-smoke
acceptance-smoke:
	@$(DENO) run --allow-read --allow-write --allow-env --allow-run \
		acceptance/runner/main.ts \
		--scenario contracts-describe \
		--backend fixture \
		--allow-backend-mismatch

# C08 deterministic stub-backend smoke. Drives a real `specify` CLI
# through `slice create → transition (defined) → transition (building) →
# fixture-copy → transition (complete) → slice merge run` for the
# `contracts-describe-stub` scenario. Skips with exit 0 (and prints why)
# when `specify` is not resolvable on PATH and `SPECIFY_BIN` is unset,
# matching the policy used by `acceptance-cross-repo-setup-smoke`.
.PHONY: acceptance-stub-smoke
acceptance-stub-smoke:
	@$(DENO) run --allow-read --allow-write --allow-env --allow-run \
		acceptance/runner/main.ts \
		--scenario contracts-describe-stub \
		--backend stub

# C16 cross-repo aggregator. Runs every cross-repo smoke serially and
# reports a single PASS/SKIP/FAIL summary at the end. Each child smoke
# is responsible for skipping gracefully when `specify` (or the
# operator-results JSON, recorded trace, RFC-9 surface, etc.) is
# missing; the aggregator does not pre-check those preconditions.
#
# Aggregator behavior (see scripts/acceptance-aggregate.ts):
#   * never fail-fast — every target runs even if an earlier one failed,
#   * captures per-target stdout/stderr to a temp logs directory,
#   * re-emits captured output on failure so PR review can see the cause,
#   * exits non-zero only after the final summary table is printed.
#
# `acceptance-cross-repo` is the operator-/CI-driven full sweep — it
# includes `define`, which itself skips with exit 0 unless
# OPERATOR_RESULTS (or --cursor-sdk in a future amendment) is set.
# `acceptance-cross-repo-deterministic` is the same minus `define`, for
# unattended runs that must not depend on operator-supplied JSON.
.PHONY: acceptance-cross-repo
acceptance-cross-repo:
	@$(DENO) run \
		--allow-read --allow-write --allow-env --allow-run \
		scripts/acceptance-aggregate.ts \
		--label cross-repo \
		acceptance-cross-repo-setup-smoke \
		acceptance-cross-repo-plan-smoke \
		acceptance-cross-repo-execute-smoke \
		acceptance-cross-repo-finalize-smoke \
		acceptance-cross-repo-define-smoke \
		acceptance-cross-repo-contracts-build-smoke \
		acceptance-cross-repo-omnia-build-smoke \
		acceptance-cross-repo-vectis-build-smoke \
		acceptance-cross-repo-recorded-smoke

.PHONY: acceptance-cross-repo-deterministic
acceptance-cross-repo-deterministic:
	@$(DENO) run \
		--allow-read --allow-write --allow-env --allow-run \
		scripts/acceptance-aggregate.ts \
		--label cross-repo-deterministic \
		acceptance-cross-repo-setup-smoke \
		acceptance-cross-repo-plan-smoke \
		acceptance-cross-repo-execute-smoke \
		acceptance-cross-repo-finalize-smoke \
		acceptance-cross-repo-contracts-build-smoke \
		acceptance-cross-repo-omnia-build-smoke \
		acceptance-cross-repo-vectis-build-smoke \
		acceptance-cross-repo-recorded-smoke

# Convenience aggregator that runs the narrow suite (Tier 1) plus the
# full cross-repo aggregator. Useful before pushing a release tag when
# you want every Specify acceptance signal in one command.
.PHONY: acceptance-all
acceptance-all:
	@$(DENO) run \
		--allow-read --allow-write --allow-env --allow-run \
		scripts/acceptance-aggregate.ts \
		--label all \
		acceptance-smoke \
		acceptance-stub-smoke \
		acceptance-cross-repo

# Touched-file tier selector (C16). Prints the recommended `make`
# targets for the current diff (vs `origin/main`, falling back to
# `HEAD~1`). Does NOT auto-execute the targets — pipe the output into
# `xargs -n1 make` (or `make $(make acceptance-tiers)`) when you want
# the script to drive the build.
#
# Examples:
#   make acceptance-tiers                                   # selection only
#   make acceptance-tiers TIER_ARGS='--explain'             # selection + reasons
#   make acceptance-tiers TIER_ARGS='--files Makefile'      # ad-hoc selection
.PHONY: acceptance-tiers
acceptance-tiers:
	@$(DENO) run \
		--allow-read --allow-run \
		scripts/acceptance-tier.ts $(TIER_ARGS)

.PHONY: use-local-plugins
use-local-plugins:
	@bash ./scripts/use-local-plugins.sh

.PHONY: use-team-plugins
use-team-plugins:
	@bash ./scripts/use-team-plugins.sh
