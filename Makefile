ifeq ($(wildcard specify-cli/Cargo.toml),)
  SPECDEV_MANIFEST ?= ../specify-cli/Cargo.toml
else
  SPECDEV_MANIFEST := specify-cli/Cargo.toml
endif

.PHONY: check test ci use-local-plugins use-team-plugins

check:
	cargo run --release --manifest-path $(SPECDEV_MANIFEST) --bin specdev -- check --framework-root .

test:
	cargo test --manifest-path $(SPECDEV_MANIFEST) -p specify-authoring

ci: check test

use-local-plugins:
	@bash ./scripts/use-local-plugins.sh

use-team-plugins:
	@bash ./scripts/use-team-plugins.sh
