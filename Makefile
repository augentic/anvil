# Directory on PATH where `make install-specify` symlinks the built binary for
# the acceptance sweep. Override on the command line, e.g.
# `make install-specify INSTALL_DIR=/usr/local/bin`.
INSTALL_DIR ?= $(HOME)/.local/bin

# Resolve CLI using Specify.toml or Specify.local.toml (gitignored overlay).
# N.B. drop `+nightly -Zscript` once it stabilizes (rust-lang/cargo#16569). 
RESOLVE := cargo +nightly -Zscript scripts/specify.rs

.PHONY: lint install-specify use-local-dev use-local-plugins use-team-plugins

lint:
	$(RESOLVE) lint framework

# Adapter-local dev: materialize specify via scripts/specify.rs --install (same cli
# contract as make lint / make install-specify), build WASI tools from cli.path,
# write tools.yaml sidecars, and repopulate the plugin cache. Requires a gitignored
# Specify.local.toml with cli = { path = "../specify-cli" }. ARGS=--skip-wasi skips
# the WASI build. The nightly shebang also allows ./scripts/use-local-dev.rs.
use-local-dev:
	@cargo +nightly -Zscript scripts/use-local-dev.rs $(ARGS)

install-specify:
	@mkdir -p "$(INSTALL_DIR)"
	@ln -sfn "$(CURDIR)/$$($(RESOLVE) --install)" "$(INSTALL_DIR)/specify"
	@specify --version 2>/dev/null || echo "Add $(INSTALL_DIR) to PATH before the sweep."

# Repopulate the Cursor plugin cache from the working tree. The typed
# marketplace.json parse lives in use-local-dev.rs --plugins-only (no jq/bash).
use-local-plugins:
	@cargo +nightly -Zscript scripts/use-local-dev.rs --plugins-only

# Clear the augentic plugin cache via the CLI's own verb (journaled, marketplace-
# scoped). Cursor refetches the published plugins on restart.
use-team-plugins:
	@$(RESOLVE) plugins refresh --yes
