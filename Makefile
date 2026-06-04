ifeq ($(wildcard specify-cli/Cargo.toml),)
  SPECIFY_MANIFEST ?= ../specify-cli/Cargo.toml
else
  SPECIFY_MANIFEST := specify-cli/Cargo.toml
endif

SPECIFY_BIN_DIR := $(abspath $(dir $(SPECIFY_MANIFEST))target/release)

.PHONY: lint acceptance use-local-plugins use-team-plugins

lint:
	cargo run --release --manifest-path $(SPECIFY_MANIFEST) --bin specify -- lint framework --framework-root .

# Deterministic acceptance surface only. Builds the release binary, runs the
# static checks plus the fixture-backed acceptance tests (the deterministic
# proof for every scenario marked `automated` in the catalog), then prints the
# PATH export line that puts this build's `specify` on PATH for the manual
# sweep. Deliberately NOT wired into CI. `cargo make test` in specify-cli
# remains the full deterministic surface.
acceptance:
	cargo build --release --manifest-path $(SPECIFY_MANIFEST) --bin specify
	@$(MAKE) lint
	cargo test --release --manifest-path $(SPECIFY_MANIFEST) --test fan_in_fan_out --test source_extract --test slice --test plan_orchestrate --test workspace
	@echo
	@echo "Put this build's specify on your PATH for the manual sweep:"
	@echo "    export PATH=\"$(SPECIFY_BIN_DIR):$$PATH\""

use-local-plugins:
	@bash ./scripts/use-local-plugins.sh

use-team-plugins:
	@bash ./scripts/use-team-plugins.sh
