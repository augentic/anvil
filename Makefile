ifeq ($(wildcard specify-cli/Cargo.toml),)
  SPECDEV_MANIFEST ?= ../specify-cli/Cargo.toml
else
  SPECDEV_MANIFEST := specify-cli/Cargo.toml
endif

.PHONY: lint check-schemas ci use-local-plugins use-team-plugins

lint:
	cargo run --release --manifest-path $(SPECDEV_MANIFEST) --bin specdev -- lint --framework-root .

# Authoritative mirror list lives in scripts/check-schema-mirror.sh.
check-schemas:
	bash ./scripts/check-schema-mirror.sh

ci: lint check-schemas

use-local-plugins:
	@bash ./scripts/use-local-plugins.sh

use-team-plugins:
	@bash ./scripts/use-team-plugins.sh
