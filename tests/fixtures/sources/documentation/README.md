# `documentation` source adapter fixture

Worked example for the `documentation` source adapter at [`sources/documentation/`](../../../../sources/documentation/). Exercises both operations of the W2.2 contract: `enumerate` emits one candidate per top-level concept under `## Candidate inventory` in `discovery.md`; `extract` returns one Evidence YAML per candidate with `documentation` authority and the `requirement` / `criterion` / `decision` / `section` claim kinds.

## Layout

```text
input/
  account.md           # top heading: "Account"
  password-reset.md    # top heading: "Password reset"
expected/
  discovery.md         # expected enumerate output (candidate inventory section)
  evidence/
    account.yaml       # expected extract output for candidate: account
    password-reset.yaml # expected extract output for candidate: password-reset
```

## Bindings assumed by the fixture

- `<source-key>` = `product-notes`
- `$SOURCE_DIR` = `input/`
- Two candidate ids: `account`, `password-reset` (alphabetical order)

## Validation

The Evidence YAMLs under `expected/evidence/` validate against [`schemas/evidence.schema.json`](https://github.com/augentic/specify-cli/blob/main/schemas/evidence.schema.json) in the CLI repo. The candidate blocks in `expected/discovery.md` follow the grammar in [`schemas/discovery/candidate.schema.json`](https://github.com/augentic/specify-cli/blob/main/schemas/discovery/candidate.schema.json).
