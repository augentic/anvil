TOOLING_MANIFEST := tooling/Cargo.toml

# In-repo sparse checkout (CI layout) when present; else sibling default.
ifeq ($(wildcard specify-cli/schemas/source.schema.json),)
  SPECIFY_CLI_DIR ?= ../specify-cli
else
  SPECIFY_CLI_DIR := specify-cli
endif
export SPECIFY_CLI_DIR

.PHONY: check test ci use-local-plugins use-team-plugins

check:
	cargo run --release --manifest-path $(TOOLING_MANIFEST) -- check

test:
	cargo test --manifest-path $(TOOLING_MANIFEST)

ci: check test

use-local-plugins:
	@bash ./scripts/use-local-plugins.sh

use-team-plugins:
	@bash ./scripts/use-team-plugins.sh
