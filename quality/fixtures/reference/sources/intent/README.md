# `sources/intent` round-trip fixture

Static fixture covering the degenerate N=1 path for the [`intent` source adapter](https://github.com/augentic/specify-adapters/tree/main/sources/intent). The fixture is shape-only: it pins the inputs `/spec:plan` and `/spec:refine` pass to the adapter's `survey` and `extract` prompts, and the outputs those prompts must emit.

The canonical quality profiles are the executable surface that consumes this fixture. From this repository, the fixture is read-only documentation of the contract.

## Files

| File                                              | Role                                                                                |
| ------------------------------------------------- | ----------------------------------------------------------------------------------- |
| [`input.yaml`](input.yaml)                        | The `Source` binding + `Lead`/`slice-name` arguments the briefs are called with. |
| [`expected-survey.md`](expected-survey.md)  | The lead block `survey` must append under `## Lead inventory`.         |
| [`expected-extract.yaml`](expected-extract.yaml)  | The `Evidence` document `extract` must return for `/spec:refine` to persist.        |
