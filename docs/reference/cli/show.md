# emery show

Print a reviewable document of the current generation to stdout.

## Synopsis

```bash
emery show spec
emery show design
```

## Description

The one read verb: renders the named document of the generation the `current` pointer names — a verifiable, non-authoritative projection of the store, never a second authority. `spec` and `design` are the whole generation; there is no `show bindings` or `show receipts`.

Text output is the document body alone — a deliberate exception to the result-line convention so `emery show spec | less` (or a redirect) is the document byte for byte. The generation id rides the JSON envelope.

There is no working-tree copy to edit. Changing the specification means changing a *source* — the intent text, the workspace the adapters extract, or the adapter list — and re-running [`emery specify`](specify.md).

Before any generation is committed the verb fails typed with `spec-not-generated` (exit `1`). A pointer naming a missing or unreadable generation fails closed with `spec-home-corrupt` — corruption is never an empty result.

## Options

| Option | Description |
|--------|-------------|
| `spec` \| `design` (positional) | Which reviewable document to print. |
| `--format` | Global output format: `json` wraps the document with its generation id. |

## JSON output

When `--format json` is provided, returns:

- `generation` — the current generation id
- `document` — which document `body` carries: `spec` or `design`
- `body` — the document bytes

## See also

- [`emery specify`](specify.md) commits the generation this verb renders; see the [CLI reference](index.md).
