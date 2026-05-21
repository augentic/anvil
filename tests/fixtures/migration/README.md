# Migration fixtures

Used by `tests/migration_test.ts` to exercise
`scripts/migrate-to-2.0.sh` end-to-end.

- `1.x/` — minimal but representative Specify 1.x project tree. Covers
  every transform the script is required to perform: top-level
  `adapters/` rename; `.specify/project.yaml` key renames + version bump;
  `.specify/registry.yaml` per-project `adapter:` rename;
  `.specify/plan.yaml` slice reshape (bare-string sources + standalone
  `candidate:`, mixed structured / bare bindings, lifecycle collapse,
  status sanitisation); archived
  `.specify/archive/plans/<id>/plan.yaml` lifecycle collapse;
  `.specify/sources.yaml` `value:` -> `path:`;
  `.specify/.cache/adapters/<name>/` -> `.cache/targets/<name>/` rename;
  baseline `.specify/specs/<slice>/layout.yaml` retirement; baseline
  `.specify/specs/<slice>/composition.yaml` warning; retired
  `plugins/vectis/skills/image-layout-inferer/` warning.
- `2.0/` — golden output of running the migration against `1.x/`.
- `expected-dry-run.txt` — golden `--dry-run` log (the project root is
  substituted with `<PROJECT>` so the test is portable).

Regenerate `2.0/` by copying `1.x/` to a fresh directory, running
`scripts/migrate-to-2.0.sh <tmp>`, and copying the result back. Regenerate
`expected-dry-run.txt` by running `scripts/migrate-to-2.0.sh --dry-run`
on a copy of `1.x/` and substituting the temp path with `<PROJECT>`.
