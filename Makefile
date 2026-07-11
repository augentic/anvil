# Directory on PATH where `make install-cli` symlinks the built binary for
# the eval sweep. Override on the command line, e.g.
# `make install-cli INSTALL_DIR=/usr/local/bin`.
INSTALL_DIR ?= $(HOME)/.local/bin

# Cursor plugin cache root and this marketplace's name (see
# .cursor-plugin/marketplace.json) for `make use-local-plugins`.
CURSOR_HOME ?= $(HOME)/.cursor
MARKETPLACE := augentic

.PHONY: ci install-cli use-local-plugins use-team-plugins
.PHONY: dev-doctor dev-check dev-run dev-live dev-full

# Full local gate: the Rust workspace CI (cargo make, Makefile.toml at the
# repo root). Framework prose invariants run as plain cargo tests
# (tests/framework/) inside the same gate.
ci:
	cargo make ci

# --- unified developer loop (scripts/dev.sh; mirrored in specify-adapters) ---

# Validate sibling layout, toolchain, WASI target, and cursor-agent.
# LIVE=1 adds a command-mode credential probe (one real model call —
# `cursor-agent status` alone does not prove --print auth).
dev-doctor:
	@bash scripts/dev.sh doctor $(if $(LIVE),--live,)

# Fastest model-free rung: native harness seam/replay tests, plus the
# named adapter's native tests when ADAPTER=<name> is given.
dev-check:
	@bash scripts/dev.sh check $(ADAPTER)

# Run specify-dev against any consumer project without changing
# directory: make dev-run PROJECT=/path/to/project ARGS='plan status'.
dev-run:
	@bash scripts/dev.sh run "$(PROJECT)" $(ARGS)

# One deliberate live-model run. Bare: the native-shim guest execute
# loop. ADAPTER=<name> [SCENARIO=<live test>]: exactly one adapter live
# eval scenario (prose overlay on once artifacts exist).
dev-live:
	@bash scripts/dev.sh live $(ADAPTER) $(SCENARIO)

# The explicit outer gate: doctor --live, deterministic checks,
# composed WASM/WIT coverage, and the composed guest execute loop.
# Never the default edit loop.
dev-full:
	@bash scripts/dev.sh full

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

# Clear the augentic plugin cache so Cursor refetches on restart.
# (`specify plugins refresh` retired with the native provisioning
# surface; the cache clear is a plain rm until it lands in-guest.)
use-team-plugins:
	rm -rf "$(CURSOR_HOME)/plugins/cache/$(MARKETPLACE)"
	@echo "Cleared $(MARKETPLACE) plugin cache; restart Cursor to refetch."

# Any other target passes through to cargo make (Makefile.toml), so the
# engine convention `make test` / `make fmt` / `make check` keeps working
# from the repo root.
.PHONY: %
%:
	@cargo make $@
