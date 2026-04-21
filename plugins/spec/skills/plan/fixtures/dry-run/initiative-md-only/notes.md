Pins the RFC-3a §"When are registry.yaml and initiative.md required?"
readiness-gate widening: /spec:plan is valid with no CLI inputs when
initiative.md:inputs is non-empty.

The readiness report omits the `Sources:` block (no `--source` was
supplied) and adds an `Inputs` block naming the two entries the skill
read from `initiative.md:inputs` via `specify initiative brief show
--format json`, tagged with their `kind`. The provenance note
(`from initiative.md:inputs, no CLI flags supplied`) tells the
operator the inputs did not come from the command line.
