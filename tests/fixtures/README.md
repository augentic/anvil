# Integration test fixtures

Each subdirectory holds the on-disk fixture trees one integration suite stages into its tempdir projects (`e2e/`, `plan/`, `merge/`, `journal/`, …); `lint-framework/` carries the golden JSON envelopes for `specify lint framework` (regenerate with `REGENERATE_GOLDENS=1 cargo nextest run --test it`).
