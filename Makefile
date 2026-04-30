DENO := $(or $(shell command -v deno 2>/dev/null),$(wildcard $(HOME)/.deno/bin/deno))

.PHONY: checks
checks:
	@$(DENO) run --allow-read scripts/checks.ts

.PHONY: use-local-plugins
use-local-plugins:
	@bash ./scripts/use-local-plugins.sh

.PHONY: use-team-plugins
use-team-plugins:
	@bash ./scripts/use-team-plugins.sh
