# sources-only — `/change:execute` resolves a single source key

The plan entry `user-registration` declares `sources: [monolith]` and a description with no delta-targeting intent. `/change:execute` looks up `monolith` in the plan's top-level `sources` map, finds the local path `/path/to/legacy-codebase`, and passes the pair through to `/spec:define` as a single `source` flag.

## Resolution trace

```text
plan entry: user-registration
  sources: [monolith]
plan's top-level sources map:
  monolith: /path/to/legacy-codebase

resolve "monolith":
  → found in top-level map
  → value is a local filesystem path (starts with "/")
  → pass through as --source monolith=/path/to/legacy-codebase
```

## Pinned invocation

Contents of `invocation.txt`:

```text
/spec:define user-registration source monolith=/path/to/legacy-codebase
```

## Rendered define step

When the driver emits the per-slice output block for this entry, the `Processing:` header suffix carries the `sources` list and the define step body shows the extract sub-step with the resolved path on the `Source:` line:

```text
### Processing: user-registration (sources: [monolith])

Step 1/3: define
  - extract sub-step (via /spec:extract)
      Source: /path/to/legacy-codebase
      Artifacts: specs/user-registration/spec.md, design.md ✓
  Artifacts: proposal.md, specs, design.md, tasks.md ✓
```

The `Source:` value is the verbatim string from the plan's top-level `sources` map; the driver did not canonicalize, expand (`~`, `$HOME`), or statfs the path.

## Invariants pinned

1. **Key (`monolith`) travels verbatim** from the plan entry into the `--source` flag. Define's brief pipeline receives the key and can retain it for provenance when it hands the value to `/spec:extract`.
2. **No `--affects` flag is emitted.** The `affects` field has been removed from the plan schema. Delta targeting is inferred by the define skill from the change's description.
3. **Path classification is content-only.** The driver recognised `/path/to/legacy-codebase` as a local path (leading `/`) rather than a git URL. The classification affects only the transcript's extract-sub-step phrasing; the flag itself carries the string unchanged either way.
