# The version of the `specify` binary to install. Override on the command line, e.g.
# `make install-specify SPECIFY_VERSION=0.1.0`.
SPECIFY_VERSION ?= next

.PHONY: lint install-specify

lint:
	SPECIFY_VERSION=$(SPECIFY_VERSION) ./scripts/specify.sh lint

install-specify:
	@bin="$$(SPECIFY_VERSION=$(SPECIFY_VERSION) ./scripts/specify.sh --mode bin-path)" && \
		install_dir="$$(./scripts/specify.sh --mode config-key path)" && \
		mkdir -p "$$install_dir" && \
		ln -sfn "$$bin" "$$install_dir/specify" && \
		{ specify --version 2>/dev/null || echo "Add $$install_dir to PATH before the sweep."; }

use-local-plugins:
	@bash ./scripts/use-local-plugins.sh

use-team-plugins:
	@bash ./scripts/use-team-plugins.sh
