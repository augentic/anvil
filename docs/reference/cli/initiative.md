# specify initiative

Manage the operator-authored initiative brief and platform registry.

## Subcommands

### specify initiative brief

Manage the operator-authored initiative brief.

```bash
specify initiative brief init
specify initiative brief show
```

`init` scaffolds `.specify/initiative.md` with frontmatter template. `show` renders the brief content.

### specify initiative registry

Manage the platform registry.

```bash
specify initiative registry show
specify initiative registry validate
```

`show` renders `registry.yaml` content. `validate` checks required fields (e.g. `description` required when multiple projects exist).
