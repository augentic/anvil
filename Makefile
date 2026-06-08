SPECIFY_VERSION ?= next
SPECIFY_MANIFEST := $(firstword $(wildcard specify-cli/Cargo.toml ../specify-cli/Cargo.toml))

SPECIFY_BIN_DIR := $(abspath $(dir $(SPECIFY_MANIFEST))target/release)

# Where `make acceptance` symlinks the freshly built `specify` so the manual
# sweep (and an agent's spawned shells) resolve it as a bare `specify` command.
# Override on the command line, e.g. `make acceptance INSTALL_DIR=~/bin`.
INSTALL_DIR ?= $(HOME)/.local/bin
SPECIFY_LINK := $(INSTALL_DIR)/specify

.PHONY: lint fcheck acceptance use-local-plugins use-team-plugins

# Delegates to the central resolver/runner, which binds the framework repo to a
# `specify` binary per SPECIFY_VERSION (next | latest | X.Y.Z | system).
lint:
	SPECIFY_VERSION=$(SPECIFY_VERSION) ./scripts/specify.sh fcheck

# Contributor alias for `lint` (replaces the old `cargo fcheck`).
fcheck: lint

# Prepares the manual operator sweep. Source-build only: requires a `specify-cli`
# checkout (sibling or nested). Builds the release binary, runs the static checks,
# then symlinks this build's `specify` into INSTALL_DIR so the manual sweep
# resolves the bare `specify` command. The symlink always points at the latest
# build, so it never goes stale. Deliberately NOT wired into CI. The deterministic
# acceptance tests are not re-run here — `cargo make test` in specify-cli is the
# single, authoritative deterministic surface. The no-checkout graceful fallback
# is scoped to `lint` only, so acceptance fails fast without a checkout.
acceptance:
ifeq ($(SPECIFY_MANIFEST),)
	@echo "make acceptance requires a specify-cli checkout (sibling ../specify-cli or nested specify-cli)." >&2
	@echo "Only 'make lint' supports the no-checkout published-binary fallback (see docs/contributing/checks.md)." >&2
	@exit 1
else
	cargo build --release --manifest-path $(SPECIFY_MANIFEST) --bin specify
	@$(MAKE) lint
	@mkdir -p "$(INSTALL_DIR)"
	@ln -sfn "$(SPECIFY_BIN_DIR)/specify" "$(SPECIFY_LINK)"
	@specify --version 2>/dev/null || echo "Add $(INSTALL_DIR) to PATH before the sweep."
endif

use-local-plugins:
	@bash ./scripts/use-local-plugins.sh

use-team-plugins:
	@bash ./scripts/use-team-plugins.sh
