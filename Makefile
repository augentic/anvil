ifeq ($(wildcard specify-cli/Cargo.toml),)
  SPECIFY_MANIFEST ?= ../specify-cli/Cargo.toml
else
  SPECIFY_MANIFEST := specify-cli/Cargo.toml
endif

SPECIFY_BIN_DIR := $(abspath $(dir $(SPECIFY_MANIFEST))target/release)

# Where `make acceptance` symlinks the freshly built `specify` so the manual
# sweep (and an agent's spawned shells) resolve it as a bare `specify` command.
# Override on the command line, e.g. `make acceptance INSTALL_DIR=~/bin`.
INSTALL_DIR ?= $(HOME)/.local/bin
SPECIFY_LINK := $(INSTALL_DIR)/specify

.PHONY: lint acceptance acceptance-scenario use-local-plugins use-team-plugins

lint:
	cargo run --release --manifest-path $(SPECIFY_MANIFEST) --bin specify -- lint framework --framework-root .

# Deterministic acceptance surface only. Builds the release binary, runs the
# static checks plus the fixture-backed acceptance tests (the deterministic
# proof for every scenario marked `automated` in the catalog), then symlinks
# this build's `specify` into INSTALL_DIR so the manual sweep resolves the bare
# `specify` command. The symlink always points at the latest build, so it never
# goes stale. Deliberately NOT wired into CI. `cargo make test` in specify-cli
# remains the full deterministic surface.
acceptance:
	cargo build --release --manifest-path $(SPECIFY_MANIFEST) --bin specify
	@$(MAKE) lint
	cargo test --release --manifest-path $(SPECIFY_MANIFEST) --test fan_in_fan_out --test source_extract --test slice --test plan_orchestrate --test workspace
	@mkdir -p "$(INSTALL_DIR)"
	@ln -sfn "$(SPECIFY_BIN_DIR)/specify" "$(SPECIFY_LINK)"
	@echo
	@echo "Symlinked specify -> $(SPECIFY_LINK)"
	@echo "Bare 'specify' resolves to: $$(command -v specify 2>/dev/null || echo '<not on PATH>')"
	@specify --version 2>/dev/null || echo "Add $(INSTALL_DIR) to PATH before the sweep."

# Scaffold one acceptance scenario's disposable environment up to its
# pre-invocation state (setup helper only — drives no /spec:* command).
# Run `make acceptance` first so the bare `specify` resolves to the build
# under test. Usage: make acceptance-scenario ID=pure-intent
acceptance-scenario:
	@bash ./scripts/acceptance-scenario.sh "$(ID)"

use-local-plugins:
	@bash ./scripts/use-local-plugins.sh

use-team-plugins:
	@bash ./scripts/use-team-plugins.sh
