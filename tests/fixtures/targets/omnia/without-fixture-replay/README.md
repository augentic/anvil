# `omnia` target — `without-fixture-replay` fixture

Worked example for the omission-is-not-an-error posture on the shared fixture-replay hook ([`fixture-replay/hook-contract.md`](../../../../../adapters/targets/fixture-replay/hook-contract.md), RFC-27 §D1). The `.metadata.yaml` in this directory is what a build emits when the slice has no `code-runtime` source binding (or when a target adapter fork has not implemented the hook).

## What this fixture demonstrates

The fixture-replay step is **optional in v1**. A slice without a `code-runtime` source binding (or a target that does not run the hook) produces no `fixture-replay:` block. `/spec:merge` does not refuse on absence — the closing message simply omits the one-line replay summary.

The shared contract states this verbatim: targets that skip the step produce no `fixture-replay` surface; omission is not an error. Vectis is the canonical example of a target adapter that ignores the hook (it has no captured-fixture surface to replay against).

## Diff posture against `../with-fixture-replay/`

The two fixtures are byte-identical apart from the trailing `fixture-replay:` block. A `diff -u .metadata.yaml ../with-fixture-replay/.metadata.yaml` should show only the appended block in the sibling, no other field deltas.

## Validation

`.metadata.yaml` follows the `SliceMetadata` shape in [`crates/domain/src/slice/metadata.rs`](https://github.com/augentic/specify-cli/blob/main/crates/domain/src/slice/metadata.rs) (kebab-case field names; closed `status:` and `outcome.phase:` enums; ISO-8601 UTC timestamps). No `fixture-replay:` field appears — the absence itself is what this fixture pins.
