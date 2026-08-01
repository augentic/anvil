## 0.36.0

Released 2026-08-01

### Compatibility

```text
engine 0.36.x  ↔  adapters 0.10.x  (WIT emery:adapter@0.1.0, floor ≥ 0.36.0)
```

### Changed

* `emery plan author --force` recreates an existing plan unconditionally (no longer limited to replaceable `pending` plans; `plan-author-not-replaceable` is gone).
* `emery plan transition <entry> --undo` becomes `emery plan undo <entry>`; `emery slice merge run` collapses to `emery slice merge`.
* Adapter store heal: on `adapter-sidecar-missing` / `adapter-digest-mismatch`, the launcher unlinks and reinstalls the pin once (offline heal still refuses). Install writes the digest sidecar before the component and validates a single manifest layer.

**Full Changelog**: https://github.com/augentic/emery/compare/v0.35.0...v0.36.0

---

Release notes for previous releases can be found on the respective release branches of the repository.

<!-- ARCHIVE_START -->
* [0.36.x](https://github.com/augentic/emery/blob/release-0.36.0/RELEASES.md)
* [0.35.x](https://github.com/augentic/emery/blob/release-0.35.0/RELEASES.md)
* [0.34.x](https://github.com/augentic/emery/blob/release-0.34.0/RELEASES.md)
* [0.33.x](https://github.com/augentic/emery/blob/release-0.33.0/RELEASES.md)
* [0.32.x](https://github.com/augentic/emery/blob/release-0.32.0/RELEASES.md)
* [0.31.x](https://github.com/augentic/emery/blob/release-0.31.0/RELEASES.md)
* [0.30.x](https://github.com/augentic/emery/blob/release-0.30.0/RELEASES.md)
* [0.28.x](https://github.com/augentic/emery/blob/release-0.28.0/RELEASES.md)
