## 0.30.0

Unreleased

### Compatibility

```text
engine 0.30.x  ↔  adapters 0.6.x  (WIT emery:adapter@0.1.0, floor ≥ 0.30.0)
```

### Added

* Product rename from Specify to **Emery**: shipped binary and Cursor skills are `emery` / `/emery:*`, on-disk state lives under `.emery/`, adapter package sugar is `emery:<name>@<version>`, and first-party components resolve from `ghcr.io/augentic/emery-adapters`.
* Omnia-hosted runtime: the engine guest is embedded in the binary and booted through `omnia::runtime!`, with a fail-closed adapters-only resolver over the host-owned store and project component cache.
* Host-owned adapter install: pinned first-party adapters pull-on-miss from GHCR into the global store (digest-gated, with OCI provenance on the sidecar); bare names resolve only the seeded project cache via `emery adapter add` or a local component at init.
* Gate 1 is the nameless `emery plan approve`; `/emery:execute` confirms approval when needed, then runs `emery plan execute`.
* Omnia-shaped release lines (`release-X.Y.Z` cut / patch / publish), root `RELEASES.md`, and `cargo binstall` / Homebrew install paths for platform archives.
* Contributor eval surface: `cargo make lab` / `cargo make eval <case>` over retained `sandbox/<case>/` trees, with optional `EVAL_LOG` file mirroring and leaner synthesis prompts for mid-tier models.

### Changed

* Workspace dependency keys match published `emery-*` crate names; short `use` paths still come from each crate's `[lib] name`.
* Release binaries matrix ships `aarch64-apple-darwin` only for this line (Linux and Intel macOS legs dropped; Windows remains unavailable until upstream `omnia-wasi-model` compiles there).
* Quick-start pins move to `contracts@0.6.0` / `intent@0.6.0`; README leads with install and a first change (Cursor Agent or terminal CLI).
* Vectis greenfield docs point at the local [`vectis-exemplar`](https://github.com/augentic/vectis-exemplar) checkout (`VECTIS_EXEMPLAR_DIR`), not an embedded template.
* Live-eval env knobs rename to `EVAL_MODEL` / `EVAL_TIMEOUT_SECS`; Omnia crates resolve from public GitHub git dependencies (no private Cargo registry config).
* Developer Guide link integrity is `mdbook-linkcheck2` via `cargo make links`; per-push WASM boundary CI is removed in favor of the operator-invoked wasm example.

**Full Changelog**: https://github.com/augentic/emery/compare/v0.29.0...v0.30.0

---

Release notes for previous releases can be found on the respective release branches of the repository.

<!-- ARCHIVE_START -->
* [0.30.x](https://github.com/augentic/emery/blob/release-0.30.0/RELEASES.md)
* [0.28.x](https://github.com/augentic/emery/blob/release-0.28.0/RELEASES.md)
