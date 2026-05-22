# `sources/intent` round-trip fixture

Static fixture covering the degenerate N=1 path for the [`intent` source adapter](../../../../adapters/sources/intent/adapter.yaml). The fixture is shape-only: it pins the inputs `/spec:plan` and `/spec:refine` pass to the [`enumerate`](../../../../adapters/sources/intent/briefs/enumerate.md) and [`extract`](../../../../adapters/sources/intent/briefs/extract.md) briefs, and the outputs those briefs must emit.

The `cli`-side acceptance harness (`specify source resolve intent` plus the synthesis runner once W3.1 lands) is the executable surface that consumes this fixture. From this repository, the fixture is read-only documentation of the contract.

## Files

| File                                              | Role                                                                                |
| ------------------------------------------------- | ----------------------------------------------------------------------------------- |
| [`input.yaml`](input.yaml)                        | The `Source` binding + `Candidate`/`slice-name` arguments the briefs are called with. |
| [`expected-enumerate.md`](expected-enumerate.md)  | The candidate block `enumerate` must append under `## Candidate inventory`.         |
| [`expected-extract.yaml`](expected-extract.yaml)  | The `Evidence` document `extract` must return for `/spec:refine` to persist.        |
