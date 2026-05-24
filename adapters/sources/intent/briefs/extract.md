# intent.extract

Emit one `Evidence` document carrying a single `kind: intent` claim. The `intent` source is degenerate — `extract` returns the operator's intent string verbatim, with `authority: intent` so downstream synthesis treats it as the highest-priority signal under the authority hierarchy (`intent > documentation > behaviour`).

## Inputs

- `Candidate` — the candidate id resolved from the slice's `Slice.sources` binding. For the degenerate intent path this equals the slice's `name`.
- `Source` — the `plan.yaml.sources.<source-key>` binding. `Source.value` carries the operator's intent string verbatim. `Source.path` is absent; no `$SOURCE_DIR` is preopened.
- `source-key` — the key the binding was registered under in `plan.yaml.sources` (typically `intent`).

## Output contract

Return one `Evidence` document for `/spec:refine` to persist at `.specify/slices/<slice>/evidence/<source-key>.yaml`:

```yaml
source: <source-key>
adapter: intent
authority: intent
candidate: <candidate-id>
claims:
  - kind: intent
    statement: "<Source.value, verbatim>"
```

Rules:

- `source` MUST equal the binding's `source-key` (referencing the top-level `plan.yaml.sources.<key>` entry). It is the source key, not the adapter name.
- `adapter` MUST be the literal string `intent`.
- `authority` MUST be the literal string `intent`. The `intent` adapter is the only first-party source that emits this authority class; `documentation` and code adapters emit `documentation` or `behaviour` per the authority hierarchy.
- `candidate` MUST equal the `Candidate` argument (the candidate id, not the slice name; the two are equal under the degenerate intent path).
- `claims` MUST contain exactly one entry with `kind: intent` and a `statement:` field carrying the operator's intent string verbatim. `claim-id` is optional on `kind: intent` — the Evidence schema only requires it on `requirement` and `criterion` kinds. Omit it unless the operator supplies a stable id.
- Do not emit a `path:` on the claim. The intent source has no filesystem locus; `path` is reserved for file-backed sources.
- Do not emit additional claims. Operators who want multi-claim intent split the work into multiple slices (the candidate per slice handles that).

## Worked example

Input:

- `Candidate` = `add-search-filter`
- `source-key` = `intent`
- `Source.value` = `Add a search filter to the user list.`

Output — `Evidence` document:

```yaml
source: intent
adapter: intent
authority: intent
candidate: add-search-filter
claims:
  - kind: intent
    statement: "Add a search filter to the user list."
```

## Notes

- Empty `claims: []` is schema-valid for sources with nothing to say, but the intent adapter is never legitimately empty — the candidate exists because the operator supplied an intent string. Treat an empty `Source.value` as an extract failure and stay `refining` per §Extraction reliability.
- Re-running `intent.extract` is idempotent: the same `(Candidate, Source)` pair yields a byte-identical Evidence document.
