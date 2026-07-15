# Convenience pass-through to Makefile.toml.
.PHONY: %
%:
# 	@cargo make $@
	@if [ "$@" = "$(firstword $(MAKECMDGOALS))" ]; then \
		cargo make "$@" $(wordlist 2,$(words $(MAKECMDGOALS)),$(MAKECMDGOALS)); \
	fi
