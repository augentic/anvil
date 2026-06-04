ifeq ($(wildcard specify-cli/Cargo.toml),)
  SPECIFY_MANIFEST ?= ../specify-cli/Cargo.toml
else
  SPECIFY_MANIFEST := specify-cli/Cargo.toml
endif

.PHONY: lint ci acceptance use-local-plugins use-team-plugins

lint:
	cargo run --release --manifest-path $(SPECIFY_MANIFEST) --bin specify -- lint framework --framework-root .

ci: lint

use-local-plugins:
	@bash ./scripts/use-local-plugins.sh

use-team-plugins:
	@bash ./scripts/use-team-plugins.sh
