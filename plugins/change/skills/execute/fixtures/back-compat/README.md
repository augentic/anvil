# Metadata back-compat (RFC-9 §2B)

This fixture is the **proof-of-concept** that `.metadata.yaml` files written before RFC-9 §2B (the bump from the implicit `version: 1` to the new `version: 2`) round-trip cleanly through the new reader. It exists because the addition of the `Outcome::RegistryAmendmentRequired` variant is a wire-format change, and the only acceptable wire-format change is a back-compat one.

The fixture is fixture-pinned, not a runtime test — `crates/change/src/lib.rs::tests::metadata_pre_rfc9_round_trips_with_default_version_one` exercises the same shape via serde directly. This README + the YAML files document the contract for human readers and make the back-compat invariant easy to inspect at a glance.

## Scenario

A change named `archived-feature` ran to completion under a pre-RFC-9 binary, was merged, and is now sitting in `.specify/archive/2024-08-03-archived-feature/.metadata.yaml`. The on-disk file therefore:

- Has **no** `version:` field (pre-RFC-9 metadata predates the concept).
- Carries one of the three original `Outcome` variants — here, `success` from the merge phase.
- Uses the existing kebab-case schema for every other field.

A subsequent `/spec:execute` run, built from the post-RFC-9 binary, has reasons to read this archived file (e.g. `specify change outcome show <name>` falls back to the archive when the active change directory is gone — see `crates/change` and `src/commands/change.rs::resolve_archived_metadata`). The new reader MUST:

1. Accept the absent `version:` field as the implicit `1`.
2. Deserialise `outcome.outcome: success` as `Outcome::Success` (the closed unit variant).
3. Round-trip the rest of the metadata unchanged.

## Layout

| File | Pins |
|---|---|
| [`archived/.metadata.yaml.before`](archived/.metadata.yaml.before) | A pre-bump `.metadata.yaml` exactly as a pre-RFC-9 binary would have written it. No `version:` field; one of the three closed `Outcome` variants. |
| [`expected-outcome-show.json`](expected-outcome-show.json) | The shape the post-bump `specify change outcome show <name> --format json` emits when reading the pre-bump file via the archive fallback. The `outcome.proposal` key is **absent** (the variant is `success`, not `registry-amendment-required`); the `outcome.outcome` is the kebab-case discriminant string `success`. |
| [`expected-loaded-internal.yaml`](expected-loaded-internal.yaml) | The internal `ChangeMetadata` representation after loading: `version: 1` (defaulted), every other field as on disk. |

## Key invariants

- **Outcome dispatch is purely by discriminant.** The reader never branches on `version:`; it looks at `outcome.outcome`'s serde tag and dispatches to the matching variant. The `version` field is informational — used by tooling that wants to surface "this archive predates the new variant" diagnostics, never as a gate.
- **Defaulted version is `1`.** `default_metadata_version()` in `crates/change/src/lib.rs` returns `1`. Any `.metadata.yaml` without a `version:` field reads as version 1.
- **New writers always stamp `METADATA_VERSION`.** `crates/change/src/actions.rs::create` and the `sample_metadata` test helper both set `version: METADATA_VERSION` (currently `2`). New `.metadata.yaml` files therefore always carry the field; only **archived** files predate it.
- **No silent migration.** The reader never rewrites the pre-bump file. The version stamp on disk stays whatever it was; the reader is the only place the implicit `1` materialises.

## Counter-example (not pinned)

A pre-bump file with a malformed `outcome.outcome` value (e.g. `outcome.outcome: bogus`) is still rejected with `Error::Yaml`. Back-compat does not weaken the schema — it only tolerates the absence of the `version:` field and the closed three-variant set the pre-bump writer produced.
