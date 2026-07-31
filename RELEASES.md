## 0.35.0

Unreleased

### Compatibility

```text
engine 0.35.x  ↔  adapters 0.9.x  (WIT emery:adapter@0.1.0, floor ≥ 0.35.0)
```

Hard cut for bare adapter bindings: embedded adapter-train auto-pinning is gone. Bare names stay bare in `project.yaml` / `plan.yaml`; use `emery adapter update` (or `emery init` / `--upgrade`) for an explicit registry refresh.

### Added

* `emery adapter update <name>` — refresh a bare adapter name to the newest published exact SemVer (CLI + HTTP). Pinned versions and local components are refused (`adapter-update-not-bare`).

### Changed

* **Bare names resolve local-first.** Cache seed wins, else newest store version (offline), else pull-latest from GHCR. No auto-pin to `emery:<name>@<train>` at init / plan author / `--upgrade`.
* `emery --version` no longer prints `(adapters <train>)`; `FIRST_PARTY_ADAPTER_TRAIN` and `Resolver::expand` are removed.
* Launcher logs every settled adapter identity (host version + adapter version + origin) to stderr; new errors `adapter-latest-failed` / `adapter-latest-none`.
* Probe / launcher peel `--debug` / `--quiet` before dispatch so seed anchoring still works under those flags.

**Full Changelog**: https://github.com/augentic/emery/compare/v0.34.0...v0.35.0

---

Release notes for previous releases can be found on the respective release branches of the repository.

<!-- ARCHIVE_START -->
* [0.35.x](https://github.com/augentic/emery/blob/release-0.35.0/RELEASES.md)
* [0.34.x](https://github.com/augentic/emery/blob/release-0.34.0/RELEASES.md)
* [0.33.x](https://github.com/augentic/emery/blob/release-0.33.0/RELEASES.md)
* [0.32.x](https://github.com/augentic/emery/blob/release-0.32.0/RELEASES.md)
* [0.31.x](https://github.com/augentic/emery/blob/release-0.31.0/RELEASES.md)
* [0.30.x](https://github.com/augentic/emery/blob/release-0.30.0/RELEASES.md)
* [0.28.x](https://github.com/augentic/emery/blob/release-0.28.0/RELEASES.md)
