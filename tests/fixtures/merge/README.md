# Merge-engine fixtures (frozen regression baseline)

Ground-truth fixtures for the spec parser and merge engine in `specify_model::spec` / `specify_workflow::merge`. They are a **frozen regression baseline** against the current Rust implementation.

Each case directory contains a subset of:

- `baseline.md` — the pre-merge baseline spec (may be empty/missing for new-baseline cases).
- `delta.md` — the delta spec to merge.
- `expected-merged.md` — the canonical merged output. The Rust merge engine in `specify_workflow::merge` must reproduce this byte-for-byte.
- `expected-merge-errors.txt` — canonical stderr for merge failures. Empty file = success.
- `expected-validation.txt` — canonical stderr from `validate_baseline`. Empty file = all coherence checks passed.

These are checked in so Rust unit tests can compare byte-for-byte without invoking any external tool. A change to one of the cases must land alongside the corresponding source edit in `specify_workflow::merge` / `specify_model::validate` and a hand-crafted update to the `expected-*` file in the same commit.
