# Directory on PATH where `make install-cli` symlinks the built binary for
# the eval sweep. Override on the command line, e.g.
# `make install-cli INSTALL_DIR=/usr/local/bin`.
INSTALL_DIR ?= $(HOME)/.local/bin

# Cursor plugin cache root and this marketplace's name (see
# .cursor-plugin/marketplace.json) for `make use-local-plugins`.
CURSOR_HOME ?= $(HOME)/.cursor
MARKETPLACE := augentic

.PHONY: ci lint install-cli use-local-plugins use-team-plugins

# Full local gate: the Rust workspace CI (cargo make, Makefile.toml at the
# repo root), then the framework lint over the in-tree prose
# (plugins/, docs/, codex/).
ci:
	cargo make ci
	$(MAKE) lint

# Framework lint over the prose surface, built from the in-tree binary.
lint:
	cargo run -q -p specify -- lint framework --framework-root .

# Build the in-tree binary and symlink it onto PATH for the eval sweep.
install-cli:
	@mkdir -p "$(INSTALL_DIR)"
	cargo build --release -p specify
	@ln -sfn "$(CURDIR)/target/release/specify" "$(INSTALL_DIR)/specify"
	@specify --version 2>/dev/null || echo "Add $(INSTALL_DIR) to PATH before the sweep."

# Mirror the working-tree plugins into the Cursor plugin cache so a local
# Cursor picks up uncommitted skill changes. Restart Cursor afterwards.
use-local-plugins:
	@cache="$(CURSOR_HOME)/plugins/cache/$(MARKETPLACE)"; \
	rm -rf "$$cache"; \
	for dir in plugins/*/; do \
		name=$$(basename "$$dir"); \
		dest="$$cache/$$name/main"; \
		mkdir -p "$$dest"; \
		cp -R "$$dir". "$$dest"; \
		echo "cached $$name"; \
	done; \
	echo "Restart Cursor to pick up local plugin changes."

# Clear the augentic plugin cache via the in-tree binary's own verb
# (journaled, marketplace-scoped). Cursor refetches on restart.
use-team-plugins:
	cargo run -q -p specify -- plugins refresh --project-dir . --yes

# Any other target passes through to cargo make (Makefile.toml), so the
# engine convention `make test` / `make fmt` / `make check` keeps working
# from the repo root.
.PHONY: %
%:
	@cargo make $@
