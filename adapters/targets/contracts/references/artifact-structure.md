# Artifact Structure

Directory layout, naming conventions, and change-level delta rules for API contract artifacts.

## Baseline Directory Layout

Contract artifacts live at root `contracts/` — a platform-level directory outside `.specify/` so interface definitions are visible as ordinary repository artifacts:

```text
contracts/
├── schemas/           # JSON Schema payload definitions
│   ├── user-registration.yaml
│   ├── user.yaml
│   ├── order-placed.yaml
│   └── error-response.yaml
├── http/              # OpenAPI 3.1 bindings
│   └── user-api.yaml
└── messages/          # AsyncAPI 3.0 bindings
    └── order-events.yaml
```

## Directory Rules

| Directory | When Present | Contents |
|-----------|-------------|----------|
| `contracts/schemas/` | **Always.** Every contract includes at least one payload schema. | JSON Schema files — one per domain type. |
| `contracts/http/` | When the platform includes HTTP interactions (REST endpoints, request/response patterns). | OpenAPI 3.1 binding files. |
| `contracts/messages/` | When the platform includes messaging interactions (pub/sub, event-driven, queue-based). | AsyncAPI 3.0 binding files. |

- `schemas/` is mandatory. If a contract has no schemas, it has no contract.
- `http/` and `messages/` are optional, present when applicable. Both may exist simultaneously when the platform uses both HTTP and messaging.
- `http/` is omitted for purely event-driven systems.
- `messages/` is omitted for purely synchronous HTTP systems.

## Why Platform-Level?

Contracts sit outside the per-adapter spec tree. A single OpenAPI document or schema type often spans multiple adapters — a `POST /users` endpoint might touch `user-registration`, `auth`, and `notifications` adapters. Flattening contracts out of the adapter hierarchy avoids the question of "which adapter owns this schema?" — nobody does; it is platform vocabulary.

Three platform concerns, three top-level locations:

- **`registry.yaml`** declares *who* the participants are.
- **`plan.yaml`** declares *what* changes are planned.
- **`contracts/`** declares *how* participants communicate.

## Naming Conventions

All contract files use **kebab-case** names with `.yaml` extensions, consistent with Specify's naming conventions for spec files, change directories, and plan entries.

| File Type | Named After | Examples |
|-----------|------------|----------|
| Schema files | The domain type they define | `user-registration.yaml`, `error-response.yaml`, `order-placed.yaml` |
| HTTP binding files | The API domain they describe | `user-api.yaml`, `billing-api.yaml` |
| Message binding files | The event domain they describe | `order-events.yaml`, `notification-events.yaml` |

One type per schema file. A single binding file may contain multiple related endpoints or channels.

## Slice-Level Delta

During a slice's define phase, proposed contract modifications live in the slice directory:

```text
.specify/slices/add-oauth/
├── contracts/
│   ├── schemas/
│   │   └── oauth-token.yaml        # New type
│   └── http/
│       └── user-api.yaml           # Updated OpenAPI (additional paths)
├── specs/
├── design.md
└── ...
```

### Delta Rules

1. **Only changed files.** The slice-level `contracts/` directory contains only the files this slice adds or replaces — not a full copy of the baseline. This keeps the diff reviewable and makes it clear what a single change contributes to the platform's contract surface.

2. **Opaque replacement.** Contract files use whole-file replacement semantics. Unlike spec files which use the ADDED/MODIFIED/REMOVED delta format, contract files are replaced wholesale. JSON Schema and OpenAPI/AsyncAPI files have their own versioning semantics (`$id`, `info.version`); a second delta-merge algorithm for YAML contract files would add complexity without benefit.

3. **No deletion mechanism.** The slice-level directory can express additions and replacements but not deletions. There is no mechanism to say "remove this file from the baseline." Contract deletion (retiring an endpoint or decommissioning a channel) is rare and is handled as a manual baseline edit.

4. **Same subdirectory structure.** The slice-level `contracts/` directory mirrors the baseline structure: `schemas/`, `http/`, `messages/`. A change that adds a new schema and updates an HTTP binding has both `contracts/schemas/new-type.yaml` and `contracts/http/existing-api.yaml`.

### Merge Semantics

When `specify slice merge run` processes a slice:

- Files in the slice's `contracts/` are copied into root `contracts/`, replacing files at the same path.
- Files absent from the slice's `contracts/` are left untouched in the baseline.
- New files (paths that do not exist in the baseline) are added.

### Conflict Detection

Two concurrent changes that both modify the same contract file (e.g. both add paths to `http/user-api.yaml`) will conflict. `specify slice merge conflict-check` detects this: if the baseline file was modified after the slice's `defined-at` timestamp, the merge is blocked. Resolution: re-run the slice's define phase against the updated baseline.

## Baseline vs Change-Level

| Aspect | Baseline | Change-Level |
|--------|----------|-------------|
| Location | `contracts/` | `.specify/slices/<name>/contracts/` |
| Scope | Full platform contract surface | Only files this slice adds or replaces |
| Lifetime | Persists across changes | Exists during the slice lifecycle, merged or dropped |
| Authority | Source of truth for the current contract state | Proposed modification, pending review and merge |

The baseline is what the writer validates specs against. The slice-level delta is what the writer produces when specs describe interactions not covered by the baseline.

## Multi-Repo Distribution

In multi-repo initiatives, contracts live in the initiating repo's root `contracts/` directory. Distribution to project clones uses the workspace infrastructure: `specify workspace sync` materialises root `contracts/` into each project clone automatically as part of the multi-repo plan-time sync.

Phase skills always read from root `contracts/` relative to their working directory — they do not need to know whether contracts were authored locally or materialised from a central source.

## See Also

- [json-schema-conventions.md](json-schema-conventions.md) -- JSON Schema payload rules
- [openapi-conventions.md](openapi-conventions.md) -- OpenAPI 3.1 binding conventions
- [asyncapi-conventions.md](asyncapi-conventions.md) -- AsyncAPI 3.0 binding conventions
