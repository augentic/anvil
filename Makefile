DENO := $(or $(shell command -v deno 2>/dev/null),$(wildcard $(HOME)/.deno/bin/deno))

.PHONY: checks
checks:
	@$(DENO) run --allow-read scripts/checks.ts

.PHONY: doc-envelopes
doc-envelopes:
	@$(DENO) run --allow-read --allow-write --allow-env scripts/gen-envelope-doc.ts

.PHONY: test
test:
	@$(DENO) test \
		--allow-read --allow-write --allow-env --allow-run --allow-net=none \
		tests/cross_repo.ts

.PHONY: test-migration
test-migration:
	@$(DENO) test \
		--allow-read --allow-write --allow-env --allow-run \
		tests/migration_test.ts

.PHONY: use-local-plugins
use-local-plugins:
	@bash ./scripts/use-local-plugins.sh

.PHONY: use-team-plugins
use-team-plugins:
	@bash ./scripts/use-team-plugins.sh
