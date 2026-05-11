# Compatibility

`specify compatibility` reports cross-project consumer impact for contracts declared in `registry.yaml`.

```bash
specify compatibility check
specify compatibility report --change <name>
```

`compat` is accepted as a shorthand alias for the command family.

## Inputs

- `registry.yaml` with producer projects listing `contracts.produces` and consumer projects listing matching `contracts.consumes`.
- Producer contracts in root `contracts/`.
- Consumer workspace views in `.specify/workspace/<consumer>/contracts/`, usually materialised by `specify workspace sync`.

The command is read-only. It does not mutate workspace clones, write journals, transition plan entries, or replace the contracts baseline validator.

## Classification

Findings use four RM-04 classifications:

| Classification | Meaning |
|---|---|
| `additive` | Backwards-compatible delta, such as an optional field or new endpoint. |
| `breaking` | Recognized consumer-impacting delta, with `change-kind` when available. |
| `ambiguous` | Changed construct that the deterministic classifier cannot prove safe. |
| `unverifiable` | Missing, malformed, invalid, or unsupported comparison input. |

## Exit Codes

`specify compatibility report --change <name>` exits `0` when it can render the report. `specify compatibility check` emits the same report and exits `0` only when every finding is additive or clean; `breaking`, `ambiguous`, and `unverifiable` findings use the normal validation-failed exit code.

## JSON Shape

With `--format json`, the command emits the standard Specify CLI envelope:

```json
{
  "envelope-version": 6,
  "change": "user-api-v2",
  "checked-pairs": 1,
  "ok": false,
  "findings": [
    {
      "classification": "breaking",
      "change-kind": "required-field-added",
      "producer-project": "backend",
      "consumer-project": "mobile",
      "producer-contract": "contracts/http/user-api.yaml",
      "consumer-contract": "contracts/http/user-api.yaml",
      "locator": "paths./users.post.requestBody.content.application/json.schema.required",
      "details": "Producer contract adds required field `phone`"
    }
  ],
  "summary": {
    "total-findings": 1,
    "additive": 0,
    "breaking": 1,
    "ambiguous": 0,
    "unverifiable": 0
  }
}
```

See [Cross-Project Compatibility](../../../plugins/contract/references/cross-project-compatibility.md) for the `change-kind` vocabulary and classification policy.
