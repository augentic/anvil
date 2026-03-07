# Pipeline Configuration

Generate the execution config for the pipeline.

## Instructions

1. Read the manifest to identify all targets and their routes
2. For each target, include:
   - crate name and project directory
   - repo URL (from registry.toml if multi-repo, or current repo)
   - which spec files apply to this target
   - the route decision (crate-updater or tdd-gen)
3. Declare cross-target dependencies from the manifest
4. Set concurrency (default: 1 for dependent targets, parallel for
   independent targets)

## Output: pipeline.toml

change = "openspec/changes/<change-name>"

[[targets]]
id = "<target-id>"
crate = "<crate-name>"
project_dir = "<path>"
route = "<crate-updater|tdd-gen>"
specs = ["<capability-1>", "<capability-2>"]

[[dependencies]]
from = "<target-id>"
to = "<target-id>"
type = "<event-schema|api-contract|shared-type>"

concurrency = 1