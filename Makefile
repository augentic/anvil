.PHONY: checks
checks:
	@./scripts/checks.sh

.PHONY: merge-specs
merge-specs:
	@python3 plugins/spec/skills/archive/scripts/merge-specs.py $(ARGS)
