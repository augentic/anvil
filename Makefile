DENO := $(or $(shell command -v deno 2>/dev/null),$(wildcard $(HOME)/.deno/bin/deno))

.PHONY: checks
checks:
	@$(DENO) run --allow-read scripts/checks.ts

.PHONY: dev-plugins
dev-plugins:
	@./scripts/dev-plugins.sh

.PHONY: prod-plugins
prod-plugins:
	@./scripts/prod-plugins.sh
