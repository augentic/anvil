# Work with Contracts Across Repos

API contracts define the machine-readable interface shapes between components. In a multi-repo platform, contracts live at `contracts/` -- a neutral, platform-level location alongside `registry.yaml`.

## The contract-first pattern

When `/change:draft` detects an API boundary between two projects in the registry, it automatically inserts a **contract change** before the implementation changes:

```yaml
changes:
  - name: auth-api-contract
    adapter: contracts@v1
    description: "Define the auth API contract"
    depends-on: []

  - name: add-auth-backend
    project: api
    depends-on: [auth-api-contract]

  - name: add-auth-ui
    project: mobile
    depends-on: [auth-api-contract]
```

The contract change defines the interface (JSON Schema payloads, OpenAPI bindings). Both implementation changes depend on it and can then execute in parallel.

## Manual contract workflow

If you are not using `/change:draft`, you can create contract changes manually:

```text
/spec:init https://github.com/augentic/specify/adapters/contracts

/spec:define "Define the user registration API contract"
/spec:build
/spec:merge
```

This produces contract artifacts at `contracts/`:

```text
contracts/
├── schemas/user-registration.yaml     # JSON Schema payload
├── http/user-api.yaml                 # OpenAPI 3.1 binding
└── messages/user-events.yaml          # AsyncAPI 3.0 binding (if messaging)
```

## Alignment validation

When you define an implementation change (Omnia or Vectis adapter), the define pipeline includes a **contracts alignment** stage. This compares your specs against baseline contracts and reports:

- **Coverage:** Interactions already defined in contracts.
- **Alignment warnings:** Spec-vs-contract mismatches.
- **Generated delta:** New contract files for uncovered interactions.

## Distributing contracts across repos

In a multi-repo setup, contracts live in the initiating repo's `contracts/`. After execution, `specify workspace push` publishes changes to each target repo. The contract files serve as the shared vocabulary -- both producer and consumer reference the same definitions.

## Cross-project compatibility classification (RM-04)

Run a compatibility report when a producer contract has changed and consumer workspace clones still hold their prior view:

```bash
specify compatibility check --change <name> --report-only   # read-only RM-04 report, always exits 0
specify compatibility check                                  # strict gate, exits 2 on non-additive findings
```

The report is read-only. It classifies producer-to-consumer deltas as `additive`, `breaking`, `ambiguous`, or `unverifiable`. The existing merge gate, `specify tool run contract -- "$PROJECT_ROOT/contracts" --format json`, still validates the merged baseline's SemVer and `x-specify-id` rules; compatibility reporting is a separate consumer-impact surface.

**Algorithm:**

1. Read the producer project's `contracts.produces` list from `registry.yaml`.
2. For each produced contract path, find consumer projects -- those listing the same path in `contracts.consumes`.
3. Compare root `contracts/<path>` with `.specify/workspace/<consumer>/contracts/<path>`.
4. Classify each comparable delta. Missing or malformed inputs become `unverifiable`; changed but unsupported constructs become `ambiguous`.

**Where the findings appear:**

- `specify compatibility check --change <name> --report-only` prints the report and exits `0` regardless of finding severity.
- `specify compatibility check` prints the same payload and exits validation-failed if any finding is `breaking`, `ambiguous`, or `unverifiable`.

**Triage:**

- If the consumer project is intentionally lagging (e.g. mobile shipping a release behind the backend), accept the drift and capture the rationale in the change or PR.
- If the consumer needs to be updated to match, spawn a follow-up consumer slice in the same plan or in a follow-up change. Use `specify plan add <name> --project <consumer> --depends-on <producer-slice>` to wire it up.
- See [Resolve cross-project compatibility findings](resolve-cross-project-contract-warnings.md) for the full triage checklist.

**What the check does not do:** it never modifies the consumer's specs, never auto-creates a follow-up change, and RM-04 does not transition plan state. RM-11 adds dependency-aware compatibility gates later.

## See also

- [Cross-Repo Changes](../tutorials/cross-repo-change.md) -- tutorial on multi-repo planning
- [Resolve cross-project compatibility findings](resolve-cross-project-contract-warnings.md) -- triage checklist
- [Contract plugin](../reference/plugins/contract.md) -- plugin reference
- [Contracts adapter](../reference/adapters/contracts.md) -- adapter reference
- [Artifact Format (contracts)](../reference/artifact-format.md#contract-artifacts-api-shape) -- format details
