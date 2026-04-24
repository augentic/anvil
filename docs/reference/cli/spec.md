# specify spec

Merge preview and baseline drift detection.

## Subcommands

### specify spec preview

Preview what a merge would do without writing anything.

```bash
specify spec preview <change-dir>
```

Shows which baseline specs would be created, modified, or removed. Used by `/spec:merge` before committing.

### specify spec conflict-check

Detect whether the baseline has changed since the change was defined.

```bash
specify spec conflict-check <change-dir>
```

Returns a pass/fail result. If conflicts are detected, the change's specs may need to be regenerated against the current baseline.
