# The version of the `specify` binary to install. Override on the command line, e.g.
# `make install-specify SPECIFY_VERSION=0.1.0`.
SPECIFY_VERSION ?= next

# The location to install the `specify` binary so it can be resolved as a bare `specify` command.
# Override on the command line, e.g. `make install-specify INSTALL_DIR=~/bin`.
INSTALL_DIR ?= $(HOME)/.local/bin

.PHONY: lint install-specify

lint:
	SPECIFY_VERSION=$(SPECIFY_VERSION) ./scripts/specify.sh lint

install-specify:
	@bin="$$(SPECIFY_VERSION=$(SPECIFY_VERSION) ./scripts/specify.sh --mode bin-path)" && \
		mkdir -p "$(INSTALL_DIR)" && \
		ln -sfn "$$bin" "$(INSTALL_DIR)/specify" && \
		{ specify --version 2>/dev/null || echo "Add $(INSTALL_DIR) to PATH before the sweep."; }

use-local-plugins:
	@bash ./scripts/use-local-plugins.sh

use-team-plugins:
	@bash ./scripts/use-team-plugins.sh
