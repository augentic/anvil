---
id: CORE-061
title: Adapter Extension Crate Missing
severity: important
trigger: adapter.yaml declares an extension block but the co-located extension/ crate directory or the committed adapter.wasm is missing.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/rules/core/CORE-061-adapter-extension-crate-missing.md
    description: Sentinel path so the whole-tree extension tool runs exactly once; the tool walks PROJECT_DIR itself rather than the passed candidate.
  - kind: tool
    value: extension
    description: Run the `extension` framework checker, which confirms every adapter that declares adapter.yaml.extension ships a co-located extension/ crate and a committed adapter.wasm. Whole-tree structural cross-fact; carries no policy.
---

## Rule

When an `adapter.yaml` declares a top-level `extension` block (RFC-48 D11), the adapter must ship its WASI extension from its own tree: a co-located Rust crate at `<adapter>/extension/` and the committed, built `adapter.wasm` at the adapter root `<adapter>/adapter.wasm` (RFC-48 D10). The extension version rides the adapter's semver, so the declaration carries no `version` or `source` — the crate at `extension/` is the source and the root `adapter.wasm` is the only shipped byte. This rule replaces the retired `adapter-tool` cross-reference rule.

This check is whole-tree: the `extension` framework tool walks every `adapters/{sources,targets}/<adapter>/` itself, so the rule's `path-pattern` names a single sentinel file to run the tool exactly once per lint.

## Look For

- An `adapter.yaml` carrying a top-level `extension` block with no `extension/` crate directory beside it.
- An `adapter.yaml` carrying a top-level `extension` block with no committed `adapter.wasm` at the adapter root.

## Fix

Add the co-located extension crate at `<adapter>/extension/` and commit the built `<adapter>/adapter.wasm` (run `specify adapter build` to compile and pack it), or remove the `adapter.yaml` extension block if the adapter ships no wasm extension.
