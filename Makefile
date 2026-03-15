.PHONY: checks
checks:
	@./scripts/checks.sh

.PHONY: dev-plugins
dev-plugins:
	@./scripts/dev-plugins.sh

.PHONY: prod-plugins
prod-plugins:
	@./scripts/prod-plugins.sh
