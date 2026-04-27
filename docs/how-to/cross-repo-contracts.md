# Work with Contracts Across Repos

API contracts define the machine-readable interface shapes between components. In a multi-repo platform, contracts live at `.specify/contracts/` -- a neutral, platform-level location alongside `registry.yaml`.

## The contract-first pattern

When `/spec:plan` detects an API boundary between two projects in the registry, it automatically inserts a **contract change** before the implementation changes:

```yaml
changes:
  - name: auth-api-contract
    schema: contracts@v1
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

If you are not using `/spec:plan`, you can create contract changes manually:

```text
/spec:init https://github.com/augentic/specify/schemas/contracts

/spec:define "Define the user registration API contract"
/spec:build
/spec:merge
```

This produces contract artifacts at `.specify/contracts/`:

```text
.specify/contracts/
├── schemas/user-registration.yaml     # JSON Schema payload
├── http/user-api.yaml                 # OpenAPI 3.1 binding
└── messages/user-events.yaml          # AsyncAPI 3.0 binding (if messaging)
```

## Alignment validation

When you define an implementation change (Omnia or Vectis schema), the define pipeline includes a **contracts alignment** stage. This compares your specs against baseline contracts and reports:

- **Coverage:** Interactions already defined in contracts.
- **Alignment warnings:** Spec-vs-contract mismatches.
- **Generated delta:** New contract files for uncovered interactions.

## Distributing contracts across repos

In a multi-repo setup, contracts live in the initiating repo's `.specify/contracts/`. After execution, `specify workspace push` publishes changes to each target repo. The contract files serve as the shared vocabulary -- both producer and consumer reference the same definitions.

## See also

- [Cross-Repo Initiatives](../tutorials/cross-repo-initiative.md) -- tutorial on multi-repo planning
- [Contracts plugin](../reference/plugins/contracts.md) -- plugin reference
- [Contracts schema](../reference/schemas/contracts.md) -- schema reference
- [Artifact Format (contracts)](../reference/artifact-format.md#contract-artifacts-api-shape) -- format details
