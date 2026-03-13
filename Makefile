# dynamically target Makefile.toml
.PHONY: %
%:
	@cargo make $@

.PHONY: checks
checks:
	@./scripts/checks.sh