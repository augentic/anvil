# `plan-validate-output/schema.json`

Canonical JSON Schema (2020-12) for the response body emitted by `specify plan validate --format json` under the v2 JSON contract (see [RFC-2](../../rfcs/archive/rfc-2-execution.md)).

## Producer

`specify plan validate --format json` (wired through [`run_plan_validate`](https://github.com/augentic/specify-cli/blob/rfc-2/src/main.rs)) always emits an object shaped like:

```json
{
  "schema-version": 2,
  "plan": {
    "name": "<kebab-initiative-name>",
    "path": "<absolute-or-relative-path-to-plan.yaml>"
  },
  "results": [
    {
      "level": "error" | "warning",
      "code": "<stable-identifier>",
      "entry": "<plan-entry-name>" | null,
      "message": "<human readable>"
    }
  ],
  "passed": true | false
}
```

`passed` is `true` when no `error`-level finding is present; warnings do not flip it. `results` is emitted as an array even when empty. The exit code is `0` when `passed` is `true`, and `2` (`EXIT_VALIDATION_FAILED`) otherwise.

## Consumer wiring

Skills that shell out to `specify plan validate --format json` should parse the response against this schema before branching on `results`. The recommended pattern in a Node- or Python-driven runner is to pin the schema via the checked-in file path rather than fetching it at runtime so validation stays hermetic:

```ts
// TypeScript / ajv
import Ajv from "ajv/dist/2020";
import schema from "../specify/schemas/plan-validate-output/schema.json" assert { type: "json" };

const ajv = new Ajv({ strict: true });
const validate = ajv.compile(schema);

function consumePlanValidate(stdout: string) {
  const payload = JSON.parse(stdout);
  if (!validate(payload)) {
    throw new Error(`plan-validate output failed schema check: ${JSON.stringify(validate.errors)}`);
  }
  return payload; // now safe to branch on .passed / .results[].code
}
```

The same `schema.json` is the source of truth for Rust-side CLI tests (`tests/plan.rs` under specify-cli); treat that file as the canonical consumer when patching the schema.

## Mirror

A byte-identical copy lives at [`augentic/specify-cli/schemas/plan-validate-output/schema.json`](https://github.com/augentic/specify-cli/tree/main/schemas/plan-validate-output/schema.json). When you edit the canonical file here, mirror the change to `specify-cli` in the same commit pair — the two files are covered by the `diff` byte-equality check in the RFC-2 cleanup acceptance tests.

## See also

- [`../plan/README.md`](../plan/README.md) — companion schema for the on-disk `plan.yaml` file this command validates.
- [`../plan/plan.schema.json`](../plan/plan.schema.json) — structural schema for the input; finds like `duplicate-name` and `dependency-cycle` reported here layer semantic checks on top.
