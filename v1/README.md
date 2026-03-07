# OpenSpec Registry

## Using the Registry

### 1. Discovery: "What services are in the vessel domain?"

Quick query (grep is fine for TOML)

```bash
rg 'domain = "vessel"' registry.toml
```

### 2. Impact analysis: "What consumes vessel-position events?"

Read `domains/vessel/README.md` — the event contracts section lists producers and consumers. The agent (or human) uses this to determine which services need spec deltas when the event schema changes.

### 3. Planning a cross-repo change

When running `/opsx:propose` in the central planning repo, the agent:

- Reads registry.toml to identify affected services by domain or capability
- Fetches current specs from each service repo (read-only git clone or API)
- Reads domain shared-types to understand cross-service contracts
- Produces per-service spec deltas under `openspec/changes/<name>/specs/<service>/`
- Produces a single manifest.md and pipeline.toml covering all targets

### 4. Executing the pipeline

The pipeline.toml references services by id, and the execution pipeline resolves each id to its repo, crate, and project_dir from the registry:

```toml
# pipeline.toml (in the change folder)
change = "openspec/changes/ais-v2-modernization"
registry = "../../registry.toml"

[[targets]]
id = "ais-connector"           # resolved from registry
specs = ["vessel-ingestion", "voyage-enrichment", "legacy-status"]
route = "crate-updater"

[[targets]]
id = "event-gateway"           # resolved from registry
specs = ["voyage-events"]
route = "crate-updater"

[[dependencies]]
from = "ais-connector"
to = "event-gateway"
type = "event-schema"
contract = "domains/vessel/shared-types.md#VesselPosition"

concurrency = 1
stop_on_dependency_failure = true
```

### 5. Keeping the registry current

When a new service is created (via the greenfield tdd-gen path), append an entry to registry.toml. When a service is decommissioned, remove it. The registry is version-controlled in the central repo — changes to it are PRs like anything else.