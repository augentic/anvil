# intent.extract

Emit one `Evidence` document carrying a single `kind: intent` claim. The `intent` source is degenerate — `extract` returns the operator's intent string verbatim, with `authority: intent` so downstream synthesis treats it as the highest-priority signal under the authority precedence (`intent > documentation > behaviour`) defined in [`authority.md`](../references/spec-runtime/synthesis/authority.md). Core synthesis reconciles it with any sibling sources' Evidence into the slice's `spec.md` — see [From sources to slices](../references/spec-runtime/reconciliation.md#slice-time-evidence-becomes-a-spec).

## Inputs

- `Lead` — the lead id resolved from the slice's `Slice.sources` binding. For the degenerate intent path this equals the slice's `name`.
- `Source` — the `plan.yaml.sources.<source>` binding. `Source.value` carries the operator's intent string verbatim. `Source.path` is absent; no `$SOURCE_DIR` is preopened.
- `source` — the key the binding was registered under in `plan.yaml.sources` (typically `intent`).

## Output contract

Return one `Evidence` document for `/spec:refine` to persist at `.specify/slices/<slice>/evidence/<source>.yaml`:

```yaml
authority: intent
lead: <lead>
claims:
  - id: <lead>
    kind: intent
    statement: "<Source.value, verbatim>"
```

The document's `(slice, source)` identity is carried by its on-disk path — the CLI persists it at `.specify/slices/<slice>/evidence/<source>.yaml`, deriving the `<source>.yaml` filename from the binding — and the adapter resolves from `plan.yaml.sources.<source>.adapter`. Neither is written in-document.

Rules:

- `authority` MUST be the literal string `intent`. The `intent` adapter is the only first-party source that emits this authority class; `documentation` and code adapters emit `documentation` or `behaviour` per the authority hierarchy.
- `lead` MUST equal the `Lead` argument (the lead id, not the slice name; the two are equal under the degenerate intent path).
- `claims` MUST contain exactly one entry with `kind: intent`, an `id:` set to the `Lead` id, and a `statement:` field carrying the operator's intent string verbatim. The `id` is the stable anchor synthesis references — although the Evidence schema only *requires* it on `requirement` / `criterion` / `example` kinds, an id-less intent claim is unanchorable, so the slice's sole requirement would render an empty `Sources:` line and fail `specify slice validate` (`spec.requirement-sources-empty`). Setting `id` equal to the lead keeps the document deterministic and idempotent.
- Do not emit a `path:` on the claim. The intent source has no filesystem locus; `path` is reserved for file-backed sources.
- Do not emit additional claims. Operators who want multi-claim intent split the work into multiple slices (the lead per slice handles that).

## Worked example

Input:

- `Lead` = `add-search-filter`
- `source` = `intent`
- `Source.value` = `Add a search filter to the user list.`

Output — `Evidence` document:

```yaml
authority: intent
lead: add-search-filter
claims:
  - id: add-search-filter
    kind: intent
    statement: "Add a search filter to the user list."
```

## Notes

- Empty `claims: []` is schema-valid for sources with nothing to say, but the intent adapter is never legitimately empty — the lead exists because the operator supplied an intent string. Treat an empty `Source.value` as an extract failure and stay `refining` per §Extraction reliability.
- Re-running `intent.extract` is idempotent: the same `(Lead, Source)` pair yields a byte-identical Evidence document.
