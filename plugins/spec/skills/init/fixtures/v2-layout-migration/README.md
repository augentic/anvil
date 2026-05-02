# v2 layout migration (fixture)

Walked-through example of `specify migrate v2-layout` upgrading a v1-layout single-repo project. The fixture is illustrative — no test harness consumes it; it ships alongside the [migration how-to](../../../../../../docs/how-to/migrate-to-v2-layout.md) as a reference shape.

## Before (v1 layout)

```text
my-project/
├── src/
│   └── lib.rs
└── .specify/
    ├── project.yaml
    ├── registry.yaml          # version: 1, projects: [{ name: my-project, url: ., schema: omnia@v1 }]
    ├── plan.yaml              # name: refactor-checkout, changes: […]
    ├── initiative.md          # name: refactor-checkout
    └── contracts/
        ├── schemas/
        │   └── checkout-payload.yaml
        └── http/
            └── checkout-api.yaml
```

Any project-aware verb against this shape errors with the stable `legacy-layout` code:

```bash
$ specify --format json status
```

```json
{
  "schema-version": 2,
  "error": "legacy-layout",
  "message": "legacy v1 layout detected; run `specify migrate v2-layout` to upgrade ([\".specify/registry.yaml\", \".specify/plan.yaml\", \".specify/initiative.md\", \".specify/contracts\"])",
  "exit_code": 1
}
```

## Migrate

```bash
$ specify --format json migrate v2-layout
```

```json
{
  "schema-version": 2,
  "moves": [
    { "from": ".specify/registry.yaml", "to": "registry.yaml", "status": "moved" },
    { "from": ".specify/plan.yaml",     "to": "plan.yaml",     "status": "moved" },
    { "from": ".specify/initiative.md", "to": "initiative.md", "status": "moved" },
    { "from": ".specify/contracts",     "to": "contracts",     "status": "moved" }
  ],
  "any-legacy-present": true,
  "any-collisions": false
}
```

## After (v2 layout)

```text
my-project/
├── src/
│   └── lib.rs
├── registry.yaml              # operator-facing (root)
├── plan.yaml                  # operator-facing (root)
├── initiative.md              # operator-facing (root)
├── contracts/                 # operator-facing (root)
│   ├── schemas/
│   │   └── checkout-payload.yaml
│   └── http/
│       └── checkout-api.yaml
└── .specify/
    └── project.yaml           # framework-managed (under .specify/)
```

A subsequent `specify status` succeeds. The migration is idempotent: re-running `specify migrate v2-layout` on the v2-layout project exits 0 with `nothing to migrate`.

## See also

- [`specify migrate v2-layout`](../../../../../../docs/reference/cli/migrate.md) — wire-shape CLI reference.
- [Migrating to the v2 layout](../../../../../../docs/how-to/migrate-to-v2-layout.md) — full operator walkthrough including multi-repo platforms and collision recovery.
