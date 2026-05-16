Pins the RFC-3a §"When are registry.yaml and change.md required?" readiness-gate widening: /change:draft is valid with no CLI inputs when change.md:inputs is non-empty.

The readiness report omits the `Sources:` block (no `--source` was supplied) and adds an `Inputs` block naming the two entries the skill read from `change.md:inputs` via `specify change show --format json`, tagged with their `kind`. The provenance note (`from change.md:inputs, no CLI flags supplied`) tells the operator the inputs did not come from the command line.
