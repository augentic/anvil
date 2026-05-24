TOOLING_MANIFEST := tooling/Cargo.toml

.PHONY: check test ci use-local-plugins use-team-plugins

# Regenerate CLI envelope docs: cargo run --release --manifest-path $(TOOLING_MANIFEST) -- docgen envelopes
# Verify drift (CI): cargo run --release --manifest-path $(TOOLING_MANIFEST) -- docgen envelopes --check

check:
	cargo run --release --manifest-path $(TOOLING_MANIFEST) -- check

test:
	cargo test --manifest-path $(TOOLING_MANIFEST)

ci: check test

use-local-plugins:
	@bash ./scripts/use-local-plugins.sh

use-team-plugins:
	@bash ./scripts/use-team-plugins.sh
