# specify source and target resolve

Adapter resolution splits by axis. The retired `specify adapter *` verb family is replaced by two entry points.

## specify source resolve

Validate and locate a source adapter manifest.

```bash
specify source resolve <name>
```

Returns the adapter root, manifest path, and brief paths for `survey` and `extract`. Used by `/spec:plan` during survey and `/spec:refine` during extract.

## specify target resolve

Validate and locate a target adapter manifest.

```bash
specify target resolve <value>
```

`<value>` may be a bare adapter name (`omnia`), a URL, or a local path. Returns manifest path and brief paths for `shape`, `build`, and `merge`. Used by `/spec:refine`, `/spec:build`, and `/spec:merge`.

## Caching

Resolved manifests cache under `.specify/.cache/manifests/{sources,targets}/<name>/`. Cache layout is per-axis and disjoint from the extraction cache at `.specify/.cache/extractions/`.

## See also

- [Anatomy of an adapter](../../explanation/adapter-anatomy.md) — source vs target contract
- [Directory layout](../directory-layout.md) — adapter paths in the repo
- [Target adapters](../targets/index.md) — first-party target adapters
