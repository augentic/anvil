# Merge-engine fixtures (frozen regression baseline)

Ground-truth fixtures for the spec parser and merge engine in `artifacts::spec` / `slice::merge`. They are a **frozen regression baseline** against the current Rust implementation.

Each case directory contains a subset of:

- `baseline.md` — the pre-merge baseline spec (may be empty/missing for new-baseline cases).
- `delta.md` — the delta spec to merge.
- `expected-merged.md` — the canonical merged output. The Rust merge engine in `slice::merge` must reproduce this byte-for-byte.
- `expected-merge-errors.txt` — canonical stderr for merge failures. Empty file = success.
- `expected-validation.txt` — canonical stderr from `validate_baseline`. Empty file = all coherence checks passed.

`crates/change/tests/merge_goldens.rs` drives the public `slice merge preview` operation over all nine cases. Cases 01–07 compare the in-memory merged output byte-for-byte; cases 08–09 compare post-merge validation output. Regenerate with `REGENERATE_GOLDENS=1 cargo nextest run -p workflow --test merge_goldens`, then review the golden diff. Parser-specific coverage remains in `crates/artifacts/tests/spec.rs`.
