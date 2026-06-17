# `documentation` source adapter fixture

Worked example for the `documentation` source adapter at [`adapters/sources/documentation/`](../../../../adapters/sources/documentation/). Exercises both operations of the contract: `survey` emits one lead per top-level concept under `## Lead inventory` in `discovery.md`; `extract` returns one Evidence YAML per lead with `documentation` authority and the `requirement` / `criterion` / `decision` / `section` claim kinds.

## Layout

```text
input/
  account.md           # top heading: "Account"
  password-reset.md    # top heading: "Password reset"
expected/
  discovery.md         # expected survey output (lead inventory section)
  evidence/
    account.yaml       # expected extract output for lead: account
    password-reset.yaml # expected extract output for lead: password-reset
```

## Bindings assumed by the fixture

- `<source>` = `product-notes`
- `$SOURCE_DIR` = `input/`
- Two lead ids: `account`, `password-reset` (alphabetical order)

## Validation

The Evidence YAMLs under `expected/evidence/` validate against [`schemas/evidence.schema.json`](https://github.com/augentic/specify/blob/main/cli/schemas/evidence.schema.json) in the CLI repo. The lead blocks in `expected/discovery.md` follow the grammar in [`schemas/discovery/lead.schema.json`](https://github.com/augentic/specify/blob/main/cli/schemas/discovery/lead.schema.json).
