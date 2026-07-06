# specify source and target resolve

Adapter resolution splits by axis into two entry points.

## specify source resolve

Validate and locate a source adapter manifest.

```bash
specify source resolve <name>
```

Returns the adapter root and manifest path; the `survey` and `extract` prompts are compiled into the adapter guest. Used by the survey and extract orchestrations.

## specify target resolve

Validate and locate a target adapter manifest.

```bash
specify target resolve <value>
```

`<value>` may be a bare adapter name (`omnia`), a URL, or a local path. Returns the adapter root and manifest path; the `guidance`, `build`, and `merge` prompts are compiled into the adapter guest. Used by the refine, build, and merge orchestrations.

## Caching

Resolved manifests cache in the out-of-tree per-project cache at `<project-cache>/manifests/{sources,targets}/<name>/`. Cache layout is per-axis.

## See also

- [Anatomy of an adapter](../../explanation/adapter-anatomy.md) — source vs target contract
- [Directory layout](../directory-layout.md) — adapter paths in the repo
- [Target adapters](../targets/index.md) — first-party target adapters
