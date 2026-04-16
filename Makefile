DENO := $(or $(shell command -v deno 2>/dev/null),$(wildcard $(HOME)/.deno/bin/deno))

.PHONY: build-vectis
build-vectis:
	cargo build --release --package vectis-cli
	cp target/release/vectis .

.PHONY: checks
checks:
	@$(DENO) run --allow-read scripts/checks.ts

.PHONY: dev-plugins
dev-plugins:
	@./scripts/dev-plugins.sh

.PHONY: prod-plugins
prod-plugins:
	@./scripts/prod-plugins.sh
