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

# Cargo target directory, honouring an exported CARGO_TARGET_DIR.
TARGET_DIR := $(if $(CARGO_TARGET_DIR),$(CARGO_TARGET_DIR),target)

# Framework lint over the prose surface. The lint runs in the workflow
# guest like every other verb: build the guest, then drive it through
# the replay runtime (ModelDefault — no cursor-agent needed) with the
# repo mounted writable at ".". The adapter-contract links resolve the
# guest's imports; lint never dispatches them.
lint:
	cargo build -q -p specify-workflow-guest --target wasm32-wasip2
	HTTP_ADDR=127.0.0.1:0 cargo run -q -p specify-runtime --bin specify-runtime-replay -- run \
		"$(TARGET_DIR)/wasm32-wasip2/debug/specify_workflow_guest.wasm" \
		--mount path=.,writable \
		--link specify:adapter/source@0.1.0 --link specify:adapter/target@0.1.0 \
		-- lint framework --framework-root .

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
