# source.extract

Emit one `Evidence` document from the bound greeting source.

Read [`references/greeting.md`](../references/greeting.md) via `read_doc` before answering.

## Inputs

- `$SOURCE_DIR` — read-only view of the bound greeting tree. Absent when the binding is an inline `value` (the material is then in the message).
- **Source key** — the authored binding key the engine passed on the wire.

Nothing outside the bound source is reachable. Extract mines this source completely in one pass.

## Claim kinds

| Kind | Required body field | When to emit |
|---|---|---|
| `requirement` | `statement` | The one greeting behaviour the tree (or the reference) states. |
| `criterion` | `criterion` | Only when the bound tree itself states an acceptance criterion. |

Do not invent a `criterion`. A `requirement` without `statement` fails the run closed.

## `id` derivation

- The requirement id stays `greeting.behaviour`.
- A criterion id equals that id or extends it with a dotted suffix (`greeting.behaviour.body`).

## Output contract

```json
{
  "authority": "documentation",
  "claims": [
    {
      "kind": "requirement",
      "id": "greeting.behaviour",
      "path": "greeting.md#L3",
      "statement": "GET /greeting returns the static string 'hello'."
    },
    {
      "kind": "criterion",
      "id": "greeting.behaviour.body",
      "path": "greeting.md#L6",
      "criterion": "The response body is exactly `hello`."
    }
  ]
}
```

Claims from an inline value omit `path`. The caller persists the document; do not write it yourself.
