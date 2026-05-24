TOOLING_MANIFEST := tooling/Cargo.toml

.PHONY: check test ci use-local-plugins use-team-plugins

# Regenerate CLI envelope docs: cargo specify-docgen-envelopes
# Verify drift (CI):             cargo specify-docgen-envelopes --check
# (Aliases live in .cargo/config.toml; the bare cargo invocations still work.)

check:
	cargo run --release --manifest-path $(TOOLING_MANIFEST) -- check

test:
	cargo test --manifest-path $(TOOLING_MANIFEST)

ci: check test

use-local-plugins:
	@bash ./scripts/use-local-plugins.sh

use-team-plugins:
	@bash ./scripts/use-team-plugins.sh
