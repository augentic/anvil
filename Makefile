ifeq ($(wildcard specify-cli/Cargo.toml),)
  SPECIFY_MANIFEST ?= ../specify-cli/Cargo.toml
else
  SPECIFY_MANIFEST := specify-cli/Cargo.toml
endif

.PHONY: lint check-schemas ci acceptance use-local-plugins use-team-plugins sync-spec-runtime

sync-spec-runtime:
	bash ./scripts/sync-adapter-spec-runtime.sh

lint: sync-spec-runtime
	cargo run --release --manifest-path $(SPECIFY_MANIFEST) --bin specify -- lint framework --framework-root .

# Authoritative mirror list lives in scripts/check-schema-mirror.sh.
check-schemas:
	bash ./scripts/check-schema-mirror.sh

ci: lint check-schemas

# Deterministic acceptance surface only. Runs the static checks plus the
# fan_in_fan_out determinism proof, then points at the manual scenario sweep.
# Intentionally NOT a prerequisite of `ci`: it is a convenience, not a required
# automated acceptance check. It does not run, fake, record, or golden-compare
# the manual scenario pack, so every acceptance/suites/lifecycle/ negative
# expectation stays held. See docs/contributing/acceptance.md.
acceptance: lint
	cargo test --manifest-path $(SPECIFY_MANIFEST) --test fan_in_fan_out
	@echo ""
	@echo "Deterministic surface passed. Drive the manual scenario sweep per docs/contributing/acceptance.md (catalog: acceptance/suites/lifecycle/README.md)."

use-local-plugins:
	@bash ./scripts/use-local-plugins.sh

use-team-plugins:
	@bash ./scripts/use-team-plugins.sh
