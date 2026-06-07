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

.PHONY: lint acceptance use-local-plugins use-team-plugins

lint:
	cargo run --release --manifest-path $(SPECIFY_MANIFEST) --bin specify -- lint framework --framework-root .

# Prepares the manual operator sweep. Builds the release binary, runs the
# static checks, then symlinks this build's `specify` into INSTALL_DIR so the
# manual sweep resolves the bare `specify` command. The symlink always points
# at the latest build, so it never goes stale. Deliberately NOT wired into CI.
# The deterministic acceptance tests are not re-run here — `cargo make test` in
# specify-cli is the single, authoritative deterministic surface.
acceptance:
	cargo build --release --manifest-path $(SPECIFY_MANIFEST) --bin specify
	@$(MAKE) lint
	@mkdir -p "$(INSTALL_DIR)"
	@ln -sfn "$(SPECIFY_BIN_DIR)/specify" "$(SPECIFY_LINK)"
	@specify --version 2>/dev/null || echo "Add $(INSTALL_DIR) to PATH before the sweep."

use-local-plugins:
	@bash ./scripts/use-local-plugins.sh

use-team-plugins:
	@bash ./scripts/use-team-plugins.sh
