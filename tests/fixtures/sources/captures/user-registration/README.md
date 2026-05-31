# `captures` source adapter fixture — `user-registration`

Worked example for the [`captures`](../../../../../adapters/sources/captures/) source adapter (the capture-backed replay workflow). Exercises both operations of the source contract: `survey` emits one lead per `tests/data/replays/<handler>/` directory under `## Lead inventory`; `extract` emits one `kind: example` claim per scenario file, each carrying a `sha256:` fixture digest computed over the on-disk bytes.

## Layout

```text
inputs/
  tests/data/replays/user-registration/
    happy.json            # 201 + user.created publish
    error.json            # 400 weak-password
expected/
  discovery.md            # survey output (one lead, `runtime` source key)
  evidence.yaml           # extract output (two `kind: example` claims, alphabetical)
  provenance.yaml             # sample provenance.yaml (two REQ-* / single-source resolutions)
```

## Bindings assumed by the fixture

- `<source>` = `runtime`
- `$SOURCE_DIR` = `inputs/`
- Lead id: `user-registration` (matches the handler directory name verbatim)
- Slice name (for the `provenance.yaml` sample): `user-registration`

## Validation

The `expected/evidence.yaml` document validates against [`schemas/evidence.schema.json`](https://github.com/augentic/specify-cli/blob/main/schemas/evidence.schema.json) (the capture-backed replay workflow widened `claimKind` with `example` and requires `id` on every `kind: example` claim). The `expected/provenance.yaml` document validates against [`schemas/slice/provenance.schema.json`](https://github.com/augentic/specify-cli/blob/main/schemas/slice/provenance.schema.json) (workflow §D4 provenance index). The lead block in `expected/discovery.md` follows the grammar in [`schemas/discovery/lead.schema.json`](https://github.com/augentic/specify-cli/blob/main/schemas/discovery/lead.schema.json).

`replay-digest` values are real `sha256` over the on-disk capture bytes. Re-running `shasum -a 256 inputs/tests/data/replays/user-registration/*.json` MUST reproduce the digests written into `expected/evidence.yaml`.

## What the provenance.yaml sample demonstrates

The slice binds one source (`runtime` → `captures`); every requirement carries exactly one contributing claim. Both entries land on `resolution: single-source` with `status: agreed`. The cross-source resolutions (`single-value-agreement`, `authority-resolved`, `per-slice-override`, `tied-conflict`) live in [`plugins/spec/references/synthesis/provenance.md`](../../../../../plugins/spec/references/synthesis/provenance.md); this fixture pins the simplest shape.
