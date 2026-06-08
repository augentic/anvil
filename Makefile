SPECIFY_VERSION ?= next

# Where `make acceptance` symlinks the freshly built `specify` so the manual
# sweep (and an agent's spawned shells) resolve it as a bare `specify` command.
# Override on the command line, e.g. `make acceptance INSTALL_DIR=~/bin`.
INSTALL_DIR ?= $(HOME)/.local/bin

.PHONY: lint acceptance use-local-plugins use-team-plugins

# Delegates to the central resolver/runner, which binds the framework repo to a
# `specify` binary per SPECIFY_VERSION (next | X.Y.Z).
lint:
	SPECIFY_VERSION=$(SPECIFY_VERSION) ./scripts/specify.sh lint

# Prepares the manual operator sweep (not CI): resolve a `specify` per
# SPECIFY_VERSION, run lint, then symlink it onto PATH so bare `specify` works.
acceptance: lint
	@bin="$$(SPECIFY_VERSION=$(SPECIFY_VERSION) ./scripts/specify.sh --mode bin-path)" && \
		mkdir -p "$(INSTALL_DIR)" && \
		ln -sfn "$$bin" "$(INSTALL_DIR)/specify" && \
		{ specify --version 2>/dev/null || echo "Add $(INSTALL_DIR) to PATH before the sweep."; }

use-local-plugins:
	@bash ./scripts/use-local-plugins.sh

use-team-plugins:
	@bash ./scripts/use-team-plugins.sh
