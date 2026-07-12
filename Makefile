# Convenience pass-through to Makefile.toml.
.PHONY: %
%:
	@cargo make $@
