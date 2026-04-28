# specify registry

Manage the platform registry at `.specify/registry.yaml` -- the catalogue of repositories that make up a multi-repo platform. Optional for single-repo projects.

The registry was promoted from `specify initiative registry ...` to a top-level noun group in the CLI cleanup: `registry.yaml` is platform-scoped (it spans every initiative the platform runs), not initiative-scoped, so the verb shape now reflects that.

## Subcommands

### specify registry show

Render the registry content.

```bash
specify registry show [--format json]
```

`--format json` is the canonical shape consumed by `/spec:plan`'s sync-peers step (`projects.length > 1` triggers multi-repo authoring).

### specify registry validate

Check structural and referential invariants of `.specify/registry.yaml`.

```bash
specify registry validate
```

Validates:

- Schema-level shape (required fields, kebab-case names, well-formed `url:` values).
- `description` is required when more than one project is declared (multi-repo invariant).
- Per-project `schema:` URLs are resolvable.
- When `contracts` blocks are present on entries, the producer / consumer / imports invariants are coherent.

Used by `/spec:plan` after populating contract roles, and by operators who edit `registry.yaml` by hand.

## See also

- [specify initiative](initiative.md) -- operator brief at `.specify/initiative.md`.
- [specify workspace](workspace.md) -- materialise registry projects under `.specify/workspace/<peer>/`.
- [Configuration Files → registry.yaml](../configuration.md#registryyaml) -- file format reference.
- [Migrating CLI v1](../../explanation/migrating-cli-v1.md) -- rename map for the cleanup.
