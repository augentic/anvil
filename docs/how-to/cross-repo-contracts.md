# Work with Contracts Across Repos

API contracts define the machine-readable interface shapes between components. In a multi-repo platform, contracts live at `contracts/` -- a neutral, platform-level location alongside `registry.yaml`.

## The contract-first pattern

When `/change:plan` detects an API boundary between two projects in the registry, it automatically inserts a **contract change** before the implementation changes:

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

If you are not using `/change:plan`, you can create contract changes manually:

```text
/spec:init https://github.com/augentic/specify/capabilities/contracts

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

When you define an implementation change (Omnia or Vectis schema), the define pipeline includes a **contracts alignment** stage. This compares your specs against baseline contracts and reports:

- **Coverage:** Interactions already defined in contracts.
- **Alignment warnings:** Spec-vs-contract mismatches.
- **Generated delta:** New contract files for uncovered interactions.

## Distributing contracts across repos

In a multi-repo setup, contracts live in the initiating repo's `contracts/`. After execution, `specify workspace push` publishes changes to each target repo. The contract files serve as the shared vocabulary -- both producer and consumer reference the same definitions.

## Cross-project contract validation (RFC-9 Section 3B)

After a producer change merges, `/change:execute` runs a cross-project compatibility check against every consumer project. The check is post-merge (the producer's contract is already in the baseline), advisory (warnings never halt the loop), and operator-triaged.

**Algorithm:**

1. Read the producer project's `contracts.produces` list from `registry.yaml`.
2. For each produced contract path, find consumer projects -- those listing the same path in `contracts.consumes`.
3. Run the format-appropriate `/contract:*` skill (verifier intent, with `--mode cross-project`) against each consumer's workspace clone, passing the updated contract: `/contract:openapi` for HTTP / resource APIs, `/contract:asyncapi` for evented / pub-sub / streaming, `/contract:json-schema` for shared payload schemas.
4. Surface each incompatibility as a warning in the merge transcript.
5. Write each warning to the merged change's `journal.yaml` as a `cross-project-warning:` entry, so the audit trail survives the change being archived.

**Where the warnings appear:**

- The `/change:execute` merge transcript prints a per-warning block (consumer project, contract path, finding type, finding detail) right after the per-slice merge summary.
- `specify slice journal show <change>` displays the same warnings keyed by `cross-project-warning:` even after the change is archived.

**Triage:**

- If the consumer project is intentionally lagging (e.g. mobile shipping a release behind the backend), accept the drift. The warning is in the journal for audit.
- If the consumer needs to be updated to match, spawn a follow-up consumer change in the same plan or in a follow-up initiative. Use `specify change plan add <name> --project <consumer> --depends-on <producer-change>` to wire it up.
- See [Resolve cross-project contract warnings](resolve-cross-project-contract-warnings.md) for the full triage checklist.

**What the check does not do:** it never halts the loop, never modifies the consumer's specs, never auto-creates a follow-up change. The framework reports drift; the operator decides what to do about it.

## See also

- [Cross-Repo Initiatives](../tutorials/cross-repo-change.md) -- tutorial on multi-repo planning
- [Resolve cross-project contract warnings](resolve-cross-project-contract-warnings.md) -- triage how-to for the post-merge check
- [Cross-project contract warnings on the merge transcript](../appendices/troubleshooting.md#cross-project-contract-warnings-on-the-merge-transcript) -- troubleshooting entry
- [Contract plugin](../reference/plugins/contract.md) -- plugin reference
- [Contracts capability](../reference/capabilities/contracts.md) -- capability reference
- [Artifact Format (contracts)](../reference/artifact-format.md#contract-artifacts-api-shape) -- format details
