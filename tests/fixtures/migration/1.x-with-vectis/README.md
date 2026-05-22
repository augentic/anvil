# `1.x-with-vectis` migration fixture

A minimal 1.x project tree exercised by [`tests/migration_e2e.ts`](../../../migration_e2e.ts) to verify three things:

1. The `migrate-to-2.0.sh` script rewrites every stale `$id` / `$schema` URL in baseline `composition.yaml` / `tokens.yaml` / `assets.yaml` files from the retired `adapters/vectis/` (or `schemas/vectis/`) location to `targets/vectis/schemas/<name>.schema.json`.
2. `.specify/project.yaml` migrates from `adapter:` -> `target:` and `specify_version:` -> `specify-version: 2.0.0`.
3. The migrated tree passes the 2.0 schema for `project.yaml` (loadable as the new shape) and emits a Vectis-composition warning for the operator to delete the artifact after the first 2.0 `/spec:execute`.

Unlike [`tests/fixtures/migration/1.x/`](../1.x/) (which is the larger end-to-end fixture for the byte-exact migration golden tree), this fixture is intentionally narrow: it only contains the surface area required to exercise the Vectis URL rewrite and the `project.yaml` reshape.
