# `omnia` target — `without-replay` fixture

Worked example for the omission-is-not-an-error posture on the shared replay hook ([`replay/hook-contract.md`](../../../../../adapters/shared/target-hooks/replay/hook-contract.md), the capture-backed replay workflow). The `metadata.yaml` in this directory is what a build emits when the slice has no `captures` source binding (or when a target adapter fork has not implemented the hook).

## What this fixture demonstrates

The replay step is **optional in v1**. A slice without a `captures` source binding (or a target that does not run the hook) produces no `replay:` block. `/spec:merge` does not refuse on absence — the closing message simply omits the one-line replay summary.

The shared contract states this verbatim: targets that skip the step produce no `replay` surface; omission is not an error. Vectis is the canonical example of a target adapter that ignores the hook (it has no runtime-capture surface to replay against).

## Diff posture against `../with-replay/`

The two fixtures are byte-identical apart from the trailing `replay:` block. A `diff -u metadata.yaml ../with-replay/metadata.yaml` should show only the appended block in the sibling, no other field deltas.

## Validation

`metadata.yaml` follows the `SliceMetadata` shape in [`crates/workflow/src/slice/metadata.rs`](https://github.com/augentic/specify/blob/main/engine/crates/workflow/src/slice/metadata.rs) (kebab-case field names; closed `status:` and `outcome.phase:` enums; ISO-8601 UTC timestamps). No `replay:` field appears — the absence itself is what this fixture pins.
