DENO := $(or $(shell command -v deno 2>/dev/null),$(wildcard $(HOME)/.deno/bin/deno))

.PHONY: checks
checks:
	@$(DENO) run --allow-read scripts/checks.ts

.PHONY: test
test: checks

.PHONY: setup
setup:
	@command -v deno >/dev/null 2>&1 || { echo "Installing Deno..."; curl -fsSL https://deno.land/install.sh | sh; }
	@echo "Deno: $$($(DENO) --version | head -1)"

.PHONY: dev-plugins
dev-plugins:
	@./scripts/dev-plugins.sh

.PHONY: prod-plugins
prod-plugins:
	@./scripts/prod-plugins.sh
