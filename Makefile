DENO := $(or $(shell command -v deno 2>/dev/null),$(wildcard $(HOME)/.deno/bin/deno))

.PHONY: checks
checks:
	@$(DENO) run --allow-read scripts/checks.ts

# Direct RM-01 acceptance proof. The test creates a temp hub, two local
# fixture repos, fake gh/SSH, a deterministic contract-first plan, the
# baseline/residue commit split, workspace push, external merge marker,
# and finalize. It skips with exit 0 when `specify` is unavailable.
.PHONY: acceptance-cross-repo
acceptance-cross-repo:
	@$(DENO) test \
		--allow-read --allow-write --allow-env --allow-run --allow-net=none \
		tests/rm01_cross_repo_test.ts

.PHONY: acceptance-cross-repo-deterministic
acceptance-cross-repo-deterministic: acceptance-cross-repo

.PHONY: acceptance-all
acceptance-all: acceptance-cross-repo

.PHONY: acceptance-tiers
acceptance-tiers:
	@$(DENO) run \
		--allow-read --allow-run \
		scripts/acceptance-tier.ts $(TIER_ARGS)

.PHONY: use-local-plugins
use-local-plugins:
	@bash ./scripts/use-local-plugins.sh

.PHONY: use-team-plugins
use-team-plugins:
	@bash ./scripts/use-team-plugins.sh
