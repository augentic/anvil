DENO := $(or $(shell command -v deno 2>/dev/null),$(wildcard $(HOME)/.deno/bin/deno))

.PHONY: check
check:
	@$(DENO) run --allow-read --allow-env scripts/check.ts

.PHONY: doc-envelopes
doc-envelopes:
	@$(DENO) run --allow-read --allow-write --allow-env scripts/gen-envelope-doc.ts

.PHONY: test
test:
	@$(DENO) test \
		--allow-read --allow-write --allow-env --allow-run --allow-net=none \
		tests/cross_repo.ts

.PHONY: docs docs-serve docs-prereqs
# Requires: pinned mdbook + mdbook-d2 + mdbook-linkcheck + mdbook-pagetoc +
# mdbook-template + D2 on PATH. Bootstrap with `make docs-prereqs`; see
# docs/README.md for the version table.
docs:
	mdbook build docs

docs-serve:
	mdbook serve docs

docs-prereqs:
	@bash ./scripts/docs-prereqs.sh

.PHONY: ci
ci: check test

.PHONY: use-local-plugins
use-local-plugins:
	@bash ./scripts/use-local-plugins.sh

.PHONY: use-team-plugins
use-team-plugins:
	@bash ./scripts/use-team-plugins.sh
